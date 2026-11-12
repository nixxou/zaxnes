# NESHAWK-PROFILE-12 — item 14 ships, and the hunt resumes at tree frame 38

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `52dfcbbf` (lot NESHAWK-PROFILE-11).

## Steps

- [x] 1. **Ship item 14 at −1** — measured by lot 11, costless, and the clock every later
      measurement of this lot is taken against. First difference of any kind: tree frame **38**.
- [ ] 2. **Tree frame 38**: the residual is +2 with the NMI entry itself at 0 — the shape of a DMA
      cost. Bus stream of frames 36-38 on both machines, execute-to-execute.
- [ ] 3. **Rounds** — next first difference, same loop, up to five rounds, *Faxanadu* re-checked
      after every item.
- [ ] 4. **No regression** and the final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` |
| `CARGO_INCREMENTAL` | `0` |
| disk at the start | `G:` 31 GB free (stop at 5), `C:` 8.3 GB (stop at 1) |

---

## Step 1 — item 14 ships at −1

### The answer first

`NesHawkCompat::SOFT_RESET_PPU_DOT_BIAS` is **−1**, where lot 11 left it at `0` for want of a
source reading. Nothing else changed: the site, the constant, the mechanism and the differential
test are lot 11's, and the test was already parametric, so shipping the item moved its **value**
there and not its logic.

**Why it ships now, when item 11's shape ships off.** The two are not the same shape at all. Item
11 is right in isolation and **expensive** — it costs the movie 7 080 frames and an `AccuracyCoin`
row. Item 14 is right in isolation and **free**, structurally: the site is the soft reset, which
only a movie's RESET column reaches, so every ROM of every suite runs the code it ran before. The
one thing against it is that the reading is open, and that is a reason to keep reading rather than
a reason to run a clock known to be one dot out — which is the operative point for this lot, since
an item hunt whose window opens on such a clock measures that dot and not the item.

### The measurement, re-taken on this tree

`frame-trace --frame 1 --frames 80 --profile neshawk --dmc-census --event-log --event-pc C06E
--event-pc C37C --event-pc 80FC --event-pc 804C` against `bizhawk-runs/kirby-jp-cyc0.txt`, every
event on its **absolute** cycle (`marks.py`):

| | shipped `0` (lot 11) | **shipped `−1` (this lot)** |
|---|---|---|
| frames in step on every event | 3-13 | **3-37** |
| first difference of any kind | tree frame **14**, −6 | tree frame **38**, **+2** |
| frame 38's residual | — | +2 on 11 events of 14, +0 on 3 |
| the first non-zero event | frame 14's `nmi` | frame 38's `pc$C37C`, +2 |
| `item_14_…` differential test | 0 vs 0 | **−1 vs 0**, PPU 7 158 dots apart, pass |

The reading is recorded as still open in the constant's own note, with **`PPU.cs` `Reset()`'s
`idleSynch = true`** named as the best candidate and the reason it does not yet predict the item
written beside it: implemented faithfully its effect **alternates** 0/−1 rather than accumulating,
because both machines then skip once every two frames, so what is missing is whatever makes the
first skip after the reset land on only one of them.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-12 step 1/4 done: item 14 ships at -1 dot and the first difference is tree frame 38
