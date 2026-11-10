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
- [x] 2. The architecture.
- [x] 3. The NesHawk profile.
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

---

## Step 2 — the architecture

The core is now **generic over the timing profile**, both instantiations build and run, and the
`Standard` one costs nothing it did not cost before. What the step actually took is two surprises
about how Rust and LLVM treat a type parameter, both measured and both recorded here because the
next lot that adds a hot entry point will meet them again.

### What was built

| | |
|---|---|
| `crates/zax-nes/src/timing_profile.rs` | the `TimingProfile` trait and the two zero-sized types `Standard` and `NesHawkCompat`. Members are **associated constants only** (`NAME`, `STATE_ID` so far), so nothing is stored, passed or compared at run time. |
| `nes_bus.rs` | `pub struct NesBusOf<P: TimingProfile>`, with a `PhantomData<P>` field and `impl<P> CpuBus for NesBusOf<P>` — the bus carries the parameter because the DMA is here **and** because `CpuBus::read`/`write` are trait methods that cannot take one. |
| `nes_core.rs` | `pub struct NesCoreOf<P: TimingProfile>`, holding `bus: NesBusOf<P>`. |
| both files | `pub type NesBus = NesBusOf<Standard>;` and `pub type NesCore = NesCoreOf<Standard>;` |
| `tests/timing_profile_tests.rs` | four tests (below). |
| `clippy.toml` | `BizHawk`, `NesHawk`, `QuickNes` added to `doc-valid-idents`. |

`Ppu2C02`, `Cartridge`, `Cpu6502`, `InputPorts`, `Apu2A03` and `DmcChannel` are **not** generic
types. The last two were going to be, and are not: see surprise 1.

### Surprise 1 — a default type parameter does not do what the brief assumed

The brief's plan was `NesCore<P: TimingProfile = Standard>`, "so every existing use compiles
unchanged". **It does not.** Rust applies a type parameter's default when it resolves a *type*, and
not when it resolves a path through it, so with a default on the struct:

| | |
|---|---|
| `NesCore::STATE_VERSION` and its fourteen siblings | **E0283**, "type annotations needed" |
| `NesBus::RAM_SIZE`, `NesCore::MAX_RUN_AHEAD`, `NesCore::SUPPORTED_MAPPERS`, ... | the same |
| `NesCore::new()`, `NesCore::state_builtin_patch(...)` | the same |
| `[u8; NesBus::RAM_SIZE]` | the same, and not fixable by an annotation |

Measured on this tree: 24 call sites in `zax-app`, `zax-settings`, `zax-scripting`, `zax-core-api`
and `zax-harness`, plus a dozen in `zax-nes`'s own integration tests, every one of them a path
through a constant or an associated function. Two further shapes were tried before the one that
works:

1. moving the constants into an `impl NesCore<Standard>` block — a concrete instantiation **does**
   resolve `NesCore::STATE_VERSION`, in const position too. It fixes the constants, leaves the
   associated *functions* broken, and costs 33 `Self::X` edits inside `nes_core.rs`;
2. a **parameterised** alias, `pub type NesCore<P = Standard> = NesCoreOf<P>;`. It fixes the
   constants *and* the functions in expression position, and still fails in const position
   (`[u8; NesBus::RAM_SIZE]`);
3. a **plain** alias, `pub type NesCore = NesCoreOf<Standard>;`. Everything resolves, in every
   position. Adopted.

So the generic structs are `NesCoreOf<P>` and `NesBusOf<P>`, and the old names are aliases to the
`Standard` instantiation. The result is what the brief wanted and what the default could not give:
**not one line outside `zax-nes` changed**, and the public API — including every savestate
constant — is the parent commit's.

The brief writes `NesCore<NesHawkCompat>`; the spelling in the code is `NesCoreOf<NesHawkCompat>`,
for that reason and no other.

### Surprise 2 — a type parameter is not free unless it is placed carefully

`golden-core` said the emulation had not moved, from the very first build. `bench-core` said the
*program* had, twice, and the two causes are different.

