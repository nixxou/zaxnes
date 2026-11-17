#!/usr/bin/env bash
# Golden-core of the fast core against the standard reference, then the 2D bench. Run from rust/.
S=${FAST_SCRATCH:?set FAST_SCRATCH}; H=target/release/zax-harness.exe; T=G:/3DSen/test-originals
ZAXNES_FAST_CORE=1 $H golden-core $T $S/fast.jsonl ${FRAMES:-600} > $S/fast.log 2>&1
python tools/fast/agree.py $S/std.jsonl $S/fast.jsonl --rows | head -${ROWS:-20}
for r in "Super Mario Bros. [World][!!].nes" "Mega Man 2 [USA][!!].nes" "Metroid [USA][!!].nes" "Super Mario Bros. 3 [USA][Rev 1][!!].nes"; do
  for f in "" --fast; do printf "%-7s %-24s " "${f:-std}" "${r:0:22}"
    $H bench-core "$T/$r" --frames 2000 --press "200-210:start,320-330:start,450-2500:right,500-2500:b" $f 2>&1 | grep "core run_frame" | awk '{print $3}'
  done
done
