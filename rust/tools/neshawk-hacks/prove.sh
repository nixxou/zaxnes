#!/usr/bin/env bash
# The proof of lot NESHAWK-HACKS, three processes at a time: for each NesHawk .bk2, the build
# before the lot, the build with the lot (no extension), and the same with a locked extension
# (ZAXNES_NESHAWK_WIDE=1), each writing the work-RAM checksum of every frame; then cmp.
R=/g/3DSen/neshawk-hacks-runs
cd "$R" || exit 1
movies='G:\3DSen\test-movies'
export ZAXNES_BIZHAWK_GAMEDB='G:\3DSen\ext-tools\BizHawk-2.11.1\gamedb'
one() { # out-dir exe name movie rom
  mkdir -p "$R/$1"
  "bin/$2" movie-sync "$4" "$5" --engine neshawk --trace-step 1 --trace "$(cygpath -w "$R/$1")\$3.txt" > "$R/$1/$3.log" 2>&1
  echo "$1 $3 exit=$?"
}
movie() { # name movie rom
  one baseline zax-harness-baseline.exe "$1" "$2" "$3" &
  one lot-none zax-harness-step4.exe "$1" "$2" "$3" &
  ZAXNES_NESHAWK_WIDE=1 one lot-locked zax-harness-step4.exe "$1" "$2" "$3" &
  wait
  for v in lot-none lot-locked; do
    if cmp -s "baseline/$1.txt" "$v/$1.txt"; then echo "$1 $v IDENTICAL ($(wc -l < baseline/$1.txt) lines)"; else echo "$1 $v DIFFERENT"; fi
  done
}
movie nova "$movies\tasvideos\cephla-novathesquirrel.bk2" 'G:\3DSen\nova\nova.nes'
movie faxanadu "$movies\bk2\invariel-faxanadu.bk2" "$movies\roms\Faxanadu [USA][!!].nes"
movie kirby "$movies\bk2\keyword-kirbysadventure-noabilities.bk2" "$movies\roms\Hoshi no Kirby - Yume no Izumi no Monogatari [Japan][!!].nes"
