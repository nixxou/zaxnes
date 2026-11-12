# EVAL-SYNC-3 — the Reset semantics of a `.bk2`, frame 246 of *Kirby*, and the mapper's soft reset

Branch `eval/bk2`, worktree `cleanroom/wt-bk2`, base `5091ccb6` (lot EVAL-SYNC-2).

EVAL-SYNC-2 ended with *Kirby* desynchronising somewhere inside the first hundred frames and no
cause, because the reference it had could not see below frame 100. The owner then produced a
**per-frame** reference — `G:/3DSen/test-movies/bizhawk-runs/kirby-jp-step1.txt`, 400 lines of
`<f> <checksum> lag=<n> ram=<4096 hex digits>` — and measured two things with it:

1. `BizHawk` runs the frame whose Reset bit is set **normally** and resets **after** it, which is
   `--resets late` in EVAL-SYNC-2's instrument and not its default;
2. with that reading the two machines agree to **frame 246** and the first real difference is
   `$004B` at the start of frame **247**.

This lot acts on both, and on one decision the owner took on 2026-11-10: the mapper's soft reset
is to be **fixed**, which makes this the first lot of the three to change `zax-nes`.

## Steps

- [x] 1. The Reset semantics in the `.bk2` reader, and the same question for `.fm2`.
- [x] 2. What frame 246 does, on Zaxnes.
- [x] 3. What the hardware does there.
- [x] 4. The mapper's soft reset.
- [x] 5. The report.
- [x] 6. The final pass.

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
| disk on `G:` at the start | 11 GB free |

---

## Step 1 — when a reset happens, in each of the two formats

### What changed, and what it is relative to

Every `FrameInput::command` of a `MovieFile` means **"applied before this frame runs"** — that is
`movie_sync::step`'s one invariant and it has not moved. What is new is that a *format* may place
its column differently, which `MovieFile::reset_after_frame` now carries and `relocate_resets`
turns into a base shift:

| | the file's column means | base shift | `--resets on` | `early` | `late` |
|---|---|---|---|---|---|
| `.fm2` | before the flagged frame | 0 | the flagged frame | one before | one after |
| `.bk2` | **after** the flagged frame | **+1** | **one after the flagged frame** | the flagged frame | two after |

So `--resets` is now **relative to the format's own placement** rather than to the file's own
frame numbers, and the three honouring values still bracket a one-frame uncertainty. The
consequence for anyone re-running EVAL-SYNC-2's commands: that lot's `--resets late` on a `.bk2`
is this lot's **default**, and this lot's `--resets early` is that lot's default. Both documents
say so where they say the figure.

### The `.bk2` answer, measured on the owner's per-frame reference

`movie-sync --engine zaxnes --lag --frames 400 --trace-step 1 --trace-ref <ref> --ram-ref <ref>`
against `kirby-jp-step1.txt`, at each of the four placements:

