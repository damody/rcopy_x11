#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)
ARCH=$(dpkg --print-architecture)
PACKAGE="rcopy-x11"
BUILD_ROOT="$ROOT/target/deb/$PACKAGE"
DEB="$ROOT/dist/${PACKAGE}_${VERSION}_${ARCH}.deb"

if [ -z "$VERSION" ]; then
    printf '%s\n' "Could not read workspace version from Cargo.toml" >&2
    exit 1
fi

case "$ARCH" in
    amd64) ;;
    *)
        printf '%s\n' "Warning: untested Debian architecture: $ARCH" >&2
        ;;
esac

echo "Building release binaries..."
cargo build --release -p rcopyd -p rcopy-picker

rm -rf "$BUILD_ROOT"
mkdir -p \
    "$BUILD_ROOT/DEBIAN" \
    "$BUILD_ROOT/usr/bin" \
    "$BUILD_ROOT/usr/share/doc/$PACKAGE" \
    "$ROOT/dist"

install -m 0755 "$ROOT/target/release/rcopy" "$BUILD_ROOT/usr/bin/rcopy"
install -m 0755 "$ROOT/target/release/rcopyd" "$BUILD_ROOT/usr/bin/rcopyd"
install -m 0755 "$ROOT/scripts/setup-x11.sh" "$BUILD_ROOT/usr/bin/rcopy-x11-setup"
install -m 0644 "$ROOT/README.md" "$BUILD_ROOT/usr/share/doc/$PACKAGE/README.md"
install -m 0644 "$ROOT/docs/usage.md" "$BUILD_ROOT/usr/share/doc/$PACKAGE/usage.md"

cat > "$BUILD_ROOT/DEBIAN/control" <<EOF
Package: $PACKAGE
Version: $VERSION
Section: utils
Priority: optional
Architecture: $ARCH
Maintainer: damody <damody@users.noreply.github.com>
Depends: xclip, xdotool, libgtk-4-1, libadwaita-1-0
Description: Local-first X11 clipboard history picker
 rcopy_x11 stores clipboard history locally, shows a GTK picker, and restores
 selected clipboard items through xclip with optional automatic paste through
 xdotool.
EOF

dpkg-deb --root-owner-group --build "$BUILD_ROOT" "$DEB"
printf '%s\n' "$DEB"
