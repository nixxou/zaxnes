#!/usr/bin/env bash
# Callgrind of bench-core in 2D (no profile) for the fast-core lot. Inside WSL.
# Usage: cg.sh <linux zax-harness> <out dir> <tag> <rom file name> [bench-core options...]
H=$1; OUT=$2; TAG=$3; ROM=$4; shift 4
mkdir -p "$OUT"
T=/mnt/g/3DSen/test-originals
B="200-210:start,320-330:start,450-2500:right,500-2500:b"
valgrind --tool=callgrind --callgrind-out-file="$OUT/cg.$TAG" "$H" bench-core "$T/$ROM" \
  --frames ${FRAMES:-300} --warmup 60 --press "$B" "$@" > "$OUT/bench.$TAG.txt" 2> "$OUT/vg.$TAG.txt"
callgrind_annotate --inclusive=yes "$OUT/cg.$TAG" 2>/dev/null | head -200 > "$OUT/top.$TAG.txt"
callgrind_annotate "$OUT/cg.$TAG" 2>/dev/null | head -60 > "$OUT/self.$TAG.txt"
grep -E "^summary:|^totals:" "$OUT/cg.$TAG" | head -1
