#!/usr/bin/env bash
# Builds the Linux aarch64 binary of the Rust port and packs the PortMaster package
# rust/target/package/Zaxnes.zip (Anbernic H700 handhelds: muOS, Knulli, ROCKNIX; RK3566 too).
#
#   rust/build/package-linux-aarch64.sh            cross build on an x86_64 Linux host
#   rust/build/package-linux-aarch64.sh --native   build on an aarch64 machine (no glibc pin)
#
# Why zig: the container's glibc is newer than the firmwares' (2.31 on muOS, Knulli, ROCKNIX).
# cargo-zigbuild links against glibc 2.31 headers and symbols whatever the host has, and builds
# the C and C++ dependencies (Lua, Dear ImGui, SDL2) with `zig cc` for the same baseline.
#   pip install ziglang && cargo install cargo-zigbuild --locked
#   sudo apt install cmake        (SDL2 link library, see below)
#   rustup target add aarch64-unknown-linux-gnu
#
# SDL2: the firmware's libSDL2-2.0.so.0 is patched for its GPU driver (libmali on muOS and
# Knulli, Panfrost on ROCKNIX), so the program links it dynamically (feature `firmware-sdl`) and
# the package ships none. SDL2 is still compiled once from the pinned source of the sdl2-sys
# crate (2.26.x), with every backend turned off (rust/build/aarch64-sdl2-zig.cmake): that copy
# only provides the symbols and the soname for the linker and is not packaged. GL, EGL, KMS and
# audio are loaded by the firmware's SDL at run time.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
rust_dir="$(cd "$script_dir/.." && pwd)"
root_dir="$(cd "$rust_dir/.." && pwd)"
target=aarch64-unknown-linux-gnu
glibc=2.31
target_dir="${CARGO_TARGET_DIR:-$rust_dir/target}"
out_dir="$target_dir/package"

if [[ "${1:-}" == "--native" ]]; then
    (cd "$rust_dir" && cargo build --release -p zax-app --features firmware-sdl)
    binary="$target_dir/release/zaxnes"
else
    export SDL2_TOOLCHAIN="${SDL2_TOOLCHAIN:-$script_dir/aarch64-sdl2-zig.cmake}"
    (cd "$rust_dir" && cargo zigbuild --release -p zax-app --features firmware-sdl \
        --target "$target.$glibc")
    binary="$target_dir/$target/release/zaxnes"
fi

[[ -x "$binary" ]] || { echo "error: $binary was not built" >&2; exit 1; }

# The package must load on the oldest firmware: refuse a binary that needs a newer glibc, or
# any library beyond the base system and the firmware's SDL2.
objdump="$(command -v aarch64-linux-gnu-objdump || command -v objdump)"
readelf="$(command -v aarch64-linux-gnu-readelf || command -v readelf)"
newest="$("$objdump" -T "$binary" | grep -o 'GLIBC_[0-9.]*' | sort -Vu | tail -n 1 || true)"
if [[ -n "$newest" && "$(printf '%s\n' "${newest#GLIBC_}" "$glibc" | sort -V | tail -n 1)" != "$glibc" ]]; then
    echo "error: $binary needs $newest, newer than glibc $glibc" >&2
    exit 1
fi
unexpected="$("$readelf" -d "$binary" | sed -n 's/.*Shared library: \[\(.*\)\]/\1/p' \
    | grep -v -E '^(libc|libm|libdl|libpthread|librt|libgcc_s)\.so|^ld-linux-aarch64|^libSDL2-2\.0\.so\.0$' || true)"
if [[ -n "$unexpected" ]]; then
    echo "error: unexpected dynamic dependencies: $unexpected" >&2
    exit 1
fi
echo "binary: max $newest, dependencies ok"

stage="$out_dir/stage"
port="$stage/zaxnes"
rm -rf "$stage"
mkdir -p "$port/licenses" "$port/profiles" "$port/roms"

install -m 755 "$binary" "$port/zaxnes.aarch64"
cp "$root_dir/LICENSE" "$root_dir/THIRD-PARTY-NOTICES.md" "$root_dir/ASSETS-LICENSES.md" \
    "$port/licenses/"
if grep -q $'\r' "$script_dir/portmaster/Zaxnes.sh"; then
    echo "error: Zaxnes.sh has CRLF line ends; the firmware's bash needs LF" >&2
    exit 1
fi
cp "$script_dir/portmaster/Zaxnes.sh" "$stage/Zaxnes.sh"
cp "$script_dir/portmaster/zaxnes.gptk" "$script_dir/portmaster/port.json" \
    "$script_dir/portmaster/gameinfo.xml" "$script_dir/portmaster/README.md" \
    "$script_dir/portmaster/screenshot-placeholder.txt" "$port/"
chmod 755 "$stage/Zaxnes.sh"
printf '%s\n' "Put your 3dSen profiles (.3dn or converted profile files) here." \
    > "$port/profiles/README.txt"
printf '%s\n' "Put your .nes files here." > "$port/roms/README.txt"

zip_path="$out_dir/Zaxnes.zip"
rm -f "$zip_path"
(cd "$stage" && zip -q -r -X "$zip_path" Zaxnes.sh zaxnes)
rm -rf "$stage"
echo "package: $zip_path"
