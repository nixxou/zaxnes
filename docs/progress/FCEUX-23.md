# FCEUX-23 — the engine abstraction (FCEUX-2) and the per-game choice (FCEUX-3)

Branch `feat/fceux-engine`, worktree `cleanroom/wt-fceux2`, base `dev` `591836cc`.

Design note: `docs/review/FCEUX-ENGINE.md` §2 (the trait), §3 (savestates), §4 (the per-game
choice), §6 (the 3D state). Previous lots: `docs/progress/FCEUX-1.md`, `docs/progress/FCEUX-6.md`.

## Steps

- [x] 1. Bring the FCEUX work onto this branch by cherry-pick; the baseline.
- [x] 2. `zax-core-api`: the types both engines share, a pure move.
- [x] 3. The `Engine` trait, and both implementations.
- [x] 4. The application drives the trait.
- [x] 5. The per-game choice (FCEUX-3).
- [ ] 6. The app with FCEUX, end to end.
- [ ] 7. Docs.
- [ ] 8. Final pass.

## The environment this lot ran in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-fx` |
| `TMP` / `TEMP` | `<worktree>/rust/target-fx/tmp` |
| the fork | `cleanroom/fceux-zax` branch `zax`, its existing MSVC build linked, **not rebuilt** |
| `rust/test-roms` | a directory junction to `wt-fceux`'s copy (gitignored; the set lot FCEUX-6 used) |
| SDL2 | `--features zax-app/bundled`, since no `SDL2.lib` is on the machine's link path |

---

## Step 1 — the cherry-pick, and the baseline

`git cherry-pick c0a97023..feat/fceux-core`, 19 commits in order, no merge. They became
`ff8bd82f..7aaa9755`.

### The one conflict, and how it was resolved

**`rust/crates/zax-harness/src/main.rs`**, at commit `ed74ea0e` *FCEUX-1 (4/8): the Rust side*.
Both sides had grown the same four places since `c0a97023`:

| Place | `dev`'s side | the branch's side | kept |
|---|---|---|---|
| `USAGE` | a `classify` line | four FCEUX lines | **both**, `classify` then the FCEUX four |
| `run`'s match arms | a `classify` arm | `fceux-run`/`-frame`/`-tiles`/`-bench`/`-compare` arms, each `#[cfg(feature = "fceux-core")]` | **both**, `classify` first then the five |
| the help function | `print_help()`, prints, takes `classify::USAGE` as a ninth paragraph; its doc comment records the LUA-MERGE reason it exists | `help() -> String`, returns, takes eight and appends `{fceux_usage}` | **`dev`'s shape** (`print_help`, prints) **with the branch's `fceux_usage`**: the branch's reason for the function (clippy's line budget on `run`) and `dev`'s are the same reason, and `dev`'s doc comment states it, so the one that survives is the one with the history in it |
| the `fceux_usage` block | — | `#[cfg(feature = "fceux-core")]` paragraph list | **kept verbatim**, and the later commits of the branch (`0ed86250`, `0fc216d3`) grew it to four paragraphs on top of this resolution without a second conflict |

Checked against the branch's own file afterwards: `git show feat/fceux-core:…/main.rs` differs from
the resolution by exactly the four `classify` additions and the `print_help` shape, and by nothing
else. Carried forward as the branch had it, deliberately: the FCEUX lines of the `USAGE` **const**
are unconditional on the branch, so a build without `fceux-core` lists five commands it does not
have. That is the branch's behaviour, not a conflict, and not this step's to change.

The remaining 18 commits applied with no conflict; `lib.rs` and `main.rs` auto-merged twice more.

### What the cherry-pick changed, and what it did not

`git diff 591836cc..HEAD --stat`: 22 files, +8 078/−6. **`zax-nes` is not among them.** The only
file of an existing crate that changed apart from the harness's `main.rs`, `lib.rs` and
`Cargo.toml` is `golden_core.rs`, and its whole diff is three `fn` → `pub(crate) fn`
(`collect_roms`, `relative_name`, `hex`) so that `fceux_run` can reuse them. No expression changed,
so golden-core's output cannot have.

### Builds and tests

| | |
|---|---|
| `cargo build --workspace --release --features zax-app/bundled` | ok |
| `cargo build --workspace --release --features zax-app/bundled,zax-harness/fceux-core` | ok |
| `cargo test --release -p zax-fceux -p zax-harness` (feature **off**) | all green, 65 tests, 1 ignored |
| the same `--features zax-fceux/fceux-core,zax-harness/fceux-core` | all green, 89 tests, 4 ignored |

### `fceux-tiles` on *Super Mario Bros.* frame 200

    zax-harness fceux-tiles "…/Super Mario Bros. [World][!!].nes" … --frames 200
    1 of 1 ROMs identical at every compared frame

and the row itself: `"zaxnes":969,"fceux":969,"same_in_order":969,"same_as_multiset":969,`
`"pixels_identical":true,"class":"identical"`. **969 / 969**, as FCEUX-6 step 2 left it.

### The baseline of this lot

**golden-core over the 98 originals** (`G:/3DSen/test-originals`, 1 200 frames, feature off):

| build | sha256 of the `.jsonl` |
|---|---|
| this branch at `7aaa9755` | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773` |
| a clean build of `dev` `591836cc` (`git archive` into a fresh directory, its own target dir) | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773` |

