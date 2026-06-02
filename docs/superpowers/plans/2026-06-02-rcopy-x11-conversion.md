# rcopy X11 Conversion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert rcopy from a Wayland clipboard manager into an X11-only clipboard manager and publish `master` to `https://github.com/damody/rcopy_x11.git`.

**Architecture:** Keep the current crate boundaries. Replace the integration command backend from `wl-paste`/`wl-copy`/`wtype` to `xclip`/`xdotool`, then update setup and docs so the fork presents itself as X11-only.

**Tech Stack:** Rust workspace, Tokio command execution, GTK4/libadwaita picker, SQLite storage, systemd user service, `xclip`, `xdotool`, GNOME `gsettings`.

---

## File Structure

- Modify `crates/integration/src/clipboard.rs`: rename the command backend from `WlClipboard` to `X11Clipboard`, use `xclip` commands, keep MIME priority and HTML cleanup behavior.
- Modify `crates/integration/src/paste.rs`: rename paste backend from `WtypePasteBackend` to `XdotoolPasteBackend`, call `xdotool key ctrl+v`.
- Modify `crates/integration/src/lib.rs`: export the renamed X11 backend types.
- Modify `crates/core/src/config.rs`: default `paste_command` becomes `xdotool`.
- Modify `crates/daemon/src/main.rs` and related references if they instantiate old backend names.
- Modify `scripts/setup-hyprland.sh`: replace with X11 setup behavior or create `scripts/setup-x11.sh` and remove stale Wayland script.
- Modify `README.md` and `docs/usage.md`: rewrite setup and verification instructions for X11.
- Keep `.gitignore` as-is because it already ignores `/target/`, `*.db`, `*.db-shm`, and `*.db-wal`.

---

### Task 1: Convert Clipboard Backend To xclip

**Files:**
- Modify: `crates/integration/src/clipboard.rs`
- Modify: `crates/integration/src/lib.rs`
- Modify: `crates/daemon/src/main.rs`

- [ ] **Step 1: Write failing tests for xclip commands**

In `crates/integration/src/clipboard.rs`, rename tests around configured clipboard commands so they assert `xclip`-style behavior. Add fixtures that support:

```sh
xclip -selection clipboard -t TARGETS -o
xclip -selection clipboard -t text/plain;charset=utf-8 -o
xclip -selection clipboard -t text/plain -i
```

Expected Rust test names:

```rust
#[tokio::test]
async fn x11_clipboard_reads_parameterized_plain_text_target() { /* fixture */ }

#[tokio::test]
async fn x11_clipboard_prefers_plain_text_over_html_when_both_are_available() { /* fixture */ }

#[tokio::test]
async fn x11_clipboard_writes_plain_text_with_xclip_target() { /* fixture */ }
```

- [ ] **Step 2: Run focused tests and verify they fail**

Run:

```bash
cargo test -p rcopy-integration x11_clipboard_
```

Expected: tests fail or do not compile because the backend still uses `wl-paste`/`wl-copy`.

- [ ] **Step 3: Implement xclip backend**

In `crates/integration/src/clipboard.rs`:

- Rename `WlClipboard` to `X11Clipboard`.
- Change defaults to `Self::with_commands("xclip", "xclip")`.
- List targets with `xclip -selection clipboard -t TARGETS -o`.
- Read target with `xclip -selection clipboard -t <mime> -o`.
- Write target with `xclip -selection clipboard -t <mime> -i`.
- Update error text from `wl-paste`/`wl-copy` to `xclip`.
- Keep `offered_mime_for`, `normalize_plain_text`, `select_single_write_mime`, and PNG/HTML/text behavior.

In `crates/integration/src/lib.rs`, export `X11Clipboard`.

In `crates/daemon/src/main.rs`, instantiate `X11Clipboard::new()`.

- [ ] **Step 4: Run focused tests and verify they pass**

Run:

```bash
cargo test -p rcopy-integration x11_clipboard_
```

