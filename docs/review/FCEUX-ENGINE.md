# FCEUX as a second emulation engine — the design

Written at the end of lot **FCEUX-1**, which was the proof of concept: FCEUX's core now runs under
Zaxnes's harness, produces the same kind of frame and audio, and produces a first version of the
tile records the 3D renderer needs. This note says what integrating it into the application would
take, what each engine would and would not support, and what it would cost — lot by lot.

Everything here rests on what that lot measured; the numbers are in
`docs/progress/FCEUX-1.md` and `docs/review/FCEUX-ENGINE-COMPARISON.md`, not invented here.

## 1. What FCEUX is for, and what it is not for

The reasons to have it are **coverage**, not accuracy:

- a fallback for games Zaxnes emulates badly;
- FCEUX's ~270 mappers against Zaxnes's own list;
- `.fm2` movies that sync, because they are replayed on the engine that recorded them;
- a permanent second opinion on Zaxnes's core.

The comparison settled the accuracy question and it is one-sided. Of the 113 public test ROMs that
report a verdict on both engines, **Zaxnes passes 95 and FCEUX 58**; 37 are passed by Zaxnes and
not by FCEUX, and **none the other way round**. FCEUX must therefore never become the default for
a game Zaxnes already runs: it is the engine you reach for when Zaxnes cannot run a game at all,
or runs it wrongly.

One further fact belongs in any decision to ship it: on the 291 public ROMs, **one takes the FCEUX
process down** — `other/oam3.nes`, through a one-byte heap overflow in FCEUX's own `$2007` write
path (`ppu.cpp:978`, diagnosed with ASAN; §7 below). An engine that can corrupt the heap on a
cartridge image cannot be given an arbitrary ROM in the same process as the user's unsaved game
without a decision about that first.

## 2. The engine abstraction `zax-app` would need

### What the application actually uses

`zax-app` drives the core through `Box<NesCore>` handed to a worker thread
(`core_worker.rs:16,83`) and calls, across `game_session.rs`, `emulation/host/*`,
`achievements/mod.rs`, `input/routing.rs` and `frame_profiler.rs`: `load_rom`, `reset`,
`soft_reset`, `run_frame_ahead`, `restore_real_frame`, `frame_count`/`set_frame_count`,
`frame_batch`, `wide_picture_into`, `read_memory`/`write_memory`, `save_state_to`,
`export_battery_ram`/`import_battery_ram`, `port_type`/`set_port_type`, `original_image`,
`hack_options`/`options_patch`, and `cartridge()`/`bus()`/`ppu()`.

That list splits cleanly in two, and the split is the design:

| | |
|---|---|
| **Engine trait** — both engines implement | load, close, power, reset, run a frame, the frame's picture, the frame's audio, the frame's tile batch, read/write CPU memory, savestate to and from a buffer, region, the input word, the frame counter, battery RAM |
| **Zaxnes only** — not on the trait | `wide_picture_into` and every widescreen field of the batch, the hack packs and `options_patch`, `set_sprite_limit`, `cartridge()`/`bus()`/`ppu()`, the margin decoders, VS. System, `run_frame_ahead`'s exactness |

The second column is not a gap to be filled later: the widescreen extension and the hack packs live
in **Zaxnes's bus** — they are patches applied to the machine, not features of an emulator — and
porting them to FCEUX would mean reimplementing the bus inside the fork. The honest design keeps
them Zaxnes-only and has the UI disable them when FCEUX is the engine for a game.

### The shape

    trait Engine: Send {
        fn load_rom(&mut self, image: &[u8], name: &str) -> Result<RomInfo, LoadError>;
        fn power(&mut self); fn reset(&mut self);
        fn run_frame(&mut self, input: u16) -> i64;
        fn pixel_frame_indices_into(&self, out: &mut Vec<u8>);
        fn frame_batch(&self) -> FrameBatch<'_>;
        fn read_audio(&self, out: &mut [f32], stereo: bool) -> Result<usize, AudioError>;
        fn read_memory(&self, address: u16) -> u8;
        fn write_memory(&mut self, address: u16, value: u8);
        fn save_state_to(&self, out: &mut Vec<u8>) -> Result<(), StateError>;
        fn try_load_state(&mut self, data: &[u8]) -> Result<(), StateError>;
        fn capabilities(&self) -> Capabilities;   // widescreen, hacks, run-ahead, tiles…
    }

Three constraints the trait has to respect, all found by measurement rather than guessed:

