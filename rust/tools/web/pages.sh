#!/usr/bin/env bash
# Prepares the GitHub Pages site (https://nixxou.github.io/zaxnes/) in a local `gh-pages`
# branch, in its own worktree next to this one. Nothing is pushed: the last line printed is the
# push to run once the site looks right (python -m http.server -d <worktree> 8080).
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../../.." && pwd)"
bash "$here/build.sh"
site="${CARGO_TARGET_DIR:-$repo/rust/target-web}/site"
pages="${PAGES_WORKTREE:-$repo/../wt-gh-pages}"
if [ ! -d "$pages" ]; then
    if git -C "$repo" show-ref --verify --quiet refs/heads/gh-pages; then
        git -C "$repo" worktree add "$pages" gh-pages
    else
        git -C "$repo" worktree add --detach "$pages"
        git -C "$pages" checkout --orphan gh-pages
        git -C "$pages" rm -rf --quiet . || true
    fi
fi
find "$pages" -mindepth 1 -maxdepth 1 ! -name .git -exec rm -rf {} +
cp "$site/index.html" "$site/zaxnes.js" "$site/zaxnes.wasm" "$pages/"
# The interface's face (Roboto, Apache-2.0, with its licence).
cp -r "$site/fonts" "$pages/"
touch "$pages/.nojekyll"
git -C "$pages" add -A
git -C "$pages" commit -q -m "Zaxnes web build $(git -C "$repo" rev-parse --short HEAD)" || echo "nothing changed"
echo "ready: $pages"
echo "to publish: git -C \"$pages\" push origin gh-pages   (then Settings > Pages > branch gh-pages)"
