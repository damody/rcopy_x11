# rcopy Wayland Clipboard Manager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a local-first Ditto-inspired clipboard manager for Hyprland/wlroots Wayland with searchable history, text/HTML/PNG capture, pin/favorite/delete, and optional `wtype` paste.

**Architecture:** Use a Rust workspace split by responsibility: domain logic in `rcopy-core`, SQLite persistence in `rcopy-storage`, Wayland command integrations in `rcopy-integration`, background capture and IPC in `rcopyd`, and the GTK4 picker in `rcopy-picker`. Keep all external Wayland command execution behind traits so daemon behavior can be tested with mocks.

**Tech Stack:** Rust 2021, Cargo workspace, `anyhow`, `thiserror`, `serde`, `toml`, `chrono`, `uuid`, `sha2`, `rusqlite`, `tokio`, Unix sockets, GTK4/Libadwaita, `wl-clipboard`, `wtype`, SQLite.

---

## File Structure

- `Cargo.toml`: workspace members and shared package metadata.
- `.gitignore`: Rust build artifacts, local database files, editor noise.
- `crates/core`: shared types, config parsing, content hashing, MIME selection, search ranking.
- `crates/storage`: SQLite schema and repository API for clipboard items.
- `crates/integration`: clipboard and paste backend traits plus `wl-clipboard`/`wtype` command implementations.
- `crates/daemon`: `rcopyd` binary, clipboard monitor loop, IPC server, capture orchestration.
- `crates/picker`: `rcopy` GTK4 binary, picker view model, daemon IPC client.
- `docs/usage.md`: setup, Hyprland binding, runtime dependencies, manual verification.

## Task 1: Scaffold Rust Workspace

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `crates/core/Cargo.toml`
- Create: `crates/core/src/lib.rs`
- Create: `crates/storage/Cargo.toml`
- Create: `crates/storage/src/lib.rs`
- Create: `crates/integration/Cargo.toml`
- Create: `crates/integration/src/lib.rs`
- Create: `crates/daemon/Cargo.toml`
- Create: `crates/daemon/src/main.rs`
- Create: `crates/picker/Cargo.toml`
- Create: `crates/picker/src/main.rs`

- [ ] **Step 1: Create workspace manifests and starter crates**

Create `Cargo.toml`:

```toml
[workspace]
members = [
  "crates/core",
  "crates/storage",
  "crates/integration",
  "crates/daemon",
  "crates/picker",
]
resolver = "2"

[workspace.package]
edition = "2021"
license = "MIT"
version = "0.1.0"

[workspace.dependencies]
anyhow = "1"
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde", "clock"] }
glib = "0.19"
gtk4 = { version = "0.8", package = "gtk4" }
libadwaita = { version = "0.6", package = "libadwaita" }
rusqlite = { version = "0.31", features = ["bundled", "chrono", "blob"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
tempfile = "3"
thiserror = "1"
tokio = { version = "1", features = ["full"] }
toml = "0.8"
uuid = { version = "1", features = ["v4", "serde"] }
```

Create `.gitignore`:

```gitignore
/target/
*.db
*.db-shm
*.db-wal
.env
```

Create `crates/core/Cargo.toml`:

```toml
[package]
name = "rcopy-core"
edition.workspace = true
license.workspace = true
version.workspace = true

[dependencies]
chrono.workspace = true
serde.workspace = true
sha2.workspace = true
thiserror.workspace = true
toml.workspace = true
uuid.workspace = true
```

Create `crates/core/src/lib.rs`:

```rust
pub fn crate_ready() -> bool {
    true
}
```

Create `crates/storage/Cargo.toml`:

```toml
[package]
name = "rcopy-storage"
edition.workspace = true
license.workspace = true
version.workspace = true

[dependencies]
chrono.workspace = true
rcopy-core = { path = "../core" }
rusqlite.workspace = true
thiserror.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

Create `crates/storage/src/lib.rs`:

```rust
pub fn crate_ready() -> bool {
    true
}
```

Create `crates/integration/Cargo.toml`:

```toml
[package]
name = "rcopy-integration"
edition.workspace = true
license.workspace = true
version.workspace = true

[dependencies]
anyhow.workspace = true
async-trait.workspace = true
rcopy-core = { path = "../core" }
thiserror.workspace = true
tokio.workspace = true
```

Create `crates/integration/src/lib.rs`:

```rust
pub fn crate_ready() -> bool {
    true
}
```

Create `crates/daemon/Cargo.toml`:

```toml
[package]
name = "rcopyd"
edition.workspace = true
license.workspace = true
version.workspace = true

[[bin]]
name = "rcopyd"
path = "src/main.rs"

[dependencies]
anyhow.workspace = true
rcopy-core = { path = "../core" }
rcopy-integration = { path = "../integration" }
rcopy-storage = { path = "../storage" }
serde_json.workspace = true
tokio.workspace = true
```

Create `crates/daemon/src/main.rs`:

```rust
fn main() {
    println!("rcopyd scaffold");
}
```

Create `crates/picker/Cargo.toml`:

```toml
[package]
name = "rcopy-picker"
edition.workspace = true
license.workspace = true
version.workspace = true

[[bin]]
name = "rcopy"
path = "src/main.rs"

[dependencies]
anyhow.workspace = true
glib.workspace = true
gtk4.workspace = true
libadwaita.workspace = true
rcopy-core = { path = "../core" }
serde_json.workspace = true
tokio.workspace = true
```

Create `crates/picker/src/main.rs`:

```rust
fn main() {
    println!("rcopy picker scaffold");
}
```

- [ ] **Step 2: Verify workspace compiles**

Run:

```bash
cargo test --workspace
```

Expected:

```text
test result: ok
```

- [ ] **Step 3: Commit scaffold**

```bash
git add Cargo.toml .gitignore crates
git commit -m "chore: scaffold Rust workspace"
```

## Task 2: Core Domain Types, MIME Selection, Hashing, Search, and Config

**Files:**
- Modify: `crates/core/src/lib.rs`
- Create: `crates/core/src/config.rs`
- Create: `crates/core/src/hash.rs`
- Create: `crates/core/src/item.rs`
- Create: `crates/core/src/mime.rs`
- Create: `crates/core/src/search.rs`

- [ ] **Step 1: Write core tests**

Create module tests inside the new files:

```rust
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
fn supported_mime_types_are_selected_in_stable_order() {
    let offered = vec![
        "image/jpeg".to_string(),
        "text/html".to_string(),
        "text/plain;charset=utf-8".to_string(),
        "image/png".to_string(),
    ];
    assert_eq!(
        select_supported_mimes(&offered),
        vec![MimeKind::TextPlain, MimeKind::TextHtml, MimeKind::ImagePng]
    );
}