**Byte-identical**, which is what the reasoning above predicted and what the brief asked to be
verified rather than assumed. That digest is this lot's golden-core baseline; every later step is
measured against it.

**`render` baseline** (`--assets assets-neutral`, 640x480, defaults otherwise), the captures step 4
has to reproduce at tolerance 0:

| capture | sha256 |
|---|---|
| `smb-300/Super Mario Bros. [World][!!].png` | `1a04889cc411cc800b98f7af56753af1e6f6b2b8d6ab6f42512e0d67838d7303` |
| `smb-300/… -2d.png` | `e85b4826381fb7e052402dfc1bc760bb98fc0d38289ba1e518774ae1a248bc97` |
| `smb-1200/Super Mario Bros. [World][!!].png` | `ad5bb22c0a408af1b6c39a0e7eb0ea5e33b51eae1712812373aecce31b9d35d7` |
| `smb-1200/… -2d.png` | `14ce068b681fec9c0a4869aa0d24e1e8dd5e392a7b8d2432da1554149ab63ea2` |
| `smb3-300/Super Mario Bros. 3 [USA][!!].png` | `fe4b017918e007f7bc719e1db5183749bdfda4f7196fc42158d30bded14d3dcd` |
| `smb3-300/… -2d.png` | `a4188e6294e22f5a393b5919777110a2231202dda5c8beb291b50a79d11f925e` |
| `smb3-1200/Super Mario Bros. 3 [USA][!!].png` | `31a40a16bca19f5d3f1d18faf1cd8e508782706aae2bba6b095e50a0feab77b9` |
| `smb3-1200/… -2d.png` | `40bdafe9fc9b2a0af1795843bf1007bddb488f2b5dd392457c3c9e6b3ccc2946` |
| `mm3-300/Mega Man 3 [USA][!!].png` | `5fcbfb0a878f5060ebdb8a58d27fefe7df64f8948a90314e595431ff8b84e462` |
| `mm3-300/… -2d.png` | `59252059ecdd5ca6e9dd4a0d26a04cbf5d7bdd28f7667299c1f90b0dd39af392` |
| `mm3-1200/Mega Man 3 [USA][!!].png` | `7eb64ba997213fdde9a548c7564a0f3b661e37617648967940887da716331ef4` |
| `mm3-1200/… -2d.png` | `ee59e944668493ec0692a64708052a82f629feebc93b5a4fdc22047f2a8d872e` |

**`bench-core` baseline**, each game with its own profile, three runs each, every run alone
(i9-13900K):

| ROM | `core run_frame` ms | `total` ms | frames/s |
|---|---|---|---|
| *Super Mario Bros. 3* | 1.1787 / 1.1722 / 1.1699 | 1.4075 / 1.3977 / 1.3917 | 710 / 715 / 719 |
| *Mega Man 3* | (1.2492) / 0.9214 / 0.9364 | (1.3566) / 1.0232 / 1.0420 | (737) / 977 / 960 |

*Mega Man 3*'s first run is in brackets because it is a cold one — 1 135 of its 1 200 frames over
1 ms against 199 and 225 on the two after it, which is the machine and not the code. The figures
step 4's +1 % is measured against are the **medians of the warm runs**: SMB3 **1.3977 ms** total
and **1.1722 ms** of core, *Mega Man 3* **1.0326 ms** total and **0.9289 ms** of core.

---

## Step 2 — `zax-core-api`, the types both engines share

A new crate with **no dependencies of its own**, which is the point: `zax-fceux` must not depend
on the core it is an alternative to, and `zax-nes` is a leaf crate and stays one. It answers the
open question §2 of the design note left (`frame_batch` returning a type that lived in `zax-nes`)
the way the note recommended.

### What moved, and what deliberately did not

| In `zax-core-api` | Came from | Why it is on the boundary |
|---|---|---|
| `TileRecord` | `zax_nes::tile_record` | the record of SPEC-02 §4.1, which both engines emit |
| `FrameBatch`, `VisibleRows`, `STANDARD_ROWS` | `zax_nes::frame_batch` | what `Engine::frame_batch` returns |
| `PaletteSnapshot` (**new name**, `[u8; 32]`) | — | the other half of a batch (REQ-CORE-045). A transparent alias, so no field and no literal changed; it exists because lot FCEUX-6 found 29 frames called identical on their records alone with a palette byte differing |
| `NesRegion` | `zax_nes::nes_region` | `Engine::region` |
| `InputWord` (alias), `button::*`, `PORT_2_SHIFT`, `PORT_COUNT` | the doc comment of `NesCore::run_frame` | `Engine::run_frame`'s argument. The layout was only ever prose; it is now a named type with its bits, and a test |
| `ControllerPortType`, `InvalidPort` | `zax_nes::input` | `Engine::port_type`/`set_port_type` |
| `RomLoadResult`, `RomLoadFailure` | `zax_nes::rom` | what a load returns |
| `InvalidState`, `StateLoadFailure`, `StateLoaded`, `NoRomLoaded` | `zax_nes::state`, `zax_nes::nes_core` | what a savestate returns |
| `AudioError` and the three bounds `MIN_SAMPLE_RATE`, `MAX_SAMPLE_RATE`, `MAX_RATIO_ADJUSTMENT` | `zax_nes::apu` | `Engine::read_audio`. The bounds had to come with it: `AudioError`'s `Display` prints them |

