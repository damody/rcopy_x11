use sha2::{Digest, Sha256};

use crate::ClipboardPayload;

pub fn content_hash(payload: &ClipboardPayload) -> String {
    let mut hasher = Sha256::new();
    if let Some(text) = &payload.text_plain {
        update_framed_field(&mut hasher, b"text/plain", text.as_bytes());
    }
    if let Some(html) = &payload.text_html {
        update_framed_field(&mut hasher, b"text/html", html.as_bytes());
    }
    if let Some(image) = &payload.image_png {
        update_framed_field(&mut hasher, b"image/png", image);
    }
    format!("{:x}", hasher.finalize())
}

fn update_framed_field(hasher: &mut Sha256, name: &[u8], value: &[u8]) {
    hasher.update((name.len() as u64).to_le_bytes());
    hasher.update(name);
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value);
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

    #[test]
    fn content_hash_distinguishes_embedded_field_markers() {
        let plain_only = ClipboardPayload {
            text_plain: Some("a\0text/html\0b".into()),
            text_html: None,
            image_png: None,
        };
        let plain_and_html = ClipboardPayload {
            text_plain: Some("a".into()),
            text_html: Some("b".into()),
            image_png: None,
        };

        assert_ne!(content_hash(&plain_only), content_hash(&plain_and_html));
    }
}
