#!/usr/bin/env bash
# Builds the Linux aarch64 package of the Rust port (RG40XXV and similar handhelds), then packs
# zaxnes, the licences, HOWTO-RUN.md and RG40XXV.txt into rust/target/package.
#
#   rust/build/package-linux-aarch64.sh            cross build on an x86_64 Linux host
#   rust/build/package-linux-aarch64.sh --native   build on the device itself
#
# Cross toolchain (x86_64 host, Debian or Ubuntu):
#   sudo apt install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu
#   rustup target add aarch64-unknown-linux-gnu
#   The linker is set in rust/.cargo/config.toml, or export
#   CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc.
#
# SDL2: the build links SDL2 statically (feature `bundled`), so the package needs no libSDL2 on
# the device. Without it, build against the system library of the target instead (a sysroot with
# PKG_CONFIG_SYSROOT_DIR and PKG_CONFIG_PATH pointing at the aarch64 .pc files) and copy
# libSDL2-2.0.so.0 next to zaxnes, with the RG40XXV's own libraries.
#
# Not verified on a device yet: the notes in RG40XXV.txt are the settings to try first.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
rust_dir="$(cd "$script_dir/.." && pwd)"
root_dir="$(cd "$rust_dir/.." && pwd)"
target=aarch64-unknown-linux-gnu

features=(--features bundled)
if [[ "${1:-}" == "--native" ]]; then
    # On the device, the host and the target are the same: no cross linker needed.
    target_args=()
else
    target_args=(--target "$target")
fi

(cd "$rust_dir" && cargo build --release -p zax-app "${features[@]}" "${target_args[@]}")

if [[ "${1:-}" == "--native" ]]; then
    binary="$rust_dir/target/release/zaxnes"
else
    binary="$rust_dir/target/$target/release/zaxnes"
fi

version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$rust_dir/Cargo.toml" | head -n 1)"
name="zaxnes-$version-linux-aarch64"
package="$rust_dir/target/package/$name"
rm -rf "$package"
mkdir -p "$package/profiles"

cp "$binary" "$package/zaxnes"
cp "$root_dir/LICENSE" "$root_dir/HOWTO-RUN.md" "$root_dir/ASSETS-LICENSES.md" \
    "$root_dir/THIRD-PARTY-NOTICES.md" "$package/"
# AMD-033 §3: the profile folder ships empty, with a line saying what to put in it.
printf '%s\n' "Déposez ici vos profils .3dn ; ils sont chargés au démarrage. Aucun profil n'est fourni." \
    > "$package/profiles/README.txt"
cat > "$package/RG40XXV.txt" <<'NOTES'
Zaxnes on the RG40XXV (Linux aarch64, SDL2 on KMS/DRM)
=====================================================

Start from a terminal on the device:

    SDL_VIDEODRIVER=KMSDRM ./zaxnes --size 640x480 --no-audio --rom game.nes

- KMSDRM: SDL2 draws straight to the display with no X11 or Wayland. Leave the
  front-end (EmulationStation or similar) before starting Zaxnes, or the DRM device is busy.
  If the window does not appear, try SDL_VIDEODRIVER=KMSDRM alone, then `--size` at the
  panel's native resolution (640x480 on the RG40XXV).
- OpenGL: the aarch64 Linux build uses GLES 3.0 by itself. ZAXNES_GLES=1 forces the same
  choice on another platform.
- Audio: start with --no-audio if the sound device is busy; otherwise SDL2 uses ALSA.
- Profiles and saves go to the user data folder: $XDG_DATA_HOME/Zaxnes, or ~/.local/share/Zaxnes,
  unless a portable.txt file sits next to the binary (then ./userdata). Use --userdata <dir> to choose another.
- Check the build without a display: ./zaxnes --help and ./zaxnes --list-profiles.
- Licences: LICENSE (GPL-3.0), ASSETS-LICENSES.md, THIRD-PARTY-NOTICES.md.
NOTES

tar -C "$rust_dir/target/package" -czf "$rust_dir/target/package/$name.tar.gz" "$name"
echo "package: $rust_dir/target/package/$name.tar.gz"
