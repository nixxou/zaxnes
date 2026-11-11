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
- [x] 3. Add it to the profile, with a differential test, and re-run *Kirby* on every window.
      **Item 11 is in, it is exact — every DMC fetch of frame 11 now lands on NesHawk's own
      cycle, where the baseline was 208 out — and it makes the movie *worse*: 400-1 100 falls
      from 1 100 to 925.** So the baseline's wrong power-on phase was compensating for something,
      and step 4 names it. A stale test of lot NESHAWK-PROFILE-4's was also found failing at
      `63f937ea` and is fixed here.
- [x] 4. Iterate. **One round.** It isolates the remaining cycle of frame 11 to the odd-slip
      reload fetch and finds the source and the measurement in **conflict**, which is Q-182; item
      1c is left as it is and nothing is changed.
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

---

## Step 3 — item 11 in the profile, and the measurement that says it is not alone

### The answer first

**Item 11 is exact, and it costs the movie.** Both halves are measurements, not readings:

1. With item 11 the **whole of frame 11's DMC fetch train lands on NesHawk's own cycle.** Against
   `bk2dmc1.lua`'s per-byte fetch ledger, every fetch from `$FCC1` to `$FCCD` sits at a **constant
   3 cycles** from BizHawk's — the two instruments' calibration for this event kind, the same 3 on
   all thirteen — where the baseline was **+211 and climbing out of step with the write**. The
   write-started fetch of `$FCC0` is at 5 106 on both, exactly as before.
2. And the movie is **worse**: the 400-1 100 window's last frame with no live difference falls from
   **1 100 to 925**, and the 1-1 100 cycle ledger goes from **867** frames with an event off its
   anchor to **920**.

That pair is only a contradiction if the DMC's power-on phase were the only thing wrong, and it is
not: **the baseline's wrong phase was cancelling an error somewhere else**, and step 4 round 1
names it and retracts it.

### The A/B, measured with one binary

The baseline column is this lot's own tree with `NesHawkCompat::DMC_POWER_ON_TIMER` set back to
`0`, built and run exactly like the item column — not lot NESHAWK-PROFILE-4's reported figures.
That matters: the two differ, and the committed baseline is what an item is owed a comparison
against.

| *Kirby*, `--profile neshawk` | baseline (`timer = 427`) | **item 11** (`1019`) |
|---|---|---|
| 1-400, last frame with no live difference | 400 | **400** |
| 1-400, frames identical to the byte | 281 | **194** |
| 400-1 100, last frame with no live difference | **1 100** | **925** |
| 400-1 100, frames identical to the byte | 388 | **0** |
| all lag counts, both windows | agree | **agree** |

| the 1-1 100 cycle ledger | baseline | **item 11** |
|---|---|---|
| frames with an event off the frame's anchor | **867** of 1 099 | 920 of 1 099 |
| `nmi` at residual 0 | 1 094 of 1 094 | 1 054 of 1 054 |
| `pc$C3A1` at 0 | **514** of 1 094 | 499 of 1 054 |
| `pc$C074` at 0 | 277 of 988 | 233 of 962 |
| `pc$C06E` at 0 | 268 of 988 | 209 of 962 |
| the first frame with a residual | 11, at **−2** | 11, at **−1** |

Two readings of that table are worth separating, because they point different ways:

* **frame 11's own residual halves**, −2 to −1, which is item 11 doing exactly what step 2 said it
  would and stopping one cycle short of the frame being exact;
* **`identical to the byte` collapses to 0** on the 400-1 100 window while `no live difference`
  still reaches 925. Those two metrics differ by the **stack page**, so what item 11 moves on every
  frame of that window is the *pushed PC* — the spin loop's phase — and not the game's live state.
  That is the signature §4h gave the whole problem, and it says the remaining cycle is carried
  frame to frame rather than re-made each frame.

### Where the last cycle of frame 11 is

