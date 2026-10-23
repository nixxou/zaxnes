<p align="center">
  <img src="branding/zaxnes-logo.svg" alt="Zaxnes" width="420">
</p>

# Zaxnes

**Zaxnes** is a free, **GPL-3.0 NES emulator that renders games in 3D**: each frame's tiles and
sprites are turned into extruded 3D objects, placed in a scene and drawn with a real camera,
lighting and shadows. A *3D profile* describes, per game, which patterns become which geometry,
and carries Lua scripts that drive the scene while the game plays.

> **Independent project, not affiliated with Geod Studio or Nintendo. NES is a trademark of
> Nintendo.**

Zaxnes is a **clean-room reimplementation**, written from a written specification and from public
documentation only. It is distributed under the **GNU General Public License v3**
([LICENSE](LICENSE)); part of `Zax.Engine3D` is adapted from **3DNes_Engine** (© Geod Studio),
an earlier, publicly released GPL-3.0 version of the 3D conversion engine — the files concerned
carry an attribution header, and that licence is why Zaxnes is GPL-3.0 too.

It contains **no file taken from 3dSen**, the program its specification describes. « Sen » was
the working name used while it was written and is still what the specification, the clean-room
journal and the verification reports call it (AMD-029).

**Status: both verification rounds passed, and the publishable base is in place.** Round 1
(requirement-by-requirement verification of the 673 requirements, `verify/round1/R1-final.md`)
closed with **0 non-conformity**, the round-2 findings were folded back in
(`spec/changes/AMD-021`, `AMD-022`), and phase **P6 — closure** produced the spec index
`spec/SPEC-v1.2-INDEX.md` and the traceability check `spec/TRACEABILITY-CHECK-P6.md`. Phase
**P7a** (`spec/changes/AMD-024-p7-assets-libres.md`, REQ-PUB-001 to 009) made the program
**asset-free**. `spec/changes/AMD-031-retrait-tv.md` then removed the decorative TV box
altogether: the scene is the game's image and its background, and the release ships no asset
file at all.

## No assets from another program, no ROMs, no profiles

Nothing this repository or its release carries comes from another program's build
(REQ-PUB-002). Everything the program shows or plays is **produced by its own code** or read
from your machine at run time **without being redistributed**. The release carries **no asset
file at all** (AMD-031):

- the **skyboxes** « Gradient » and « Fantasy » — gradient, stars, nebula, and a procedural
  sun, moon and cloud layer — are GLSL, not images (REQ-PUB-003);
- the four **interface sounds** of SPEC-11 are synthesised, one distinct timbre per event
  (REQ-PUB-005). There is **no menu music**: REQ-PUB-005 was revised on 2026-10-21 to drop it
  rather than replace the three tracks of the original build;
- the **interface font** is a face already installed on your system — Segoe UI, Arial as the
  fallback — opened at run time and never copied. Without one, Dear ImGui's built-in font takes
  over and the accents and the « ▸ » fall back to ASCII (REQ-PUB-006).

**No ROM is distributed.** Playing a game needs a dump you own legally.

**No game profile is distributed.** The library starts with two generic entries written in
code: « 2D Mode », which plays any ROM without a 3D conversion, and « 3D par défaut (sans
profil) », which converts any ROM with the default geometry of SPEC-04. If you legally hold a
`.3dn` profile, **Fichier ▸ Importer un profil…** copies it into your data folder and it
appears in the library — the tile-encoding key it needs is a constant of the code
(REQ-PUB-007).

### Bring your own 3dSen

If you own 3dSen, its profiles are already on your disk and
`spec/changes/AMD-030-import-3dsen.md` makes them importable in one action: **Fichier ▸ Importer
depuis 3dSen…** — or `--import-3dsen <folder-or-bundle>` on the command line — reads the
profile bundle of **your** installation, checks every profile with the SPEC-03 decoder and
writes the ones that decode, unmodified, into your own profile folder as `<name>.3dn`. They show
up in the library at once, under the title each profile carries.

