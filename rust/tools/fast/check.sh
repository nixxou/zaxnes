#!/usr/bin/env bash
# Golden-core of the fast core against the standard reference, then the 2D bench. Run from rust/.
S=${FAST_SCRATCH:-./fast-scratch}; H=target/release/zax-harness.exe; T=G:/3DSen/test-originals
P="100-110:start,200-210:start,300-2400:right,320-360:a,500-540:a,700-760:a,900-930:b,1000-1040:a,1200-1300:left,1400-1450:a,1600-1700:down,1800-1830:a"
export ZAXNES_GOLDEN_PRESS=$P ZAXNES_GOLDEN_EVERY=30
[ -f $S/stdp.jsonl ] || $H golden-core $T $S/stdp.jsonl 1800 > $S/stdp.log 2>&1
ZAXNES_FAST_CORE=1 $H golden-core $T $S/fastp.jsonl 1800 > $S/fastp.log 2>&1
unset ZAXNES_GOLDEN_PRESS ZAXNES_GOLDEN_EVERY
python tools/fast/agree.py $S/stdp.jsonl $S/fastp.jsonl --rows | awk 'NR<=3 || /mapper/' | head -${ROWS:-25}
python tools/fast/agree.py $S/stdp.jsonl $S/fastp.jsonl | tail -1
for r in "Super Mario Bros. [World][!!].nes" "Mega Man 2 [USA][!!].nes" "Metroid [USA][!!].nes" "Super Mario Bros. 3 [USA][Rev 1][!!].nes"; do
  for f in "" --fast; do printf "%-7s %-24s " "${f:-std}" "${r:0:22}"
    $H bench-core "$T/$r" --frames 2000 --press "200-210:start,320-330:start,450-2500:right,500-2500:b" $f 2>&1 | grep "core run_frame" | awk '{print $3}'
  done
done
# WIDE=1: the same comparison with a 64-pixel widescreen margin and the advanced patches.
if [ -n "$WIDE" ]; then
  export ZAXNES_GOLDEN_PRESS=$P ZAXNES_GOLDEN_EVERY=30 ZAXNES_GOLDEN_WIDE=64 ZAXNES_GOLDEN_ADVANCED=1
  [ -f $S/stdw.jsonl ] || $H golden-core $T $S/stdw.jsonl 1800 > $S/stdw.log 2>&1
  ZAXNES_FAST_CORE=1 $H golden-core $T $S/fastw.jsonl 1800 > $S/fastw.log 2>&1
  unset ZAXNES_GOLDEN_PRESS ZAXNES_GOLDEN_EVERY ZAXNES_GOLDEN_WIDE ZAXNES_GOLDEN_ADVANCED
  echo "widescreen:"; python tools/fast/agree.py $S/stdw.jsonl $S/fastw.jsonl --rows | awk '/mapper/' | head -${ROWS:-25}
  python tools/fast/agree.py $S/stdw.jsonl $S/fastw.jsonl | tail -1
fi
