use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::future::join_all;
use once_cell::sync::Lazy;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::models::channel::Channel;
use crate::models::user::User;
use crate::modules::memory::find_tokens;

const SEARCH_URL: &str = "https://talk-pilsner.kakao.com/talkfinder-service/collection/search.json";

static COMBOS: Lazy<Vec<(&'static str, &'static str, &'static str)>> = Lazy::new(|| {
    let opts = ["true", "false"];
    let mut v = Vec::with_capacity(8);
    for f1 in opts {
        for f5 in opts {
            for f8 in opts {
                v.push((f1, f5, f8));
            }
        }
    }
    v
});

pub struct KakaoTalk {
    pub program: String,
    pid_cache: Mutex<Option<u32>>,
    auth_cache: Mutex<Option<String>>,
    sso_cache: Mutex<Option<String>>,
    cookies_cache: Mutex<Option<HashMap<String, String>>>,
    user_cache: Mutex<Option<User>>,
}

fn build_blocking_client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .cookie_store(true)
        .build()?)
}

fn build_async_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?)
}

impl KakaoTalk {
    pub fn new() -> Self {
        Self {
            program: "KakaoTalk.exe".to_string(),
            pid_cache: Mutex::new(None),
            auth_cache: Mutex::new(None),
            sso_cache: Mutex::new(None),
            cookies_cache: Mutex::new(None),
            user_cache: Mutex::new(None),
        }
    }

    pub async fn pid(&self) -> u32 {
        let mut guard = self.pid_cache.lock().await;
        if let Some(p) = *guard {
            if p != 0 {
                return p;
            }
        }
        let pid = find_pid_by_name(&self.program).unwrap_or(0);
        if pid != 0 {
            *guard = Some(pid);
        }
        pid
    }

    /// Clear all cached state (pid, auth, sso, cookies, user) so the next
    /// access re-checks against the current KakaoTalk process.
    pub async fn invalidate(&self) {
        *self.pid_cache.lock().await = None;
        *self.auth_cache.lock().await = None;
        *self.sso_cache.lock().await = None;
        *self.cookies_cache.lock().await = None;
        *self.user_cache.lock().await = None;
    }

    /// Live-check: hits the login_token endpoint with the cached auth token.
    /// Returns true only if the token is still valid (status == 0).
    pub async fn check_auth_alive(&self) -> bool {
        let auth = match self.authorization().await {
            Ok(t) => t,
            Err(_) => return false,
        };
        let client = match build_blocking_client() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let resp = client
            .get("https://katalk.kakao.com/win32/account/login_token.json")
            .header("Authorization", auth)
            .header("A", "win32/26.3.1/ko")
            .header("User-Agent", "KT/26.3.1 Wd/10.0.19045 ko")
            .send();
        let resp = match resp {
            Ok(r) => r,
            Err(_) => return false,
        };
        let json: Value = match resp.json() {
            Ok(j) => j,
            Err(_) => return false,
        };
        json.get("status").and_then(Value::as_i64) == Some(0)
    }

    pub async fn authorization(&self) -> Result<String> {
        {
            let guard = self.auth_cache.lock().await;
            if let Some(t) = guard.as_ref() {
                return Ok(t.clone());
            }
        }
        let pid = self.pid().await;
        if pid == 0 {
            return Err(anyhow!("{} is not running.", self.program));
        }
        let tokens = find_tokens(pid)?;
        let client = build_blocking_client()?;
        for token in tokens {
            let resp = client
                .get("https://katalk.kakao.com/win32/account/login_token.json")
                .header("Authorization", &token)
                .header("A", "win32/26.3.1/ko")
                .header("User-Agent", "KT/26.3.1 Wd/10.0.19045 ko")
                .send();
            let resp = match resp {
                Ok(r) => r,
                Err(_) => continue,
            };
            let json: Value = match resp.json() {
                Ok(j) => j,
                Err(_) => continue,
            };
            if json.get("status").and_then(Value::as_i64) == Some(0) {
                let mut guard = self.auth_cache.lock().await;
                *guard = Some(token.clone());
                return Ok(token);
            }
        }
        Err(anyhow!("Could not find a valid Authorization token."))
    }

