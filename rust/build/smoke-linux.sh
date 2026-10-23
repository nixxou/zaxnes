#!/usr/bin/env bash
# End-to-end smoke test of the real `zaxnes` application on Linux (release build), under Xvfb.
#
#   rust/build/smoke-linux.sh
#
# Needs: xvfb xdotool imagemagick wmctrl openbox python3 (apt install xvfb xdotool imagemagick
# wmctrl openbox), SDL2 and Mesa (software GL is forced). Checks, with the public test ROMs:
# --help / --list-profiles, the welcome screen, a game running and drawing, pause, save/load
# state, reset, the options window, the French UI and back, rewind and fast forward, audio output
# (SDL disk driver, not silent), the files written in the user-data folder, and a clean exit
# (code 0, no panic) from Ctrl+Q and from a window-manager close. Exits non-zero on the first
# failure.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
BIN=${ZAXNES_BIN:-$ROOT/target/release/zaxnes}
VIDEO_ROM=$ROOT/test-roms/spritecans-2011/spritecans.nes
AUDIO_ROM=$ROOT/test-roms/apu_mixer/square.nes
WORK=$(mktemp -d)
DISPLAY_NUM=${SMOKE_DISPLAY:-97}
export DISPLAY=:$DISPLAY_NUM LIBGL_ALWAYS_SOFTWARE=1 SDL_AUDIODRIVER=dummy

PIDS=()
cleanup() {
  kill "${PIDS[@]}" 2>/dev/null || true
  rm -rf "$WORK"
}
trap cleanup EXIT

fail() { echo "SMOKE FAIL: $*" >&2; exit 1; }
ok() { echo "ok: $*"; }

[[ -x $BIN ]] || (cd "$ROOT" && cargo build --release -p zax-app) || fail "build"
for tool in Xvfb openbox xdotool import compare convert wmctrl python3; do
  command -v "$tool" >/dev/null || fail "missing tool: $tool"
done

# --- 1. command line -------------------------------------------------------------------------
"$BIN" --help | grep -q -- "--rom" || fail "--help"
"$BIN" --list-profiles --userdata "$WORK/empty" | grep -q "2D Mode" || fail "--list-profiles"
ok "--help and --list-profiles"

# --- display ----------------------------------------------------------------------------------
Xvfb ":$DISPLAY_NUM" -screen 0 1280x720x24 >/dev/null 2>&1 &
PIDS+=($!)
sleep 2
openbox >/dev/null 2>&1 &
PIDS+=($!)
sleep 1

shot() { import -window root "$WORK/$1.png"; }
# Pixels that differ between two shots (fuzz absorbs the FPS counter's anti-aliasing noise).
changed() { compare -fuzz 8% -metric AE "$WORK/$1.png" "$WORK/$2.png" null: 2>&1 || true; }
# A key must be held for at least one frame: the application samples the keyboard state.
tap() { xdotool keydown "$1"; sleep "${2:-0.25}"; xdotool keyup "$1"; sleep 0.5; }
chord() { xdotool keydown "$1"; sleep 0.1; tap "$2"; xdotool keyup "$1"; sleep 0.5; }
# The menu bar only: it does not move with the game, so it tells the language.
menu_changed() { compare -fuzz 8% -metric AE -extract 400x40+0+0 "$WORK/$1.png" "$WORK/$2.png" null: 2>&1 || true; }
click() { xdotool mousemove "$1" "$2" click 1; sleep 0.8; }
stddev() { convert "$WORK/$1.png" -crop 1280x670+0+50 -format '%[fx:standard_deviation]' info:; }
nonblank() { python3 -I -c "import sys; sys.exit(0 if float(sys.argv[1]) > 0.02 else 1)" "$(stddev "$1")" \
  || fail "$1 is blank"; }
alive() { kill -0 "$APP" 2>/dev/null || fail "application died ($1)"; }

start() { # <log> <args...>
  local log=$1; shift
  "$BIN" "$@" >"$WORK/$log" 2>&1 &
  APP=$!
  PIDS+=($APP)
}
finish() { # waits for the exit, checks code 0 and no panic
  local log=$1 code=0
  for _ in $(seq 40); do kill -0 "$APP" 2>/dev/null || break; sleep 0.25; done
  kill -0 "$APP" 2>/dev/null && fail "$log: still running after the quit request"
  wait "$APP" || code=$?
  [[ $code -eq 0 ]] || fail "$log: exit code $code"
  ! grep -q -i "panicked" "$WORK/$log" || fail "$log: panic"
}

