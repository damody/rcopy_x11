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
        "@as []" | "[]") NEW_BINDINGS="['$GNOME_KEY_PATH']" ;;
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