Expected: X11 clipboard tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/integration/src/clipboard.rs crates/integration/src/lib.rs crates/daemon/src/main.rs
git commit -m "改用 X11 剪貼簿後端"
```

---

### Task 2: Convert Auto Paste Backend To xdotool

**Files:**
- Modify: `crates/integration/src/paste.rs`
- Modify: `crates/integration/src/lib.rs`
- Modify: `crates/core/src/config.rs`
- Modify: `crates/daemon/src/main.rs`

- [ ] **Step 1: Write failing tests for xdotool**

In `crates/integration/src/paste.rs`, replace `wtype` tests with:

```rust
#[tokio::test]
async fn xdotool_paste_backend_sends_ctrl_v_key_event() {
    // fixture records argv and expects: key ctrl+v
}

#[tokio::test]
async fn xdotool_paste_backend_reports_missing_command_as_unavailable() {
    // missing command returns PasteError::NotAvailable
}
```

In `crates/core/src/config.rs`, update the default config test to expect:

```rust
assert_eq!(config.paste_command, "xdotool");
```

- [ ] **Step 2: Run focused tests and verify they fail**

Run:

```bash
cargo test -p rcopy-integration xdotool_paste_backend
cargo test -p rcopy-core config_defaults_capture_text_html_and_images
```

Expected: tests fail or do not compile while code still refers to `wtype`.

- [ ] **Step 3: Implement xdotool backend**

In `crates/integration/src/paste.rs`:

- Rename `WtypePasteBackend` to `XdotoolPasteBackend`.
- Default command should be `xdotool`.
- Spawn command with args `["key", "ctrl+v"]`.
- Update error messages to mention `xdotool`.

In `crates/integration/src/lib.rs`, export `XdotoolPasteBackend`.

In `crates/core/src/config.rs`, set `paste_command: "xdotool".to_string()`.

In `crates/daemon/src/main.rs`, instantiate `XdotoolPasteBackend`.

- [ ] **Step 4: Run focused tests and verify they pass**

Run:

```bash
cargo test -p rcopy-integration xdotool_paste_backend
cargo test -p rcopy-core config_defaults_capture_text_html_and_images
```

Expected: all focused tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/integration/src/paste.rs crates/integration/src/lib.rs crates/core/src/config.rs crates/daemon/src/main.rs
git commit -m "改用 xdotool 自動貼上"
```

---

### Task 3: Replace Setup Script With X11 Setup

**Files:**
- Create: `scripts/setup-x11.sh`
- Delete: `scripts/setup-hyprland.sh`

- [ ] **Step 1: Create X11 setup script**

Create `scripts/setup-x11.sh` with behavior equivalent to:

```sh
#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
RCOPY="$ROOT/target/debug/rcopy"
RCOPYD="$ROOT/target/debug/rcopyd"

echo "Building rcopy binaries..."
cargo build -p rcopyd -p rcopy-picker

if [ "${XDG_SESSION_TYPE:-}" != "x11" ]; then
  printf '%s\n' "Warning: this session is not X11. rcopy_x11 requires an X11 session." >&2
fi

for command in xclip xdotool; do
  if ! command -v "$command" >/dev/null 2>&1; then
    printf '%s\n' "Warning: missing dependency: $command" >&2
  fi
done

SYSTEMD_USER_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
mkdir -p "$SYSTEMD_USER_DIR"
cat > "$SYSTEMD_USER_DIR/rcopyd.service" <<EOF
[Unit]
Description=rcopy X11 clipboard daemon

[Service]
Type=simple
WorkingDirectory=$ROOT
ExecStart=$RCOPYD
Restart=on-failure

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable --now rcopyd.service

if command -v gsettings >/dev/null 2>&1; then
  GNOME_KEY_PATH="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/rcopy/"
  GNOME_SCHEMA="org.gnome.settings-daemon.plugins.media-keys"
  GNOME_CUSTOM_SCHEMA="org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$GNOME_KEY_PATH"
  CURRENT_BINDINGS=$(gsettings get "$GNOME_SCHEMA" custom-keybindings 2>/dev/null || printf '[]')
  NEW_BINDINGS="$CURRENT_BINDINGS"
  case "$CURRENT_BINDINGS" in
    *"$GNOME_KEY_PATH"*) ;;
    "@as []"|"[]") NEW_BINDINGS="['$GNOME_KEY_PATH']" ;;
    \[*\]) NEW_BINDINGS=$(printf '%s' "$NEW_BINDINGS" | sed "s|]$|, '$GNOME_KEY_PATH']|") ;;
    *) NEW_BINDINGS="['$GNOME_KEY_PATH']" ;;
  esac
  gsettings set "$GNOME_SCHEMA" custom-keybindings "$NEW_BINDINGS"
  gsettings set "$GNOME_CUSTOM_SCHEMA" name "rcopy clipboard picker"
  gsettings set "$GNOME_CUSTOM_SCHEMA" command "$RCOPY"
  gsettings set "$GNOME_CUSTOM_SCHEMA" binding "<Control>grave"
fi

printf '\nrcopyd user service is enabled and running.\n'
printf 'Shortcut: Ctrl+grave -> %s\n' "$RCOPY"
```

