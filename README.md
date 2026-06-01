# rcopy

`rcopy` is a local-first clipboard manager for Hyprland and other wlroots-based
Wayland sessions.

It captures supported clipboard content into a local SQLite database, exposes a
Unix-socket daemon, and provides a GTK4/Libadwaita picker for searching,
restoring, pinning, favoriting, and deleting clipboard history. Restoring an item
writes it back to the clipboard and can attempt an automatic paste with `wtype`.

This first version is intentionally local only. It does not include groups,
device sync, login, accounts, or cloud storage.

## Dependencies

Runtime commands:

- `wl-clipboard` for `wl-paste` and `wl-copy`
- `wtype` for optional automatic paste
- GTK4 runtime libraries
- Libadwaita runtime libraries

Build dependencies:

- Rust and Cargo
- GTK4 development package
- Libadwaita development package

Common package names are `libgtk-4-dev` and `libadwaita-1-dev` on Debian/Ubuntu,
`gtk4-devel` and `libadwaita-devel` on Fedora, and `gtk4` and `libadwaita` on
Arch Linux.

## Run

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

## Hyprland Binding

Add a Ctrl+backtick binding like this to `hyprland.conf`:

```text
bind = CTRL, grave, exec, cargo run --manifest-path /home/damody/work/rcopy/Cargo.toml -p rcopy-picker
```

Keep `rcopyd` running in your session before opening the picker.

## Clipboard Support

The daemon currently captures these MIME types:

- `text/plain`
- `text/html`
- `image/png`

Unsupported MIME types are ignored. If at least one supported payload is present,
the item is stored. Restore currently writes one best representation through
`wl-copy`: PNG first, then HTML, then plain text.

In the picker, `Enter` restores the selected item and attempts automatic paste.
`Shift+Enter` restores only the plain-text representation when one exists. If
`wtype` is unavailable or paste fails, the content remains on the clipboard for a
manual paste.

See [docs/usage.md](docs/usage.md) for setup details and a manual verification
checklist.
