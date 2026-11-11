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

## Step 3 — the NesHawk source, and two candidates eliminated by arithmetic

Read through `raw.githubusercontent.com` (BizHawk, **MIT**; nothing cloned, downloaded or copied):
`src/BizHawk.Emulation.Cores/Consoles/Nintendo/NES/APU.cs` — `DMCUnit.Run`, `DMCUnit.Clock`,
`DMCUnit.Fetch`, `DMCUnit.set_lenctr_en`, `APU.RunOneLast` — and `.../NES.Core.cs` — `RunCpuOne`.
Each item below names the function it came from so the reading can be checked.

### `RunCpuOne`'s DMC block, in the order it tests things

```
1. the OAM DMA block      if (oam_dma_exec && apu.dmc_dma_countdown != 1 && !dmc_realign)
                            cpu_deadcounter > 0 ? cpu_deadcounter-- : one OAM get-or-put cycle
                            oam_dma_index == 512 -> oam_dma_exec = false
2. dmc_realign = false
3. the DMC DMA block      if (apu.dmc_dma_countdown > 0)
                            if (countdown == 1) dmc_realign = true
                            if (!cpu.RDY && !cpu.rdy_freeze && countdown == apu.DMC_RDY_check)
                                countdown += 2
                            cpu.RDY = false; dmc_dma_exec = true; countdown--
                            countdown == 0 ? RunDMCFetch() (+ the two glitch tests) : RunDMCHaltFetch()
4. apu.RunOneFirst()      -> ... dmc.Run()
5. cpu.IRQ = _irq_apu || Board.IrqSignal
6. the fill_glitch_2 abort        if (apu.dmc.fill_glitch_2_end)
7. cpu.ExecuteOne()
8. Board.ClockCpu();  9. the sample emit;  10. apu.RunOneLast()
11. if (!cpu.RDY && !dmc_dma_exec && !oam_dma_exec) cpu.RDY = true
```

This confirms NESHAWK-PROFILE step 1's reading of items 1a and 1b to the line, and adds three
things that lot did not have.

### Candidate A — the `+2`, which is item 1's missing half and **has no constant**

NESHAWK-PROFILE step 1 described the `+2` and then split item 1 into **1a** (the halt lands on the
request cycle) and **1b** (a fixed four cycles). `DMC_HALT_ON_REQUEST_CYCLE` and
`DMC_FETCH_HAS_TWO_DUMMIES` shipped; **the `+2` did not** — there is no third constant. The source
says it in two halves:

* `DMCUnit.Run` lowers `nes.cpu.RDY` on the request cycle **whatever the CPU is doing there**, and
  `MOS6502X`'s write microstep executes unconditionally and never touches `rdy_freeze`. So on
  NesHawk a request raised on a **write** cycle halts from that cycle and the write still happens;
* three cycles later, on the cycle `countdown == DMC_RDY_check == 2`, `!cpu.rdy_freeze` says *no
  read microstep has stalled yet*, i.e. the CPU has been writing throughout — and the countdown
  gains **2**, so the fetch lands two cycles later and the whole thing costs **six**.

**Zaxnes has neither half**: `halted_read_cycle` is the only caller of `promote_dmc_wait` on a lone
fetch, so a request raised on a write cycle waits for the **next read cycle** and then costs its
ordinary four. Step 2's census counts that class: in frame 4666 **five** of the 52 fetches slip —
four by one cycle and one by more — and across frames 4 640-4 670 the count runs from 1 to 12 a
frame. Net, per slipped fetch, NesHawk gives the CPU its cycle back **one cycle earlier**, which is
the direction frame 4666 needs.

### Candidate B — item 5, with the exact register list

`APU.RunOneLast`, in order: the **joypad strobe** when `dmc.timer % 2 == 1`, the sequencer IRQ
clear when it is even, the length-clock flags, the **`dmc_reload_countdown`** block (item 2,
already implemented), and then

```
if (pending_reg != -1)
    if (pending_reg is 0x4003, 0x4007, 0x4010, 0x4015 or 0x4017) _WriteReg(...); pending_reg = -1
    else if (dmc.timer % 2 == 0)                                  _WriteReg(...); pending_reg = -1
```

So the five registers above are applied at the end of the write's own cycle and **every other APU
register waits for the first cycle on which `dmc.timer` is even** — `$4000`-`$4002`, `$4004`-`$4006`,
`$4008`, `$400A`-`$400E`, `$4011`, `$4012`, `$4013`. NESHAWK-PROFILE step 1 named `$4011`/`$4012`/
`$4013`; the list is wider than that, and `$4010` (the rate) is on the *immediate* side.

Step 2's census says *Kirby* writes several of the deferred registers every frame (`$4004`,
`$4006`, `$400C`, `$400E`, `$4000`) and `$4011` once a frame. **Only `$4012` and `$4013` can move
the CPU**, and they are written only at the *first* trigger of a sample run (4 641, 4 647, 4 659,
4 479 in the window), never at a re-trigger.

### Candidate C — `dmc_realign`, which lengthens an OAM transfer

