# NESHAWK-PROFILE — two timing profiles for the 2A03, and the NesHawk-compatible one

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `b604dc83` (lot EVAL-SYNC-4).

EVAL-SYNC-4 left frame 926 of the *Kirby* `.bk2` **undetermined**: Zaxnes's CPU runs about 45 to 55
cycles behind NesHawk's inside that frame, during a 257-byte DPCM sample, and the only mechanism in
the frame of the right size is the DMC's own sample-fetch DMA. It could not be closed from that
side because *"`NesHawk`'s standing on it: **not published**"* — no figure for its halt cost, its
alignment or its abort behaviour was available, and its source was out of scope for that lot.

This lot is allowed to read it. It builds the architecture for **two variants of the console chip**
— Zaxnes's own 2A03 and a NesHawk-compatible one — and fills the second with the differences the
source actually shows.

## Steps

- [x] 1. Verify the differences verbatim.
- [ ] 2. The architecture.
- [ ] 3. The NesHawk profile.
- [ ] 4. The engine variant and its selection.
- [ ] 5. The experiment.
- [ ] 6. Report and final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` (**Windows** path form — EVAL-SYNC-4 §6) |
| `CARGO_INCREMENTAL` | `0` |
| `ZAXNES_ROMS` | `G:/3dsen/demo/roms` |
| `ZAXNES_TEST_ROMS` | `G:/3dsen/cleanroom/assets-free/test-roms` |
| `ZAXNES_PROFILES` | `G:/3dsen/cleanroom/assets-neutral` |
| disk on `G:` at the start | 9.73 GB free (the brief's stop is 5 GB) |

## The sources read, and their licences

| | |
|---|---|
| **BizHawk** (NesHawk) | MIT. `src/BizHawk.Emulation.Cores/Consoles/Nintendo/NES/APU.cs`, `.../NES.Core.cs`, `src/BizHawk.Emulation.Cores/CPUs/MOS 6502X/MOS6502X.cs` and `Execute.cs`. Read through `raw.githubusercontent.com`; nothing cloned or downloaded. |
| **Mesen2** | GPL-3.0. `Core/NES/NesCpu.cpp`, for contrast only. Zaxnes's DMA already carries `// Portions adapted from Mesen 2 (c) Sour, GPL-3.0` at `nes_bus.rs:680`, `nes_bus.rs:793`, `nes_bus.rs:1316` and `apu/dmc_channel.rs:217,334`. |
| **nesdev wiki** | *DMA*, *APU DMC*, *Cycle reference chart*, as EVAL-SYNC-4 §4 quotes them. |

Nothing taken from NesHawk is copied into this tree: the profile is written from the *behaviour*
the source describes, and each item below names the file and the function it was read from so the
reading can be checked. Code comments carry the same references.

---

## Step 1 — the differences, verified

The brief's summary was built from a second-hand reading of the sources. **Three of its four items
are confirmed, one of them with its mechanism corrected, and the reading turned up two more
differences the summary does not mention.** The one correction matters, and so does one arithmetic
consequence that points the opposite way from frame 926 — both are recorded below rather than
smoothed over.

### How NesHawk's cycle is laid out, which every item below hangs off

`NES.Core.cs`, `RunCpuOne()`, read verbatim. One call is one CPU cycle, and its order is:

```
1.  the OAM DMA block        if (oam_dma_exec && apu.dmc_dma_countdown != 1 && !dmc_realign)
2.  dmc_realign = false
3.  the DMC DMA block        if (apu.dmc_dma_countdown > 0)
4.  apu.RunOneFirst()        -> pulse[0..1].Run(), triangle.Run(), noise.Run(), dmc.Run()
5.  cpu.IRQ = _irq_apu || Board.IrqSignal
6.  the fill_glitch_2 abort
7.  cpu.ExecuteOne()
8.  Board.ClockCpu()
9.  the sample emit
10. apu.RunOneLast()
11. if (!cpu.RDY && !dmc_dma_exec && !oam_dma_exec) cpu.RDY = true
```

Two things follow and are used throughout: **`dmc.Run()` runs before `cpu.ExecuteOne()`**, so
anything it does to `RDY` halts the CPU on that same cycle; and **`RunOneLast()` runs after it**,
so anything deferred to there lands at the end of the cycle.