Under `drift.py`'s calibration frame 11's events sort into: every `$40xx` write at **0**, the
`r$4015` at **−1**, `pc$C3A1` (the NMI handler's `RTI`) at **−1**, `pc$C074` at **−1**. So the
cycle is lost **between the last write and the `$4015` read**, and the only DMA in that stretch is
the fetch of `$FCC8`:

    cyc    kind   instr  SL:DOT     addr val  ready cost  dmc(timer,bits,bytes)
    8041  fetch    2321  49:246   $FCC7 68      -    4    (50,8,442)
    8473  fetch    2453  53:181   $FCC8 73      -    3    (50,8,441)    <- three, not four
    8905  fetch    2592  57:116   $FCC9 00      -    4    (50,8,440)

`$FCC8` is the one fetch of the frame that costs **three** cycles rather than four, and the
constant that spends it is **item 1c**, `DMC_HALT_LANDS_ON_A_WRITE_CYCLE`. Step 4 reads NesHawk's
own source on that and finds item 1c has the **sign wrong**.

### The item, as committed

`TimingProfile::DMC_POWER_ON_TIMER`, read at `apu/dmc_channel.rs` `reset` and reached from
`nes_core.rs` `power_on` alone — one site, which runs once a machine.

| | |
|---|---|
| `Standard` | `0`, meaning "one period of the rate in force", which is the line `DmcChannel::reset` had before the profile existed — the same source, not merely an equivalent |
| `NesHawkCompat` | `1019`, which is NesHawk's `timer = 1020` in this core's convention |

**The convention, checked against both bodies rather than assumed.** NesHawk's `Run()` is
`if (timer > 0) timer--; if (timer == 0) { timer = timer_reload; Clock(); }`, so a timer of `N`
clocks first on the `N`th call. This core's `clock_timer` is `if self.timer > 0 { self.timer -= 1;
return false }` and then plays the bit, so a timer of `T` clocks first on call `T + 1`. Hence
`T = N - 1 = 1019`, and the differential test measures that rather than restating it: the second
fetch — the first one the output unit asks for, as against the one the `$4015` write starts — comes
out at **3 424** on `Standard` and **4 016** on `NesHawkCompat`, which is `T + 1 + 7 × 428` on each,
**592** apart, and even.

### A stale test of lot NESHAWK-PROFILE-4's, found failing at `63f937ea`

`timing_profile_tests.rs`'s `a_rom_that_plays_no_dpcm_runs_identically_on_both_profiles` **fails at
the commit this lot started from**, and not because of anything in this lot: `595615` against
`593227`, a gap of **2 388** cycles, which is item 10's 7 165 PPU dots divided by three and nothing
else. It was checked by reverting this lot's five files and running the target alone.

Its premise — *"every item the NesHawk profile carries is about the DMC's sample-fetch DMA, so a
ROM that never starts a DPCM sample must run identically on both profiles for ever"* — stopped
being true when lot NESHAWK-PROFILE-4 added a **PPU** item, and that lot's reported "1 142 passed,
0 failed" did not catch it.

Fixed here in the stronger form rather than deleted: the test now asserts the gap is **exactly**
`PPU_START_UP_DOTS_SKIPPED / 3` and that `Standard`'s own constant is 0, so it still fails the
moment a DMC item reaches the common path, and it fails too if item 10 ever stops being the only
thing a silent ROM can see.

### The suite

`cargo test --release -p zax-nes`: **1 194 passed, 0 failed, 11 ignored** — against 1 132 passed
and **1 failed** at `63f937ea` (the failing target aborts the run, which is why the passing count
moves by more than one). `cargo fmt --all --check` clean; `cargo clippy --release -p zax-nes
--all-targets -- -D warnings` clean.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-5 step 3 done: item 11 is exact, it costs the movie, and item 1c is why

---

## Step 4 — one round, and the source and the measurement disagree (**Q-182**)

### The answer first

The round did its job: it isolated the remaining cycle of frame 11 to **one fetch**, read the
source on it — and the source says this core is **already right**. So the round ends in a
documented conflict rather than an item, and **nothing is changed on either profile**.

### The ledger, and the first event that differs

With item 11 in, frame 11's events against BizHawk's own (`drift.py`, census ledger against
`kirby-jp-dmc1.txt` with the fetch rows dropped so the sequences match):

| | residual |
|---|---|
| `nmi`, `oam`, every `$40xx` write to and including `w$4007` | **0** |
| `r$4015` | **−1** |
| `pc$C3A1`, the NMI handler's `RTI` | **−1** |
| `pc$C074` | **−1** |

Every write of the frame is exact and the cycle goes missing between the last write and the
`$4015` read. There is exactly one DMA in that stretch:

    8470  request  $FCC8  ready 8470      <- the CPU is writing $4007 on this very cycle
    8470  write    $4007
    8471  halt     $FCC8  ready 8470      <- slip 1: an ODD run of write cycles
    8473  fetch    $FCC8  cost 3          <- item 1c's three cycles rather than four

### The measurement, which is unambiguous

Charging that fetch **four** cycles — `DMC_HALT_LANDS_ON_A_WRITE_CYCLE = false`, item 1c dropped,
item 11 kept — was built and run:

| | baseline | item 11 | **item 11, item 1c dropped** |
|---|---|---|---|
| *Kirby* 1-400, last frame with no live difference | 400 | 400 | **400** |
| *Kirby* 400-1 100, the same | **1 100** | 925 | **1 100** |
| 1-1 100, frames with an event off the anchor | 867 | 920 | **861** |
| `pc$C3A1` at residual 0 | 514 | 499 | **580** of 1 094 |
| `pc$C074` at 0 | 277 | 233 | **307** |
| the first frame with a residual | 11 | 11 | **13** — frame 11 is exact on every tag |
| the thirty DMC/DMA/APU ROMs, `--profile neshawk` | 19/5/6 | 19/5/6 | **19/5/6**, ROM for ROM |

So it is better on five counts and costs the ROM suite nothing.

### And the source says three is right

Read verbatim at tag 2.11.1 (MIT) for this round, which is the reason the measurement was not
simply taken:

| | |
|---|---|
| `NES.Core.cs` `RunCpuOne` | runs the DMC countdown block, **then** `apu.RunOneFirst()`, **then** `cpu.ExecuteOne()` |
| `APU.cs` `DMCUnit.Run()` | on the request cycle `A` lowers `RDY` and sets `dmc_dma_countdown = 3`, `DMC_RDY_check = 2`. Its own comment: *"VisualNES and test roms verify that DMC DMA is 1 cycle shorter for calls from writes / however, if the third cycle lands on a write, there is a 2 cycle delay even if the instruction only has a single write / therefore, the RDY_check is 2 in both cases"* |
| `NES.Core.cs` | `if (!cpu.RDY && !cpu.rdy_freeze && apu.dmc_dma_countdown == apu.DMC_RDY_check) apu.dmc_dma_countdown += 2;` — so the `+2` can first fire on `A+2` |
| `MOS 6502X/Execute.cs` | a **read** microstep is `rdy_freeze = !RDY; if (RDY) { … }`; a **write** microstep executes unconditionally and **never assigns `rdy_freeze`** |

For the slip of 1 this frame actually has: the write on `A` leaves `rdy_freeze` false, the read on
`A+1` sets it **true**, the `+2` is therefore suppressed on `A+2`, the fetch is on `A+3` and the
CPU's stalled read on **`A+4`**. This core halts on the first read cycle `A+1` and spends three, so
its read is on `A+4` as well. **The two machines agree, and item 1c's odd-slip cycle is what makes
them agree.** The same derivation gives `A+4` for a slip of 0, `A+6` for 2 and `A+6` for 3, and
this core's halt-at-`A+w`-plus-four reproduces all four.

The item's "wrong sign" reading of NesHawk's comment — that the `+2` only ever *lengthens* a fetch,
so a three-cycle one cannot be NesHawk's — was the round's first conclusion and it is **wrong**:
the `+2` is how NesHawk pays for having let the writes through from `A`, and this core pays for the
same thing by starting four cycles later at `A+w`. The two bookkeepings meet.

### What is not pinned, and the provisional choice

**The cycle BizHawk's Lua reports a DMA read on.** Lot NESHAWK-PROFILE-3's calibration — an
execute at 0, a read or a write at 1 — was measured on **CPU** accesses (`r$4015 -> w$4011` at
exactly 0 on 75 of 75) and does not transfer to a read performed by a DMA unit. With item 11 the
thirteen reload fetches of frame 11 sit at a **constant 3** from BizHawk's reported cycles and the
one write-started fetch at **0**; whether that 3 is bookkeeping or three real cycles is what decides
whether the missing cycle is at this fetch at all.

**Q-182** records both readings, the measurement, the derivation and the one Lua run that would
settle it (the fetch's own callback cycle *and* the cycle the stalled read executes on, for one
known reload — the gap is 1 under one reading and 2 under the other). Referenced in code at
`nes_bus.rs` `promote_dmc_wait` and at `timing_profile.rs`'s `DMC_HALT_LANDS_ON_A_WRITE_CYCLE`.

**Item 1c stays exactly as it is.** A measurement that improves one movie and three ledger counts
does not overturn a derivation that is verbatim from three files of both sources, and the lot will
not ship a behaviour change it cannot explain.

### Why the rounds stop at one

The round loop is *ledger → first differing event → source → item*, and this round completed every
step but the last: the event is isolated to one fetch, the source is read, and the source refuses
the item. A second round would re-measure the same cycle with the same unpinned calibration. What
the next lot needs is not another ledger but the **one Lua run in Q-182**, which is three lines of
script and no new reference.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-5 step 4 done: one round, the cycle isolated to one fetch, and Q-182
