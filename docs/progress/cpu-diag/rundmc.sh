#!/bin/bash
S=$(cd "$(dirname "$0")" && pwd)
B=G:/3DSen/ext-tools/visualnes-bench/benchcpu.exe
H=G:/3DSen/wt-accuracy/rust/target-acc/release/zax-harness.exe
python $S/mkdmc.py $S
for p in dmc0 dmc1; do
  $B G:/3DSen/ext-tools/visualnes-src/Core $S/$p.bin $S/n-$p.txt 1200 ${EXTRA_RESET:-4} > /dev/null
  $H cpu-bus-trace $S/$p.bin --cycles 1190 --alignment 3 --phase -8 $EXTRA --out $S/z-$p.txt
  w=$(awk '$2=="4015" && $3=="W" {print $1; exit}' $S/z-$p.txt)
  echo "== $p zaxnes: $(awk -v w=$w '$1>=w && $1<=w+9' $S/z-$p.txt | tr '\n' '|')"
  echo "   netlist: $(awk 'f{print} /^[0-9]+ 8000 R/{f=1}' $S/n-$p.txt | awk '$2=="4015"{f=1} f{n++; print $2, $3, $4} n==10{exit}' | tr '\n' '|')"
done
