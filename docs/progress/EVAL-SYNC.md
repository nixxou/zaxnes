# EVAL-SYNC — where Zaxnes's engine parts from a TAS movie, and whether `.bk2` can be played

Branch `eval/bk2`, worktree `cleanroom/wt-bk2`, base `432492c4` (lot FCEUX-48, which gave the
application a second engine and `.fm2` playback on it).

This is an **evaluation**. It measures and explains; it changes no emulation in `zax-nes`.

Read first: `docs/progress/FCEUX-48.md` (how `.fm2` plays on both engines),
`docs/review/FCEUX-ENGINE.md` §5.

## Steps

- [x] 1. The divergence instrument (`zax-harness movie-sync`).
- [ ] 2. Run it on the four movies; the table.
- [ ] 3. Explain each first divergence, and classify it.
- [ ] 4. `.bk2` reading.
- [ ] 5. The report, `docs/review/MOVIE-SYNC.md`.
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
