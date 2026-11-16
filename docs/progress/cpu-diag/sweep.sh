cd "$(dirname "$0")"
H=G:/3DSen/wt-corespec/rust/target/release/zax-harness.exe
echo "== nmi: power-on PPU phase (alignment 3)"
for ph in -20 -16 -13 -12 -10 -8 -6 -4 -3 0 4; do
  $H cpu-bus-trace nmi.bin --cycles 29990 --alignment 3 --phase $ph --flip-get-put --dmc 431,8 --out sw.txt > /dev/null
  printf "phase %4s: %s\n" $ph "$(python cmp3.py n4-nmi.txt sw.txt | head -1)"
done
echo "== nmi: defaults (no setting)"
$H cpu-bus-trace nmi.bin --cycles 29990 --out sw-nmi-default.txt > /dev/null
python cmp3.py n4-nmi.txt sw-nmi-default.txt | head -3
echo "== dmc0: without the get/put flip"
$H cpu-bus-trace dmc0.bin --cycles 29990 --alignment 3 --phase -8 --dmc 431,8 --out sw-dmc0-noflip.txt > /dev/null
python cmp3.py n4-dmc0.txt sw-dmc0-noflip.txt | head -3
echo "== oam0: without the get/put flip"
$H cpu-bus-trace oam0.bin --cycles 3990 --alignment 3 --phase -8 --dmc 431,8 --out sw-oam0-noflip.txt > /dev/null
python cmp3.py n4-oam0.txt sw-oam0-noflip.txt | head -3
echo "== dmc: without the DMC timer setting"
$H cpu-bus-trace dmc.bin --cycles 29990 --alignment 3 --phase -8 --flip-get-put --out sw-dmc-nodmc.txt > /dev/null
python cmp3.py n4-dmc.txt sw-dmc-nodmc.txt | head -3
