#!/bin/sh
# Builds `zax-harness` from a CLEAN tree, for an instruction count that means something.
#
# PERFORMANCE-HISTORY.md section 2 and 3: every compared commit is extracted into a brand-new
# directory with its own CARGO_TARGET_DIR, never a reused one (a reused target folder is the
# artefact that once gave R0 a false regression), and built with
# CARGO_PROFILE_RELEASE_STRIP=false so callgrind has symbols to name. Nothing else of the
# release profile changes, so the codegen is the shipped one.
#
# Run it inside the WSL2 Ubuntu (its rustc is the same version with the same LLVM as the
# Windows host's, so the Linux binary's hot code is the Windows binary's). The tar comes from
# `make-tree.sh`, which runs on the host because a worktree's `.git` is a host path.
#
#   usage: build-clean.sh <label> <tree.tar>
#
# <label> names the output binary: $PR1_WORK/bin/zax-harness-<label>
#
# Environment:
#   PR1_WORK   scratch root (default $HOME/pr1). Needs about 4 GB free per build; the build
#              directory is deleted as soon as the binary has been copied out of it.
set -eu

label=${1:?usage: build-clean.sh <label> <tree.tar>}
tar_file=${2:?usage: build-clean.sh <label> <tree.tar>}

work=${PR1_WORK:-$HOME/pr1}
tree=$work/tree-$label
out=$work/bin
mkdir -p "$out"

# A fresh directory, never reused: remove any remnant of an earlier run under this label.
rm -rf "$tree"
mkdir -p "$tree"
tar -xf "$tar_file" -C "$tree"

# The host checks files out with CRLF, so a tar of the working tree carries them while a
# `git archive` tar does not. Normalise, or the two trees differ in every line of every file
# and nothing can be compared.
find "$tree" -type f \( -name '*.rs' -o -name '*.toml' -o -name '*.lock' \) \
    -exec sed -i 's/\r$//' {} +

echo "build-clean: $label from $tar_file in $tree" >&2
(
    cd "$tree/rust"
    CARGO_PROFILE_RELEASE_STRIP=false \
    CARGO_TARGET_DIR="$tree/tgt" \
        cargo build --release -p zax-harness --bin zax-harness
)
cp "$tree/tgt/release/zax-harness" "$out/zax-harness-$label"

# The tree and its ~4 GB target directory have served their purpose; the binary is the artefact.
rm -rf "$tree"
ls -l "$out/zax-harness-$label" >&2