**2a. A generic function is monomorphised in the crate that calls it.** A generic `run_frame`
therefore moved the whole per-cycle chain under it — `Cpu6502::step`, `CpuBus::read`,
`NesBusOf::tick`, `run_dma` — out of `zax-nes`'s code generation and into every consumer's, where
it is partitioned and inlined against different neighbours. Five interleaved rounds against the
parent commit, with not one instruction of emulation changed:

| | parent | generic `run_frame` | |
|---|---|---|---|
| Super Mario Bros. 3 | 1.1297 ms/frame | 1.1728 | **+3.8 %** |
| Mega Man 3 `[USA][!!]` | 0.9072 | 0.9294 | **+2.4 %** |

The fix is to declare `run_frame` **once per instantiation** — `impl NesCoreOf<Standard>` and
`impl NesCoreOf<NesHawkCompat>`, each a one-line forward to the generic
`NesCoreOf::run_frame_profiled` — so that a consumer makes an ordinary cross-crate call again.

**2b. A generic body with one call site is inlined into its wrapper.** The parent's `run_frame` was
reached through `Box<dyn Engine>` and so was never inlined into anything; the new one had a wrapper
to be inlined *into*, and LLVM took it. That left:

| | parent | wrapped, still inlined | |
|---|---|---|---|
| Super Mario Bros. 3 | 1.15560 ms/frame | 1.17304 | **+1.5 %** |
| Mega Man 2 | 0.72803 | 0.75133 | **+3.2 %** |

twelve interleaved rounds each, the new build slower in **twelve of twelve** on Super Mario Bros. 3
— far outside the plus or minus 1.5 % spread of either binary on its own, and so not noise.
`#[inline(never)]` on `run_frame_profiled` puts the parent's structure back.

### The proof the step owes

**`golden-core`, the 98 originals, 1 200 frames each, release, feature off:**

    8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773

