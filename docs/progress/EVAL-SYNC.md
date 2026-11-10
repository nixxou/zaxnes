# EVAL-SYNC — where Zaxnes's engine parts from a TAS movie, and whether `.bk2` can be played

Branch `eval/bk2`, worktree `cleanroom/wt-bk2`, base `432492c4` (lot FCEUX-48, which gave the
application a second engine and `.fm2` playback on it).

This is an **evaluation**. It measures and explains; it changes no emulation in `zax-nes`.

Read first: `docs/progress/FCEUX-48.md` (how `.fm2` plays on both engines),
`docs/review/FCEUX-ENGINE.md` §5.

## Steps

- [x] 1. The divergence instrument (`zax-harness movie-sync`).
- [x] 2. Run it on the four movies; the table.
- [x] 3. Explain each first divergence, and classify it.
- [x] 4. `.bk2` reading.
- [x] 5. The report, `docs/review/MOVIE-SYNC.md`.
- [ ] 6. Final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` |
| `CARGO_INCREMENTAL` | `0` |
| the fork, MSVC | `cleanroom/fceux-zax/build-msvc`, the existing build — **not rebuilt** |
| the movies | `G:/3DSen/test-movies/*.fm2` (four), their ROMs in `roms/` |
| `ZAXNES_ROMS` | `G:/3dsen/demo/roms` |
| `ZAXNES_TEST_ROMS` | `G:/3dsen/cleanroom/assets-free/test-roms` |
| `ZAXNES_PROFILES` | `G:/3dsen/cleanroom/assets-neutral` |

## The inputs, read before anything was written

### Which dump each movie was recorded on

An `.fm2`'s `romChecksum` is the **MD5 of the image after its 16-byte iNES header**
(`zax_nes::movie::fm2`'s own rule). Measured over the four candidate dumps:

| movie | `romChecksum` | the dump that matches |
|---|---|---|
| `Kabuki - Quantum Fighter_mod_mod2.fm2` | `fenJYDKAFYdKTOSh5hNtRg==` | `Kabuki - Quantum Fighter (USA).nes` |
| `Kabuki-NTSC-no-FDS.fm2` | the same | the same |
| `MetalForce-DTC5CompV8.fm2` | `o6cWwGS6c6ChZeWRi8pBUw==` | `Metal Force [Korea][Unl][!].nes` |
| `taseditor-kirbysadventure.fm2` | `Nv+hAqy6EbXFdCfYrFrZjg==` | **`Kirby's Adventure [USA][!!].nes`** |

*Kirby*'s other dump, `[Rev 1][!!]`, is `dYdE7C0vo9HyAgWGQB+X6A==` and is **not** the one the
movie was made on. The `[!!]` dump is.

### What the four headers say

| | Kabuki ×2 | Metal Force | Kirby |
|---|---|---|---|
| frames | 33 306 | 47 454 | 123 937 |
| `palFlag` | 0 | 0 | 0 |
| `port0` / `port1` / `port2` | 1 / 0 / 0 | 1 / 0 / 0 | 1 / 0 / 0 |
| `fourscore` / `microphone` | 0 / 0 | 0 / 0 | 0 / 0 |
| `NewPPU` | 0 | 0 | 0 |
| `FDS` | **1** (`_mod_mod2`), absent (`no-FDS`) | 0 | 0 |
| `RAMInitOption` / `RAMInitSeed` | **absent** | **absent** | **absent** |
| a command at a frame | `\|2\|` at frame 0 (power-on) | none | **`\|1\|` at frame 5 — a real soft reset** |
| mapper | 4 (MMC3) | 156 (Daou DIS23C01) | 4 (MMC3), battery |

Three things that follow, each found by reading rather than by running, and each one a thing the
instrument has to get right before any measurement of it means anything:

1. **`FDS 1` on a cartridge game.** The brief said one header claims it; it is
   `Kabuki - Quantum Fighter_mod_mod2.fm2`, and the ROM it names is a plain iNES cartridge
   (mapper 4). The *image*, not the header, decides whether the machine has a disk drive, so the
   instrument notes the contradiction and plays on. FCEUX's own movie mode refuses the file by
   name (lot FCEUX-48's refusal table), which is a fact about the application rather than about
   this movie, and is recorded as such.
2. **A soft reset mid-movie.** *Kirby*'s frame 5 carries `|1|`. The application's **Zaxnes** `.fm2`
   path does not replay a movie's resets at all — `movies.rs`'s `play_fm2` warns
   "the movie's resets are not replayed, the first at frame N" and goes on. FCEUX's movie mode
   replays it. That alone would part the two machines at frame 5.
3. **No `RAMInitOption` means FCEUX's own pattern, not zeroes.** `MovieData`'s constructor
   defaults `RAMInitOption` to **0** and `FCEUI_LoadMovie` copies it into the global
   (`movie.cpp:414,1100`), and option 0 in `FCEU_MemoryRand` (`fceu.cpp:990-1021`) is the
   `00 00 00 00 FF FF FF FF` pattern — *not* all zeroes. `zax-nes` powers on with work RAM
   **all zero** (`NesBus::reset`, `nes_bus.rs:398`, REQ-CORE-004). So the two machines start every
   one of these four movies from a different 2 KiB, which no amount of accurate emulation can
   recover from if the game reads it.

These are the three things step 3 would otherwise have "discovered" as divergences; they are
written down here first so that what step 3 reports is what is left after them.

---

## Step 1 — the divergence instrument

`crates/zax-harness/src/movie_sync.rs` (the command) and `movie_file.rs` (the movie, reduced to
what drives a machine), with `tests/movie_sync_tests.rs` beside their own unit tests. Both modules
are **outside** the `fceux-core` feature: `--engine zaxnes` is a measurement the default build can
make, and only the second machine is `cfg`'d, so the whole module is compiled and clippy'd in both
builds.

### What it compares, and the one thing that had to be invented

Per frame, in lockstep: the **2 KiB of work RAM** (which parts first — a game's own variables move
before the picture does), the **picture** (256×240 colour indices), and whether the frame was
**input-sensitive**. And **frame 0** before all of them: the machine at power-on, work RAM only,
because a movie whose two machines start from a different 2 KiB has already lost.

The third one had to be invented, and the reasoning is in `probe_input_sensitivity`'s own doc
comment. **Neither engine publishes a lag flag**: FCEUX keeps one but no C API call reaches it (and
the fork is read-only in this lot), and `zax-nes` keeps none at all — and this lot may not add one.
So the frame is **measured** instead: run it twice from the same state, once with the movie's word
and once with its complement; the frame is *input-sensitive* when the two outcomes differ. A frame
the game spent without reading `$4016` cannot react to either word.

Two things it is not, stated in the code rather than left for a reader to find: it is not FCEUX's
`lagFlag` (a game that polls and discards measures insensitive), and it does not look past the
frame. Neither matters for a *comparison*, which is what it is used for — **both machines are
measured by the same rule**, which is the property that would be lost by using FCEUX's own flag on
one side and something else on the other.

The probe runs **before** the real frame, and the real frame runs last. That order is not
cosmetic: `NesCore::restore_core_state` deliberately leaves the picture out of the block, so a
probe run *after* the real one would overwrite the picture it was meant to be checking.
`the_input_sensitivity_probe_leaves_the_run_exactly_where_it_was` is that contract on one engine
and `the_probe_leaves_the_lockstep_run_where_it_was_on_both_machines` on both — FCEUX's state block
being a different mechanism from `CoreState`, one is no evidence about the other.

### The four things the work found before any movie was run

**1. FCEUX cannot be asked for a seeded random fill from outside movie mode**, so the instrument
writes the 2 KiB itself on **both** machines. `PowerNES` reseeds `RAMInitSeed` from `rand()` on
every power-on unless a movie is active (`fceu.cpp:1031-1039`) — which §5 of the design note
records as the reason `.fm2` playback needs FCEUX's movie mode at all, and which also means a
plain `set_ram_init(Random(seed))` + `power()` gives a different 2 KiB every time. So
`FceuxMachine::power` powers the machine and then writes `RamPattern::fill`'s bytes through
`write_memory`. That also makes "the same power-on RAM" true by construction rather than by two
implementations agreeing.

`set_ram_init` is still called, because `RAMInitOption` fills more than the work RAM: a
cartridge's **CHR-RAM** gets it too (`ines.cpp:79`) and no API call reaches that. None of this
lot's four ROMs has CHR-RAM, so it is a limitation recorded rather than one that bites.

`the_random_pattern_is_the_one_fceux_writes` pins the generator against **FCEUX itself** — from
inside movie mode, which is the only place it can be pinned — for four seeds. `RamPattern::Random`
is therefore `FCEU_MemoryRand` option 3 byte for byte, not merely a faithful-looking reading of it.

**2. `nestest.nes` is not a ROM the two engines agree on.** It was the obvious ROM for the
"finds nothing when there is nothing" test and it is wrong: they part at frame **4**, on the
**power-on palette RAM** ($3F00-$3F1F: Zaxnes has a palette, FCEUX has zeros), whatever the
work-RAM pattern is. `fceux-tiles` says the same thing independently, so this is not the new code.

Finding a ROM that does agree took two sweeps, and both figures are worth keeping because they say
how rare agreement is:

| | |
|---|---|
| public ROMs run through `fceux-tiles` at frames 60 and 120 | **290** |
| of those, with **no pixel difference** at any compared frame | **25** |
| of those 25, run through `movie-sync` over 121 frames, with **no work-RAM difference** either | **5** |

The five are `apu_reset/len_ctrs_enabled`, `apu_reset/works_immediately`,
`blargg_ppu_tests_2005.09.15b/vbl_clear_time`, `dmc_tests/buffer_retained` and `dmc_tests/latency`.
Two of the five part once a movie carries a **reset**, so the test ROM is
`dmc_tests/buffer_retained.nes`, which agrees with inputs, with two resets and with the probe on.

**3. A difference can be in dead memory, and the instrument says so rather than hiding it.**
`branch_timing_tests/1.Branch_Basics.nes` has no pixel difference at all and still parts on three
bytes at `$01FA-$01FC` — **below the stack pointer**, leftovers of earlier pushes that no game
reads. It is reported, because an instrument that decides for itself which bytes matter cannot be
checked. Step 3 has to do that judging, with the addresses in front of it.

**4. `.fm2`'s `FDS 1` on a cartridge, handled where it belongs.** `Fm2Movie::parse` refuses the
file outright and **must not change** (it is the application's reader). So `MovieFile::read_fm2`
removes the line before handing the bytes over, with a note, *only* when the image is an iNES
cartridge — and refuses by name when it is not.

### The command

```
zax-harness movie-sync <movie.fm2> <rom.nes> [options]
  --engine zaxnes|fceux|both   (default both)
  --frames <n> --from <n>      a cap, and the first frame compared
  --ram default|ff|zero|random:<seed>    override the header's power-on work RAM
  --lag / --no-lag             the input-sensitivity probe (default on; 3x the frames)
  --lag-from <n> --lag-to <n>  probe only inside a window
  --keep-going <n>             frames to compare after the first divergence
  --ram-bytes <n>              differing addresses named per line (default 16)
  --progress <n> --report <file.json>
  --movie-mode                 `--engine fceux`: FCEUX's own movie mode, as the app plays it
  --png <dir> --png-frames a,b,c
```

`--engine fceux --movie-mode` is the authority on "does this movie play on FCEUX"; the lockstep run
feeds FCEUX the inputs **directly** instead, because movie mode overwrites the pads inside the
frame and the probe could then never see anything. That the two FCEUX paths give the same machine
is a measurement, not an assumption: `the_two_fceux_paths_give_the_same_frames`, at three lengths.

### Checked

| | |
|---|---|
| `cargo fmt --check` | clean |
| clippy `-D warnings --all-targets`, feature **on** and **off** | clean |
| `movie_file` and `movie_sync` unit tests | **14**, green in both builds |
| `movie_sync_tests`, feature **on**, `--include-ignored --test-threads 1` | **11 passed**, 0 failed |
| the same, feature **off** | **4 passed** (the seven lockstep ones are `cfg`'d out) |
| `zax-harness` lib tests, feature off | 21 passed |

---

## Step 2 — the four movies

Every figure below is one `movie-sync --engine both --lag` run over the **whole** movie, with the
input-sensitivity probe on every frame. The exact commands are at the end of this note.

### The table

| | Kabuki ×2 | Metal Force | Kirby |
|---|---|---|---|
| frames | 33 306 | 47 454 | 123 937 |
| mapper | 4 (MMC3) | 156 (Daou) | 4 (MMC3) |
| **first divergence, anything** | frame **2** | frame **5** | frame **3** |
| what it is | 1 byte, `$01FB` `56`/`53` — **in the stack page** | **1 pixel** at (231,79) | **82 RAM bytes**, 68 of them outside the stack |
| first divergence **outside the stack page** | frame **258** | the same frame | the same frame |
| first divergence that would be **seen** | frame **787** (5 pixels at (209,191)) | the same frame | frame **4** (a lag frame on FCEUX, not on Zaxnes) |
| compared frames that differ visibly | **32 395** of 33 304 | **5** of 47 449 | **123 646** of 123 934 |
| which ones, when they are few | — | **5, 20 313, 26 278, 26 298, 29 735** | — |
| last frame with the same picture | 1 167 | **47 454, the last** | 100 488 |
| final picture, Zaxnes | `1ded20b9…` | `3de90a64…` | `6e842956…` |
| final picture, FCEUX | `a3cbc349…` | **`3de90a64…` — the same** | `939584ae…` |

The two *Kabuki* files are **the same movie**: their 33 306 frame lines are byte-identical and the
only difference between the files is the one line `FDS 1`. Their two runs are identical figure for
figure, which is what `MovieFile::read_fm2` dropping that line is supposed to produce.

### How far FCEUX plays each one — confirmed, not assumed

`--engine fceux --movie-mode` hands the file to FCEUX's own `FCEUI_LoadMovie`, which is what the
application does:

| movie | FCEUX's own movie clock at the end | final picture |
|---|---|---|
| Kabuki (`-no-FDS`) | **33 306 / 33 306** | `a3cbc349…` |
| Kabuki (`_mod_mod2`) | **refused**: "this movie needs a Famicom Disk System disk, which the FCEUX engine cannot play here" | — |
| Metal Force | **47 454 / 47 454** | `3de90a64…` |
| Kirby | **123 937 / 123 937** | `939584ae…` |

Each of those three digests is **the same digest the lockstep run's FCEUX side ends on**. That is
the check the instrument needed and did not have before: feeding FCEUX the inputs directly — which
the lockstep run must do, movie mode overwriting the pads inside the frame — gives the same machine
as FCEUX's own movie mode, on real movies of 33 000 to 124 000 frames rather than on a 150-frame
synthetic one.

So **FCEUX plays all three to the end**, and the one it refuses it refuses because of the header
contradiction the owner already knew about, not because of anything in the movie.

### How far Zaxnes plays each one — looked at, not only hashed

A digest says two runs differ; it does not say whether the run is still a game. The pictures were
written out and read.

| | frame | Zaxnes | FCEUX |
|---|---|---|---|
| **Kabuki** | 1 000 | stage 1, score 50, TIME 246 | **the same picture** |
| | 10 000 | still in a level, score 180 | an **inter-stage cutscene** (four characters round a table) |
| | 33 306 (last) | still in stage 1, **score 100**, TIME 116 | the **ending**: the red-haired character, the credits pane |
| **Metal Force** | 47 454 (last) | the player mid-level with a boss sprite | **the same picture, byte for byte** |
| **Kirby** | 123 937 (last) | a grass level, 1 life, score 17 800 | the **star corridor** of the endgame, 8 lives, an ENEMY bar |

Kabuki's Zaxnes run also has a **380-frame stretch from frame 29 187 in which the picture does not
change at all**, which FCEUX's run does not have — the shape of a run that has stopped being
played rather than one that is merely different.

So, in the terms the brief asks for:

| movie | how far FCEUX plays it | how far Zaxnes plays it |
|---|---|---|
| Kabuki | **to the end, correctly** (the ending is on screen) | **to frame ~787 of 33 306**, after which the two runs are different games. The inputs go on being fed for another 32 500 frames and the run ends in stage 1 |
| Metal Force | **to the end, correctly** | **to the end** — 5 frames of 47 454 differ visibly and the last picture is byte-identical |
| Kirby | **to the end, correctly** | **to frame 3** |

### Three things this step establishes before step 3 starts

1. **Metal Force is not a failure to explain, it is a near-sync to explain.** Five differing frames
   over 47 454, no drift, the same last picture. Whatever happens at frame 5 does not compound.
2. **Kabuki's first divergence is in the stack, and that is not noise.** The byte at `$01FB` is one
   of the three an interrupt pushes — a program counter and the flags. A different byte there means
   the two machines **took the interrupt at a different point**, which is the most useful single
   fact this step produced.
3. **Kirby parts at frame 3 with 82 bytes at once**, and on FCEUX's side most of the differing
   bytes are still the power-on `00`/`FF` pattern while Zaxnes's hold data — so Zaxnes has run
   *further* through the game's initialisation by frame 3, rather than having run it differently.

---

## Step 3 — why each movie parts, and what class it is

The short answer, before the working: **all three are class (b)** — an FCEUX inaccuracy the movie
was recorded on. **No class (a) was found**: not one of the three divergences lands on a mechanism
where Zaxnes is the one failing a public test. The one class (c) item that still bites is in the
**application**, not in `zax-nes`, and it is written up at the end with its size.

### The setup differences were settled before the first measurement (class c)

Step 1 found four of them by reading, and the instrument honours all four on both machines: the
region, the power-on work RAM, the per-frame reset commands, and the `FDS 1` claim on a cartridge.
What each is worth was then **measured** rather than assumed — the same movie on Zaxnes alone under
the three deterministic fills, 2 000 frames (47 454 for *Metal Force*):

| | `--ram zero` | `--ram default` | `--ram ff` |
|---|---|---|---|
| Metal Force | `3de90a647172d715` | the same | the same |
| Kabuki | `eb96fb9d5d6fc02d` | the same | the same |
| **Kirby** | `34f0ab4acf20a98d` | **`107d27b697e2aa5c`** | **`04648ed5dc58ca23`** |

So the power-on pattern changes nothing for two of the three games — they clear their own RAM — and
for *Kirby* it changes the run outright. It is still **not** the cause of *Kirby*'s first
divergence: with both machines on `zero`, on `ff` and on `default` the two engines part at
**frame 3 in all three cases**.

### Metal Force — 5 frames of 47 454, and none of them is in the CPU

**What it is.** Frame 5 is one pixel: (231,79), Zaxnes black, FCEUX grey. The frame is a flat
screen — frame 4 is **61 440 pixels of one colour** and frame 6 is 61 440 of another — so rendering
is off and what is on screen is the backdrop. Scanline 79 is where the backdrop changes, and the
two engines put the change **one dot apart**: Zaxnes from x=231, FCEUX from x=232. Work RAM is
identical, before and after.

The other four (20 313, 26 278, 26 298, 29 735) are all on **row 0 only**, 6 to 15 pixels in the
middle of the row, again with the work RAM identical. They are the top scanline of a frame whose
sub-scanline phase differs.

**Why.** A sub-scanline difference in PPU phase. The two engines' measured disagreement on exactly
this is on the record: of the ten `ppu_vbl_nmi` singles, Zaxnes passes nine and FCEUX fails all
ten, and the two that are about the dot count of a frame are `09-even_odd_frames` (Zaxnes
`00 01 01 02`, passed; FCEUX `04`, **Failed #2**) and `10-even_odd_timing` (Zaxnes `08 08 09 07`,
passed; FCEUX `02`, **Failed #2**) — the skipped dot of the odd pre-render line, which is a one-dot
shift of the whole frame.

**Class (b)**, and the mildest possible one: the CPU never diverges, the movie syncs, the last
picture of the 47 454 is byte-identical on the two engines. Nothing to fix on either side.

### Kabuki — an interrupt taken a few cycles apart, and 33 000 frames of consequences

**What it is, frame by frame**, with the work RAM read at every one:

| frame | what differs |
|---|---|
| 1 | nothing |
| **2** | **one byte, `$01FB`: `56` on Zaxnes, `53` on FCEUX** — in the stack page |
| 3-8 | nothing at all |
| 9 | `$01E5 00/3A, $01E6 00/F9, $01E7 3A/00, $01E8 F9/00` — **the same two bytes (`3A F9`), two addresses apart** |
| 258 | the first bytes outside the stack: `$0018`, and `$0072`-`$0079` — eight consecutive bytes |
| **787** | the first pixels: five, at (209,191) |
| 1 167 | the last frame on which the two pictures are ever the same again |

Three readings, in order of how much they say:

1. **Frame 2's byte is in the stack, and that is the most informative place it could be.** The
   bytes an interrupt pushes are the return address and the flags. One byte differing by **3** is a
   return address whose low half is three bytes further along the instruction stream: the two
   machines **took the interrupt at a different point in the same loop**, a couple of CPU cycles
   apart.
2. **Frame 9 says how much apart.** `3A F9` is written at `$01E7`/`$01E8` on Zaxnes and at
   `$01E5`/`$01E6` on FCEUX — the same value, **two bytes lower on FCEUX**. Two bytes is one return
   address: at that moment the two machines' stacks are at a different depth, which is what an
   interrupt taken on one and not on the other leaves behind.
3. **Frames 3 to 8 are identical**, which rules out a steady drift. It is one event, not a clock
   running at a different rate.

**Why.** FCEUX takes the NMI at the wrong cycle, and the public test says so by how much:
`ppu_vbl_nmi/rom_singles/05-nmi_timing` — **Zaxnes `00 4`, passed; FCEUX `00 3`, Failed #1**. The
test prints the cycle at which the NMI was taken; the two answers are **one CPU cycle apart**, and
Zaxnes's is the right one. The rest of that family is the same story — `01-vbl_basics`,
`02-vbl_set_time`, `03-vbl_clear_time`, `06-suppression`, `07-nmi_on_timing`, `08-nmi_off_timing`:
Zaxnes passes every one, FCEUX fails every one.

*Kabuki* is MMC3, so there is a second interrupt in play, and FCEUX is wrong about that one too:
it fails **all five** of `mmc3_test` 1 to 5 and all five of `mmc3_test_2`, each with a named
message (`Should decrement when A12 is toggled via PPUADDR`, `Counter isn't working when reloaded
with 255`, `Should be clocked when A12 changes to 1 via PPUADDR write`, **`Scanline 0 IRQ should
occur later when $2000=$08`**, `Should reload and set IRQ every clock when reload is 0`), and
Zaxnes passes every one. The instrument was run on `mmc3_test/4-scanline_timing.nes` itself with a
blank movie: the two engines part at frame 4, and at frame 2 000 the screen says, in so many
words, **Zaxnes `4-scanline_timing / Passed`** and **FCEUX `Scanline 0 IRQ should occur later when
$2000=$08 / 4-scanline_timing / Failed #2`**.

Which of the two interrupts fires at frame 2 is not settled here, and saying which would need
cycle-level tracing this lot has no hook for. It does not change the class: both candidates are
mechanisms Zaxnes passes the public test on and FCEUX fails it on.

**Class (b).** Nothing in `zax-nes` to fix. For this TAS to sync on Zaxnes, Zaxnes would have to
take the NMI one cycle early and clock the MMC3 counter wrongly in five specific ways — that is,
fail ten public tests it currently passes.

### Kirby — the vblank the game waits for at boot

**What it is.** Frames 1 and 2 are identical. **Frame 3 differs in 82 bytes**, 68 of them outside
the stack. Reading the two sides tells which machine is where: at almost every differing address
**FCEUX still holds the power-on `00`/`FF` pattern and Zaxnes holds data** — `$0627`-`$0636` is
`0E` sixteen times on Zaxnes and `FF FF 00 00 00 00 FF FF …` on FCEUX, which is the pattern
untouched. And at frame 4 it is the same one step on: Zaxnes has filled `$0200`-`$02FD` with `F0`
(the sprite table pushed off-screen) and written `4B 49 52 42 59` — `KIRBY` — at `$0517`, and
FCEUX has done neither yet.

So the two machines are not doing different things; **Zaxnes is one step further through the same
initialisation**.

Two things that are *not* the cause, both measured rather than argued:

* **not the power-on RAM.** With both machines on `zero`, on `ff` and on `default`, the first
  divergence is frame 3 in all three;
* **not a whole-frame offset of the boot.** `--warm-up` was added to the instrument for exactly
  this question — blank frames on one engine before the movie starts. `fceux:1`, `fceux:2`,
  `fceux:3` and `zaxnes:1` were all tried: none of them makes the two agree, and none of them
  moves the first divergence past frame 3. Whatever is apart is apart **inside** a frame.

**Why.** *Kirby* waits for vblank at boot with the background off, and that is the one thing the
public test names: `ppu_vbl_nmi/rom_singles/01-vbl_basics` — Zaxnes `01-vbl_basics / Passed`,
FCEUX **`VBL period is too short with BG off / 01-vbl_basics / Failed #7`**. Both screens were
captured and read. A vblank period that is too short with the background off is precisely a boot
loop that finishes at the wrong time, which is precisely "one step further through the same
initialisation".

**Class (b).**

### The class (c) item that is left, and it is in the application

The instrument honours the movie's header; **`zax-app`'s own `.fm2` path on the Zaxnes engine does
not**, and that is a defect in the shipping application rather than in this evaluation:

| what the header says | the harness | `zax-app`'s `play_fm2` (`emulation/host/movies.rs`) |
|---|---|---|
| `RAMInitOption` / `RAMInitSeed` | applied to the machine | **ignored** — the core powers on all-zero |
| a reset command on a frame | replayed (`soft_reset` / power) | **not replayed**; it warns "les resets du film ne sont pas rejoués, le premier à la frame N" (`movies.rs:174-183`) |
| `palFlag` | applied | **compared and warned about**, not applied (`movies.rs:158-164`) |

What it is worth, measured: of the three games the RAM pattern changes the run for **one** —
*Kirby*, three fills and three different runs — and the reset matters for the same one, its movie
carrying a soft reset at frame 5. Fixing it would **not** make either movie sync, because the
reason they do not sync is class (b); it would remove a second, avoidable reason.

The size of the fix, for whoever takes it: `Movie`'s start state is built by `session.reset(...)`
then `core.save_state(true)` (`movies.rs:184-187`), so the pattern would be written over
`bus_mut().ram_mut()` between those two lines — three lines — and the region set from the header
on the line above. The resets are the real work: `MovieRun` has no notion of a command on a frame,
so replaying them means carrying `Fm2Movie::resets()` into the movie and acting on it in the frame
loop. It is a change to the application's movie player and **not** to `zax-nes`, and it is left for
the owner to schedule rather than made here, because this lot is an evaluation and the fix changes
what a shipped feature does.

### What was *not* found, said plainly

**No class (a).** On the 113 public ROMs that report a verdict on both engines, Zaxnes passes 95
and FCEUX 58, and the count passed by FCEUX and not by Zaxnes is **0** (lot FCEUX-1 step 5, and
unchanged since). Every mechanism the three divergences land on is one of the 37 Zaxnes passes and
FCEUX fails. That is not a proof that Zaxnes has no error here — it is the strongest evidence
available short of hardware, and it points the other way.

The one public test both engines fail is `mmc3_test/6-MMC6`, and Zaxnes's failure there is a
**decision**, not a defect: REQ-CORE-100 pins mapper 4 to MMC3 revision B and `6-MMC6` is the
revision A half of a contradictory pair (AMD-061, Q-114). `test_roms_tests` already records it as
an expected failure.

### No Q ticket was opened, and why

`spec/SPEC-GUIDE.md` is explicit about what a ticket is for: *"Quand une ambiguïté bloque,
l'implémenteur ouvre un ticket `spec/changes/Q-NNN.md` et ne devine pas en silence"* — a **blocking
ambiguity in the spec**, with the provisional choice the implementer made. Nothing here is that:
the spec is not ambiguous about any of the
three mechanisms — REQ-CORE-015/020 and REQ-CORE-100 say what the machine does, `zax-nes` does it,
and the public tests agree with `zax-nes`. The findings are about **FCEUX**, which the spec does
not govern. So there is no ticket to write, and writing one would put a claim about another
emulator into the project's specification.

---

## Step 4 — `.bk2`, and the result that decides the lot

A `.bk2` arrived in `G:/3DSen/test-movies/bk2/` while this lot was running, together with the two
*Faxanadu* dumps it needs and — which turned out to matter more than the movie —
**`bizhawk-runs/`, the owner's own `BizHawk` run of it**: screenshots at frames 1 000, 10 000,
30 000, 60 000, 83 000 and the last frame, for each dump, and a log of a CRC every 100 frames with
a running lag count.

That makes this step something better than a format exercise. The question *"can Zaxnes play a
`.bk2`"* stops being "does the file parse" and becomes **"does the run match the one the movie was
recorded from"**, and there is a reference to answer it against.

### The answer

**Zaxnes's run of the 83 218-frame movie is the same run `BizHawk` made**, at every frame compared:

| frame | Zaxnes against `BizHawk`'s own screenshot |
|---|---|
| 1 000 | **identical** |
| 10 000 | **identical** |
| 30 000 | **identical** |
| 60 000 | **identical** |
| 83 000 | **identical** |
| 83 218 (last) | **identical** |

"Identical" is measured rather than eyeballed. `BizHawk` writes 256×224 (the overscan crop) and
its NES colour table is not Zaxnes's, so the comparison takes this run's rows 8 to 231 against the
reference's rows 0 to 223 and asks whether **every colour of one picture maps to exactly one
colour of the other**. It does, at all six frames, on 9 to 14 distinct colours. A single pixel of
genuinely different *content* would break that mapping, because the colour that pixel took would
then have two images.

And it holds for **both dumps**. The owner ran the movie on `[USA][!!]` and on `[USA][Rev 1][!!]`
and the two `BizHawk` runs differ from frame 100 onwards; Zaxnes run on each dump matches that
dump's reference, and the two Zaxnes runs end on different pictures (`03d7e6a1…` and
`13920624…`). So this is not a movie that would look right whatever the machine did.

### The lag count, which is a second and independent check

The owner's log carries `BizHawk`'s own **lag counter**: `lag=13398` at frame 83 200, the run
ending at 83 218.

This lot's input-sensitivity probe, run over the whole movie on Zaxnes alone, reports **69 820 of
83 218 frames input-sensitive** — that is **13 398 insensitive frames**. The same number.

Two things follow, and the second is the one worth keeping:

1. Zaxnes agrees with `NesHawk` about *which* frames the game polled the pad on, over 83 218
   frames, not only about what is on screen;
2. **the probe is a faithful stand-in for a lag flag.** Step 1 had to invent it because neither
   engine publishes one, and said plainly that it is an operational definition that could differ
   from `lagFlag` in a known direction. On the one run where a real emulator's counter is
   available to check it against, it is exact.

### And FCEUX cannot play this movie at all

The same `.bk2` fed to FCEUX, frame for frame:

| | |
|---|---|
| the two engines part at | frame **3** |
| the first frame that would be seen | frame **12**, 61 435 pixels of 61 440 |
| FCEUX's picture | **does not change for 10 768 frames from frame 14** |
| FCEUX at frame 1 000 | still on the **title screen**, "START / CONTINUE" |
| FCEUX at frame 83 218 | the **password screen**, with nonsense typed into it |
| Zaxnes at frame 1 000, and at every other frame compared | `BizHawk`'s own picture |

FCEUX never gets past the title: the movie's press of Start does not take on it. So on this movie
the roles of steps 2 and 3 are **exactly reversed** — the engine that fails is FCEUX, the engine
that follows the movie is Zaxnes, and it is the same class (b) finding seen from the other side.
`--engine fceux --movie-mode` is not even an option here: FCEUX's movie code reads `.fm2` and
nothing else.

### What the reader does, and what it refuses

`crates/zax-harness/src/bk2.rs`, read from the **public description of the format on TASVideos**
and from the file's own self-description. No `BizHawk` source of any kind was consulted.

**The input log describes itself, and that is the whole design.** The `LogKey:` line names every
button in the order the frame lines write them, `#` starting a new pipe-separated field:

```text
LogKey:#Power|Reset|#P1 Up|P1 Down|P1 Left|P1 Right|P1 Start|P1 Select|P1 B|P1 A|#P2 Up|…|P2 A|
```

so `|..|........|........|` is two console buttons and then eight per pad. The reader **builds the
mapping from the key** rather than hard-coding a layout, and refuses by name any button it has no
word for. That is not fastidiousness: a `.bk2`'s pad order is `Up Down Left Right Start Select B
A` and an `.fm2`'s is `RLDUTSBA` with bit 7 first, so a reader that assumed either would replay
the wrong buttons and look perfectly healthy doing it.
`a_bk2_and_an_fm2_of_the_same_inputs_drive_the_machine_to_the_same_place` is that risk as a test:
400 frames of a sequence that presses every button, written in both formats, must leave `nestest`
on the same picture — with a silent `.bk2` of the same length as the control, because without it
the agreement would prove nothing.

Refused, each with the thing named:

| the movie needs | the message names |
|---|---|
| a `CoreState` member | "anchored to a savestate … a state of another emulator's core" |
| `StartsFromSavestate` / `StartsFromSaveRam` | "starts from a savestate" / "from a battery save" |
| `Platform` other than `NES` | the system it is of |
| a `RegionOverride` other than 0 | the value, and that the public description does not say what it means |
| non-empty `BoardProperties` | the properties, by key |
| `Famicom`, or a port that is not `ControllerNES`/`UnpluggedNES`, or anything on `FamicomExpPort` | the device |
| a `LogKey` naming anything else | the names it could not read |

The format description is explicit that a movie is savestate-anchored by the **existence** of
`CoreState` whatever the header says, so both are checked and the member wins.

**One thing is deliberately not refused.** The header's `SHA1` is checked against the whole image
and against the data after the iNES header, and when it matches neither the movie is reported
**unverified** rather than "recorded on another ROM". The description says the header carries
"SHA1 (also CRC32 or MD5)" and does **not** say what the digest is taken over, so a refusal would
be a claim this lot cannot support — and the one real `.bk2` in hand proves the point: its
`9A8BF108…` is neither dump's whole-file SHA-1 nor either dump's data SHA-1, and the movie
nevertheless replays on both of them exactly.

Two things the format does not carry, each turned into a note on the report rather than a guess:

* **no power-on work RAM.** There is no `RAMInitOption` in a `.bk2` the way there is in an `.fm2`,
  so the machine's own fill is used — all zero, which is `zax-nes`'s (REQ-CORE-004). *Faxanadu*
  does not read it, as the exact match proves;
* **no region.** `RegionOverride` 0 means the core decides, so the region is taken from the ROM's
  own iNES header — `InesHeader`'s rule. Any other value is refused rather than guessed.

### Checked

| | |
|---|---|
| `bk2` unit tests | **9**, on archives the tests build themselves to the documented layout |
| `movie_sync_tests` | **13** with the feature (2 of them `.bk2`), **6** without |
| `zax-harness` lib tests | **41** |
| `cargo fmt --check`, clippy `-D warnings --all-targets`, both builds | clean |
| the two new dependencies | `zip 7.3` and `sha1 0.11`, **already `zax-app`'s own at those versions**; the build was made with `--offline` to prove nothing is fetched |

---

## Step 5 — the report

`docs/review/MOVIE-SYNC.md`: the instrument and what it was shown to be able to do, the table of
step 2, every divergence of step 3 with its class and its evidence, the `.bk2` support with its
limits and the `BizHawk` comparison, **what is still unknown** (§5, which bounds the `.bk2`
result rather than leaving it to be over-read), and the recommendation on `NesHawk` (§6: **no**,
with four reasons and three things to do instead, in order of what they are worth).

`docs/review/FCEUX-ENGINE.md` §5 gains a pointer to it, because a reader who has just read how
`.fm2` playback works on the FCEUX engine is exactly the reader who should be told that the
movies do **not** sync on Zaxnes's own and why.

## The commands, to re-run everything

Set up once:

```sh
export CARGO_TARGET_DIR=<worktree>/rust/target-ev
export CARGO_INCREMENTAL=0
export TMP=<worktree>/rust/target-ev/tmp TEMP="$TMP"
cd <worktree>/rust
cargo build --release -p zax-harness --features fceux-core
H=./target-ev/release/zax-harness.exe
M=G:/3DSen/test-movies           # Windows-style paths: the ROM names hold [ ] and !, which a
R=$M/roms                        # Git-Bash path conversion mangles
```

### The four `.fm2` movies (step 2), whole movie, probe on

About 1½ minutes each, 5½ for *Kirby*:

```sh
$H movie-sync "$M/Kabuki-NTSC-no-FDS.fm2"            "$R/Kabuki - Quantum Fighter (USA).nes" --engine both --lag
$H movie-sync "$M/Kabuki - Quantum Fighter_mod_mod2.fm2" "$R/Kabuki - Quantum Fighter (USA).nes" --engine both --lag
$H movie-sync "$M/MetalForce-DTC5CompV8.fm2"         "$R/Metal Force [Korea][Unl][!].nes"     --engine both --lag
$H movie-sync "$M/taseditor-kirbysadventure.fm2"     "$R/Kirby's Adventure [USA][!!].nes"     --engine both --lag
```

Add `--report <file.json>` for the whole measurement, `--progress 20000` for a line now and then.

### How far FCEUX plays each one, through its own movie mode

```sh
$H movie-sync "$M/<movie>.fm2" "$R/<rom>.nes" --engine fceux --movie-mode --no-lag
```

The `_mod_mod2` file is refused here by name, its header claiming `FDS 1`.

### The pictures, to look at rather than hash

```sh
$H movie-sync "$M/Kabuki-NTSC-no-FDS.fm2" "$R/Kabuki - Quantum Fighter (USA).nes" \
    --engine both --no-lag --png <dir> --png-frames 1000,10000,20000,33306
```

### Step 3's own experiments

```sh
# what the power-on pattern is worth, per game
for r in zero default ff; do $H movie-sync "$M/<movie>.fm2" "$R/<rom>.nes" --engine zaxnes --no-lag --ram $r --frames 2000; done
# is it a whole-frame offset of the boot?
for w in fceux:1 fceux:2 fceux:3 zaxnes:1; do $H movie-sync "$M/taseditor-kirbysadventure.fm2" "$R/Kirby's Adventure [USA][!!].nes" --engine both --no-lag --frames 600 --from 1 --warm-up $w; done
# the verdicts, read off the screen (write a blank .fm2 of 2000 frames first)
T=G:/3dsen/cleanroom/assets-free/test-roms/nes-test-roms
$H movie-sync blank2000.fm2 "$T/ppu_vbl_nmi/rom_singles/01-vbl_basics.nes" --engine both --no-lag --frames 2000 --png <dir> --png-frames 2000
$H movie-sync blank2000.fm2 "$T/mmc3_test/4-scanline_timing.nes"           --engine both --no-lag --frames 2000 --png <dir> --png-frames 2000
```

A blank `.fm2` of *n* frames is `version 3` / `palFlag 0` / `port0 1` / `port1 0` / `port2 0` and
then *n* lines of `|0|........|||`.

### The `.bk2` (step 4)

```sh
$H movie-sync "$M/bk2/invariel-faxanadu.bk2" "$R/Faxanadu [USA][!!].nes" --engine both --no-lag \
    --png <dir> --png-frames 1000,10000,30000,60000,83000,83218
$H movie-sync "$M/bk2/invariel-faxanadu.bk2" "$R/Faxanadu [USA][!!].nes" --engine zaxnes --lag
```

The second prints `69820 of 83218 probed frames were input-sensitive`, which is **13 398**
insensitive — `BizHawk`'s own `lag=13398`.

The comparison against `BizHawk`'s screenshots is a short script that reads the two PNGs, takes
this run's rows 8 to 231 against the reference's 224, and checks that every colour of one maps to
exactly one colour of the other. It is not committed (it reads pictures of a commercial game);
the rule is one paragraph and the §4 of the report states it.

**To run a new `.bk2` the owner adds**, the whole of steps 2 to 4 for it is:

```sh
$H movie-sync "$M/bk2/<new>.bk2" "$R/<its rom>.nes" --engine both --lag --report <out.json> --progress 20000
```

If the reader refuses it, the message names the one thing it cannot honour.

### The tests

```sh
export ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms
cargo test --release -p zax-harness --features fceux-core --lib
cargo test --release -p zax-harness --features fceux-core --test movie_sync_tests -- --include-ignored --test-threads 1
cargo test --release -p zax-harness --test movie_sync_tests -- --include-ignored    # feature off
```