`countdown == 1` sets `dmc_realign`, and the OAM block's gate is
`dmc_dma_countdown != 1 && !dmc_realign`. So on the two cycles around the DMC's fetch the OAM
transfer does **not** advance: a DMC fetch that falls inside an OAM transfer makes that transfer
**two cycles longer**. Zaxnes's `run_dma` does the opposite — "the transfer's own cycles then stand
in for its halt and dummy cycles" — so the fetch costs the transfer almost nothing.

Frame 4666 has exactly one site: the OAM transfer starts at cycle 61 and the first DMC request is
at 106, inside it.

### Candidate D — `fill_glitch` and `fill_glitch_2` are **arithmetically impossible on this movie**

This is the step's firmest result, and it closes NESHAWK-PROFILE's item 6 for *Kirby* without an
experiment. The two flags are set in `RunCpuOne`, on the cycle the fetch **completes**:

| set | condition at the completing fetch |
|---|---|
| `fill_glitch` | `apu.dmc.timer == 2 && apu.dmc.out_bits_remaining == 0 && sample_length != 0` |
| `fill_glitch_2` | `timer == 4 && out_bits_remaining == 0 && sample_length == 1` |

and `set_lenctr_en(false)` sets `fill_glitch_2` when
`((timer == 3) \|\| (timer == 2)) && out_bits_remaining == 0 && sample_length != 0`, or
`pending_disable` when `(timer == 1) && out_bits_remaining == 0 && sample_length != 0`, or — third
in the chain — `pending_disable` (and `dmc_irq_glitch`) when a DMA, a `delay` or a
`dmc_reload_countdown` is in flight.

`DMCUnit.Clock` is what decides `out_bits_remaining`: it is reloaded to **7** and the sample buffer
is emptied on the same clock, then counted 7 → 0, so **`out_bits_remaining == 0` is the last bit
period of a byte and the buffer is full throughout it**. The request appears on the clock that
reloads it, when `out_bits_remaining == 7` and `timer == timer_reload`.

*Kirby* writes `$4010` = 0E, so `timer_reload` is **72** CPU cycles and a byte is 576 (step 2
measured exactly 576 between fetches). A fetch completes three cycles after its request, with
`timer == 69` and `out_bits_remaining == 7`. For either flag to be set the fetch would have to
complete with `timer` at 2 or 4 **and** `out_bits_remaining` at 0 — seven bit periods and seventy
cycles away. **Neither flag can ever be set at this rate**, and `set_lenctr_en(false)`'s first two
arms need `out_bits_remaining == 0 && sample_length != 0`, which step 2 measured as
`bits_remaining = 1` (NesHawk's 0) with `bytes_remaining = 0` at the one `$4015` = 0F of the
window — `sample_length` already zero, so the third arm's `else` is taken and it is what this core
does today.

So **item 6 is not frame 4666's item**, and that is proved from the source and the measured rate
rather than guessed. (It stays a real difference for a game with a fast rate — index 15 is 54
cycles a bit — and a candidate for whoever meets one.)

### Candidate E — the IRQ sampling point, and a new one for the controller

`cpu.IRQ = _irq_apu || Board.IrqSignal` is step 5 of `RunCpuOne`: after the DMA block and
`RunOneFirst`, **before** `ExecuteOne`. EVAL-SYNC-4 measured *Kirby*'s MMC3 IRQ entering its
handler at the same dot on both machines and step 2 measured it again (SL 174 dot 288), so this is
not this movie's.

`RunOneLast`'s **first** action is new and is recorded for whoever needs it: a `$4016` strobe is
not applied at the write but on the first cycle with **`dmc.timer % 2 == 1`**
(`nes.joypadStrobed` → `nes.strobe_joyport()`). That is a *controller* item, not a DMA one, and
*Kirby* strobes once a frame many cycles before it reads, so it cannot move this movie; it is
written down because nothing else in this tree records it.

### What step 4 implements, and the arithmetic it is up against

A, B and C, each behind its own associated constant, in that order. And the honest caveat, stated
before the measurement rather than after it: **step 2 measured the miss at about 220 cycles and the
whole of frame 4666's DMA at 721** (208 DMC + 513 OAM), so **no change to the cost of a DMC fetch
can close frame 4666 by arithmetic** — A is worth about one cycle per slipped fetch (five in that
frame), B nothing at all unless `$4012`/`$4013` are written, C about two cycles once a frame.

What the arithmetic does **not** decide is the same thing it did not decide in NESHAWK-PROFILE,
where item 1a closed frame 926 against an arithmetic that pointed the other way: these items move
**which cycle** each fetch lands on, and with it the interleaving of the whole frame. And step 2
found a second reason to measure rather than predict — **the two machines have differed in the
stack page on every one of the 3 566 frames from 1 100 on**, and at frames 4 663-4 665 the
difference is not noise but a **one-address shift** of a block of fifteen bytes (ours
`$01E7`-`$01F5` = `E2 40 85 32 33 E2 F8 00 12 86 83 C3 22 01 00`, the reference's the same bytes
one address higher). A shifted block of dead stack is the fossil of the two machines' stack
pointers having been one byte apart when that routine pushed — a real transient difference, below
the live classification, that predates frame 4666.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-2 step 3/6 done: the NesHawk source for it

---