# --- 2. welcome screen, then a game with 2D Mode -----------------------------------------------
UD=$WORK/ud
start welcome.log --userdata "$WORK/ud-welcome"
sleep 4; alive welcome; shot welcome; nonblank welcome
chord ctrl q; finish welcome.log
ok "welcome screen, Ctrl+Q exit 0"

start game.log --rom "$VIDEO_ROM" --profile "2D Mode" --userdata "$UD"
sleep 6; alive game; shot run1; sleep 1; shot run2; nonblank run1
[[ $(changed run1 run2) -gt 200 ]] || fail "the picture does not move: frames are not running"
ok "window open, frames running, picture rendered"

tap p; shot p1; sleep 1; shot p2
[[ $(changed p1 p2) -lt 20 ]] || fail "pause does not freeze the picture"
shot paused
chord shift F1                       # save state in slot 1 (paused picture)
tap p; sleep 1.5; shot moved
[[ $(changed paused moved) -gt 200 ]] || fail "resume does not run again"
ok "pause / resume"

# Loading a slot goes through the launch job, which ends in Running (as in the C# application):
# the check is that the saved scene comes back after a reset emptied it.
chord ctrl r; sleep 0.4; shot reset
[[ $(changed moved reset) -gt 200 ]] || fail "reset has no visible effect"
ok "reset"
chord ctrl F1; sleep 0.6; shot loaded
[[ $(changed reset loaded) -gt 1500 ]] || fail "load state did not bring the saved scene back"
shot loaded2
[[ $(changed loaded loaded2) -gt 200 ]] || fail "the game does not run after loading a state"
ok "save state, load state"

tap period 1.5; alive fast-forward
tap comma 1.5; alive rewind
ok "fast forward and rewind keys (no crash)"

chord ctrl comma; sleep 0.5; shot options
[[ $(changed reset options) -gt 20000 ]] || fail "options window did not open"
click 800 160                          # Interface tab
click 540 295; click 400 342; sleep 0.5; shot french  # language combo, Français
[[ $(menu_changed options french) -gt 100 ]] || fail "UI did not change language"
click 540 295; click 400 322; sleep 0.5; shot english
[[ $(menu_changed options english) -lt 20 ]] || fail "UI did not come back to English"
ok "options window, French and back"
chord ctrl q; finish game.log
ok "Ctrl+Q exit 0 with the options window open"

# --- 3. user-data folder as the C# app writes it -----------------------------------------------
for f in settings.json games.json saves/index.json saves/1.state saves/1.png; do
  [[ -s $UD/$f ]] || fail "missing user-data file $f"
done
grep -q '"file": "1.state"' "$UD/saves/index.json" || fail "slot 1 not recorded in saves/index.json"
ok "user-data: settings.json games.json saves/index.json saves/1.state saves/1.png"

# --- 3b. the 3D path without a profile (default geometry), F11 back to 2D ----------------------
start three.log --rom "$VIDEO_ROM" --profile "3D par défaut (sans profil)" --userdata "$WORK/ud-3d"
sleep 12; alive three; shot three1; nonblank three1; sleep 1; shot three2
[[ $(changed three1 three2) -gt 200 ]] || fail "3D mode: the picture does not move"
tap F11; sleep 1; alive three; shot two
chord ctrl q; finish three.log
cp "$WORK"/three1.png /tmp/three1.png; ok "3D default geometry renders, F11, exit 0"

# --- 4. window close from the window manager, and audio ----------------------------------------
AUDIO=$WORK/audio.raw
SDL_AUDIODRIVER=disk SDL_DISKAUDIOFILE=$AUDIO start audio.log --rom "$AUDIO_ROM" \
  --profile "2D Mode" --userdata "$WORK/ud-audio"
sleep 8; alive audio
wmctrl -c Zaxnes; finish audio.log
python3 -I - "$AUDIO" <<'PY' || fail "audio output is silent"
import array, sys
data = open(sys.argv[1], "rb").read()
samples = array.array("h")
samples.frombytes(data[: len(data) // 2 * 2])
sys.exit(0 if len(samples) > 100_000 and max(samples) - min(samples) > 1000 else 1)
PY
ok "audio is not silent, window close exit 0"

echo "SMOKE PASS"