1. **`Engine` is `Send` but must not be `Sync`.** FCEUX keeps its whole machine in file-scope
   globals, so there is one per process; the handle may *move* to the worker thread, which is
   exactly what `core_worker` does, but two threads must never be inside it. `zax-fceux::Fceux`
   already enforces this with a `PhantomData<Cell<()>>`, and the trait must not add `Sync`.
2. **One live FCEUX at a time**, enforced rather than hoped for: `Fceux::new` returns
   `Error::AlreadyLive` while another exists. Anything that wants two machines at once — a live
   A/B of the two engines, or run-ahead implemented by running a second copy — cannot be done with
   FCEUX in one process.
3. **`FrameBatch<'_>` borrows.** FCEUX's records arrive through a C callback into a buffer the
   library owns, so the FCEUX implementation has to copy them into its own `Vec` once a frame and
   lend that. The cost was not measured; it is 969 records of 24 bytes for Super Mario Bros., so
   about 23 KB a frame.

A fourth point is a genuine open question for the lot that does this: `frame_batch` returns
`zax_nes::FrameBatch`, which lives in the core crate. Either that type moves to a small
`zax-core-api` crate both engines depend on, or `zax-fceux` gains a `zax-nes` dependency for the
types alone. The first is cleaner and is what this note recommends; it is a mechanical but
wide-reaching change (every `use zax_nes::{FrameBatch, TileRecord, …}` in the app, the renderer and
the harness).

## 3. Rollback and run-ahead over FCEUX savestates

Both already work in principle: `zax_save_state`/`zax_load_state` write to and read from a caller
buffer, uncompressed, and the round trip is tested (`a_savestate_round_trip_brings_the_picture_back`
checks that the picture returns *and* that a sentinel written to work RAM afterwards is rolled
back, so the load cannot pass by doing nothing).

What differs from Zaxnes:

- **Size and speed.** An FCEUX state for `nestest` is **79 296 bytes** with the hint reporting
  93 304. Zaxnes's rollback history uses `capture_core_state`/`restore_core_state` into a reused
  `CoreState`, which is a different and cheaper mechanism than its own `save_state`; FCEUX has only
  the serialised form, so a rollback second at 60 Hz is ~4.8 MB of churn. The lot that does this
  should measure before choosing a history depth, and should reuse one `Vec` per slot
  (`save_state_into` already does, which is why it takes `&mut Vec<u8>`).
- **Run-ahead cannot be done Zaxnes's way.** `run_frame_ahead` + `restore_real_frame` is exact in
  Zaxnes. On FCEUX it would have to be save → run N → take the frame → load, per frame, which is
  correct but costs a state save and load every frame. That is the honest implementation and its
  cost should be stated in the UI rather than hidden.
- **No cross-engine states.** A state saved on one engine must never be offered to the other.
  Whatever slot format the app uses has to record which engine wrote it and refuse the mismatch;
  FCEUX's own `FCSX` magic is a help but not enough, since the app's wrapper is its own format.

## 4. The per-game choice, in the UI and in the settings

The engine is a **per-game** setting, not a global one, because the reason to use FCEUX is always
a particular game. That suggests:

- a per-game option `engine = zaxnes | fceux` beside the existing per-game options, defaulting to
  `zaxnes`, stored where `emulation/host/game_options.rs` keeps the rest;
- a global default (`zaxnes`) and nothing more — no "auto", because an automatic choice would need
  a quality judgement the application cannot make;
- the choice taking effect **on load**, never mid-game: switching engines means re-powering the
  machine, and the UI should say so rather than appear to hot-swap;
- the capability flags of §2 driving the UI: with FCEUX chosen, widescreen, the hack packs and the
  run-ahead slider are shown disabled with the reason, not hidden — a hidden control reads as a
  bug.
- a visible marker while FCEUX is the engine. A user who reports "the 3D is wrong" must be able to
  tell us which engine produced it, and §6 is why that matters.

## 5. `.fm2` playback

This is the clearest win and the easiest lot. FCEUX's `movie.cpp` plays `.fm2` itself; the C API
needs three more calls (open a movie from a buffer, report the frame count and whether playback has
finished, stop) and the RAM-initialisation fields are **already** on the API precisely because a
movie header carries them.

One thing to carry forward, recorded in `zax/zaxfceux.h` and worth repeating: `RamInit::Random`
cannot have its seed pinned from outside, because `PowerNES` reseeds from `rand()` on every
power-on **unless a movie is active** (`fceu.cpp:1031-1039`). That is not a defect to work around:
it is why a movie replays at all, and it means `.fm2` playback has to go through FCEUX's own movie
mode rather than through our own frame feeding.

