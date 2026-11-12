# FCEUX-48 — `.fm2` playback on FCEUX (FCEUX-4) and the heap overflow (FCEUX-8)

Branch `feat/fceux-engine`, worktree `cleanroom/wt-fceux2`, base `c9c73033` (lot FCEUX-23).
Fork `cleanroom/fceux-zax`, branch `zax`, base `61c7b080`.

Design note: `docs/review/FCEUX-ENGINE.md` §5 (`.fm2`), §7 (the overflow and the vetting), §9 (the
lots). Previous lots: `docs/progress/FCEUX-1.md`, `FCEUX-6.md`, `FCEUX-23.md`.

Why this lot exists, in the owner's words: in the profile editor, **play a complete TAS (`.fm2`)
and edit the 3dn profile on the fly**. `.fm2` movies only stay in sync on the engine that recorded
them, and the 3D on FCEUX matches Zaxnes's wherever the two machines agree (§6 of the design note).

## Steps

- [x] 1. The FCEUX-8 fix in the fork.
- [x] 2. The validated-games marker.
- [x] 3. `.fm2` playback on FCEUX, in the core.
- [x] 4. `.fm2` in the application.
- [x] 5. The app with a real movie, end to end.
- [x] 6. Docs.
- [x] 7. Final pass.

## The environment this lot ran in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-fx` |
| `TMP` / `TEMP` | `<worktree>/rust/target-fx/tmp` |
| the fork, MSVC | `cleanroom/fceux-zax/build-msvc` |
| the fork, ASan | `cleanroom/fceux-zax/build-asan`, a WSL (Ubuntu 24.04, g++ 13.3) tree with `-fsanitize=address -g` |
| `ZAXNES_ROMS` | `G:/3dsen/demo/roms` |
| `ZAXNES_TEST_ROMS` | `G:/3dsen/cleanroom/assets-free/test-roms` |
| `ZAXNES_PROFILES` | `G:/3dsen/cleanroom/assets-neutral` |
| SDL2 | `--features zax-app/bundled` |

---

## Step 1 — the CHR-RAM heap overflow (FCEUX-8)

Fork commit `c9f388c3`. `src/ines.cpp` and `zax/README.md`; the whole diagnosis is in the fork's
`zax/README.md` § *The CHR-RAM overflow* and the design note's rewritten §7.

### The brief was right that §7's diagnosis was wrong, and in both directions

| §7 said | what it is |
|---|---|
| "a one-byte heap overflow" | ASAN reports the **first** byte outside the buffer. The whole of `$0400-$1FFF` is outside it |
| "the apparent one-line fix is to pass `mCHRRAMSize` to `SetupCartCHRMapping`" | that leaves `CHRmask8` at 0 for an 8 KiB chip, so the window is unchanged and **the overflow stands** |

The cause is `setchr8()` (`src/cart.cpp:262-274`): all eight 1 KiB `VPage` slots are pointed at one
**linear** 8 KiB window from the chip base, `VPageR[x] = &CHRptr[r][V << 13]` with no per-slot
offset, while `SetupCartCHRMapping` computes `CHRmask8 = (size >> 13) - 1` and **clamps it to 0**
when that underflows (`src/cart.cpp:133,138`). A chip under 8 KiB therefore gets an 8 KiB window
over a smaller buffer. `setchr4` and `setchr2` clamp the same way below 4 KiB and 2 KiB; at or above
the window the mask is sound even for a non-power-of-two size, since `V <= (size >> n) - 1` gives
`(V + 1) << n <= size`.

`oam3.nes`: header `4E 45 53 1A 01 00 70 08 … 04 …` — mapper 7 (ANROM, which calls `setchr8`),
NES 2.0, byte 11 `0x04` so `vram_size = 64 << 4 = 1024` and `battery_vram_size = 0`.

### The fix

`src/ines.cpp:1217-1234`: pad the allocation to **8 KiB**, keep `FCEU_MemoryRand(VROM, CHRRAMSize)`
on the **declared** size, `memset` the padding to zero, and keep passing the declared size to
`SetupCartCHRMapping` and `AddExState`. The three things that keeps intact, each of which a
different-looking fix would break:

* **movies.** `FCEU_MemoryRand` (`src/fceu.cpp:990-1020`) draws one `xoroshiro128plus` value per
  byte under `RAMInitOption` 3, so filling the declared size consumes exactly the values it
  consumed before. §5 of the design note is about a movie's header carrying that option, so this is
  not a hypothetical;
* **savestates.** `AddExState(VROM, CHRRAMSize, 0, "CHRR")` keeps the block at its old length;
* `iNES_ExecPower` (`src/ines.cpp:78-79`) re-fills the declared size on every power-on and leaves
  the padding alone, so the padding stays zero for the life of the cartridge.

It carries **no `// ZAX:` marker**, deliberately: it is a bug in FCEUX rather than an adaptation for
Zaxnes, and the owner is to submit it upstream as it stands. `zax/README.md`'s change table says so
and now lists five `src/` files rather than four.