### Item 1 — the halt lands on the request cycle itself, and there is no alignment cycle

**Confirmed, and it is the biggest of the four.** `APU.cs`, `DMCUnit.Run()`, whose control flow is:

```
if (!sample_buffer_filled && ((sample_length > 0) || fill_glitch_2)
    && (apu.dmc_dma_countdown == -1) && (delay == 0))
    if (!fill_glitch)
        if (!apu.call_from_write)
            nes.cpu.RDY = false;              <-- the halt, on the request cycle
            nes.dmc_dma_exec = true;
            apu.dmc_dma_countdown = 3;
            apu.DMC_RDY_check = 2;
        else
            if (timer % 2 == 0) delay = 3; else delay = 2;
    else
        sample_buffer_filled = true; fill_glitch = false;
if (delay != 0)
    delay--;
    if (delay == 0)
        apu.dmc_dma_countdown = 3; apu.DMC_RDY_check = 2; apu.call_from_write = false;
```

`RDY` is lowered inside `Run()`, which `RunOneFirst()` calls at step 4 — **before**
`cpu.ExecuteOne()` at step 7. So the cycle the memory reader runs dry on is already a halted cycle,
and the countdown of 3 adds three more:

| cycle | `dmc_dma_countdown` | what the DMC DMA block does | the CPU |
|---|---|---|---|
| the request cycle | `-1` → `3` (in `Run()`) | not entered (`-1`, not `> 0`) | **halted** (`RDY` low before `ExecuteOne`) |
| +1 | `3` → `2` | `RunDMCHaltFetch()` | **halted** |
| +2 | `2` → `1` | `RunDMCHaltFetch()`, and the `+2` test | **halted** |
| +3 | `1` → `0`, then `-1` | `RunDMCFetch()` → `dmc.Fetch()` | **halted** |
| +4 | `-1` | — | runs (step 11 raised `RDY` at the end of +3) |

**Four halted cycles, fixed.** `RunDMCHaltFetch()` is `nes.ReadMemory(nes.cpu.address_bus)` — the
re-read of the CPU's own address, which is Zaxnes's `dma_repeat_read`.

**And there is no get/put parity anywhere in it.** The only `% 2` in the whole DMC DMA path is
`timer % 2` in the `call_from_write` branch (item 3) and `dmc.timer % 2` in `RunOneLast`'s pending
register dispatch (item 5) — neither is a get/put cycle test, and neither is on the reload path.
NesHawk's reload DMA costs four cycles whatever cycle it starts on.

**Zaxnes, for contrast** (`nes_bus.rs`): `poll_dmc_dma` raises the request inside the cycle's
`tick`, `hold_dmc_dma(0)` holds it so that the earliest halt is the **next** cycle, the halt may
land only on a cycle the CPU **reads** on (`read_cycle` → `halted_read_cycle`), and
`dmc_fetch_cycles` then spends the dummy cycle, **an alignment cycle when `(cycles & 1) == 0`**,
and the fetch — `DMC_DMA_CYCLES_ALIGNED = 3` or `DMC_DMA_CYCLES = 4`.

**Mesen2 agrees with Zaxnes**, as the brief said: `NesCpu.cpp` computes
`bool getCycle = (_state.CycleCount & 0x01) == 0;` and gates the halt and dummy cycles on it.

The `+2`: `RunCpuOne` step 3, with NesHawk's own comment,

```
// By this point the cpu should be frozen, if it is not, then we are in a multi-write opcode,
// add another cycle delay
if (!cpu.RDY && !cpu.rdy_freeze && (apu.dmc_dma_countdown == apu.DMC_RDY_check))
    apu.dmc_dma_countdown += 2;
```

`DMC_RDY_check` is `2` on both paths that set it, so the test fires on cycle +2 of the table above.
`MOS6502X`'s `Execute.cs` is where the other half is: a read microstep does
`rdy_freeze = !RDY; if (RDY) { … }`, while a **write** microstep executes unconditionally and never
touches `rdy_freeze`. So `!cpu.rdy_freeze` on cycle +2 means *the CPU was writing and ignored the
halt*, and the two extra cycles give the writes room — NesHawk's rendering of nesdev's *"If the CPU
is writing, it ignores the halt and the DMA unit waits until the next cycle to try again"*. Zaxnes
renders the same sentence by never calling the DMA path from a write cycle at all.

