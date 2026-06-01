# rcopy Usage

## Supported Environment

`rcopy` targets Hyprland and wlroots-based Wayland sessions. GNOME and KDE are
outside the first-version support target.

The project is a Rust workspace with these runtime pieces:

- `rcopyd`: background daemon for clipboard capture and picker IPC
- `rcopy-picker`: package that builds the GTK picker binary

## Dependencies

Install the runtime command dependencies:

- `wl-clipboard`, which provides `wl-paste` and `wl-copy`
- `wtype`, used only for automatic paste after restore

Install GTK4 and Libadwaita runtime and development packages before building the
picker. Common package names:

- Debian/Ubuntu: `libgtk-4-dev`, `libadwaita-1-dev`
- Fedora: `gtk4-devel`, `libadwaita-devel`
- Arch Linux: `gtk4`, `libadwaita`

## Start rcopy

Start the daemon in one terminal:

```bash
cargo run -p rcopyd
```

Start the picker in another terminal:

```bash
cargo run -p rcopy-picker
```

The daemon stores history in `rcopy.db` in the current working directory. The
daemon and picker communicate over the default socket path
`$XDG_RUNTIME_DIR/rcopy/rcopyd.sock`. If `XDG_RUNTIME_DIR` is unavailable, the
fallback is a user-scoped temp directory like `/tmp/rcopy-$UID/rcopyd.sock`.

## Hyprland Binding

Add a Ctrl+backtick binding like this to `hyprland.conf`:

```text
bind = CTRL, grave, exec, cargo run --manifest-path /home/damody/work/rcopy/Cargo.toml -p rcopy-picker
```

Run `cargo run -p rcopyd` from the repo before using the binding. A session
manager or Hyprland `exec-once` entry can keep the daemon running once you choose
how you want to install it.

## Clipboard Behavior

The daemon polls the clipboard and captures:

- `text/plain`
- `text/html`
- `image/png`

Unsupported MIME types are ignored. If at least one supported payload is present,
the item is stored. Duplicate payloads are collapsed by content hash and moved
back to the top when seen again.

When restoring, `rcopyd` writes the selected item back with `wl-copy`. The current
backend writes one representation per restore: `image/png` first, `text/html`
second, and `text/plain` last. `Shift+Enter` in the picker requests a plain-text
restore and writes only `text/plain` when the item has that representation.

## Picker Controls

- Type in the search box to filter history.
- `Up` and `Down` move the selected row.
- `Enter` restores the selected item and attempts automatic paste.
- `Shift+Enter` restores the selected item as plain text only.
- `Delete` hides the selected item from history.
- `Ctrl+P` toggles pin.
- `Ctrl+F` toggles favorite.
- `Escape` closes the picker.

Rows also include Pin and Favorite buttons. Deleted items are soft-deleted and no
longer appear in picker results.

## Paste Behavior

Automatic paste uses:

```bash
wtype -M ctrl -k v -m ctrl
```

If `wtype` is missing, unavailable to the compositor, or exits with an error,
rcopy still leaves the restored content on the clipboard so you can paste
manually.

## Not Included

This version does not include groups, cross-device sync, login, accounts, or
cloud storage. Clipboard history is local to the SQLite database used by the
daemon.

## Manual Verification Checklist

1. Start a Hyprland or wlroots-based Wayland session.
2. Confirm `wl-paste`, `wl-copy`, and `wtype` are available on `PATH`.
3. Run `cargo run -p rcopyd` and leave it running.
4. Copy plain text and open the picker with `cargo run -p rcopy-picker`.
5. Confirm the text item appears and search can find it.
6. Copy rich HTML from a browser and confirm the item appears with HTML/text
   content available.
7. Copy a PNG image and confirm an image item appears.
8. Pin and favorite an item, then close and reopen the picker to confirm the
   flags persist.
9. Delete an item and confirm it no longer appears in picker results.
10. Select an item with `Enter` and confirm it returns to the clipboard and
    attempts paste.
11. Select a rich text item with `Shift+Enter` and confirm the plain-text version
    is restored.
12. Temporarily run the picker with `wtype` unavailable on `PATH`, select an
    item, and confirm manual paste still works from the restored clipboard.
