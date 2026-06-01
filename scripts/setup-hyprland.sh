#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
RCOPYD="$ROOT_DIR/target/debug/rcopyd"
RCOPY="$ROOT_DIR/target/debug/rcopy"
SERVICE_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
SERVICE_FILE="$SERVICE_DIR/rcopyd.service"

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

cat <<EOF

rcopyd user service is enabled and running.

Add this binding to your Hyprland config:

bind = CTRL, grave, exec, $RCOPY

Then reload Hyprland:

hyprctl reload

Check daemon status with:

systemctl --user status rcopyd.service
EOF
