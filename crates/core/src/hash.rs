use sha2::{Digest, Sha256};

use crate::ClipboardPayload;

pub fn content_hash(payload: &ClipboardPayload) -> String {
    let mut hasher = Sha256::new();
    if let Some(text) = &payload.text_plain {
        hasher.update(b"text/plain\0");
        hasher.update(text.as_bytes());
        hasher.update(b"\0");
    }
    if let Some(html) = &payload.text_html {
        hasher.update(b"text/html\0");
        hasher.update(html.as_bytes());
        hasher.update(b"\0");
    }
    if let Some(image) = &payload.image_png {
        hasher.update(b"image/png\0");
        hasher.update(image);
        hasher.update(b"\0");
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClipboardPayload;

    #[test]
    fn text_and_html_hashes_change_when_content_changes() {
        let a = ClipboardPayload {
            text_plain: Some("alpha".into()),
            text_html: Some("<b>alpha</b>".into()),
            image_png: None,
        };
        let b = ClipboardPayload {
            text_plain: Some("beta".into()),
            text_html: Some("<b>alpha</b>".into()),
            image_png: None,
        };
        assert_ne!(content_hash(&a), content_hash(&b));
    }
}
