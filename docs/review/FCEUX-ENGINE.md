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

One further fact belonged in any decision to ship it, and lot FCEUX-48 has since closed it: on the
291 public ROMs, **one took the FCEUX process down** — `other/oam3.nes`, through a heap overflow in
FCEUX's own `$2007` write path, reachable from a cartridge image. It is **fixed** in the fork (§7,
rewritten: the cause was not what lot FCEUX-1 thought), so an arbitrary ROM is no longer a
memory-safety question. What is left of the decision is a judgement about *quality* — which games
the 3D on FCEUX is worth offering for — and §6 and the validated-games marker of lot FCEUX-48 are
that.

## 2. The engine abstraction `zax-app` needed

> **Done by lot FCEUX-23 (steps 2 to 4).** `zax-core-api` holds the shared types and the `Engine`
> trait; `zax-nes` and `zax-fceux` both implement it; the application holds one `EngineHandle`
> over a `Box<dyn Engine>`. What follows is this section as it was written, with the three places
> the outcome differs from the plan marked **[FCEUX-23]**. The working is
> `docs/progress/FCEUX-23.md` steps 2 to 4.
>
> One rule of the trait that cost a defect and is worth stating for the lots that follow:
> **`load_rom` reports the detected region, it does not apply it** (REQ-CORE-013), which is
> `NesCore::load_rom`'s own contract. FCEUX keeps its region in a **process global that a
> cartridge load leaves alone**, so an engine that reads the region back off the core after a load
> reports the *previous* game's — and with the TV mode on automatic the application would then
> have played the new game at it. `FceuxEngine` reads the region off the image's header by
> `InesHeader`'s rule and re-asserts its own on the core. Anything else in `zax-fceux` that is
> tempted to ask FCEUX what it is set to should assume the same about process-global state
> (step 8).

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

**[FCEUX-23]** The recommendation was taken: `zax-core-api` exists, has no dependencies of its
own, and `zax-nes` re-exports every moved type at the path it was at — so the whole workspace
compiled on the re-exports alone before a single use site was touched. Three things the plan above
did not foresee, each with its reason:

1. **`sample_rate()` is on the trait too**, not only `set_sample_rate`. FCEUX's core takes 44 100,
   48 000, 96 000 and nothing else, where `zax-nes` resamples anything from 22 050; a rate inside
   that range which FCEUX cannot produce is snapped to the nearest it can, and without a getter
   that snap would be a silent pitch error in the host's audio device.
2. **`export_battery_ram` copies** into a `&mut Vec<u8>` instead of lending `&[u8]`: FCEUX's bytes
   come across the C API and cannot be lent out of a `&self`.
3. **Battery RAM needed three calls in the fork** — `zax_battery_ram_size`,
   `zax_get_battery_ram`, `zax_set_battery_ram` — because FCEUX reaches its save memory only
   through a `.sav` *file* and the core-only build has nowhere to put one. They add no patch to
   any FCEUX file; `zax/README.md` records them.

And one thing the application needed that is not on the trait at all: FCEUX's samples are behind a
`!Sync` handle, while the audio thread holds a clone of **Zaxnes's APU ring**. `EngineHandle` keeps
a shared ring for an engine that has none of its own and fills it once a frame; the Zaxnes path is
the one of before this lot, byte for byte.

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

> **Done by lot FCEUX-23 step 5**, as the five points below ask. In the settings it is
> `emu.engine`, an enumeration of two values with the global default `zaxnes`, effect
> `NextRomLoad` — which is what makes the options window print "(at the next load)" of its own
> accord. The descriptor is in **every** build and only the *row* is gated on the `fceux-core`
> feature, so a `games.json` written by a build with FCEUX loads in one without it; there, the
> value is read and written without error and ignored. The four groups of controls are greyed with
> a reason that names the fix, through `options::inert_reason`, the mechanism that already greyed
> nine other rows. The marker has three places: a notification at every load on an engine other
> than Zaxnes's, the engine's name opening the performance overlay's machine line, and the row
> itself with its tooltip in both languages. Savestate slots record the engine that wrote them in
> the `coreId` field they already had, and a mismatch is refused **before** the engine is asked to
> read the block, with a message that says what to do rather than "corrupted".

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

