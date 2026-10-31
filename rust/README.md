# Zaxnes: the code

The technical side of [Zaxnes](../README.md): how to build it, test it, and how the code is laid
out. Zaxnes 0.3.0 is written in Rust; versions 0.1.0 and 0.2.0 (C# / .NET) are kept on the
[`archive-dotnetversion`](../../../tree/archive-dotnetversion) branch.

## Building

The code is a Cargo workspace in this folder (the toolchain is pinned by
`rust-toolchain.toml`).

```
cargo build --release -p zax-app                      # target/release/zaxnes, system SDL2
cargo build --release -p zax-app --features bundled   # SDL2 built and linked statically
cargo build --release -p zax-app --features bundled,wgpu   # with Vulkan and DirectX 12
cargo run --release -p zax-app -- --rom <game.nes> --profile "3D par défaut (sans profil)"
```

`bundled` needs CMake and a C compiler (the MSVC toolchain on Windows). The release packages
are made by:

| Script | Output |
|---|---|
| `rust/build/package-windows.ps1` | `rust/target/package/Zaxnes-<version>-win-x64.zip` |
| `rust/build/package-linux-x64.sh` | `rust/target/package/Zaxnes-<version>-linux-x64.tar.gz` |
| `rust/build/package-linux-aarch64.sh` | `rust/target/package/Zaxnes.zip` (PortMaster) |

The two Linux scripts use [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) to
target glibc 2.31; each script's header lists what it needs.

### Tests

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

No ROM and no path of anyone's machine is in this repository. The tests that need one of your
own, legally owned dumps or profiles are `#[ignore]`d and read the folders named by environment
variables (`ZAXNES_ROMS`, `ZAXNES_PROFILES`, `ZAXNES_3DSEN_INSTALL`); see
[`LOCAL-CHECKS.md`](LOCAL-CHECKS.md). The reference outputs the core and the renderer
are compared with are in `goldens/`; the public test ROMs they are computed over go in
`test-roms/`, which is not committed.

## Graphics back ends

The desktop builds draw with **OpenGL 3.3** by default, or with **Vulkan** or (on Windows)
**DirectX 12** (Options ▸ Video ▸ Graphics API, or `--backend vulkan|dx12`). In Vulkan,
post-processing tools such as ReShade read the depth of the 3D scene (ReShade and DirectX 12 do
not work together yet); see [HOWTO-RUN.md](../HOWTO-RUN.md). The handheld build uses
**OpenGL ES 3.0**.

## Architecture

One crate per responsibility. The core, profile, conversion, scripting and settings crates
carry **no platform, windowing or graphics dependency** and never advance on their own: the host
calls "run one frame", so that a libretro frontend can host the same code later.

| Crate | Role |
|---|---|
| `zax-nes` | NES core: CPU, PPU with the tile-emission contract, APU, mappers, save states |
| `zax-profiles` | `.3dn` profile format and catalogue |
| `zax-engine3d` | 2D → 3D conversion: shape recognition, geometry, draw list |
| `zax-scripting` | Lua 5.4 runtime of the profiles' scripts |
| `zax-settings` | settings model: keys, domains, defaults, persistence |
| `zax-render` | OpenGL 3.3 / GLES 3.0 renderer drawing into a host-provided framebuffer, and the wgpu (Vulkan, DirectX 12) renderer of the desktop builds |
| `zax-platform` | window, GL context, input devices and audio output (SDL2) |
| `zax-app` | the `zaxnes` program: frame loop, Dear ImGui interface, handheld interface |
| `zax-harness` | headless tools: offscreen captures, mesh export, the Lua API listing |
| `zax-goldens` | readers for the reference outputs the tests compare with |

Data flows one way: the core emits one batch of tile records per emulated frame →
`zax-engine3d` recognises the shapes and builds the meshes → `zax-render` draws the resulting
draw list. Lua scripts hook into the conversion, never into the core.

## Documentation

- [`docs/WIDESCREEN-EXTENSION.md`](docs/WIDESCREEN-EXTENSION.md): the widescreen's emulator side
  (the extra registers the hacks use), and [`docs/hacks/`](docs/hacks) one folder per game's
  widescreen hack.
- [`docs/MOVIES.md`](docs/MOVIES.md): the `.zmv` movies.
- [`build/portmaster/README.md`](build/portmaster/README.md): the handheld build and its controls.

## Specification

The behaviour is described by a written specification in [`docs/spec/`](../docs/spec): `SPEC-00`
to `SPEC-12`, the decisions and the amendments in `docs/spec/changes/`; `SPEC-v1.2-INDEX.md` is
the entry point. Requirement ids appear in the tests that verify them (`// REQ-<MOD>-NNN`).
