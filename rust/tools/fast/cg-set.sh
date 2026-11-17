#!/usr/bin/env bash
# Callgrind of the fast-core set (inside WSL). Usage: cg-set.sh <out dir> [extra bench-core options]
OUT=$1; shift
H=/root/fastcore/target/release/zax-harness
for e in "smb1|Super Mario Bros. [World][!!].nes" "mm2|Mega Man 2 [USA][!!].nes" "smb3|Super Mario Bros. 3 [USA][Rev 1][!!].nes"; do
  k=${e%%|*}; r=${e#*|}
  printf "%s " "$k"
  bash /mnt/g/3DSen/wt-fastcore/rust/tools/fast/cg.sh "$H" "$OUT" "$k" "$r" "$@"
done
