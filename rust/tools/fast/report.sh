#!/usr/bin/env bash
# Inclusive and self tables of a cg-set run. Usage: report.sh <dir> [tags...]
D=$1; shift
for k in "${@:-smb1 smb3}"; do
  echo "== $k"
  bash /mnt/g/3DSen/wt-fastcore/rust/tools/fast/incl.sh "$D" "$k" | head -16
  bash /mnt/g/3DSen/wt-fastcore/rust/tools/fast/show.sh "$D" "$k" 50
done
