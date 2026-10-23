#!/usr/bin/env bash
# Scripted check for single instance and clean quit, on Linux, without a desktop.
#
# Launches the Orbit binary under a virtual display and its own D-Bus session, then:
#   1. launches it a second time: that copy must hand over and exit by itself, leaving one Orbit
#      and one rclone;
#   2. launches it with --quit: the running Orbit must stop rclone and exit, leaving neither.
#
# Usage: scripts/smoke-quit.sh [path-to-orbit-binary]
# Needs: xvfb-run, dbus-run-session, an rclone on PATH or next to the binary.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${1:-$ROOT/src-tauri/target/debug/orbit}"
[ -x "$BIN" ] || { echo "No Orbit binary at $BIN (build with: npm run tauri build -- --debug --no-bundle)" >&2; exit 2; }

# A throwaway home so this run uses fresh settings and touches nothing of the user's.
if [ "${ORBIT_SMOKE_INNER:-}" != "1" ]; then
  export HOME="$(mktemp -d)"
fi
export XDG_DATA_HOME="$HOME/.local/share" XDG_CONFIG_HOME="$HOME/.config" XDG_CACHE_HOME="$HOME/.cache"
export PATH="$ROOT/src-tauri/binaries:$PATH"
mkdir -p "$XDG_DATA_HOME/com.orbit.rclone"
# No splash, so the first instance is ready sooner.
echo '{"splash":"off"}' > "$XDG_DATA_HOME/com.orbit.rclone/settings.json"
# Software rendering: there is no GPU on a virtual display.
export WEBKIT_DISABLE_COMPOSITING_MODE=1 LIBGL_ALWAYS_SOFTWARE=1

count() { pgrep -c -x "$1" || true; }
wait_until() { # wait_until <seconds> <command...>
  local secs=$1; shift
  for _ in $(seq 1 $((secs * 2))); do "$@" && return 0; sleep 0.5; done
  return 1
}
orbit_count() { pgrep -c -x orbit || true; }
rcd_count() { count rclone; }

run_test() {
  echo "== starting the first Orbit"
  "$BIN" >"$HOME/first.log" 2>&1 &
  FIRST=$!
  wait_until 30 bash -c '[ "$(pgrep -c -x rclone || true)" -ge 1 ]' || { echo "FAIL: rclone did not start"; cat "$HOME/first.log"; return 1; }
  sleep 2
  echo "   orbit: $(orbit_count), rclone rcd: $(rcd_count)"

  echo "== launching a second Orbit (must exit on its own)"
  if timeout 15 "$BIN" >"$HOME/second.log" 2>&1; then :; else
    code=$?
    [ "$code" -ne 124 ] || { echo "FAIL: the second Orbit did not exit"; cat "$HOME/second.log"; return 1; }
  fi
  sleep 1
  [ "$(orbit_count)" -eq 1 ] || { echo "FAIL: expected 1 orbit process, found $(orbit_count)"; return 1; }
  [ "$(rcd_count)" -eq 1 ] || { echo "FAIL: expected 1 rclone rcd, found $(rcd_count)"; return 1; }
  echo "   single instance OK"

  echo "== asking it to quit (--quit)"
  timeout 15 "$BIN" --quit >"$HOME/quit.log" 2>&1 || true
  wait_until 30 bash -c '! kill -0 '"$FIRST"' 2>/dev/null' || { echo "FAIL: the first Orbit is still running"; cat "$HOME/first.log"; return 1; }
  sleep 1
  [ "$(orbit_count)" -eq 0 ] || { echo "FAIL: orbit processes remain: $(orbit_count)"; return 1; }
  [ "$(rcd_count)" -eq 0 ] || { echo "FAIL: rclone rcd processes remain: $(rcd_count)"; return 1; }
  echo "   clean quit OK (no orbit, no rclone)"
}

cleanup() { pkill -x orbit 2>/dev/null || true; pkill -x rclone 2>/dev/null || true; }

if [ "${ORBIT_SMOKE_INNER:-}" = "1" ]; then
  # Inside the virtual display and the D-Bus session.
  trap cleanup EXIT
  run_test
  exit $?
fi

# Re-run this script inside a virtual display with its own D-Bus session (the single-instance
# check and the tray use D-Bus on Linux).
export ORBIT_SMOKE_INNER=1
exec xvfb-run -a -s "-screen 0 1280x800x24" dbus-run-session -- "$0" "$BIN"