The other three RAM fills are deterministic, which is what let step 5's comparison mean anything.

## 6. The tile records, and the honest state of the 3D path

**Rewritten by lot FCEUX-6, which did the work this section used to list as remaining.** What
follows is the state after it. The figures are
`docs/review/FCEUX-ENGINE-COMPARISON.md` § *FCEUX-6 tile parity*, and the working is
`docs/progress/FCEUX-6.md`.

### What lot FCEUX-1 left, and what became of it

| FCEUX-1's item | State |
|---|---|
| 1. **Mid-frame scroll** — "the one thing standing between 89 % and near-100 %" | **done** (FCEUX-6 step 2). The fork keeps `zax_row_start` / `zax_row_next` beside `RefreshAddr` — `zax-nes`'s own two fields, maintained by the same rules — and captures the row at the **end** of the line (AMD-021 R2-005). *Super Mario Bros.* frame 200 went from 863 of 969 records to **969 of 969**; the originals identical at every compared frame went from 7 to 41 on this step alone |
| 2. **MMC2/MMC4** latch per column, sprites by x | **done** (step 4), to the requirement (REQ-CORE-039, REQ-CORE-044), and measured to break nothing — but **no mapper 9 or 10 game is in either ROM set**, so it is unproven in use. *Punch-Out!!* has a profile in the catalogue; a lot with the ROM should close this |
| 3. **MMC5** extended attributes and the vertical split | **done** (step 5), and the one thing any available ROM exercised was different: sprite patterns were being read through `VPage` instead of the chip's **sprite** bank set `MMC5SPRVPage`, which `mmc5test` showed as 13 wrong sprite records a frame. Mode 1 and the split are implemented and gate-checked, but **no mapper-5 ROM here ever enters either** (all four stay in CHR mode 0/2/3 with `$5200` at zero over 1200 frames) |
| 4. **The new PPU** | still out of scope, and now scoped: see below |
| 5. **Never** the widescreen fields | unchanged |

Two things the lot found that were not on the list:

* **the palette was never compared.** A frame batch is the records *and* 32 palette bytes
  (REQ-CORE-045) and the 3D renderer reads both. On *Final Fantasy* frame 300 all 960 records are
  identical, `$3F0B` is 32 on Zaxnes and 0 on FCEUX, the 2D pictures are identical and **0.69 % of
  the 3D picture is not**. 21 frames of the originals and 8 of the public ROMs were being called
  identical on this account;
* **the RAM fill was wrong for a record comparison.** `FCEU_MemoryRand` fills the work RAM *and* a
  CHR-RAM cart's CHR with `00 00 00 00 FF FF FF FF` while `zax-nes` clears both, so the two
  machines were not comparable. The tools now default to a zero fill, worth +6 originals and +15
  public ROMs on their own.

### Where it stands

| | 98 originals, frames 60/300/1200/3000 | 291 public ROMs, frames 60/300 |
|---|---|---|
| ROMs identical at every compared frame | 7 → **37 / 98** | 102 → **176 / 291** |
| frames `capture` (ours) | 176 → **34 / 392** | 122 → **3 / 578** |
| of which a capture defect | **0** | **0** |

All 37 remaining capture-class frames are named by cause — a blank screen the pixel test cannot
judge, an OAM array rotated by a constant, one sprite in another OAM slot, or a palette entry — and
each carries the frame at which the two engines' pictures first differed, which is 3 to 10 in every
case. `fceux-tiles` prints the unexplained count, and it is 0.

**Where the records and the palette are identical, the 3D picture is identical**: 14 of 26 rendered
cells come out at **100.000 % of pixels at tolerance 0**, with identical mesh counts, triangles,
opaque tiles and draw calls. Mesh counts match in 23 of the 26 cells, including several games whose
pictures differ. So the 3D question reduces to the record question, with nothing left over.

### Per game: ship-ready for 3D on FCEUX, or not

Judged on the 13 games with a 3dn profile among the 98 originals, frames 300 and 1200.