    async fn sso_token(&self) -> Result<String> {
        {
            let guard = self.sso_cache.lock().await;
            if let Some(t) = guard.as_ref() {
                return Ok(t.clone());
            }
        }
        let auth = self.authorization().await?;
        let client = build_blocking_client()?;
        let resp = client
            .get("https://katalk.kakao.com/win32/account/login_token.json")
            .header("Authorization", auth)
            .header("A", "win32/26.3.1/ko")
            .header("User-Agent", "KT/26.3.1 Wd/10.0.19045 ko")
            .send()?;
        let json: Value = resp.json()?;
        let token = json
            .get("token")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("sso token missing"))?
            .to_string();
        let mut guard = self.sso_cache.lock().await;
        *guard = Some(token.clone());
        Ok(token)
    }

    pub async fn cookies(&self) -> Result<HashMap<String, String>> {
        {
            let guard = self.cookies_cache.lock().await;
            if let Some(c) = guard.as_ref() {
                return Ok(c.clone());
            }
        }
        let sso = self.sso_token().await?;
        let cookie_jar = Arc::new(reqwest::cookie::Jar::default());
        let client = reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(true)
            .cookie_provider(cookie_jar.clone())
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let cont = urlencoding::encode("https://pf.kakao.com");
        let url = format!(
            "https://accounts.kakao.com/weblogin/login_redirect?continue={}&token={}",
            cont, sso
        );
        let _ = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36")
            .header("Accept-Language", "ko-KR,ko;q=0.9")
            .send();

        let mut map = HashMap::new();
        let target_url = "https://accounts.kakao.com".parse::<reqwest::Url>()?;
        if let Some(header) = reqwest::cookie::CookieStore::cookies(&*cookie_jar, &target_url) {
            for piece in header.to_str().unwrap_or("").split("; ") {
                if let Some((k, v)) = piece.split_once('=') {
                    map.insert(k.to_string(), v.to_string());
                }
            }
        }
        let pf_url = "https://pf.kakao.com".parse::<reqwest::Url>()?;
        if let Some(header) = reqwest::cookie::CookieStore::cookies(&*cookie_jar, &pf_url) {
            for piece in header.to_str().unwrap_or("").split("; ") {
                if let Some((k, v)) = piece.split_once('=') {
                    map.insert(k.to_string(), v.to_string());
                }
            }
        }

        let mut guard = self.cookies_cache.lock().await;
        *guard = Some(map.clone());
        Ok(map)
    }

    pub async fn user(&self) -> Result<User> {
        {
            let guard = self.user_cache.lock().await;
            if let Some(u) = guard.as_ref() {
                return Ok(u.clone());
            }
        }
        let cookies = self.cookies().await?;
        let cookie_header = cookies_to_header(&cookies);
        let client = build_blocking_client()?;
        let resp = client
            .get("https://cmail.kakao.com/v3/users/profile")
            .header("Accept", "application/json, text/plain, */*")
            .header("Accept-Language", "ko-KR,ko;q=0.9")
            .header("Origin", "https://mail.kakao.com")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36")
            .header("Cookie", cookie_header)
            .send()?;
        let json: Value = resp.json()?;
        let user = User::from_value(&json);
        let mut guard = self.user_cache.lock().await;
        *guard = Some(user.clone());
        Ok(user)
    }

    pub fn fetch_channel(
        &self,
        url_or_encoded_id: &str,
        authorization: &str,
        cookies: &HashMap<String, String>,
    ) -> Option<Channel> {
        let encoded_id = Channel::extract_encoded_id(url_or_encoded_id);
        let cookie_header = cookies_to_header(cookies);
        let client = build_blocking_client().ok()?;
        let url = format!(
            "https://pf.kakao.com/rocket-web/web/profiles/{}/simple",
            encoded_id
        );
        let referer = format!("https://pf.kakao.com/{}", encoded_id);
        let resp = client
            .get(&url)
            .header("Accept", "application/json")
            .header("Accept-Language", "ko-KR,ko;q=0.9")
            .header("Referer", referer)
            .header("Cookie", cookie_header)
            .send()
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let json: Value = resp.json().ok()?;
        Channel::from_simple(&encoded_id, &json, authorization, cookies)
    }

    pub async fn search_channels(&self, keyword: &str) -> Result<HashMap<String, Channel>> {
        let auth = self.authorization().await?;
        let cookies = self.cookies().await?;
        let client = build_async_client()?;
        let seen: Arc<Mutex<HashMap<String, Channel>>> = Arc::new(Mutex::new(HashMap::new()));

        let mut futs = Vec::new();
        for (f1, f5, f8) in COMBOS.iter().copied() {
            let client = client.clone();
            let auth = auth.clone();
            let cookies = cookies.clone();
            let seen = seen.clone();
            let keyword = keyword.to_string();
            futs.push(tokio::spawn(async move {
                fetch_combo(client, &auth, &cookies, &keyword, f1, f5, f8, seen).await;
            }));
        }
        join_all(futs).await;

        let map = seen.lock().await.clone();
        Ok(map)
    }
}

