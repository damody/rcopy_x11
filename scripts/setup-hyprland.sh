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
systemctl --user enable --now rcopyd.service

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

cat <<EOF

rcopyd user service is enabled and running.

Hyprland binding installed in:

$RCOPY_HYPR_CONF

Binding:

bind = CTRL, grave, exec, $RCOPY

Your main Hyprland config should source it:

source = $RCOPY_HYPR_CONF

If Hyprland did not reload automatically, run:

hyprctl reload

Check daemon status with:

systemctl --user status rcopyd.service
EOF
