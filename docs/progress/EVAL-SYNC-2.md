# EVAL-SYNC-2 — where Zaxnes parts from NesHawk on an MMC3 game with resets

Branch `eval/bk2`, worktree `cleanroom/wt-bk2`, base `f1061097` (lot EVAL-SYNC).

EVAL-SYNC concluded that Zaxnes replays a NesHawk `.bk2` exactly, from **one** movie — *Faxanadu*,
MMC1, no resets — and said in so many words that an MMC3 game had not been tried. The owner then
checked a second `.bk2` that **does** sync to the end on BizHawk 2.11.1 and found that Zaxnes
desyncs on it before frame 1 000. This lot is that second movie.

| | |
|---|---|
| the movie | `G:/3DSen/test-movies/bk2/keyword-kirbysadventure-noabilities.bk2` |
| | TASVideos 6192M, BizHawk 2.8, NesHawk, 156 312 frames, **seven Reset presses**, the first at frame 3 |
| the ROM | `Hoshi no Kirby - Yume no Izumi no Monogatari [Japan][!!].nes`, MMC3/TxROM |
| the reference | `G:/3DSen/test-movies/bizhawk-runs/kirby-jp.txt` |
| the calibration | `fax-usa.txt`, `fax-rev1.txt` — the two runs EVAL-SYNC showed Zaxnes already matches |

This is an **evaluation**, like EVAL-SYNC: it measures and explains, and it changes no emulation in
`zax-nes`.

## Steps

- [x] 1. A RAM trace in `movie-sync`, calibrated on the two *Faxanadu* references.
- [x] 2. The first divergence on Kirby, narrowed to the frame.
- [x] 3. The cause, with a class for each finding.
- [ ] 4. Re-check: Kirby after any player fix, and *Faxanadu* to show nothing regressed.
- [ ] 5. The report.
- [ ] 6. The final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` |
| `CARGO_INCREMENTAL` | `0` |
| the fork, MSVC | `cleanroom/fceux-zax/build-msvc`, the existing build — **not rebuilt** |
| `ZAXNES_ROMS` | `G:/3dsen/demo/roms` |
| `ZAXNES_TEST_ROMS` | `G:/3dsen/cleanroom/assets-free/test-roms` |
| `ZAXNES_PROFILES` | `G:/3dsen/cleanroom/assets-neutral` |

## What the inputs say, read before anything was run

### The movie

`Header.txt`, in full:

| key | value |
|---|---|
| `MovieVersion` | `BizHawk v2.0` |
| `emuVersion` / `OriginalEmuVersion` | `Version 2.8` |
| `Platform` | `NES` |
| `GameName` | `Hoshi no Kirby - Yume no Izumi no Monogatari` |
| `SHA1` | `63DCAB3395AB9D1CF98CC93BFC872C29FCA6E6F3` |
| `BoardName` | `TxROM` |
| `Core` | `NesHawk` |
| `rerecordCount` | `155556` |

`SyncSettings.json` carries `"BoardProperties":{}`, `"RegionOverride":0`,
`"NesLeftPort":"ControllerNES"`, `"NesRightPort":"UnpluggedNES"`, `"Famicom":false`,
`"FamicomExpPort":"UnpluggedFam"` and **`"InitialWRamStatePattern":null`** — every one of them a
value `crates/zax-harness/src/bk2.rs` already accepts, which is why the reader takes the file.

The `LogKey` is the one EVAL-SYNC's reader was written against:
`#Power|Reset|#P1 Up|P1 Down|P1 Left|P1 Right|P1 Start|P1 Select|P1 B|P1 A|`. No `Power` press.
**Seven `Reset` presses**, each exactly one frame long, at the 0-based frame indices

    3, 22554, 25798, 29798, 62576, 87950, 92437

### The reference trace

`kirby-jp.txt`: 1 564 numeric lines, frames 100 to 156 300 every 100, then
`end frame 156312 mode FINISHED`. Its lag counter is **6 at frame 100** and only **24 at frame
156 300** — a TAS that lags 24 frames in 156 312. The two *Faxanadu* references are 833 lines each
and end on `lag=13398`, which is the figure EVAL-SYNC's probe reproduced exactly.

---

## Step 1 — the RAM trace, and the setup difference the calibration found

