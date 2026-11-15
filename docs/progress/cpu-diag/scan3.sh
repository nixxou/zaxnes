#!/bin/bash
# scan3.sh: DMC output-unit phase with the netlist's get/put parity; total differing cycles over
# dmc, dmc0 and dmc1 (one power-on phase must fit all three)
S=$(cd "$(dirname "$0")" && pwd)
H=G:/3DSen/wt-accuracy/rust/target-acc/release/zax-harness.exe
for bits in 8 7 6 5 4 3 2 1; do
  for t in $(seq ${FROM:-0} ${TO:-470}); do
    total=0
    for p in dmc dmc0 dmc1; do
      $H cpu-bus-trace $S/$p.bin --cycles 2990 --alignment 3 --phase -8 --flip-get-put --dmc $t,$bits --out $S/zs.txt 2>/dev/null
      r=$(python $S/cmp.py $S/n4-$p.txt $S/zs.txt 1 | head -1 | awk '{print $4}')
      total=$((total + r))
    done
    echo "$bits $t $total"
  done
done | sort -k3 -n | head -5
