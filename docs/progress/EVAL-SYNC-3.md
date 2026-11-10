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
- [ ] 3. What the hardware does there.
- [ ] 4. The mapper's soft reset.
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