`crates/zax-harness/src/ram_trace.rs` (the trace, the reference reader and the comparison) with
four options in `movie-sync`:

```
  --trace <file.txt>   a work-RAM checksum every --trace-step frames with the running count of
                       input-insensitive frames, in the reference traces' own shape
  --trace-step <n>     how often a line is written (default 100)
  --trace-ref <file>   a BizHawk reference trace to hold the run against, line for line
  --dump <dir> --dump-frames a,b,c   those frames' 2 KiB of work RAM, per engine, raw
```

The checksum is the owner's, reproduced rather than improved on: `s = s*31 + byte` over
`$0000`-`$07FF`, modulo 2^32, eight lower-case hex digits. `--dump` is there because a checksum
says two machines differ and only the bytes say **how** — and it earned its place twice in this
step.

### The offset is measured, not assumed

`RamTrace` keeps **every** frame's checksum, not only the one line in a hundred it writes out, so
the comparison can try a range of offsets (±3) and *report* the one that fits. The answer, on both
*Faxanadu* references:

| | |
|---|---|
| the offset | **0** — the reference's frame *f* is this run's machine after *f* frames |
| the runner-up, at either ±1 | **0** lines of 832 agree |

So the two numberings line up with no shift, which is what `emu.framecount()` being 0 before the
first input word implies — but it is now a measurement, and the runner-up's 0 is the margin that
makes it one.

### What the calibration found on the way: NesHawk's power-on work RAM is **not** zeroes

The first calibration run matched **nothing**: 0 of 832 lines, first difference at frame 100.
Every one of the 832 **lag counts agreed exactly**, which is a run in step, so the 2 KiB was the
thing to look at. Three facts, in the order they came:

1. the difference between this run's checksum and the reference's was the **same constant**
   `0x29F49E` at frames 100, 200 and 300 — a fixed set of addresses holding fixed different
   values, not a drift;
2. over the whole 83 218 frames that constant takes only 336 distinct values on 832 lines, in long
   stretches (one of them 130 lines from frame 39 700 to the last) — the signature of a handful of
   bytes, not of a desynchronised machine;
3. of the three deterministic fills the instrument can write, **only one puts any line in step**:

| `--ram` | lines of 832 agreeing, `fax-usa` | `fax-rev1` |
|---|---|---|
| `zero` (what EVAL-SYNC's `.bk2` reader used) | **0** | 0 |
| `ff` | **0** | 0 |
| `default` — `00 00 00 00 FF FF FF FF` | **207** | **220** |

So `NesHawk` powers the work RAM on with the `00 00 00 00 FF FF FF FF` pattern, the same one
FCEUX's `RAMInitOption 0` writes. That is a **class (c) setup difference** in the harness's own
`.bk2` reader, and it is fixed there: `bk2::read` now asks for `RamPattern::FceuxDefault`, with
the measurement in the note it prints rather than a claim about a core nobody here may read.
`SyncSettings.json`'s `InitialWRamStatePattern` — the one field of the format that speaks about
the 2 KiB, and `null` in both movies in hand — is now read too, and **refused by name** when it
is anything else, because the public description does not say how a non-null value names a fill
and a wrong 2 KiB is not something emulation can recover from.

Neither *Faxanadu* run's picture changed: the last picture is still `03d7e6a1…` on `[USA][!!]`
and `13920624…` on `[Rev 1][!!]`, byte for byte what EVAL-SYNC measured with all-zero RAM. This
game does not read its uninitialised RAM, which is why EVAL-SYNC's six-screenshot match held
anyway.

### So do the two *Faxanadu* traces match line for line? No — and exactly why not

| | `fax-usa` | `fax-rev1` |
|---|---|---|
| lines compared | 832 | 832 |
| checksum lines agreeing | **207** | **220** |
| **lag counts agreeing** | **832 of 832** | **832 of 832** |
| the last agreeing checksum line | **83 200 — the last one** | **83 200** |
| the first differing line | 700 | 700 |

The differing lines are **the same frames on both dumps** (700, 800, 900, 1400, 1500, 1600,
1800, 2000, …), and they cluster on the reference's own lag frames: the reference's lag counter
steps at 700, 900, 1000, 1400, 1600 and 1700, which are differing lines, while the quiet stretches
between them agree.

What the residual is was then settled rather than argued. For each of the 625 differing lines the
delta was solved for a **single** differing byte — for each address *a*, whether
`delta · 31^-(2047-a)` is a value in `-255..255`:

| | `fax-usa` | `fax-rev1` |
|---|---|---|
| differing lines | 625 | 612 |
| of those, explained by **one byte** | **54** | **73** |
| expected by chance over that many lines | **0.15** | 0.15 |
| the page those bytes are in | **`$0100` — the stack — 54 of 54** | `$0100`, 71 of 73 |
| the addresses | `$01D7`-`$01FB` | `$01D7`-`$01FB`, and `$0006`/`$0007` twice |

54 single-byte explanations where chance predicts 0.15, every one of them in the top of the stack
page: the two machines are reading the 2 KiB at a **different point inside the frame**, so the
leftovers below the stack pointer — the bytes an interrupt pushed and the program has popped —
differ. That is a property of where each emulator ends a frame, not of what it emulated: 207
lines agree exactly, including the last; every one of the 832 lag counts agrees; and EVAL-SYNC's
six screenshots are identical.

For what follows, that fixes what a *real* divergence looks like in this instrument and what it
does not: a run in step scores a quarter of its lines, keeps its lag column exactly, and keeps
agreeing to the end. **The last agreeing line is the marker**, not the first differing one.

### Checked

| | |
|---|---|
| `cargo fmt --check` | clean |
| clippy `-D warnings --all-targets`, feature **off** and **on** | clean |
| `zax-harness` lib tests | **42**, up from 31: 9 new in `ram_trace`, 2 new in `bk2` |
| `movie_sync_tests`, feature off | **9**, 3 of them new |

---

## Step 2 — where Kirby parts, and what it is not

### The run

`movie-sync --engine zaxnes --lag`, the whole 156 312 frames, against `kirby-jp.txt`:

| | |
|---|---|
| checksum lines agreeing | **0 of 1 563** |
| the first differing line | **100**: `bc167b92` here, `e8fc3340` in the reference |
| the last agreeing line | **none** |
| distinct deltas over the 1 563 lines | **1 563** — every line its own |
| input-insensitive frames | **25**, against the reference's **24** |

Beside *Faxanadu*'s 207 agreeing lines with every residual in the stack page, this is not a
sampling artefact: **the machines are running different games by frame 100.**

### What the lag column says, which is less than it looks

The two lag counters step at the same lines and on the same cause — a reset — and only three of
the seven resets are counted differently:

| line | 22 600 | 25 800 | 25 900 | 29 800 | 29 900 | 62 600 | 88 000 | 92 500 |
|---|---|---|---|---|---|---|---|---|
| reference | 6→9 | 9→**10** | 10→**12** | 12→**13** | 13→**15** | 15→18 | 18→**21** | 21→**24** |
| Zaxnes | 6→9 | 9→**11** | 11→**12** | 12→**14** | 14→**15** | 15→18 | 18→**22** | 22→**25** |

A reset costs **three** lag frames on both machines; at 25 800 and 29 800 the two engines only
split those three across the 100-frame boundary differently and are back in agreement one line
later. The one real difference is the reset at frame 87 950, where Zaxnes spends **four** —
which is the whole of the 25-against-24.

That agreement is **not** evidence of sync here, and saying so matters. *Kirby* lags 24 frames in
156 312 and 6 of them are in the first 100: outside the resets the counter never moves, so it
cannot distinguish two runs. On *Faxanadu*, with 13 398 lag frames, it could and did.

### It is not a time shift

If Zaxnes were simply running the same game late or early, some frame of ours would hold the
reference's 2 KiB. Every frame of the first 5 000 was checksummed (`--trace-step 1`) and looked
up against each reference value from 100 to 5 000: **no frame of ours matches any line of
theirs.** The difference is of content, not of phase.

### How big the difference at line 100 is

The solver of step 1 was run on the first six lines. Where *Faxanadu* gave 54 single-byte
explanations all in the stack page, *Kirby* gives:

| line | 100 | 200 | 300 | 400 | 500 | 600 |
|---|---|---|---|---|---|---|
| one-byte explanations | 0 | 0 | 0 | 0 | 0 | 0 |
| two-byte solutions | 29 | 30 | 32 | 28 | 32 | 20 |

Thirty two-byte solutions is what **chance** produces (step 1's own figure on a run known to be in
step), so they are noise: many bytes differ by frame 100, not one or two.

### What is on screen

| frame 1 000 | |
|---|---|
| `BizHawk` (`kirby-jp-f1000.png`) | Vegetable Valley, playing, `SCORE : 0003400` |
| Zaxnes | the **LEVEL 1 : VEGETABLE VALLEY** intro card, Kirby flying in over the wipe |

Zaxnes is behind — but not merely behind. Our frames **900 to 2 600, every tenth**, were written
out and held against `kirby-jp-f1000.png` under the colour-bijection rule EVAL-SYNC used (this
run's rows 8-231 against the reference's 224, every colour of one mapping to exactly one colour
of the other). **None of the 171 matches.** The same script says `MATCH` on *Faxanadu* at frames
1 000 (9 colours) and 10 000 (14 colours), so it is a test that can pass.

### So the divergence is inside frames 1 to 100

The reference has no line before 100 and no screenshot before 1 000, so that is the floor the
data itself imposes. Inside that window the movie does exactly four things: a **Reset on frame 3**
(0-based; this instrument's frame 4) and three **one-frame A presses at frames 63, 97 and 212**.

Everything else in the window was then eliminated by measurement, each a run of 3 000 frames
against the reference (none of them puts a single line in step):

| what was varied | values tried | lines agreeing |
|---|---|---|
| the power-on work RAM (`--ram`) | `default`, `zero`, `ff` | 0, 0, 0 |
| the cartridge's PRG-RAM (`--prg-ram`) | `default`, `zero`, `ff` | 0, 0, 0 |
| the reset column (`--resets`) | `on`, `off`, `early`, `late` | 0, 0, 0, 0 |
| all twelve combinations of the first and third | | 0 |
| blank frames before the movie (`--warm-up zaxnes:`) | 1, 2, 3, 4 | 0, 0, 0, 0 |
| the PPU write-inhibition window (`--ppu-warmup`) | on | 0 |

Two of those are worth more than a zero:

* **`--resets off` changes the run from frame 4** (its per-frame trace parts there from the
  honoured one) **and yet changes nothing that can be seen**: the pictures at frames 400 and
  1 000 are identical with and without every reset in the movie. So the first reset is real but
  not where the two machines part;
* **`--ppu-warmup` changes the run not at all** — 1 200 frames of per-frame checksums, byte for
  byte identical to the run without it. That is a result, not a null: it says *Kirby* writes
  neither PPUCTRL nor PPUMASK in the frame after power-on or after the frame-4 reset, so the
  window `zax-nes` does not model (step 3) is not what parts this movie.

---

## Step 3 — the cause, in the order the brief asks for it

Said first, because the honest answer is not the tidy one: **the cause of Kirby's desync was not
found.** What was found is one **class (c)** setup difference (step 1, fixed), one **class (a)**
emulation error that is real, hardware-grounded and *not* this desync, and a list of eliminations
that is worth more than a guess would have been. The two candidates the eliminations leave are
named at the end with what would settle each.

### (a) The reset

**How the `.bk2` column maps.** The `LogKey:` line names `Reset` as the second console button of
the first field, so the column is a **per-frame boolean** — the public description of the format
carries no "held" semantics and no sub-frame position; a reset held over *n* frames is *n*
consecutive frames with the bit set. `bk2::button` turns it into `FrameCommand::SoftReset` and
`movie_sync::step` taps the line **before** that frame runs, which lines up with `emu.framecount()`
being the frame about to be emulated. All seven of this movie's resets are **exactly one frame**
(indices 3, 22 554, 25 798, 29 798, 62 576, 87 950, 92 437), so the question of a held reset does
not arise here — and step 2 measured the one-frame uncertainty anyway (`--resets early|late`:
nothing).

**What a reset does on the hardware** (nesdev, "CPU power up state" and "PPU power up state",
*after reset*), beside what `zax-nes` does:

| | hardware | `zax-nes` |
|---|---|---|
| internal RAM | *"unchanged after a reset"* | kept (`soft_reset` clears nothing) ✓ |
| CPU A, X, Y, flags | unchanged; `I` set; `S -= 3`; PC from `$FFFC` | the same (`Cpu6502::soft_reset`) ✓ |
| cartridge PRG-RAM / CHR-RAM | unchanged | kept ✓ |
| APU | triangle phase 0, `$4011 &= 1`, `$4017` mode kept | the same; the frame-sequencer choice is Q-078 §2, and the six public `apu_reset` ROMs pass ✓ |
| PPU state | PPUCTRL/PPUMASK 0, write latch cleared, `$2007` buffer 0, odd-frame flag even | the same ✓ |
| PPU writes ignored for ~29 658 CPU cycles | yes | **not modelled** (Q-078 §1) |
| **the mapper** | **nothing happens to it** | **put back to its power-on state** |

The last row is the finding, and it is not a reading of a grey area. nesdev's "Cartridge
connector" lists every pin of both the 60-pin Famicom and the 72-pin NES edge — power, the CPU
address, data and control lines, the PPU lines, CIRAM, audio, `/IRQ`, the CIC pins — and there is
**no `/RESET` among them**. A mapper cannot see the console's reset button, so an MMC3 keeps every
register the game last wrote into it across as many resets as the player presses.

> **Class (a) — Zaxnes emulation error.** `NesCore::soft_reset`
> (`rust/crates/zax-nes/src/nes_core.rs:533`) calls `Cartridge::reset`, which for mapper 4 is
> `Mmc3Board::reset` (`rust/crates/zax-nes/src/cartridges/mappers/mmc3.rs:1162`): the eight bank
> registers, the bank-select latch, the PRG-RAM protect byte, the whole IRQ unit
> (`irq_latch`, `irq_counter`, `irq_reload`, `irq_enabled`, `irq_pending`, `irq_prescaler`) and
> the A12 watcher all go back to power-on.
>
> **Reproduction**: `rust/crates/zax-harness/tests/soft_reset_mapper_tests.rs`, three tests on a
> public MMC3 ROM and no movie —
> `a_soft_reset_puts_the_mmc3_bank_registers_back_to_power_on` selects a PRG bank, checks the
> `$8000`-`$9FFF` window really moved, presses RESET and finds the window back where it started.
>
> **Size of the fix**: one line in `NesCore::soft_reset` — not calling `Cartridge::reset` — plus a
> decision for the boards that *do* react to a reset without a pin (the reset-counting multicarts,
> which detect it by the absence of M2 rather than by a wire) and an amendment to REQ-CORE-122,
> whose "PPU, APU **et mapper** suivent leur état documenté *after reset*" is what the current code
> implements. The mapper half of that clause has no documented state behind it. **Not fixed here**:
> this is an evaluation, and the clause is the spec's.

**And it is not what parts this movie.** `--resets off` leaves *Kirby*'s pictures at frames 400
and 1 000 **identical** to the honoured run, and neither puts a line of the trace in step. The
first reset lands at frame 4, three frames into a boot that has not yet written a bank register
worth keeping. Where it would bite is the six later resets, at 22 554 and after — by which point
the two machines are 22 000 frames apart and nothing can be measured.

### (b) The power-on state

Everything that could be varied from outside the core was varied, and the Zaxnes run held against
`kirby-jp.txt` over 3 000 frames each time:

| the memory | how it is varied | tried | lines in step |
|---|---|---|---|
| the console's **work RAM** (2 KiB) | `--ram` | `default`, `zero`, `ff` | **the step-1 finding**: `default` is NesHawk's, measured on *Faxanadu*; on *Kirby* all three give 0 |
| the cartridge's **PRG-RAM** (8 KiB, battery) | `--prg-ram`, through `NesCore::import_battery_ram` | `default`, `zero`, `ff` | 0, 0, 0 |
| the PPU's **OAM, palette RAM and nametable RAM** | `--ppu-ram`, through `$2003`/`$2004` and `$2006`/`$2007` | `default`, `zero`, `ff`, `random:0` | 0, 0, 0, 0 |

The third one needed proving before its zeros meant anything — an option that writes nowhere says
"no difference" on every game — so
`a_debug_write_to_the_ppu_registers_reaches_the_palette_and_oam` reads each of the three back
through the PPU's own ports, and the CLI shows the effect plainly: *Kirby*'s frame 1 is **grey**
(`$00`) with `--ppu-ram zero` and **black** (`$3F`) with `--ppu-ram ff`, the backdrop following
the palette it was given. By frame 60 the game has written its own and the two runs converge;
the work RAM is identical across 400 frames.

The one power-on item that could **not** be varied is the **MMC3's own registers**. nesdev:
*"Because the values in R6, R7, and $8000 are unspecified at power on, the reset vector must point
into $E000-$FFFF, and code must initialize these before jumping out of $E000-$FFFF."* `zax-nes`
chooses `bank = [0;8]`, `bank[7] = 1`, `select = 0`; `NesHawk`'s choice is not public, and nothing
outside the core can set them before the first frame. That nesdev sentence is also the reason to
expect it not to matter: a correct MMC3 game initialises them from the fixed bank before it can
care.

### (c) The MMC3 IRQ and A12

Not a candidate, and the evidence is the public tests rather than a reading. Lot FCEUX-1 and
EVAL-SYNC measured the ten `mmc3_test` / `mmc3_test_2` ROMs on both engines: **Zaxnes passes all
of them** (the eleventh, `6-MMC6`, is the revision-A half of a contradictory pair and is an
expected failure by REQ-CORE-100 / AMD-061 / Q-114), and FCEUX fails all ten. The A12 watcher
(`rust/crates/zax-nes/src/cartridges/a12_watcher.rs`) is written from nesdev's own description —
a filter of ten dots, about three CPU cycles, chosen so that the four-dot dips of the nametable
and attribute fetches do not count while the long lows of the sprite phase and of a hand-written
`$2006` do — and MR-A12-1 re-checked it against the sources.

A mapper-IRQ error also has a shape, and *Kirby*'s run does not have it: a counter that fires on
the wrong line tears the status bar or shakes the split, and the pictures at frames 400 to 2 600
are well-formed, with the HUD where it belongs.

### (d) What the narrowing leaves

Two candidates survive, and neither can be settled with the instrument this lot is allowed to
build. Both are inside the first ~60 frames, which is where step 2 put the divergence, and both
are documented choices of `zax-nes` rather than oversights:

1. **Where the PPU is at power-on.** REQ-CORE-020's notes and Q-003 put `zax-nes`'s machine at the
   **start of vblank** of the region in force (`Ppu2C02::reset`: `scanline = vblank_scanline`,
   `dot = 1`). `NesHawk`'s power-on position is not published. A game whose boot is a chain of
   vblank waits — which *Kirby*'s is: its first eight frames are a flat backdrop — comes out of
   that chain at a frame boundary that depends on the starting dot, and a TAS whose first input is
   a **one-frame A press at frame 63** needs that boundary to the frame. Nothing outside the core
   can move the starting dot, so this was not tried.
2. **The write-inhibition window for `$2005` and `$2006`.** `--ppu-warmup` covers `$2000` and
   `$2001` and found *Kirby* writes neither in the frame after a reset — but the window covers
   PPUSCROLL and PPUADDR too, and those cannot be undone from outside the core: zeroing `v`, `t`
   and `fine_x` needs two `$2006` writes after a *side-effecting* read of `$2002`, which would
   clear the vblank flag the game is about to read. Q-078 §1 chose not to model the window at all,
   for the power-on as well as for the reset.

What would settle both is the same thing: a `cfg`-gated experiment **inside `zax-nes`** — a
power-on dot the harness can choose, and a cycle counter in the PPU that drops writes to the four
registers for 29 658 cycles — with the feature off by default and golden-core unchanged. That is
twenty or thirty lines in `Ppu2C02`, and it is a change to the core, which this lot may not make.
With it, the command that answers the question is the one `--trace-ref` line already in use.

### No Q ticket was opened, and why

The one place the spec could be said to be wrong is REQ-CORE-122's "PPU, APU **et mapper** suivent
leur état documenté *after reset*", since the mapper has no documented after-reset state. But
`spec/SPEC-GUIDE.md` reserves a `Q-NNN` for an ambiguity that **blocks an implementer**, with the
provisional choice they made: nothing is blocked here, the clause was implemented as written
(Q-078 already records the reading of its two companions), and what is needed is an **amendment**
— a decision by the owner that the clause should not cover the mapper — not a question from an
evaluator. It is written up in `docs/review/MOVIE-SYNC.md` with its size so that the owner can
raise one.
