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
(cd "$rust_dir" && cargo zigbuild --locked --release -p zax-app --features firmware-sdl \
    --target "$target.$glibc")
binary="$target_dir/$target/release/zaxnes"

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
install -m 644 "$root_dir/LICENSE" "$root_dir/THIRD-PARTY-NOTICES.md" "$root_dir/ASSETS-LICENSES.md" \
    "$root_dir/HOWTO-RUN.md" "$dir/"
printf '%s\n' "Déposez ici vos profils .3dn ; ils sont chargés au démarrage. Aucun profil n'est fourni." \
    > "$dir/profiles/README.txt"
cat > "$dir/README.txt" <<EOF
Zaxnes $version — émulateur NES avec rendu 3D (Linux x86_64)
================================================

Nécessite SDL2 (paquet libsdl2-2.0-0 sur Debian/Ubuntu, sdl2 sur Arch/Fedora) et OpenGL 3.3.
Aucune ROM et aucun profil ne sont fournis.

    ./zaxnes
    ./zaxnes --rom "jeu.nes" --profile "3D par défaut (sans profil)"
    ./zaxnes --list-profiles
EOF

# The package holds the binary, the documents and profiles/README.txt, and nothing else.
shipped="$(cd "$dir" && find . -type f | sort | tr '\n' ' ')"
expected="./ASSETS-LICENSES.md ./HOWTO-RUN.md ./LICENSE ./README.txt ./THIRD-PARTY-NOTICES.md ./profiles/README.txt ./zaxnes "
if [[ "$shipped" != "$expected" ]]; then
    echo "error: unexpected package contents: $shipped" >&2
    exit 1
fi

archive="$out_dir/$name.tar.gz"
rm -f "$archive"
tar -C "$stage" --owner=0 --group=0 -czf "$archive" "$name"
rm -rf "$stage"
echo "package: $archive"
