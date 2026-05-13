use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use regex::bytes::Regex;
use std::collections::HashSet;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
#[cfg(windows)]
use windows_sys::Win32::System::Memory::{
    VirtualQueryEx, MEMORY_BASIC_INFORMATION, MEM_COMMIT, MEM_PRIVATE, PAGE_GUARD, PAGE_NOACCESS,
};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};

static AUTH_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)[Aa]uthorization:\s*([0-9a-f]{32}[0-9]{15,25}[A-Za-z0-9+/\-_]{60,}={0,2})",
    )
    .unwrap()
});

static CRED_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([0-9a-f]{32}[0-9]{15,25}[A-Za-z0-9+/\-_]{60,}={0,2})\x00").unwrap()
});

const CHUNK: usize = 0x400000; // 4MB

#[cfg(windows)]
fn read_process_memory(hproc: HANDLE, addr: usize, size: usize) -> Vec<u8> {
    let mut buf = vec![0u8; size];
    let mut read: usize = 0;
    let ok = unsafe {
        ReadProcessMemory(
            hproc,
            addr as *const _,
            buf.as_mut_ptr() as *mut _,
            size,
            &mut read,
        )
    };
    if ok == 0 {
        Vec::new()
    } else {
        buf.truncate(read);
        buf
    }
}

#[cfg(windows)]
fn enum_private_regions(hproc: HANDLE) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let mut addr: usize = 0;
    let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };

    loop {
        let ret = unsafe {
            VirtualQueryEx(
                hproc,
                addr as *const _,
                &mut mbi,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        if ret == 0 {
            break;
        }

        let skip = mbi.State != MEM_COMMIT
            || mbi.Type != MEM_PRIVATE
            || (mbi.Protect & PAGE_NOACCESS) != 0
            || (mbi.Protect & PAGE_GUARD) != 0;

        if !skip {
            regions.push((mbi.BaseAddress as usize, mbi.RegionSize));
        }

        let next = addr.saturating_add(mbi.RegionSize);
        if next <= addr || next >= 0x7FFF_FFFF_FFFF {
            break;
        }
        addr = next;
    }
    regions
}

#[cfg(windows)]
pub fn find_tokens(pid: u32) -> Result<Vec<String>> {
    let hproc =
        unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, 0, pid) };
    if hproc == 0 {
        return Err(anyhow!(
            "OpenProcess failed (PID={}), administrator privileges required",
            pid
        ));
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();

    let regions = enum_private_regions(hproc);
    for (base, size) in regions {
        let mut offset = 0usize;
        while offset < size {
            let chunk_size = std::cmp::min(CHUNK, size - offset);
            let data = read_process_memory(hproc, base + offset, chunk_size);

            for cap in AUTH_RE.captures_iter(&data) {
                if let Some(m) = cap.get(1) {
                    if let Ok(s) = std::str::from_utf8(m.as_bytes()) {
                        if !seen.contains(s) {
                            seen.insert(s.to_string());
                            out.push(s.to_string());
                        }
                    }
                }
            }
            for cap in CRED_RE.captures_iter(&data) {
                if let Some(m) = cap.get(1) {
                    if let Ok(s) = std::str::from_utf8(m.as_bytes()) {
                        if !seen.contains(s) {
                            seen.insert(s.to_string());
                            out.push(s.to_string());
                        }
                    }
                }
            }

            offset += chunk_size;
        }
    }

    unsafe { CloseHandle(hproc) };
    Ok(out)
}

#[cfg(not(windows))]
pub fn find_tokens(_pid: u32) -> Result<Vec<String>> {
    Err(anyhow!("find_tokens is only available on Windows"))
}