- [ ] **Step 2: Remove Wayland setup script**

Run:

```bash
git rm scripts/setup-hyprland.sh
chmod +x scripts/setup-x11.sh
```

- [ ] **Step 3: Verify script syntax**

Run:

```bash
sh -n scripts/setup-x11.sh
```

Expected: no output and exit code 0.

- [ ] **Step 4: Commit**

```bash
git add scripts/setup-x11.sh scripts/setup-hyprland.sh
git commit -m "新增 X11 安裝腳本"
```

---

### Task 4: Update Documentation To X11

**Files:**
- Modify: `README.md`
- Modify: `docs/usage.md`

- [ ] **Step 1: Rewrite README**

Update README to say:

- Project name is `rcopy_x11`.
- It is X11-only.
- Requirements include `xclip` and `xdotool`.
- Setup command is `./scripts/setup-x11.sh`.
- Shortcut is `Ctrl+\``.
- Manual commands use `xclip`, not Wayland tools.

- [ ] **Step 2: Rewrite usage docs**

Update `docs/usage.md` to include:

```bash
xclip -selection clipboard -t TARGETS -o
printf 'hello' | xclip -selection clipboard -t text/plain -i
```

Remove stale setup instructions for Hyprland, Wayland, `wl-paste`, `wl-copy`, and `wtype`.

- [ ] **Step 3: Verify stale text is gone**

Run:

```bash
rg -n "Wayland|wayland|Hyprland|wl-paste|wl-copy|wtype|wlroots" README.md docs scripts crates
```

Expected: no stale runtime references, except historical design documents under `docs/superpowers` if included in the search.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/usage.md
git commit -m "更新 X11 使用文件"
```

---

### Task 5: Final Verification And Publish

**Files:**
- No source changes expected unless verification exposes a defect.

- [ ] **Step 1: Run full automated verification**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build -p rcopyd -p rcopy-picker
```

Expected: all commands pass.

- [ ] **Step 2: Runtime verification when possible**

Run:

```bash
printf 'rcopy x11 test' | xclip -selection clipboard -t text/plain -i
socket="${XDG_RUNTIME_DIR:-/tmp}/rcopy/rcopyd.sock"
printf '{"type":"Capture"}\n' | nc -U "$socket"
printf '{"type":"Search","query":"rcopy x11 test"}\n' | nc -U "$socket"
```

Expected under X11 with dependencies installed: search returns an item whose `payload.text_plain` contains `rcopy x11 test`.

If the session is not X11 or dependencies are missing, record that manual runtime verification was skipped.

- [ ] **Step 3: Confirm ignored files are not tracked**

Run:

```bash
git status --short --ignored
git ls-files | rg '(^target/|rcopy\.db|\.db-(shm|wal)$)' || true
```

Expected: `target/` and `rcopy.db*` may appear only as ignored files; `git ls-files` prints nothing.

- [ ] **Step 4: Push to X11 remote**

Run:

```bash
git remote set-url origin https://github.com/damody/rcopy_x11.git
git push -u origin master
```

Expected: local `master` is pushed to remote `master`.

