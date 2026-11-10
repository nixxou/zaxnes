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
- [ ] 2. What frame 246 does, on Zaxnes.
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