> **Done by lot FCEUX-48 (FCEUX-4), steps 3 to 5.** The working is `docs/progress/FCEUX-48.md`;
> the user-facing half is `rust/docs/MOVIES.md` §6 and `HOWTO-RUN.md`. What follows is this section
> as it was written, with what the outcome added.

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

### What it became

The reason above held, and it is now a test rather than an argument: two movies with the same
`RAMInitSeed` leave byte-identical work RAM over all 2 KiB and two with different seeds do not,
which **cannot be arranged from outside movie mode at all**.

**Six C API calls, not three.** The three the plan named, plus a `zax_movie_inspect` that reads a
header **without touching the machine**, and the length and the frame as separate queries. The
extra one is the important one: `FCEUI_LoadMovie` re-powers the machine as its third act, so a
refusal decided after it would leave the player's game restarted — and a host that can only say
"unsupported" is no better than one that plays the movie wrong. One patch of `src/`:
`zax_movie_override`, the same consumed-once `FCEUFILE` mechanism `zax_rom_override` already was,
because this build has no filesystem to put a movie file on.

**What is refused**, judged from the header before anything is committed, each with the thing
named: `FDS`, `NewPPU`, `fourscore`, `microphone`, a port device that is neither a standard pad nor
an empty socket, anything on the expansion port, a stated `romChecksum` that is not the loaded
ROM's, and anything that is not a version-3 `.fm2`. A movie that states **no** checksum is played:
claiming nothing is not claiming the wrong thing.

**An empty socket is accepted, and that is not a detail**: a one-player `.fm2` records `port1 0`,
so refusing it would have refused a large share of real TASes. Step 7's own suite run is what
found that, and accepting it needed the fork to *install* the devices the header asks for —
FCEUX asks its driver to, and this build's driver is a stub, so an empty socket would otherwise
have been replayed with the API's own pad on port 2.

**In the application**, opening an `.fm2` plays it on FCEUX and switches the **session's** engine
for that playback, leaving the game's own `emu.engine` untouched — a movie is not a change of mind.
Stopping does not switch back, deliberately: §4's rule is that an engine change re-powers the
machine and never happens mid-game, and a movie's load re-powers anyway while a stop does not. A
`.fcm` keeps Zaxnes's own importer, FCEUX's movie code refusing that format outright.
Everything around the frames is EDITOR-1's machinery reused untouched — the timeline's checkpoints
and seek, the movie bar, pause and frame step, the 3D conversion, the profile's Lua hooks, the
script reload — with three stated exceptions: no movie explorer (it needs a second core), no notes
(an `.fm2` is never written over), and no input hook of ours (FCEUX's movie mode loads each record
over the pad state itself).

**Three things the plan did not foresee**, all three found by measuring:

1. **FCEUX's movie clock has two behaviours a host must not guess at.** `FinishPlayback` runs at
   the *top* of a frame rather than the bottom of the last one (`movie.cpp:1245-1250`), so the mode
   lags the end by one frame and **the clock, not the mode, is the signal**; and
   `currFrameCounter++` is unconditional while a movie is loaded, so the clock counts *past* the
   end and "over" is `frame >= length`.
2. **The region, in the same shape as the defect §2 warns about.** `FCEUI_LoadMovie` applies
   `palFlag` through `FCEUI_SetVidSystem`, which reaches the emulation but **not** `pal_emulation`,
   the driver global `FCEUI_GetRegion` reads — so a PAL movie would have run at PAL while the API
   reported NTSC and the host paced at 60 Hz. The fork re-asserts the region after the load.
