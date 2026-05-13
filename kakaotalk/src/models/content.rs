#[derive(Debug, Clone)]
pub enum Content {
    Text(String),
    Image(String),
}

impl Content {
    pub fn text<S: Into<String>>(s: S) -> Self {
        Content::Text(s.into())
    }

    pub fn image<S: Into<String>>(path: S) -> Self {
        Content::Image(path.into())
    }
}
