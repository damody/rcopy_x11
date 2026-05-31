use chrono::{DateTime, Utc};
use rcopy_core::{rank_items, ClipboardItem, ClipboardPayload};
use rusqlite::{params, Connection};
use std::{path::Path, time::Duration};
use thiserror::Error;
use uuid::Uuid;

const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("uuid parse error: {0}")]
    Uuid(#[from] uuid::Error),
    #[error("time parse error: {0}")]
    Time(#[from] chrono::ParseError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct Repository {
    conn: Connection,
}

impl Repository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref();
        let conn = Connection::open(path)?;
        configure_connection(&conn, true)?;
        let repo = Self { conn };
        repo.migrate()?;
        Ok(repo)
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        configure_connection(&conn, false)?;
        let repo = Self { conn };
        repo.migrate()?;
        Ok(repo)
    }

    fn migrate(&self) -> Result<(), StorageError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clipboard_items (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                last_used_at TEXT NOT NULL,
                content_hash TEXT NOT NULL UNIQUE,
                mime_types TEXT NOT NULL,
                text_plain TEXT,
                text_html TEXT,
                image_png BLOB,
                is_pinned INTEGER NOT NULL DEFAULT 0,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                deleted_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_clipboard_items_last_used
            ON clipboard_items(last_used_at DESC);
            CREATE INDEX IF NOT EXISTS idx_clipboard_items_deleted
            ON clipboard_items(deleted_at);",
        )?;
        Ok(())
    }

    pub fn upsert_item(&self, item: &ClipboardItem) -> Result<Uuid, StorageError> {
        let mime_types = serde_json::to_string(&item.mime_types)?;
        let id = self.conn.query_row(
            "INSERT INTO clipboard_items
            (id, created_at, last_used_at, content_hash, mime_types, text_plain, text_html, image_png, is_pinned, is_favorite, deleted_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(content_hash) DO UPDATE SET
                last_used_at = excluded.last_used_at,
                mime_types = excluded.mime_types,
                text_plain = excluded.text_plain,
                text_html = excluded.text_html,
                image_png = excluded.image_png,
                deleted_at = NULL
            RETURNING id",
            params![
                item.id.to_string(),
                item.created_at.to_rfc3339(),
                item.last_used_at.to_rfc3339(),
                item.content_hash,
                mime_types,
                item.payload.text_plain,
                item.payload.text_html,
                item.payload.image_png,
                item.is_pinned as i64,
                item.is_favorite as i64,
                item.deleted_at.map(|time| time.to_rfc3339()),
            ],
            |row| row.get::<_, String>(0),
        )?;
        Ok(Uuid::parse_str(&id)?)
    }

    pub fn get_item(&self, id: Uuid) -> Result<Option<ClipboardItem>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, created_at, last_used_at, content_hash, mime_types, text_plain, text_html, image_png, is_pinned, is_favorite, deleted_at
             FROM clipboard_items WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id.to_string()])?;

        rows.next()?.map(row_to_item).transpose()
    }

    pub fn search(&self, query: &str) -> Result<Vec<ClipboardItem>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, created_at, last_used_at, content_hash, mime_types, text_plain, text_html, image_png, is_pinned, is_favorite, deleted_at
             FROM clipboard_items WHERE deleted_at IS NULL ORDER BY last_used_at DESC LIMIT 500",
        )?;
        let mut rows = stmt.query([])?;
        let mut items = Vec::new();

        while let Some(row) = rows.next()? {
            items.push(row_to_item(row)?);
        }

        Ok(rank_items(query, items))
    }

    pub fn soft_delete(&self, id: Uuid) -> Result<(), StorageError> {
        self.conn.execute(
            "UPDATE clipboard_items SET deleted_at = ?2 WHERE id = ?1",
            params![id.to_string(), Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn set_pinned(&self, id: Uuid, value: bool) -> Result<(), StorageError> {
        self.conn.execute(
            "UPDATE clipboard_items SET is_pinned = ?2 WHERE id = ?1",
            params![id.to_string(), value as i64],
        )?;
        Ok(())
    }

    pub fn set_favorite(&self, id: Uuid, value: bool) -> Result<(), StorageError> {
        self.conn.execute(
            "UPDATE clipboard_items SET is_favorite = ?2 WHERE id = ?1",
            params![id.to_string(), value as i64],
        )?;
        Ok(())
    }
}

fn configure_connection(conn: &Connection, file_backed: bool) -> Result<(), StorageError> {
    conn.busy_timeout(SQLITE_BUSY_TIMEOUT)?;
    if file_backed {
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    }
    Ok(())
}

fn row_to_item(row: &rusqlite::Row<'_>) -> Result<ClipboardItem, StorageError> {
    let id: String = row.get(0)?;
    let created_at: String = row.get(1)?;
    let last_used_at: String = row.get(2)?;
    let mime_types: String = row.get(4)?;
    let deleted_at: Option<String> = row.get(10)?;

    Ok(ClipboardItem {
        id: Uuid::parse_str(&id)?,
        created_at: parse_time(&created_at)?,
        last_used_at: parse_time(&last_used_at)?,
        content_hash: row.get(3)?,
        mime_types: serde_json::from_str(&mime_types)?,
        payload: ClipboardPayload {
            text_plain: row.get(5)?,
            text_html: row.get(6)?,
            image_png: row.get(7)?,
        },
        is_pinned: row.get::<_, i64>(8)? != 0,
        is_favorite: row.get::<_, i64>(9)? != 0,
        deleted_at: deleted_at.as_deref().map(parse_time).transpose()?,
    })
}

fn parse_time(input: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    Ok(DateTime::parse_from_rfc3339(input)?.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use rcopy_core::{content_hash, ClipboardItem, ClipboardPayload};
    use uuid::Uuid;

    use super::Repository;

    #[test]
    fn inserts_and_fetches_clipboard_item() {
        let repo = test_repo();
        let item = make_item("alpha");

        let persisted_id = repo.upsert_item(&item).unwrap();

        let fetched = repo.get_item(persisted_id).unwrap().unwrap();
        assert_eq!(persisted_id, item.id);
        assert_eq!(fetched.id, item.id);
        assert_eq!(fetched.payload.text_plain.as_deref(), Some("alpha"));
    }

    #[test]
    fn duplicate_hash_updates_existing_item_last_used() {
        let repo = test_repo();
        let first = make_item("alpha");
        let mut second = first.clone();
        second.id = Uuid::new_v4();
        second.last_used_at = first.last_used_at + chrono::Duration::seconds(30);

        let first_id = repo.upsert_item(&first).unwrap();
        let second_id = repo.upsert_item(&second).unwrap();

        let items = repo.search("").unwrap();
        let fetched = repo.get_item(second_id).unwrap().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(first_id, first.id);
        assert_eq!(second_id, first.id);
        assert_ne!(second_id, second.id);
        assert_eq!(fetched.id, first.id);
        assert_eq!(items[0].last_used_at, second.last_used_at);
    }

    #[test]
    fn file_backed_connections_have_busy_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path().join("clipboard.db")).unwrap();

        let timeout_ms: i64 = repo
            .conn
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();

        assert!(timeout_ms >= 5_000);
    }

    #[test]
    fn soft_deleted_items_are_hidden_from_search() {
        let repo = test_repo();
        let item = make_item("alpha");

        repo.upsert_item(&item).unwrap();
        repo.soft_delete(item.id).unwrap();

        assert!(repo.search("alpha").unwrap().is_empty());
    }

    #[test]
    fn pin_and_favorite_are_persisted() {
        let repo = test_repo();
        let item = make_item("alpha");

        repo.upsert_item(&item).unwrap();
        repo.set_pinned(item.id, true).unwrap();
        repo.set_favorite(item.id, true).unwrap();

        let fetched = repo.get_item(item.id).unwrap().unwrap();
        assert!(fetched.is_pinned);
        assert!(fetched.is_favorite);
    }

    fn test_repo() -> Repository {
        Repository::open_in_memory().unwrap()
    }

    fn make_item(text: &str) -> ClipboardItem {
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
