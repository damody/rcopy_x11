# rcopy_x11 Usage

## Supported Environment

`rcopy_x11` targets X11 desktop sessions.

The project is a Rust workspace with these runtime pieces:

- `rcopyd`: background daemon for clipboard capture and picker IPC
- `rcopy-picker`: package that builds the GTK picker binary named `rcopy`

## Dependencies

Install the runtime command dependencies:

- `xclip`, used for X11 clipboard reads and writes
- `xdotool`, used only for automatic paste after restore

Install GTK4 and Libadwaita runtime and development packages before building the
picker. Common package names:

- Debian/Ubuntu: `libgtk-4-dev`, `libadwaita-1-dev`
- Fedora: `gtk4-devel`, `libadwaita-devel`
- Arch Linux: `gtk4`, `libadwaita`

## Setup

Run:

```bash
./scripts/setup-x11.sh
```

The script builds the binaries, installs the `rcopyd` user service, and installs
a GNOME/Ubuntu custom shortcut for `Ctrl+\`` when `gsettings` is available.

## Start Manually

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

## Clipboard Behavior

The daemon captures:

- `text/plain`
- `text/html`
- `image/png`

Unsupported MIME targets are ignored. If at least one supported payload is
present, the item is stored. Duplicate payloads are collapsed by content hash and
moved back to the top when seen again.

When restoring, `rcopyd` writes the selected item back with `xclip`. The backend
writes one representation per restore: `image/png` first, `text/plain` second,
and `text/html` last. `Shift+Enter` in the picker requests a plain-text restore
and writes only `text/plain` when the item has that representation.

## Picker Controls

- Type in the search box to filter history.
- `Up` and `Down` move the selected row.
- `Enter` restores the selected item and attempts automatic paste.
- `Shift+Enter` restores the selected item as plain text only.
- `Delete` hides the selected item from history.
- `Ctrl+P` toggles pin.
- `Ctrl+F` toggles favorite.
- `Escape` closes the picker.

Deleted items are soft-deleted and no longer appear in picker results.

## Paste Behavior

Automatic paste uses:

```bash
xdotool key ctrl+v
```

If `xdotool` is missing or exits with an error, rcopy still leaves the restored
content on the clipboard so you can paste manually.

## Manual Clipboard Commands

List X11 clipboard targets:

```bash
xclip -selection clipboard -t TARGETS -o
```

Write plain text to the X11 clipboard:

```bash
printf 'hello' | xclip -selection clipboard -t text/plain -i
```

Read plain text from the X11 clipboard:

```bash
xclip -selection clipboard -t text/plain -o
```

## Not Included

This version does not include groups, cross-device sync, login, accounts, or
cloud storage. Clipboard history is local to the SQLite database used by the
daemon.

## Manual Verification Checklist

1. Start an X11 session.
2. Confirm `xclip` and `xdotool` are available on `PATH`.
3. Run `cargo run -p rcopyd` and leave it running.
4. Copy plain text and open the picker with `cargo run -p rcopy-picker`.
5. Confirm the text item appears and search can find it.
6. Copy rich HTML from a browser and confirm the item appears with text content.
7. Copy a PNG image and confirm an image item appears.
8. Pin and favorite an item with `Ctrl+P` and `Ctrl+F`, then close and reopen
   the picker to confirm the flags persist.
9. Delete an item and confirm it no longer appears in picker results.
10. Select an item with `Enter` and confirm it returns to the clipboard and
    attempts paste.
11. Select a rich text item with `Shift+Enter` and confirm the plain-text version
    is restored.
12. Temporarily run the picker with `xdotool` unavailable on `PATH`, select an
    item, and confirm manual paste still works from the restored clipboard.