Left in `zax-nes` on purpose, because they are Zaxnes's *machine* and not the boundary: the
savestate **format** (`StateWriter`, `StateReader`, `CoreState`), `InputPorts` and the Zapper
(`ZapperPosition`, whose `is_on_screen` reads `RegionTiming`, and which is not on the trait), the
bus, the PPU, the cartridges, the widescreen extension, the hack packs, the margin decoders and
the VS. System. `Apu2A03::MIN_BUFFER_SAMPLES` stayed too — that is the APU's own buffer rule, not
a shared bound.

### How nothing else had to change

`zax-nes` re-exports every moved type **at the path it was at** — `zax_nes::FrameBatch`,
`zax_nes::rom::RomLoadResult`, `zax_nes::state::InvalidState`, `zax_nes::apu::AudioError`,
`zax_nes::input::ControllerPortType`, `zax_nes::ControllerPortType`, … — and
`Apu2A03::MIN_SAMPLE_RATE`/`MAX_SAMPLE_RATE`/`MAX_RATIO_ADJUSTMENT` are now *defined as* the
crate's constants, so their values and every message are unchanged to the character. The whole
workspace compiled with that alone, before a single use site was touched.

Then the use sites that only need the types moved to `zax_core_api`: 15 files of `zax-app`
(including its 5 tests), 13 of `zax-harness`, 3 of `zax-engine3d`, 1 test module each of
`zax-render` and `zax-scripting`. Everything that needs Zaxnes's *machine* kept naming `zax_nes` —
`NesCore`, `NesBus`, `ppu::SpriteLimit`, `builtin_patches`, `hack_pack`, `movie`, `cartridges`,
`vs`, `wide`, `rom::InesHeader`, `ZapperPosition` — which is why several files now import from
both, and that split is exactly the split §2 of the design note drew.

`zax-scripting`'s `zax-nes` dev-dependency became a `zax-core-api` one: its test support only
ever built a `FrameBatch`.

### One real change beyond the move

`zax-harness`'s `run` was two lines past clippy's hundred once step 1's merge had put `classify`
and the five FCEUX arms in the same match — and the budget counts **source** lines, so the
`#[cfg]` that keeps the five out of a default build does not help. The five moved into their own
`fceux_command(args) -> Option<Result<…>>`, tried from the fallthrough arm (the command names
above are all distinct from theirs), with a `#[cfg(not(…))]` twin that returns `None`. No
behaviour change: same commands, same errors, same usage text.

### What it proves

| | |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets --features zax-app/bundled -- -D warnings` | clean |
| the same `,zax-harness/fceux-core` | clean |
| `cargo test --release --workspace --features zax-app/bundled` | **3 241 passed, 0 failed**, 89 ignored |
| the same `,zax-harness/fceux-core` | **3 265 passed, 0 failed**, 92 ignored |
| `cargo test --release -p zax-core-api` | 9 passed — the new file below |
| **golden-core over the 98 originals, feature off** | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773`, **byte-identical to the step-1 baseline** |

