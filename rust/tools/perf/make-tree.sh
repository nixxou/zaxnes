#!/bin/sh
# Packs a CLEAN source tree into a tar, for `build-clean.sh` to build in a fresh directory.
#
# Separate from the build on purpose. In a git **worktree** the `.git` file holds an absolute
# path to the main repository, so a worktree checked out on the Windows side is not a git
# repository as the WSL2 Ubuntu sees it: git has to run on the host and only the tar crosses
# over. That is also why this script takes no shortcut through the working directory -- a copy
# of the folder would drag in `target-*`, scratch files and everything else ignored.
#
#   usage: make-tree.sh <ref|--worktree> <out.tar>
#
# <ref>        any git ref; its tree is taken with `git archive`
# --worktree   the working tree as `git ls-files` sees it, for a variant not yet committed
set -eu

source=${1:?usage: make-tree.sh <ref|--worktree> <out.tar>}
out=${2:?usage: make-tree.sh <ref|--worktree> <out.tar>}

top=$(git rev-parse --show-toplevel)
cd "$top"

if [ "$source" = "--worktree" ]; then
    git ls-files -z | tar --null -T - -cf "$out"
else
    git archive --format=tar -o "$out" "$source"
fi
echo "make-tree: $source -> $out" >&2