#[allow(clippy::too_many_arguments)]
async fn fetch_combo(
    client: reqwest::Client,
    authorization: &str,
    cookies: &HashMap<String, String>,
    keyword: &str,
    f1: &str,
    f5: &str,
    f8: &str,
    seen: Arc<Mutex<HashMap<String, Channel>>>,
) {
    let extras = format!(
        r#"{{"channel":{{"searchOption":{{"f1":"{}","f5":"{}","f8":"{}"}},"filterView":false}}}}"#,
        f1, f5, f8
    );
    let mut page = 1u32;

    loop {
        let resp = client
            .get(SEARCH_URL)
            .query(&[
                ("collection", "CHANNEL"),
                ("query", keyword),
                ("size", "100"),
                ("from", "tab-chats"),
                ("page", &page.to_string()),
                ("extras", &extras),
            ])
            .header("Authorization", authorization)
            .header("talk-agent", "android/25.11.2")
            .header("User-Agent", "Mozilla/5.0")
            .header("Accept", "*/*")
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(_) => break,
        };
        let data: Value = match resp.json().await {
            Ok(j) => j,
            Err(_) => break,
        };

        let result = match data.get("result") {
            Some(r) if !r.is_null() => r,
            _ => break,
        };
        let docs = result
            .get("docGroups")
            .and_then(|g| g.as_array())
            .and_then(|g| g.first())
            .and_then(|d| d.get("docs"))
            .and_then(|d| d.as_array());
        let docs = match docs {
            Some(d) if !d.is_empty() => d,
            _ => break,
        };

        let mut new_channels = Vec::new();
        for doc in docs {
            if let Some(mut ch) = Channel::from_search_doc(doc) {
                ch.authorization = authorization.to_string();
                ch.cookies = cookies.clone();
                new_channels.push(ch);
            }
        }
        {
            let mut g = seen.lock().await;
            for ch in new_channels {
                g.entry(ch.id.clone()).or_insert(ch);
            }
        }

        let has_next = result
            .get("hasNext")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !has_next {
            break;
        }
        page += 1;
    }
}

fn cookies_to_header(cookies: &HashMap<String, String>) -> String {
    let mut s = String::new();
    for (k, v) in cookies {
        if !s.is_empty() {
            s.push_str("; ");
        }
        s.push_str(k);
        s.push('=');
        s.push_str(v);
    }
    s
}

#[cfg(windows)]
fn find_pid_by_name(program: &str) -> Option<u32> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return None;
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        let mut found: Option<u32> = None;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                if name.eq_ignore_ascii_case(program) {
                    found = Some(entry.th32ProcessID);
                    break;
                }
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
        found
    }
}

#[cfg(not(windows))]
fn find_pid_by_name(_program: &str) -> Option<u32> {
    None
}

impl Default for KakaoTalk {
    fn default() -> Self {
        Self::new()
    }
}
