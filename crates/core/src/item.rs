use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardPayload {
    pub text_plain: Option<String>,
    pub text_html: Option<String>,
    pub image_png: Option<Vec<u8>>,
}

impl ClipboardPayload {
    pub fn is_empty(&self) -> bool {
        self.text_plain.is_none() && self.text_html.is_none() && self.image_png.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub content_hash: String,
    pub mime_types: Vec<String>,
    pub payload: ClipboardPayload,
    pub is_pinned: bool,
    pub is_favorite: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
pub fn sample_item(
    text: &str,
    is_pinned: bool,
    is_favorite: bool,
    age_seconds: i64,
) -> ClipboardItem {
    let now = Utc::now();
    let payload = ClipboardPayload {
        text_plain: Some(text.to_string()),
        text_html: None,
        image_png: None,
    };
    ClipboardItem {
        id: Uuid::new_v4(),
        created_at: now - chrono::Duration::seconds(age_seconds),
        last_used_at: now - chrono::Duration::seconds(age_seconds),
        content_hash: crate::content_hash(&payload),
        mime_types: vec!["text/plain".to_string()],
        payload,
        is_pinned,
        is_favorite,
        deleted_at: None,
    }
}
