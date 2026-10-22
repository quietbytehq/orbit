#!/usr/bin/env bash
# Downloads the pinned rclone build for this machine into src-tauri/binaries/,
# named the way Tauri expects sidecars: rclone-<target-triple>.
# Verifies the archive against rclone's published SHA256SUMS before using it.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(tr -d '[:space:]' < "$ROOT/scripts/rclone-version.txt")"
OUT_DIR="$ROOT/src-tauri/binaries"

case "$(uname -m)" in
  x86_64) ARCH=amd64; TRIPLE=x86_64-unknown-linux-gnu ;;
  aarch64) ARCH=arm64; TRIPLE=aarch64-unknown-linux-gnu ;;
  *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

ZIP="rclone-${VERSION}-linux-${ARCH}.zip"
BASE="https://downloads.rclone.org/${VERSION}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "Fetching rclone ${VERSION} (linux/${ARCH})"
curl -fsSL -o "$WORK/$ZIP" "$BASE/$ZIP"
curl -fsSL -o "$WORK/SHA256SUMS" "$BASE/SHA256SUMS"

expected="$(grep " ${ZIP}\$" "$WORK/SHA256SUMS" | awk '{print $1}')"
actual="$(sha256sum "$WORK/$ZIP" | awk '{print $1}')"
if [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
  echo "SHA256 mismatch for ${ZIP}" >&2
  exit 1
fi

unzip -q -j "$WORK/$ZIP" "rclone-${VERSION}-linux-${ARCH}/rclone" -d "$WORK"
mkdir -p "$OUT_DIR"
install -m 0755 "$WORK/rclone" "$OUT_DIR/rclone-${TRIPLE}"
echo "Installed $OUT_DIR/rclone-${TRIPLE}"