| Game | 3D on FCEUX | Why |
|---|---|---|
| **Mega Man 1** | **ship** | records and palette identical on all 6 dumps at every compared frame; 100.000 % of pixels, 234 meshes on both |
| **Mega Man 3** | **ship** | identical on all 9 dumps at every frame; 100.000 %, 147 meshes on both |
| **Ninja Gaiden III** | **ship** | identical on all 4 dumps at every frame; 100.000 %. This is the game whose status bar switches the nametable mirroring mid-frame, so it is the best evidence step 2's rule is right |
| **Mega Man 2** | **ship** | 6 of 8 dumps identical at every frame, the other two (`Rockman 2-DWnN`) differing only at frame 3000; 100.000 % at both rendered frames |
| **Metroid** | **ship** | records and palette identical at 300 and 1200 on all 4 dumps; 100.000 %, 194 and 72 meshes on both. 4 of its 16 frames are emulation-class, all at 3000 |
| **Blaster Master** | **ship** | 5 of 6 dumps identical at every frame; 100.000 % at both rendered frames |
| **Final Fantasy** | **ship, with one blemish** | all 960 records identical on every dump at every frame. The *only* difference anywhere is palette entry `$3F0B` at frames 60 and 300, worth 0.69 % of the picture; frame 1200 is 100.000 %. An emulation difference (the machines diverge at frame 3), not a capture one |
| **Adventure Island** | **ship** | 99.999 % at both rendered frames, identical mesh counts and triangles. Its 8 capture-class frames are a 32-entry OAM rotation — the `$4014` transfer starting at a different `$2003` — which renumbers sprites without moving them |
| **Super Mario Bros.** | **borderline** | frame 300 is 100.000 %; frame 1200 is **89.647 %** with identical mesh counts, from sprites held in different OAM slots. Playable, visibly not the same picture |
| **Castlevania** | **no** | 89.785 % at 1200, and the record counts themselves differ (1024 against 1028 at 300) — the only game here where the mesh count differs too. Its dumps diverge in 2D from frame 6 or 13; this is FCEUX against Zaxnes, not the capture |
| **Excitebike** | **no** | 100.000 % at 300 but **86.997 %** at 1200, with 89 meshes against 91. Diverges in 2D from frame 8 |
| **Super Mario Bros. 3** | **blocked on a decision** | 98.5 % at both frames with identical mesh counts. **Needs `ppu-2007-render-read` on in `zax-nes`**: FCEUX applies the hardware rule for a `$2007` access made while rendering unconditionally, `zax-nes` keeps it behind that non-default feature (Q-139) precisely because it moves this game's picture. With the feature on the two engines' records become identical row for row |
| **Zelda II** | **blocked on the same decision** | 89.9 % at both frames, identical mesh counts. Same cause as *Super Mario Bros. 3* |

**The recommendation this section used to make — FCEUX in 2D only — no longer holds.** Eight of the
thirteen games are ship-ready for 3D on FCEUX, two more are one owner decision away, and the three
that are not are failing on FCEUX's *emulation* differing from Zaxnes's, which no amount of capture
work can fix and which the per-game engine choice of §4 already handles: offer FCEUX for the games
in the first group, and Zaxnes for *Castlevania*, *Excitebike* and *Super Mario Bros.*

Two caveats on that recommendation, both stated rather than buried:

* it rests on **one dump per game at frames 300 and 1200 with no button pressed**, which is a title
  screen or a transition on several of them (*Mega Man 3* frame 300 is blank on both engines). It
  says the two engines agree; it does not say how a game looks in play. A lot that wants a shipping
  decision should write `--press` scripts into real gameplay and render those;
* the other 65 dumps were not rendered, because `CLAUDE.md` forbids long GPU campaigns. Their
  answer follows from the sweep — a dump whose records and palette are identical renders
  identically, which is measured, not assumed — but it is derived rather than observed.

### What the capture costs

With the capture **on**, over 20 000 frames each on an i9-13900K, every run alone (FCEUX-6 step 7):

| ROM | Zaxnes, capture on | FCEUX, capture on | FCEUX, capture off |
|---|---|---|---|
| *Super Mario Bros. 3* | 881 f/s | **4 964 f/s** | 5 813 f/s |
| *Super Mario Bros.* | 1 130 f/s | **4 686 f/s** | 5 495 f/s |
| *Mega Man 3* | 1 009 f/s | **4 504 f/s** | 5 308 f/s |

The capture costs FCEUX a steady **17 %** of its frame — 0.03 ms, near identical across boards, as
a fixed ~970 records a frame should be — and FCEUX stays **4.1× to 5.6×** faster than Zaxnes with
both engines capturing. Lot FCEUX-1's "about 6×" was measured with the capture off; the honest
range with it on is 4× to 6×, varying by board. There is no Zaxnes-without-capture figure because
REQ-CORE-033 makes its capture unconditional. Part of FCEUX's 17 % is the binding rather than the
capture: `zax-fceux`'s FFI sink allocates a `Vec` and copies all ~970 records inside the frame,
which a host could avoid by writing into a reusable buffer as `zax-nes` does.

