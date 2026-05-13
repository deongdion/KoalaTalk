use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct User {
    pub email: String,
    pub name: String,
    pub profile_url: String,
}

impl User {
    pub fn from_value(data: &Value) -> Self {
        let p = data.get("profile").unwrap_or(data);
        User {
            email: p
                .get("kakaoEmail")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            name: p
                .get("nickName")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            profile_url: p
                .get("imageUrl")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        }
    }
}