**Byte-identical** to the value the brief names (lot FCEUX-23's baseline, and EVAL-SYNC's,
EVAL-SYNC-2's, -3's and -4's). Run twice: once on the first generic build and once on the final
one, the same hash both times — which is the point, because between those two runs the *program*
changed by 1.5 to 3 per cent and the *emulation* did not change at all.

**`bench-core`, final build against the parent, six interleaved rounds per ROM** (`--frames 1200
--warmup 120`, the `core run_frame` line only, the two binaries alternating which goes first so
that neither always gets the cold slot):

| ROM | parent | this commit | |
|---|---|---|---|
| Super Mario Bros. 3 | 1.13933 ms/frame | 1.10465 | **-3.0 %** |
| Mega Man 3 `[USA][!!]` | 0.91257 | 0.88537 | **-3.0 %** |
| Tetris | 0.86203 | 0.84290 | **-2.2 %** |
| Mega Man 2 | 0.72932 | 0.73300 | **+0.5 %** |
| Metroid | 0.74215 | 0.74675 | **+0.6 %** |

Worst case **+0.6 %**, inside the owner's +1 % budget; three of the five ROMs come out about 3 %
*faster*. The mixed signs are what a 5 440-byte shift of `.text` does to code placement, and they
are the reason the two regressions above were believed instead of being written off: a layout
effect changes sign from game to game, while 2a and 2b were one-sided and twelve-for-twelve.

**Machine code.** `cargo asm` is not installed and was not installed. `objdump`, `nm` and
`llvm-objdump` are absent from this machine too, so the substitute is `dumpbin /HEADERS` on the
release `zax-harness.exe` (`lto = "fat"`, `codegen-units = 1`, `strip = true`, so there are no
per-function symbols left to compare):

| | `.text` virtual size |
|---|---|
| parent commit | `0x4B0FC0` (4 919 232) |
| generic, no `#[inline(never)]` | `0x4B2DC0` (4 924 864) |
| **this commit** | `0x4B2700` (4 924 672), **+5 440 bytes, +0.11 %** |

The last figure is the same with and without the `impl NesCoreOf<NesHawkCompat>` block — `0x4B2700`
both ways — which says the second core is **not in the binary at all** yet: `/OPT:REF` drops it
because nothing calls it. So the +5 440 bytes is the `Standard` side's own code, and the honest
statement is the narrow one: **the `Standard` path's behaviour is identical to the byte and its
speed is inside the budget, but its machine code is not claimed to be identical instruction for
instruction** — the instrument that would settle that does not exist on this machine. Step 4, where
the second core becomes reachable, re-measures both numbers.

### The tests

`crates/zax-nes/tests/timing_profile_tests.rs`, four of them:

| | |
|---|---|
| `the_two_profiles_are_named_and_numbered_apart` | the names and the savestate bytes |
| `the_plain_core_name_is_the_standard_profile` | `NesCore` and `NesCoreOf<Standard>` are one type |
| `a_profile_costs_the_core_no_memory` | both profiles are zero-sized and the two cores are the same size |
| `a_rom_that_plays_no_dpcm_runs_identically_on_both_profiles` | 20 frames of the minimal ROM, the same cycle count and the same picture |

The last is the **durable** one, and that is why it is written this way. Every item the NesHawk
profile will carry is about the DMC's sample-fetch DMA, so a ROM that starts no sample must run
identically on the two profiles for ever; the day it does not, an item was put in front of the
profile instead of behind it.

### Suites

| | passed | ignored |
|---|---|---|
| `zax-nes`, release | **1 181** | 10 |
| `zax-harness`, release, feature off | **125** | 1 |

`zax-nes` is EVAL-SYNC-4's 1 187 `--include-ignored` total plus this step's four new tests
(1 181 + 10 = 1 191). `zax-harness` is unchanged (125 + 1 = 126). `cargo fmt --all --check` and
`cargo clippy --workspace --all-targets -- -D warnings` are clean.

### One thing this machine cannot do, recorded once

`cargo build --workspace` **cannot link the `zaxnes` binary here**: `LNK1181: cannot open input
file 'SDL2.lib'`, and there is no `SDL2.lib` anywhere on the machine. It is not a regression of
this lot — nothing in this commit touches the application's linking — and it is why the suites
above are run per crate, as EVAL-SYNC-4's were. `cargo clippy --workspace --all-targets` does not
link, so it still covers the application and every one of its test targets.

### Proposed requirement wording (the spec is not edited, per the brief)

> **REQ-CORE-124** — The core emulates one of two **timing profiles** of the 2A03: the standard
> profile, and a NesHawk-compatible profile whose differences from it are listed in
> `docs/review/NESHAWK-PROFILE.md`. The profile is fixed when a core is created and cannot change
> while a ROM is loaded. The standard profile's behaviour is the behaviour of every requirement
> elsewhere in this specification; a savestate records its profile, and a restore into a core of
> the other profile is refused (REQ-CORE-091).

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE step 2/6 done: the architecture

---

## Step 3 — the NesHawk profile

Items 1, 2 and 3 of step 1 are implemented on `NesHawkCompat`, each behind its own associated
constant, with the `Standard` arm of every site written as the parent commit's code so that arm is
not merely equivalent but the same source. Items 5 and 6 stay candidates, as step 1 planned.

### The four constants, and the one site each reads

| constant | where it is read | what it does |
|---|---|---|
| `DMC_HALT_ON_REQUEST_CYCLE` | `nes_bus.rs` `poll_dmc_dma` | holds the request for **this** cycle rather than the next, so the cycle the memory reader ran dry on is itself the halt cycle (item 1a) |
| `DMC_FETCH_HAS_TWO_DUMMIES` | `nes_bus.rs` `promote_dmc_wait`, `dmc_fetch_cycles`, `run_dma`, `dma_cycle`, `is_a_lone_dmc_fetch` | a sixth `dma` bit, `DMA_DUMMY2`: a fetch is a fixed four cycles — halt, two re-reads, fetch — and no parity is tested anywhere (item 1b) |
| `DMC_LENGTH_LAGS_THE_FETCH` | `apu/dmc_channel.rs` `supply_byte`, `step_reload_countdown`, `quiet_clocks`, `save_state`/`load_state`; `apu/mod.rs` `step_cpu_cycle` | a three-cycle reload countdown: the length, the loop and the IRQ land at the end of the fetch's cycle plus two (item 2) |
| `DMC_ENABLE_WRITES_MID_SAMPLE` | `apu/dmc_channel.rs` `set_enabled`; `nes_bus.rs` `after_apu_write` | a `$4015` enable with the buffer empty starts a write-style fetch even mid-sample: displaced by the **DMC timer's** parity, and three cycles rather than four (item 3) |

Two pieces of state were added, both inert on the `Standard` profile: `DmcChannel::reload_countdown`
(item 2) and `NesBus::dmc_from_write` (item 3, "the pending request came from a `$4015` write, so it
is the short fetch").

**No savestate format moved.** The countdown is written **only** on the `NesHawkCompat` profile
(`if P::DMC_LENGTH_LAGS_THE_FETCH` in `DmcChannel::save_state`), so a `Standard` block is byte for
byte the block every earlier version wrote and `NesCore::STATE_VERSION` stays at 10. Telling the two
profiles' blocks apart is step 4's, and will rest on the core name at the head of the block rather
than on a version bump.

### Three findings worth their own lines

**`$4015` bit 4's window is three cycles wide and two of them are visible.** The fetch's own cycle
is a halted cycle on either machine, so no program can read `$4015` on it; the reads a program can
make are on `F + 1` and `F + 2`, and both show bit 4 **set** on `NesHawkCompat` where `Standard`
shows it clear. The test does that by making the read the DMA halts be a read of `$4015` itself,
which is the only way to see `F + 1` at all.

**The countdown is stepped once at the fetch, not three times after it.** NesHawk sets
`dmc_reload_countdown = 3` inside `Fetch()` and spends the first of the three in `RunOneLast()` of
the fetch's **own** cycle. This core performs a DMA read *after* the cycle's APU step, so
`supply_byte` sets 3 and immediately calls `step_reload_countdown` once; without that the window
would land a cycle late. It is one line and it is the difference between matching NesHawk and being
off by one on every sample end.

**A reload's get/put parity is not live on this core either, in practice.** Item 1b says NesHawk
tests no parity. Measuring the `Standard` profile's reload fetch at sixteen phases of the DMC timer
gave **four cycles every time**: every entry of both rate tables is even and `DmcChannel::reset`
fixes the output unit's phase on purpose (for blargg's `sync_dmc`), so a reload's dry cycle always
has the same parity and the alignment cycle is always spent. The 3-cycle fetch is reachable — the
`$4015` **load**, whose delay is chosen by the APU cycle's parity, and a reload whose dry cycle
passes on a write so that the halt lands a cycle later — and the item-1b test moves the parity that
second way. **Item 1b therefore costs the two profiles nothing against each other on an ordinary
reload**; what separates them there is item 1a alone, one cycle earlier.

### Unit tests — `crates/zax-nes/tests/neshawk_profile_tests.rs`, six

Every one is **differential**: it measures the same thing on both profiles and asserts the gap the
item predicts, so a regression on the `Standard` profile fails it too.

| | |
|---|---|
| `item_1a_the_halt_lands_on_the_request_cycle_itself` | sixteen timer phases: on `NesHawkCompat` the dry cycle's own read is halted and costs 5 (four halted cycles and the read); on `Standard` it costs 1 and the **next** read cycle is the halted one |
| `item_1b_the_fetch_is_a_fixed_four_cycles_whatever_the_parity` | the dry cycle passed on a write and the read displaced one cycle either way: `NesHawkCompat` 5 and 5, `Standard` 4 and 5 |
| `item_2_bit_4_of_4015_stays_set_after_the_last_fetch` | the four `$4015` values after a one-byte sample's only fetch: `Standard` never shows bit 4, `NesHawkCompat` shows it set, set, clear |
| `item_2_the_dmc_irq_is_raised_three_cycles_late` | bit 7 over the same window: `Standard` from the first read, `NesHawkCompat` from the third |
| `item_3_a_mid_sample_4015_enable_displaces_the_fetch_and_shortens_it` | after a mid-sample `$4015 = $10`: `Standard` halts the very next read cycle for 4 or 5 cycles, `NesHawkCompat` lets two or more read cycles pass and then costs 4 (three halted cycles and the read) |
| `the_reload_countdown_survives_a_savestate_round_trip` | the NesHawk core reads back every byte it wrote |

### The public test ROMs, on both profiles

`zax-harness test-roms` grew `--profile standard|neshawk` (the new `profile_choice` module, which
step 4 reuses for `movie-sync` and `frame-trace`); its five readers are now generic over the
profile, with one `match` at the top of `run_one`. Generic code cannot name the per-instantiation
`run_frame`, so `zax-nes` gained the small public trait `RunFrame` whose method **is** that
function.

**The `Standard` column is unchanged, ROM for ROM and row for row** — including `AccuracyCoin`'s
128/147, which is EVAL-SYNC-4 §4's figure to the test.

| suite | `Standard` | `NesHawkCompat` |
|---|---|---|
| `dmc_dma_during_read4` (5) | 3 pass, 2 manual | **2 pass, 1 fail**, 2 manual |
| `sprdma_and_dmc_dma` (2) | 2 pass | **0 pass, 2 fail** |
| `apu_test` (19, singles + the combined ROM + `pal_apu_tests`) | 19 pass | **17 pass, 2 fail** |
| `dmc_tests` (4) | 4 manual | 4 manual |
| `AccuracyCoin` | **128/147** | **120/147** |

The five ROMs and the eight `AccuracyCoin` rows that change, each against the item that moves it:

| what fails on `NesHawkCompat` | the item, and why it must |
|---|---|
| `apu_test/7-dmc_basics` #19, and `apu_test.nes` at its test 7 | the ROM's own text is *"There should be a one-byte buffer that's filled immediately if empty"*. Item 3 is exactly the denial of that: a `$4015` enable with the buffer empty queues a fetch two or three cycles out instead of at once. |
| `dmc_dma_during_read4/dma_4016_read` | prints `08 08 08 08 07` where the hardware prints `08 08 07 08 08`: the extra controller clock the halt causes lands one read later. That is item 1a, one cycle earlier, read out of a program. |
| `sprdma_and_dmc_dma`, `sprdma_and_dmc_dma_512` | blargg's fine-synchronisation loop is *written* to be one cycle longer than the DMC's byte so that it drifts into alignment ("3421+4 clocks per iter"). A chip whose fetch is a fixed four cycles has no alignment to drift into, so the loop cannot converge on it. This is a deduction from item 1b and the ROM's own documentation, not a claim about any particular build of BizHawk. |
| `AccuracyCoin`: `DMA + $2002 Read`, `DMA + $4015 Read`, `DMC DMA + OAM DMA`, `Delta Modulation Channel`, `DMC Reload Timing`, `DMA Landing on Write`, `Implied Dummy Reads` | seven rows that each place a DMC fetch against another access to the cycle. Items 1a and 1b move every one of those placements. |
| `AccuracyCoin`: `Interrupt flag latency` | item 2: the DMC IRQ is raised at the end of the fetch's cycle plus two, so the cycle the CPU first samples it on moves with it. |
| `dmc_dma_during_read4/dma_2007_read` (no documented verdict, so **manual** on both) | its screen moves from `11 22 / 11 22 / 44 55 / 11 22 / 11 22` to `11 22 / 11 22 / 11 22 / 11 22 / 44 55` — the same one-read displacement as `dma_4016_read`, recorded because the row's verdict cannot record it. |

**These are not regressions and they are not excused either.** The brief allows the
`NesHawkCompat` profile to fail what NesHawk itself would fail, and every failure above is traced
to an item that NesHawk's source does and the hardware does not. The profile exists to replay a
`.bk2`, not to be more accurate: `docs/review/NESHAWK-PROFILE.md` (step 6) says so in the place a
reader will look.

### The `Standard` profile, re-proved after the items were written

| | |
|---|---|
| `golden-core`, 98 originals, 1 200 frames | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773`, **byte-identical** |
| `zax-nes`, release, `--include-ignored` | **1 187** passed, 0 failed |
| `zax-harness`, release, feature off | **125** passed, 1 ignored |
| `AccuracyCoin` | 128/147, the same 19 failures as EVAL-SYNC-4 |
| `cargo fmt --all --check`, clippy `-D warnings --all-targets` | clean |

`zax-nes` goes from EVAL-SYNC-4's 1 187 by step 2's four tests and this step's six, and back to
1 187 because the ten new ones replace nothing: 1 181 + 10 ignored at step 2, 1 187 + 10 here.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE step 3/6 done: the NesHawk profile
