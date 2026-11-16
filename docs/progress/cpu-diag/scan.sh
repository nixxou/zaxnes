#!/bin/bash
# scan.sh <prog> <netlist trace> <alignment> <cycles>: differing cycles per PPU phase offset
S=$(cd "$(dirname "$0")" && pwd)
H=G:/3DSen/wt-accuracy/rust/target-acc/release/zax-harness.exe
for ph in $(seq -16 16); do
  $H cpu-bus-trace $S/$1.bin --cycles $4 --alignment $3 --phase $ph --out $S/zp.txt 2>/dev/null
  r=$(python $S/cmp.py $2 $S/zp.txt 1 | head -1 | awk '{print $4}')
  printf "%s:%s " "$ph" "$r"
done
echo