#[test]
fn search_prefers_pinned_then_favorite_then_recent_matches() {
    let recent = sample_item("recent alpha", false, false, 30);
    let favorite = sample_item("favorite alpha", false, true, 20);
    let pinned = sample_item("pinned alpha", true, false, 10);
    let ranked = rank_items("alpha", vec![recent.clone(), favorite.clone(), pinned.clone()]);
    assert_eq!(ranked[0].id, pinned.id);
    assert_eq!(ranked[1].id, favorite.id);
    assert_eq!(ranked[2].id, recent.id);
}

#[test]
fn config_defaults_capture_text_html_and_images() {
    let config = AppConfig::default();
    assert!(config.capture_text);
    assert!(config.capture_html);
    assert!(config.capture_images);
    assert!(config.auto_paste);
    assert_eq!(config.paste_command, "wtype");
}
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```bash
cargo test -p rcopy-core
```

Expected: compile failures for undefined `ClipboardPayload`, `content_hash`, `MimeKind`, `select_supported_mimes`, `sample_item`, `rank_items`, and `AppConfig`.

- [ ] **Step 3: Implement core modules**

`crates/core/src/lib.rs`:

```rust
pub mod config;
pub mod hash;
pub mod item;
pub mod mime;
pub mod search;

pub use config::AppConfig;
pub use hash::content_hash;
pub use item::{ClipboardItem, ClipboardPayload};
pub use mime::{select_supported_mimes, MimeKind};
pub use search::rank_items;
```

`crates/core/src/item.rs`:

```rust
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
pub fn sample_item(text: &str, is_pinned: bool, is_favorite: bool, age_seconds: i64) -> ClipboardItem {
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
```

`crates/core/src/hash.rs`:

```rust
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
```

`crates/core/src/mime.rs`:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MimeKind {
    TextPlain,
    TextHtml,
    ImagePng,
}

pub fn select_supported_mimes(offered: &[String]) -> Vec<MimeKind> {
    let has_text = offered.iter().any(|m| m.starts_with("text/plain"));
    let has_html = offered.iter().any(|m| m == "text/html");
    let has_png = offered.iter().any(|m| m == "image/png");
    let mut selected = Vec::new();
    if has_text {
        selected.push(MimeKind::TextPlain);
    }
    if has_html {
        selected.push(MimeKind::TextHtml);
    }
    if has_png {
        selected.push(MimeKind::ImagePng);
    }
    selected
}
```

`crates/core/src/search.rs`:

```rust
use crate::ClipboardItem;

pub fn rank_items(query: &str, mut items: Vec<ClipboardItem>) -> Vec<ClipboardItem> {
    let needle = query.trim().to_lowercase();
    items.retain(|item| {
        item.deleted_at.is_none()
            && (needle.is_empty()
                || item
                    .payload
                    .text_plain
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&needle)
                || item
                    .payload
                    .text_html
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&needle))
    });
    items.sort_by(|a, b| {
        b.is_pinned
            .cmp(&a.is_pinned)
            .then_with(|| b.is_favorite.cmp(&a.is_favorite))
            .then_with(|| b.last_used_at.cmp(&a.last_used_at))
    });
    items
}
```

`crates/core/src/config.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub database_path: PathBuf,
    pub max_history_items: usize,
    pub auto_paste: bool,
    pub paste_command: String,
    pub capture_text: bool,
    pub capture_html: bool,
    pub capture_images: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database_path: PathBuf::from("rcopy.db"),
            max_history_items: 10_000,
            auto_paste: true,
            paste_command: "wtype".to_string(),
            capture_text: true,
            capture_html: true,
            capture_images: true,
        }
    }
}

impl AppConfig {
    pub fn from_toml(input: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(input)
    }
}
```

- [ ] **Step 4: Run core tests**

```bash
cargo test -p rcopy-core
```

Expected:

```text
test result: ok
```

- [ ] **Step 5: Commit core domain**

```bash
git add crates/core
git commit -m "feat: add core clipboard domain"
```

## Task 3: SQLite Storage Repository

**Files:**
- Modify: `crates/storage/src/lib.rs`
- Create: `crates/storage/src/repository.rs`

- [ ] **Step 1: Write storage tests**

Create tests in `crates/storage/src/repository.rs`:

```rust
#[test]
fn inserts_and_fetches_clipboard_item() {
    let repo = test_repo();
    let item = make_item("alpha");
    repo.upsert_item(&item).unwrap();
    let fetched = repo.get_item(item.id).unwrap().unwrap();
    assert_eq!(fetched.id, item.id);
    assert_eq!(fetched.payload.text_plain.as_deref(), Some("alpha"));
}

#[test]
fn duplicate_hash_updates_existing_item_last_used() {
    let repo = test_repo();
    let first = make_item("alpha");
    let mut second = first.clone();
    second.last_used_at = first.last_used_at + chrono::Duration::seconds(30);
    repo.upsert_item(&first).unwrap();
    repo.upsert_item(&second).unwrap();
    let items = repo.search("").unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].last_used_at, second.last_used_at);
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
```

- [ ] **Step 2: Run tests and verify they fail**

```bash
cargo test -p rcopy-storage
```

Expected: compile failures for undefined `Repository`, `upsert_item`, `get_item`, `search`, `soft_delete`, `set_pinned`, and `set_favorite`.

- [ ] **Step 3: Implement repository**

`crates/storage/src/lib.rs`:

```rust
pub mod repository;