Through the 3D path, `bench-core` headless gives Zaxnes 742, 865 and 986 f/s on the three games
with their own profiles, and the cost after the core is dominated by the **profile's Lua** (0.129
ms on *Super Mario Bros. 3*) rather than by the mesh conversion (0.059 ms) — worth knowing before
anyone optimises the mesh builder. FCEUX through the same path projects to about **3×** Zaxnes; the
harness has no continuous 3D loop on FCEUX, so that one is a projection and labelled so.

### The new PPU, scoped

`newppu` is still out of scope and the capture does nothing there. What hooking it would take, from
reading it:

1. **the two capture moments already exist as positions.** `FCEUX_PPU_Loop` has an explicit
   `for (int sl = 0; sl < normalscanlines; sl++)` with the 32-tile fetch loop inside it, so the
   sprite capture goes at the top of the `sl` body and the background row after the `xt` loop —
   the same two places `DoLine` has them;
2. **the row address has to be assembled, not shadowed.** `newppu` keeps the scroll as five
   separate counters (`ppur.fv`, `v`, `h`, `vt`, `ht`, plus `fh`) rather than a packed 15-bit
   `RefreshAddr`, so step 2's pair becomes a value built from the five at the right instants. Likely
   *fewer* hook sites than the old PPU's five, because `newppu` keeps the latch install explicit
   (`ppur.install_latches()`) instead of folding it into `Fixit1` and `Fixit2`;
3. **the sixteen-byte pattern read transfers unchanged.** `bgdata.main[xt].pt` holds only the two
   pattern bytes of the line being drawn, so the capture still walks the nametable itself, which is
   what it already does and is PPU-agnostic;
4. **the MMC5 half has nothing to mirror.** `newppu` implements **neither** the extended attribute
   mode nor the vertical split — its only MMC5 references are the `MMC5_hb` scanline hook — so a
   capture there would be right for free on CHR mode 0 and wrong in exactly the way `newppu`'s own
   picture is wrong on mode 1 and the split.

## 7. The FCEUX defect found, and what to do about it

`other/oam3.nes` (mapper 7) corrupts the heap from about 300 frames in. ASAN on the fork's Linux
build:

    heap-buffer-overflow, WRITE of size 1
      #0 B2007      src/ppu.cpp:978
      #1 WrMem      src/x6502.cpp:173
    0 bytes after a 1024-byte region allocated by iNES_Init, src/ines.cpp:1220

`VPage[tmp >> 10][tmp] = V` writes one byte past the CHR-RAM. `ines.cpp:1217-1222` pads the
allocation up to 1 KiB — its own comment says the padding is there "to prevent malicious access
from NES software" — but then calls `SetupCartCHRMapping` with the **unpadded** `CHRRAMSize`, so a
1 KiB-granular `VPage` index can still reach `VROM[1024]`.

This is **upstream FCEUX's**, not the fork's: every frame of the trace is FCEUX's own code, no stub
or patch of ours appears in it, and the path is reached by the plain API. The apparent one-line fix
is to pass `mCHRRAMSize` to `SetupCartCHRMapping`, but that changes how a cart with sub-1 KiB
CHR-RAM mirrors — an emulation decision about exotic hardware — so lot FCEUX-1 recorded it and
**left it to the owner** rather than taking it unilaterally.

Three options, in increasing cost:

1. take the one-line fix in the fork and report it upstream (cheapest; needs a judgement on the
   mirroring);
2. run FCEUX in a child process in the shipped application too, as the harness now does
   (robust, and it also bounds any future such bug — but it means a frame and audio transport
   across a process boundary, which is a lot of work and latency);
3. ship FCEUX only for games on a vetted list (cheap, and defensible given that FCEUX exists for
   specific games anyway).

This note recommends **1 plus 3**: fix it in the fork, report it, and still not hand FCEUX an
arbitrary ROM silently.

Note also that a crashing FCEUX child **hung** rather than dying when started by the harness
(Windows fault reporting waiting on something that never answers), which is why `fceux-run` gives
each child a deadline. Any supervising process must do the same; waiting on `exit` is not enough.

## 8. Builds for Android (NDK) and PortMaster (aarch64)

