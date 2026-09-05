#!/bin/sh
# Install the latest rtop release without requiring administrator privileges.
set -eu

REPO="Mohammed-Bahr/rtop"
APP="rtop"
INSTALL_DIR="${RTOP_INSTALL_DIR:-${HOME:-}/.local/bin}"

fail() {
    printf 'rtop installer: error: %s\n' "$*" >&2
    exit 1
}

command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v tar >/dev/null 2>&1 || fail "tar is required"
[ -n "${HOME:-}" ] || fail 'HOME is not set; set HOME or RTOP_INSTALL_DIR'

OS=$(uname -s 2>/dev/null || true)
ARCH=$(uname -m 2>/dev/null || true)
case "$OS:$ARCH" in
    Linux:x86_64|Linux:amd64) TARGET="linux-x86_64"; FORMAT=tar.gz ;;
    Linux:aarch64|Linux:arm64) TARGET="linux-aarch64"; FORMAT=tar.gz ;;
    Darwin:x86_64|Darwin:amd64) TARGET="macos-x86_64"; FORMAT=tar.gz ;;
    Darwin:arm64|Darwin:aarch64) TARGET="macos-aarch64"; FORMAT=tar.gz ;;
    MINGW*:x86_64|MSYS*:x86_64|CYGWIN*:x86_64|Windows_NT:x86_64|MINGW*:amd64|MSYS*:amd64|CYGWIN*:amd64|Windows_NT:amd64)
        TARGET="windows-x86_64"; FORMAT=zip ;;
    *) fail "unsupported platform: OS=$OS architecture=$ARCH (supported: Linux x86_64/aarch64, macOS x86_64/arm64, Windows x86_64)" ;;
esac

case "$FORMAT" in
    tar.gz) ASSET="$APP-$TARGET.tar.gz" ;;
    zip) ASSET="$APP-$TARGET.zip" ;;
esac
if [ "$FORMAT" = zip ] && ! command -v unzip >/dev/null 2>&1; then
    fail "unzip is required for Windows release extraction"
fi

BASE_URL="https://github.com/$REPO/releases"
LATEST_URL="$BASE_URL/latest/download"
TMP_DIR=$(mktemp -d 2>/dev/null || mktemp -d -t rtop-install)
cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT HUP INT TERM

ARCHIVE="$TMP_DIR/$ASSET"
CHECKSUMS="$TMP_DIR/checksums.txt"
printf 'Downloading %s latest release...\n' "$ASSET"
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
    "$LATEST_URL/$ASSET" -o "$ARCHIVE" || fail "could not download $ASSET; check that a GitHub Release exists"
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
    "$LATEST_URL/checksums.txt" -o "$CHECKSUMS" || fail 'could not download checksums.txt'

EXPECTED=$(awk -v file="$ASSET" '$2 == file { print $1; exit }' "$CHECKSUMS")
[ "${EXPECTED:-}" ] || fail "checksums.txt has no entry for $ASSET"
case "$EXPECTED" in *[!0123456789abcdefABCDEF]*) fail 'invalid SHA256 checksum format' ;; esac
[ "${#EXPECTED}" -eq 64 ] || fail 'invalid SHA256 checksum length'
if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL=$(sha256sum "$ARCHIVE" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
    ACTUAL=$(shasum -a 256 "$ARCHIVE" | awk '{print $1}')
else
    fail 'sha256sum or shasum is required for checksum verification'
fi
[ "$EXPECTED" = "$ACTUAL" ] || fail 'checksum verification failed; refusing to install'

EXTRACT_DIR="$TMP_DIR/extracted"
mkdir "$EXTRACT_DIR"
case "$FORMAT" in
    tar.gz) tar -xzf "$ARCHIVE" -C "$EXTRACT_DIR" ;;
    zip) unzip -q "$ARCHIVE" -d "$EXTRACT_DIR" ;;
esac
BINARY="$EXTRACT_DIR/$APP"
[ "$FORMAT" = zip ] && BINARY="$EXTRACT_DIR/$APP.exe"
[ -f "$BINARY" ] || fail "release archive does not contain expected binary"

mkdir -p "$INSTALL_DIR"
chmod 0755 "$BINARY"
TMP_BINARY="$INSTALL_DIR/.$APP.tmp.$$"
rm -f "$TMP_BINARY"
cp "$BINARY" "$TMP_BINARY"
chmod 0755 "$TMP_BINARY"
if [ "$FORMAT" = tar.gz ]; then
    mv -f "$TMP_BINARY" "$INSTALL_DIR/$APP"
    [ -x "$INSTALL_DIR/$APP" ] || fail 'installation failed'
else
    mv -f "$TMP_BINARY" "$INSTALL_DIR/$APP.exe"
    [ -f "$INSTALL_DIR/$APP.exe" ] || fail 'installation failed'
fi

printf 'Installed %s to %s\n' "$APP" "$INSTALL_DIR"
case ":${PATH:-}:" in
    *":$INSTALL_DIR:"*) printf 'Run: %s --version\n' "$APP" ;;
    *)
        printf 'Note: %s is not currently in PATH. Add this line to your shell profile:\n' "$INSTALL_DIR"
        printf '  export PATH="%s:\$PATH"\n' "$INSTALL_DIR"
        printf 'Then start a new shell (or run: export PATH="%s:\$PATH")\n' "$INSTALL_DIR"
        ;;
esac
