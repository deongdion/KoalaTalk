use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use kakaotalk::{Channel, Content, KakaoTalk};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

// ─── DTOs ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct ProfileDto {
    pub name: String,
    pub email: String,
    #[serde(rename = "profileUrl")]
    pub profile_url: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChannelDto {
    pub id: String,
    pub title: String,
    #[serde(rename = "encodedId")]
    pub encoded_id: String,
    #[serde(rename = "imageUrl")]
    pub image_url: String,
    #[serde(rename = "friendCount")]
    pub friend_count: i64,
    pub consult: bool,
    pub keyword: String,
    pub address: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum BlockDto {
    Text { content: String },
    Image { path: String },
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct DelaysDto {
    #[serde(rename = "addMinSec", default)]
    pub add_min: f32,
    #[serde(rename = "addMaxSec", default)]
    pub add_max: f32,
    #[serde(rename = "sendMinSec", default)]
    pub send_min: f32,
    #[serde(rename = "sendMaxSec", default)]
    pub send_max: f32,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Ok,
    Skip,
    Err,
}

#[derive(Debug, Serialize, Clone)]
pub struct LogEvent {
    pub level: LogLevel,
    pub text: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct DoneEvent {
    pub ok: bool,
    pub sent: u32,
    pub skipped: u32,
}

// ─── shared state ─────────────────────────────────────────────────────

pub struct AppState {
    pub kt: Arc<KakaoTalk>,
    pub running: Mutex<bool>,
    /// extracted channels indexed by id, kept across the extract → send flow
    pub channels: Mutex<HashMap<String, Channel>>,
    /// channels we've already messaged (persisted to disk)
    pub sent_cache: Mutex<HashSet<String>>,
    pub cache_path: PathBuf,
    /// cooperative cancel signal; checked by long-running loops
    pub cancel_flag: Arc<AtomicBool>,
}

fn load_sent_cache(path: &std::path::Path) -> HashSet<String> {
    match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => HashSet::new(),
    }
}

fn save_sent_cache(path: &std::path::Path, cache: &HashSet<String>) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string(cache) {
        let _ = std::fs::write(path, s);
    }
}

// ─── commands ─────────────────────────────────────────────────────────

#[tauri::command]
async fn check_kakao(app: AppHandle, state: State<'_, AppState>) -> Result<ProfileDto, String> {
    let kt = state.kt.clone();
    // Always re-evaluate against the current process / auth state so that the
    // "다시 확인" button works after KakaoTalk is launched.
    kt.invalidate().await;

    let pid = kt.pid().await;
    if pid == 0 {
        emit_log(
            &app,
            LogLevel::Err,
            "KakaoTalk.exe 프로세스를 찾을 수 없습니다 (관리자 권한으로 실행 중인지 확인하세요).",
        );
        return Err("process_not_found".into());
    }
    emit_log(&app, LogLevel::Info, format!("KakaoTalk PID = {pid}"));

    match kt.authorization().await {
        Ok(_) => emit_log(&app, LogLevel::Ok, "Auth 토큰 추출 및 검증 성공."),
        Err(e) => {
            let msg = e.to_string();
            emit_log(
                &app,
                LogLevel::Err,
                format!("Auth 토큰 추출 실패: {msg}"),
            );
            if msg.contains("Could not find a valid Authorization token") {
                emit_log(
                    &app,
                    LogLevel::Info,
                    "💡 PC 카카오톡에서 [더보기 ⋯] → [메일 ✉] 아이콘을 한 번 클릭한 뒤 \"다시 확인\" 버튼을 눌러주세요. (메일 화면 진입 시 카카오톡이 메모리에 토큰을 적재합니다.)",
                );
            }
            return Err(format!("auth_failed: {msg}"));
        }
    }

    match kt.user().await {
        Ok(user) => {
            emit_log(&app, LogLevel::Ok, format!("프로필 조회 성공: {}", user.name));
            Ok(ProfileDto {
                name: user.name,
                email: user.email,
                profile_url: user.profile_url,
            })
        }
        Err(e) => {
            emit_log(&app, LogLevel::Err, format!("프로필 조회 실패: {e}"));
            Err(format!("profile_failed: {e}"))
        }
    }
}

#[tauri::command]
async fn extract_channels(
    app: AppHandle,
    state: State<'_, AppState>,
    keywords: Vec<String>,
    dedupe: bool,
) -> Result<Vec<ChannelDto>, String> {
    {
        let mut g = state.running.lock().await;
        if *g {
            return Err("already running".into());
        }
        *g = true;
    }
    state.cancel_flag.store(false, Ordering::SeqCst);

    let result = extract_inner(&app, &state, &keywords, dedupe).await;
    *state.running.lock().await = false;
    result
}

#[tauri::command]
async fn cancel_work(state: State<'_, AppState>) -> Result<(), String> {
    state.cancel_flag.store(true, Ordering::SeqCst);
    Ok(())
}

async fn ensure_auth(app: &AppHandle, kt: &Arc<KakaoTalk>) -> bool {
    if kt.check_auth_alive().await {
        return true;
    }
    emit_log(
        app,
        LogLevel::Info,
        "Auth 토큰이 만료된 것 같습니다. 카카오톡에서 토큰을 다시 가져옵니다...",
    );
    kt.invalidate().await;
    if kt.authorization().await.is_err() {
        emit_log(
            app,
            LogLevel::Err,
            "토큰 재추출 실패. 카카오톡 연결이 끊어졌습니다.",
        );
        let _ = app.emit("auth_lost", ());
        return false;
    }
    if !kt.check_auth_alive().await {
        emit_log(
            app,
            LogLevel::Err,
            "재추출한 토큰도 검증에 실패했습니다. 작업을 중단합니다.",
        );
        let _ = app.emit("auth_lost", ());
        return false;
    }
    emit_log(app, LogLevel::Ok, "토큰 재추출 성공. 작업을 계속합니다.");
    true
}

async fn extract_inner(
    app: &AppHandle,
    state: &State<'_, AppState>,
    keywords: &[String],
    dedupe: bool,
) -> Result<Vec<ChannelDto>, String> {
    let kt = state.kt.clone();

    if !ensure_auth(app, &kt).await {
        return Err("auth_lost".into());
    }

    emit_log(app, LogLevel::Info, format!("키워드 {}개 추출 시작.", keywords.len()));

    let mut by_id: HashMap<String, Channel> = HashMap::new();
    let mut dtos: Vec<ChannelDto> = Vec::new();
    let mut seen_for_dedupe: std::collections::HashSet<String> = std::collections::HashSet::new();

    for kw in keywords {
        if state.cancel_flag.load(Ordering::SeqCst) {
            emit_log(app, LogLevel::Info, "사용자에 의해 추출이 중단되었습니다.");
            break;
        }
        emit_log(app, LogLevel::Info, format!(r#""{}" 검색 중..."#, kw));
        match kt.search_channels(kw).await {
            Ok(found) => {
                emit_log(app, LogLevel::Ok, format!(r#""{}": {}개 발견."#, kw, found.len()));
                for (id, ch) in found {
                    if dedupe && seen_for_dedupe.contains(&id) {
                        continue;
                    }
                    seen_for_dedupe.insert(id.clone());
                    dtos.push(ChannelDto {
                        id: id.clone(),
                        title: ch.title.clone(),
                        encoded_id: ch.encoded_id.clone(),
                        image_url: ch.image_url.clone(),
                        friend_count: ch.friend_count,
                        consult: ch.consult,
                        keyword: kw.clone(),
                        address: ch.location_address.clone(),
                    });
                    by_id.entry(id).or_insert(ch);
                }
            }
            Err(e) => emit_log(app, LogLevel::Err, format!("검색 실패: {e}")),
        }
    }

    let mut channels = state.channels.lock().await;
    channels.clear();
    for (id, ch) in by_id {
        channels.insert(id, ch);
    }
    dtos.sort_by(|a, b| b.friend_count.cmp(&a.friend_count));
    emit_log(app, LogLevel::Ok, format!("총 {}개 채널 추출.", dtos.len()));
    Ok(dtos)
}

#[tauri::command]
async fn run_automation(
    app: AppHandle,
    state: State<'_, AppState>,
    keywords: Vec<String>,
    blocks: Vec<BlockDto>,
    delays: DelaysDto,
) -> Result<DoneEvent, String> {
    {
        let mut g = state.running.lock().await;
        if *g {
            return Err("already running".into());
        }
        *g = true;
    }
    state.cancel_flag.store(false, Ordering::SeqCst);

    let result = run_inner(&app, &state, &keywords, &blocks, &delays, None).await;
    *state.running.lock().await = false;
    result
}

#[tauri::command]
async fn send_to_selected(
    app: AppHandle,
    state: State<'_, AppState>,
    #[allow(non_snake_case)] channelIds: Vec<String>,
    blocks: Vec<BlockDto>,
    delays: DelaysDto,
) -> Result<DoneEvent, String> {
    {
        let mut g = state.running.lock().await;
        if *g {
            return Err("already running".into());
        }
        *g = true;
    }
    state.cancel_flag.store(false, Ordering::SeqCst);

    let result = run_inner(&app, &state, &[], &blocks, &delays, Some(&channelIds)).await;
    *state.running.lock().await = false;
    result
}

fn emit_log(app: &AppHandle, level: LogLevel, text: impl Into<String>) {
    let _ = app.emit(
        "log",
        LogEvent {
            level,
            text: text.into(),
        },
    );
}

fn random_delay(min_sec: f32, max_sec: f32) -> Duration {
    let lo = min_sec.max(0.0);
    let hi = max_sec.max(lo);
    if hi <= 0.0 {
        return Duration::ZERO;
    }
    let secs = if (hi - lo).abs() < f32::EPSILON {
        lo
    } else {
        rand::thread_rng().gen_range(lo..=hi)
    };
    Duration::from_secs_f32(secs)
}

async fn run_inner(
    app: &AppHandle,
    state: &State<'_, AppState>,
    keywords: &[String],
    blocks: &[BlockDto],
    delays: &DelaysDto,
    selected_ids: Option<&[String]>,
) -> Result<DoneEvent, String> {
    let contents: Vec<Content> = blocks
        .iter()
        .map(|b| match b {
            BlockDto::Text { content } => Content::text(content.clone()),
            BlockDto::Image { path } => Content::image(path.clone()),
        })
        .collect();

    let kt = state.kt.clone();
    let targets: Vec<Channel> = if let Some(ids) = selected_ids {
        let map = state.channels.lock().await;
        ids.iter()
            .filter_map(|id| map.get(id).cloned())
            .collect()
    } else {
        // direct mode: search now
        let mut acc: HashMap<String, Channel> = HashMap::new();
        emit_log(app, LogLevel::Info, format!("키워드 {}개 · 블럭 {}개로 시작.", keywords.len(), blocks.len()));
        for kw in keywords {
            emit_log(app, LogLevel::Info, format!(r#""{}" 검색 중..."#, kw));
            match kt.search_channels(kw).await {
                Ok(found) => {
                    emit_log(app, LogLevel::Info, format!("{}개 채널 발견.", found.len()));
                    for (id, ch) in found {
                        acc.entry(id).or_insert(ch);
                    }
                }
                Err(e) => emit_log(app, LogLevel::Err, format!("검색 실패: {e}")),
            }
        }
        acc.into_values().collect()
    };

    let mut sent: u32 = 0;
    let mut skipped: u32 = 0;

    if !ensure_auth(app, &kt).await {
        return Err("auth_lost".into());
    }

    let sent_cache_snapshot: HashSet<String> = state.sent_cache.lock().await.clone();

    for (i, ch) in targets.into_iter().enumerate() {
        if state.cancel_flag.load(Ordering::SeqCst) {
            emit_log(app, LogLevel::Info, "사용자에 의해 작업이 중단되었습니다.");
            break;
        }
        // Skip channels we've already messaged in past sessions.
        if sent_cache_snapshot.contains(&ch.id) {
            emit_log(
                app,
                LogLevel::Skip,
                format!("{}: 이미 전송한 채널, 건너뜀.", ch.title),
            );
            skipped += 1;
            continue;
        }
        // Periodically re-verify auth.
        if i > 0 && i % 5 == 0 && !ensure_auth(app, &kt).await {
            break;
        }
        match process_channel(app, &ch, &contents, delays).await {
            ChannelResult::Sent => {
                sent += 1;
                let mut cache = state.sent_cache.lock().await;
                cache.insert(ch.id.clone());
                save_sent_cache(&state.cache_path, &cache);
            }
            ChannelResult::Skipped => skipped += 1,
            ChannelResult::Failed => {}
        }
    }

    emit_log(
        app,
        LogLevel::Ok,
        format!("완료. 전송 {sent}건 · 건너뜀 {skipped}건."),
    );

    let done = DoneEvent { ok: true, sent, skipped };
    let _ = app.emit("done", done.clone());
    Ok(done)
}

enum ChannelResult {
    Sent,
    Skipped,
    Failed,
}

async fn process_channel(
    app: &AppHandle,
    ch: &Channel,
    contents: &[Content],
    delays: &DelaysDto,
) -> ChannelResult {
    if !ch.is_chattable() {
        emit_log(
            app,
            LogLevel::Skip,
            format!("{}: 1:1 문의 미지원, 건너뜀.", ch.title),
        );
        return ChannelResult::Skipped;
    }

    let added = match ch.add().await {
        Ok(v) => v,
        Err(e) => {
            emit_log(app, LogLevel::Err, format!("{}: 친구추가 호출 실패: {e}", ch.title));
            return ChannelResult::Failed;
        }
    };

    if !added {
        emit_log(app, LogLevel::Skip, format!("{}: 친구추가 실패, 건너뜀.", ch.title));
        return ChannelResult::Skipped;
    }

    let post_add = random_delay(delays.add_min, delays.add_max);
    if !post_add.is_zero() {
        emit_log(app, LogLevel::Info, format!("친구추가 후 {:.1}초 대기...", post_add.as_secs_f32()));
        tokio::time::sleep(post_add).await;
    }

    let title = ch.title.clone();
    let ch_clone = ch.clone();
    let contents_clone = contents.to_vec();
    let ok = tokio::task::spawn_blocking(move || ch_clone.send(&contents_clone))
        .await
        .unwrap_or(false);

    if ok {
        emit_log(app, LogLevel::Ok, format!("{}: 메시지 {}개 전송 완료.", title, contents.len()));
        let post_send = random_delay(delays.send_min, delays.send_max);
        if !post_send.is_zero() {
            tokio::time::sleep(post_send).await;
        }
        ChannelResult::Sent
    } else {
        emit_log(app, LogLevel::Err, format!("{}: 메시지 전송 실패.", title));
        ChannelResult::Failed
    }
}

// ─── entry ────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_sent_cache_count(state: State<'_, AppState>) -> Result<usize, String> {
    Ok(state.sent_cache.lock().await.len())
}

#[tauri::command]
async fn clear_sent_cache(state: State<'_, AppState>) -> Result<(), String> {
    let mut cache = state.sent_cache.lock().await;
    cache.clear();
    save_sent_cache(&state.cache_path, &cache);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let cache_path = app
                .path()
                .app_data_dir()
                .map(|d| d.join("sent_channels.json"))
                .unwrap_or_else(|_| PathBuf::from("sent_channels.json"));
            let sent_cache = load_sent_cache(&cache_path);
            app.manage(AppState {
                kt: Arc::new(KakaoTalk::new()),
                running: Mutex::new(false),
                channels: Mutex::new(HashMap::new()),
                sent_cache: Mutex::new(sent_cache),
                cache_path,
                cancel_flag: Arc::new(AtomicBool::new(false)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            check_kakao,
            extract_channels,
            run_automation,
            send_to_selected,
            cancel_work,
            get_sent_cache_count,
            clear_sent_cache
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