3. **Three defects in the application's own code that only an engine like FCEUX exposes**, two of
   them pre-existing. A savestate of an engine other than Zaxnes's does **not** carry the cartridge, so
   `script_reload` — which rebuilds the session and restores a state — left a session with no ROM:
   **F5 during an FCEUX game could not have worked at all.** And a movie the engine plays itself
   cannot survive a state restore either, the block being the machine and not the movie mode, so
   the reload loads the movie again — which is what makes a reload during such a playback *always
   exact*. And the third, this lot's own and found by a test written long before the engine
   existed: **a refused movie reset the game**, the engine switch re-powering the machine before
   the refusal was decided. The movie is now judged on a throw-away engine first, which also tells
   the host whether the ROM loads there at all — a ROM it cannot load is about the *game* and falls
   back to the other engine's reader rather than refusing a playable movie.

### End to end, measured

*Super Mario Bros.* with its own 3dn profile, a **5 400-frame** movie, headless through the real
host (`docs/progress/FCEUX-48.md` step 5):

| | |
|---|---|
| the scripted sequence fed to FCEUX, then the `.fm2` written from it and replayed | **identical at every one of the 5 400 frames** |
| records reaching the 3D path | every frame that has anything to draw; 164 frames have none and **every one is a blank screen** (a death or a level change, the PPU's rendering off), 0 with a picture and no record |
| the draw list at frame 5 400 | 282 tiles, 282 with a mesh |
| pause, two frame steps, a 300-frame seek back and a resume, against an uninterrupted playback | **identical** at frames 1 200, 2 400, 3 600 and 5 000 |
| reloading the profile at movie frame 2 000 | the engine and the movie kept, the frame returned to, **the machine byte-identical** |
| **movie playback with the 3D on FCEUX** | **1 726 frames/s, 0.5795 ms a frame** — the application's own loop, so core, capture, conversion and draw list; about **28× real time** |

## 6. The tile records, and the honest state of the 3D path

**Rewritten by lot FCEUX-6, which did the work this section used to list as remaining.** What
follows is the state after it. The figures are
`docs/review/FCEUX-ENGINE-COMPARISON.md` § *FCEUX-6 tile parity*, and the working is
`docs/progress/FCEUX-6.md`.

### What lot FCEUX-1 left, and what became of it

| FCEUX-1's item | State |
|---|---|
| 1. **Mid-frame scroll** — "the one thing standing between 89 % and near-100 %" | **done** (FCEUX-6 step 2). The fork keeps `zax_row_start` / `zax_row_next` beside `RefreshAddr` — `zax-nes`'s own two fields, maintained by the same rules — and captures the row at the **end** of the line (AMD-021 R2-005). *Super Mario Bros.* frame 200 went from 863 of 969 records to **969 of 969**; the originals identical at every compared frame went from 7 to 41 on this step alone |
| 2. **MMC2/MMC4** latch per column, sprites by x | **done** (step 4), to the requirement (REQ-CORE-039, REQ-CORE-044), and **proven in use by lot FCEUX-23 step 6**: `test-originals-extra` has *Punch-Out!!*, which is mapper 9, and its records are **identical in order** at frames 60, 300 and 1 200 — 1 021 and 988 of them — with identical pictures. Frame 3 000 is emulation-class, the two machines having parted in 2D at frame 4. This item is closed |
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
| **Punch-Out!!** | **ship** | *added by lot FCEUX-23 step 6*, and the ROM that closed item 2 above: records identical in order at 60, 300 and 1 200, pictures identical, and the application's 3D path hands the renderer 189 tiles all with meshes. Mapper 9, so it is the only evidence the MMC2 latch work has |
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

> **Done by lot FCEUX-48 step 1 (FCEUX-8), and lot FCEUX-1's diagnosis below was wrong.** It is
> not a one-byte overflow and it is not fixed by passing `mCHRRAMSize` to `SetupCartCHRMapping`.
> This section is rewritten; the working is `docs/progress/FCEUX-48.md` step 1 and the fork's
> `zax/README.md` § *The CHR-RAM overflow*. The **upstream report is the owner's** to make: the
> fix in the fork is written to be submitted as it stands, and carries no `// ZAX:` marker for
> that reason.

`other/oam3.nes` (mapper 7) corrupted the heap from about 300 frames in. ASAN on the fork's Linux
build, before the fix:

    heap-buffer-overflow, WRITE of size 1
      #0 B2007      src/ppu.cpp:1008
      #1 WrMem      src/x6502.cpp:173
    0 bytes after a 1024-byte region allocated by iNES_Init, src/ines.cpp:1220

### The real cause

ASAN reports the *first* byte outside the buffer, which is what made this look like a one-byte
overflow. It is not: **the whole of `$0400-$1FFF` is outside it.**

`setchr8()` (`src/cart.cpp:262-274`) points all eight 1 KiB `VPage` slots at one **linear** 8 KiB
window from the chip base — `VPageR[x] = &CHRptr[r][V << 13]` for every `x`, with no per-slot
offset — and `SetupCartCHRMapping` (`src/cart.cpp:126-139`) computes `CHRmask8 = (size >> 13) - 1`
and then **clamps it to 0** when that underflows. So for a chip smaller than 8 KiB the window is
still 8 KiB wide while the buffer is not. `setchr4` and `setchr2` clamp the same way below 4 KiB
and 2 KiB. At or above the window the mask is sound even for a size that is not a power of two,
since `V <= (size >> n) - 1` gives `(V + 1) << n <= size`.

A chip that small comes from exactly one place: the iNES loader's NES 2.0 branch
(`src/ines.cpp:1215`), where byte 11 encodes each half of the CHR-RAM as `64 << shift` and may
declare as little as 64 bytes. `oam3.nes` declares 1 KiB and is mapper 7 (ANROM), which calls
`setchr8`. The loader padded the allocation to **1 KiB** — `VPage`'s *granularity*, not the width
of the window `setchr8` installs — which is why its own comment about preventing "malicious access
from NES software" did not.

This is **upstream FCEUX's**, not the fork's: every frame of the trace is FCEUX's own code, no stub
or patch of ours appears in it, and the path is reached by the plain API.

### The fix taken

Pad the allocation to **8 KiB** in `iNES_Init` and leave everything else alone: the **declared**
size still goes to `SetupCartCHRMapping` and to `AddExState`, and `FCEU_MemoryRand` still fills
exactly the declared size. All three matter —

* `FCEU_MemoryRand` draws one `xoroshiro128plus` value per byte under `RAMInitOption` 3, so the
  random RAM-init option consumes the same values as before. **A movie must not be affected**, and
  §5's whole point is that a movie's header carries that option;
* `AddExState(VROM, CHRRAMSize, …)` keeps the `CHRR` block at its old length, so savestates do not
  move either;
* the padding is zeroed and never read back as cartridge data.

**The apparent one-line fix does not work.** Passing `mCHRRAMSize` to `SetupCartCHRMapping` leaves
`CHRmask8` at 0 for an 8 KiB chip — so the window is unchanged and the overflow stands — and it
*would* change how a sub-8 KiB chip mirrors under `setchr1`/`setchr2`, which is the emulation
decision lot FCEUX-1 rightly did not want to take.

What the fix is **not**: it makes the access safe, not accurate. Above the declared size the PPU now
sees the zero padding where the hardware — and `zax-nes`, which wraps every CHR access modulo the
buffer (`cartridges/mod.rs`'s `wrap_offset`) — would mirror the chip. Making `setchr8` mirror per
slot is a change to a function every board uses; it is not needed to close the memory error and it
was not taken.

### What it was checked against

| | |
|---|---|
| `oam3.nes` under ASAN | **3 000 frames clean**; it faults at about frame 300 without the fix |
| `oam3.nes` on MSVC and on Linux | the **same digests**, which it could not give before — the overflow read whatever the allocator had put there |
| `nestest.nes`, 60 frames | unchanged digest for digest, state hint and length included |
| every other CHR-RAM registration in FCEUX | checked by reading; **none has the defect**. The UNIF path's floor is 8 KiB, mapper 77's 6 KiB chip is addressed only by `setchr2r`/`setchr4r` whose windows fit, and every other sub-8 KiB chip is addressed only by `setchr1r`. The full table is in the fork's `zax/README.md` |
| the ROM sets | of **446** iNES images across the three sets, **exactly one** declares a NES 2.0 CHR-RAM under 8 KiB, and it is `oam3.nes`. Nothing else can have moved |
| the two engines on `oam3.nes` | `fceux-tiles` at frames 60, 300, 1 200 and 3 000: **identical in order at every one**, 1 024 records, pictures identical. Their pictures first part at frame 3, so the padding-against-mirroring difference above is latent on this ROM rather than absent |

### The vetting, which is the other half

Three options were on the table, in increasing cost:

1. take the fix in the fork and report it upstream;
2. run FCEUX in a child process in the shipped application too, as the harness does;
3. ship FCEUX only for games on a vetted list.

This note recommended **1 plus 3**, and lot FCEUX-48 did both: step 1 is the fix, and step 2 is the
vetted list as a **marker** rather than a gate — a table of the games validated for 3D on FCEUX,
identified by ROM hash, with the engine row and the HUD saying "not validated on FCEUX" for
anything else. The option stays available for every game, which is the owner's call and is recorded
as such: with the overflow closed, an arbitrary ROM is no longer a memory-safety question, and
offering the engine with an honest marker is better than a list that silently refuses a game.

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
| **FCEUX-2** | ~~The `Engine` trait and the type move to `zax-core-api`~~ — **done, lot FCEUX-23 steps 2 to 4.** | ~~large~~ |
| **FCEUX-3** | ~~The per-game choice~~ — **done, lot FCEUX-23 step 5.** | ~~medium~~ |
| **FCEUX-4** | ~~`.fm2` playback over FCEUX's own movie mode, and the UI to open a movie~~ — **done, lot FCEUX-48 steps 3 to 5.** | ~~small~~ |
| **FCEUX-5** | Rollback and run-ahead over FCEUX states, with the measurements §3 asks for before a history depth is chosen. | medium |
| **FCEUX-6** | The tile capture finished: mid-frame scroll first (item 1 of §6), then MMC2/MMC4, then MMC5, then the new PPU. 3D on FCEUX ships at the end of this, not before. | large, and best split in two |
| **FCEUX-7** | Android and PortMaster cross-builds of the fork, and the size question of §8. | small to medium, mostly toolchain |
| **FCEUX-8** | ~~The `oam3.nes` fix, reported upstream, and whatever vetting §7 settles on~~ — **done, lot FCEUX-48 steps 1 and 2.** | ~~small~~ |

The order matters: FCEUX-2 gates everything, FCEUX-4 is the cheapest thing with visible value, and
FCEUX-6 is the only one that should hold up shipping — because 2D FCEUX is already useful and 3D
FCEUX is not yet right.

**What lot FCEUX-23 leaves for the rest**, from having done FCEUX-2 and FCEUX-3:

* **FCEUX-4** (`.fm2`) is unblocked and still small. The engine's RAM fill is already settable
  (`FceuxEngine::set_ram_init`) for exactly the reason §5 gives, and the C API still needs its
  three movie calls.
* **FCEUX-5** (run-ahead over FCEUX states) has its place reserved:
  `Capabilities::run_ahead` is `false` on FCEUX, `EngineHandle::run_frame_ahead` runs the plain
  frame there, and the three run-ahead rows are greyed with that reason — so the lot has to raise
  one flag and fill one method rather than find a seam. **Rollback needed nothing at all**: the
  application's history has always used `save_state_to` with one reused buffer a slot, which is
  what §3 asks for, so it already works on FCEUX.
* **FCEUX-7** (Android, PortMaster) is unchanged, except that the three battery calls are now part
  of the fork's surface and so part of what has to cross-compile — which is nothing, being plain
  C++ over globals FCEUX already exports.
* **FCEUX-8** (`oam3.nes`, the vetting of §7) is unchanged and is still the thing that decides
  whether an **arbitrary** ROM may be handed to FCEUX. Lot FCEUX-23 did not vet anything: the
  per-game option lets a user choose FCEUX for any game in the library, and §7's one-byte heap
  overflow is still live in the fork.

**What lot FCEUX-48 leaves for the two that remain**, from having done FCEUX-4 and FCEUX-8:

* **FCEUX-5** (rollback and run-ahead over FCEUX states) is where it was and now has one more
  thing in its favour and one more thing to be careful of. In its favour: step 5 measured movie
  playback with the whole 3D path at **1 726 frames/s on FCEUX**, so the state churn §3 worried
  about is cheap at this speed — a rollback second is ~4.8 MB and the engine is doing 28× real
  time. To be careful of: **`EngineHandle::cycle_count` on an engine without a counter of its own
  is the sum of the frames this *session* has run**, not the cycles since power-on, so it does not
  come back with a restored state (step 4 records this in its doc comment). Anything that compares
  two paths to the same frame — which run-ahead does by construction — must not use it, and
  `movie_timeline::machine_digest` hashes it.
* **FCEUX-7** (Android, PortMaster) gains two small things to cross-compile and loses nothing: the
  six movie calls of step 3, which are plain C++ over FCEUX's own `movie.cpp`, and the `ines.cpp`
  fix of step 1, which is three lines of allocation arithmetic. Both are architecture-blind.
* And one thing neither lot owns, which belongs to a release lot: **the fork's patch set over
  `src/` grew from three files to five** (§10), so the "complete corresponding source" duty now
  covers `movie.cpp` and `ines.cpp` as well. The `ines.cpp` one is a bug fix the owner is to submit
  upstream, and carries no `// ZAX:` marker for that reason.

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

**After lot FCEUX-23** the figure is unchanged where it matters: that lot added
`zax_battery_ram_size`, `zax_get_battery_ram` and `zax_set_battery_ram`, and all three live in
`zax/zaxfceux.cpp` — **no FCEUX file was patched**, so `git diff <upstream-base>..zax` over `src/`
is still the same three files, and the fork's own `zax/` directory grew by about 120 lines.

**After lot FCEUX-48 the figure has moved**, and this is the one place in this note it is
recorded: `src/` is now **six files** — the three above, `src/boards/mmc2and4.cpp` which lot
FCEUX-6 added and the figure above never counted, and two from this lot. The fork's own
`zax/README.md` has the full table and is what makes a tarball reviewable.

* `src/movie.cpp` — `zax_movie_override`, the consumed-once `FCEUFILE` of the movie load, the same
  mechanism `src/fceu.cpp` already had for a cartridge image. Marked `// ZAX:`, an adaptation;
* `src/ines.cpp` — the CHR-RAM allocation padded to 8 KiB (§7). **A bug fix in FCEUX itself**, and
  deliberately **not** marked `// ZAX:` so that it can be submitted upstream as it stands. The
  upstream report is the owner's to make.

A release lot therefore owes the tarball over a slightly larger diff, and owes the upstream report
as a separate, non-blocking act.

A release lot therefore owes one more thing nobody has built yet: the fork's source has to be
**published or packaged** with each release that includes the library. A tarball of the fork at the
built commit, with its hash recorded in the release notes, is the least that satisfies it.