pub use repository::{Repository, StorageError};
```

`crates/storage/src/repository.rs`:

```rust
use chrono::{DateTime, Utc};
use rcopy_core::{rank_items, ClipboardItem, ClipboardPayload};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("uuid parse error: {0}")]
    Uuid(#[from] uuid::Error),
    #[error("time parse error: {0}")]
    Time(#[from] chrono::ParseError),
}

pub struct Repository {
    conn: Connection,
}

impl Repository {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
        let repo = Self { conn };
        repo.migrate()?;
        Ok(repo)
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
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

    pub fn upsert_item(&self, item: &ClipboardItem) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO clipboard_items
            (id, created_at, last_used_at, content_hash, mime_types, text_plain, text_html, image_png, is_pinned, is_favorite, deleted_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(content_hash) DO UPDATE SET
              last_used_at = excluded.last_used_at,
              mime_types = excluded.mime_types,
              text_plain = excluded.text_plain,
              text_html = excluded.text_html,
              image_png = excluded.image_png,
              deleted_at = NULL",
            params![
                item.id.to_string(),
                item.created_at.to_rfc3339(),
                item.last_used_at.to_rfc3339(),
                item.content_hash,
                item.mime_types.join("\n"),
                item.payload.text_plain,
                item.payload.text_html,
                item.payload.image_png,
                item.is_pinned as i64,
                item.is_favorite as i64,
                item.deleted_at.map(|t| t.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    pub fn get_item(&self, id: Uuid) -> Result<Option<ClipboardItem>, StorageError> {
        self.conn
            .query_row(
                "SELECT id, created_at, last_used_at, content_hash, mime_types, text_plain, text_html, image_png, is_pinned, is_favorite, deleted_at
                 FROM clipboard_items WHERE id = ?1",
                params![id.to_string()],
                row_to_item,
            )
            .optional()
            .map_err(StorageError::from)
    }

    pub fn search(&self, query: &str) -> Result<Vec<ClipboardItem>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, created_at, last_used_at, content_hash, mime_types, text_plain, text_html, image_png, is_pinned, is_favorite, deleted_at
             FROM clipboard_items WHERE deleted_at IS NULL ORDER BY last_used_at DESC LIMIT 500",
        )?;
        let items = stmt
            .query_map([], row_to_item)?
            .collect::<Result<Vec<_>, _>>()?;
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

fn row_to_item(row: &rusqlite::Row<'_>) -> Result<ClipboardItem, rusqlite::Error> {
    let id: String = row.get(0)?;
    let created_at: String = row.get(1)?;
    let last_used_at: String = row.get(2)?;
    let mime_types: String = row.get(4)?;
    let deleted_at: Option<String> = row.get(10)?;
    Ok(ClipboardItem {
        id: Uuid::parse_str(&id).map_err(to_sql_error)?,
        created_at: parse_time(&created_at).map_err(to_sql_error)?,
        last_used_at: parse_time(&last_used_at).map_err(to_sql_error)?,
        content_hash: row.get(3)?,
        mime_types: mime_types.lines().map(str::to_string).collect(),
        payload: ClipboardPayload {
            text_plain: row.get(5)?,
            text_html: row.get(6)?,
            image_png: row.get(7)?,
        },
        is_pinned: row.get::<_, i64>(8)? != 0,
        is_favorite: row.get::<_, i64>(9)? != 0,
        deleted_at: deleted_at
            .as_deref()
            .map(parse_time)
            .transpose()
            .map_err(to_sql_error)?,
    })
}

fn parse_time(input: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    Ok(DateTime::parse_from_rfc3339(input)?.with_timezone(&Utc))
}

fn to_sql_error<E>(err: E) -> rusqlite::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    rusqlite::Error::ToSqlConversionFailure(Box::new(err))
}

#[cfg(test)]
fn test_repo() -> Repository {
    Repository::open_in_memory().unwrap()
}

#[cfg(test)]
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
        content_hash: rcopy_core::content_hash(&payload),
        mime_types: vec!["text/plain".to_string()],
        payload,
        is_pinned: false,
        is_favorite: false,
        deleted_at: None,
    }
}
```

- [ ] **Step 4: Add missing `uuid` dependency to storage**

Modify `crates/storage/Cargo.toml`:

```toml
uuid.workspace = true
```

- [ ] **Step 5: Run storage tests**

```bash
cargo test -p rcopy-storage
```

Expected:

```text
test result: ok
```

- [ ] **Step 6: Commit storage**

```bash
git add crates/storage
git commit -m "feat: add SQLite clipboard storage"
```

## Task 4: Integration Backends for Clipboard and Paste

**Files:**
- Modify: `crates/integration/src/lib.rs`
- Create: `crates/integration/src/clipboard.rs`
- Create: `crates/integration/src/paste.rs`

- [ ] **Step 1: Write integration tests using test doubles**

Create tests in `crates/integration/src/clipboard.rs` and `crates/integration/src/paste.rs`:

```rust
#[tokio::test]
async fn mock_clipboard_round_trips_payload() {
    let payload = ClipboardPayload {
        text_plain: Some("alpha".into()),
        text_html: Some("<b>alpha</b>".into()),
        image_png: Some(vec![137, 80, 78, 71]),
    };
    let clipboard = MemoryClipboard::new(payload.clone());
    assert_eq!(clipboard.read_supported().await.unwrap(), payload);
    clipboard.write_payload(&payload).await.unwrap();
    assert_eq!(clipboard.last_written().unwrap(), payload);
}

#[tokio::test]
async fn disabled_paste_backend_reports_not_available() {
    let paste = DisabledPasteBackend;
    let error = paste.paste().await.unwrap_err();
    assert!(matches!(error, PasteError::Unavailable(_)));
}
```

- [ ] **Step 2: Run tests and verify they fail**

```bash
cargo test -p rcopy-integration
```

Expected: compile failures for undefined clipboard and paste backend traits/types.

- [ ] **Step 3: Implement backend traits and command backends**

`crates/integration/src/lib.rs`:

```rust
pub mod clipboard;
pub mod paste;

pub use clipboard::{ClipboardBackend, ClipboardError, MemoryClipboard, WlClipboard};
pub use paste::{DisabledPasteBackend, PasteBackend, PasteError, WtypePasteBackend};
```

`crates/integration/src/clipboard.rs`:

```rust
use async_trait::async_trait;
use rcopy_core::ClipboardPayload;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("clipboard command failed: {0}")]
    Command(String),
    #[error("clipboard state unavailable")]
    StateUnavailable,
}

