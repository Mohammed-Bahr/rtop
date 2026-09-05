#!/bin/sh
set -eu

INSTALL_DIR="${RTOP_INSTALL_DIR:-${HOME:-}/.local/bin}"
removed=0
for file in "$INSTALL_DIR/rtop" "$INSTALL_DIR/rtop.exe"; do
    if [ -f "$file" ]; then
        rm -f "$file"
        printf 'Removed %s\n' "$file"
        removed=1
    fi
done
[ "$removed" -eq 1 ] || printf 'rtop is not installed in %s\n' "$INSTALL_DIR"
