# rcopy_x11

`rcopy_x11` is a local-first clipboard manager for X11 sessions.

It captures supported clipboard content into a local SQLite database, exposes a
Unix-socket daemon, and provides a GTK4/Libadwaita picker for searching,
restoring, pinning, favoriting, and deleting clipboard history. Restoring an item
writes it back to the X11 clipboard and can attempt an automatic paste with
`xdotool`.

This version is intentionally local only. It does not include groups, device
sync, login, accounts, or cloud storage.

## Dependencies

Runtime commands:

- `xclip` for reading and writing the X11 clipboard
- `xdotool` for optional automatic paste
- GTK4 runtime libraries
- Libadwaita runtime libraries

Build dependencies:

- Rust and Cargo
- GTK4 development package
- Libadwaita development package

Common package names are `libgtk-4-dev` and `libadwaita-1-dev` on Debian/Ubuntu,
`gtk4-devel` and `libadwaita-devel` on Fedora, and `gtk4` and `libadwaita` on
Arch Linux.

## Setup

Run the setup script from the repository root:

```bash
./scripts/setup-x11.sh
```

The script builds the binaries, installs the `rcopyd` user service, and installs
a GNOME/Ubuntu custom shortcut when `gsettings` is available:

```text
Ctrl+`
```

The shortcut runs `target/debug/rcopy`.

## Run Manually

Start the daemon:

```bash
cargo run -p rcopyd
```

Open the picker:

```bash
cargo run -p rcopy-picker
```

Both processes use the same default socket path:
`$XDG_RUNTIME_DIR/rcopy/rcopyd.sock`. If `XDG_RUNTIME_DIR` is not set, rcopy uses
a user-scoped temp path like `/tmp/rcopy-$UID/rcopyd.sock`.

## Clipboard Support

The daemon captures these MIME types when the X11 clipboard exposes them:

- `text/plain`
- `text/html`
- `image/png`

Unsupported MIME types are ignored. If at least one supported payload is present,
the item is stored. Restore currently writes one best representation through
`xclip`: PNG first, then plain text, then HTML.

In the picker, `Enter` restores the selected item and attempts automatic paste.
`Shift+Enter` restores only the plain-text representation when one exists. If
`xdotool` is unavailable or paste fails, the content remains on the clipboard for
a manual paste.

## Picker Controls

- Type in the search box to filter history.
- `Up` and `Down` move the selected row.
- `Enter` restores the selected item and attempts automatic paste.
- `Shift+Enter` restores the selected item as plain text only.
- `Delete` hides the selected item from history.
- `Ctrl+P` toggles pin.
- `Ctrl+F` toggles favorite.
- `Escape` closes the picker.

See [docs/usage.md](docs/usage.md) for setup details and a manual verification
checklist.