#[async_trait]
pub trait ClipboardBackend: Send + Sync {
    async fn read_supported(&self) -> Result<ClipboardPayload, ClipboardError>;
    async fn write_payload(&self, payload: &ClipboardPayload) -> Result<(), ClipboardError>;
}

#[derive(Clone)]
pub struct MemoryClipboard {
    current: Arc<Mutex<ClipboardPayload>>,
    written: Arc<Mutex<Option<ClipboardPayload>>>,
}

impl MemoryClipboard {
    pub fn new(payload: ClipboardPayload) -> Self {
        Self {
            current: Arc::new(Mutex::new(payload)),
            written: Arc::new(Mutex::new(None)),
        }
    }

    pub fn last_written(&self) -> Result<ClipboardPayload, ClipboardError> {
        self.written
            .lock()
            .map_err(|_| ClipboardError::StateUnavailable)?
            .clone()
            .ok_or(ClipboardError::StateUnavailable)
    }
}

#[async_trait]
impl ClipboardBackend for MemoryClipboard {
    async fn read_supported(&self) -> Result<ClipboardPayload, ClipboardError> {
        self.current
            .lock()
            .map_err(|_| ClipboardError::StateUnavailable)
            .map(|payload| payload.clone())
    }

    async fn write_payload(&self, payload: &ClipboardPayload) -> Result<(), ClipboardError> {
        *self
            .written
            .lock()
            .map_err(|_| ClipboardError::StateUnavailable)? = Some(payload.clone());
        Ok(())
    }
}

pub struct WlClipboard;

#[async_trait]
impl ClipboardBackend for WlClipboard {
    async fn read_supported(&self) -> Result<ClipboardPayload, ClipboardError> {
        let text_plain = read_mime("text/plain").await.ok();
        let text_html = read_mime("text/html").await.ok();
        let image_png = read_bytes("image/png").await.ok();
        Ok(ClipboardPayload {
            text_plain,
            text_html,
            image_png,
        })
    }

    async fn write_payload(&self, payload: &ClipboardPayload) -> Result<(), ClipboardError> {
        if let Some(html) = &payload.text_html {
            write_mime("text/html", html.as_bytes()).await?;
        }
        if let Some(image) = &payload.image_png {
            write_mime("image/png", image).await?;
        }
        if let Some(text) = &payload.text_plain {
            write_mime("text/plain", text.as_bytes()).await?;
        }
        Ok(())
    }
}

