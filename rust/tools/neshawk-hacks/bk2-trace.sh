#!/usr/bin/env bash
# Plays the NesHawk .bk2 movies to their end on the NesHawk engine and writes, per movie, the
# work-RAM checksum of EVERY frame (lag probe on, so each frame also round-trips a savestate).
# Usage: bk2-trace.sh <harness.exe> <out-dir> [extra movie-sync options...]
# Two runs (before/after a change) are compared with `diff -r`.
# Windows paths: MSYS does not convert arguments holding `[!!]`.
# Zelda II (tasvideos/arc,inzultv2-zeldaii.bk2) is left out: its Input Log's LogKey is refused
# by the harness's .bk2 reader on the build before this lot already.
set -u
harness=$1; out=$2; shift 2
mkdir -p "$out"
movies='G:\3DSen\test-movies'
export ZAXNES_BIZHAWK_GAMEDB='G:\3DSen\ext-tools\BizHawk-2.11.1\gamedb'
wout=$(cygpath -w "$out")
run() {
  name=$1; movie=$2; rom=$3; shift 3
  "$harness" movie-sync "$movie" "$rom" --engine neshawk --trace-step 1 \
    --trace "$wout\\$name.txt" "$@" > "$out/$name.log" 2>&1
  echo "$name exit=$? lines=$(wc -l < "$out/$name.txt" 2>/dev/null)"
}
run kirby "$movies\\bk2\\keyword-kirbysadventure-noabilities.bk2" \
  "$movies\\roms\\Hoshi no Kirby - Yume no Izumi no Monogatari [Japan][!!].nes" "$@" &
run faxanadu "$movies\\bk2\\invariel-faxanadu.bk2" "$movies\\roms\\Faxanadu [USA][!!].nes" "$@" &
run nova "$movies\\tasvideos\\cephla-novathesquirrel.bk2" 'G:\3DSen\nova\nova.nes' "$@" &
wait