Nothing is downloaded and nothing is redistributed: only your local installation is read, the
bundle itself is never modified, moved or copied, and no file of 3dSen enters this repository or
the release (REQ-PUB-002 still holds). The reader — a baseline LZ4 block decoder, the public
Unity « UnityFS » bundle format and its serialized files — is written from the format
description of AMD-030 §4 and adds no dependency. Identical profiles are skipped, so a second
import adds nothing; a name clash gets a ` (2)` suffix. See
[HOWTO-RUN.md §9.1](HOWTO-RUN.md) for the user-facing details.

Profiles you already have can also simply be dropped next to the executable:
`spec/changes/AMD-033-dossier-profiles-exe.md` makes Zaxnes scan a `profiles` folder beside
`Zaxnes.exe` in every mode — normal, portable and `--userdata` — on top of your own. The published build ships
it empty, with a `README.txt` and no profile (REQ-PUB-002 still holds). That folder is only ever
read: imports keep writing to your data folder, a missing or read-only one is not an error, and
your own folder wins every clash — same file name, identical contents or same profile
identifier. The library shows which folder each profile came from.

`--assets <dir>` still reads a folder of asset files **you** supply (`profiles.json`,
`cubemaps/`, `meshes/`, `textures/`, `audio/`). It is never looked for automatically, and
nothing in the repository or the release depends on it (REQ-PUB-001).

[`ASSETS-LICENSES.md`](ASSETS-LICENSES.md) lists every asset, where it comes from and under
which licence; [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md) lists the third-party code.

**[HOWTO-RUN.md](HOWTO-RUN.md)** (in French) is the user guide: menus, shortcuts, options, the
save-state layout and the full command line.

## Architecture

One project per responsibility. The first four libraries plus the harness carry **no platform,
windowing, graphics or wall-clock dependency**, hold no mutable static state, and never advance
on their own: the host calls "run one frame". `Zax.Render` draws into a framebuffer the **host
supplies**. That is a hard constraint of `PLAN.md`, so that a libretro frontend can host the
same code later, and `tests/Acceptance/ArchitectureTests.cs` asserts it.

| Project | Role | Platform dependency |
|---|---|---|
| `src/Zax.Nes` | NES core: CPU, PPU with the tile-emission contract, APU, 31 mappers, savestates | none |
| `src/Zax.Profiles` | `.3dn` profile format (protobuf wire format) and catalogue | none |
| `src/Zax.Engine3D` | 2D → 3D conversion: shape recognition, geometry, abstract meshes | none |
| `src/Zax.Scripting` | Lua 5.4 runtime for the profiles' scripts | native Lua only |
| `src/Zax.Settings` | Settings model: keys, domains, defaults, libretro core options | none |
| `src/Zax.Render` | OpenGL 3.3 core renderer drawing into a **host-provided** FBO | Silk.NET.OpenGL |
| `src/Zax.Platform` | Window and GL context (GLFW), input devices, OpenAL output | yes |
| `src/Zax.App` | Desktop host: frame loop, offscreen target, ImGui interface, persistence | yes |
| `tools/Zax.Harness` | Headless driver: mesh and draw-list export, offscreen captures, stress runs | GL for `render` and `stress` only |
| `tests/Zax.*.Tests` | Implementer tests (role R3) | — |
| `tests/Acceptance` | Round-1 verification tests (role R4) | — |

Data flows one way: the core emits one batch of tile records per emulated frame →
`Zax.Engine3D` recognises the shapes and builds the meshes → `Zax.Render` draws the resulting
draw list into the host's framebuffer. Lua scripts hook into the conversion, never into the
core.

## Build, test, run

.NET 10 SDK, Windows x64.

```
dotnet build -c Release
dotnet test  -c Release
dotnet run --project src/Zax.App -c Release
dotnet run --project src/Zax.App -c Release -- --rom <game.nes> --profile "3D par défaut (sans profil)"
dotnet run --project src/Zax.App -c Release -- --list-profiles
```

The suite takes about twenty minutes, most of it spent in the 1080p performance benches of
SPEC-12. It has two halves:

```
dotnet test -c Release                                      everything
dotnet test -c Release --filter "Requires!=OriginalAssets"  the public half
```

No ROM and no path of anyone's machine is in this repository: the tests that play one of the
user's own, legally owned dumps carry `[Trait("Requires", "UserRoms")]` and read the folder the
environment variable **`ZAXNES_ROMS`** names. They are *skipped*, with a reason naming that
variable, until it is set — see `HOWTO-RUN.md` §12, which also covers `build/publish.ps1` and
the `stress` command of the harness.