async fn read_mime(mime: &str) -> Result<String, ClipboardError> {
    let output = Command::new("wl-paste")
        .args(["--no-newline", "--type", mime])
        .output()
        .await
        .map_err(|e| ClipboardError::Command(e.to_string()))?;
    if !output.status.success() {
        return Err(ClipboardError::Command(format!("wl-paste failed for {mime}")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

async fn read_bytes(mime: &str) -> Result<Vec<u8>, ClipboardError> {
    let output = Command::new("wl-paste")
        .args(["--type", mime])
        .output()
        .await
        .map_err(|e| ClipboardError::Command(e.to_string()))?;
    if !output.status.success() {
        return Err(ClipboardError::Command(format!("wl-paste failed for {mime}")));
    }
    Ok(output.stdout)
}

async fn write_mime(mime: &str, bytes: &[u8]) -> Result<(), ClipboardError> {
    let mut child = Command::new("wl-copy")
        .args(["--type", mime])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| ClipboardError::Command(e.to_string()))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| ClipboardError::Command("wl-copy stdin unavailable".into()))?
        .write_all(bytes)
        .await
        .map_err(|e| ClipboardError::Command(e.to_string()))?;
    let status = child
        .wait()
        .await
        .map_err(|e| ClipboardError::Command(e.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        Err(ClipboardError::Command(format!("wl-copy failed for {mime}")))
    }
}
```

`crates/integration/src/paste.rs`:

```rust
use async_trait::async_trait;
use thiserror::Error;
use tokio::process::Command;

#[derive(Debug, Error)]
pub enum PasteError {
    #[error("paste backend unavailable: {0}")]
    Unavailable(String),
    #[error("paste command failed: {0}")]
    Command(String),
}

#[async_trait]
pub trait PasteBackend: Send + Sync {
    async fn paste(&self) -> Result<(), PasteError>;
}

pub struct DisabledPasteBackend;

#[async_trait]
impl PasteBackend for DisabledPasteBackend {
    async fn paste(&self) -> Result<(), PasteError> {
        Err(PasteError::Unavailable("automatic paste disabled".into()))
    }
}

pub struct WtypePasteBackend {
    command: String,
}

impl WtypePasteBackend {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }
}

#[async_trait]
impl PasteBackend for WtypePasteBackend {
    async fn paste(&self) -> Result<(), PasteError> {
        let status = Command::new(&self.command)
            .args(["-M", "ctrl", "v", "-m", "ctrl"])
            .status()
            .await
            .map_err(|e| PasteError::Command(e.to_string()))?;
        if status.success() {
            Ok(())
        } else {
            Err(PasteError::Command(format!("{} exited with {status}", self.command)))
        }
    }
}
```

- [ ] **Step 4: Run integration tests**

```bash
cargo test -p rcopy-integration
```

Expected:

```text
test result: ok
```

- [ ] **Step 5: Commit integration backends**

```bash
git add crates/integration
git commit -m "feat: add clipboard and paste backends"
```

## Task 5: Daemon Capture Service and IPC Protocol

**Files:**
- Modify: `crates/daemon/Cargo.toml`
- Modify: `crates/daemon/src/main.rs`
- Create: `crates/daemon/src/capture.rs`
- Create: `crates/daemon/src/ipc.rs`
- Create: `crates/daemon/src/service.rs`

- [ ] **Step 1: Write daemon unit tests**

Create tests in `crates/daemon/src/capture.rs` and `crates/daemon/src/service.rs`:

```rust
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
async fn restore_writes_clipboard_even_when_paste_fails() {
    let repo = Repository::open_in_memory().unwrap();
    let item = make_service_item("alpha");
    repo.upsert_item(&item).unwrap();
    let clipboard = MemoryClipboard::new(ClipboardPayload {
        text_plain: None,
        text_html: None,
        image_png: None,
    });
    let service = DaemonService::new(repo, clipboard.clone(), DisabledPasteBackend);
    let response = service.restore(item.id, true).await.unwrap();
    assert_eq!(response.paste_attempted, true);
    assert_eq!(response.paste_succeeded, false);
    assert_eq!(clipboard.last_written().unwrap().text_plain.as_deref(), Some("alpha"));
}
```

- [ ] **Step 2: Run tests and verify they fail**

```bash
cargo test -p rcopyd
```

Expected: compile failures for undefined `capture_once`, `CaptureResult`, `DaemonService`, and `restore`.

- [ ] **Step 3: Implement capture and service modules**

`crates/daemon/src/main.rs`:

```rust
mod capture;
mod ipc;
mod service;

use anyhow::Result;
use rcopy_core::AppConfig;
use rcopy_integration::{WlClipboard, WtypePasteBackend};
use rcopy_storage::Repository;
use service::DaemonService;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::default();
    let repo = Repository::open(&config.database_path)?;
    let clipboard = WlClipboard;
    let paste = WtypePasteBackend::new(config.paste_command.clone());
    let service = DaemonService::new(repo, clipboard, paste);
    ipc::serve_default_socket(service).await
}
```

`crates/daemon/src/capture.rs`:

```rust
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

pub async fn capture_once<C>(repo: &Repository, clipboard: &C) -> Result<CaptureResult, CaptureError>
where
    C: ClipboardBackend,
{
    let payload = clipboard.read_supported().await?;
    if payload.is_empty() {
        return Ok(CaptureResult::IgnoredEmpty);
    }
    let now = Utc::now();
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
    let item = ClipboardItem {
        id: Uuid::new_v4(),
        created_at: now,
        last_used_at: now,
        content_hash: content_hash(&payload),
        mime_types,
        payload,
        is_pinned: false,
        is_favorite: false,
        deleted_at: None,
    };
    repo.upsert_item(&item)?;
    Ok(CaptureResult::Stored(item.id))
}
```

`crates/daemon/src/service.rs`:

```rust
use rcopy_core::ClipboardItem;
use rcopy_integration::{ClipboardBackend, ClipboardError, PasteBackend, PasteError};
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
    #[error("paste error: {0}")]
    Paste(#[from] PasteError),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
        Self { repo, clipboard, paste }
    }

    pub fn search(&self, query: &str) -> Result<Vec<ClipboardItem>, ServiceError> {
        Ok(self.repo.search(query)?)
    }

    pub async fn restore(&self, id: Uuid, auto_paste: bool) -> Result<RestoreResponse, ServiceError> {
        let item = self.repo.get_item(id)?.ok_or(ServiceError::NotFound(id))?;
        self.clipboard.write_payload(&item.payload).await?;
        if !auto_paste {
            return Ok(RestoreResponse {
                paste_attempted: false,
                paste_succeeded: false,
            });
        }
        let paste_succeeded = self.paste.paste().await.is_ok();
        Ok(RestoreResponse {
            paste_attempted: true,
            paste_succeeded,
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
fn make_service_item(text: &str) -> ClipboardItem {
    let now = chrono::Utc::now();
    let payload = rcopy_core::ClipboardPayload {
        text_plain: Some(text.to_string()),
        text_html: None,
        image_png: None,
    };
    ClipboardItem {
        id: Uuid::new_v4(),
        created_at: now,
        last_used_at: now,
        content_hash: rcopy_core::content_hash(&payload),
        mime_types: vec!["text/plain".to_string()],
        payload,
        is_pinned: false,
        is_favorite: false,
        deleted_at: None,
    }
}
```

`crates/daemon/src/ipc.rs`:

```rust
use anyhow::Result;
use rcopy_integration::{ClipboardBackend, PasteBackend};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use uuid::Uuid;

use crate::service::DaemonService;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum IpcRequest {
    Search { query: String },
    Restore { id: Uuid, auto_paste: bool },
    Delete { id: Uuid },
    Pin { id: Uuid, value: bool },
    Favorite { id: Uuid, value: bool },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum IpcResponse<T> {
    Ok { value: T },
    Err { message: String },
}

pub async fn serve_default_socket<C, P>(service: DaemonService<C, P>) -> Result<()>
where
    C: ClipboardBackend + 'static,
    P: PasteBackend + 'static,
{
    let path = "/tmp/rcopyd.sock";
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)?;
    loop {
        let (stream, _) = listener.accept().await?;
        handle_client(stream, &service).await?;
    }
}

async fn handle_client<C, P>(stream: UnixStream, service: &DaemonService<C, P>) -> Result<()>
where
    C: ClipboardBackend,
    P: PasteBackend,
{
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    let request: IpcRequest = serde_json::from_str(&line)?;
    let response = match request {
        IpcRequest::Search { query } => serde_json::to_string(&IpcResponse::Ok {
            value: service.search(&query)?,
        })?,
        IpcRequest::Restore { id, auto_paste } => serde_json::to_string(&IpcResponse::Ok {
            value: service.restore(id, auto_paste).await?,
        })?,
        IpcRequest::Delete { id } => {
            service.soft_delete(id)?;
            serde_json::to_string(&IpcResponse::Ok { value: () })?
        }
        IpcRequest::Pin { id, value } => {
            service.set_pinned(id, value)?;
            serde_json::to_string(&IpcResponse::Ok { value: () })?
        }
        IpcRequest::Favorite { id, value } => {
            service.set_favorite(id, value)?;
            serde_json::to_string(&IpcResponse::Ok { value: () })?
        }
    };
    let stream = reader.get_mut();
    stream.write_all(response.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    Ok(())
}
```

- [ ] **Step 4: Add daemon dependencies**

Modify `crates/daemon/Cargo.toml` dependencies:

```toml
chrono.workspace = true
thiserror.workspace = true
uuid.workspace = true
```

- [ ] **Step 5: Run daemon tests**

```bash
cargo test -p rcopyd
```

Expected:

```text
test result: ok
```

- [ ] **Step 6: Commit daemon service**

```bash
git add crates/daemon
git commit -m "feat: add daemon service and IPC protocol"
```

## Task 6: Clipboard Monitor Loop

**Files:**
- Modify: `crates/daemon/src/capture.rs`
- Modify: `crates/daemon/src/main.rs`

- [ ] **Step 1: Add monitor loop test with a single injected tick**

Add to `crates/daemon/src/capture.rs`:

```rust
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
```

- [ ] **Step 2: Run test and verify it fails**

```bash
cargo test -p rcopyd monitor_tick_captures_once
```

Expected: compile failure for undefined `monitor_tick`.

- [ ] **Step 3: Implement monitor tick and polling loop**

Add to `crates/daemon/src/capture.rs`:

```rust
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
```

Modify `crates/daemon/src/main.rs` to spawn capture beside IPC:

```rust
mod capture;
mod ipc;
mod service;

use anyhow::Result;
use rcopy_core::AppConfig;
use rcopy_integration::{WlClipboard, WtypePasteBackend};
use rcopy_storage::Repository;
use service::DaemonService;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::default();
    let capture_repo = Repository::open(&config.database_path)?;
    let service_repo = Repository::open(&config.database_path)?;
    tokio::spawn(capture::monitor_loop(
        capture_repo,
        WlClipboard,
        Duration::from_millis(750),
    ));
    let service = DaemonService::new(
        service_repo,
        WlClipboard,
        WtypePasteBackend::new(config.paste_command.clone()),
    );
    ipc::serve_default_socket(service).await
}
```

- [ ] **Step 4: Run daemon tests**

```bash
cargo test -p rcopyd
```

Expected:

```text
test result: ok
```

- [ ] **Step 5: Commit monitor loop**

```bash
git add crates/daemon
git commit -m "feat: monitor clipboard in daemon"
```

## Task 7: Picker IPC Client and View Model

**Files:**
- Modify: `crates/picker/Cargo.toml`
- Create: `crates/picker/src/ipc_client.rs`
- Create: `crates/picker/src/view_model.rs`
- Modify: `crates/picker/src/main.rs`

- [ ] **Step 1: Write view model tests**

Create tests in `crates/picker/src/view_model.rs`:

```rust
#[test]
fn selected_index_moves_within_bounds() {
    let mut model = PickerModel::new(vec![item("a"), item("b")]);
    model.move_down();
    model.move_down();
    assert_eq!(model.selected_index, 1);
    model.move_up();
    model.move_up();
    assert_eq!(model.selected_index, 0);
}

#[test]
fn selected_item_returns_current_item() {
    let mut model = PickerModel::new(vec![item("a"), item("b")]);
    model.move_down();
    assert_eq!(model.selected_item().unwrap().payload.text_plain.as_deref(), Some("b"));
}
```

- [ ] **Step 2: Run tests and verify they fail**

```bash
cargo test -p rcopy-picker
```

Expected: compile failures for undefined `PickerModel`.

- [ ] **Step 3: Implement picker view model and IPC client**

`crates/picker/src/view_model.rs`:

```rust
use rcopy_core::ClipboardItem;

#[derive(Clone, Debug)]
pub struct PickerModel {
    pub items: Vec<ClipboardItem>,
    pub selected_index: usize,
}

impl PickerModel {
    pub fn new(items: Vec<ClipboardItem>) -> Self {
        Self {
            items,
            selected_index: 0,
        }
    }

    pub fn move_down(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.items.len() - 1);
        }
    }

    pub fn move_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn selected_item(&self) -> Option<&ClipboardItem> {
        self.items.get(self.selected_index)
    }
}

#[cfg(test)]
fn item(text: &str) -> ClipboardItem {
    let now = chrono::Utc::now();
    let payload = rcopy_core::ClipboardPayload {
        text_plain: Some(text.to_string()),
        text_html: None,
        image_png: None,
    };
    ClipboardItem {
        id: uuid::Uuid::new_v4(),
        created_at: now,
        last_used_at: now,
        content_hash: rcopy_core::content_hash(&payload),
        mime_types: vec!["text/plain".into()],
        payload,
        is_pinned: false,
        is_favorite: false,
        deleted_at: None,
    }
}
```

`crates/picker/src/ipc_client.rs`:

```rust
use anyhow::Result;
use rcopy_core::ClipboardItem;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum IpcRequest {
    Search { query: String },
    Restore { id: Uuid, auto_paste: bool },
    Delete { id: Uuid },
    Pin { id: Uuid, value: bool },
    Favorite { id: Uuid, value: bool },
}

#[derive(Debug, Deserialize)]
pub struct RestoreResponse {
    pub paste_attempted: bool,
    pub paste_succeeded: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum IpcResponse<T> {
    Ok { value: T },
    Err { message: String },
}

pub struct IpcClient {
    socket_path: String,
}

impl IpcClient {
    pub fn new(socket_path: impl Into<String>) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<ClipboardItem>> {
        self.request(IpcRequest::Search {
            query: query.to_string(),
        })
        .await
    }

    pub async fn restore(&self, id: Uuid, auto_paste: bool) -> Result<RestoreResponse> {
        self.request(IpcRequest::Restore { id, auto_paste }).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        self.request(IpcRequest::Delete { id }).await
    }

    pub async fn pin(&self, id: Uuid, value: bool) -> Result<()> {
        self.request(IpcRequest::Pin { id, value }).await
    }

    pub async fn favorite(&self, id: Uuid, value: bool) -> Result<()> {
        self.request(IpcRequest::Favorite { id, value }).await
    }

    async fn request<T>(&self, request: IpcRequest) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut stream = UnixStream::connect(&self.socket_path).await?;
        let mut encoded = serde_json::to_vec(&request)?;
        encoded.push(b'\n');
        stream.write_all(&encoded).await?;
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        match serde_json::from_str::<IpcResponse<T>>(&line)? {
            IpcResponse::Ok { value } => Ok(value),
            IpcResponse::Err { message } => anyhow::bail!(message),
        }
    }
}
```

Modify `crates/picker/src/main.rs`:

```rust
mod ipc_client;
mod view_model;

fn main() {
    rcopy_picker_main();
}

fn rcopy_picker_main() {
    println!("rcopy picker");
}
```

- [ ] **Step 4: Add picker dependencies**

Modify `crates/picker/Cargo.toml` dependencies:

```toml
chrono.workspace = true
serde = { workspace = true }
uuid.workspace = true
```

- [ ] **Step 5: Run picker tests**

```bash
cargo test -p rcopy-picker
```

Expected:

```text
test result: ok
```

- [ ] **Step 6: Commit picker model and IPC client**

```bash
git add crates/picker
git commit -m "feat: add picker IPC client and model"
```

## Task 8: GTK4 Quick Picker UI

**Files:**
- Modify: `crates/picker/src/main.rs`
- Create: `crates/picker/src/ui.rs`

- [ ] **Step 1: Implement GTK application shell**

Create `crates/picker/src/ui.rs`:

```rust
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Entry, Label, ListBox, Orientation};
use libadwaita as adw;

pub fn run() {
    adw::init().expect("libadwaita initializes");
    let app = Application::builder()
        .application_id("dev.rcopy.picker")
        .build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let search = Entry::builder()
        .placeholder_text("Search clipboard")
        .hexpand(true)
        .build();
    let list = ListBox::builder().vexpand(true).build();
    let empty = Label::new(Some("No clipboard items"));
    list.append(&empty);
    let root = GtkBox::new(Orientation::Vertical, 8);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);
    root.append(&search);
    root.append(&list);
    let window = ApplicationWindow::builder()
        .application(app)
        .title("rcopy")
        .default_width(720)
        .default_height(520)
        .child(&root)
        .build();
    window.present();
    search.grab_focus();
}
```

Modify `crates/picker/src/main.rs`:

```rust
mod ipc_client;
mod ui;
mod view_model;

