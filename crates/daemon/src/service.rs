use crate::capture::{capture_once, CaptureError, CaptureResult};
use rcopy_core::ClipboardItem;
use rcopy_integration::{ClipboardBackend, ClipboardError, PasteBackend};
use rcopy_storage::{Repository, StorageError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("item not found: {0}")]
    NotFound(Uuid),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("clipboard error: {0}")]
    Clipboard(#[from] ClipboardError),
    #[error("capture error: {0}")]
    Capture(#[from] CaptureError),
    #[error("plain text payload unavailable: {0}")]
    PlainTextUnavailable(Uuid),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureResponse {
    pub captured: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreResponse {
    pub paste_attempted: bool,
    pub paste_succeeded: bool,
}

pub struct DaemonService<C, P> {
    repo: Repository,
    clipboard: C,
    paste: P,
}

impl<C, P> DaemonService<C, P>
where
    C: ClipboardBackend,
    P: PasteBackend,
{
    pub fn new(repo: Repository, clipboard: C, paste: P) -> Self {
        Self {
            repo,
            clipboard,
            paste,
        }
    }

    pub fn search(&self, query: &str) -> Result<Vec<ClipboardItem>, ServiceError> {
        Ok(self.repo.search(query)?)
    }

    pub async fn capture_current(&self) -> Result<CaptureResponse, ServiceError> {
        let result = capture_once(&self.repo, &self.clipboard).await?;
        Ok(CaptureResponse {
            captured: matches!(result, CaptureResult::Stored(_)),
        })
    }

    pub async fn restore(
        &self,
        id: Uuid,
        auto_paste: bool,
        plain_text_only: bool,
    ) -> Result<RestoreResponse, ServiceError> {
        let item = self.repo.get_item(id)?.ok_or(ServiceError::NotFound(id))?;
        let payload = if plain_text_only {
            if item.payload.text_plain.is_none() {
                return Err(ServiceError::PlainTextUnavailable(id));
            }
            rcopy_core::ClipboardPayload {
                text_plain: item.payload.text_plain.clone(),
                text_html: None,
                image_png: None,
            }
        } else {
            item.payload.clone()
        };
        self.clipboard.write_payload(&payload).await?;
        if !self.repo.mark_used(id)? {
            return Err(ServiceError::NotFound(id));
        }

        if !auto_paste {
            return Ok(RestoreResponse {
                paste_attempted: false,
                paste_succeeded: false,
            });
        }

        Ok(RestoreResponse {
            paste_attempted: true,
            paste_succeeded: self.paste.paste().await.is_ok(),
        })
    }

    pub fn soft_delete(&self, id: Uuid) -> Result<(), ServiceError> {
        if self.repo.soft_delete(id)? {
            Ok(())
        } else {
            Err(ServiceError::NotFound(id))
        }
    }

    pub fn set_pinned(&self, id: Uuid, value: bool) -> Result<(), ServiceError> {
        if self.repo.set_pinned(id, value)? {
            Ok(())
        } else {
            Err(ServiceError::NotFound(id))
        }
    }

    pub fn set_favorite(&self, id: Uuid, value: bool) -> Result<(), ServiceError> {
        if self.repo.set_favorite(id, value)? {
            Ok(())
        } else {
            Err(ServiceError::NotFound(id))
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use rcopy_core::{content_hash, ClipboardItem, ClipboardPayload};
    use rcopy_integration::{DisabledPasteBackend, MemoryClipboard};
    use rcopy_storage::Repository;
    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn restore_writes_clipboard_even_when_paste_fails() {
        let repo = Repository::open_in_memory().unwrap();
        let item = make_service_item("alpha");
        let item_id = repo.upsert_item(&item).unwrap();
        let clipboard = MemoryClipboard::new(ClipboardPayload {
            text_plain: None,
            text_html: None,
            image_png: None,
        });
        let service = DaemonService::new(repo, clipboard.clone(), DisabledPasteBackend);

        let response = service.restore(item_id, true, false).await.unwrap();

        assert!(response.paste_attempted);
        assert!(!response.paste_succeeded);
        assert_eq!(
            clipboard.last_written().unwrap().text_plain.as_deref(),
            Some("alpha")
        );
    }

    #[tokio::test]
    async fn restore_updates_last_used_at() {
        let repo = Repository::open_in_memory().unwrap();
        let older = make_service_item("older");
        let newer = make_service_item("newer");
        let older_id = repo.upsert_item(&older).unwrap();
        let newer_id = repo.upsert_item(&newer).unwrap();
        let clipboard = MemoryClipboard::new(ClipboardPayload {
            text_plain: None,
            text_html: None,
            image_png: None,
        });
        let service = DaemonService::new(repo, clipboard, DisabledPasteBackend);

        assert_eq!(service.search("").unwrap()[0].id, newer_id);
        service.restore(older_id, false, false).await.unwrap();

        assert_eq!(service.search("").unwrap()[0].id, older_id);
    }

    #[tokio::test]
    async fn restore_plain_text_only_writes_only_plain_text() {
        let repo = Repository::open_in_memory().unwrap();
        let now = Utc::now();
        let payload = ClipboardPayload {
            text_plain: Some("alpha".to_string()),
            text_html: Some("<b>alpha</b>".to_string()),
            image_png: Some(vec![1, 2, 3]),
        };
        let item = ClipboardItem {
            id: Uuid::new_v4(),
            created_at: now,
            last_used_at: now,
            content_hash: content_hash(&payload),
            mime_types: vec![
                "text/plain".to_string(),
                "text/html".to_string(),
                "image/png".to_string(),
            ],
            payload,
            is_pinned: false,
            is_favorite: false,
            deleted_at: None,
        };
        let item_id = repo.upsert_item(&item).unwrap();
        let clipboard = MemoryClipboard::new(ClipboardPayload {
            text_plain: None,
            text_html: None,
            image_png: None,
        });
        let service = DaemonService::new(repo, clipboard.clone(), DisabledPasteBackend);

        service.restore(item_id, false, true).await.unwrap();

        assert_eq!(
            clipboard.last_written().unwrap(),
            ClipboardPayload {
                text_plain: Some("alpha".to_string()),
                text_html: None,
                image_png: None,
            }
        );
    }

    #[tokio::test]
    async fn restore_plain_text_only_reports_missing_plain_text() {
        let repo = Repository::open_in_memory().unwrap();
        let now = Utc::now();
        let payload = ClipboardPayload {
            text_plain: None,
            text_html: Some("<b>alpha</b>".to_string()),
            image_png: None,
        };
        let item = ClipboardItem {
            id: Uuid::new_v4(),
            created_at: now,
            last_used_at: now,
            content_hash: content_hash(&payload),
            mime_types: vec!["text/html".to_string()],
            payload,
            is_pinned: false,
            is_favorite: false,
            deleted_at: None,
        };
        let item_id = repo.upsert_item(&item).unwrap();
        let clipboard = MemoryClipboard::new(ClipboardPayload {
            text_plain: None,
            text_html: None,
            image_png: None,
        });
        let service = DaemonService::new(repo, clipboard, DisabledPasteBackend);

        assert!(matches!(
            service.restore(item_id, false, true).await,
            Err(ServiceError::PlainTextUnavailable(id)) if id == item_id
        ));
    }

    #[tokio::test]
    async fn service_deletes_pins_favorites_and_searches_items() {
        let repo = Repository::open_in_memory().unwrap();
        let item = make_service_item("alpha");
        let item_id = repo.upsert_item(&item).unwrap();
        let clipboard = MemoryClipboard::new(ClipboardPayload {
            text_plain: None,
            text_html: None,
            image_png: None,
        });
        let service = DaemonService::new(repo, clipboard, DisabledPasteBackend);

        service.set_pinned(item_id, true).unwrap();
        service.set_favorite(item_id, true).unwrap();
        let items = service.search("alpha").unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].is_pinned);
        assert!(items[0].is_favorite);

        service.soft_delete(item_id).unwrap();
        assert!(service.search("alpha").unwrap().is_empty());
    }

    #[test]
    fn item_actions_report_not_found_for_missing_ids() {
        let repo = Repository::open_in_memory().unwrap();
        let clipboard = MemoryClipboard::new(ClipboardPayload {
            text_plain: None,
            text_html: None,
            image_png: None,
        });
        let service = DaemonService::new(repo, clipboard, DisabledPasteBackend);
        let missing = Uuid::new_v4();

        assert!(matches!(
            service.soft_delete(missing),
            Err(ServiceError::NotFound(id)) if id == missing
        ));
        assert!(matches!(
            service.set_pinned(missing, true),
            Err(ServiceError::NotFound(id)) if id == missing
        ));
        assert!(matches!(
            service.set_favorite(missing, true),
            Err(ServiceError::NotFound(id)) if id == missing
        ));
    }

    fn make_service_item(text: &str) -> ClipboardItem {
        let now = Utc::now();
        let payload = ClipboardPayload {
            text_plain: Some(text.to_string()),
            text_html: None,
            image_png: None,
        };

        ClipboardItem {
            id: Uuid::new_v4(),
            created_at: now,
            last_used_at: now,
            content_hash: content_hash(&payload),
            mime_types: vec!["text/plain".to_string()],
            payload,
            is_pinned: false,
            is_favorite: false,
            deleted_at: None,
        }
    }
}
