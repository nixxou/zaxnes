#!/usr/bin/env bash
# Per-line costs of one source file in a callgrind run. Usage: lines.sh <cg file> <source path> [min Ir]
callgrind_annotate --auto=yes --show-percs=no "$1" "$2" 2>/dev/null \
 | awk -v src="$(basename "$2")" 'index($0, "Auto-annotated source:") && index($0, src) {f=1; next} index($0, "Auto-annotated source:") {f=0} f' \
 | awk -v min="${3:-3000000}" '{n=$1; gsub(",","",n); if (n ~ /^[0-9]+$/ && n+0 >= min) print}'
