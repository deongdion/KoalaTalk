//! Win32 chatroom automation: clipboard text/image and key paste-and-send.
//! Windows-only.

use crate::models::content::Content;

#[cfg(not(windows))]
pub fn open_chatroom(_title: &str, _window_open_secs: f32) -> Option<(isize, isize, isize)> {
    None
}

#[cfg(not(windows))]
pub fn send_to_chatroom(
    _title: &str,
    _contents: &[Content],
    _search_box: isize,
    _kt_hwnd: isize,
    _chatroom_hwnd: isize,
    _paste_delay_secs: f32,
) -> bool {
    false
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ffi::{c_void, OsStr};
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr;
    use std::thread;
    use std::time::Duration;

    use windows_sys::Win32::Foundation::{CloseHandle, GlobalFree, HANDLE, HWND, LPARAM, WPARAM};
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW,
        SetClipboardData,
    };
    use windows_sys::Win32::System::Diagnostics::Debug::WriteProcessMemory;
    use windows_sys::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, VirtualAllocEx, VirtualFreeEx, GMEM_MOVEABLE,
        MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
    };
    use windows_sys::Win32::System::Threading::{
        AttachThreadInput, GetCurrentThreadId, OpenProcess, PROCESS_VM_OPERATION,
        PROCESS_VM_WRITE,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        keybd_event, SetFocus, KEYEVENTF_KEYUP, VK_CONTROL, VK_MENU, VK_RETURN,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, EnumWindows, FindWindowExW, FindWindowW, GetForegroundWindow,
        GetWindow, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, SendMessageW,
        SetForegroundWindow, ShowWindow, GW_CHILD, SW_RESTORE, WM_CLOSE, WM_DROPFILES,
        WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_SETTEXT,
    };

    type HGlobal = *mut c_void;

    const CF_DIB: u32 = 8;
    const CF_UNICODETEXT: u32 = 13;

    const FRIENDS_TAB_X: i32 = 34;
    const FRIENDS_TAB_Y: i32 = 38;

    fn to_w(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    fn find_window(class: Option<&str>, title: Option<&str>) -> HWND {
        let class_w = class.map(to_w);
        let title_w = title.map(to_w);
        unsafe {
            FindWindowW(
                class_w
                    .as_ref()
                    .map(|v| v.as_ptr())
                    .unwrap_or(ptr::null()),
                title_w
                    .as_ref()
                    .map(|v| v.as_ptr())
                    .unwrap_or(ptr::null()),
            )
        }
    }

    fn find_window_ex(parent: HWND, after: HWND, class: Option<&str>, title: Option<&str>) -> HWND {
        let class_w = class.map(to_w);
        let title_w = title.map(to_w);
        unsafe {
            FindWindowExW(
                parent,
                after,
                class_w
                    .as_ref()
                    .map(|v| v.as_ptr())
                    .unwrap_or(ptr::null()),
                title_w
                    .as_ref()
                    .map(|v| v.as_ptr())
                    .unwrap_or(ptr::null()),
            )
        }
    }

    fn alloc_global(data: &[u8]) -> Result<HGlobal, &'static str> {
        let h = unsafe { GlobalAlloc(GMEM_MOVEABLE, data.len()) };
        if h.is_null() {
            return Err("GlobalAlloc failed");
        }
        let lock_ptr = unsafe { GlobalLock(h) };
        if lock_ptr.is_null() {
            unsafe { GlobalFree(h) };
            return Err("GlobalLock failed");
        }
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), lock_ptr as *mut u8, data.len());
            GlobalUnlock(h);
        }
        Ok(h)
    }

    fn set_clipboard_raw(fmt: u32, data: &[u8]) -> Result<(), &'static str> {
        let h = alloc_global(data)?;
        if unsafe { OpenClipboard(0) } == 0 {
            unsafe { GlobalFree(h) };
            return Err("OpenClipboard failed");
        }
        unsafe { EmptyClipboard() };
        let result = unsafe { SetClipboardData(fmt, h as HANDLE) };
        unsafe { CloseClipboard() };
        if result == 0 {
            unsafe { GlobalFree(h) };
            return Err("SetClipboardData failed");
        }
        Ok(())
    }

    fn set_clipboard_text(text: &str) -> Result<(), &'static str> {
        let buf: Vec<u16> = text.encode_utf16().chain(once(0)).collect();
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len() * 2)
        };
        set_clipboard_raw(CF_UNICODETEXT, bytes)
    }

    fn set_clipboard_image(path: &str) -> Result<(), String> {
        let img = image::open(path).map_err(|e| format!("image open: {e}"))?;
        let rgb = img.to_rgb8();
        let (width, height) = rgb.dimensions();
        let w = width as i32;
        let h = height as i32;

        let row_bytes = (width * 3) as usize;
        let pad = ((row_bytes + 3) & !3) - row_bytes;
        let padded_row = row_bytes + pad;
        let mut raw = vec![0u8; padded_row * height as usize];

        let raw_data = rgb.as_raw();
        for y in 0..height as usize {
            let src_y = height as usize - 1 - y;
            let src_off = src_y * row_bytes;
            let row = &raw_data[src_off..src_off + row_bytes];
            let dst = &mut raw[y * padded_row..y * padded_row + row_bytes];
            for (i, px) in row.chunks_exact(3).enumerate() {
                dst[i * 3] = px[2];
                dst[i * 3 + 1] = px[1];
                dst[i * 3 + 2] = px[0];
            }
        }

        let mut bih = Vec::with_capacity(40);
        bih.extend(40u32.to_le_bytes());
        bih.extend(w.to_le_bytes());
        bih.extend(h.to_le_bytes());
        bih.extend(1u16.to_le_bytes());
        bih.extend(24u16.to_le_bytes());
        bih.extend(0u32.to_le_bytes());
        bih.extend((raw.len() as u32).to_le_bytes());
        bih.extend(2835i32.to_le_bytes());
        bih.extend(2835i32.to_le_bytes());
        bih.extend(0u32.to_le_bytes());
        bih.extend(0u32.to_le_bytes());

        let mut dib = bih;
        dib.extend(&raw);

        let mut png_buf: Vec<u8> = Vec::new();
        {
            use image::ImageEncoder;
            let encoder = image::codecs::png::PngEncoder::new(&mut png_buf);
            encoder
                .write_image(rgb.as_raw(), width, height, image::ExtendedColorType::Rgb8)
                .map_err(|e| format!("png encode: {e}"))?;
        }

        let h_dib = alloc_global(&dib).map_err(|e| e.to_string())?;
        let h_png = alloc_global(&png_buf).map_err(|e| e.to_string())?;

        let cf_png_name = to_w("PNG");
        let cf_png = unsafe { RegisterClipboardFormatW(cf_png_name.as_ptr()) };

        if unsafe { OpenClipboard(0) } == 0 {
            unsafe {
                GlobalFree(h_dib);
                GlobalFree(h_png);
            }
            return Err("OpenClipboard failed".into());
        }
        unsafe {
            EmptyClipboard();
            if SetClipboardData(CF_DIB, h_dib as HANDLE) == 0 {
                GlobalFree(h_dib);
            }
            if SetClipboardData(cf_png, h_png as HANDLE) == 0 {
                GlobalFree(h_png);
            }
            CloseClipboard();
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn drop_file(hwnd: HWND, path: &str) -> Result<(), &'static str> {
        let mut pid: u32 = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };

        let h_proc = unsafe { OpenProcess(PROCESS_VM_OPERATION | PROCESS_VM_WRITE, 0, pid) };
        if h_proc == 0 {
            return Err("OpenProcess failed");
        }

        let mut data: Vec<u8> = Vec::new();
        data.extend(20u32.to_le_bytes());
        data.extend(0u32.to_le_bytes());
        data.extend(0u32.to_le_bytes());
        data.extend(0u32.to_le_bytes());
        data.extend(1u32.to_le_bytes());

        let path_w: Vec<u16> = OsStr::new(path)
            .encode_wide()
            .chain(once(0))
            .chain(once(0))
            .collect();
        let path_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(path_w.as_ptr() as *const u8, path_w.len() * 2)
        };
        data.extend_from_slice(path_bytes);

        let mem = unsafe {
            VirtualAllocEx(
                h_proc,
                ptr::null(),
                data.len(),
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        if mem.is_null() {
            unsafe { CloseHandle(h_proc) };
            return Err("VirtualAllocEx failed");
        }

        let mut written: usize = 0;
        let ok = unsafe {
            WriteProcessMemory(
                h_proc,
                mem,
                data.as_ptr() as *const _,
                data.len(),
                &mut written,
            )
        };
        if ok == 0 {
            unsafe {
                VirtualFreeEx(h_proc, mem, 0, MEM_RELEASE);
                CloseHandle(h_proc);
            }
            return Err("WriteProcessMemory failed");
        }

        unsafe {
            PostMessageW(hwnd, WM_DROPFILES, mem as WPARAM, 0);
        }

        let h_proc_val = h_proc;
        let mem_val = mem as usize;
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(5));
            unsafe {
                VirtualFreeEx(h_proc_val, mem_val as *mut c_void, 0, MEM_RELEASE);
                CloseHandle(h_proc_val);
            }
        });

        Ok(())
    }

    /// Aggressively bring `hwnd` to the foreground and set keyboard focus
    /// on `target`.  Retries once if the first attempt fails.
    fn force_foreground(hwnd: HWND, target: HWND) {
        for attempt in 0..2 {
            unsafe {
                // Alt-key trick: Windows lets a thread call SetForegroundWindow
                // only while processing input — a synthetic Alt press satisfies
                // that requirement.
                keybd_event(VK_MENU as u8, 0, 0, 0);
                keybd_event(VK_MENU as u8, 0, KEYEVENTF_KEYUP, 0);

                ShowWindow(hwnd, SW_RESTORE);
                BringWindowToTop(hwnd);

                let cur_tid = GetCurrentThreadId();
                let tgt_tid = GetWindowThreadProcessId(hwnd, ptr::null_mut());
                AttachThreadInput(cur_tid, tgt_tid, 1);
                SetForegroundWindow(hwnd);
                SetFocus(target);
                AttachThreadInput(cur_tid, tgt_tid, 0);

            }
            thread::sleep(Duration::from_millis(150));

            // Verify
            let fg = unsafe { GetForegroundWindow() };
            if fg == hwnd {
                return;
            }
            if attempt == 0 {
                thread::sleep(Duration::from_millis(200));
            }
        }
    }

    /// Ctrl+V then Enter.  Assumes the chatroom already has foreground focus
    /// (caller must have called `force_foreground` once before the loop).
    fn paste_and_send(wait_after_paste: f32, wait_after_send: f32) {
        unsafe {
            keybd_event(VK_CONTROL as u8, 0, 0, 0);
            keybd_event(b'V', 0, 0, 0);
            thread::sleep(Duration::from_millis(50));
            keybd_event(b'V', 0, KEYEVENTF_KEYUP, 0);
            keybd_event(VK_CONTROL as u8, 0, KEYEVENTF_KEYUP, 0);
        }
        thread::sleep(Duration::from_secs_f32(wait_after_paste));

        unsafe {
            keybd_event(VK_RETURN as u8, 0, 0, 0);
            thread::sleep(Duration::from_millis(50));
            keybd_event(VK_RETURN as u8, 0, KEYEVENTF_KEYUP, 0);
        }
        thread::sleep(Duration::from_secs_f32(wait_after_send));
    }

    fn pid_of(hwnd: HWND) -> u32 {
        let mut pid: u32 = 0;
        if hwnd != 0 {
            unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        }
        pid
    }

    /// Returns true if `hwnd` belongs to the same process as `ref_hwnd`.
    fn same_process(hwnd: HWND, ref_hwnd: HWND) -> bool {
        let a = pid_of(hwnd);
        let b = pid_of(ref_hwnd);
        a != 0 && a == b
    }

    /// Collect all visible top-level windows belonging to `target_pid`.
    fn enum_windows_for_pid(target_pid: u32) -> Vec<HWND> {
        struct Ctx {
            pid: u32,
            out: Vec<HWND>,
        }
        unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> i32 {
            let ctx = &mut *(lparam as *mut Ctx);
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == ctx.pid && IsWindowVisible(hwnd) != 0 {
                ctx.out.push(hwnd);
            }
            1 // continue
        }
        let mut ctx = Ctx { pid: target_pid, out: Vec::new() };
        unsafe { EnumWindows(Some(cb), &mut ctx as *mut Ctx as LPARAM) };
        ctx.out
    }

    /// Find a KakaoTalk chatroom window that appeared after a search.
    /// A chatroom is identified as a visible top-level window (same process)
    /// that contains a RICHEDIT50W child (the chat input box) and is not
    /// the main KakaoTalk window.
    fn find_chatroom_hwnd(kt_hwnd: HWND) -> HWND {
        let pid = pid_of(kt_hwnd);
        if pid == 0 {
            return 0;
        }
        let windows = enum_windows_for_pid(pid);
        for hwnd in windows {
            if hwnd == kt_hwnd {
                continue;
            }
            let rich = find_window_ex(hwnd, 0, Some("RICHEDIT50W"), None);
            if rich != 0 {
                return hwnd;
            }
        }
        0
    }

    fn cleanup(search_box: HWND, kt_hwnd: HWND, chatroom_hwnd: HWND) {
        // Only close the chatroom if it is a separate window that belongs to the
        // same KakaoTalk process.  This prevents accidentally sending WM_CLOSE to
        // the KoalaTalk (Tauri) window or any other unrelated window.
        if chatroom_hwnd != 0
            && chatroom_hwnd != kt_hwnd
            && same_process(chatroom_hwnd, kt_hwnd)
        {
            unsafe { PostMessageW(chatroom_hwnd, WM_CLOSE, 0, 0) };
            thread::sleep(Duration::from_millis(200));
        }
        if search_box != 0 {
            let empty = to_w("");
            unsafe { SendMessageW(search_box, WM_SETTEXT, 0, empty.as_ptr() as LPARAM) };
        }
    }

    pub fn open_chatroom(title: &str, window_open_secs: f32) -> Option<(isize, isize, isize)> {
        let kt = find_window(None, Some("카카오톡"));
        if kt == 0 {
            return None;
        }

        // Remember which chatroom windows already exist so we don't confuse
        // a pre-existing one with the one we're about to open.
        let before: std::collections::HashSet<HWND> = {
            let pid = pid_of(kt);
            enum_windows_for_pid(pid)
                .into_iter()
                .filter(|&h| h != kt && find_window_ex(h, 0, Some("RICHEDIT50W"), None) != 0)
                .collect()
        };

        let main_view = find_window_ex(kt, 0, Some("EVA_ChildWindow"), None);
        if main_view == 0 {
            return None;
        }

        let tab_handle = unsafe { GetWindow(main_view, GW_CHILD) };
        let lparam: LPARAM = ((FRIENDS_TAB_Y << 16) | FRIENDS_TAB_X) as LPARAM;
        unsafe {
            SendMessageW(tab_handle, WM_LBUTTONDOWN, 0, lparam);
            SendMessageW(tab_handle, WM_LBUTTONUP, 0, lparam);
        }
        thread::sleep(Duration::from_millis(300));

        let contact = find_window_ex(main_view, 0, Some("EVA_Window"), None);
        let search_box = find_window_ex(contact, 0, Some("Edit"), None);
        if search_box == 0 {
            return None;
        }

        let title_w = to_w(title);
        unsafe { SendMessageW(search_box, WM_SETTEXT, 0, title_w.as_ptr() as LPARAM) };
        thread::sleep(Duration::from_millis(1500));
        unsafe {
            PostMessageW(search_box, WM_KEYDOWN, VK_RETURN as WPARAM, 0);
            thread::sleep(Duration::from_millis(50));
            PostMessageW(search_box, WM_KEYUP, VK_RETURN as WPARAM, 0);
        }
        let wait_ms = ((window_open_secs * 1000.0) as u64).max(500);
        thread::sleep(Duration::from_millis(wait_ms));

        // Find the chatroom: enumerate KakaoTalk's windows and pick the NEW
        // one that has a RICHEDIT50W child.  This works regardless of which
        // window has foreground focus.
        let pid = pid_of(kt);
        let mut chatroom_hwnd: HWND = 0;
        let windows = enum_windows_for_pid(pid);
        for hwnd in windows {
            if hwnd == kt || before.contains(&hwnd) {
                continue;
            }
            if find_window_ex(hwnd, 0, Some("RICHEDIT50W"), None) != 0 {
                chatroom_hwnd = hwnd;
                break;
            }
        }

        // Fallback: if no NEW chatroom appeared, maybe the search opened an
        // existing chatroom window (already in `before`).  Pick any chatroom
        // that isn't the main window.
        if chatroom_hwnd == 0 {
            chatroom_hwnd = find_chatroom_hwnd(kt);
        }

        Some((search_box, kt, chatroom_hwnd))
    }

    pub fn send_to_chatroom(
        _title: &str,
        contents: &[Content],
        search_box: isize,
        kt_hwnd: isize,
        chatroom_hwnd: isize,
        paste_delay_secs: f32,
    ) -> bool {
        let mut hwnd: HWND = chatroom_hwnd;
        if hwnd == 0 {
            hwnd = kt_hwnd;
        }
        if hwnd == 0 {
            hwnd = find_window(None, Some("카카오톡"));
        }
        if hwnd == 0 {
            cleanup(search_box, kt_hwnd, chatroom_hwnd);
            return false;
        }

        let input_box = find_window_ex(hwnd, 0, Some("RICHEDIT50W"), None);
        if input_box == 0 {
            cleanup(search_box, kt_hwnd, chatroom_hwnd);
            return false;
        }

        // Acquire foreground focus ONCE before the content loop.
        // Calling force_foreground per-block would send repeated Alt
        // keypresses that activate KakaoTalk's menu bar and steal focus
        // away from the input box.
        force_foreground(hwnd, input_box);
        // Extra settling time: on the 2nd+ channel the chatroom window is
        // already part of a running KakaoTalk process, and the Alt-key trick
        // inside force_foreground can briefly put KakaoTalk into its menu
        // activation mode (~200 ms).  Without this pause the very first
        // keybd_event fires while focus is still in transition and the first
        // content block gets swallowed.  Subsequent blocks are fine because
        // the wait_after_paste / wait_after_send delays give enough time.
        thread::sleep(Duration::from_millis(400));

        let mut ok = true;
        for item in contents {
            match item {
                Content::Text(text) => {
                    if let Err(e) = set_clipboard_text(text) {
                        eprintln!("[send_to_chatroom] clipboard text error: {e}");
                        ok = false;
                        break;
                    }
                    // clipboard set → Ctrl+V immediately (no delay between)
                    paste_and_send(paste_delay_secs, 0.5);
                }
                Content::Image(path) => {
                    let size_mb = match Path::new(path).metadata() {
                        Ok(m) => (m.len() as f32) / (1024.0 * 1024.0),
                        Err(e) => {
                            eprintln!("[send_to_chatroom] stat: {e}");
                            ok = false;
                            break;
                        }
                    };
                    let send_wait = (2.0_f32 + size_mb * 1.5).clamp(2.0, 300.0);
                    if let Err(e) = set_clipboard_image(path) {
                        eprintln!("[send_to_chatroom] clipboard image error: {e}");
                        ok = false;
                        break;
                    }
                    // Images need more time between Ctrl+V and Enter for
                    // KakaoTalk to render the preview in the input box.
                    paste_and_send((paste_delay_secs + 0.5).max(1.0), send_wait);
                }
            }
        }

        cleanup(search_box, kt_hwnd, chatroom_hwnd);
        ok
    }
}

#[cfg(windows)]
pub use imp::{open_chatroom, send_to_chatroom};