Nothing in the fork is x86-specific: it is plain C++17, it was built and run on **Windows x64
(MSVC 14.43) and Linux x64 (g++ 13.3)** with identical digests, and it needs only zlib, which
FCEUX already vendors. Both targets should therefore be cross-compiles and nothing more. What to
expect:

- **Android (NDK).** `zax-android` already cross-compiles the Rust side; the fork needs a CMake
  toolchain-file invocation per ABI (`arm64-v8a` at least) and the library dropped where
  `zax-fceux`'s build script can find it — `ZAX_FCEUX_LIB` exists for exactly this. The one real
  risk is the MSVC/libstdc++ split: the build script links `stdc++` on non-MSVC, which on Android
  is `c++_shared` or `c++_static`, so that line needs a third case.
- **PortMaster (aarch64).** The same cross-compile with the system zlib. The handheld profile uses
  `target-cpu=cortex-a53`; the fork's CMake takes `CMAKE_C_FLAGS`/`CXX_FLAGS` so the same tuning
  can be passed in.
- **Nothing was attempted in lot FCEUX-1** — no cross toolchain was present and the lot was
  forbidden to download one. These are expectations, not results, and the lot that does them
  should expect a day of toolchain work rather than a week of porting.

### Binary size

Measured, from a release build of the whole core with all ~270 boards:

| | |
|---|---|
| Windows x64, as a DLL | **1.2 MB** |
| Linux x64, as a shared object | **1.4 MB** |
| static archives (unstripped, before linking) | 7.3 MB / 2.8 MB |

So the real cost to the shipped binary is **about 1.2 MB of code**, plus whatever the linker keeps
of zlib. For a desktop build that is noise. For PortMaster it is worth a second thought, and the
boards are the bulk of it: a build that compiled only the boards a vetted game list needs would be
a fraction of that, at the price of a per-target source list.

## 9. The lots this would take

Sizes are relative to lot FCEUX-1, which was one session and covered steps 1-8 of its brief.

| Lot | What | Size |
|---|---|---|
| **FCEUX-2** | The `Engine` trait and the type move to `zax-core-api`; `zax-nes` and `zax-fceux` both implement it; `zax-app` drives the trait. No new behaviour, wide diff. | large — the type move touches the app, the renderer and the harness |
| **FCEUX-3** | The per-game choice: setting, UI, capability flags disabling widescreen/hacks/run-ahead, the engine marker, and the savestate slots recording their engine. | medium |
| **FCEUX-4** | `.fm2` playback: three more C API calls over FCEUX's own movie mode, the RAM-init fields already being in place, and the UI to open a movie. | small — the easiest real win |
| **FCEUX-5** | Rollback and run-ahead over FCEUX states, with the measurements §3 asks for before a history depth is chosen. | medium |
| **FCEUX-6** | The tile capture finished: mid-frame scroll first (item 1 of §6), then MMC2/MMC4, then MMC5, then the new PPU. 3D on FCEUX ships at the end of this, not before. | large, and best split in two |
| **FCEUX-7** | Android and PortMaster cross-builds of the fork, and the size question of §8. | small to medium, mostly toolchain |
| **FCEUX-8** | The `oam3.nes` fix, reported upstream, and whatever vetting §7 settles on. | small |

The order matters: FCEUX-2 gates everything, FCEUX-4 is the cheapest thing with visible value, and
FCEUX-6 is the only one that should hold up shipping — because 2D FCEUX is already useful and 3D
FCEUX is not yet right.

## 10. Licence, and what shipping the library requires

FCEUX is **GPL-2.0-or-later**; "or later" lets it be used under Zaxnes's GPL-3.0, so the combined
work is GPL-3.0 whether the library is linked statically (as it is) or dynamically. Every FCEUX
copyright header is kept, the files added in `zax/` carry the same notice, and
`THIRD-PARTY-NOTICES.md` has the entry.

**Distributing a Zaxnes binary with this library requires the complete corresponding source of the
fork to travel with it**, offered under the same terms alongside the binary. That is why the fork
keeps FCEUX's history intact: every change of ours is `git diff <upstream-base>..zax`, which at the
end of lot FCEUX-1 is **three files and 51 added lines** in `src/` — `types.h`, `fceu.cpp` and
`ppu.cpp`, each change marked `// ZAX:` — plus the self-contained `zax/` directory.
`zax/README.md` lists them.

A release lot therefore owes one more thing nobody has built yet: the fork's source has to be
**published or packaged** with each release that includes the library. A tarball of the fork at the
built commit, with its hash recorded in the release notes, is the least that satisfies it.
