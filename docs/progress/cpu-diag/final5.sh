#!/bin/bash
# final5.sh: the corpus with every DMA cycle traced on the Zaxnes side too (harness built with
# --features bus-trace).
S=$(cd "$(dirname "$0")" && pwd)
H=${H:-G:/3DSen/wt-corespec/rust/target/release/zax-harness.exe}
for p in ${@:-loop modes allops branch dma oam0 oam1 nmi irq dmc dmc0 dmc1 dmcoam dmc2007 illegal dmc4016 intbranch fc5}; do
  c=4000
  case $p in nmi|dmc|dmc0|dmc1|dmcoam|dmc2007|dmc4016) c=30000;; irq|intbranch|fc5) c=60000;; esac
  $H cpu-bus-trace $S/$p.bin --cycles $((c - 10)) --alignment 3 --phase -8 --flip-get-put --dmc 431,8 --out $S/z5-$p.txt > /dev/null
  printf "%-9s %s\n" $p "$(python $S/cmp3.py $S/n4-$p.txt $S/z5-$p.txt | tr '\n' ' ')"
done
