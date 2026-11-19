#!/usr/bin/env bash
# Builds the Linux x86_64 desktop binary of the Rust port and packs
# rust/target/package/Zaxnes-<version>-linux-x64.tar.gz.
#
#   rust/build/package-linux-x64.sh
#
# Same recipe as package-linux-aarch64.sh: cargo-zigbuild links against glibc 2.31 whatever the
# host has, so the binary runs on any distribution from 2020 on (Ubuntu 20.04, Debian 11...).
# SDL2 is the system's libSDL2-2.0.so.0, linked dynamically (feature `firmware-sdl`): the
# distribution's SDL2 knows its display server (X11 or Wayland), audio server and GPU driver.
# SDL2 is compiled once from the pinned source of the sdl2-sys crate, with every backend off
# (rust/build/x86_64-sdl2-zig.cmake), only to give the linker its symbols; it is not packaged.
# The Vulkan renderer (feature `wgpu`, `--backend vulkan`) loads the system's libvulkan.so.1 at
# run time, when it is chosen: it is no link dependency.
#   pip install ziglang && cargo install cargo-zigbuild --locked
#   sudo apt install cmake
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
rust_dir="$(cd "$script_dir/.." && pwd)"
root_dir="$(cd "$rust_dir/.." && pwd)"
target=x86_64-unknown-linux-gnu
glibc=2.31
target_dir="${CARGO_TARGET_DIR:-$rust_dir/target}"
out_dir="$target_dir/package"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$rust_dir/Cargo.toml" | head -n 1)"

export SDL2_TOOLCHAIN="${SDL2_TOOLCHAIN:-$script_dir/x86_64-sdl2-zig.cmake}"
# The FCEUX engine is built in (owner decision, 2026-11-12), from the fork built with zig for the
# same glibc (`sh zax/build-linux-zig.sh` in the fork). ZAX_FCEUX_DIR names the fork's root; its
# sources ship in the package (GPL), as fceux-zax-source.zip.
: "${ZAX_FCEUX_DIR:?set ZAX_FCEUX_DIR to the root of the fceux-zax fork}"
export ZAX_FCEUX_LIB="${ZAX_FCEUX_LIB:-$ZAX_FCEUX_DIR/build-linux-zig}"
# The hack packs ship as files in a zxpk folder next to the binary, taken from a clone of
# zaxnes_3dn that ZAXNES_3DN_DIR names; none is built into the binary.
: "${ZAXNES_3DN_DIR:?set ZAXNES_3DN_DIR to a clone of zaxnes_3dn (the hack packs)}"
[[ -d "$ZAXNES_3DN_DIR/packs" ]] || { echo "error: no packs folder in $ZAXNES_3DN_DIR" >&2; exit 1; }
export ZAXNES_EMBED_PACKS=none
(cd "$rust_dir" && cargo zigbuild --locked --release -p zax-app \
    --features firmware-sdl,shaders,fceux-core --target "$target.$glibc")
# The harness of the editor mode (the editor folder's AGENT.md): renders from any angle without
# the window, lists a profile's patterns.
(cd "$rust_dir" && cargo zigbuild --locked --release -p zax-harness --target "$target.$glibc")
binary="$target_dir/$target/release/zaxnes"
harness="$target_dir/$target/release/zax-harness"
[[ -x "$harness" ]] || { echo "error: $harness was not built" >&2; exit 1; }

[[ -x "$binary" ]] || { echo "error: $binary was not built" >&2; exit 1; }

# Refuse a binary that needs a newer glibc, or any library beyond the base system and SDL2.
newest="$(objdump -T "$binary" | grep -o 'GLIBC_[0-9.]*' | sort -Vu | tail -n 1 || true)"
if [[ -n "$newest" && "$(printf '%s\n' "${newest#GLIBC_}" "$glibc" | sort -V | tail -n 1)" != "$glibc" ]]; then
    echo "error: $binary needs $newest, newer than glibc $glibc" >&2
    exit 1
fi
unexpected="$(readelf -d "$binary" | sed -n 's/.*Shared library: \[\(.*\)\]/\1/p' \
    | grep -v -E '^(libc|libm|libdl|libpthread|librt|libgcc_s)\.so|^ld-linux-x86-64|^libSDL2-2\.0\.so\.0$' || true)"
if [[ -n "$unexpected" ]]; then
    echo "error: unexpected dynamic dependencies: $unexpected" >&2
    exit 1
fi
echo "binary: max $newest, dependencies ok"

name="Zaxnes-$version-linux-x64"
stage="$out_dir/stage"
dir="$stage/$name"
rm -rf "$stage"
mkdir -p "$dir/profiles"

install -m 755 "$binary" "$dir/zaxnes"
install -m 755 "$harness" "$dir/zax-harness"
install -m 644 "$root_dir/LICENSE" "$root_dir/THIRD-PARTY-NOTICES.md" "$root_dir/ASSETS-LICENSES.md" \
    "$root_dir/HOWTO-RUN.md" "$dir/"
# The hack packs, with their licence (they are not under the GPL).
mkdir -p "$dir/zxpk"
install -m 644 "$ZAXNES_3DN_DIR"/packs/*.zxpk "$ZAXNES_3DN_DIR/LICENSE.md" \
    "$ZAXNES_3DN_DIR/ADDITIONAL-PERMISSIONS.md" "$dir/zxpk/"
echo "package: $(ls "$dir/zxpk" | grep -c '\.zxpk$') hack packs from zaxnes_3dn $(git -C "$ZAXNES_3DN_DIR" rev-parse --short HEAD)"
# GPL: the corresponding source of the FCEUX fork linked into zaxnes, at the commit built.
git -C "$ZAX_FCEUX_DIR" archive --format=zip --prefix=fceux-zax/ -o "$dir/fceux-zax-source.zip" HEAD
printf '%s\n' "Déposez ici vos profils .3dn ; ils sont chargés au démarrage. Aucun profil n'est fourni." \
    > "$dir/profiles/README.txt"
cat > "$dir/README.txt" <<EOF
Zaxnes $version — émulateur NES avec rendu 3D (Linux x86_64)
================================================

Nécessite SDL2 (paquet libsdl2-2.0-0 sur Debian/Ubuntu, sdl2 sur Arch/Fedora) et OpenGL 3.3.
Aucune ROM n'est fournie ; les profils 3D faits pour Zaxnes (Faxanadu, Metal Force) sont
intégrés au programme et installés au premier lancement.

    ./zaxnes
    ./zaxnes --rom "jeu.nes" --profile "3D par défaut (sans profil)"
    ./zaxnes --list-profiles
EOF

# The package holds the binary, the documents, profiles/README.txt and the packs, and nothing else.
shipped="$(cd "$dir" && find . -type f ! -path './zxpk/*.zxpk' | sort | tr '\n' ' ')"
expected="./ASSETS-LICENSES.md ./HOWTO-RUN.md ./LICENSE ./README.txt ./THIRD-PARTY-NOTICES.md ./fceux-zax-source.zip ./profiles/README.txt ./zax-harness ./zaxnes ./zxpk/ADDITIONAL-PERMISSIONS.md ./zxpk/LICENSE.md "
if [[ "$shipped" != "$expected" ]]; then
    echo "error: unexpected package contents: $shipped" >&2
    exit 1
fi

archive="$out_dir/$name.tar.gz"
rm -f "$archive"
tar -C "$stage" --owner=0 --group=0 -czf "$archive" "$name"
rm -rf "$stage"
echo "package: $archive"
