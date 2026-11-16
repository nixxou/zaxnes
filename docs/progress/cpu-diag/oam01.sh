#!/bin/bash
# oam0/oam1: OAM DMA started on one CPU cycle or the next; DMA length on each side
S=$(cd "$(dirname "$0")" && pwd)
B=G:/3DSen/ext-tools/visualnes-bench/benchcpu.exe
H=G:/3DSen/wt-accuracy/rust/target-acc/release/zax-harness.exe
python - $S <<'EOF'
import sys, os
S = sys.argv[1]
for name, x in (('oam0', 0), ('oam1', 1)):
    prg = bytearray(0x7FFA)
    code = bytes([0xA2, 0xFF, 0x9A, 0xA2, x, 0xBD, 0xFF, 0x02, 0xA9, 0x02, 0x8D, 0x14, 0x40,
                  0xE8, 0x4C, 0x0D, 0x80])
    prg[:len(code)] = code
    prg[0x1000] = 0x40
    open(os.path.join(S, name + '.bin'), 'wb').write(bytes(prg))
EOF
for p in oam0 oam1; do
  $B G:/3DSen/ext-tools/visualnes-src/Core $S/$p.bin $S/n-$p.txt 1200 4 > /dev/null
  $H cpu-bus-trace $S/$p.bin --cycles 1190 --alignment 3 --phase -8 $EXTRA --out $S/z-$p.txt
  zw=$(awk '$2=="4014" {print $1; exit}' $S/z-$p.txt)
  zn=$(awk -v w=$zw '$1>w && $2!="DMA" {print $1; exit}' $S/z-$p.txt)
  nw=$(awk '$2=="4014" && $3=="W" {print $1; exit}' $S/n-$p.txt)
  nn=$(awk -v w=$nw '$1>w+1 && $2 ~ /^80/ {print $1; exit}' $S/n-$p.txt)
  echo "$p: zaxnes write $zw next CPU access $zn (+$((zn-zw))); netlist write $nw next fetch $nn (+$((nn-nw)))"
  echo "   netlist first DMA cycles: $(awk -v w=$nw '$1>w && $1<=w+4 {print $2 $3}' $S/n-$p.txt | tr '\n' ' ')"
done
