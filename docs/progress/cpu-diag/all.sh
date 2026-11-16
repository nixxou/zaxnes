#!/bin/bash
# all.sh: the whole corpus, netlist (reset hold 4) against Zaxnes (alignment 3, phase -8,
# the netlist's get/put parity), every cycle
S=$(cd "$(dirname "$0")" && pwd)
B=G:/3DSen/ext-tools/visualnes-bench/benchcpu.exe
H=G:/3DSen/wt-accuracy/rust/target-acc/release/zax-harness.exe
python $S/mkprog.py $S > /dev/null
python $S/mkprog2.py $S > /dev/null
python $S/mkdmc.py $S > /dev/null
for p in loop modes allops branch dma oam0 oam1 nmi irq dmc dmc0 dmc1; do
  c=4000
  case $p in nmi) c=30000;; irq) c=60000;; dmc|dmc0|dmc1) c=30000;; esac
  [ -f $S/n4-$p.txt ] || $B G:/3DSen/ext-tools/visualnes-src/Core $S/$p.bin $S/n4-$p.txt $c 4 > /dev/null
  $H cpu-bus-trace $S/$p.bin --cycles $((c - 10)) --alignment 3 --phase -8 --flip-get-put --out $S/z4-$p.txt
  printf "%-7s %s\n" $p "$(python $S/cmp.py $S/n4-$p.txt $S/z4-$p.txt 3 | tr '\n' ' ')"
done
