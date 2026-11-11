# NESHAWK-PROFILE-2 — the next difference the NesHawk profile is missing, from frame 4666

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `428c50e6` (lot NESHAWK-PROFILE).

Lot NESHAWK-PROFILE closed frame 926 of the *Kirby* `.bk2` with four items read out of NesHawk's
source and put behind a timing profile, and ran out of reference: `kirby-jp-step1b.txt` stops at
frame 1 100. The owner has now produced `kirby-jp-step1c.txt`, frames **1 100 to 10 000** per frame,
and it says the profile parts again at **frame 4666** with frame 926's own signature — the frame
semaphore still set when the NMI lands, Zaxnes late. This lot finds the item behind it.

## Steps

- [ ] 1. Reproduce, and classify the isolated differences between 1 100 and 4 665.
- [ ] 2. Frame 4666, instruction by instruction.
- [ ] 3. The NesHawk source for it.
- [ ] 4. Add it to the profile, and iterate 2-4 up to five rounds.
- [ ] 5. No regression.
- [ ] 6. Report and final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` (Windows path form) |
| `CARGO_INCREMENTAL` | `0` |
| `ZAXNES_ROMS` | `G:/3dsen/demo/roms` |
| `ZAXNES_TEST_ROMS` | `G:/3dsen/cleanroom/assets-free/test-roms` |
| `ZAXNES_PROFILES` | `G:/3dsen/cleanroom/assets-neutral` |
| disk on `G:` at the start | 9.8 GB free (the brief's stop is 5 GB) |
| disk on `C:` at the start | 1.9 GB free (the brief's stop is 1 GB) |

## The sources read, and their licences

| | |
|---|---|
| **BizHawk** (NesHawk) | MIT. `src/BizHawk.Emulation.Cores/Consoles/Nintendo/NES/APU.cs` and `.../NES.Core.cs`, read through `raw.githubusercontent.com`; nothing cloned or downloaded. |
| **Mesen2** | GPL-3.0, for contrast only. |
| **nesdev wiki** | *APU DMC*, *DMA*. |

---

## Step 1 — reproduced, and the 35 isolated frames classified

### The comparison, reproduced to the byte

`movie-sync "$K" "$KR" --engine zaxnes --lag --frames 10001 --trace-step 1 --trace-ref
kirby-jp-step1c.txt --ram-ref kirby-jp-step1c.txt --ram-ref-table c-table.txt`, no `--profile`
(the `.bk2`'s `Core` header is `NesHawk`, so the NesHawk chip is the default and the run prints
`zaxnes/neshawk`):

| | |
|---|---|
| frames compared | **8 901** (1 100 to 10 000) |
| frames with no difference outside the stack page and `$0000-$001F` | **3 531** of 8 901 |
| **the last frame with no live difference** | **4 665** |
| the first live difference | frame 1 205, one byte, `$0488` |
| frames with a live difference below 4 666 | **35**, every one isolated |
| all 8 901 lag counts | **agree** |
| checksum lines agreeing | 0 of 8 901 — and that is the artefact, not a divergence (a checksum covers the stack page and `$0000-$001F`, which is exactly where the reading-instant difference lives; §4f of the review already says so) |

**Frame 4666 is the owner's, to the byte:**

```
4665 total=20 live=0  stack=20 temp=0
4666 total=18 live=4  stack=14 temp=0  $0038=86/87  $0039=01/00  $0048=12/1C  $0049=13/00
4667 total=83 live=46 stack=28 temp=9
4668 total=96 live=38 stack=47 temp=11
4669 total=169 live=134 ...
```

(Zaxnes/BizHawk. `$0038`/`$0039` are the frame semaphore pair of EVAL-SYNC-4 §4e, `$0048`/`$0049`
the frame counters it drives.) From 4667 the divergence is total and never heals: 5 370 of the 8 901
frames carry a live difference, all of them at or after 4 666.

### The 35 isolated frames: 24 are the artefact, 11 are real and transient

`frame-trace --frame <f> --converge kirby-jp-step1c.txt` on each, which answers whether **some**
instant of our frame holds the reference's live memory exactly:

| | frames |
|---|---|
| **artefact** — an instant of our frame matches the reference's 2 KiB live-for-live, so the two machines ran the same frame and wrote it down at different moments | **24**: 1205, 1254, 1277, 1287, 1321, 1360, 1443, 1674, 1678, 1770, 1877, 1919, 2051, 2082, 2893, 3122, 3130, 3246, 3324, 3939, 4448, 4464, 4593, 4657 |
| **real** — no instant of the frame agrees, so the frame genuinely ran differently — **and the next frame is clean again** | **11**: 1752, 1776, 2072, 3355, 3356, 4168, 4338, 4402, 4475, 4576, 4616 |

Of the 24, nineteen match at **−6 to −1** instructions from the boundary, which is EVAL-SYNC-4's
measured 5-to-17-cycle offset again; four are further out (1254 at −58, 2082 at −89, 2893 at −1 486,
4593 at +1) and 2893's −1 486 is the same oddity frame 733 was (§5 of the review).

**The 11 real ones carry the signal this lot needs**, and it is one line: **every one of them has a
DPCM sample playing** — 51 to 69 DMC sample fetches in the frame — where the artefact class includes
frames with **none at all** (1205 has 0 fetches, and 1 OAM transfer like every other frame). So the
missing item is still on the DMC's DMA, which is where steps 2 and 3 look.

Two of the eleven are worth naming:

* **frame 4475** carries `$0038=87/86`, `$0048=22/12`, `$0049=24/13` — **frame 4666's own signature,
  191 frames early, and it heals.** The same mechanism fires there, costs the frame its semaphore,
  and the game catches up; at 4666 it does not. 52 fetches, the same count as 4666's neighbourhood.
* **frame 4576** is the widest, 16 live bytes in one run `$04BF-$04CE`, a buffer filled from a
  different place.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-2 step 1/6 done: reproduce, and classify the isolated differences

---
