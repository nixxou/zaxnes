# NESHAWK-PROFILE-10 — the wall at 9 353

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `08a881b8` (lot NESHAWK-PROFILE-9).

## Steps

- [x] 1. **Ship the exact configuration**: items 12, 1c and 13 **on**. Every comparison of this lot
      is against it; the baseline it reproduces is lot 9's **9 353**.
- [ ] 2. **The wall at 9 353.** The four marks over frames 26-9 360 on both machines, the first
      frame whose residual leaves −1, then an execute-to-execute bus stream of that frame.
- [ ] 3. **Rounds** — until frame 156 312 or five rounds.
- [ ] 4. **No regression**, report, final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` |
| `CARGO_INCREMENTAL` | `0` |
| disk at the start | `G:` 32 GB free (stop at 5), `C:` 8.8 GB (stop at 1) |

---

## Step 1 — the exact configuration ships

### The answer first

The three items lot NESHAWK-PROFILE-9 measured and shipped off are **on**:
`DMC_HALT_LANDS_ON_A_WRITE_CYCLE` (1c), `RESET_LANDS_ON_THE_FRAME_BOUNDARY_DOT` (12) and
`OAM_DMA_WAITS_AT_THE_END` (13). The choice is the one lot 9 named: each is measured right on
BizHawk's own stream, and the 45 095 the all-off profile reaches is **two errors cancelling**, which
is not a configuration a later difference can be measured against.

`movie-sync --engine zaxnes --profile neshawk --no-lag --frames 10000 --ram-ref kirby-jp-step1c.txt`
reproduces lot 9's figure exactly: **the last frame with no live difference is 9 353**, 237 frames
identical to the byte and 8 192 of 8 901 with no difference outside the stack page.

### What moved

| | |
|---|---|
| `crates/zax-nes/src/timing_profile.rs` | the three constants on `NesHawkCompat`, each note rewritten to say it ships **on** and what it costs |
| `crates/zax-nes/tests/neshawk_profile_tests.rs` | the `ItemsWaiting` test profile is **gone** — it was `NesHawkCompat` plus the three items, so with them on it is `NesHawkCompat`. The three tests measure the shipped profile directly, as item 11's did when it shipped, and the three "the shipped profile does not carry it" assertions are deleted rather than inverted |

`cargo test -p zax-nes --release`: every target passes, `neshawk_profile_tests` 13 of 13 with 1
ignored. Item 1c reads `[4, 5, 4, 5]` against `Standard`'s `[5, 4, 5, 4]`, item 13's four phases sum
to 1 027 on each, and item 12's before-reset frame is short exactly where the boundary dot is an
instruction's first cycle.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-10 step 1/4 done: the exact configuration ships, the baseline is 9 353
