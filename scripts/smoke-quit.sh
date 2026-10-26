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

if [ "${ORBIT_SMOKE_INNER:-}" != "1" ]; then
  case "$BIN" in
    /snap/bin/*)
      # A strict snap keeps its own home under ~/snap/orbit; settings cannot be preset from
      # outside, so the splash plays and the first launch takes a little longer.
      export ORBIT_SMOKE_SNAP=1
      ;;
    *)
      # A throwaway home so this run uses fresh settings and touches nothing of the user's.
      export HOME="$(mktemp -d)"
      export XDG_DATA_HOME="$HOME/.local/share" XDG_CONFIG_HOME="$HOME/.config" XDG_CACHE_HOME="$HOME/.cache"
      mkdir -p "$XDG_DATA_HOME/com.orbit.rclone"
      # No splash, so the first instance is ready sooner.
      echo '{"splash":"off"}' > "$XDG_DATA_HOME/com.orbit.rclone/settings.json"
      ;;
  esac
fi
export PATH="$ROOT/src-tauri/binaries:$PATH"
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

# Orbit's data folder: inside a strict snap it sits under the snap's own home.
data_dir() {
  if [ -n "${ORBIT_SMOKE_SNAP:-}" ]; then
    echo "$(getent passwd "$(id -u)" | cut -d: -f6)/snap/orbit/current/.local/share/com.orbit.rclone"
  else
    echo "${XDG_DATA_HOME:-$HOME/.local/share}/com.orbit.rclone"
  fi
}
diagnose() { # diagnose <log file>: what the app printed, what rclone logged, what is running
  echo "--- $1"; cat "$1" 2>/dev/null || true
  local rlog; rlog="$(data_dir)/logs/rclone.log"
  echo "--- $rlog"; tail -n 40 "$rlog" 2>/dev/null || echo "(no rclone log)"
  echo "--- processes"; pgrep -a -x orbit || true; pgrep -a -x rclone || true
}

run_test() {
  echo "== starting the first Orbit"
  "$BIN" >"$HOME/first.log" 2>&1 &
  FIRST=$!
  wait_until 60 bash -c '[ "$(pgrep -c -x rclone || true)" -ge 1 ]' || { echo "FAIL: rclone did not start"; diagnose "$HOME/first.log"; return 1; }
  # Let the splash finish and the window settle before the second launch.
  sleep "${ORBIT_SMOKE_SNAP:+8}${ORBIT_SMOKE_SNAP:-2}"
  echo "   orbit: $(orbit_count), rclone rcd: $(rcd_count)"

  echo "== launching a second Orbit (must exit on its own)"
  if timeout 15 "$BIN" >"$HOME/second.log" 2>&1; then :; else
    code=$?
    [ "$code" -ne 124 ] || { echo "FAIL: the second Orbit did not exit"; diagnose "$HOME/second.log"; return 1; }
  fi
  sleep 1
  [ "$(orbit_count)" -eq 1 ] || { echo "FAIL: expected 1 orbit process, found $(orbit_count)"; return 1; }
  [ "$(rcd_count)" -eq 1 ] || { echo "FAIL: expected 1 rclone rcd, found $(rcd_count)"; return 1; }
  echo "   single instance OK"

  echo "== asking it to quit (--quit)"
  timeout 15 "$BIN" --quit >"$HOME/quit.log" 2>&1 || true
  wait_until 30 bash -c '! kill -0 '"$FIRST"' 2>/dev/null' || { echo "FAIL: the first Orbit is still running"; diagnose "$HOME/first.log"; return 1; }
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