#### The arithmetic that points the wrong way, and is recorded rather than hidden

A lone reload fetch costs **NesHawk a fixed 4** cycles and **Zaxnes 3 or 4**. So on every fetch
where Zaxnes is aligned and spends 3, NesHawk spends 4 and falls **one cycle behind** Zaxnes —
which is the *opposite* sign from frame 926, where Zaxnes is 45 to 55 cycles **behind**. Item 1
cannot by itself be frame 926's cause, and implementing it alone may well move *Kirby* the wrong
way.

Three things keep it on the list even so, and step 5 is the arbiter:

1. the per-fetch cost is not the only thing that moves. Item 1 also moves **where** the fetch lands
   — the request cycle itself rather than the next read cycle — which moves the DMC's phase and so
   which cycle every later fetch of the sample lands on. That is not a sum over fetches;
2. the `+2` fires on an unknown fraction of fetches and costs NesHawk **6**, which is the other
   direction again;
3. items 2 and 3 move the *end* of a sample and the *start* of a load, and item 2 feeds the
   `$4015` bit-4 branch at `$8241` that EVAL-SYNC-4 §3 named as the thing that turns a few cycles
   of slack into two different games.

**No item below is adopted because it makes *Kirby* sync.** Each is adopted because NesHawk's
source does it; step 5 measures what the set of them does, and if *Kirby* moves the wrong way that
is the measurement and it will be reported as one.

### Item 2 — `sample_length`, the loop and the IRQ land three cycles after the fetch

**Confirmed, and it is the item that reaches the `$4015` branch.** `DMCUnit.Fetch()` does *not*
decrement the length:

```
if (sample_length != 0)
    nes.dmc_dma_controller_conflict = true
    sample_buffer = apu.nes.ReadMemory((ushort) sample_address)
    nes.dmc_dma_controller_conflict = false
    sample_buffer_filled = true
    …the $4015/$4016 bus-conflict cases…
    sample_address = (ushort)(sample_address + 1)
    if (sample_address == 0) sample_address = 0x8000
    apu.dmc_reload_countdown = 3
```

and `RunOneLast()` — step 10, the end of the cycle — spends it:

```
if (dmc_reload_countdown > 0)
    dmc_reload_countdown--
    if (dmc_reload_countdown == 0)
        dmc.sample_length--
        if (dmc.sample_length == 0)
            if (dmc.loop_flag)
                dmc.sample_address = dmc.user_address; dmc.sample_length = dmc.user_length
            else if (dmc.irq_enabled && !dmc_irq_glitch)
                dmc_irq = true
        if (dmc.pending_disable)
            dmc.sample_length = 0; dmc.pending_disable = false; dmc_irq_glitch = false
```

And bit 4 of `$4015` is `dmc.IsLenCntNonZero()`, which is `return sample_length != 0;`.

So on NesHawk the **three CPU cycles after the last fetch of a sample** still read bit 4 **set**,
the DMC IRQ is raised three cycles late, and a looping sample restarts three cycles late. Zaxnes
does all of it at the fetch, in `DmcChannel::supply_byte` (`dmc_channel.rs:292`), and reads bit 4
off `active()` = `bytes_remaining > 0`. **Mesen2 agrees with Zaxnes**: the fetch calls
`SetDmcReadBuffer(readValue)` and the length is handled inside it.

This is a three-cycle window, once per sample, in which the two machines disagree about the one bit
`Kirby` reads at `$823C` and branches on at `$8241`.

### Item 3 — `call_from_write` is set whenever the buffer is empty, **mid-sample included**

**Confirmed, and the brief's wording is right where I expected it to be wrong.** `set_lenctr_en`:

```
else                                    // en == true
    if (sample_length == 0)
        sample_address = user_address
        sample_length  = user_length
    if (!sample_buffer_filled)          <-- OUTSIDE the length test
        apu.call_from_write = true
```

The `!sample_buffer_filled` test is a sibling of the restart, not a child of it. So a `$4015` write
with bit 4 set while a sample is **already playing** and the buffer happens to be empty sets
`call_from_write` — and `Run()` then takes its `else` branch for the next fetch:

