# rcopy X11 Conversion Design

## Goal

Convert this project from a Wayland clipboard manager into an X11-only clipboard manager and publish it to `https://github.com/damody/rcopy_x11.git` on `master`.

The X11 version should keep the current local-only Ditto-like workflow:

- `rcopyd` stores clipboard history.
- `rcopy` opens the picker.
- Selecting an item restores it to the clipboard and attempts automatic paste.
- No groups, cloud sync, login, or multi-device features.

## Scope

This is a fork-style conversion, not a dual Wayland/X11 backend.

In scope:

- Replace Wayland clipboard commands with X11 commands.
- Replace Wayland automatic paste command with X11 automatic paste command.
- Update setup scripts, README, and usage docs to describe X11.
- Keep the current picker UI and storage behavior.
- Commit changes with a Chinese commit message.
- Push `master` to `https://github.com/damody/rcopy_x11.git`.

Out of scope:

- Wayland fallback support.
- Runtime backend selection.
- New UI features.
- Data migration beyond keeping the existing SQLite schema compatible.

## Backend Commands

Use `xclip` for clipboard read/write:

- List offered targets: `xclip -selection clipboard -t TARGETS -o`
- Read a MIME target: `xclip -selection clipboard -t <mime> -o`
- Write a MIME target: `xclip -selection clipboard -t <mime> -i`

Use `xdotool` for automatic paste:

- `xdotool key ctrl+v`

The default config should set `paste_command` to `xdotool`.

## Clipboard Behavior

The current MIME behavior should be preserved where X11 supports it:

- Prefer `text/plain` over `text/html` when both are available.
- Read the exact offered target name when it includes parameters.
- Strip HTML document fragments from plain-text payloads when the plain text is actually an HTML document.
- Keep HTML only when plain text is unavailable.
- Keep PNG support through `image/png`.
- When restoring an item, prefer PNG, then plain text, then HTML.
- When restoring plain text that contains an HTML document fragment from old history, clean it before writing.

## Setup Behavior

Create or update an X11 setup script, preferably `scripts/setup-x11.sh`.

The script should:

- Build `rcopyd` and `rcopy`.
- Check that the session is X11 and warn if it is not.
- Check for `xclip` and `xdotool`.
- Install or update the systemd user service for `rcopyd`.
- Install a GNOME/Ubuntu custom shortcut for `Ctrl+\`` when `gsettings` is available.
- Avoid adding build outputs or SQLite runtime files to Git.

Existing Wayland-specific setup should either be removed or clearly replaced so users do not accidentally install the wrong backend.

## Documentation

Update README and usage docs to describe:

- X11-only support.
- Required packages: Rust, GTK4/libadwaita development libraries, `xclip`, `xdotool`.
- Setup command.
- `Ctrl+\`` shortcut behavior.
- Manual verification commands using `xclip`.

Remove stale references to:

- Wayland
- Hyprland
- wlroots
- `wl-paste`
- `wl-copy`
- `wtype`

## Testing

Automated verification:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo build -p rcopyd -p rcopy-picker`

Manual verification:

- If running under X11 with `xclip` and `xdotool` installed, test one capture/search/restore path.
- If the current session is not X11, report that runtime verification could not be completed in this environment.

## Git Publishing

After implementation and verification:

- Commit with a Chinese message.
- Set `origin` to `https://github.com/damody/rcopy_x11.git`.
- Push local `master` to remote `master`.