`crates/zax-core-api/tests/shared_types_tests.rs` pins what the move itself could have broken
rather than re-testing the types (`zax-nes`'s own tests do that, through the re-exports): the four
`AudioError` messages to the character and the APU's three constants against the crate's; the
input word's eight bits and port 2's shift; the region's four persisted numbers; what a load
result carries on success and on failure; that `ForeignCore` is a cause distinct from
`CorruptState` (step 5 needs it); that a batch is the records *and* 32 palette bytes; and — the
one that makes "pure move" checkable rather than asserted — that `zax_nes::TileRecord`,
`zax_nes::NesRegion`, `zax_nes::rom::RomLoadResult`, `zax_nes::state::StateLoadFailure`,
`zax_nes::input::ControllerPortType`, `zax_nes::ControllerPortType`, `zax_nes::apu::AudioError`
and `zax_nes::VisibleRows` each resolve to the **very same type** as the `zax_core_api` one.

### A note on the build, for whoever resumes this

`cargo build --workspace --all-targets` in **debug** filled G: and failed with `LNK1318 PDB
LIMIT` and `no space on device`. Use `cargo check --workspace --all-targets` to compile-check and
`cargo test --release` to run; the release profile strips, so it costs a fraction of the disk.

---

## Step 3 — the `Engine` trait, and both implementations

`zax_core_api::Engine`, shaped by §2 of the design note, with `Capabilities`, `Capability` and
`EngineKind` beside it. `Send`, **not `Sync`** — FCEUX keeps one machine in process globals, so a
handle may move to the emulation thread but two threads must never be inside it, and the trait
must not undo `zax-fceux`'s `PhantomData<Cell<()>>`.

### What is on it, and what is not

On it: load / close / is-running / name, power / reset / soft reset, run a frame, the frame
counter and its setter, the 2D picture, the frame batch, read audio and the sample rate (set
**and** get — see below), read and write CPU memory, savestate to and from a buffer with its size
bound, battery RAM, the region, the port types, `capabilities()`, `kind()`, and
`as_any()`/`as_any_mut()`.

Off it, and reached by the downcast after the capability flag has said the engine has them:
`wide_picture_into` and every widescreen field, the hack packs and `options_patch`,
`set_sprite_limit`, `cartridge()`/`bus()`/`ppu()`, `run_frame_ahead`/`restore_real_frame`, and
`capture_core_state`/`restore_core_state` — so **rollback keeps using the fast in-memory path on
Zaxnes**, which is the whole reason this is a downcast and not a trait method FCEUX would have to
fake. `engine_trait_tests.rs` exercises every one of those through the downcast, and that the
downcast answers `None` for the wrong type rather than panicking.

`as_any` is a method rather than an `Engine: Any` supertype, because `Any` demands `'static` — a
promise the trait has no reason to make — and because `&dyn Engine` cannot be upcast to `&dyn Any`
on the pinned toolchain.

### Three additions to the brief's list, each for a reason found while writing it

1. **`sample_rate()` on the trait.** FCEUX's `FCEUI_Sound` takes only 44 100, 48 000, 96 000 and
   0 (`driver.h:152-154`), where `zax-nes` resamples anything from 22 050 to 96 000. A rate
   outside *that* range is refused exactly as `zax-nes` refuses it; a rate inside it that FCEUX
   cannot produce is **snapped to the nearest one it can**, and `sample_rate()` reports what the
   core really runs — so a host feeding an audio device stays in tune instead of playing 22 050 Hz
   samples at 44 100. Without the getter the snap would have been a silent pitch error. **The
   owner should know**: a user who sets 22 050 or 32 000 and chooses FCEUX for a game gets 44 100
   or 48 000.
2. **`export_battery_ram(&self, out: &mut Vec<u8>)` copies** instead of lending `&[u8]`. FCEUX's
   bytes come across the C API and cannot be lent out of a `&self`. `NesCore::export_battery_ram`
   still lends, and a caller holding the concrete core still uses it; the trait method is the one
   place `engine_impl.rs` is not a bare forward.
3. **Battery RAM needed three calls in the fork** — `zax_battery_ram_size`,
   `zax_get_battery_ram`, `zax_set_battery_ram`, committed to `fceux-zax` as `61c7b080` and
   recorded in its `zax/README.md`. FCEUX reaches its save memory only through a `.sav` **file**
   (`FCEU_SaveGameSave`, `cart.cpp:539`) and the core-only build has nowhere to put one; the three
   calls hand over the same bytes without the file, the `SaveGame` buffers of `currCartInfo`
   concatenated, which is the layout `cart.cpp` and `movie.cpp` already use. **No FCEUX file is
   patched** — all three live in `zax/zaxfceux.cpp` — so the fork's patch set is still the three
   files §10 of the design note counts. The fork was rebuilt and its self-test re-run:
   `frames=60 pixels=e455081e6a3eefe9 emphasis=d395e456e4e72325 audio=7fc10a9ce4ed8fcd`,
   unchanged.

### `impl Engine for NesCore` — `crates/zax-nes/src/engine_impl.rs`

Thin delegation, every method `#[inline]`, **no emulation code touched**. Two things worth naming:

* `power()` and `reset()` are the same call, because `NesCore::reset` **is** the power cycle
  (DEC-02-Q3). The trait keeps them apart because FCEUX's are two.
* `NesCore::STATE_MAGIC` and `FOREIGN_CORE_MESSAGE` are now *defined as* `zax-core-api`'s
  `state_magic(EngineKind::Zaxnes)` and `FOREIGN_CORE_MESSAGE`. Same values — `0x304E_4553` and
  the spec's literal string — so every block written before this lot still identifies itself, and
  the two engines cannot drift apart on either.

### `impl Engine for FceuxEngine` — `crates/zax-fceux/src/engine.rs`

`Fceux` stays the bare safe wrapper; `FceuxEngine` is the layer above it, holding what the trait
needs and FCEUX does not keep. **No `unsafe` in the new file**: every FFI call is still in
`core.rs`.

| What FCEUX lacks | What the engine does |
|---|---|
| the ROM's name | keeps the host's |
| `NesRegion::Multi` | keeps the host's choice and gives the core NTSC, since Multi *is* NTSC timing (REQ-CORE-015); without this a round trip through `region()` would silently turn Multi into Ntsc |
| a settable frame counter | an **offset**. FCEUX's counter is the shim's, zeroed on a load and a power cycle but **not** on the RESET line, where AMD-044 REQ-CORE-122 restarts it; computing the offset from whatever the core reports makes all four paths agree without the fork having to change |
| a cycle count | the region's nominal frame, 29 781 NTSC / 33 248 PAL. The application uses the figure for the audio ratio and the profiler, never for emulation |
| records it can lend | copies the C callback's ~970 records into its own `Vec` once a frame, which is what §2 of the note predicted (~23 KB a frame) |
| an audio ring | one of its own, `i16` → `f32` at `/32768`, a fifth of a second deep with the APU's floor of 10 000 (REQ-CORE-071), **entirely or not at all** and an odd stereo buffer refused — `zax-nes`'s two rules, so a host switching engines notices nothing |
| a default sample rate | starts at 48 000, `Apu2A03::DEFAULT_SAMPLE_RATE`; the core itself starts silent |
| a typed load failure | reads the cause off the image's header. See the leniency note below |
| a self-describing state | a header: the engine's magic, a version, the core name, the ROM name, the region, the frame count, then FCEUX's opaque block as a length-prefixed blob. So `consumed` is meaningful (a host may place its own data after it, REQ-CORE-091), `StateLoaded::rom_name` is answerable, and **a Zaxnes block is refused as `ForeignCore` with the spec's own message** rather than as "corrupt" |

The scratch buffers for the audio ring and the state payload are behind a `RefCell`, because
`read_audio` and `save_state_to` are `&self` on the trait — they are on `zax-nes` too, whose ring
sits behind a `Mutex` for the same reason. A cell rather than a mutex because this handle is
`!Sync` by construction, so there is no second thread to lock against; `try_borrow_mut` keeps the
`panic = deny` lint honest.

**Defaults the engine sets for the application**: the tile capture **on** (the 3D path reads it;
the harness's own tools use `Fceux` directly and pay nothing) and the RAM fill **zero**, which is
`zax-nes`'s, and which lot FCEUX-6 measured as worth +6 originals and +15 public ROMs of record
agreement on its own. `FceuxEngine::set_ram_init` keeps it settable, because `.fm2` playback must
use the movie's own `RAMInitOption`/`RAMInitSeed` and **never** zero (lot FCEUX-4).

### Two emulation differences the tests found and now record

* **FCEUX is far more lenient about a ROM image.** A header announcing more PRG than the file
  holds is *loaded* (padded out), and so is a well-formed image on mapper 255 — `zax-nes` refuses
  the first as `TruncatedFile` and the second as `UnsupportedMapper` (REQ-CORE-011). In practice
  FCEUX refuses only an image it cannot recognise as any format at all, so `CorruptHeader` is the
  cause it reports; the classifier's other two branches are there for a refusal of a well-formed
  image, which neither ROM set has an example of. The test asserts the *leniency* so that a change
  upstream fails it rather than passing silently.
* **FCEUX's state block cannot restore without the game loaded.** `zax-nes`'s self-contained block
  can (REQ-CORE-090); FCEUX's holds the machine, not the cartridge. The engine says so in as many
  words instead of failing obscurely, and the application's slot wrapper keeps the ROM anyway.

### What it proves

| | |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings`, feature **off** | clean |
| the same with `fceux-core` on both crates | clean |
| `cargo test --release --workspace`, feature off | **3 250 passed, 0 failed**, 89 ignored |
| the same with the feature | **3 293 passed, 0 failed**, 92 ignored |
| `crates/zax-nes/tests/engine_trait_tests.rs` | 9 tests: the lifecycle, power against soft reset, the frame's three outputs, the region and the ports, the state round trip and its engine stamp, battery RAM against the borrow the core still lends, every capability on, the downcast reaching all six Zaxnes-only features, and that an engine moves to another thread |
| `crates/zax-fceux/tests/fceux_engine_tests.rs` | 19 tests, all through `Engine`: the load and its typed cause, the batch's records and palette, the picture before the first frame, the audio contract and the rate snap, memory, the frame counter, soft against hard reset, the state round trip inside a larger buffer, a Zaxnes state refused as `ForeignCore`, three corrupt blocks refused with the engine left stopped, `Multi` round-tripping, the Zapper refused, the capability reasons, the downcast, the RAM fill, and battery RAM on a cart without one |
| **golden-core over the 98 originals, feature off** | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773`, **byte-identical to the step-1 baseline** |

---

## Step 4 — the application drives the trait

The application used to hold `Box<NesCore>`. It now holds **`EngineHandle`**
(`crates/zax-app/src/emulation/engine_handle.rs`), which wraps a `Box<dyn Engine>` and puts every
Zaxnes-only operation the application makes in one file.

### The shape, and why the diff is small

`EngineHandle` implements `Deref<Target = dyn Engine>`, so every call site that only wants the
trait — `frame_count()`, `run_frame()`, `read_memory()`, `try_load_state()`, … — reads exactly as
it did. The Zaxnes-only operations are **inherent** methods on the handle, and an inherent method
wins over a `Deref` one, so `self.core.wide_picture_into(…)` now resolves to the façade's version
without the call site moving. Each of those is a checked `Any` downcast
(`zaxnes()` / `zaxnes_mut()`) with a stated answer for an engine that has not got the feature:

| | on Zaxnes | on another engine |
|---|---|---|
| `wide_picture_into` | the widened picture | the plain 256 x 240 one, 240 rows |
| `picture_shift`, `picture_motion`, `picture_camera` | the core's | `None` |
| `split_steady_sprites` | the split | the picture untouched, the overlay cleared |
| `run_frame_ahead(word, n)` | the exact sequence | the plain frame (run-ahead is off, and the interface says why) |
| `holds_a_real_frame`, `restore_real_frame` | the core's | `false` |
| `bus`, `bus_mut`, `ppu`, `cartridge`, `apu`, `pixel_frame_with_mask` | `Some(…)` | `None` |
| `set_widescreen_margin`, `…_vertical_margin`, `set_sprite_limit`, `set_smooth_scroll_ahead`, `set_builtin_patches`, `set_advanced_patches`, `set_forced_patch`, `set_hack_options`, `set_zapper_position_source` | applied | ignored |
| `builtin_patch`, `options_patch` | the core's | `None` |
| `hack_options`, `applied_components`, `export_flash_save` | the core's | empty |
| `capture_core_state` / `restore_core_state` | the fast in-memory snapshot, `Ok(true)` | `Ok(false)`, and `Capability::FastCoreSnapshot` is what the caller checks |
| `drop_pending_samples`, `set_audio_output_enabled` | the APU's | ignored |
| `audio_reader` | the APU's ring | `None` |
| `speed`, `audio_ratio_adjustment` | the core's | 1.0 and 0.0 |
| `set_speed`, `set_audio_ratio_adjustment` | applied | accepted and ignored: one frame's samples a frame whatever the factor says |

Four of them are **derived rather than refused**, so both engines answer:

* `timing()` is `RegionTiming::for_region(region())` — it was `NesCore::timing()`, which is the
  same expression;
* `cycle_count()` is the core's on Zaxnes and the sum of what the frames reported otherwise, which
  is what the audio ratio and the frame profiler use it for;
* `work_ram_into(out)` is `bus().ram()` where there is a bus and 2 048 `read_memory` calls where
  there is not — the snapshot the scripts' `ReadMem` serves while the emulation thread has the
  engine (REQ-LOOP-067);
* `original_image()` is the core's on Zaxnes and the image the handle kept at `load_rom`
  otherwise, so the achievements' ROM hash and the save-state header work on either.

`battery_ram()` returns a `Cow`: **borrowed** on Zaxnes, so the default build copies nothing where
it copied nothing before, owned where the bytes have to come across a C API.

### Where the engine now moves rather than being shared

`CoreWorker` takes an `EngineHandle` instead of a `Box<NesCore>` and hands it back. That is what
makes `Engine: Send` without `Sync` enough: the engine *moves* to the emulation thread for one
frame, which was already the design, and is exactly the rule FCEUX's process-global machine needs.

Two second machines in the application are now **documented as Zaxnes-only**, because FCEUX cannot
have two instances in one process:

* `FramePipeline::spare`, the ROM-less engine that stands in the moved-out field. Nothing is ever
  asked of it, so it is always `EngineHandle::zaxnes_core()`.
* the **movie explorer**, which replays a movie's timeline in a second machine on its own thread.
  It builds its own Zaxnes engine, which is consistent with what `Capabilities` already says about
  a movie's exactness.

### Call sites changed

`achievements` (`CoreMemory`, `start_game`, `reset`, `restore_progress`, `after_frame`, `tick`,
`request_game`, `on_event`, `on_login`, `on_game_loaded`, `hash_image`) and
`movie_timeline::machine_digest` take `&EngineHandle` instead of `&NesCore`, so **the memory the
achievements read is whichever engine's is live** — which is what step 6 checks. `hash_image` kept
the Zaxnes order of preference (the cartridge where there is one and no patch rewrote the ROM,
`original_image` otherwise) rather than being simplified, so the hash of a patched game does not
move.

`host/mod.rs`, `jobs.rs`, `movies.rs`, `save_states.rs`, `script_reload.rs`, `application.rs`,
`movie_explorer.rs`, `game_session.rs` and nine test files follow the façade. Two substitutions
are worth naming because they had to be checked rather than assumed:

* the 2D path's backdrop colour was `core.ppu().snapshot_palette()[0] & 0x3F` and is now
  `core.frame_batch().palette[0] & 0x3F`. `NesCore::frame_batch` builds its `palette` field *from*
  `snapshot_palette()`, so it is the same byte;
* `bus_mut().apu_mut().drop_pending_samples()` / `set_output_enabled(…)`, in four places, became
  `drop_pending_samples()` / `set_audio_output_enabled(…)` on the handle rather than an
  `Option<&mut Apu2A03>` every caller would have to unwrap.

### What it proves

| | |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings`, both feature sets | clean |
| `cargo test --release --workspace`, feature off | **3 250 passed, 0 failed**, 89 ignored |
| the same with `fceux-core` | **3 299 passed, 0 failed**, 93 ignored |
| **golden-core over the 98 originals, feature off** | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773`, **byte-identical to the step-1 baseline** |
| **the `render` comparison, tolerance 0, minimum 100 %** | *Super Mario Bros.*, *Super Mario Bros. 3* and *Mega Man 3* at frames 300 and 1 200: **12 of 12 captures 100.000 % within 0 levels**, 0 off |

`crates/zax-app/tests/engine_handle_tests.rs` adds 6 tests and one `#[ignore]`d measurement: that
the default build holds Zaxnes with every capability on, that the trait arrives through `Deref`,
that each Zaxnes-only method gives the core's own answer, that `timing()` follows the region and
`cycle_count()` the frames, that the work-RAM snapshot *is* the bus's bytes, and that a savestate
through the handle is byte-for-byte the core's own block.

### Performance, and what `bench-core` can and cannot say

`bench-core` on the two games, four runs each, every run alone:

| ROM | `core run_frame` ms | `total` ms | frames/s | against the baseline's warm median |
|---|---|---|---|---|
| *Super Mario Bros. 3* | 1.1415 / 1.1390 / 1.1453 / 1.1251 | 1.3660 / 1.3599 / 1.3761 / 1.3428 | 732 / 735 / 727 / 745 | median 1.3630 ms against 1.3977 ms: **−2.5 %** |
| *Mega Man 3* | 0.8861 / 0.8947 / 0.8897 / 0.8902 | 0.9895 / 1.0008 / 0.9948 / 0.9939 | 1011 / 999 / 1005 / 1006 | median 0.9944 ms against 1.0326 ms: **−3.7 %** |

Inside the owner's +1 % — on the fast side of it — but **the figure does not measure this step**,
and saying so matters more than the number: `zax-harness`'s `bench-core` drives `NesCore`
**directly**, not through `EngineHandle`, so it times `zax-nes`, which this lot does not touch.
The two negative figures are therefore machine drift between the step-1 run and this one, and they
also say what this machine's noise floor is: a couple of per cent.

So the +1 % question was measured where the abstraction actually is, by
`what_the_abstraction_costs_on_the_applications_own_path`: the same frames of *Super Mario Bros.*
through `EngineHandle` and through `Box<NesCore>`, with the four Zaxnes-only reads the session
makes of every frame, 21 rounds of 1 000 frames each way, alternating:

    EngineHandle against Box<NesCore> over 21 000 frames each:
      median +0.401 %, quartiles -1.662 % to +1.363 %, all rounds together -0.724 %

**The cost is below this machine's measurement floor.** The median is +0.40 %, the pooled figure
is −0.72 %, and the round-to-round spread is ±1.7 % — so the honest statement is not "it costs
0.4 %" but "it is smaller than the noise". That agrees with what the code does: one indirect call
and four `TypeId` comparisons per frame, which is single-digit nanoseconds against a 1.2 ms frame.
**No enum dispatch is needed**, which is the alternative the brief allowed if `dyn` had cost more
than 1 %.

A first attempt at this measurement used 3 and then 7 rounds of 4 000 frames and produced +0.8 %,
−20 %, +16 % and +39 % — one hiccup inside one half of a long round swings the ratio by tens of
per cent. That is why the test runs many short rounds and reports a median and quartiles instead
of a single ratio; the rejected shape is recorded here so that nobody repeats it.

---

## Step 5 — the per-game engine choice (FCEUX-3)

§4 of the design note, line by line.

### The setting

`emu.engine`, a new `SettingId::EmuEngine`: an **Enum** of exactly two values — `0 Zaxnes`,
`1 FCEUX`, and **no "auto"**, because an automatic choice would need a quality judgement the
application cannot make. Scope `User`, which is what makes it both a per-game option (the model
layers a game's own value over the preference, and `games.json` stores it beside the other
per-game options) and a **global default**, which is `zaxnes` too. Effect `NextRomLoad`, so the
options window prints "(at the next load)" beside the row *of its own accord* — that is "the UI
says so", using the mechanism that was already there rather than a new sentence.

**The descriptor is in every build.** Only the row is gated: `options::is_shown` drops
`emu.engine` off a build without `fceux-core`, exactly as it drops `video.backend` off one without
`wgpu`. So a `games.json` written by a build with FCEUX loads in one without it and is written
back unchanged — the value is read and written without error and ignored, which is what the brief
asks for.

### How it takes effect

`EmulationHost::engine_for_game(game_id)` reads the **store** rather than the model, because the
launch has to know at stage 0, before `apply_game_options` has put the new game's options in
force: that is the moment it decides whether the session can be kept. A changed engine joins "a
different game" and "a generic profile" as a reason to rebuild the session, so the machine is
re-powered and the choice never applies mid-game. `chosen_engine()` is the model's value — what
the *next* load will use, and what the options window greys its rows from. `running_engine()` is
what the machine in play actually is, for the marker.

`SessionRequest` gained an `engine` field, and `GameSession::create` builds
`EngineHandle::of_kind(engine)`. `of_kind` **falls back to Zaxnes with a line in the log** when the
engine asked for cannot be made — a build without the feature, or FCEUX already alive — because a
game the user asked for must still start, and the fallback is never silent.

### The disabled controls, and the marker

`options::inert_reason` is the hook that greys a row and prints why beside it, and it was already
there for nine other cases. Four groups join it while FCEUX is the chosen engine, each with its own
reason in English and French:

| rows | key |
|---|---|
| `video.widescreen`, `…_extra`, `…_advanced`, `…_strict_edges` | `hint.fceux_no_widescreen` |
| `emu.builtin_patches`, `emu.forced_patch`, `emu.hack_options` | `hint.fceux_no_hacks` |
| `video.remove_sprite_limit` | `hint.fceux_no_sprite_limit` |
| `emu.run_ahead`, `emu.smooth_scroll_ahead`, `emu.smooth_scroll_view` | `hint.fceux_no_run_ahead` |

**Shown disabled, not hidden** — §4's "a hidden control reads as a bug" — and each reason names
the fix, which is a per-game setting the user owns ("choose the Zaxnes engine for this game").
They read the *setting* rather than the live engine, deliberately: a user who has just chosen
FCEUX should see the consequence at once rather than after reloading.

The **visible marker** has three places, because §6 of the design note is why it matters — a user
who reports "the 3D is wrong" must be able to say which engine drew it:

* a notification **at every load** on an engine other than Zaxnes's, "Playing on the FCEUX
  engine.", with the same line in the log;
* the performance overlay's machine line, which now opens with the engine's name on **either**
  engine (`Zaxnes engine  Ntsc  60 fps nominal  frame 1234`);
* the row itself in the game's options, with its detailed tooltip in both languages.

### Savestates and rollback

The slot header already carried `core_id` and `core_state_version`, hard-coded to
`NesCore::CORE_NAME` and `STATE_VERSION`. They are now **the live engine's**, through
`EngineHandle::state_core_id()` / `state_version()` — on Zaxnes the same two strings and numbers
as before, so every slot written before this lot still matches.

`restore_slot` refuses a mismatch **before** the engine is asked to read the block, with
`save_state_file::OTHER_ENGINE_MESSAGE`: *"This save state was made with the other emulation
engine. Set this game's engine back to the one that wrote it to load the state."* Not
`CORRUPTED_MESSAGE` — the state is not corrupt, and "corrupted" is not something a user can act
on. Each engine would also refuse the other's block itself (its first four bytes name its engine,
`zax_core_api::state_magic`), which is the belt to that braces: a slot whose header was lost
cannot be mistaken either.

**Rollback needed nothing.** The application's history has always used
`core.save_state_to(false, buffer)` with **one reused buffer per slot** — `capture_with` hands the
oldest snapshot's allocation back to be overwritten — so it is the serialised path §3 of the design
note asks for, and it works on FCEUX through the trait unchanged. The cheap in-memory snapshot
(`capture_core_state`) is used *inside* `zax-nes` by `run_frame_ahead`, which `EngineHandle` routes
to for Zaxnes and nowhere else; `Capability::FastCoreSnapshot` is the flag that says so.

**Run-ahead on FCEUX is off** (`Capabilities::FCEUX.run_ahead == false`), as lot FCEUX-5's brief
expects: `EngineHandle::run_frame_ahead` runs the plain frame there and the three run-ahead rows
are greyed with that reason, rather than being slow and silent.

### FCEUX's audio reaches the audio thread

A problem the step had to solve rather than inherit: the audio thread holds a cloned
`zax_nes::apu::AudioReader`, which is `Arc<Mutex<…>>` **into Zaxnes's APU**. FCEUX's samples are in
the engine handle, which is `!Sync` by construction.

So the handle grew a `SharedRing` — `Arc<Mutex<VecDeque<f32>>>`, a fifth of a second at 96 kHz,
oldest sample out when full — which exists **only** on an engine that has no ring of its own. The
session fills it once per real frame (`pump_audio`), and `console_samples()` / `audio_source()`
hand the audio thread either the APU's reader (Zaxnes, the path of before this lot byte for byte)
or a `ConsoleSamples` over that ring. `audio_available()` and `drop_pending_samples()` answer for
both. **The Zaxnes path is unchanged**: same reader, same reads, and `pump_audio` returns at once.

### `RAMInit`

Already settled in step 3 and worth repeating here because the brief asks for it: the engine sets
**zero**, which is `zax-nes`'s fill, in `FceuxEngine::new`, and `set_ram_init` keeps it settable
because `.fm2` playback must use the movie's own `RAMInitOption`/`RAMInitSeed` — or FCEUX's default
fill for a movie that records neither — and **never** zero (lot FCEUX-4).

### Tests, goldens and what they said

`crates/zax-app/tests/engine_choice_tests.rs`, 8 tests that pass **in both builds** and assert the
difference between them: the descriptor's kind, scope, effect, default and its two choices with no
third; the two numeric values and their names; that the setting is stored and read back and
layered as a game's option while the row exists exactly in a build with the feature; that
`of_kind(Fceux)` falls back rather than refusing the game where there is no FCEUX; that the four
groups of rows are greyed with their reason and that each reason is a real sentence in both
languages; that the capability flags and the rows say the same thing; that a slot records its
engine and the refusal is not the corruption message; and that the two block magics differ and
identify themselves, Zaxnes's still being `SEN0`.

Five existing goldens and tests had to learn about the new row, each in the way the file already
handled `video.backend`:

| | |
|---|---|
| `zax-settings/tests/catalogue.rs` | `emu.engine` joins the named list of this reimplementation's own additions (REQ-PERS-001) |
| `goldens/desktop-options-shown.txt` | `emu.engine [fceux-core]`, a second tag beside `[wgpu]`; the test's tag reader became a two-entry table, because clippy folds an `if` on two `cfg!`s into a constant and then objects to it |
| `desktop_options_golden_tests.rs` | the engine joins the graphics API as a row a feature gates, in three places: the layout check, the orphan-tooltip check and the tag reader |
| `options_window_tests.rs` | REQ-UI-202's "every user preference is shown" excepts it in a build with one engine, as it excepts the graphics API without `wgpu` |
| the two string tables | the label, the detailed tooltip and the five new hint texts, in English and French |

| | |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings`, both feature sets | clean |
| `cargo test --release --workspace`, feature off | **3 264 passed, 0 failed**, 90 ignored |
| the same with `zax-app/fceux-core` and `zax-harness/fceux-core` | **3 307 passed, 0 failed**, 93 ignored |
| **golden-core over the 98 originals, feature off** | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773`, **byte-identical to the step-1 baseline** |