fn main() {
    ui::run();
}
```

- [ ] **Step 2: Verify picker binary builds**

```bash
cargo build -p rcopy-picker
```

Expected:

```text
Finished
```

- [ ] **Step 3: Add list rendering helper**

Extend `crates/picker/src/ui.rs`:

```rust
use rcopy_core::ClipboardItem;

fn preview_text(item: &ClipboardItem) -> String {
    if let Some(text) = &item.payload.text_plain {
        return text.lines().next().unwrap_or("").chars().take(160).collect();
    }
    if item.payload.text_html.is_some() {
        return "[HTML content]".to_string();
    }
    if item.payload.image_png.is_some() {
        return "[PNG image]".to_string();
    }
    "[Unsupported content]".to_string()
}

fn item_label(item: &ClipboardItem) -> Label {
    let prefix = match (item.is_pinned, item.is_favorite) {
        (true, true) => "[Pinned] [Favorite] ",
        (true, false) => "[Pinned] ",
        (false, true) => "[Favorite] ",
        (false, false) => "",
    };
    Label::new(Some(&format!("{prefix}{}", preview_text(item))))
}
```

- [ ] **Step 4: Add UI smoke test for pure preview function**

Add to `crates/picker/src/ui.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_text_prefers_plain_text() {
        let now = chrono::Utc::now();
        let payload = rcopy_core::ClipboardPayload {
            text_plain: Some("alpha\nbeta".into()),
            text_html: Some("<b>alpha</b>".into()),
            image_png: None,
        };
        let item = ClipboardItem {
            id: uuid::Uuid::new_v4(),
            created_at: now,
            last_used_at: now,
            content_hash: rcopy_core::content_hash(&payload),
            mime_types: vec!["text/plain".into(), "text/html".into()],
            payload,
            is_pinned: false,
            is_favorite: false,
            deleted_at: None,
        };
        assert_eq!(preview_text(&item), "alpha");
    }
}
```

- [ ] **Step 5: Run picker tests and build**

```bash
cargo test -p rcopy-picker
cargo build -p rcopy-picker
```

Expected:

```text
test result: ok
Finished
```

- [ ] **Step 6: Commit GTK shell**

```bash
git add crates/picker
git commit -m "feat: add GTK quick picker shell"
```

## Task 9: Picker Actions for Restore, Delete, Pin, and Favorite

**Files:**
- Modify: `crates/picker/src/ui.rs`
- Modify: `crates/picker/src/view_model.rs`

- [ ] **Step 1: Add action state tests**

Add to `crates/picker/src/view_model.rs`:

```rust
#[test]
fn remove_selected_item_updates_selection() {
    let mut model = PickerModel::new(vec![item("a"), item("b")]);
    model.move_down();
    let removed = model.remove_selected().unwrap();
    assert_eq!(removed.payload.text_plain.as_deref(), Some("b"));
    assert_eq!(model.selected_index, 0);
    assert_eq!(model.items.len(), 1);
}