**What the fix is not.** It makes the access safe, not accurate: above the declared size the PPU
sees the zero padding, where `zax-nes` mirrors (`cartridges/mod.rs`'s `wrap_offset`, modulo the
buffer length — the hardware's own behaviour for a 1 KiB chip). Making `setchr8` mirror per slot is
a change to a function every board uses and an emulation decision; it is not needed to close the
memory error and was not taken.

### Verified

| | |
|---|---|
| `oam3.nes`, ASan, 3 000 frames | **clean**. Without the fix it faults at about frame 300 — reproduced first, so the test is known to be able to fail |
| `oam3.nes` on MSVC and on ASan/Linux | `frames=3000 pixels=f2810f098f36bc51 emphasis=0f7bf6886a9c2325 audio=3d5558af9c5e2ee5 samples=2395928`, `state: hint=85250 written=72137` — **the same on both**, which it could not be before: the overflow read whatever the allocator had put after the block |
| `nestest.nes`, 60 frames, both builds | `pixels=e455081e6a3eefe9 emphasis=d395e456e4e72325 audio=7fc10a9ce4ed8fcd samples=47917`, `state: hint=93304 written=79296` — the fork's recorded digests, unchanged |
| the MSVC library rebuilt | `zax\build-msvc.cmd`, ok (the script's `vswhere.exe` line prints "not recognized" and falls through to its own search; harmless and pre-existing) |
| every `zax-fceux` test, feature on, `--include-ignored` | **35 passed**, 0 failed, 0 ignored — 15 `fceux_core_tests` (including `the_tile_capture_does_not_change_what_fceux_draws`) and 20 `fceux_engine_tests` |

### Everywhere else CHR-RAM is registered

`grep -rn SetupCartCHRMapping src/` gives 42 sites. Every one with a size under 8 KiB was read; the
table is in the fork's `zax/README.md`. The summary:

* **the UNIF path** (`src/unif.cpp:535-547`) takes 8, 16, 32, 128 or 256 KiB — **8 KiB is the
  floor**, so it has no sub-window chip at all, and `UNIFGI`'s `memset(UNIFchrrama, 0, 8192)` on
  power (`:575`) is in bounds for the same reason. Nothing to fix;
* **mapper 77** (`src/boards/77.cpp:69`) is the one sub-8 KiB chip addressed by anything wider than
  `setchr1r`: 6 KiB, with `setchr2r(0x10, 0x0800, 2)` and `setchr4r(0x10, 0x1000, 0)`. Both fit —
  `CHRmask2 = 2` puts the 2 KiB window at `[4096, 6144)` and `CHRmask4 = 0` the 4 KiB one at
  `[0, 4096)`. Safe, and **not** touched;
* every other sub-8 KiB chip — MMC3 mappers 74, 165, 191, 192, 194, 195 at 2 or 4 KiB, mappers 252
  and 253 at 2 KiB, and `n106`'s `NTARAM` at 2 KiB — is addressed **only** by `setchr1r`, whose
  mask is 1 or 3 and never clamped. Safe;
* `fds.cpp` and the rest are 8 KiB or more, or a window of an already larger buffer.

### The two engines on `oam3.nes`, after the fix

`zax-harness fceux-tiles`, frames 60 / 300 / 1 200 / 3 000:

    1 of 1 ROMs identical at every compared frame

1 024 records at every frame, **identical in order**, pictures identical. `first_pixel_diff` is 3,
so the two machines do part early and come back together; the padding-against-mirroring difference
of the fix is therefore **latent** on this ROM rather than absent, and stated that way rather than
claimed to be harmless in general.

And the scope of the whole change, measured rather than argued: of the **446** iNES images in
`assets-free/test-roms`, `demo/roms` and `test-originals`, **exactly one** declares a NES 2.0
CHR-RAM under 8 KiB — `oam3.nes`, 1 024 bytes, mapper 7. No other ROM in reach can have moved.

---

## Step 2 — the validated-games marker

Commit `ec55c48a`. `crates/zax-app/src/emulation/fceux_validated.rs` and the three places the
marker shows, with `tests/fceux_validated_tests.rs` beside the module's own unit tests.

### What identifies a game, and why that one

The **CRC-32 of the ROM data** — PRG and CHR with the iNES header, a trainer and any overdump left
out, which is the CRC the No-Intro set gives: `zax_nes::builtin_patches::data_crc`. The brief asked
for "the way the app identifies a ROM for its profile — CRC/hash — never by file name", and reading
the application first turned up four different identifiers, which is worth recording for the next
lot:

| identifier | where | what it keys |
|---|---|---|
| the absolute ROM **path** | `host/game_options.rs:13` `game_options_key` | the per-game options and the per-game profile in `games.json` |
| a profile **id** (SHA-256 of the *profile file* when the `.3dn` carries none) | `zax-profiles/.../user_folder.rs:508` | a library entry |
| the ROM data **CRC-32** | `zax_nes::builtin_patches::data_crc`, cached per file in `persistence/rom_crc_cache.rs` | the built-in patches and the widescreen hack certificates |
| the ROM **SHA-1** of PRG+CHR | `persistence/save_state_file.rs:389` | savestates, movies, battery RAM |

So "the way the app identifies a ROM for its profile" is literally its **path**, which is exactly
what the brief forbids. The CRC is the one the application already uses for **per-ROM facts about
the bytes** — a patch fits this image, a hack pack is present in it — which is what a validation
verdict is, so that is the one taken. It also costs nothing: `certify_rom` already fills the cache
at every launch, before the load job begins, so the interface reads no file while it draws.

### What is in the table

The **nine games §6 of the design note marks ship**, by the dumps each verdict rests on: *Mega Man*
6, *Mega Man 2* 8, *Mega Man 3* 9, *Ninja Gaiden III* 4, *Metroid* 4, *Blaster Master* 5 of its 6,
*Final Fantasy* 7, *Adventure Island* 3, *Punch-Out!!* 1 — **47 dumps**.

The membership was **measured rather than copied from the prose**: `fceux-tiles` was re-run over
the 98 originals at frames 60, 300, 1 200 and 3 000 after step 1's fork change, and it gives
**37 of 98 identical at every compared frame** — §6's own figure, to the ROM, which is also a
re-verification that step 1 moved nothing. Two things that reading settled, and that a lot working
from §6's prose alone would have got wrong:

* *Blaster Master*'s "5 of 6 dumps" are four `Blaster Master` dumps **plus `Chou-WS-M [Japan]`**,
  which is the Japanese release of the same game; the one that is not identical is `[Europe]`, and
  it is **not** in the table;
* *Adventure Island*'s three dumps include `Takahashi Meijin no Bouken-jima [Japan]`, and **none**
  of the three is `identical` at every frame — all three are the OAM rotation §6 names. So all
  three carry the deviation rather than one of them being clean.

Three dumps' worth of deviation, each written once and attached to the dumps that have it:

| deviation | dumps | what §6 says |
|---|---|---|
| the two cores part at frame 3 000 | *Metroid* ×4, *Punch-Out!!*, *Rockman 2* ×2 | identical at 60, 300 and 1 200 |
| palette entry `$3F0B` early | every *Final Fantasy* dump | all 960 records identical; 0.69 % of the 3D picture, and frame 1 200 is 100.000 % |
| the OAM rotated by a constant | *Adventure Island* ×3 | renumbers sprites without moving them; 99.999 % |

What is **not** in it is in the module's doc comment with the reason, because the absence is the
information: *Super Mario Bros.* (borderline, 89.647 % at 1 200), *Castlevania* and *Excitebike*
(no), *Super Mario Bros. 3* and *Zelda II* (**blocked on the `ppu-2007-render-read` decision** —
a *decision* away from the table rather than a measurement away), the one *Blaster Master* dump,
and every game with no 3dn profile.

### A marker, not a gate

The engine option stays available for **every** game, which is the owner's choice. The marker has
three places:

1. **the engine row**, which stays **live** and carries a note beside it, in both languages — the
   mechanism is `options::fceux_validation_note`, built on the same pattern as
   `forced_run_ahead_note` (a note on a live row) and deliberately **not** `inert_reason`, which is
   what greys the four rows FCEUX cannot honour. A test asserts `inert_reason(EmuEngine, …)` is
   still `None`, so the option cannot become a refusal by accident;
2. **the notification at every load** on an engine other than Zaxnes's, which lot FCEUX-23 started
   and which now finishes its sentence: "Playing on the FCEUX engine; this ROM is not validated for
   3D on it.";
3. **the performance overlay's machine line**, which gains `validated` or `NOT validated for 3D`
   after the engine's name.

**"Not validated" and "nothing is known" are kept apart** everywhere — `Status::NotValidated`
against `Status::Unknown` — because a ROM whose CRC was never read deserves no claim either way,
and printing the second as the first would be a false statement about a measurement. `Unknown`
prints nothing.

The table and its tests are compiled in **both** builds, which is deliberate: it is a fact about a
measurement rather than about the `fceux-core` feature. Only the row's note is gated, through the
`fceux_is_the_engine` predicate that was already there.

### Checked

| | |
|---|---|
| `cargo fmt --check` | clean |
| clippy `-D warnings`, whole workspace, feature **on** and **off** | clean (the overlay's machine line moved into `engine_validation` to stay inside the line budget) |
| `fceux_validated_tests` | 5 tests, green in **both** builds — the engine-row branch asserts the note with the feature and its absence without |
| the module's unit tests | 6 more: no duplicate CRC, sorted by CRC, the nine games with their dump counts, the five rejected games absent, a lookup giving the game and the deviation, and the three statuses told apart |
| `desktop_options_golden_tests`, `options_window_tests`, `engine_choice_tests` | 32 tests green — the shown-rows golden is unchanged, the marker adding no row |

---

## Step 3 — `.fm2` playback in the core

Fork commit `a864bf52`; the Rust side is the commit this section is in. The fork's
`zax/README.md` § *`.fm2` movie playback* has the C side's whole reasoning.

### The C API, and the one patch

Six calls: `zax_movie_inspect`, `zax_movie_play`, `zax_movie_stop`, `zax_movie_mode`,
`zax_movie_length`, `zax_movie_frame`, plus a `zax_movie_info` struct. §5 of the design note
estimated "three more C API calls"; it is six because a **refusal has to be nameable** — a host
that can only say "unsupported" is no better than one that plays the movie wrong — so inspecting a
header without touching the machine is its own call, and the length and the frame are separate
queries rather than one.

One patch of `src/`: `zax_movie_override`, a consumed-once `FCEUFILE *` checked where
`FCEUI_LoadMovie` opens its path, which is the mechanism `zax_rom_override` already was for a
cartridge image. The alternative would have been to reimplement `FCEUI_LoadMovie`'s tail in
`zax/`, which §5 rules out by asking for FCEUX's own movie mode, and which would have had to be
kept in step with it for ever.

### Why FCEUX's movie mode, confirmed rather than assumed

§5's reason is `PowerNES` reseeding the random fill from `rand()` on every power-on **unless a
movie is active**. `a_seeded_random_fill_is_reproducible_from_inside_a_movie` is that claim as a
test: two movies with `RAMInitSeed 12345` give byte-identical work RAM over all 2 KiB, a third with
seed 999 does not, and the fill is neither all zero nor all `$FF`. Outside movie mode the API
cannot do that at all, which is why `RamInit::Random`'s own doc comment warns about it.

### What the header decides, and what is refused

Everything §5 lists, and all of it is FCEUX's own `FCEUI_LoadMovie` rather than ours: `palFlag`,
`RAMInitOption`/`RAMInitSeed`, the port devices, and the power-on-or-savestate start.

Refused, with the thing **named** — and judged from the header *before* anything is committed,
because `FCEUI_LoadMovie` re-powers the machine as its third act and a refusal after that would
leave the player's game restarted:

| the header says | the message names |
|---|---|
| `FDS 1` | "a Famicom Disk System disk" |
| `NewPPU 1` | "FCEUX's new PPU" |
| `fourscore 1` | "a Four Score (four pads)" |
| `microphone 1` | "the Famicom microphone" |
| `port0`/`port1` not `SI_GAMEPAD` | "an input device other than a standard pad" |
| `port2` not `SIFC_NONE` | "an expansion-port device" |
| a `romChecksum` that is not the loaded ROM's | "recorded on another ROM, so it would not replay" |
| not a version-3 `.fm2`, or truncated | "not a version 3 .fm2 movie" |

A movie that states **no** checksum is played rather than refused: claiming nothing is not
claiming the wrong thing, and a hand-written movie states none.

The port refusal is also what makes the fork's no-op `FCEUD_SetInput` stub **correct** rather than
merely harmless. That stub is called from exactly one place in this build — `movie.cpp:1116`, where
a movie asks for its ports — so refusing every arrangement but two standard pads is what guarantees
the pads `zax_load_rom` installed are the ones the movie wanted.

### Two things the work found

**1. The region again, in the same shape as lot FCEUX-23 step 8's defect.** `FCEUI_LoadMovie`
applies `palFlag` with `FCEUI_SetVidSystem`, which reaches the *emulation* — `PAL`, the scanline
counts, the sound — but **not** `pal_emulation`, the driver-owned global `FCEUI_GetRegion` reads.
So a PAL movie played on a machine last set to NTSC would have run at PAL while the API reported
NTSC, and the host would have paced the video at 60 Hz. `zax_movie_play` now re-asserts the
movie's region with `FCEUI_SetRegion` (which ends in `FCEUI_SetVidSystem` itself) before any frame
runs. `the_region_of_the_header_is_applied` is the test, and it also checks the way back: an NTSC
movie after a PAL one brings the machine back rather than leaving the last one's.

**2. FCEUX's movie clock has two behaviours a host must not guess at**, both now in the wrapper's
doc comments and both pinned by tests:

* `FinishPlayback` runs at the **top** of a frame rather than the bottom of the last one
  (`movie.cpp:1245-1250`), so after a movie's last frame `movie_mode()` still says `Playing` and
  only becomes `Finished` when another frame is asked for. **The clock, not the mode, is the
  signal** a host should use;
* `currFrameCounter++` (`movie.cpp:1329`) is **unconditional** while a movie is loaded, so the
  clock goes on counting past the end — a 5-frame movie reads `(5, 7)` after seven frames. "Over"
  is `frame >= length`, and a host computing a remaining count has to saturate.

Neither is a defect; both would have been a defect in the application had step 4 assumed otherwise.

### The Rust side

`sys.rs` gains four error codes, three mode constants, the `repr(C)` `ZaxMovieInfo` and the six
declarations; `core.rs` gains `MovieMode`, `MovieHeader` with `refusal()`, and
`inspect_movie` / `play_movie` / `stop_movie` / `movie_mode` / `movie_position`. Four new `Error`
variants, `MovieUnsupported` carrying the named reason. Every new `unsafe` block is one FFI call
with a `// SAFETY:` note, in `zax-fceux` and behind `fceux-core`, as the owner's rule requires.

`play_movie` inspects first and refuses in Rust from `MovieHeader::refusal()`, *and* the C side
judges again before committing. The double check is deliberate: the C API is the one that must be
safe whoever calls it, and the Rust side is the one that can name the reason.

### The tests

`crates/zax-fceux/tests/fceux_movie_tests.rs`, **11 tests**, all green. The movies are **built by
hand** in the test file to the `.fm2` format `src/movie.cpp`'s own parser accepts — a version-3
text header and one `|commands|RLDUTSBA|RLDUTSBA||` line a frame, the eight characters being
`MovieRecord::mnemonics` indexed by bit with bit 7 printed first. The brief offered recording
through FCEUX's own recorder instead; that was not taken because its recorder writes to a **path**
(`openRecordingMovie`) and would have needed a second patch of the fork for a build with no
filesystem, and because a hand-built movie pins the format rather than a round trip through our own
writer.

| test | what it holds |
|---|---|
| `a_movie_plays_back_to_the_same_frames_as_the_same_inputs_fed_directly` | the brief's own check: 240 frames of a scripted sequence on `nestest`, fed directly and then played as a movie, give **the same picture digest at every one of the 240 frames**. The pads are held at `0x55`/`0xAA` during playback — a value in no frame of the script — so a movie that failed to override them could not accidentally agree |
| `the_ram_init_option_of_the_header_is_honoured` | `RAMInitOption` 1 and 2, each with the engine deliberately set to the *other* fill beforehand, read back as `$FF` and `$00` at four addresses before a frame has run |
| `a_seeded_random_fill_is_reproducible_from_inside_a_movie` | the same seed gives the same 2 KiB, a different seed does not |
| `the_header_is_read_without_touching_the_machine` | every field of a header read while a game runs; no movie loaded, the frame counter and the picture unmoved |
| `a_movie_the_build_cannot_play_is_refused_by_name_and_changes_nothing` | the six unsupported headers, each refused by `refusal()` *and* by `play_movie`, with the reason in the message — and after all six the frame counter and the picture are where they were |
| `a_movie_recorded_on_another_rom_is_refused` | a stated wrong checksum refused; the same movie without one played |
| `rubbish_is_refused_as_a_format_error_and_a_movie_needs_a_rom` | empty, prose, `version 2`, a truncated magic; and a movie with no ROM loaded |
| `stopping_unloads_the_movie_and_gives_the_pads_back` | stopping mid-movie, twice, and the host's pads live again with the machine where the movie left it |
| `the_region_of_the_header_is_applied` | finding 1 above, both ways |
| `playback_finishes_at_the_last_record_and_says_so` | finding 2 above, both halves |
| `a_savestate_during_playback_carries_the_movies_clock` | a state at movie frame 40, 60 frames on, the state reloaded, and the **same 60 picture digests** replayed — which is the seam step 4's backward seek stands on, measured here rather than hoped for |

`fceux_core_tests.rs`'s header-drift test grew from 19 constants to 26, so the seven new `#define`s
cannot drift from the header without a test failure.

| | |
|---|---|
| `cargo test --release -p zax-fceux --features fceux-core`, ignored included | **46 passed**, 0 failed (35 before this step) |
| `cargo fmt --check` | clean |
| clippy `-D warnings`, whole workspace, feature on and off | clean |
| the fork's `nestest` digests, 60 frames | `pixels=e455081e6a3eefe9 emphasis=d395e456e4e72325 audio=7fc10a9ce4ed8fcd`, `state: hint=93304 written=79296` — unchanged |

---

## Step 4 — `.fm2` in the application

Commit `5e6397f9`. `emulation/host/own_movies.rs` is the new module; the module's own doc comment
holds the design and is the thing to read first.

### What was found before writing anything

The application already had an `.fm2` path, and reading it first — as the brief asked — turned up
**two** independent `.fm2` implementations in the workspace, which is worth recording because the
names are the same:

| | where | what it is |
|---|---|---|
| Zaxnes's own importer | `zax-nes/src/movie/fm2.rs` (439 lines), with `fcm.rs` beside it | a clean-room reader "from its public documentation (TASVideos)", which `movies.rs`'s `play_fm2` turns into a Zaxnes `Movie` (a synthetic start state plus `fm2.words()`) and plays through `MovieRun` |
| FCEUX's own | `zax-fceux`, step 3 of this lot | FCEUX's movie mode, which is the only one that honours `RAMInitOption`/`RAMInitSeed` |

The second was not wired into the application at all before this step. Everything **around** the
frames, on the other hand, was already built by lot EDITOR-1 and needed nothing: the
`MovieTimeline` and its checkpoints and seek, the movie bar, the pause and the frame step, the 3D
conversion, the profile's Lua hooks (all of which hang off `run_frame`) and `script_reload`. So the
step is a *second source of frames*, not a second movie system.

### The dispatch, and what the default build does

`play_movie` gains one test before the old one: an **`.fm2`** plays on the engine that recorded it
when the build has one. A **`.fcm`** does not — FCEUX's own `LoadFM2` refuses that format outright
("FCM File format is no longer supported. Please use Tools > Convert FCM") — so it keeps Zaxnes's
importer, and so does every `.fm2` in a build without `fceux-core`.

The discriminator is `EngineHandle::plays_its_own_movies()`, which is the five-method group added
to the handle in **two whole `cfg`'d definitions**, one per build, as `EngineHandle::fceux_core`
already was: a build without the feature names no FCEUX type at all and every one of the five is a
constant. Two tests in `engine_parity_tests.rs`, compiled in **both** builds, pin exactly that:
Zaxnes's core plays no movie of its own and answers all five with "not me" rather than failing, and
only a build with the feature has an engine that does.

### The engine the playback forces

`movie_engine: Option<EngineKind>` on the host, read by `ensure_session` in place of
`chosen_engine()`. The game's own `emu.engine` is **never written**, which the test checks through
`engine_for_game` as well as through the store.

Stopping the movie does **not** switch back, and that is a decision rather than an omission: §4's
rule is that an engine change means re-powering the machine and never happens mid-game. A movie
load re-powers anyway, so forcing the engine for it costs nothing; stopping does not, so switching
back would throw away the frame the author is looking at. The game returns to its own engine at the
next load, and the notification says so. A launch needs nothing special for that: stage 0 compares
`session.core().kind()` with `engine_for_game`, which differ, so the session is rebuilt.

### Three things that differ from Zaxnes's movie path, each with its reason

* **no movie explorer.** It replays the movie in a *second* core and FCEUX keeps one machine per
  process (`movie_explorer.rs` says so itself); a Zaxnes explorer would replay the inputs on the
  wrong machine and the timeline's own trust check would throw the work away. Guarded on
  `core.plays_its_own_movies()`;
* **no notes.** An annotation belongs to a Zaxnes movie file and an `.fm2` is never written over,
  so `Ctrl+M` is no longer *offered* (`can_execute` asks the new `movie_takes_notes()`) and the
  command says why if it is reached anyway;
* **no input hook of ours.** `FCEUMOV_AddInputState` loads each record over the pad state inside
  `run_frame`, so the loop feeds its usual word and the engine ignores it. The end-to-end test
  deliberately hands over `InputAction::NONE` during playback, so a path that needed our word
  would fail rather than pass by luck.

### Two defects in existing code that only an engine like FCEUX exposes

Both found by the tests rather than by reading, and both were already there before this step:

**1. `script_reload` left a session with no ROM on any engine but Zaxnes's.** It rebuilds the
session and restores a state taken with `save_state(true)` — and `FceuxEngine::save_state_to`
answers `include_rom_image` as **"no" whatever it is asked**, FCEUX's block being the machine and
not the cartridge. So F5 during an FCEUX game could not have worked at all: the restore would have
been handed a machine with no cartridge. It now reads the ROM again for such an engine, through
`load_rom_into_new_session`, which is the launch's own stage 4 in the same order (patches, ROM,
region, battery) and is shared with the movie path.

*A hazard checked and found absent, recorded so that nobody has to check it again:* the one-FCEUX-
per-process rule could have made the rebuild fall back to Zaxnes silently. It does not —
`GameSession::dispose` takes `self` **by value**, so the old engine is freed before
`ensure_session` runs. Both call sites now say so in a comment, because the opposite would be a
silent engine change under the author.

**2. A movie the engine plays itself cannot survive a state restore either**, for the same reason:
the block is the machine, not the movie mode. The reload therefore **loads the movie again**, which
puts its clock at 0 — and that is what makes a reload during such a playback **always exact**: the
frame the author was on is reachable only by replaying to it with the new scripts, which is what
`timeline.seek_exactly` already does. A movie that can no longer be played after the reload is said
rather than silently dropped.

### Tests

`crates/zax-app/tests/fceux_movie_app_tests.rs`, 6 `#[ignore]`d tests over `LoopHarness` and the
real host, movies built by hand in the file. All green.

| test | what it holds |
|---|---|
| `opening_an_fm2_plays_it_on_fceux_without_changing_the_games_own_engine` | the game launched on **Zaxnes** by its own setting, the movie played on **FCEUX**, `engine_for_game` still Zaxnes, the clock following and the draw list all tiles with meshes |
| `the_movie_pauses_steps_and_resumes` | a paused movie's clock does not advance, `FrameAdvance` moves it exactly one, a resume moves it on |
| `a_backward_seek_replays_the_same_frames` | a seek from frame 300 back to 90 through the engine's own checkpoints, forward again, and the same machine |
| `reloading_the_profile_mid_movie_keeps_the_movie_and_the_engine` | F5 at frame 200 keeps the engine and the movie, seeks back to 200 of its own accord and goes on |
| `a_movie_the_build_cannot_play_is_refused_with_its_reason` | four headers refused with the thing **named in the notification**, the game still running after each |
| `stopping_the_movie_leaves_the_game_running_and_the_next_load_restores_the_engine` | stop, the game goes on on FCEUX, and the next launch is back on Zaxnes |

One measurement note the tests forced, now in `EngineHandle::cycle_count`'s doc comment:
**`cycle_count` on an engine without a counter of its own is the sum of the frames this *session*
has run**, not the cycles since power-on. A seek that replays 200 frames a second time leaves it
200 frames higher than an uninterrupted run at the same frame, so `movie_timeline::machine_digest`
— which hashes it — cannot be used to compare two paths to the same frame on FCEUX. The tests hash
the work RAM and the picture instead, and say why.

---

## Step 5 — the application with a real movie, end to end

`crates/zax-app/tests/fceux_movie_end_to_end_tests.rs`, 4 `#[ignore]`d tests, **all green**.
*Super Mario Bros.* with its own 3dn profile, a movie of **5 400 frames**, headless through
`LoopHarness` over the real `EmulationHost`.

### The movie is a recording, not a file built to the format

The brief asked for a movie "built by recording a scripted input sequence on FCEUX — no download",
and that is literally what the first test does, in this order:

1. the scripted sequence — Start at the title screen, then right with a jump every 96 frames — is
   **fed to FCEUX frame by frame** through the application's own loop, and every frame is hashed;
2. the `.fm2` is written from that same sequence;
3. it is played back through `Command::OpenMovie`, and the two runs are compared **frame for
   frame**.

**The 5 400 digests are identical**, so the file replays the run it was recorded from — which is
the only property a recording has that a hand-written file does not. FCEUX's own recorder was not
used and could not be: it writes to a *path* (`openRecordingMovie`) and the fork's build has no
filesystem.

Two things had to be right for the comparison to mean anything, and the first cost a failing run:

* **the recording has to start at power-on.** A launch leaves the frame counter wherever the pacer
  left it, while a movie's load puts it at 0, and three frames of *Super Mario Bros.* attract mode
  is enough to part the two runs — the first attempt parted at frame 39, the frame before the
  scripted Start. The recording now power-cycles first and asserts the counter is 0;
* **the frames have to be run one at a time**, through `run_session_frame` rather than the pacer,
  so that one call is exactly one frame with exactly that input.

### What the end-to-end run says

| | |
|---|---|
| the movie | **5 400 frames**, *Super Mario Bros.* with `assets-neutral/profiles/Super Mario Bros.bytes` through the catalogue |
| the recording against the playback | **identical at every one of the 5 400 frames** |
| **the 3D path receives records every frame that has anything to draw** | 164 of the 5 400 frames have **no** record, and **every one of them is a blank screen** — the picture is a single colour. *Super Mario Bros.* blanks the screen at a death and between levels, the PPU's rendering off, and this run dies; frames with something on screen and no record: **0** |
| the draw list at frame 5 400 | **282 tiles, 282 with a mesh** |
| pause, step, seek and resume against an uninterrupted playback | **identical** at frames 1 200, 2 400, 3 600 and 5 000, with a pause, 30 idle ticks, two frame steps, a 300-frame seek back and a resume between each pair — and each mark checked again on the way back as well as against the straight run |
| reloading the profile mid-movie | at movie frame 2 000: the engine and the movie kept, the reload seeks back to 2 000 of its own accord, **the machine at 2 000 is byte-identical to the one before the reload**, and the draw list is 272 tiles before and 272 after |
| **frames per second of movie playback with 3D on FCEUX** | **1 726 frames/s, 0.5795 ms a frame** (3 000 timed frames after 240 of warm-up; a second run gave 1 695 f/s) |

The frame-rate figure is the application's own loop, so it carries the core, the tile capture, the
3D conversion and the draw-list build — everything but the renderer. For scale, lot FCEUX-23 step 6
measured the same loop at **2 410 f/s** on *Super Mario Bros. 3* without a movie and **407 f/s** on
Zaxnes; movie playback on FCEUX is therefore about **28× real time**, which is what makes scrubbing
a 5 000-frame TAS in the profile editor usable.

---

## Step 6 — the docs

### `docs/review/FCEUX-ENGINE.md`

* **§5** opens with what this lot did and keeps the plan as written, with what the outcome added
  after it: six C API calls rather than three and why the extra one matters (a refusal has to be
  nameable, and `FCEUI_LoadMovie` re-powers the machine as its third act, so a refusal decided
  after it would restart the player's game); the table of what is refused; the application's side
  with the engine forced for the playback and not for the game; the three things the plan did not
  foresee (FCEUX's movie clock's two behaviours, the region global again, and the two defects in
  the application's own code that only an engine like FCEUX exposes); and the end-to-end figures.
* **§7** was rewritten in step 1 — the real cause, the fix, what it is not, what it was checked
  against, and that **the upstream report is the owner's**. §1's claim that the overflow is live
  was struck at the same time.
* **§9** strikes **FCEUX-4** and **FCEUX-8** from the lot table and records what this lot leaves
  the two that remain: FCEUX-5 gains an argument in its favour (1 726 frames/s makes the state
  churn §3 worried about cheap) and one thing to be careful of (`cycle_count` does not come back
  with a restored state on FCEUX, and `machine_digest` hashes it); FCEUX-7 gains the six movie
  calls and the `ines.cpp` fix to cross-compile, both architecture-blind. Plus the one thing
  neither owns: the patch set over `src/` grew.
* **§10** records that growth, which is the licence-relevant figure: `src/` is now **six files**,
  and the two this lot adds are of **different kinds** — `movie.cpp` is an adaptation marked
  `// ZAX:`, `ines.cpp` is a fix to FCEUX itself and deliberately unmarked so that it can be
  submitted upstream as it stands.

### `rust/docs/MOVIES.md` — a new §6, "`.fm2` on the FCEUX engine"

How to open one (**nothing new to learn** — the file chooses the engine, not the user), what
happens then and what is said on screen, that stopping does not switch back and why, the three
differences from a Zaxnes movie each with its reason, the full table of what is refused with the
message each one gives, that a `.fcm` and a build without the feature keep Zaxnes's own importer,
and the measured figures.

### `HOWTO-RUN.md` (French, the user's guide)

Two additions, both in §3 beside the options they belong with:

* the **engine row** now also says whether the ROM is validated for the 3D on FCEUX, that the list
  is nine games and 47 dumps recognised by the CRC of their data and never by a file name, and
  that it is a marker and not a refusal — a game absent from it loads and plays, and says that
  nobody checked its 3D;
* a new **"Lire un film `.fm2` sur FCEUX"**: that it is the engine's reason for being (play a
  complete TAS and edit the 3dn live), that the three ways of opening a movie are unchanged, the
  three things that happen and are announced, that stopping does not switch back and why, what is
  unchanged (the bar, the pause, the frame step, seeking, the 3D, the Lua, `F5`), the three
  differences in a table, the list of what is refused, and that nothing about the default build
  changed.

### `THIRD-PARTY-NOTICES.md`

The FCEUX entry names lot FCEUX-48 and carries the one figure a distributor needs: **six files of
`src/`**, with the two this lot adds separated by kind — the adaptation and the upstream fix — and
the note that the signalling of the fix upstream is the owner's.

---

## What is left after this lot

**FCEUX-5 — rollback and run-ahead over FCEUX states.** §3 of the design note asks for a
measurement before a history depth is chosen, and this lot has given it a free one: movie playback
with the whole 3D path runs at **1 726 frames/s** on FCEUX, so the ~4.8 MB a rollback second costs
is nothing at that speed. The seam is still the one lot FCEUX-23 cut — one capability flag to
raise and one method to fill — and **rollback already works**, the history having always used the
serialised state. One warning this lot leaves: **`EngineHandle::cycle_count` does not come back
with a restored state** on an engine without a counter of its own (it is the sum of the frames the
session has run), so run-ahead — which compares two paths to the same frame by construction — must
not use it, nor `movie_timeline::machine_digest`, which hashes it.

**FCEUX-7 — Android and PortMaster cross-builds.** Unchanged, and this lot adds nothing hard to
it: six movie calls of plain C++ over FCEUX's own `movie.cpp`, and three lines of allocation
arithmetic in `ines.cpp`. Both are architecture-blind. §8's expectation of "a day of toolchain
work rather than a week of porting" stands.

**What the owner must decide**, in addition to the four lot FCEUX-23 listed (whether `fceux-core`
is on in a release, which is a licence decision first; the sample rate FCEUX cannot produce; which
games FCEUX is worth offering for):

1. **Whether to submit the `ines.cpp` fix upstream, and when.** It is written to be submitted as
   it stands and carries no `// ZAX:` marker for that reason. Until it is upstream, the fork
   carries a divergence in a file it did not before, which a release tarball has to include.
2. **Whether the validated-games list should ever become a gate.** This lot made it a *marker*,
   which was §7's recommendation plus the owner's own rule that the option stays available for
   every game. With the heap overflow closed there is no safety argument left for a gate; what
   remains is a judgement about how much an unvetted 3D picture costs the project in support.
3. **Whether `Super Mario Bros. 3` and `Zelda II` join the list**, which is the
   `ppu-2007-render-read` decision of Q-139 and §6 and not a measurement — with that feature on in
   `zax-nes`, the two engines' records become identical row for row and both games would be
   validated for 3D on FCEUX.

**Two things stated rather than left implied**, as lot FCEUX-23 did:

* the validated list rests on §6's own caveats, which have not moved: one dump per game at frames
  300 and 1 200 **with no button pressed**, and 65 of the 91 dumps never rendered. This lot's own
  5 400-frame run on *Super Mario Bros.* is the first long play-through either engine has been
  compared on, and *Super Mario Bros.* is **not** on the validated list;
* `.fm2` playback is read-only. There is no `.fm2` **writer** anywhere in the workspace, so
  Zaxnes cannot record a movie another emulator can play. Nobody has asked for one; it is recorded
  here because a user who can play a TAS will reasonably expect to be able to make one.

## Step 7 — the final pass

Everything the brief's "final check" asks for, re-run at the end rather than trusted from the step
that first produced it. **Three real things came out of it**, all three found by running the whole
suite in the **feature-on** build — which no earlier step had done end to end — and none of them a
flaky test.

### The three things the pass found

**1. The port rule refused every one-player `.fm2`.** A one-player movie records `port1 0` for the
socket nothing is plugged into; step 3's rule demanded `SI_GAMEPAD` on both pad ports, so such a
movie was refused with a message about an input device other than a standard pad. A large share of
real TASes are one-player. `SI_NONE` is now accepted on either pad port.

Accepting it needed a second change, or it would have been the wrong kind of permissive: FCEUX asks
its **driver** to install the devices a movie's header wants (`FCEUD_SetInput`, `movie.cpp:1116`)
and this build's driver is a no-op stub, so the movie would have been replayed with the API's own
pad still on port 2 and a game that polls that port would have seen buttons the movie never
recorded. `zax_movie_play` now installs what the header asks for and `zax_movie_stop` puts the two
standard pads back. The test is the one that would have caught it: a one-player movie, built with
**no field for port 2** as FCEUX's own format has it, played twice — once with every button of both
pads held down, once with none — and the two runs are identical picture for picture.

**2. A refused movie reset the game.** Switching the session's engine re-powers the machine, and
the refusal was decided *after* the switch, so opening the wrong file cost the player the game they
were playing. `fm2_movie_tests`'s **`a_refused_fm2_leaves_the_game_alone`** is exactly that
contract and is what caught it — a test written long before this engine existed.

The movie is now judged **before anything is committed**: with that engine already in play it is
asked directly (nothing to lose), and otherwise the question goes to a **throw-away engine** of
that kind with the game's ROM loaded into it, dropped before the answer is used. That earned a
distinction worth having:

| | |
|---|---|
| a **header** this engine cannot play | refused, the game untouched, the thing named |
| a **ROM** it cannot load | about the *game*, not the movie — so the other engine's own reader takes the movie, with a warning that a reader which is not the one it was recorded on can drift. Refusing there would leave a playable movie unplayable |

**3. Four tests describe a path a feature-on build no longer takes.** Three in `fm2_movie_tests`
and one in `script_reload_tests` assert Zaxnes's `MovieRun`, its explorer's trust, or a *fast*
reload during a movie — none of which exists when an `.fm2` goes to FCEUX's own movie mode. They
are now compiled only without the feature, each with the reason written where it is, and
`fceux_movie_app_tests` / `fceux_movie_end_to_end_tests` cover the same ground there. **Two
assertions were kept in both builds** because they are about the application rather than about one
engine: the refusal leaving the game alone, and `script_reload_tests`'s "the movie was lost", which
now asks `movie_mode()` rather than `session.movie()`.

### fmt and clippy

| | |
|---|---|
| `cargo fmt --check` | clean |
| clippy `-D warnings`, whole workspace, `--all-targets`, feature **off** | clean |
| the same, feature **on** (`zax-app/fceux-core`, `zax-harness/fceux-core`, `zax-fceux/fceux-core`) | clean |

One note on how, for whoever repeats it: the final clippy runs were made with **`--release`**
rather than in the default dev profile. The disk was at 4.9 GB when the pass began — under the
brief's own 5 GB stop — and a dev-profile `clippy` of this workspace writes a 2.8 GB `debug` tree,
so it was deleted and the release profile used, which reuses the release tree and costs nothing.
The dev-profile run was made too: feature **off** at the start of this pass (clean), and the whole
workspace feature **on** at the end of step 4 and `-p zax-app` at the end of step 5, both clean.

### The test suites

`cargo test --release --workspace` with `--include-ignored`,
`ZAXNES_ROMS=G:/3dsen/demo/roms`, `ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms` and
`ZAXNES_PROFILES=G:/3dsen/cleanroom/assets-neutral`, `--skip three_d_sen_real_install_tests` (those
six want an installation of the original program, which this role must never touch):

| | passed | failed | ignored | FCEUX-23's figure |
|---|---|---|---|---|
| feature **off** | **3 363** | 0 | 0 | 3 349 |
| feature **on** | **3 434** | 0 | 0 | 3 402 |

The feature-on run is `--test-threads 1`: FCEUX keeps one machine per process, and the lot's own
test files serialise themselves, but a single-threaded run is the honest way to take the figure.

### golden-core

`zax-harness golden-core "G:/3DSen/test-originals" <out.jsonl> 1200`, feature off:

    8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773

**Byte-identical** to lot FCEUX-23's baseline, which is the owner's rule.

### `bench-core`, and how the +1 % was actually settled

First, the structural answer: `git diff --stat c9c73033..HEAD` over `zax-nes`, `zax-engine3d`,
`zax-render`, `zax-core-api` **and `zax-harness`** is **empty**. `bench-core` lives in `zax-harness`
and drives `NesCore` directly, so it compiles from byte-identical source and cannot have moved.

That is an argument, so it was measured as well — and the first measurement looked bad: 1.4593 ms
on *Super Mario Bros. 3* against FCEUX-23 step 8's 1.3464, **+8.4 %**. The machine was busy
(something external freed 12 GB of disk mid-pass). The right answer was to measure the baseline in
the **same session**, as step 1 of lot FCEUX-23 did for golden-core: `git archive c9c73033` into a
scratch tree, `cargo build --release -p zax-harness` there, and run the two binaries **interleaved**.

| *Super Mario Bros. 3*, 8 interleaved runs each | min | median | max | spread |
|---|---|---|---|---|
| baseline `c9c73033` | 1.3400 | **1.3522** | 1.3677 | 2.07 % |
| this lot | 1.3416 | **1.3522** | 1.3665 | 1.86 % |

**Median against median: +0.000 %. Min against min: +0.119 %.**

| *Mega Man 3*, 3 interleaved runs each | min | median | max |
|---|---|---|---|
| baseline `c9c73033` | 0.9943 | **0.9972** | 1.0129 |
| this lot | 0.9938 | **0.9990** | 1.0276 |

**Median +0.181 %, min −0.050 %.** Both inside the owner's +1 %, and the lesson for the next lot is
the method rather than the figure: a lone run of this benchmark on this machine carries up to 10 %
of noise, and interleaving two binaries in one session removes it.

### The fork, from clean build directories

| | |
|---|---|
| **MSVC**, `build-msvc` deleted, `zax\build-msvc.cmd` | ok, 500 targets, `zaxfceux.lib` 8 382 750 bytes |
| **Linux (WSL)**, `build-linux` deleted, `sh zax/build-linux.sh` | ok, `libzaxfceux.a` 2 922 224 bytes |
| **ASan (WSL)**, `build-asan` deleted and reconfigured with `-fsanitize=address -g` | ok |

And the digests from all three, which is the point of building them:

| ROM | MSVC | Linux | ASan |
|---|---|---|---|
| `nestest.nes`, 60 frames | `e455081e6a3eefe9` / `d395e456e4e72325` / `7fc10a9ce4ed8fcd`, 47 917 samples, state 93 304 / 79 296 | the same | the same |
| `oam3.nes`, 3 000 frames | `f2810f098f36bc51` / `0f7bf6886a9c2325` / `3d5558af9c5e2ee5`, 2 395 928 samples, state 85 250 / 72 137 | the same | the same, **and no ASan report** |

**The ASan result of step 1 holds from a clean tree**: `oam3.nes` runs 3 000 frames with no
heap-buffer-overflow, where it faulted at about frame 300 before the fix. That the same ROM now
gives the same digests on MSVC, on Linux and under ASan is the strongest statement available: the
overflow used to read whatever the allocator had put after the block, which no two allocators agree
about.

### The end-to-end results of step 5 and step 6, re-run against the clean library

All 28 FCEUX tests green (`zax-fceux` 12 + `fceux_end_to_end` 6 + `fceux_movie_app` 6 +
`fceux_movie_end_to_end` 4), with the figures reproduced:

| | |
|---|---|
| the 5 400-frame movie against the run it was recorded from | **identical at every frame** |
| frames with no record | 164, **every one a blank screen**; 0 with a picture and no record |
| the draw list at frame 5 400 | 282 tiles, 282 with a mesh |
| interrupted against uninterrupted playback | **identical** at frames 1 200, 2 400, 3 600, 5 000 |
| a profile reload at movie frame 2 000 | 272 draw tiles before and after, **the machine identical** |
| **movie playback with 3D on FCEUX** | **1 888 frames/s (0.5295 ms a frame)** — against step 5's 1 726 and 1 695, the machine being quieter now. About **31× real time** |
| step 6 of lot FCEUX-23, re-run | *Super Mario Bros.* 995 records → **252** draw tiles, 252 with a mesh; 19 200 samples after 120 frames; 8 snapshots in 240 frames with the rollback leaving frame 300 |

### The application starts, both builds

| | feature off | feature on |
|---|---|---|
| `zaxnes --help` | exit 0, 91 lines | exit 0, 91 lines, **byte-identical** |
| `zaxnes --list-profiles --assets assets-neutral` | exit 0, 104 lines | exit 0, 104 lines, **byte-identical** |

Both identical between the builds, which is the right answer: the engine is a per-game *setting*
and a movie chooses its own engine from the file, so neither of these two outputs should mention
either.

### The disk

The pass began with **4.9 GB** free on G:, under the brief's own 5 GB stop, and the first thing it
did was free space rather than carry on: `target-fx/debug` (2.8 GB, which only `cargo check` and a
dev-profile `clippy` want) and lot FCEUX-23's `base-591836cc` scratch tree (25 MB) were deleted,
giving 7.6 GB. Something external then freed about 12 GB during the run, and the pass finished with
23 GB. The baseline tree built for the `bench-core` comparison was deleted after it was used.

Anyone repeating this on a tight disk: delete `target-fx/debug` first, use `cargo clippy --release`
and `cargo test --release`, and expect the release tree alone to be about 5.8 GB.
