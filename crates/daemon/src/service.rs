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

    pub async fn restore(
        &self,
        id: Uuid,
        auto_paste: bool,
    ) -> Result<RestoreResponse, ServiceError> {
        let item = self.repo.get_item(id)?.ok_or(ServiceError::NotFound(id))?;
        self.clipboard.write_payload(&item.payload).await?;

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
        Ok(self.repo.soft_delete(id)?)
    }

    pub fn set_pinned(&self, id: Uuid, value: bool) -> Result<(), ServiceError> {
        Ok(self.repo.set_pinned(id, value)?)
    }

    pub fn set_favorite(&self, id: Uuid, value: bool) -> Result<(), ServiceError> {
        Ok(self.repo.set_favorite(id, value)?)
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

        let response = service.restore(item_id, true).await.unwrap();

        assert!(response.paste_attempted);
        assert!(!response.paste_succeeded);
        assert_eq!(
            clipboard.last_written().unwrap().text_plain.as_deref(),
            Some("alpha")
        );
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
