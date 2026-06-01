#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
RCOPYD="$ROOT_DIR/target/debug/rcopyd"
RCOPY="$ROOT_DIR/target/debug/rcopy"
SERVICE_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
SERVICE_FILE="$SERVICE_DIR/rcopyd.service"
HYPR_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/hypr"
HYPR_CONF="$HYPR_DIR/hyprland.conf"
RCOPY_HYPR_CONF="$HYPR_DIR/rcopy.conf"

echo "Building rcopy binaries..."
cargo build --manifest-path "$ROOT_DIR/Cargo.toml" -p rcopyd -p rcopy-picker

if [ "${XDG_SESSION_TYPE:-}" != "wayland" ]; then
    cat <<EOF

Warning: this session is not Wayland.
Current XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-unset}

rcopy's clipboard backend uses wl-clipboard, so clipboard capture and paste need
a Wayland session.
EOF
fi

if ! command -v hyprctl >/dev/null 2>&1; then
    cat <<EOF

Warning: hyprctl was not found in PATH.
The Hyprland binding file will be written, but this script cannot reload or
verify Hyprland. If you are not logged into Hyprland, Ctrl+grave will not work.
EOF
fi

mkdir -p "$SERVICE_DIR"
cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=rcopy Wayland clipboard daemon

[Service]
Type=simple
ExecStart=$RCOPYD
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user import-environment WAYLAND_DISPLAY DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_TYPE XDG_SESSION_DESKTOP 2>/dev/null || true
if command -v dbus-update-activation-environment >/dev/null 2>&1; then
    dbus-update-activation-environment --systemd WAYLAND_DISPLAY DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_TYPE XDG_SESSION_DESKTOP >/dev/null 2>&1 || true
fi
systemctl --user enable --now rcopyd.service
systemctl --user restart rcopyd.service

mkdir -p "$HYPR_DIR"
cat > "$RCOPY_HYPR_CONF" <<EOF
# rcopy clipboard picker
bind = CTRL, grave, exec, $RCOPY
EOF

if [ -f "$HYPR_CONF" ]; then
    if ! grep -Fq "source = $RCOPY_HYPR_CONF" "$HYPR_CONF"; then
        cp "$HYPR_CONF" "$HYPR_CONF.rcopy-backup"
        {
            printf '\n# rcopy clipboard picker\n'
            printf 'source = %s\n' "$RCOPY_HYPR_CONF"
        } >> "$HYPR_CONF"
    fi
else
    cat > "$HYPR_CONF" <<EOF
# Hyprland config created by rcopy setup.
# If you already keep Hyprland config somewhere else, add this source line there:
source = $RCOPY_HYPR_CONF
EOF
fi

if command -v hyprctl >/dev/null 2>&1; then
    hyprctl reload >/dev/null 2>&1 || true
fi

if command -v gsettings >/dev/null 2>&1; then
    GNOME_KEY_PATH="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/rcopy/"
    GNOME_SCHEMA="org.gnome.settings-daemon.plugins.media-keys"
    GNOME_CUSTOM_SCHEMA="org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$GNOME_KEY_PATH"
    CURRENT_BINDINGS=$(gsettings get "$GNOME_SCHEMA" custom-keybindings 2>/dev/null || printf '[]')

    NEW_BINDINGS="$CURRENT_BINDINGS"
    case "$NEW_BINDINGS" in
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

cat <<EOF

rcopyd user service is enabled and running.

Hyprland binding installed in:

$RCOPY_HYPR_CONF

Binding:

bind = CTRL, grave, exec, $RCOPY

GNOME/Ubuntu Wayland shortcut:

<Control>grave -> $RCOPY

Your main Hyprland config should source it:

source = $RCOPY_HYPR_CONF

If Hyprland did not reload automatically, run:

hyprctl reload

Check daemon status with:

systemctl --user status rcopyd.service

Current session:

XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-unset}
XDG_CURRENT_DESKTOP=${XDG_CURRENT_DESKTOP:-unset}
EOF
