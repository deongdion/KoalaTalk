use std::collections::HashMap;

use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

use crate::models::content::Content;

static ENCODED_ID_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([@_][A-Za-z0-9]+)").unwrap());

#[derive(Debug, Clone, Default)]
pub struct Channel {
    pub id: String,
    pub title: String,
    pub friend_count: i64,
    pub url: String,
    pub image_url: String,
    pub uuid: String,
    pub encoded_id: String,
    pub official_badge: bool,
    pub business_badge: bool,
    pub consult: bool,
    pub chat_bot: bool,
    pub normal: bool,
    pub order_bot: bool,
    pub shopping_bot: bool,
    pub booking_bot: bool,
    pub added_channel: bool,
    pub reservation: bool,
    pub coupon_title: String,
    pub location_address: String,
    pub location_x: f64,
    pub location_y: f64,
    pub news_title: String,
    pub news_url: String,
    pub authorization: String,
    pub cookies: HashMap<String, String>,
}

fn s(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn b(v: &Value, key: &str) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn i(v: &Value, key: &str) -> i64 {
    v.get(key).and_then(Value::as_i64).unwrap_or(0)
}

fn f(v: &Value, key: &str) -> f64 {
    v.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

impl Channel {
    pub fn extract_encoded_id(url: &str) -> String {
        let trimmed = url.trim();
        match ENCODED_ID_RE.captures(trimmed) {
            Some(c) => c.get(1).unwrap().as_str().to_string(),
            None => trimmed.to_string(),
        }
    }

    pub fn from_simple(
        encoded_id: &str,
        data: &Value,
        authorization: &str,
        cookies: &HashMap<String, String>,
    ) -> Option<Channel> {
        let profile = data.get("profile")?;
        let title = profile
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if title.is_empty() {
            return None;
        }
        let consult = profile
            .get("consult")
            .and_then(|c| c.get("consult_flag"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let id = profile
            .get("id")
            .map(|v| match v {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => encoded_id.to_string(),
            })
            .unwrap_or_else(|| encoded_id.to_string());
        Some(Channel {
            id,
            title,
            friend_count: 0,
            url: format!("https://pf.kakao.com/{}", encoded_id),
            image_url: String::new(),
            uuid: s(profile, "uuid"),
            encoded_id: encoded_id.to_string(),
            official_badge: false,
            business_badge: false,
            consult,
            chat_bot: false,
            normal: true,
            order_bot: false,
            shopping_bot: false,
            booking_bot: false,
            added_channel: false,
            reservation: false,
            coupon_title: String::new(),
            location_address: String::new(),
            location_x: 0.0,
            location_y: 0.0,
            news_title: String::new(),
            news_url: String::new(),
            authorization: authorization.to_string(),
            cookies: cookies.clone(),
        })
    }

    pub fn from_search_doc(doc: &Value) -> Option<Channel> {
        let channel = doc.get("attr")?.get("channel")?;
        let info = channel.get("info")?;
        let module = channel.get("module").cloned().unwrap_or(Value::Null);
        let coupon = module.get("coupon").cloned().unwrap_or(Value::Null);
        let location = module.get("location").cloned().unwrap_or(Value::Null);
        let news = module.get("news").cloned().unwrap_or(Value::Null);

        let url = info
            .get("link")
            .and_then(|l| l.get("url"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let image_url = info
            .get("image")
            .and_then(|im| im.get("url"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let news_url = news
            .get("link")
            .and_then(|l| l.get("url"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        Some(Channel {
            id: s(info, "id"),
            title: s(info, "title"),
            friend_count: i(info, "friendCount"),
            url,
            image_url,
            uuid: s(info, "uuid"),
            encoded_id: s(info, "encodedId"),
            official_badge: b(info, "officialBadge"),
            business_badge: b(info, "businessBadge"),
            consult: b(info, "consult"),
            chat_bot: b(info, "chatBot"),
            normal: b(info, "normal"),
            order_bot: b(info, "orderBot"),
            shopping_bot: b(info, "shoppingBot"),
            booking_bot: b(info, "bookingBot"),
            added_channel: b(info, "addedChannel"),
            reservation: b(info, "reservation"),
            coupon_title: s(&coupon, "title"),
            location_address: s(&location, "address"),
            location_x: f(&location, "x"),
            location_y: f(&location, "y"),
            news_title: s(&news, "title"),
            news_url,
            authorization: String::new(),
            cookies: HashMap::new(),
        })
    }

    pub fn is_chattable(&self) -> bool {
        self.consult
    }

    pub fn send(&self, contents: &[Content], window_open_secs: f32, paste_delay_secs: f32) -> bool {
        use crate::modules::windows::{open_chatroom, send_to_chatroom};

        match open_chatroom(&self.title, window_open_secs) {
            Some((search_box, kt_hwnd, chatroom_hwnd)) => {
                send_to_chatroom(&self.title, contents, search_box, kt_hwnd, chatroom_hwnd, paste_delay_secs)
            }
            None => false,
        }
    }

    pub async fn add(&self) -> Result<bool> {
        let mut cookie_header = String::new();
        for (k, v) in &self.cookies {
            if !cookie_header.is_empty() {
                cookie_header.push_str("; ");
            }
            cookie_header.push_str(k);
            cookie_header.push('=');
            cookie_header.push_str(v);
        }

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36")
            .build()?;

        let url = format!(
            "https://pf.kakao.com/rocket-web/web/profiles/{}/friend/add",
            self.encoded_id
        );
        let referer = format!("https://pf.kakao.com/{}", self.encoded_id);

        let resp = client
            .put(&url)
            .query(&[
                ("is_welcome_coupon", "false"),
                ("web_referrer", "home_etc_web"),
            ])
            .header("Accept", "*/*")
            .header("Accept-Language", "ko-KR,ko;q=0.9")
            .header("Cache-Control", "no-cache")
            .header("Origin", "https://pf.kakao.com")
            .header("Referer", referer)
            .header("Cookie", cookie_header)
            .send()
            .await?;

        let json: Value = resp.json().await?;
        Ok(json
            .get("is_friend")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    }
}
