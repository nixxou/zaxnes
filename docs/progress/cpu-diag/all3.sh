#!/bin/bash
# all3.sh: the hard cases, with the settings that make the netlist's power-on (step 4)
S=$(cd "$(dirname "$0")" && pwd)
B=G:/3DSen/ext-tools/visualnes-bench/benchcpu.exe
H=G:/3DSen/wt-accuracy/rust/target-acc/release/zax-harness.exe
python $S/mkprog3.py $S > /dev/null
for p in ${@:-dmcoam dmc4016 dmc2007 illegal intbranch fc5}; do
  c=30000
  case $p in intbranch|fc5) c=60000;; illegal) c=4000;; esac
  [ -f $S/n4-$p.txt ] || $B G:/3DSen/ext-tools/visualnes-src/Core $S/$p.bin $S/n4-$p.txt $c 4 > /dev/null
  $H cpu-bus-trace $S/$p.bin --cycles $((c - 10)) --alignment 3 --phase -8 --flip-get-put --dmc 431,8 --out $S/z4-$p.txt
  printf "%-9s %s\n" $p "$(python $S/cmp.py $S/n4-$p.txt $S/z4-$p.txt 4 | tr '\n' ' ')"
done
