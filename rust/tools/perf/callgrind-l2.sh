#!/bin/sh
# Runs callgrind over the **level-2** set: the five commercial games, each with its 3D pack,
# 600 frames after the default 120 of warm-up, with inputs pressed.
#
# `callgrind-set.sh` is lot PR-1's instrument: thirteen ROMs, 300 frames, no pack, no input. It
# is a measure of the emulated core alone, and lot MF-5b went down on all thirteen of it while
# going **up** on two of the five games below. The difference is the pack: with one loaded the
# widescreen extension's execution zone is live (so `WideExtension::zone_offset` is on the
# instruction-fetch path), the games are in play rather than on a title screen, and a frame
# costs two and a half times as many instructions. This script is the level-2 instrument, so a
# lot can measure that before R0 does.
#
#   usage: callgrind-l2.sh <label> [jobs]
#
# Reads  $PR1_WORK/bin/zax-harness-<label>   (see build-clean.sh)
# Writes $PR1_WORK/l2-<label>/<key>.ir       the whole-process instruction count, one number
#        $PR1_WORK/l2-<label>/<key>.ann      callgrind_annotate, per-symbol *exclusive*
#        $PR1_WORK/l2-<label>/<key>.inc      callgrind_annotate --inclusive=yes, for the stages
#        $PR1_WORK/l2-<label>/<key>.log      the harness's own per-stage milliseconds
#        $PR1_WORK/l2-<label>/stages.txt     "<key> <stage> <Ir>", the table R0 compares
#
# `<key>.ann` and `<key>.ir` are named as `callgrind-set.sh` names them, so `callgrind-diff.py`
# reads a level-2 directory unchanged.
#
# Environment:
#   PR1_WORK        scratch root (default $HOME/pr1)
#   L2_FRAMES       emulated frames (default 600; the warm-up stays the harness's own 120)
#   ZAX_L2_ROMS     the commercial ROM root
#   ZAX_L2_PACKS    the 3D pack root
set -eu

label=${1:?usage: callgrind-l2.sh <label> [jobs]}
jobs=${2:-5}

here=$(cd "$(dirname "$0")" && pwd)
work=${PR1_WORK:-$HOME/pr1}
frames=${L2_FRAMES:-600}
rom_root=${ZAX_L2_ROMS:?callgrind-l2.sh: set ZAX_L2_ROMS to the commercial ROM root}
pack_root=${ZAX_L2_PACKS:?callgrind-l2.sh: set ZAX_L2_PACKS to the 3D pack root}

# The input script R0's level-2 run presses: start twice to get past the title and the file
# screen, then right and B held for the rest of the run so the game is in play.
press=200-210:start,320-330:start,450-2500:right,500-2500:b

bin=$work/bin/zax-harness-$label
[ -x "$bin" ] || { echo "callgrind-l2: no $bin -- run build-clean.sh first" >&2; exit 1; }

out=$work/l2-$label
mkdir -p "$out"

# The Lua garbage collector's mode changes how many instructions the frame hooks cost, so it is
# pinned rather than left to whatever the default of the day is.
export ZAXNES_LUA_GC=smooth

one() {
    key=$1
    rom=$2
    pack=$3
    cd "$out"
    valgrind --tool=callgrind --dump-instr=no --cache-sim=no --branch-sim=no \
        --callgrind-out-file="$out/$key.out" \
        "$bin" bench-core "$rom" --profile "$pack" --frames "$frames" --press "$press" \
        >"$out/$key.log" 2>"$out/$key.vg"
    sed -n 's/^==[0-9]*== *Collected *: *\([0-9]*\)$/\1/p' "$out/$key.vg" >"$out/$key.ir"
    callgrind_annotate --threshold=100 "$out/$key.out" >"$out/$key.ann" 2>&1
    callgrind_annotate --inclusive=yes --threshold=100 "$out/$key.out" >"$out/$key.inc" 2>&1
    rm -f "$out/$key.out"
    echo "  $key $(cat "$out/$key.ir")" >&2
}

# A here-document and not a pipe: see the note in `callgrind-set.sh` on the batch of runs a
# subshell once orphaned.
list=$(grep -v '^[[:space:]]*#' "$here/roms-l2.txt" | grep -v '^[[:space:]]*$')

running=0
while IFS= read -r line; do
    key=${line%%[   ]*}
    paths=${line#"$key"}
    rom=${paths%%|*}
    pack=${paths#*|}
    # Trim the padding spaces the table is aligned with.
    rom=$(printf '%s' "$rom" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')
    pack=$(printf '%s' "$pack" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')
    if [ ! -f "$rom_root/$rom" ]; then
        echo "callgrind-l2: $key: no $rom_root/$rom -- skipped" >&2
        continue
    fi
    if [ ! -f "$pack_root/$pack" ]; then
        echo "callgrind-l2: $key: no $pack_root/$pack -- skipped" >&2
        continue
    fi
    one "$key" "$rom_root/$rom" "$pack_root/$pack" &
    running=$((running + 1))
    if [ "$running" -ge "$jobs" ]; then
        wait
        running=0
    fi
done <<LIST
$list
LIST
wait

# The stage table: the inclusive count of the one function that is the stage. R0 takes the
# largest of the entries a name has (`callgrind_annotate --inclusive=yes` lists a function once
# per file the inlined code came from), which is the one that is the whole call tree.
for inc in "$out"/*.inc; do
    key=$(basename "$inc" .inc)
    sed -E 's/ \[[^]]*\]$//' "$inc" | awk -v k="$key" '
        function put(s) { gsub(",", "", $1); if (!(s in v) || $1 + 0 > v[s] + 0) v[s] = $1 }
        /NesCore>::run_frame$/            { put("core") }
        /Ppu2C02>::run_dots$/             { put("ppu") }
        /Apu2A03>::run_cpu_cycles$/       { put("apu") }
        /ConversionEngine>::load_profile$/ { put("load_profile") }
        /ConversionEngine>::convert$/     { put("convert") }
        /ConversionEngine>::run_frame_hooks$/ { put("lua_hooks") }
        /DrawListBuilder>::build$/        { put("draw_list") }
        /bench_core::run$/                { put("total") }
        END { for (s in v) printf "%s %s %s\n", k, s, v[s] }' | sort
done >"$out/stages.txt"

echo "callgrind-l2: $out" >&2
