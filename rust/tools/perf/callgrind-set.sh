#!/bin/sh
# Runs callgrind over the ROM set of `roms.txt` and writes one annotation per ROM.
#
# PERFORMANCE-HISTORY.md section 3's command, repeated over a set rather than over the one or
# two ROMs the earlier lots used. The Ir count is deterministic to the instruction: the same
# binary twice gives the same number, which is what makes a one-instruction-per-cycle change
# measurable where wall clock has three times that as noise. The runs are independent, so they
# go in parallel -- that changes no count, callgrind being a simulator.
#
#   usage: callgrind-set.sh <label> [jobs]
#
# Reads  $PR1_WORK/bin/zax-harness-<label>  (see build-clean.sh)
# Writes $PR1_WORK/cg-<label>/<key>.ann     callgrind_annotate output, per-symbol
#        $PR1_WORK/cg-<label>/<key>.ir      the whole-process instruction count, one number
#
# Environment:
#   PR1_WORK        scratch root (default $HOME/pr1)
#   PR1_FRAMES      emulated frames (default 300, as every MF lot used)
#   ZAX_TEST_ROMS   the `free` root (default /mnt/g/3dsen/cleanroom/assets-free/test-roms)
#   ZAX_COMM_ROMS   the `comm` root (default /mnt/g/3dsen/demo/roms)
set -eu

label=${1:?usage: callgrind-set.sh <label> [jobs]}
jobs=${2:-8}

here=$(cd "$(dirname "$0")" && pwd)
work=${PR1_WORK:-$HOME/pr1}
frames=${PR1_FRAMES:-300}
free_root=${ZAX_TEST_ROMS:-/mnt/g/3dsen/cleanroom/assets-free/test-roms}
comm_root=${ZAX_COMM_ROMS:-/mnt/g/3dsen/demo/roms}

bin=$work/bin/zax-harness-$label
[ -x "$bin" ] || { echo "callgrind-set: no $bin -- run build-clean.sh first" >&2; exit 1; }

out=$work/cg-$label
mkdir -p "$out"

one() {
    key=$1
    rom=$2
    cd "$out"
    valgrind --tool=callgrind --dump-instr=no --cache-sim=no --branch-sim=no \
        --callgrind-out-file="$out/$key.out" \
        "$bin" bench-core "$rom" --frames "$frames" --warmup 0 >"$out/$key.log" 2>&1
    # "Collected : <n>" is the whole process, the figure every MF lot's table reports.
    sed -n 's/^==[0-9]*== *Collected *: *\([0-9]*\)$/\1/p' "$out/$key.log" >"$out/$key.ir"
    callgrind_annotate --threshold=100 "$out/$key.out" >"$out/$key.ann" 2>&1
    rm -f "$out/$key.out"
    echo "  $key $(cat "$out/$key.ir")" >&2
}

running=0
# shellcheck disable=SC2162  # no backslash processing wanted in the ROM paths
grep -v '^[[:space:]]*#' "$here/roms.txt" | grep -v '^[[:space:]]*$' | while read key root path; do
    case $root in
        free) rom=$free_root/$path ;;
        comm) rom=$comm_root/$path ;;
        *) echo "callgrind-set: $key: unknown root '$root'" >&2; continue ;;
    esac
    if [ ! -f "$rom" ]; then
        echo "callgrind-set: $key: no $rom -- skipped" >&2
        continue
    fi
    one "$key" "$rom" &
    running=$((running + 1))
    if [ "$running" -ge "$jobs" ]; then
        wait
        running=0
    fi
done
wait
echo "callgrind-set: $out" >&2
