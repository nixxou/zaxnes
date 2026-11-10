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
- [ ] 4. `.fm2` in the application.
- [ ] 5. The app with a real movie, end to end.
- [ ] 6. Docs.
- [ ] 7. Final pass.

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