* the CPU is **not** halted on the request cycle (`RDY = false` lives only in the
  `!call_from_write` arm);
* `delay` is `3` when `timer % 2 == 0` and `2` otherwise — two or three cycles in which the CPU
  keeps running;
* when `delay` reaches 0, `dmc_dma_countdown = 3` and `DMC_RDY_check = 2` are set from `Run()`,
  again without lowering `RDY`. The halting therefore starts on the *next* cycle, through
  `RunCpuOne`'s block, and runs `3 → 2 → 1 → 0`: **three halted cycles, not four**.

So a `call_from_write` fetch is *displaced* two or three cycles later and costs **three** cycles
instead of four. Zaxnes's `DmcChannel::set_enabled` (`dmc_channel.rs:218`) returns a delay of 2 or 3
**only when `bytes_remaining == 0`** and returns `0` — the plain four-cycle reload — when a sample
is already playing.

For *Kirby* this may fire rarely: EVAL-SYNC-4 §2 found no `$4015` write at all in eight of its nine
isolated frames, and the frame-926 branch at `$8241` is *taken*, which is the arm that does **not**
write. The item is implemented because the source does it, not because it is expected to move this
movie.

### Item 4 — the power-on work RAM and the Reset placement

Already handled, in the `.bk2` player rather than in the core (EVAL-SYNC-2/-3): the
`00 00 00 00 FF FF FF FF` pattern, the refusal of a non-`null` `InitialWRamStatePattern`, and a
`.bk2`'s `Reset` applied **after** the flagged frame. Nothing to do here, and nothing that belongs
in a timing profile.

### Item 5 (new) — `$4011`, `$4012` and `$4013` writes are applied on an even DMC-timer cycle

**Not in the brief.** `APU.cs` routes every APU register write through `pending_reg`/`pending_val`,
and `RunOneLast()` — the end of the cycle — dispatches it:

```
if (pending_reg != -1)
    if (pending_reg == 0x4003 || pending_reg == 0x4007 || pending_reg == 0x4010
        || pending_reg == 0x4015 || pending_reg == 0x4017)
        _WriteReg(pending_reg, pending_val); pending_reg = -1
    else if (dmc.timer % 2 == 0)
        _WriteReg(pending_reg, pending_val)
```

So `$4003`, `$4007`, `$4010`, `$4015` and `$4017` land at the end of the write's own cycle, and
**every other APU register — `$4011`, `$4012`, `$4013` among them — waits for the next cycle on
which `dmc.timer` is even**, which is nought or one cycle later. Zaxnes applies every APU write at
the write (`apu/mod.rs:986` `Apu2A03::write`), after the cycle has been stepped.

*Kirby* writes `$4011` once a frame and `$4012`/`$4013` once at the start of a sample
(EVAL-SYNC-4 §4), so this is worth at most a cycle of sample-address phase per sample. **A
candidate, not implemented in step 3** — it is an APU-wide register path, not a DMC one, and
putting it behind the profile would touch `Apu2A03::write` for every game. Listed for the lot that
needs it.

### Item 6 (new) — the `fill_glitch` / `fill_glitch_2` end-of-sample aborts

**Not in the brief, and this is NesHawk's answer to the gap MF-4a §Step 3 diagnosed and withdrew.**
Three mechanisms, all in `APU.cs` and `RunCpuOne`:

| | |
|---|---|
| `fill_glitch` | Set in `RunCpuOne` when, at the moment the fetch completes, `apu.dmc.timer == 2 && out_bits_remaining == 0 && sample_length != 0`. `Run()` then **fills the buffer without a DMA at all** (`sample_buffer_filled = true; fill_glitch = false;`) — no halt, no cycles. |
| `fill_glitch_2` | Set when `timer == 4 && out_bits_remaining == 0 && sample_length == 1`, and by `set_lenctr_en(false)` when `(timer == 3 \|\| timer == 2) && out_bits_remaining == 0 && sample_length != 0`. It runs a DMA with `dmc_dma_countdown = -2` and `DMC_RDY_check = -2`, and `RunCpuOne` step 6 aborts it after a single cycle: *"DMC was started in the APU, but in this case it only lasts 1 cycle and is then aborted"*. |
| `pending_disable` | `set_lenctr_en(false)` while a DMA, a `delay` or a `dmc_reload_countdown` is in flight does **not** stop the sample: it sets `pending_disable`, and `RunOneLast` applies `sample_length = 0` when the reload countdown lands. With a reload countdown in flight it also sets `dmc_irq_glitch`, which suppresses the IRQ. |

