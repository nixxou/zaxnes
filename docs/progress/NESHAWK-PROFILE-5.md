# NESHAWK-PROFILE-5 — the seed of the three cycles, and it is in the DMC's power-on timer

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `7dbe7c54` (lot NESHAWK-PROFILE-4).

Lot NESHAWK-PROFILE-4 removed the 4 778-cycle drift with item 10 (the PPU's start-up frame) and
left one thing standing: the **3-cycle phase of the two machines' frame-sync spin loops**, which at
frame 1 100 — a frame with no DMC DMA at all — shows as exactly one event off its anchor,
`pc$C06E` at −3. Its §5 named two unmeasured candidates: the **6502's own reset sequence** and the
**cycle on which the `.bk2`'s `Reset` command is applied**.

**Both are eliminated by measurement in step 1, and the seed is somewhere neither of them is.**

## Steps

- [x] 1. Find the seed: both machines ledgered from the power-on. **Frames 1 to 10 are identical
      event for event and cycle for cycle** — the power-on, the movie's soft reset and the first
      ten frames — and the first differing event is in **frame 11**, the first frame that plays a
      DPCM sample.
- [x] 2. Explain it from the sources — **item 11**, the DMC's power-on timer.
- [ ] 3. Add it to the profile, with a differential test, and re-run *Kirby* on every window.
- [ ] 4. Iterate.
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
| disk on `G:` at the start | 8.8 GB free (the brief's stop is 5 GB) |
| disk on `C:` at the start | 7.6 GB free (the brief's stop is 1 GB) |

## The sources read, and their licences

| | |
|---|---|
| **BizHawk** (NesHawk) | MIT. Read through `raw.githubusercontent.com` at tag **2.11.1**; nothing cloned or downloaded. The files and functions are named per item below |
| **nesdev wiki** | "APU DMC", "CPU power up state", "PPU power up state" |

---

## Step 1 — the ledger from the power-on, and the first event that differs

### The answer first, and it is two negatives and one positive

1. **The 6502's own reset sequence is the same on both machines, to the cycle.** NesHawk spends
   **seven** CPU cycles and begins the first instruction on the eighth; so does this core. Measured
   on both machines at the movie's soft reset, which is the one reset BizHawk's Lua can see.
2. **The cycle the `.bk2`'s `Reset` is applied on is already right.** It is applied at the **start
   of the frame after** the flagged one, before any PPU tick — which is what
   `MovieFile::reset_after_frame` already makes the player do, and what lot EVAL-SYNC-3 measured
   against the lag count.
3. **Frames 1 to 10 are identical, event for event**: every NMI entry, every IRQ entry, every
   watched PC, at residual 0 on both machines. The first event whose cycle differs is in **frame
   11**, and it is the first frame of the whole movie that plays a **DPCM sample**.

### The instrument

`bk2cyc2.lua`, which is `bk2cyc1.lua` (lot NESHAWK-PROFILE-3) with the window moved to frames
**1-1 100** and the watched PCs moved to the boot's:

| | |
|---|---|
| `$FFF0` | the reset vector's own target, read out of the ROM's `$FFFC`: `SEI / LDA #$00 / STA $8000 / JMP $C000` |
| `$C000` | the game's entry |
| `$C06E` | the `STA $39` that ends the main loop's frame — §4h's signature event |
| `$C074` | the `PLA` the frame-sync spin loop falls through to |
| `$C3A1` | the NMI handler's `RTI` |
| `$FFFC` | watched as a **read**, which is the reset sequence's own vector fetch |

The loop's interior (`$C070`, `$C072`) is deliberately **not** watched: it is a 6-cycle loop spun
for thousands of cycles a frame, so it would cost millions of lines, and `$C074`'s cycle against
the frame's own NMI entry says which instruction the `RTI` returned to just as exactly (§4h's own
table is in those terms). The same event set on this side is
`frame-trace --frame 1 --frames 1100 --event-log … --event-pc FFF0 --event-pc C000 --event-pc C06E
--event-pc C074 --event-pc C3A1`, 2.3 s for the 1 100 frames.

**One limit of BizHawk's Lua, said plainly: it cannot see frame 1.** `--lua` is loaded after the
first frame has run, so the ledger's first `F` line is frame 2 and the power-on dead frame's own
events are not in it. That is why the reset sequence below is measured at the movie's **soft**
reset (frame 5), which is in the window — and why the power-on is read from the source and checked
against the one number frame 1 does leave behind, its length.

### The 6502's reset sequence, measured on both machines

Frame 5 is the movie's first reset: the `.bk2` flags frame 4, BizHawk latches it during that frame
and applies it at the start of frame 5, whose PPU is then a start-up frame (item 10).

| | BizHawk | Zaxnes |
|---|---|---|
| the frame's own start | cycle 0 | cycle 0 |
| `r$FFFC`, the reset vector's low byte | **6** | — (this core's ledger carries reads only under `--dmc-census`) |
| `pc$FFF0`, the first instruction | **8** | **0** |
| `pc$C000`, four instructions later | **19** | **11** |
| `pc$FFF0` → `pc$C000` | **11** | **11** |

The two instruments report a read **one cycle later** than an execute, so BizHawk's vector fetch is
on its cycle 5 and its first instruction on its cycle 7 — **a seven-cycle reset sequence whose
sixth cycle reads `$FFFC`**. That is NesHawk's `VOP_RESET` microcode exactly (step 2), and it is
this core's `Cpu6502::reset` exactly: two reads at `PC`, three stack reads, the two vector reads,
then the first opcode fetch on cycle 7.

**The eight cycles of offset are the frame boundary and not the chip**: NesHawk applies the reset
*inside* `FrameAdvance`, so its reset cycles are the first seven CPU cycles of the new frame, while
this core's `NesCoreOf::soft_reset` runs them between frames. The program's own position is
identical — `pc$FFF0` → `pc$C000` is 11 cycles on both — and so is everything downstream of it, to
the end of frame 10.

### The first event that differs: frame 11

`drift.py` over the two ledgers, executes only (the `$40xx` events are in BizHawk's ledger and not
in this side's plain one, so the comparison is held to the tags both carry):

    frames with at least one event off its frame's anchor: 867 of 1099
    the first: frame 11
       frame 11: pc$C3A1@301857 (-2); pc$C074@301871 (-2)

**Frames 1 to 10 are clean on every tag.** And frame 11 is the first frame of the movie with any
DMA at all: its header on this side reads `dmc=57/220 oam=1`, where frames 1 to 10 all read
`dmc=0/0 oam=0`.

### Inside frame 11, interval by interval

Every interval of the frame agrees except two, and they are adjacent:

| interval | BizHawk | Zaxnes | |
|---|---|---|---|
| `nmi` → `oam` | 50 | 51 | the read/write calibration's own 1 |
| `oam` → the first `w$4016` | 880 | 880 | |
| … the four strobe pairs, `w$4017`, `w$4012`, `w$4013`, `w$4010`, both `w$4015` | all equal | | 3 318 cycles of plain execution among them |
| **`w$4015`=1F → `w$4000`** | **3 114** | **3 108** | **Zaxnes 6 cycles short** |
| **`w$4000` → `w$4002`** | **65** | **69** | **Zaxnes 4 cycles long** |
| `w$4002` → `w$4003` → `w$4004` → `w$4006` → `w$4007` → `r$4015` | all equal | | |
| `nmi` → `pc$C3A1` | 8 815 | 8 813 | the −2 the ledger reports |

`$4015 = 1F` is the write that **starts the movie's first DPCM sample** (`$4012 = F3`, `$4013 = 1C`,
`$4010 = 0F` — rate index 15, 54 cycles a bit, 432 a byte, 449 bytes). So the first event whose
cycle differs is inside the first DMC sample of the movie, and the quantity is the DMA's.

### Where the six cycles are, from the fetches themselves

`bk2dmc1.lua` is `bk2cyc2.lua` cut to frames 10-14 with one `event.onmemoryread` per byte of the
sample's first page, `$FCC0`-`$FCFF` (`$4012 = F3` puts the sample at `$C000 + 0xF3 * 64 = $FCC0`).
A fetch **is** a DMA read of that byte, so each firing is one fetch with its own cycle, and nothing
about a cost model has to be assumed. Against this side's `--dmc-census`:

| the sample's byte | BizHawk's fetch | Zaxnes's fetch | |
|---|---|---|---|
| `$FCC0` | 5 106 | 5 106 | the write-started fetch, the same cycle |
| `$FCC1` | **5 452** | **5 663** | **211 cycles apart** |
| `$FCC2` | 5 884 | 6 095 | 211 |
| `$FCC3` … `$FCD2` | 6 316, 6 748, 7 180, … | 6 527, 6 959, 7 391, … | 211, frame after frame |

So the two machines **agree on the first fetch and part on the second**, and then run exactly 432
cycles apart for ever: the steady state is identical and the offset is a one-off of **211 cycles**,
which is within a cycle of **four of the output unit's 54-cycle bit periods**. NesHawk needs the
second byte four bit periods sooner than this core does — its output unit's bit counter is four
clocks further along — and the six cycles the CPU loses are the seven fetches of that stretch
landing on different cycles of the CPU's own work.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-5 step 1 done: the boot is exact, and the first differing event is the DMC's second fetch

---

## Step 2 — explained from the sources: **item 11**, the DMC's power-on timer

### The item

NesHawk's DMC timer **powers on at 1 020 CPU cycles**, not at one period of the rate in force.
`APU.cs`'s `DMCUnit` constructor, read verbatim at tag 2.11.1 (MIT):

```csharp
public DMCUnit(APU apu, bool pal)
{
    …
    timer_reload = DMC_RATE[0];
    timer = 1020;
    sample_buffer_filled = false;
    out_deltacounter = 64;
    out_bits_remaining = 7;
    …
}
```

`Run()` is `if (timer > 0) timer--; if (timer == 0) { timer = timer_reload; Clock(); }` and is
called once per CPU cycle out of `RunOneFirst()`, so **the output unit's first clock is on CPU
cycle 1 020 and every 428th cycle after it**. This core's `DmcChannel::reset` sets
`timer = rates[0] - 1 = 427`, so its first clock is on CPU cycle **428**.

Everything else about the two output units is the same function written two ways, and that is worth
saying because it is what makes the timer the whole of the item:

| | NesHawk | this core |
|---|---|---|
| the bit counter's start | `out_bits_remaining = 7` | `bits_remaining = 8` |
| the counter's step | `if (out_bits_remaining == 0) { out_bits_remaining = 7; … take the buffer } else out_bits_remaining--;` | `bits_remaining -= 1; if bits_remaining == 0 { bits_remaining = 8; … take the buffer }` |
| so the buffer is taken on clock | 8, 16, 24, … | 8, 16, 24, … |

Both are a period of eight with the load on the eighth clock, so the **only** free number is the
timer's own start — the same "NesHawk's timer is this core's plus one" convention the review's
parity trap already records, with `timer = 1020` meaning this core's `1019`.

### The arithmetic, which predicts the measurement to the cycle

`$4010` is written twice before frame 11 (`=00` at frame 5, `=0F` at frame 11 cycle 5 088), so
`timer_reload` is 428 for the whole boot, and both machines' clocks are a plain arithmetic
progression. The `$4010 = 0F` write is at absolute cycle **298 122**.

| | NesHawk | this core |
|---|---|---|
| the clocks | 1 020 + 428 k | 428 k |
| clocks by cycle 298 122 | 1 020 + 428·694 = 298 052, so **695** | 428·696 = 297 888, so **696** |
| the next clock | 298 052 + 428 = **298 480** | 297 888 + 428 = 298 316 |
| 695 and 696 mod 8 | **7**, so the next clock is the eighth and **takes the buffer** | **0**, so eight clocks to go |
| the buffer taken at | **298 480** | 298 316 + 7·54 = **298 694** |
| the fetch that follows, frame-relative | **5 452** | **5 660 ... 5 663** |

and 5 452 is the cycle `bk2dmc1.lua` measured on BizHawk, to the cycle. The 211 is
`298 694 - 298 480 = 214` less the three cycles of the halt this core reports the request on, and
the reason it is four bit periods and not seven is that the `$4010 = 0F` write arrives while both
timers are still counting a 428-cycle period down: NesHawk's has 358 of it left and this core's
193, so NesHawk's own next clock is 164 cycles later and three of the seven clocks of difference
are spent waiting for it.

### What a soft reset does, and it is nothing to this

Confirmed verbatim rather than assumed, because the movie presses RESET after frame 4:

| | |
|---|---|
| `APU.cs` `NESSoftReset()` | clears the sequencer's IRQ, writes `$4015 = 0`, re-applies the last `$4017` value, zeroes `sequencer_counter` and `sequencer_step`, and clears `dmc.fill_glitch` and `dmc.fill_glitch_2`. **It does not touch `dmc.timer`, `out_bits_remaining` or `timer_reload`** |
| `APU.cs` `NESHardReset()` | sets `sequencer_counter = 2` and clears the two fill glitches; the timer's 1 020 comes from the `DMCUnit` constructor, which `HardReset()` reaches through `apu = new APU(this, apu, pal)` |
| this core's `DmcChannel::soft_reset` | the output level `AND` 1, the IRQ flag, the reload countdown. The timer and the bit counter are **left alone** |

So the phase is set once, at the power-on, on both machines — and the measurement agrees: a soft
reset that re-seeded NesHawk's timer at frame 5 would put the second fetch at frame-relative 5 397,
and it is at 5 452.

### Why it cannot simply be changed for every game

This core's 427 is not an accident and the comment on `DmcChannel::reset` says so: the timer's
phase decides whether a reload fetch needs its alignment cycle, and blargg's `sync_dmc` —
`dmc_dma_during_read4` and `sprdma_and_dmc_dma`, thirty ROMs between them — has a
fine-synchronisation loop written to be one cycle longer than the DMC's 3 424-cycle byte so that it
*drifts into* alignment. A phase that makes a reload cost three cycles instead of four makes that
loop exactly 3 424 cycles and it never converges, which hangs every one of those ROMs. Mesen 2 sets
the same phase for the same reason.

**1 020 is even**, so it changes the phase by `1020 - 428 = 592` cycles — also even — and the
alignment parity every one of those ROMs depends on is untouched. That is the one thing that had to
be checked before the item could be an item, and it is why it can be one.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-5 step 2 done: item 11, the DMC's power-on timer, from both sources