The tests that need the **original** extracted assets — the 101 embedded profiles and the
cubemaps, meshes and textures that went with them — carry
`[Trait("Requires", "OriginalAssets")]` and are *skipped*, not failed, when that folder is not
beside the solution (REQ-PUB-001). The public half passes on its own: the P7a run with the
folder renamed away is **2 515 passed, 229 skipped, 0 failed**.

The render tests open a hidden OpenGL window, and the ones that need a GL 3.3 core context or
one of the user's dumps report themselves as *skipped* rather than failing when they are not
available. Two benches (`Q-042`) measure wall-clock time and can go over budget when the machine
is busy with the rest of the suite; rerun them alone before reading them as a regression.

Non-interactive entry points, which is how the verification captures were produced:

```
Zaxnes --smoke-test [--smoke-frames <n>]    render n frames and exit with 0
Zaxnes --smoke-emulated <n>                 play n emulated frames (60 = 1 s) and exit with 0
Zaxnes --ui-shots <dir>                     one PNG per interface screen, then exit
Zaxnes --builtin-font                       force the degraded font of REQ-PUB-006
Zax.Harness render --out <dir>              offscreen PNG captures, hidden GL context
Zax.Harness meshes --profile <title> --out <dir>    export the geometry of a profile
Zax.Harness lua-api --out api.txt           the Lua API the scripts really see
Zax.Harness stress [--no-gl] [--threaded]   replay the REQ-ACC-030 bench in a loop (Q-062)
```

Package versions are centralised in `Directory.Packages.props`; shared build settings in
`Directory.Build.props`.

## Windows release

```
powershell -File build/publish.ps1
```

produces a **self-contained** Windows x64 build in `artifacts/zaxnes-win-x64/` (no .NET needed on
the target machine), with `LICENSE`, `ASSETS-LICENSES.md`, `THIRD-PARTY-NOTICES.md`,
`HOWTO-RUN.md` and a `README.txt`. It then refuses the build if any asset file slipped into it
(REQ-PUB-002) and starts it to check it works:

1. the welcome screen comes up and draws 120 frames with no asset folder of any kind;
2. Elevator Action plays **60 s in 3D with no profile**, on the default geometry, with the
   procedural Gradient sky;
3. with `-ImportProfile <file.3dn> [-ImportRom <game.nes>]`, the same again on a profile
   imported into a throw-away user folder — the import path of REQ-PUB-001.

The release carries **no asset file**: everything it shows is produced by the code (AMD-031
removed the one model that used to be shipped).

The captures of the procedural skyboxes and of the welcome screen with both fonts are in
`verify/p7/`; those of the two skies are in `verify/p7/look/`.

## Specification and traceability

The normative corpus lives in `spec/`, outside this solution: `SPEC-00` to `SPEC-12` frozen at
version 1.1, the R0 decisions (`DECISIONS-R0.md`) and the amendments `changes/AMD-001` to
`AMD-031`. **`spec/SPEC-v1.2-INDEX.md`** is the entry point: for every requirement it lists each
decision and amendment that applies to it, and in what order. Requirement ids appear in the
tests that verify them (`// REQ-<MOD>-NNN`), and an ambiguity found while implementing is written
up as a `spec/changes/Q-NNN.md` ticket and referenced in the code as `// Q-NNN`.

`powershell -File tools/traceability-check.ps1` checks that `verify/round1/traceability.csv` covers
every requirement id of the spec and of the amendments; its report is
`spec/TRACEABILITY-CHECK-P6.md`.

## Tooling conventions

- `Nullable` is enabled solution-wide and nullability warnings are **errors**.
- Every project targets `net10.0` and x64; the shaders declare GL 3.3 core.
- Savestates are deterministic and self-contained, with a size bound announced in advance
  (REQ-ARCH-013), which is what a libretro `retro_serialize` needs.
- `artifacts/`, `bin/`, `obj/` and every ROM extension are git-ignored: no dump and no release
  binary is ever committed, and no asset file of any kind is committed either — the test
  `REQ_PUB_002_the_repository_carries_no_asset_file` enforces it.