That last row is, behaviour for behaviour, the **post-halt DMA abort** EVAL-SYNC-4 §4 records as
Zaxnes's one named gap on this mechanism (`AccuracyCoin`'s `Explicit DMA Abort` and `Implicit DMA
Abort`, `Q-119`, a fix built and withdrawn in MF-4a step 3). NesHawk implements it; Zaxnes does not.
`fill_glitch_2`'s single-cycle DMA is the *explicit* abort and `fill_glitch` is a one-byte fill with
no DMA at all.

**Candidate, not implemented in step 3.** It is worth one to three cycles per sample end — MF-4a's
own sizing — the sample in frame 926 does not loop, and it would need `DmcChannel` to grow three
flags and the bus to grow an abort check inside its DMA loop. It is the **first thing to add in
step 5's iterations** if the three implemented items do not close frame 926, and
`docs/review/NESHAWK-PROFILE.md` (step 6) will say how.

### The other candidates the brief asked me to look for

Looked for in NesHawk only, and none is implemented:

| | what NesHawk does | and Zaxnes |
|---|---|---|
| **power-on PPU position** | `NES.Core.cs` powers the PPU up at its own fixed start-of-frame position; no randomised or alignment-dependent offset. | the same class of thing. **Not a candidate** for a `.bk2` that agreed for 925 frames: a power-on offset would have parted at frame 1. |
| **OAM DMA alignment** | `cpu_deadcounter` is `2` or `1` by alignment and the block is gated on `apu.dmc_dma_countdown != 1 && !dmc_realign`, giving 513/514 — and `dmc_realign` makes an OAM cycle yield to the DMC's last countdown cycle. | `OAM_DMA_CYCLES_EVEN/ODD` = 513/514, and `run_dma`'s state machine gives the DMC priority. **Agrees on the count.** The `dmc_realign` interleaving rule is a candidate if a game's OAM and DMC DMA collide; `sprdma_and_dmc_dma` and `sprdma_and_dmc_dma_512` pass on Zaxnes (EVAL-SYNC-4 §4), and *Kirby* has one OAM transfer a frame at a fixed dot, so it is not this movie's. |
| **`$2002` read timing** | not examined in detail. | EVAL-SYNC-4 measured **zero** `$2002` reads in every *Kirby* frame of the window. **Ruled out for this movie** by measurement, so not read. |
| **NMI / IRQ polling** | `RunCpuOne` sets `cpu.IRQ = _irq_apu \|\| Board.IrqSignal` at step 5 — after the DMA block and `RunOneFirst`, before `ExecuteOne`. So an IRQ raised by `RunOneLast` of the previous cycle is seen on this cycle's `ExecuteOne`. | a candidate worth its own measurement, and it interacts with item 2: NesHawk's DMC IRQ is raised in `RunOneLast` three cycles after the fetch, so it is sampled one cycle later again. Not implemented; the MMC3 IRQ in *Kirby* enters its handler at the **same dot** on both machines (EVAL-SYNC-4 §2), which is the evidence against it mattering here. |

### What this means for the architecture

The three items to implement need exactly three places to vary, which is what step 2 builds:

| item | where |
|---|---|
| 1 | the bus's DMA: the halt's cycle, the fixed four cycles, no alignment, the `+2` |
| 2 | the DMC channel: a reload countdown, and `$4015` bit 4 reading off it |
| 3 | the DMC channel: `call_from_write`, its parity delay and its three-cycle DMA |

So `NesBus`, `DmcChannel` and `Apu2A03` (which owns the DMC and answers `$4015`) take the profile
parameter, and `NesCore` takes it because it owns the bus. `Ppu2C02`, `Cartridge`, `Cpu6502` and
`InputPorts` do **not**: nothing in items 1 to 3 reaches them.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE step 1/6 done: verify the differences verbatim