| `--resets` | checksum lines agreeing, of 400 | lag counts | frames with no **live** RAM difference | the first one |
|---|---|---|---|---|
| **`on`** (the new default) | **89** | **all 400 agree** | **246** | **frame 247** |
| `early` (EVAL-SYNC-2's default) | 3 | part at frame **4**: 4 here, 3 there | 3 | frame 4 |
| `late` | 3 | part at frame 5: 3 here, 4 there | 4 | frame 5 |
| `off` | 3 | part at frame 5 | 4 | frame 5 |

The lag column is what makes this a measurement rather than a preference. *Kirby* lags 6 frames in
its first 100 and the reset on frame 3 is three of them, so the counter is sharp exactly here:
resetting **before** frame 3 makes frame 3 the first lag frame and the counter reads 4 where
`BizHawk`'s reads 3; resetting **after** it leaves frame 3 a normal frame and all 400 counts
agree. The work RAM says the same thing from the other side — with `early` the 2 KiB does not move
until frame 6 while the reference's moves during frame 3.

"89 of 400 lines" and "246 of 400 frames with no live difference" are the same artefact
EVAL-SYNC-2 measured on *Faxanadu*: the two machines read the 2 KiB at a slightly different point
inside the frame, so the stack page and the scratch below `$0020` differ on a run that is
otherwise in step. §Step 2 is what the 247 is.

### The `.fm2` answer, measured against FCEUX itself

The public format page (`fceux.com/web/FM2.html`) says a frame's command field is "a variable
length decimal integer which is interpreted as a bit field corresponding to miscellaneous input
states which are **valid at the start of the frame**" — a sentence, and one that does not quite
say whether the frame it is valid at the start of is the flagged one. Reading FCEUX's source would
not settle it either (the brief's point), so the engine was used as its own reference:
`--engine fceux --movie-mode` plays the file through `FCEUI_LoadMovie`, where FCEUX's own movie
player decides when the reset happens, and `--engine fceux --resets <x>` plays the same inputs fed
in by hand.

The movie is synthetic: 60 frames on `nestest.nes`, one soft reset, one A press on frame 30. The
comparison is the whole work RAM at **every** frame (`--trace-step 1`), and the table is the count
of the 60 lines that differ from movie mode:

| the reset is on frame | `--resets early` | **`--resets on`** | `--resets late` | `--resets off` |
|---|---|---|---|---|
| 5 | 55 | **0** | 55 | — |
| 20 | 30 | **0** | 17 | 40 |
| 45 | 9 | **0** | 15 | — |

Unanimous, and the `off` column is the control that says the reset does something at all (a movie
with no reset at all differs from this one on 82 of the 60 compared lines across the two files).
So an `.fm2`'s command takes effect **before** the flagged frame, `reset_after_frame` is `false`
for that format, and nothing about `.fm2` playback changes.

This is kept as a test rather than only as a figure:
`movie_sync_tests::both::an_fm2_resets_before_the_flagged_frame_and_fceuxs_own_movie_mode_says_so`
asserts both halves — that `On` reproduces movie mode frame for frame, and that `Early` and `Late`
do **not**, without which the test would pass vacuously on a ROM the reset does not reach.

### The two instruments this step had to add

Movie mode wrote pictures and no work-RAM trace, so it could not be held against anything; it now
writes the trace (and honours `--dump`) like every other run. That is what made the `.fm2`
measurement possible at all.

And `--ram-ref <file>` / `--ram-ref-offset <n>`: the finer reference carries `ram=<4096 hex
digits>` per line, and a checksum says two machines differ while only the bytes say **how**. The
comparison is streamed (nothing keeps 2 KiB a frame) and sorts each differing address into three
classes:

| class | addresses | why it is a class |
|---|---|---|
| the stack page | `$0100-$01FF` | leftovers below the stack pointer; EVAL-SYNC-2 measured 54 of 54 single-byte residuals here on a movie that syncs exactly |
| the temporaries | `$0000-$001F` | the scratch of whatever routine is running at the moment the 2 KiB is read |
| **live** | everything else | what the game has *decided*, and the only class a divergence can be read off |

A reference carrying no `ram=` field is **refused by name** rather than silently comparing
nothing: "no line of this reference carries a `ram=<hex digits>` field". `--engine both` is
refused too — the comparison is one run against a third machine's bytes, and the lockstep loop is
two runs against each other.

### The re-runs

*Faxanadu* has no reset in its `.bk2`, which makes it the control: nothing about it can change,
and nothing did.

| | `[USA][!!]` | `[USA][Rev 1][!!]` |
|---|---|---|
| checksum lines agreeing, of 832 | **207** | **220** |
| lag counts agreeing | **832 of 832** | **832 of 832** |
| the last agreeing line | **83 200** | **83 200** |
| the last picture | `03d7e6a1…` | `13920624…` |

Byte for byte EVAL-SYNC-2's figures, including the two picture digests.

### Checked

| | |
|---|---|
| `cargo fmt --all --check` | clean |
| clippy `-D warnings --all-targets`, feature **off** and **on** | clean |
| `zax-harness` lib tests | **48**, up from 45: 3 new (1 in `movie_sync`, 2 in `ram_trace`) |
| `movie_sync_tests`, feature off | 9 | 
| `movie_sync_tests`, feature on, `--ignored --test-threads 1` | **8**, one of them new |

---

## Step 2 — what that frame does, and what it turned out to be

Said first, because it is the lot's result: **`$004B` was never the divergence, and the divergence
was not an emulation error.** `movie-sync` had been playing *Hoshi no Kirby*'s **3D widescreen
hack** instead of the ROM the movie was recorded on, and the hack's **overclock** — extra CPU time
each frame, which no NES gives — is what parted the run. With the patches off the first 400 frames
agree with `BizHawk` **byte for byte in every live address**, and the lag count over the whole
156 312 frames is **24**, exactly `BizHawk`'s.

### The numbering, settled before anything was traced

The reference's line *f* is the 2 KiB at the **start** of its frame *f*, which is the machine after
*f* frames. So "the difference appears at the start of frame 247" means it was written during the
**247th frame run**, which is `frame-trace --frame 247` and `movie-sync`'s frame 247. The brief's
"frame 246" is the same frame in the reference's own 0-based naming. Tracing 246 first found
nothing, which is how the convention was pinned rather than assumed.

### The instrument: `zax-harness frame-trace`

A new subcommand, and **entirely outside `zax-nes`** — no `cfg`, no feature, no new core API. It
plays a movie to a chosen frame through `movie-sync`'s own machinery, then drives that one frame by
hand with four public pieces that already existed:

| what it needs | what it uses |
|---|---|
| the state before an instruction, disassembled | `zax_nes::cpu::CpuTracer::capture` — there for `nestest.log` (REQ-CORE-110), and it reads only through `CpuBus::peek` (REQ-CORE-081) |
| one instruction at a time | `NesCore::cpu_and_bus_mut`, whose doc comment says "to step instructions by hand" |
| where the frame ends | `Ppu2C02::vblank_just_started`, what `run_frame`'s own loop breaks on |
| where the PPU is | `Ppu2C02::scanline` and `dot` |

Options: `--watch <addr>` (every change of an address, with the instruction that made it and the
preceding `--context` lines), `--out` (the whole frame, one line per instruction, with the PPU's
scanline and dot and the NMI/IRQ handler entries marked), `--frames <n>`, and `--converge` below.

**It checks itself.** The run takes the work-RAM checksum of the traced frame and of the same
frame played the ordinary way from the same state, and prints both. Every run of this step
reported `IDENTICAL, so tracing changed nothing`.

### `--converge`: the measurement that tells a divergence from a reading artefact

A reference line says what the 2 KiB held at the start of a frame; it does **not** say at which dot
of the previous frame the reading was taken. So `--converge <reference>` compares the run's own
2 KiB against the reference's bytes for that frame at **every instruction** of the frame (and
`--extra` instructions past its boundary), and reports where it comes closest. If some instant of
our frame holds the reference's bytes, the two machines ran the same frame and only wrote it down
at different moments.

The count that decides is **live bytes alone** — outside the stack page and `$0000-$001F`. The
stack below the stack pointer is *dead*: two machines whose interrupts landed even slightly apart
never agree on it again, so "identical to the byte" is unreachable by construction while "no live
byte differs" is exactly the question.

### What it found at frame 247, with the patches still on

| | |
|---|---|
| the frame | 8 283 instructions, 29 780 CPU cycles, one NMI, no `$2002` read at all |
| `$004B` | `INC $4B` at **`$E94F`**, instruction 8 264 — **19 instructions before the frame boundary** |
| what restores it | `PLA` / `STA $4B` at **`$E6D2`-`$E6D3`**, which the frame ends before reaching |
| the context | `$E91E: LDA $4B / CMP $67EE / BCS $E948 … $E94F: INC $4B` — a loop counter, saved and restored around a call |
| live bytes differing at the boundary | **1** (`$004B`) |
| live bytes differing at instruction 8 263 and for the 1 020 instants before it | **0** |

So **nothing a read could have decided**: `$004B` is `01` only inside a 22-instruction window of
one routine, and our frame boundary falls inside it while `BizHawk`'s reading was taken before it.
The answer to "which read would have to differ" is *none* — it is where the frame ends relative to
the program, which is the same artefact class as the stack page.

That closed the question the brief asked and opened the real one: frames 244-246 had **0** live
differences, 247 had the one explained above, **248 had 12 and 249 had 167, and from 249 onwards no
instant of the frame agreed at all.** So the real divergence is in frame 248 or 249.

### And frame 248 was not an emulation difference either

Three figures of the traced frames made it obvious, and none of them is about the 6502:

| frame | instructions | CPU cycles | instructions costing **no** cycle |
|---|---|---|---|
| 244-247 | 8 283 - 9 120 | 29 778 - 29 784 | 0 |
| **248** | **19 280** | 29 781 | **9 689** |
| 249, 250 | 18 886, 19 265 | 29 781 | 9 689, 9 689 |

19 280 instructions cannot fit in 29 781 CPU cycles — the shortest 6502 instruction is two. The
trace says where they came from: from instruction 9 552 the PPU stands at **scanline 240, dot 0**
and the cycle counter does not move for 9 689 instructions, then both resume. There is exactly one
thing in this core that does that — `WideExtension::take_stall_cycle`
(`crates/zax-nes/src/wide.rs:731`), the **3D extension's overclock**, armed at the start of line
240 (`ppu/mod.rs:1096`) and worth `$5006` scanlines of CPU time with the PPU, the APU and the clock
held still.

`frame-trace` now reports both, and the report named the cause in one word: **the 3D extension
UNLOCKED**. It is unlocked because `NesCore` powers on with `builtin_patches_enabled` **true**
(`nes_core.rs:134`), so `load_rom` hands a recognised commercial game its *widescreen patch*
instead of the image the host passed in. `builtin_patches::KIRBY_JAPAN` is exactly that patch for
this ROM. `test_roms` has always turned it off (`test_roms/mod.rs:482`); **`movie-sync` never
did** — so every Zaxnes measurement of lots EVAL-SYNC and EVAL-SYNC-2 on this game was of the hack,
not of the game.

> **Class (c) — a setup difference of the instrument, and it is fixed here.**
> `ZaxnesMachine::new` now calls `set_builtin_patches(false)` and `set_advanced_patches(false)`
> before `load_rom`, for `movie-sync` and for `frame-trace`. `--builtin-patches` turns them back
> on, which is how the difference above is shown rather than asserted. Nothing in `zax-nes`
> changed.

### What the fix is worth, measured

*Kirby*, `--resets on`, against the per-frame reference over its whole 400 frames:

| | patches **on** | patches **off** |
|---|---|---|
| checksum lines agreeing exactly | 89 of 400 | **190 of 400** |
| lag counts agreeing | 400 of 400 | **400 of 400** |
| frames with **no live difference** | 246 | **400 of 400** |
| the first live difference | frame 247 | **there is none** |
| the 3D extension | UNLOCKED, 9 689 free instructions a frame from 248 | **locked, 0** |

And over the whole movie, against `kirby-jp.txt`:

| | patches on (EVAL-SYNC-2) | patches off |
|---|---|---|
| frames run | 156 312 | 156 312 |
| lag counts agreeing | all but three resets' lines | **all 1 563** |
| input-insensitive frames | **25** | **24 — `BizHawk`'s own figure** |

Frame 247 traced again with the patches off says the same thing from the inside: the extension is
**locked**, no instruction costs no cycle, `$004B` goes `00`→`01`→`00` **inside** the frame as it
does on `BizHawk`, **4** bytes differ in the whole 2 KiB and **0** of them are live.

### What still parts, and it is much smaller than what did

The movie does not sync to the end yet. The pictures, under the colour-bijection rule (this run's
rows 8-231 against the reference's 224, every colour of one mapping to exactly one colour of the
other):

| frame | 1 000 | 10 000 | 30 000 | 60 000 | 83 000 | 156 312 |
|---|---|---|---|---|---|---|
| verdict | no | no | no | no | no | no |
| pixels differing of 57 344 | **275** | 50 537 | 15 445 | 48 207 | 46 435 | 22 912 |

275 pixels is not a desynchronised game: at frame 1 000 every differing pixel is inside **one
25x17 box** at x 147-171, y 97-113 — a single object — and the background, the HUD and the score
are identical. (Before the fix, frame 1 000 was a different screen altogether: `BizHawk` playing
Vegetable Valley with 3 400 points against Zaxnes still on the level-intro card.)

The comparison script was validated before being believed: on *Faxanadu* `[USA][!!]` it says
**MATCH at all six frames**, with the same colour counts EVAL-SYNC-2 recorded (9, 14, 11, 13, 12,
12). It is not committed, for the reason EVAL-SYNC-2 gives.

So a second divergence remains, between frame 400 — where the finest reference stops — and frame
1 000. Step 3 says what would locate it.

### Faxanadu, to show nothing regressed

| | `[USA][!!]` | `[USA][Rev 1][!!]` |
|---|---|---|
| checksum lines agreeing, of 832 | **207** | **220** |
| lag counts agreeing | **832 of 832** | **832 of 832** |
| the last agreeing line | 83 200 | 83 200 |
| the last picture | `03d7e6a1…` | `13920624…` |
| the six screenshots | **6 of 6 MATCH** | — |

Identical to EVAL-SYNC-2's figures: this game has no built-in patch, so turning the patches off
changes nothing about it, which is what makes it the control.

### Checked

| | |
|---|---|
| `cargo fmt --all --check` | clean |
| clippy `-D warnings --all-targets`, feature **off** and **on** | clean |
| `zax-harness` lib tests | **50**, up from 48: 2 new in `frame_trace` |

---

## Step 3 — what the hardware does there

Two questions came out of step 2, and they have different kinds of answer.

### (1) `$004B` at frame 247: the hardware does nothing, because no read is involved

The brief asked which read's value would have to differ for `$004B` to be `00`. The answer is
**none**. `$004B` is a loop counter that one routine increments at `$E94F` and restores from the
stack at `$E6D3`, twenty-two instructions later; it holds `01` only between those two points.
Our frame ends inside that window and `BizHawk`'s 2 KiB was read before it — and `--converge`
measures exactly that: **1 020 instants of our own frame 247 hold the reference's live memory, the
last of them 19 instructions before our frame boundary.**

That is the same class as the stack-page residuals EVAL-SYNC-2 measured on *Faxanadu*, which syncs
exactly. Neither needs a hardware explanation: the hardware has a frame boundary, and a Lua script
has a moment at which it reads RAM, and nothing says the two are the same instant. With the
built-in patches off (step 2) the whole thing disappears anyway: `$004B` goes `00`→`01`→`00`
**inside** frame 247, as it does on `BizHawk`.

### (2) The overclock at frame 248: the hardware has no such thing, and that is the point

The 9 689 instructions of frame 248 that cost no CPU cycle are the 3D extension freezing the PPU,
the APU and the clock while the CPU runs. nesdev's *Cycle reference chart* is unambiguous about
whether a console can do that:

| | NTSC |
|---|---|
| master clock | 21.477272 MHz |
| CPU | master ÷ **12** → 1.789773 MHz |
| PPU | master ÷ **4**, one dot per 4 master clocks |
| CPU cycles per frame | **29 780.5** |

The two clocks are divided from one crystal by fixed integers. There is no register, no pin and no
documented condition under which the 6502 advances and the 2C02 does not; the only thing that
moves CPU time relative to PPU time on real hardware is **DMA**, which *takes* cycles from the CPU
and never grants them. So the overclock is not a behaviour a real NES has and not one any emulator
could be right or wrong about: it is a deliberate feature of the 3D application, and the error was
that a *measurement* ran with it on.

> **Conclusion for both: not a Zaxnes emulation error.** Nothing in `zax-nes` was wrong at frame
> 247 or 248. The fault was `movie-sync`'s, and §Step 2 fixed it in `zax-harness`: the built-in
> patches are off unless `--builtin-patches` asks. The figures either side of that line are in
> step 2's own table, and `zax-nes` is byte-identical to `5091ccb6` as far as this step is
> concerned.

Zaxnes's own frames measure **29 778 to 29 784 CPU cycles**, averaging the 29 780.5 above — the
29 781 of the overclocked frames is the same number, which is how the trace could tell that the
extra 9 689 instructions were free rather than real.

### What *Kirby* actually uses mid-frame, measured rather than guessed

Before naming candidates for the divergence that remains, the trace says which machinery is even
in play. Frame 1 000, traced with the patches off:

| | |
|---|---|
| reads of `$2002` | **0** — no vblank polling, no sprite-0 hit, no sprite overflow, in the whole frame |
| NMI | one, at scanline 241 dot 28 |
| **MMC3 IRQ** | **one, at scanline 174 dot 287**, and its handler runs from zero page (`PC = $0029`) |
| the MMC3 IRQ registers | `$C000` = `$AF` and `$C001`, `$E001` written in vblank at scanline 247; `$E000` written in the IRQ handler and in the NMI handler |

So the game's one piece of mid-frame timing is the **MMC3 scanline counter**, latched at `$AF`.
nesdev, *MMC3*: *"The exact number of scanlines between IRQs is N+1, where N is the value written
to `$C000`"* — 176 scanlines — and the counter is clocked *"on each rising edge of filtered A12,
no matter what caused it"*, the filter being *"rising edge after the line has remained low for
three falling edges of M2"*. That is what `A12Watcher` is written from (MR-A12-1), and the IRQ
landing on scanline 174 is consistent with a reload requested in vblank.

### The public tests that cover it, re-run on Zaxnes for this step

| suite | ROMs | pass | the failures |
|---|---|---|---|
| `mmc3_test` | 6 | 5 | `6-MMC6` — *"IRQ shouldn't occur when reloading after counter normally reached 0"* |
| `mmc3_test_2/rom_singles` | 6 | 5 | `6-MMC3_alt` — *"IRQ shouldn't be set when reloading to 0 due to counter naturally…"* |
| `ppu_vbl_nmi` (the suite and its ten singles) | 11 | **11** | — |
| `instr_timing` | 3 | **3** | — |
| `cpu_timing_test6` | 1 | **1** | — |
| `dmc_dma_during_read4` and the DMA ROMs | 8 | **6**, 2 *manual* | neither is a failure: the two have no documented verdict |

`1-clocking`, `2-details`, `3-A12_clocking`, `4-scanline_timing` and `5-MMC3` pass on **both**
suites — which is the public check of exactly the mechanism *Kirby* uses, scanline timing
included.

The two failures are the **revision-A** half of a contradictory pair, and REQ-CORE-100 pins mapper
4 to revision B (AMD-061, Q-114). **They cannot be *Kirby*'s cause**, and that is a measurement
rather than an assumption: nesdev puts the whole A-against-B difference at `$C000` = `$00` —
revision B *"generates IRQs when the scanline counter is equal to 0"* and revision A *"only
generates IRQs when the scanline counter changes (decrements) to zero from a non-zero value"* —
and *Kirby* writes `$AF`. A game that never latches `$00` cannot tell the two revisions apart.

### Two of EVAL-SYNC-2's candidates are now eliminated

§4b of the report left two survivors, both about the boot, and both needing a `cfg`-gated
experiment inside `zax-nes` to test. **Neither is needed any more**, because the 400 frames the
fine reference covers include the whole boot and **every live byte of all 400 agrees**:

| the candidate | what it predicted | what was measured |
|---|---|---|
| the PPU's power-on dot (Q-003, REQ-CORE-020) | the boot's chain of vblank waits comes out a frame boundary apart, so the one-frame A press at frame 63 lands differently | frames 1-400 agree in every live byte, the A presses at 63, 97 and 212 included |
| the `$2005`/`$2006` write-inhibition window (Q-078 §1) | the same | the same |

So the `cfg`-gated experiment EVAL-SYNC-2 asked for **should not be written**: the question it was
to answer has been answered the other way by the reference the owner produced.

### What remains, and exactly what would decide it

**Undetermined**, and much smaller than it was. The movie still parts: the pictures differ at all
six frames, and at frame 1 000 the difference is **275 pixels of 57 344, every one of them inside a
single 25×17 box** — one object, with the background, the HUD and the score identical. So the
divergence is somewhere in **frames 401 to 1 000** and begins as one object's state.

Three things bound what it can be:

* it is **not** in the boot or the first 400 frames (above);
* it is **not** the overclock or any other part of the 3D extension, which is now locked and
  measurably idle (0 stalled instructions a frame);
* it is **not** `$2002`, sprite 0 or sprite overflow, which *Kirby* does not read at all.

What is left, in order of what the trace makes plausible:

1. **the dot at which the MMC3 IRQ lands.** Zaxnes puts it at scanline 174 dot 287 of frame 1 000.
   `4-scanline_timing` passes on both suites, so Zaxnes is right by the public test; `NesHawk`'s
   dot is not published, and a handful of dots either way moves where the game's handler runs
   relative to the picture it is rewriting;
2. **an open-bus or decay read.** The IRQ handler runs from zero page and the game writes
   `$E000` from two places, which is the shape of code that reads back a bus it does not drive;
   nothing in the frame-1 000 trace shows one, but the trace was not searched exhaustively for it;
3. **a DMC or OAM DMA cycle landing differently.** Every public DMA ROM passes, so this is last.

**The measurement that would decide it**, and it is the owner's to make — the same script that
produced `kirby-jp-step1.txt`, pointed at a later window:

| | |
|---|---|
| the frames | **400 to 1 100**, `step = 1` |
| the format | exactly `kirby-jp-step1.txt`'s: `<f> <checksum> lag=<n> ram=<4096 hex digits>` |
| why that window | it brackets the first picture difference from both sides: frame 400 is the last frame known to agree and frame 1 000 is the first known to differ |
| what it answers in one run | the frame of the first live difference and its addresses, exactly as `kirby-jp-step1.txt` gave frame 247 and `$004B` |

Held against `movie-sync --ram-ref`, that names the frame; `frame-trace --frame <f> --watch <addr>
--converge` then names the instruction, as it did in step 2. **Nothing else is needed from this
side first** — and in particular no change to `zax-nes`.

If that trace shows the first live difference is in pages `$0400`-`$05FF` (where frame 249's 167
bytes were concentrated: 124 in page 04, 28 in page 05) **and** the frame before it agrees, the
second thing to ask for is a per-scanline log of that one frame:

| per scanline 0-261, for the one frame | |
|---|---|
| `$2000` and `$2001` as the game last wrote them | the split's own state |
| the MMC3 IRQ counter, its latch, whether a reload is pending and whether IRQs are enabled | the counter Zaxnes and `NesHawk` could disagree about |
| the scanline **and dot** at which `/IRQ` is asserted, and at which the handler's first instruction runs | the one figure candidate 1 turns on |
| the CPU cycle count at the start of the scanline | so that the two machines' frames can be lined up cycle for cycle rather than frame for frame |

Zaxnes can produce all four from `frame-trace --out` today: the IRQ entry is marked in the trace
and every line carries its scanline, dot and `CYC`.

---

## Step 4 — the mapper's soft reset

The owner's decision of 2026-11-10, and the one emulation change this lot makes.

### What changed, in five places

| | |
|---|---|
| `Mapper::soft_reset` (`cartridges/mappers/mod.rs`) | **new**, and it does **nothing** by default, with the nesdev references in its own doc comment |
| `NesCore::soft_reset` (`nes_core.rs`) | calls `Cartridge::soft_reset` instead of `Cartridge::reset` |
| `Cartridge::soft_reset` (`cartridges/mod.rs`) | **new**: `board.soft_reset(memory)`, then the two cache refreshes. The mirroring and the M2 cycle counter are **no longer zeroed**; `Cartridge::reset` still zeroes both |
| `Mmc3Unl8237::soft_reset` (`mappers/mmc3_unl8237.rs`) | the **only** override: `$5001` back to `$0F` |
| `IrqBoard`, `SmallBoard`, `DecodeBoard`, `Mmc3ChrRam` | forward the call, so that the 215 hears it through their dispatches |

The four forwards are what the first run of the multicart test caught: those enums forward `reset`
by hand, so without a `soft_reset` arm the default no-op answered for every board behind them and
the 215 never heard the reset. One cold body each, in a dispatch nothing hot inlines — `soft_reset`
runs once per reset and not once per cycle, which is why the performance notes those files carry
about `irq_asserted` and `chr_view` do not apply.

### Which boards react, and why only one

The default is the hardware's. nesdev's "Cartridge connector" lists every pin of both edges — the
60-pin Famicom and the 72-pin NES: power and ground, the CPU's address, data and control lines, the
PPU's lines, CIRAM, audio, `/IRQ`, `M2`, `/ROMSEL`, `SYSTEM CLK`, the expansion pins and the CIC's
two — and **no `/RESET` among them**. nesdev's "CPU power up state", *after reset*, says the
internal RAM is "unchanged" and does not mention the cartridge at all.

The boards that do react see the reset as the **absence of M2**. nesdev, "NES 2.0 Mapper 313"
(reset-based multicarts around the MMC3 ASIC): *"When an interruption of the M2 signal is detected,
the higher address lines are automatically incremented."* Of those numbers — 60, 313, 331, 343,
349, 350, 351, 356 and the rest of the reset-based family — **this build implements none**, with one
exception: mapper **215**, whose own page states the behaviour in so many words:

> *"The Outer Bank Register is reset to its power-up value if a Reset has been detected via an
> interruption of the M2 signal. This makes it difficult to dump these cartridges using a Kazzo
> device."*

So one override, of one register, with the page's sentence beside it. Nothing else in the build
claims a documented reset reaction — searched for, not assumed: the only other mentions of a
register cleared "on reset" are mapper 163's and the PEC-9588's, and both are power-on language
(the sentence that follows each is about where the game's **reset vectors** are found, i.e. what
the board must present at power-up).

### The spec, which this lot may not edit

`docs/review/AMD-REQ-CORE-122-mapper-reset.md` carries the proposed amendment: the exact new
wording of REQ-CORE-122's third tiret, in French, with the nesdev citations and the rationale. It
is built on the moule of **AMD-061 §5**, which is the same discovery made once already for the
*hard* reset (Q-112: "sauf pour une carte dont la documentation publique prescrit que son état
survit au signal RESET ; la liste de ces cartes est tenue par la spec", list of one — mapper 28).
The proposed clause is its mirror image: for the soft reset the default is **conservation** and the
exception is **reaction**, and its list is also of one — mapper 215.

REQ-CORE-004 and DEC-02-Q3 are untouched: a hard reset stays a power cycle.

### Tests

Three new in `zax-nes`'s own `soft_reset_tests.rs`, which is where REQ-CORE-122 is measured:

| | |
|---|---|
| `req_core_122_a_soft_reset_leaves_the_mmc3_alone` | four bank registers and the mirroring survive a soft reset on a mapper-4 image; a **power cycle still takes them away** |
| `req_core_122_a_soft_reset_keeps_the_mmc3_irq_armed` | measured through the `/IRQ` **line**, with rendering off and the A12 rises made by hand as the other MMC3-clone IRQ tests make them: the latch and the enable survive, so the counter fires again with no further write — and a power cycle silences it however long it is clocked |
| `req_core_122_a_reset_detecting_multicart_puts_its_outer_bank_back` | mapper 215: `$5001` back to `$0F` and the MMC3 registers behind it untouched. `#[cfg(feature = "board-215")]` |

The multicart test uses the synthetic image `MapperRom` builds, because **there is no mapper-215
image under `assets-free/test-roms`** — §3.4 of `docs/review/MF-6-inventory.md`: 48 MMC3-clone
numbers, none with a licensed title. That is checked rather than assumed: every `.nes` under
`assets-free/test-roms` was read and its mapper number taken from its header, and none is 215.
Every other mapper-215 test of this workspace uses the same synthetic image.

And EVAL-SYNC-2's `soft_reset_mapper_tests.rs` is **flipped**, as its own module comment said
whoever fixed this should do. It is now a contract rather than a reproduction:
`a_soft_reset_puts_the_mmc3_bank_registers_back_to_power_on` becomes
`a_soft_reset_keeps_the_mmc3_bank_registers_and_a_power_cycle_does_not`, on the same public
`mmc3_test/1-clocking.nes` and with the power cycle asserted beside the soft reset.

### Checked

| | |
|---|---|
| `zax-nes`, the whole suite | **1 177 passed, 0 failed, 10 ignored** |
| `zax-harness soft_reset_mapper_tests` | 3 passed |
| **`golden-core`**, 98 originals, 1 200 frames | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773` |

The golden-core digest is **byte-identical** to FCEUX-23's baseline and to EVAL-SYNC's and
EVAL-SYNC-2's. That is the expected answer and it is also the bound on the change: `golden-core`
drives `NesCore::reset`, the power cycle, which this lot does not touch. No power-on behaviour
moved, so nothing had to be explained away.

### Kirby, re-run to the end

| | |
|---|---|
| frames run | **156 312**, the whole movie, all seven resets, no stop |
| the seven resets land on | frames 4, 22 555, 25 799, 29 799, 62 577, 87 951 and 92 438 (the `.bk2` semantics of step 1) |
| checksum lines agreeing | 2 of 1 563, the last at frame 200 |
| **lag counts agreeing** | **all 1 563**, and the total is **24** — `BizHawk`'s own figure |
| the first 400 frames, byte for byte | **190 of 400 identical, and all 400 with no live difference** |
| the six screenshots | **byte-identical to step 2's run, at all six frames** |

So the mapper fix changes **nothing at all** on this movie, and that is not a disappointment but
the expected result twice over. The only reset before the divergence is the one on frame 4, three
frames into a boot that has not yet written a bank register worth keeping — EVAL-SYNC-2 measured
that too. And the six later resets land after frame 22 000, by which time the two machines are
already apart, so nothing there can be held against anything.

**What the six later resets now do** is still worth stating, because it is new behaviour:

* the machine survives all six — the movie runs its 156 312 frames with no stop, and the picture
  keeps moving after each one (frames 22 554, 22 556 and 22 700 around the second reset are three
  different pictures);
* the resets **matter**: with `--resets off` the pictures at frames 22 556, 22 560 and 22 700 all
  differ from the honoured run, so dropping them is not a no-op this far into the movie — unlike
  at frame 400, where EVAL-SYNC-2 found dropping them changed nothing;
* and the game now re-enters its reset vector with the **MMC3 banks it had chosen** still mapped,
  which is what the hardware does and the reason nesdev requires an MMC3 game's reset vector to
  point into `$E000-$FFFF` — the one window the chip cannot move. *Kirby* does, which is why the
  six resets are survivable at all.

---

## Step 5 — the report

`docs/review/MOVIE-SYNC.md` now covers all three lots. What was added, and where:

| | |
|---|---|
| the preamble | the third lot named, with a pointer to its four sections and the warning that it is the one that changes emulation |
| **The short version** | a fifth paragraph: the cause was the instrument and not the engine, *Kirby*'s new figures, and "reason 1 of §6 is restored" |
| **§1c** (new) | the three instruments — `--ram-ref`, `frame-trace`, `--converge` — the Reset semantics of both formats with the measurement behind each, and `--builtin-patches` |
| **§4b** | the "what is left" block gains a **Superseded** note: both candidates eliminated, the `cfg`-gated experiment is not to be written |
| **§4c** (new) | why *Kirby* did not sync: `$004B` is a reading artefact, frame 248's 9 689 free instructions, the overclock, the class (c), what the fix is worth, *Kirby*'s standing, and the measurement that would close the 600-frame window |
| **§4d** (new) | the mapper's soft reset: what changed, the one board that reacts, the tests, golden-core, the amendment, and what it is worth on *Kirby* (nothing, twice over) |
| **§5** | EVAL-SYNC-2's two new bullets struck through and answered; three new ones, each a real bound rather than a to-do |
| **§6** | a new subsection: reason 1 restored, reasons 2-4 untouched, and EVAL-SYNC-2's own prediction about what a third engine would have hidden now **demonstrated**. The to-do list is renumbered: item 0 narrowed to frames 400-1 100, item 1 struck as done, and a **new item 3** warning whoever next measures accuracy that `NesCore` powers on with the 3D patches enabled |
| **§6, commands** | every command of this lot, including the `.fm2` measurement against FCEUX's own movie mode and the six public suites |

The strike-throughs are the document's own convention: a reader who was told something was unknown
is owed the answer in the place they were told it, and the strike-through is how they can tell
which lot answered it.

---

## Step 6 — the final pass

Everything the brief's final check asks for, re-run at the end rather than trusted from the step
that produced it.

### fmt and clippy

| | |
|---|---|
| `cargo fmt --all --check` | clean |
| clippy `-D warnings --all-targets`, **whole workspace**, feature **off** | clean |
| the same, feature **on** (`zax-app/fceux-core,zax-harness/fceux-core,zax-fceux/fceux-core`) | clean |

### The tests, `--include-ignored`

With `ZAXNES_ROMS=G:/3dsen/demo/roms`, `ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms`
and `ZAXNES_PROFILES=G:/3dsen/cleanroom/assets-neutral`:

| | passed | failed | ignored |
|---|---|---|---|
| **`zax-nes`**, feature off | **1 187** | 0 | 0 |
| **`zax-harness`**, feature off | **124** | 0 | 0 |
| **`zax-harness`**, feature on (`--test-threads 1`) | **144** | 0 | 0 |

The two crates this lot touches, and no other. The feature-on run is single-threaded because FCEUX
keeps one machine per process.

Of those, this lot's own: **5** new unit tests (1 in `movie_sync`, 2 in `ram_trace`, 2 in
`frame_trace`), **1** new integration test in `movie_sync_tests` (the `.fm2` reset timing against
FCEUX's own movie mode, `#[ignore]`d and feature-gated) and **3** in `zax-nes`'s
`soft_reset_tests.rs`. EVAL-SYNC-2's figures move accordingly: `zax-harness`'s lib goes from 45 to
50, its feature-off total from 118 to 124 and its feature-on total from 137 to 144;
`soft_reset_tests.rs` goes from 9 to 12.

`rust/test-roms/nestest.nes` was still in place from EVAL-SYNC-2 and did not have to be copied in
again. It is gitignored, four `bench_core_tests` load it, and it is byte-identical to
`assets-free/test-roms/nes-test-roms/other/nestest.nes`.

### golden-core

`zax-harness golden-core "G:/3DSen/test-originals" <out.jsonl> 1200`, feature off, 98 ROMs:

    8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773

**Byte-identical** to the value the brief names, which is lot FCEUX-23's baseline and EVAL-SYNC's
and EVAL-SYNC-2's — and this is the lot that changed `zax-nes`, so the digest is a result rather
than a formality. It is the expected one: `golden-core` drives `NesCore::reset`, the power cycle,
and step 4 changed only `NesCore::soft_reset`. **No change to explain.**

### The two movies, re-run with the final binary

| | |
|---|---|
| *Faxanadu* `[USA][!!]` against `fax-usa.txt` | offset **0**; **207** of 832 checksum lines, **832** of 832 lag counts, last agreeing line **83 200**, last picture `03d7e6a1…` |
| *Faxanadu* `[Rev 1][!!]` against `fax-rev1.txt` | **220** of 832, **832** of 832, last agreeing line **83 200**, last picture `13920624…` |
| both dumps' six screenshots | **12 of 12 MATCH**, with EVAL-SYNC's own colour counts (9, 14, 11, 13, 12, 12) |
| *Kirby* against `kirby-jp-step1.txt`, 400 frames | **400 of 400 frames with no live difference**, 190 identical to the byte, **400 of 400 lag counts** |
| *Kirby* against `kirby-jp.txt`, 156 312 frames | **all 1 563 lag counts agree**, 24 input-insensitive frames — `BizHawk`'s own figure; 0 of 6 screenshots, the nearest 275 pixels of 57 344 |
| the mapper fix | `zax-nes --test soft_reset_tests` — 12 passed; `zax-harness --test soft_reset_mapper_tests` — 3 passed |

### What moved

`git diff --stat 5091ccb6..HEAD` over `zax-engine3d`, `zax-render`, `zax-core-api`, `zax-fceux`
**and `zax-app`** is **empty**. `Cargo.lock` does not move either — no new dependency, and every
cargo command of this pass was run `--offline`.

| | lines |
|---|---|
| `docs/progress/EVAL-SYNC-3.md` | +639 |
| `docs/review/MOVIE-SYNC.md` | +407 |
| `docs/review/AMD-REQ-CORE-122-mapper-reset.md` | +126 |
| `rust/crates/zax-harness/src/frame_trace.rs` | +919 (new) |
| `rust/crates/zax-harness/src/movie_sync.rs` | +376 |
| `rust/crates/zax-harness/src/ram_trace.rs` | +361 |
| `rust/crates/zax-harness/src/{bk2,movie_file,lib,main}.rs` | +83 |
| `rust/crates/zax-harness/tests/{movie_sync,soft_reset_mapper}_tests.rs` | +124 |
| **`rust/crates/zax-nes`, the emulation change** | **+121 in eight source files, +151 in its tests** |

The `zax-nes` half is the whole of step 4 and nothing else: `Mapper::soft_reset` and its four
forwards, `Cartridge::soft_reset`, `Mmc3Unl8237::soft_reset`, one call site in `NesCore::soft_reset`
and two doc comments. Most of the +121 is prose.

### The disk

The pass began with 11 GB free on G: and ended with **9.8 GB**, above the brief's 5 GB stop. The
release tree under `rust/target-ev` is about 8 GB; no debug tree was ever built, every cargo
command having been run `--release`, and no GPU campaign was run.

### What is left for the owner

1. **Finish locating the *Kirby* divergence**, and it is one Lua run: the same script that produced
   `kirby-jp-step1.txt`, over frames **400 to 1 100** with `step = 1`. §4c of the report says what
   to log and what Zaxnes will do with it. Nothing is needed from this side first, and in
   particular **no change to `zax-nes`** — the `cfg`-gated PPU experiment EVAL-SYNC-2 asked for is
   struck, both its candidates having been eliminated.
2. **Adopt or amend REQ-CORE-122.** The code now implements the hardware and the clause still says
   the mapper follows an *after reset* state it has none of.
   `docs/review/AMD-REQ-CORE-122-mapper-reset.md` has the exact wording.
3. **`.bk2` in the application**, as the two earlier lots said, with the reader's third
   EVAL-SYNC-3 addition: a `.bk2`'s `Reset` happens **after** the frame whose bit is set.
4. **A warning worth putting somewhere the next harness author will read it**: `NesCore` powers on
   with the 3D built-in patches **enabled**, so any measurement that loads a recognised commercial
   ROM without turning them off is measuring the widescreen hack. It cost this campaign two lots.
5. The three `.fm2` header fields `zax-app`'s Zaxnes path ignores, unchanged from EVAL-SYNC.
