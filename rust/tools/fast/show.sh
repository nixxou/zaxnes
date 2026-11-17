#!/usr/bin/env bash
# Prints the self-cost table of one callgrind run, shortened. Usage: show.sh <dir> <tag> [lines]
sed -n "25,${3:-60}p" "$1/self.$2.txt" | sed -E 's#/mnt/g/3DSen/wt-fastcore/rust/crates/##; s#/rustc/[0-9a-f]*/library/##; s/zax_nes::cartridges::board:://g; s/zax_nes::nes_bus::KindBus/KB/g; s/ \[.*//' | cut -c1-170