#[test]
fn toggle_selected_pin_flips_state() {
    let mut model = PickerModel::new(vec![item("a")]);
    model.toggle_selected_pin();
    assert!(model.selected_item().unwrap().is_pinned);
}
```

- [ ] **Step 2: Run tests and verify they fail**

```bash
cargo test -p rcopy-picker remove_selected_item_updates_selection toggle_selected_pin_flips_state
```

Expected: compile failures for undefined `remove_selected` and `toggle_selected_pin`.

- [ ] **Step 3: Implement action state helpers**

Add to `impl PickerModel` in `crates/picker/src/view_model.rs`:

```rust
pub fn remove_selected(&mut self) -> Option<ClipboardItem> {
    if self.items.is_empty() {
        return None;
    }
    let removed = self.items.remove(self.selected_index);
    if self.selected_index >= self.items.len() {
        self.selected_index = self.items.len().saturating_sub(1);
    }
    Some(removed)
}

pub fn toggle_selected_pin(&mut self) {
    if let Some(item) = self.items.get_mut(self.selected_index) {
        item.is_pinned = !item.is_pinned;
    }
}

pub fn toggle_selected_favorite(&mut self) {
    if let Some(item) = self.items.get_mut(self.selected_index) {
        item.is_favorite = !item.is_favorite;
    }
}
```

- [ ] **Step 4: Wire GTK keyboard actions**

Extend `crates/picker/src/ui.rs` with a key controller:

```rust
use gtk4::{EventControllerKey, gdk};

