#!/bin/bash
# scan2.sh: parity x DMC timer on dmc.bin; prints the settings with the fewest differing cycles
S=$(cd "$(dirname "$0")" && pwd)
H=G:/3DSen/wt-accuracy/rust/target-acc/release/zax-harness.exe
for flip in "" "--flip-apu-parity"; do
  for t in $(seq 400 460); do
    $H cpu-bus-trace $S/dmc.bin --cycles 11990 --alignment 3 --phase -8 --dmc $t,8 $flip --out $S/zd.txt 2>/dev/null
    r=$(python $S/cmp.py $S/n-dmc.txt $S/zd.txt 1 | head -1 | awk '{print $4}')
    echo "${flip:-noflip} $t $r"
  done
done | sort -k3 -n | head -6
