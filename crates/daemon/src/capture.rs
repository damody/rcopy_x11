use chrono::Utc;
use rcopy_core::{content_hash, ClipboardItem};
use rcopy_integration::{ClipboardBackend, ClipboardError};
use rcopy_storage::{Repository, StorageError};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureResult {
    Stored(Uuid),
    IgnoredEmpty,
}

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("clipboard error: {0}")]
    Clipboard(#[from] ClipboardError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
}

pub async fn capture_once<C>(
    repo: &Repository,
    clipboard: &C,
) -> Result<CaptureResult, CaptureError>
where
    C: ClipboardBackend,
{
    let payload = clipboard.read_supported().await?;
    if payload.is_empty() {
        return Ok(CaptureResult::IgnoredEmpty);
    }

    let now = Utc::now();
    let item = ClipboardItem {
        id: Uuid::new_v4(),
        created_at: now,
        last_used_at: now,
        content_hash: content_hash(&payload),
        mime_types: mime_types_for(&payload),
        payload,
        is_pinned: false,
        is_favorite: false,
        deleted_at: None,
    };
    let persisted_id = repo.upsert_item(&item)?;

    Ok(CaptureResult::Stored(persisted_id))
}

pub async fn monitor_tick<C>(repo: &Repository, clipboard: &C) -> Result<(), CaptureError>
where
    C: ClipboardBackend,
{
    let _ = capture_once(repo, clipboard).await?;
    Ok(())
}

pub async fn monitor_loop<C>(repo: Repository, clipboard: C, interval: std::time::Duration) -> !
where
    C: ClipboardBackend,
{
    loop {
        if let Err(error) = monitor_tick(&repo, &clipboard).await {
            eprintln!("rcopyd capture warning: {error}");
        }
        tokio::time::sleep(interval).await;
    }
}

fn mime_types_for(payload: &rcopy_core::ClipboardPayload) -> Vec<String> {
    let mut mime_types = Vec::new();
    if payload.text_plain.is_some() {
        mime_types.push("text/plain".to_string());
    }
    if payload.text_html.is_some() {
        mime_types.push("text/html".to_string());
    }
    if payload.image_png.is_some() {
        mime_types.push("image/png".to_string());
    }
    mime_types
}

#[cfg(test)]
mod tests {
    use rcopy_core::ClipboardPayload;
    use rcopy_integration::MemoryClipboard;
    use rcopy_storage::Repository;

    use super::*;

    #[tokio::test]
    async fn capture_ignores_empty_payloads() {
        let repo = Repository::open_in_memory().unwrap();
        let clipboard = MemoryClipboard::new(ClipboardPayload {
            text_plain: None,
            text_html: None,
            image_png: None,
        });

        let result = capture_once(&repo, &clipboard).await.unwrap();

        assert_eq!(result, CaptureResult::IgnoredEmpty);
    }

    #[tokio::test]
    async fn capture_returns_persisted_id_for_duplicate_payloads() {
        let repo = Repository::open_in_memory().unwrap();
        let clipboard = MemoryClipboard::new(ClipboardPayload {
            text_plain: Some("alpha".into()),
            text_html: None,
            image_png: None,
        });

        let first = capture_once(&repo, &clipboard).await.unwrap();
        let second = capture_once(&repo, &clipboard).await.unwrap();

        assert_eq!(repo.search("alpha").unwrap().len(), 1);
        assert_eq!(second, first);
    }

    #[tokio::test]
    async fn monitor_tick_captures_once() {
        let repo = Repository::open_in_memory().unwrap();
        let clipboard = MemoryClipboard::new(ClipboardPayload {
            text_plain: Some("alpha".into()),
            text_html: None,
            image_png: None,
        });

        monitor_tick(&repo, &clipboard).await.unwrap();

        assert_eq!(repo.search("alpha").unwrap().len(), 1);
    }
}
