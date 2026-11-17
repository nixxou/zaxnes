#!/usr/bin/env bash
# Inclusive costs per frame of the core's stages. Usage: incl.sh <dir> <tag> [frames, default 360]
F=${3:-360}
grep -E "run_frame |run_dots::|run_cpu_cycles_as|catch_up_apu|run_cpu_cycles_fast|Cpu6502>::step::|end_capture_line|begin_capture_line|run_sprite_evaluation|run_sprite_fetches|run_span_groups|run_visible_span|ConversionEngine>::convert |bench_core::run " "$1/top.$2.txt" \
 | sed -E 's#/mnt/g/3DSen/wt-fastcore/rust/crates/##; s/zax_nes::cartridges::board:://g; s/ \[.*//' \
 | awk -v f="$F" '{n=$1; gsub(",","",n); $1=""; $2=""; printf "%8.2f M/frame %s\n", n/f/1e6, $0}' | sort -k1 -rn | awk '!seen[$0]++' | head -30
