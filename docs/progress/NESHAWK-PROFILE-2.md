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

## Step 2 — frame 4666, and what it is *not*

The headline is a negative, and it is the step's most useful result: **there is nothing unusual in
frame 4666 at all.** Its DMA is fifty-two textbook reloads, and the frame contains no site for
either of the two candidates the brief named. What the frame does say is *where* the item has to be
— and it is not inside it.

### The instrument this needed, because none existed

`frame-trace` could say "52 DMC sample fetch(es)" and no more. Frame 4666 needed *which* of the
fifty-two cost what, so this step built the census the brief asks for:

| | |
|---|---|
| `crates/zax-nes/src/dma_census.rs` | `DmaEvent` / `DmaEventKind` / `DmaCensus`: the request (with the earliest cycle its halt is allowed on), the halt, the fetch, the OAM request, every APU register write, every `$4015` read and every `$2002` read, each with the DMC's own `(timer, bits_remaining, bytes_remaining)` |
| `nes_bus.rs` | a `census` field and a `note_dma` helper, and **nine call sites, every one of them with `#[cfg(feature = "dma-census")]` on the statement itself** |
| `apu/dmc_channel.rs`, `apu/mod.rs` | `census_state()` / `dmc_census_state()`, the three numbers NesHawk's own `fill_glitch` tests |
| `zax-harness` | `frame-trace --dmc-census`, which places each event against the instruction it happened inside and charges each fetch the halt that started it |

**The feature is off by default and the hooks are `#[cfg]`-ed at the statement**, not merely inside
a function, so with it off they are removed before type-checking: the shipped core is the core
without them, which is what keeps step 5's `golden-core` and `bench-core` answerable. Every site is
reached at most once per fetch, once per transfer or once per register access — never per cycle.

    cargo build -p zax-harness --release --features dma-census

Without the feature, `--dmc-census` is a **usage error naming the flag to rebuild with**: an empty
section would read as "this frame has no DMA", which is the opposite of what it would mean.

### Frame 4666's census, in full

52 fetches, **every one a plain reload, every one costing exactly 4 cycles** — `the costs: 52x4`,
208 cycles in all — one OAM transfer at cycle 61, and:

| | |
|---|---|
| `$4015` **writes** | **none** |
| `$4010`, `$4011`, `$4012`, `$4013` writes | **none** |
| `$2002` reads | **none** (EVAL-SYNC-4 measured the same for the frame-926 window) |
| `$4015` reads | **one**, at cycle 7 417, and it is `LDA $4015` at **`$823C`** — EVAL-SYNC-4 §4e's amplifier. It reads `1F`, `AND #$10` gives `10` and the `BNE` at `$8241` is **taken**, which is the arm that does not write |
| other APU writes | `$4017`=C0 at 3 465; `$4004`, `$4006`, `$4007`, `$400C`, `$400E` between 7 024 and 7 405 |
| NMI | handler entered at instruction 1, **SL 241 dot 30** |
| MMC3 IRQ | handler entered at instruction 7 096, **SL 174 dot 288** |
| a sample end, an enable mid-sample, a register write at an alignment | **none of them is in this frame** |

So the two candidates the brief sends step 3 at — item 6's end-of-sample aborts and item 5's
`$4011`/`$4012`/`$4013` alignment — **have no site in frame 4666**. Neither does item 3's
mid-sample enable. Whatever is missing did not happen in this frame.

The one thing the census does show is the class that **is** present: five of the 52 fetches have
their halt land **later than the cycle the request allowed** — four by one cycle and one by more.
Those are the cycles on which the CPU was *writing*, which this core answers by moving the halt to
the next read cycle. They are NesHawk's `+2` sites, and step 3 reads what NesHawk charges for them.

### What frame 4666 is: the frame whose work overran, by about 220 cycles

`$0039` is the frame semaphore and the trace reads it straight off the game:

* the NMI handler does `INC $39` at **`$C37C`** — frame 4666, instruction 599;
* the main loop, when it has finished the frame's work, does `LDA #$00` / `STA $39` at **`$C06E`**
  and then spins at `$C070`/`$C072` (`BEQ $C070`) until the next NMI raises it again.

In frame 4665 that happens at instruction 5 042 and the remaining 4 271 instructions are the spin.
**In frame 4666 `$C06E` is never reached**: `$0039` stays at the 01 the NMI left, and the four live
bytes are what the work was in the middle of when the frame ran out:

| instruction | SL:dot | | |
|---|---|---|---|
| 8 877 | 239:181 | `F06F  STA $38` | `$0038` ← **87** |
| 8 880 | 239:214 | `F075  STA $49` | `$0049` ← **13** |
| 8 892 | 240:14 | `F042  STA $0575` | `$0575` ← 1C |
| 8 895 | 240:41 | `F049  STA $38` | `$0038` ← **86** |
| 8 898 | 240:74 | `F04F  STA $48` | `$0048` ← **12** |
| 8 925 | 240:338 | `B206  CMP $66E6` | the frame ends here |

and the reference's four bytes — `$0038`=87, `$0039`=00, `$0048`=1C, `$0049`=00 — are a state in
which `$C06E` **has** run: the reference finished frame 4666's work and this core did not.
(`--converge` confirms it is not a reading artefact: `$0049`=00 needs instruction 8 880 not to have
run and `$0575`=1C needs 8 892 to have run, so no single instant of our frame holds the reference's
2 KiB.)

**The size of the miss.** The stretch `$F06F` → `$C06E` costs **394** cycles where the frame has
slack (measured on a frame of the same run that does reach it: instructions 3 912 to 4 025,
cycles 138 939 185 to 138 939 579). Frame 4666 reaches `$F06F` at cycle 138 955 673 and its NMI
lands at 138 955 850 — **177 cycles of room for 394 cycles of work, so this core is about 220
cycles short.**

That number is the step's second useful result, and it rules something out: **220 cycles cannot be
bought inside frame 4666.** The frame's entire DMC DMA bill is 208 cycles, so even a chip that
fetched DPCM for free would not close it. The difference must therefore be **earlier than the
frame's own boundary** — which is possible precisely because the live RAM agreeing at the end of
frame 4665 says nothing about the middle of frame 4665.

### Where it has to be: the sample start, and the census says a sample starts every six frames

The census over frames 4 640-4 670 and 4 468-4 480 (one line per frame: fetches, cycles, OAM, how
many halts slipped, and which APU registers were written) shows the game's DPCM as a **177-byte
sample re-triggered every six frames**, and nothing else:

| | |
|---|---|
| the trigger | `$4010` = 0E (rate index 14, 72 cycles a bit, 576 a byte), `$4015` = 0F, then `$4015` = 1F |
| at 4 665 | cycle 2 804 `$4010`, 2 810 `$4015`=0F, **2 816 `$4015`=1F** with a load delay of **2** (`d=2`), halt 2 819, fetch 2 821, **cost 3**, `from_write` — item 3's own path |
| the sample | `$FE80`, 177 bytes; 47 + 52 + 52 + 26 = **177** fetches over frames 4 665-4 668 |
| the one before | started at 4 659, 47 + 52 + 52 + 26 over 4 659-4 662 |
| `$4012`/`$4013` | written only at the *first* trigger of a run (4 659, 4 647, 4 641, 4 479), not at a re-trigger |
| `$4011` | once a frame, on the frames with no sample (4 663, 4 664, 4 669, 4 670, …) |

So **frame 4665 is the frame the sample starts, and it is the last frame with no live difference.**
Every mechanism the brief lists has a site there and not in 4666: the `$4015` disable at 2 810
(item 6's `set_lenctr_en(false)`, with the DMC's timer at **3** and one bit left in the output
unit — which is the neighbourhood of `fill_glitch_2`'s `timer == 3 || timer == 2`), the `$4015`
enable at 2 816 (item 3), and the sample end three frames before at 4 662 (item 6's
`pending_disable`).

The near-miss confirms the shape rather than contradicting it: **frame 4475** carries the same four
bytes with the signs reversed (`$0038`=87/86, `$0048`=22/12, `$0049`=24/13 — there *we* finished and
the reference did not) and it is inside the sample that starts at **4473**, the same
`$4010`/`$4015`/`$4015` trigger. Two frames of the same sample run, opposite signs, one healing and
one not: a phase difference of a sample, not a per-frame cost.

### What step 3 is sent to read, in this order

1. the **`+2`** of `RunCpuOne` — the only class the census finds in frame 4666, and the one half of
   step 1's item 1 that NESHAWK-PROFILE **did not implement** (it has no constant: 1a and 1b
   shipped, the `+2` did not);
2. `set_lenctr_en(false)` and `fill_glitch_2` — there *is* a `$4015` = 0F at frame 4665 cycle 2 810
   with the DMC timer at 3;
3. `set_lenctr_en(true)`'s own delay and what NesHawk's fetch costs after it — frame 4665's
   `d=2`/cost-3 fetch is the one event of the whole window that is not a plain reload;
4. the `$4011`/`$4012`/`$4013` alignment (item 5) — a site at every *first* trigger, and `$4011`
   once a frame;
5. `dmc_realign`, because frame 4666's **first** fetch is requested at cycle 106, inside the OAM
   transfer that started at 61.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-2 step 2/6 done: frame 4666, instruction by instruction

---
