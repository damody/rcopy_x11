# rcopy Wayland Clipboard Manager Design

Date: 2026-05-31

## Goal

Build `rcopy`, a local-first clipboard manager inspired by Ditto's daily workflow, but designed for Wayland on Hyprland/wlroots.

The first version should let the user copy content normally, keep a searchable clipboard history, open a quick picker with a Hyprland key binding, choose an item, restore it to the clipboard, and attempt to paste it back into the previously focused application.

## Target Environment

- Linux Wayland session
- Hyprland or another wlroots-based compositor
- Rust application stack
- GTK4/Libadwaita quick picker UI
- SQLite local database
- `wl-clipboard` for clipboard read/write integration
- `wtype` for optional simulated paste

GNOME and KDE compatibility is not a first-version requirement.

## Non-Goals

The first version will not include:

- Clipboard groups
- Multi-device sync
- Login or account features
- Cloud storage
- Telemetry
- Plugin support
- Full Ditto feature parity

## User Workflow

1. The user copies text, HTML, or an image in any Wayland application.
2. `rcopyd` detects the clipboard update, reads supported MIME types, deduplicates the item, and writes it to SQLite.
3. The user presses a Hyprland binding such as `SUPER+V`.
4. The `rcopy` quick picker opens with search focused.
5. The user filters and selects an item using the keyboard or mouse.
6. The selected item is restored to the clipboard.
7. The paste backend attempts to invoke paste through `wtype`.
8. If automatic paste fails, the picker closes and the selected content remains on the clipboard for manual paste.

## Architecture

### `rcopyd` Daemon

`rcopyd` is the long-running background process. It owns clipboard capture, database writes, and local IPC commands.

Responsibilities:

- Monitor clipboard changes.
- Read supported MIME types: `text/plain`, `text/html`, and `image/png`.
- Calculate content hashes and avoid duplicate rows.
- Store metadata, searchable text, and binary payload references.
- Serve picker queries over local IPC.
- Handle item actions: copy, delete, pin, favorite, and update last-used time.
- Call clipboard and paste backends through clear interfaces.

### `rcopy` Quick Picker

`rcopy` is the graphical picker opened by the compositor key binding.

Responsibilities:

- Query recent and pinned history from the daemon.
- Provide fast search.
- Show item type, preview text, timestamp, pinned state, and favorite state.
- Preview image and HTML content when available.
- Support keyboard-first interaction.
- Send selected item actions to the daemon.

Expected controls:

- Type to search.
- Up/down to move selection.
- Enter to restore and paste.
- Shift+Enter to restore plain text when available.
- Delete to remove the selected item.
- Pin and favorite actions through shortcuts and visible controls.
- Escape to close.

### Storage Layer

SQLite stores item metadata, searchable content, and first-version binary payloads. PNG image payloads are stored as SQLite BLOBs to keep backup, deletion, and transactional behavior simple. The storage API must hide that detail from the UI and daemon flow so a later file-backed implementation can be added without changing callers.

Core item fields:

- `id`
- `created_at`
- `last_used_at`
- `content_hash`
- `mime_types`
- `text_plain`
- `text_html`
- `image_png`
- `is_pinned`
- `is_favorite`
- `deleted_at`

Deleted items should be soft-deleted first so accidental destructive behavior is avoidable during early development.

### Integration Backends

Backends are small replaceable interfaces.

Clipboard backend:

- Read current clipboard MIME types.
- Read supported content payloads.
- Write a selected item back to the clipboard.

Paste backend:

- Attempt automatic paste using `wtype`.
- Return a typed failure when paste cannot be attempted.
- Never treat paste failure as failure to select an item if the clipboard write succeeded.

This keeps Hyprland-specific behavior isolated from database and UI code.

## Error Handling

The application should degrade without interrupting normal clipboard use.

- Clipboard read failure: log a warning and continue monitoring.
- Unsupported MIME type: ignore that type and keep any supported payloads.
- SQLite write failure: log an error; show a concise picker error if the user action depends on it.
- Paste backend failure: keep selected content on the clipboard and optionally show a short status message.
- Database corruption: stop writes, report the issue, and avoid deleting user data automatically.

## Configuration

Use a TOML config file for the first version instead of a settings UI.

Initial settings:

- Database path
- Maximum history size or retention days
- Enable or disable automatic paste
- Paste command override
- Capture text, HTML, and image toggles

Hyprland key binding is documented as a manual config snippet instead of managed by the app.

## Testing Strategy

Unit tests:

- Content hashing and deduplication
- Database CRUD and soft delete behavior
- Search ranking and filtering
- Config parsing
- MIME selection rules

Integration tests:

- Daemon IPC command handling
- Clipboard backend mock read/write flow
- Paste backend mock success and failure
- Picker action-to-daemon command mapping where practical

Manual Hyprland verification:

- Text capture
- HTML capture
- Image capture
- Search
- Delete
- Pin
- Favorite
- Enter restore and paste
- Shift+Enter plain text restore
- Missing `wtype` fallback

## Implementation Notes

Use conservative dependencies and keep the initial codebase modular:

- `crates/core` for domain types, config, hashing, and search behavior
- `crates/storage` for SQLite persistence
- `crates/daemon` for clipboard monitoring and IPC
- `crates/picker` for GTK UI
- `crates/integration` for clipboard and paste backends

If a workspace split is too heavy during scaffolding, keep the same module boundaries inside fewer crates, but preserve the interfaces.

## Success Criteria

The first version is successful when, on Hyprland/wlroots, the user can:

- Run `rcopyd` in the background.
- Copy text, HTML, and PNG image content.
- Open the picker with a compositor key binding.
- Search and select clipboard history.
- Pin, favorite, and delete items.
- Restore selected content to the clipboard.
- Automatically paste through `wtype` when supported.
- Fall back cleanly to manual paste when automatic paste fails.
