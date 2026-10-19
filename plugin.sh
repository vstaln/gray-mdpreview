#!/bin/sh
here="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
sys="$(uname -s)"
case "$sys" in
MINGW* | MSYS* | CYGWIN*) rel="x86_64-pc-windows-msvc/gray-mdpreview.exe" ;;
Darwin*)
    case "$(uname -m)" in
    arm64) rel="aarch64-apple-darwin/gray-mdpreview" ;;
    *) rel="x86_64-apple-darwin/gray-mdpreview" ;;
    esac ;;
*)
    case "$(uname -m)" in
    aarch64 | arm64) rel="aarch64-unknown-linux-musl/gray-mdpreview" ;;
    *) rel="x86_64-unknown-linux-musl/gray-mdpreview" ;;
    esac ;;
esac
if [ -x "$here/$rel" ]; then exec "$here/$rel" "$@"; fi
for cand in "$here/target/release/gray-mdpreview.exe" "$here/target/release/gray-mdpreview" "$here/target/debug/gray-mdpreview.exe" "$here/target/debug/gray-mdpreview"; do
    if [ -x "$cand" ]; then exec "$cand" "$@"; fi
done
echo "gray-mdpreview binary not found in $here" >&2
exit 1