fn attach_keyboard_shortcuts(window: &ApplicationWindow) {
    let controller = EventControllerKey::new();
    let window_for_keys = window.clone();
    controller.connect_key_pressed(move |_, key, _, _| {
        match key {
            gdk::Key::Escape => {
                window_for_keys.close();
                true
            }
            _ => false,
        }
    });
    window.add_controller(controller);
}
```

Call it in `build_ui` before `window.present()`:

```rust
attach_keyboard_shortcuts(&window);
```

- [ ] **Step 5: Run picker tests and build**

```bash
cargo test -p rcopy-picker
cargo build -p rcopy-picker
```

Expected:

```text
test result: ok
Finished
```

- [ ] **Step 6: Commit picker actions**

```bash
git add crates/picker
git commit -m "feat: add picker action state"
```

## Task 10: Runtime Documentation and Hyprland Setup

**Files:**
- Create: `README.md`
- Create: `docs/usage.md`

- [ ] **Step 1: Write README**

Create `README.md`:

````markdown
# rcopy

`rcopy` is a local-first clipboard manager for Hyprland/wlroots Wayland sessions.

It captures text, HTML, and PNG clipboard content, keeps searchable local history in SQLite, and provides a GTK quick picker. Selecting an item restores it to the clipboard and can attempt automatic paste through `wtype`.

## Runtime dependencies

- `wl-clipboard`
- `wtype`
- GTK4
- Libadwaita

## First-run flow

Start the daemon:

```bash
cargo run -p rcopyd
```

Open the picker:

```bash
cargo run -p rcopy-picker
```

## Hyprland binding

Add a binding like this to `hyprland.conf`:

```text
bind = SUPER, V, exec, cargo run --manifest-path /home/damody/work/rcopy/Cargo.toml -p rcopy-picker
```
````

Create `docs/usage.md`:

````markdown
# rcopy Usage

## Supported first-version environment

`rcopy` targets Hyprland and wlroots-based Wayland sessions. GNOME and KDE are outside the first-version support target.

## Clipboard behavior

The daemon captures:

- `text/plain`
- `text/html`
- `image/png`

Unsupported MIME types are ignored. If at least one supported payload is present, the item is stored.

## Paste behavior

When an item is selected, `rcopyd` writes it back to the clipboard. If automatic paste is enabled, it runs:

```bash
wtype -M ctrl v -m ctrl
```

If `wtype` fails, the selected content remains on the clipboard for manual paste.

## Manual verification

1. Start `rcopyd`.
2. Copy plain text and confirm it appears in the picker.
3. Copy rich HTML from a browser and confirm text/HTML metadata is stored.
4. Copy a PNG image and confirm the item appears as image content.
5. Search for a text item.
6. Pin and favorite an item.
7. Delete an item and confirm it is hidden.
8. Select an item and confirm it returns to the clipboard.
9. Temporarily remove `wtype` from `PATH` and confirm manual paste fallback still works.
````

- [ ] **Step 2: Commit docs**

```bash
git add README.md docs/usage.md
git commit -m "docs: add rcopy usage guide"
```

## Task 11: Whole-Workspace Verification

**Files:**
- Modify only if verification reveals a concrete compile or test failure.

- [ ] **Step 1: Format**

```bash
cargo fmt --all
```

Expected: command exits successfully.

- [ ] **Step 2: Run tests**

```bash
cargo test --workspace
```

Expected:

```text
test result: ok
```

- [ ] **Step 3: Build binaries**

```bash
cargo build --workspace
```

Expected:

```text
Finished
```

- [ ] **Step 4: Inspect git status**

```bash
git status --short
```

Expected: either empty output, or only intentional formatting/build-fix changes.

- [ ] **Step 5: Commit verification fixes if any**

If Step 4 shows source changes from formatting or compile fixes:

```bash
git add Cargo.toml crates README.md docs
git commit -m "chore: verify workspace"
```

If Step 4 is clean, do not create an empty commit.

## Spec Coverage Review

- Clipboard history capture: Tasks 2, 4, 5, and 6.
- Text, HTML, and PNG support: Tasks 2, 3, and 4.
- SQLite storage and soft delete: Task 3.
- Search, pin, favorite, delete: Tasks 2, 3, 5, 7, and 9.
- Hyprland/wlroots integration through `wl-clipboard` and `wtype`: Tasks 4, 5, 6, and 10.
- GTK4/Libadwaita picker: Tasks 7, 8, and 9.
- Fallback when automatic paste fails: Tasks 4 and 5.
- TOML config defaults: Task 2.
- Manual verification documentation: Task 10.

## Known Execution Notes

- GTK and Libadwaita development packages must be installed on the machine before `rcopy-picker` can compile.
- The command integration uses `wl-paste`, `wl-copy`, and `wtype`; tests rely on mocks and do not require a live Wayland session.
- The initial daemon monitor uses polling. It is deliberately isolated in `capture.rs` so it can be replaced by event-based clipboard watching without touching storage, IPC, or picker code.
