# The two timing profiles of the 2A03, and how to add an item to the NesHawk one

Lots **NESHAWK-PROFILE** through **NESHAWK-PROFILE-10**, branch `feat/neshawk-profile`, base
`b604dc83` (lot EVAL-SYNC-4). The working is `docs/progress/NESHAWK-PROFILE.md` and `-2` to `-10`;
the result in context is `docs/review/MOVIE-SYNC.md` §4f to §4k, and the instruments are in
`docs/review/NESHAWK-PROFILE-3-SCRIPTS.md`, `-5-SCRIPTS.md`, `-6-SCRIPTS.md`, `-7-SCRIPTS.md`,
`-8-SCRIPTS.md`, `-9-SCRIPTS.md` and `-10-SCRIPTS.md`.

> **NESHAWK-PROFILE-10 shipped the three measured items and closed the NMI boundary by
> measurement.** Lot 9 left three items measured right and shipped **off**, because the all-off
> profile reached a further *Kirby* frame (45 095) than any of them — on two errors cancelling,
> which lot 8 had predicted in writing. Lot 10's one instruction was to **ship the exact
> configuration** — items **12**, **1c** and **13** all on — and measure everything against it.
> The movie therefore parts at **9 353**, the all-off profile's 45 095 is gone, and the costs are
> exactly what the three constants' own notes predicted before the pass ran: two public ROMs
> (`sprdma_and_dmc_dma`, both of them), two `AccuracyCoin` rows (121/147 → **119/147**) and 45 and
> 104 *Faxanadu* checksum lines (247 and 314 → **202** and **210**). The `Standard` profile is
> untouched and this time provably so from the diff alone: the whole change is three `false` →
> `true` inside `impl TimingProfile for NesHawkCompat`, and `golden-core` is byte-identical.
> **Then the clock was found to part 9 326 frames before the work does.** The four marks hold −1 to
> BizHawk frame 26 and leave it at frame **27**, where the bus stream shows the two machines
> running the **same 8 363 instructions at a residual of exactly −1** with **no DMA of any kind**,
> parting on whether the vblank NMI displaces the spin loop's `BEQ $C070` — three cycles, lot 3's
> own mechanism with the phase now exact. Round 1 closed that candidate **four ways** and shipped
> no item: a constant dot phase is refused by the first **one-dot** sweep anyone has taken (every
> earlier sweep moved multiples of 3 dots, so twenty values were one residue class and the
> sub-cycle phase had never been varied); an **odd** net dot offset is unreachable because the
> constant is applied twice, and a post-reset-only bias of ±1 or ±2 dots is **exactly inert** with
> a +300 control proving the knob live; the odd-frame dot skip's parity moves a statistic and not
> the wall; and what the source describes at that site works out to **exactly one CPU cycle in all
> three alignments**, which is item 8, which `ppu_vbl_nmi` `05-nmi_timing` refuses because NesHawk
> passes it. The next lot's candidates are the brief's other two, untouched: the **OAM/DMC
> interleave** (item 13's unmeasured half, the likely cause of *Faxanadu*'s 45 and 104 lines) and
> the **MMC3 IRQ polling point**. `docs/progress/NESHAWK-PROFILE-10.md`.

> **NESHAWK-PROFILE-9 made the reset land where NesHawk's does, found a third difference, and
> shipped all three items off.** Lot 8's one job is done — **item 12**: NesHawk's frame ends at a
> PPU dot (`PPU.run.cs` `TickPPU_preVBL`) and its reset abandons the instruction in flight
> (`MOS 6502X` `NESSoftReset` is `opcode = VOP_RESET; mi = 0;`), so a frame the movie taps RESET
> after ends at that dot, and the four once-a-frame marks now read the **same −1 either side of the
> movie's reset** where they read −1 and then +2. With item 1c's gate thus open, the item its own
> 705-fetch measurement says is right went back on and frame **40**'s strobe collision closed. Then
> round 1 found the movie's **first difference of any kind** — tree frame 11's **first OAM
> transfer**, which NesHawk charges **514** cycles and this core **513** — and measured its shape
> off BizHawk's own stream: the 512 accesses land on the same cycles, NesHawk spends no halt cycle
> of its own and ends the transfer with **one cycle carrying no bus access**, so the two take the
> hardware's own 513 and 514 on **opposite phases** (**item 13**). With it the transfer agrees to
> the cycle and the marks hold −1 on BizHawk frames 2-12 and 16-26. **All three ship off**: every
> configuration that changes anything parts at **9 353** — which is lot 3's own wall, the frame item
> 8 was invented to pass — and item 13 alone costs *Faxanadu* 45 and 104 checksum lines. The
> shipped 45 095 stands because its errors cancel, exactly as lot 8 predicted. Three measured
> items, three constants, two new differential tests, one corrected old one, and the next lot's
> window named: frame 9 353. §4m of `docs/review/MOVIE-SYNC.md`,
> `docs/progress/NESHAWK-PROFILE-9.md`.

> **NESHAWK-PROFILE-8 found where the CPU phase is born, and restored item 1c.** Lot 6's
> retraction of item 1c was itself wrong, and the measurement that says so needs no cross-machine
> calibration at all: over 36 frames and **705** sample fetches of BizHawk's own bus stream, a
> reload whose halt passes **no** CPU write costs the CPU **4** cycles (648 of 648), one that passes
> **one** costs **3** (46 of 46) and one that passes **two** costs **4** (11 of 11) — `stall =
> 4 − (slip mod 2)`, item 1c's own rule, no exceptions. Lot 6 took the slip from *this* core's census
> and the stall from BizHawk's stream, and the two machines' CPUs are **two cycles apart inside the
> instruction** from the movie's reset onwards. The item nevertheless **ships off**, because the lot
> also found the second half: the four once-a-frame marks of frames 1-56 read a **constant −1**
> before the `.bk2`'s reset and a **constant +2** after it, because **NesHawk's frame ends at a
> fixed PPU dot, mid-instruction**, where this core's ends after the instruction that straddles it —
> so the reset lands **3 CPU cycles late**, the PPU restarts there (item 10), and the game's
> NMI-synced code runs 3 cycles late against the CPU-clocked DMC train for the rest of the movie.
> That is what puts NesHawk's halts on write cycles and this core's on read cycles, and it explains
> both collisions: frame **40**, where *this* core runs Kirby's extra input pass, and frame **55**,
> where NesHawk does. Item 1c alone removes frame 40's and costs the movie 44 170 frames; the next
> lot's one job is the frame boundary. §4l of `docs/review/MOVIE-SYNC.md`,
> `docs/progress/NESHAWK-PROFILE-8.md`.

> **NESHAWK-PROFILE-7 retired item 6 by measurement and moved the movie's first difference from
> frame 45 085 to frame 55.** The DMC fetch train is **exact**: over frames 11 to 59 the two
> machines make the same **1 524** sample fetches, of the same addresses, in the same order, at an
> absolute-cycle residual of **−1**. So nothing an end-of-sample rule could fix is missing, and lot
> 6's window could not have answered the question it was asked — at frame 45 085 the two machines
> are not playing the same sample (NesHawk restarts at tree frames 45 085, 45 093, 45 097 and this
> core at 45 047, 45 067, 45 087). What divides them is a DMC halt landing on the game's own
> `LDA $4016,X` read on NesHawk and not here, at frame **55**, which costs Kirby's input routine one
> extra pass (10 `$4016` strobes against 8); the two machines' whole DPCM schedule then agrees event
> for event to frame **1002**, and the work RAM to 45 095 because the sound driver's state lives in
> the cartridge's PRG-RAM, which `--ram-ref` does not compare. **Both constants that could move the
> CPU stream against the DMC's train were swept over thirteen values and refused**, and the DMA
> accounting was measured exact against BizHawk on thirteen fetches. `docs/progress/NESHAWK-PROFILE-7.md`.

> **NESHAWK-PROFILE-6 answered Q-182 by measurement, retracted item 1c, and turned item 11 on.**
> A NesHawk DMC reload fetch costs the CPU **four** cycles whatever the CPU was writing on the
> request cycle, so item 1c's odd-slip cycle is not a NesHawk behaviour at all — and with it gone
> item 11 (`DMC_POWER_ON_TIMER = 1019`) pays: *Kirby* goes from parting at frame **8 005** to frame
> **45 095**. The measurement is calibration-free, which is what Q-182 was waiting for: of frame
> 11's `$40xx` accesses every **write** lands at residual 0 against BizHawk's own up to and
> including the `$4007` write the disputed request falls on, and the `$4015` read 252 cycles later
> lands at +1, with exactly one fetch between them. It costs **one** `AccuracyCoin` row of this
> profile (`DMA + $4015 Read`, 122/147 → 121/147; `Standard` keeps it). §4k of
> `docs/review/MOVIE-SYNC.md` has the trade, and NesHawk's own `AccuracyCoin` run leaves that row
> `NOT RUN`, so the recipe's gate does not fire.

## What this is, in three sentences

Zaxnes's core is generic over a **timing profile**: `zax_nes::Standard` is the 2A03 it has always
emulated, and `zax_nes::NesHawkCompat` is one that makes BizHawk NesHawk's choices where they
differ. The `.bk2` player selects the second automatically when a movie's `Core` header says
`NesHawk`, and nothing else ever does. With it, the *Hoshi no Kirby* `.bk2` that parted at frame
926 replays with **no live difference from BizHawk's own work RAM until frame 9 353** of 156 312,
and both *Faxanadu* dumps agree on **202** and **210** of their 832 checksum lines where the
standard profile manages 207 and 220.

**The headline used to read 45 095 and 247/314, and NESHAWK-PROFILE-10 is why it does not.** Those
figures came from a profile that left three measured-right items **off**, and lot 8 had predicted in
writing what lot 9 then measured: the all-off profile reaches a further frame **because two of its
errors cancel**. With items 12, 1c and 13 on — each measured off BizHawk's own bus stream, none of
them a fitted constant — the movie parts at 9 353. That is a worse number and a better machine, and
the distinction is the whole point of the trap *"when two errors cancel, fixing one makes things
worse — and that is progress, not a setback"* below. The figure to move is 9 353.

> **NESHAWK-PROFILE-4 changed the headline, and downwards for one movie.** Lot NESHAWK-PROFILE-3
> took *Kirby* to frame **18 321** with item 8, a one-CPU-cycle lag of the whole /NMI line, and
> said in the same breath that the item was blunt. Lot NESHAWK-PROFILE-4 proved it is not a
> NesHawk difference at all — `ppu_vbl_nmi` `05-nmi_timing` measures the vblank NMI's arrival at
> one-PPU-dot resolution over all three alignments, and **NesHawk passes it**, so NesHawk takes
> that NMI on the cycle this core does — and **retracted** it, putting the boot difference it was
> standing in for (item 10) in its place. The trade is stated in full in §4i of
> `docs/review/MOVIE-SYNC.md`: eleven public ROMs and four `AccuracyCoin` rows and 111 and 145
> *Faxanadu* checksum lines bought back, and 10 316 *Kirby* frames given up.

## What it is **not**

**The NesHawk profile is not the more accurate chip.** On every item below, Zaxnes's own profile
agrees with Mesen 2 and with the nesdev wiki and the NesHawk one does not; the NesHawk profile
**fails seven public test ROMs and ten `AccuracyCoin` rows that the standard profile passes**
(`docs/progress/NESHAWK-PROFILE.md` step 3 has the first table, each failure traced to the item
that causes it). `AccuracyCoin` went 120/147 at NESHAWK-PROFILE, 122/147 at NESHAWK-PROFILE-2,
**118/147** at NESHAWK-PROFILE-3's item 8, **back to 122/147** when NESHAWK-PROFILE-4 retracted it,
**122/147** still after NESHAWK-PROFILE-5, which moved no suite at all, **121/147** after
NESHAWK-PROFILE-6 retracted item 1c — the one row being `DMA + $4015 Read` — and **119/147** since
NESHAWK-PROFILE-10 shipped items 12, 1c and 13. It exists so that a movie recorded against those
choices can be replayed against them, which is a different job from being right.

The seven ROMs, measured by NESHAWK-PROFILE-10 (`test-roms --profile neshawk`, 205 pass / 14 fail
against `standard`'s 212 / 7): `apu_test` and its `7-dmc_basics`, `cpu_interrupts_v2` and its
`4-irq_and_dma`, `dmc_dma_during_read4/dma_4016_read`, and **both** ROMs of
`sprdma_and_dmc_dma` — the last two being item 13's own predicted cost, since `Standard`'s OAM
phase is the one that suite measures. The ten `AccuracyCoin` rows are `$2002 flag timing`,
`Controller Strobing`, `DMA Landing on Write`, `DMC DMA + OAM DMA`, `DMC Reload Timing`, `Delta
Modulation Channel`, `Frame Counter 4-step`, `Frame Counter 5-step`, `Implied Dummy Reads` and
`Interrupt flag latency`, with `Hybrid Addresses` going the other way.

**`ppu_vbl_nmi` and `vbl_nmi_timing` are 11/11 and 7/7 on both profiles**, which is load-bearing
rather than decorative: `05-nmi_timing` is the ROM whose passing on NesHawk refuses item 8, and it
is what lets NESHAWK-PROFILE-10 close the NMI boundary again after re-reading `TickPPU_VBL`.
`cpu_interrupts_v2` is 6/6 on `standard` and **4/6** on `neshawk`.

So: never make it the default, never offer it as a "better accuracy" setting, and never fix a game
with it. If a game needs it, the bug is somewhere else.

## The items, each with its source

Every item is an associated constant of `zax_nes::TimingProfile`, read at exactly the sites listed.
`Standard`'s value is `false` everywhere, and the `else` arm of every site is the code this core had
before the profile existed — not an equivalent of it, the same source.

| | what the NesHawk profile does | read from | sites |
|---|---|---|---|
| `DMC_HALT_ON_REQUEST_CYCLE` (1a) | the halt of a DMC reload lands on the **request cycle itself**, because `DMCUnit.Run()` lowers `RDY` and `RunOneFirst()` runs before `cpu.ExecuteOne()` | `APU.cs` `DMCUnit.Run`, `NES.Core.cs` `RunCpuOne` | `nes_bus.rs` `poll_dmc_dma` |
| `DMC_FETCH_HAS_TWO_DUMMIES` (1b) | a fetch is a **fixed four** cycles — halt, two re-reads, fetch — with no get/put alignment, there being no `% 2` on that path | `APU.cs` `dmc_dma_countdown = 3`, `RunDMCHaltFetch` | `nes_bus.rs` `promote_dmc_wait`, `dmc_fetch_cycles`, `run_dma`, `dma_cycle`, `is_a_lone_dmc_fetch` |
| `DMC_LENGTH_LAGS_THE_FETCH` (2) | the sample length, the loop and the IRQ land **three cycles after** the fetch, so `$4015` bit 4 stays set across them | `APU.cs` `DMCUnit.Fetch` (`dmc_reload_countdown = 3`) and `RunOneLast` | `apu/dmc_channel.rs` `supply_byte`, `step_reload_countdown`, `quiet_clocks`, the savestate; `apu/mod.rs` `step_cpu_cycle` |
| `DMC_ENABLE_WRITES_MID_SAMPLE` (3) | a `$4015` enable with the buffer empty starts a write-style fetch **even mid-sample**, delayed by the **DMC timer's** parity and three cycles long | `APU.cs` `set_lenctr_en` (the `!sample_buffer_filled` sibling test) and `Run`'s `else` branch | `apu/dmc_channel.rs` `set_enabled`; `nes_bus.rs` `after_apu_write` |
| `PPU_START_UP_DOTS_SKIPPED` (10) | the PPU's **first frame after a power-on and after a soft reset is a start-up frame of `241 * 341 - 4 = 82 177` dots**, not a whole one of 89 342 — so the first frame boundary comes **2 389 CPU cycles earlier**, and the reset does it again from the top of the frame. 7 165 dots on the NesHawk profile, which is `89 342 - 82 177` | `NES/PPU.cs` `Reset()` (`ppudead = 1`, `start_up_offset = 4`, and `regs_reset()`'s `ppur = new PPUREGS()`), `NES/PPU.run.cs` `NewDeadPPU()`, `PPU.cs` `runppu()` (which clocks the CPU throughout) — against this core's power-on at the start of vblank (Q-003) and its `Ppu2C02::soft_reset`, which does not restart the frame (Q-078) | `nes_core.rs` `power_on` and `soft_reset`, through `NesBusOf::run_ppu_start_up_dots` and `Ppu2C02::run_start_up_dots` |
| `DMC_POWER_ON_TIMER` (11) — **on since NESHAWK-PROFILE-6** | the DMC's timer powers on at a flat **1 020 CPU cycles** rather than one period of the rate in force, putting the output unit's whole phase — and the cycle every sample fetch of a game lands on — `1020 - 428 = 592` cycles further on. NesHawk's `timer = 1020` is this core's `1019`, the convention being NesHawk's-plus-one. **The shipped value is `0`**, which is one period and the line `DmcChannel::reset` always had: see the constant's note and Q-182 | `APU.cs` `DMCUnit`'s constructor (`timer = 1020` beside `timer_reload = DMC_RATE[0]`) and `Run()`; `NESSoftReset()` and `NESHardReset()`, neither of which re-seeds it | `apu/dmc_channel.rs` `reset`, reached from `nes_core.rs` `power_on` alone |
| `DMC_FETCH_STALLS_AN_OAM_TRANSFER` (7) | a DMC fetch that falls **inside an OAM transfer** costs that transfer **two** cycles where this core's loses one, because NesHawk's `dmc_realign` also skips the OAM cycle *after* the fetch | `NES.Core.cs` `RunCpuOne` (the OAM block's `dmc_dma_countdown != 1 && !dmc_realign` gate, and `dmc_realign = true` on the fetch cycle) | `nes_bus.rs` `run_dma`, the `dmc_may_read` arm |
| `DMC_HALT_LANDS_ON_A_WRITE_CYCLE` (1c) — **on since NESHAWK-PROFILE-10** | the halt of a DMC reload lands on the request cycle **even when the CPU is writing on it**, so an **odd** slip costs the fetch one cycle less: `stall = 4 − (slip mod 2)`. Measured on **705** fetches of BizHawk's own bus stream with no cross-machine calibration — slip 0 → 4 (648 of 648), slip 1 → **3** (46 of 46), slip 2 → 4 (11 of 11), no exceptions | `NES.Core.cs`'s `if (!cpu.RDY && !cpu.rdy_freeze && apu.dmc_dma_countdown == apu.DMC_RDY_check) apu.dmc_dma_countdown += 2;` with `DMC_RDY_check = 2`, against `MOS 6502X` `Execute.cs` where a write microstep runs whatever `RDY` is doing | `nes_bus.rs` `promote_dmc_wait`, `is_a_lone_dmc_fetch` |
| `OAM_DMA_WAITS_AT_THE_END` (13) — **on since NESHAWK-PROFILE-10** | an OAM transfer's wait cycle is at the **end**: the 512 accesses start on the first read-phase cycle at or after the halt, and the CPU loses **one more cycle after the last `$2004` write**. So the two profiles take the hardware's own 513 and 514 (nesdev, "DMA") on **opposite phases** — `[513, 514, 513, 514]` against `[514, 513, 514, 513]`, summing to 1 027 on every phase, which no offset could do. It is the *Kirby* movie's **first difference of any kind** | the `$4014` handler's `sprdma_countdown = 1` and `RunCpuOne`'s `if (apu.dmc.timer % 2 == 0) cpu_deadcounter = 2; else cpu_deadcounter = 1;`, measured first off BizHawk's own stream (`q184.lua`) and read second | `nes_bus.rs` `run_dma`, twice |
| `RESET_LANDS_ON_THE_FRAME_BOUNDARY_DOT` (12) — **on since NESHAWK-PROFILE-10** | a frame the movie taps RESET after ends **at the PPU dot the frame boundary falls on**, before the instruction that straddles it, so the reset's seven cycles begin on that dot's own CPU cycle rather than 3 cycles later. It is item 1c's gate: those 3 cycles are what put NesHawk's halts on write cycles and this core's on read cycles | `PPU.run.cs` `TickPPU_preVBL()` sets `nes.frame_is_done = true` at a dot with the CPU clocked from inside the same loop; `NES.Core.cs` `FrameAdvance` then applies the `.bk2`'s RESET at the top of the next frame and `MOS 6502X`'s `NESSoftReset()` — `opcode = VOP_RESET; mi = 0;` — **abandons the instruction in flight** | `nes_core.rs` `run_frame_profiled_gen<const BEFORE_RESET>` and `run_frame_before_soft_reset`, through `NesBusOf::frame_boundary_is_the_next_cycle` |

**BizHawk is MIT** and nothing is copied from it: each item is written from the behaviour its source
describes, and the table names the function so that the reading can be checked. Mesen 2 is GPL-3.0
and the standard arms, which agree with it, carry their attribution where they already did.

**Which item closed *Kirby*** was measured rather than assumed, one item at a time:

| | |
|---|---|
| frame **926** | **1a** does it (1b then takes the residue from 17 frames to 8), **3** does it independently, and **2 is exactly inert** on that movie. `docs/progress/NESHAWK-PROFILE.md` step 5 |
| frame **4666** | **1c** does it, and the movie goes from parting at 4 665 to parting at **9 355** — 3 531 clean frames to 8 175 of 8 901, and 5 370 frames with a live difference to 726. **3's parity correction and 7 are *exactly* inert**: each leaves the 8 901-frame table byte-identical. `docs/progress/NESHAWK-PROFILE-2.md` step 4 |
| frame **9354** | **8** did it, and the movie went from parting at 9 353 to running out of the reference entirely — 8 175 clean frames to 8 792 of 8 901 and 0 frames identical to the byte to 213, and a second reference then took it to 18 321. **Item 8 has since been retracted**: it is not a NesHawk difference (see below), and with it gone the movie parts at 8 005 again. The OAM alignment that looked like the next item is **eliminated by arithmetic**. `docs/progress/NESHAWK-PROFILE-3.md` steps 2 and 3 |
| frame **8005**, and the reason it is still 8 005 | **11 does not do it, and that is a measurement and not a shrug.** The DMC's power-on phase was the first thing in the whole movie whose cycle differs once item 10 removed the drift: frames 1 to 10 — the power-on, the movie's soft reset and the first ten frames — are identical event for event and cycle for cycle on both machines, and frame 11, the first that plays a DPCM sample, has NesHawk fetching the sample's second byte 211 cycles earlier. Item 11 fixes exactly that, to the cycle, on all thirteen fetches of the frame — and **costs the movie 7 080 frames**, because the wrong phase was cancelling one cycle elsewhere. The cycle is isolated (the one reload fetch whose request lands on a write cycle) and the source refuses the item that would pay it, which is **Q-182**. `docs/progress/NESHAWK-PROFILE-5.md` steps 1 to 4 |
| frame **55**, which is where the movie really parts | **nothing yet, and the frame is the result.** NESHAWK-PROFILE-7 measured the whole prefix instead of one window: the DMC fetch train is exact for 1 524 fetches, the DPCM schedule agrees event for event to frame **1002**, and the first difference of **any** kind is at frame 55, where a DMC halt lands on the game's `LDA $4016,X` read cycle — the **same absolute cycle** on both machines — and NesHawk's CPU is on that read while this core's is seven cycles further on in the same 14-cycle loop. NesHawk's pad byte comes back corrupted (`C0`, then `41`), Kirby re-reads, and the frame costs it ~155 cycles more. The reason 45 095 ever looked like the parting frame is that the extra pass reads the **same pad value**, so the work RAM cannot see it, and Kirby's sound driver keeps its state in the **cartridge's PRG-RAM at `$6000`**, which `--ram-ref` does not compare. `docs/progress/NESHAWK-PROFILE-7.md` steps 1 and 2 |
| frame **45 095** | **item 1c's retraction**, with item 11. *Kirby* goes from 8 005 to 45 095 — the `step1c` and `step1d` windows reached in full and `step1e` parting at 45 095. What it parts on now is **not a DMA cost**: the `$4015` write that restarts frame 45 085's sample, the CPU stream and the write-started fetch are exact to the cycle, and the **next** fetch is 111 cycles later here against 372 on NesHawk — the DMC output unit owed 3 bits where NesHawk's owed about 7. That is **item 6**'s ground. `docs/progress/NESHAWK-PROFILE-6.md` steps 1 and 2 |
| frame **45 095**, and why it is still 45 095 | **item 1c is right and cannot be shipped yet, and the reset is why.** NESHAWK-PROFILE-8 measured the four once-a-frame marks of frames 1-56 and found the residual a **constant** — −1 before the `.bk2`'s reset, +2 after it — leaving its constant at frame **12**, inside the movie's first frame with any DMA at all. The first instruction whose start cycle differs is frame 11's `$C562 STA ($18,X)`, **10** cycles here and **9** there, and the rule is item 1c on 705 fetches. The +2 is the reset: NesHawk's frame ends at a PPU dot mid-instruction, so its reset lands 3 CPU cycles earlier, the PPU restarts there and the game's code runs 3 cycles late against the DMC train for ever. Item 1c alone closes frame 40's collision and costs 44 170 frames; the frame boundary is the next lot's one job. `docs/progress/NESHAWK-PROFILE-8.md` steps 1 and 2 |
| frame **45 095**, and why it is *still* 45 095 | **the reset is fixed, item 1c is on, a third item is measured — and the three together part at 9 353.** NESHAWK-PROFILE-9 closed item 1c's gate (item 12, the frame boundary) and found the movie's first difference of any kind, tree frame 11's first OAM transfer at 514 cycles against 513 (item 13). Each is measured off BizHawk's own stream; together they hold the four marks at −1 through BizHawk frame 12 and again on 16-26, close frame 40's collision, and take `step1`'s byte-identical frames from 288 to 318 of 400. **And the movie parts at 9 353 in every configuration that changes anything** — lot 3's own wall, which item 8 was invented to pass. The shipped trio-off keeps 45 095 because its errors cancel. `docs/progress/NESHAWK-PROFILE-9.md` steps 1 to 3 |
| frame **9 353**, which is where it parts now | **the three items ship ON, and the clock parts 9 326 frames before the work does.** NESHAWK-PROFILE-10 shipped items 12, 1c and 13 together, on the owner's instruction and on each item's own measurement, so 45 095 — two errors cancelling — is gone. Then the four marks over BizHawk frames 25-9 360 (`q185.lua`) put the **first** difference at BizHawk frame **27**, not 9 353: the two machines run the same **8 363 instructions at a residual of exactly −1** with **no DMA of any kind** and part on whether the vblank NMI displaces the spin loop's `BEQ $C070`. Round 1 closed that **four ways** and shipped no item — a constant dot phase (the first **one-dot** sweep taken; every earlier one moved multiples of 3), an odd net dot offset (unreachable, and a post-reset-only bias of ±1-2 dots is **exactly inert**, with a +300 control), the odd-frame skip's parity, and the NMI rule (item 8 again, refused by `05-nmi_timing`). `docs/progress/NESHAWK-PROFILE-10.md` steps 2 and 3 |
| the **4 778-cycle drift**, and every ROM item 8 cost | **10** does it. BizHawk's own cycle ledger says its first frame is 27 392 cycles and so is the frame the movie's first reset lands in, where this core runs a whole 29 781 — and `2 * (29 781 - 27 392) = 4 778` is the constant drift NESHAWK-PROFILE-3 measured and could not explain. With item 10 the drift of the NMI entry over frames 2-1 201 is **-2 ± 5** instead of 4 776 ± 5, every one of the eleven ROMs and four `AccuracyCoin` rows item 8 cost is back, and *Faxanadu* goes to **247** and **314** of 832 — past the standard profile's own 207 and 220. *Kirby* parts at **8 005**. `docs/progress/NESHAWK-PROFILE-4.md` steps 1 and 2 |

**A parity trap, written down because it cost a lot a round.** NesHawk's DMC `timer` is **this
core's plus one** at every instant: it stores `timer_reload = DMC_RATE[i]` (428 for index 0) and
`Run()` reloads to it, so it runs 428 → 1, while `DmcChannel::clock_timer` stores `rates[i] - 1`
and runs 427 → 0. Both are read after the same cycle's clock. So **every `timer % 2` taken from
NesHawk's source has to be inverted here** — item 3 was written with the source's own text and the
wrong convention, and NESHAWK-PROFILE-2 round 2 corrected it.

## The candidates that are **not** implemented, and why

Read from the same two files, each with the reason it was left out, so nobody re-reads them from
scratch (NESHAWK-PROFILE step 1 items 5 and 6, and NESHAWK-PROFILE-2 step 3):

| | what NesHawk does | why not implemented |
|---|---|---|
| **item 5**, the APU register delay | `RunOneLast`'s `pending_reg` dispatch applies `$4003`, `$4007`, `$4010`, `$4015` and `$4017` at the end of the write's own cycle and **every other APU register on the first cycle with `dmc.timer % 2 == 0`** — so `$4000`-`$4002`, `$4004`-`$4006`, `$4008`, `$400A`-`$400E`, `$4011`, `$4012`, `$4013` all wait nought or one cycle. A second write while one is pending **loses the first**: `WriteReg` overwrites `pending_reg` with no flush | **Measured as CPU-timing-neutral on the one movie there is.** Of the registers it defers only `$4012` and `$4013` can move a DMC fetch, and the sample address they set is not read until the next fetch hundreds of cycles later. It would touch `Apu2A03::write` for every game and grow a pending-write register, a dispatch in `step_cpu_cycle` and a savestate field. **It is still the first item to add for a game whose audio differs**, because the lost-write rule is audible |
| **item 6**, the end-of-sample aborts — **retired** | `fill_glitch_2` (a one-cycle DMA, `dmc_dma_countdown = -2`, then aborted) and `pending_disable` (a `$4015` disable that waits for the reload countdown, with `dmc_irq_glitch` suppressing the IRQ), both set by `set_lenctr_en(false)` and both needing `out_bits_remaining == 0` **and** `sample_length != 0`. `fill_glitch` — the "buffer fill with no DMA at all" — is **assigned `true` nowhere at 2.11.1**: `Run()` reads it and clears it and no arm sets it | **Retired by NESHAWK-PROFILE-7, by measurement.** Over frames 11 to 59 the two machines make the **same 1 524 sample fetches, of the same addresses, in the same order**, at an absolute-cycle residual of **−1**, through every sample end and every restart of that window — so the output unit's bit phase is exact and there is nothing an end-of-sample rule could fix. Lot 6's "3 bits owed against about 7" at frame 45 085 compares **two different restarts**: NesHawk's sample runs tree frames 45 085-45 091 and this core's 45 087-45 093. Also useful from the reading: `DMCUnit.Clock` tests `out_bits_remaining == 0` **before** decrementing and reloads with **7**, so its counter is **this core's minus one** — the same `+1` convention trap as the timer, and `out_bits_remaining == 0` there means `bits_remaining == 1` here |
| **item 10's dot count** and **item 11's seed**, as knobs | the two constants that move the game's code against the DMC's train: the game is frame-synced, so its code position is set by the PPU and the fetch train by the DMC's own timer, and the start-up dots run with no CPU cycle attached | **Both swept and refused, NESHAWK-PROFILE-7 step 2.** `PPU_START_UP_DOTS_SKIPPED` at 7 156, 7 159, 7 162, **7 165**, 7 168, 7 171, 7 174 and `DMC_POWER_ON_TIMER` at 1 013, 1 015, 1 017, **1 019**, 1 021, 1 023, 1 025, measured on the controller-strobe table of frames 2-1 100: **not one of the thirteen reproduces the frame-55 collision**, and the shipped pair is the best of them (23 disagreeing frames of 30 conflicted). The residual is not a phase constant |
| the **`$4016` strobe delay** | `RunOneLast`'s *first* action: a strobe is applied on the next cycle with **`dmc.timer % 2 == 1`**, not at the write | A **controller** item, not a DMA one, and nothing in this tree records it. *Kirby* strobes once a frame many cycles before it reads, so it cannot move this movie |
| the **IRQ sampling point** | `cpu.IRQ = _irq_apu \|\| Board.IrqSignal` is step 5 of `RunCpuOne`: after the DMA block and `RunOneFirst`, before `ExecuteOne` | *Kirby*'s MMC3 IRQ enters its handler at the **same dot** on both machines (EVAL-SYNC-4 §2, re-measured at SL 174 dot 288 in NESHAWK-PROFILE-2 step 2) |
| the **OAM transfer's alignment** | `NES.Core.cs`'s `sprdma_countdown` block: `if (apu.dmc.timer % 2 == 0) cpu_deadcounter = 2; else cpu_deadcounter = 1;`, so the extra cycle is decided by the parity of the **DMC's timer** and nothing in the decision looks at the cycle, where this core waits for the first get cycle | **Eliminated by arithmetic, and for every game rather than for one movie.** The DMC's timer counts down one per CPU cycle and reloads to `rates[i] - 1`, so its period is `rates[i]` — and all sixteen NTSC rates and all sixteen PAL ones are **even**. The timer's parity is therefore locked to the CPU cycle's from power-on, no reload and no `$4010` write can unlock it, and NesHawk's rule is a different *expression* of this core's rule. It was implemented, measured byte-identical over 81 frames, and reverted: a constant whose two arms are provably the same function cannot be given a differential test |
| the **branch interrupt quirk** | `Execute.cs` `RelBranch_Stage3` sets `branch_irq_hack` for every taken non-page-crossing branch (`interrupt_pending` is assigned `false` in `Fetch1` and never assigned `true` anywhere), and `Fetch1` then skips the whole interrupt test on the instruction that follows | **Eliminated by measurement against a reading that predicted the opposite.** The interrupt-phase ROM of NESHAWK-PROFILE-3 step 2 puts every `LDA` boundary of its spin loop immediately after such a branch, and NesHawk names the `LDA` as the displaced instruction on **20 of 64** frames. This core's own rule — a taken non-page-crossing branch polls one cycle earlier, `Cpu6502::branch` — is what `cpu_interrupts_v2` `5-branch_delays_irq` states in words, and both cores pass it |
| **item 8**, the CPU's view of /NMI a cycle late | `PPU.run.cs` `TickPPU_VBL` latches `nmi_destiny` and an `NMI_offset` read off `cpu_stepcounter` at cycle 0 of the NMI scanline and assigns `nes.cpu.NMI = true` at cycle `3 + NMI_offset`, so the assignment always lands on the dot after a CPU-cycle boundary and `Fetch1` reads it at the next one | **Implemented by NESHAWK-PROFILE-3, retracted by NESHAWK-PROFILE-4, and the retraction is the stronger result.** `ppu_vbl_nmi` `05-nmi_timing` prints *which instruction* the vblank NMI displaced, over ten lines that each move the whole test one PPU dot later, and checks the ten answers against a CRC — so it sweeps all three alignments at one-dot resolution. **NesHawk passes it**, and so does the standard profile, so NesHawk takes that NMI on the cycle this core does and no item that moves it can pass. Three shapes were implemented and measured before the conclusion was drawn (the table is `docs/progress/NESHAWK-PROFILE-4.md` step 1); all three fail it. What item 8 was really standing in for is **item 10**, the boot |
| **items 1c, 12 and 13** — **all three SHIPPED ON by NESHAWK-PROFILE-10**, and so no longer candidates | — | They are in the implemented table above, each with its source and its sites. Kept here as a pointer because four lots' worth of notes send a reader to this row: lot 8 restored item 1c and shipped it off, lot 9 added items 12 and 13 and shipped all three off — the all-off profile reaching a further *Kirby* frame (45 095) **because two of its errors cancel** — and lot 10 shipped the three measured items instead, which parts the movie at **9 353** and costs `sprdma_and_dmc_dma` (both ROMs), two `AccuracyCoin` rows and 45 and 104 *Faxanadu* checksum lines, every figure exactly as the three constants' own notes predicted. `docs/progress/NESHAWK-PROFILE-8.md`, `-9.md` and `-10.md` |
| the **power-on PPU position**, **`$2002` read timing** | — | the `$2002` read timing is ruled out by measurement in NESHAWK-PROFILE step 1: *Kirby* makes **zero** `$2002` reads in the frames measured. The **power-on PPU position** was ruled out there too — "a power-on offset would have parted at frame 1" — and that was **wrong**: it is item 10, and the reason frame 1 does not part is that the game spends its first frames in a wait loop. NESHAWK-PROFILE-4 step 2 measured it instead of arguing from it |

## The instrument: `frame-trace --dmc-census`

Built by NESHAWK-PROFILE-2 step 2 because aggregate counts could not diagnose frame 4666. It
prints, for one frame, every DMC request with the earliest cycle its halt was allowed on, every
halt, every fetch with the cycles it cost, every OAM request, every APU register write, and every
`$4015` and `$2002` read — each placed against the instruction it happened inside, with the DMC's
own `(timer, bits_remaining, bytes_remaining)` — and then the totals and a histogram of the fetch
costs.

    cargo build -p zax-harness --release --features dma-census
    zax-harness frame-trace <movie> <rom> --frame <n> --dmc-census

It is **an instrument and not emulation**: `crates/zax-nes/src/dma_census.rs` exists only under the
`dma-census` feature, and **every one of its nine call sites in `nes_bus.rs` carries
`#[cfg(feature = "dma-census")]` on the statement itself**, so with the feature off (the default,
and what every suite is run with) the hooks are removed before type-checking and the shipped core is
the core without them. Every site is reached at most once per fetch, once per transfer or once per
register access — never per cycle. Without the feature, `--dmc-census` is a **usage error naming
the flag to rebuild with**: an empty section would read as "this frame has no DMA", which is the
opposite of what it would mean.

## Two more instruments: the cycle ledger, and the interrupt-phase ROM

Lot NESHAWK-PROFILE-4 added none of its own: it ran `bk2cyc1.lua`'s existing output
(`kirby-jp-cyc0.txt`, frames 2 to 1 201 — the boot window, written but never analysed by the third
lot) through `drift.py`, and that is where item 10 came from. **A ledger already taken is worth
reading before a new one is taken.**

Built by NESHAWK-PROFILE-3, because the census could say what a frame's DMA cost and not **what
either machine was doing on a given cycle**.

### `frame-trace --event-log <file> [--event-pc <addr>]…`

The **cycle ledger**: one `F` line per frame carrying the CPU cycle the frame started on, its
length, its instruction count and its DMA bill, then one line per event with the cycle it happened
on *counted from the frame's start* — every NMI and IRQ handler entry, every `--event-pc`, and
(with `--dmc-census`) every `$40xx` access, the `$4016` strobe among them.

    zax-harness frame-trace <movie> <rom> --frame 1100 --frames 8901         --event-log ledger.txt --event-pc C06E --event-pc C37C

It captures no instruction and disassembles nothing, so 8 901 frames cost **19 s**; `--out` and
`--watch` are refused with it rather than served empty. **The format is a contract with the other
machine**: `bk2cyc1.lua` writes the same file from BizHawk, so the two can be aligned event for
event, and `frame_trace.rs`'s `the_cycle_ledger_writes_the_format_the_other_machine_is_read_in`
pins it so that changing it is visibly also a change to that script.

Two calibrations the alignment needs, both **measured and not assumed**:

* the two instruments report an *execute* at the cycle the instruction begins on and a *read* or a
  *write* **one cycle later**. With the +1 applied, `r$4015 → w$4011` comes out at exactly 0 on
  **75 of 75** occurrences of a 101-frame window — which no wrong value could do;
* a frame's own `start` is **not** comparable (BizHawk ends a frame on a PPU condition, the tracer
  notices the boundary after the instruction that crossed it). What is exact is an event's
  *absolute* cycle, and within a frame the cycles between two events. So each frame is anchored on
  its own NMI handler entry.

### The interrupt-phase ROM

`crates/zax-nes/tests/neshawk_profile_tests.rs`'s `phase_program`: a `LDA $00` / `BEQ` spin loop
and an NMI handler that records the low byte of the **interrupted** PC, in two variants that differ
in one bit — whether the taken branch crosses a page. The loop's phase relative to the vblank walks
by itself (a frame is 29 780 or 29 781 cycles, the loop's period 6 or 7), so 64 frames sweep every
phase with no delay slide to write.

    cargo test -p zax-nes --test neshawk_profile_tests interrupt_phase -- --nocapture
    ZAXNES_DUMP_ROM=phase.nes cargo test -p zax-nes --test neshawk_profile_tests         the_interrupt_phase_rom_is_written -- --ignored

The second command writes the `.nes` **BizHawk is then given**, so the two machines answer the same
bytes rather than two copies of the program; `phaserom.lua` reads the tables back. It is what
eliminated the branch quirk.

**What it cannot do**, recorded by NESHAWK-PROFILE-4 so that nobody spends a round on it again:
it cannot settle a question about the NMI *rule*. All three tables — NesHawk's, `Standard`'s and
`NesHawkCompat`'s — are the **same cyclic pattern** at different offsets (NesHawk's is this core's
rotated by 15 frames of the 18-frame period, and the two profiles' are two other rotations), so
what it measures is the loop's **phase**, which the boot sets. That is why NESHAWK-PROFILE-3
compared only its *shape*, and the shape is all it has.

## How to add an item — the whole recipe

1. **Read it, and write down where.** The item goes in the progress note of your lot with the
   NesHawk file and function it came from and what the lines mean. An item nobody can check is not
   an item.
2. **Add one associated constant** to `TimingProfile` in `crates/zax-nes/src/timing_profile.rs`,
   with the doc comment carrying the source; `false` (or "as today") on `Standard`, the NesHawk
   value on `NesHawkCompat`.
3. **Read it at the fewest sites you can**, each as `if P::YOUR_ITEM { … } else { … }` where the
   `else` arm is the existing code **unchanged**. That is what makes the standard profile free: the
   constant folds and the `else` arm is all that is emitted. Do not add a runtime field that the
   standard profile has to test.
4. **New state goes in the struct, and in the savestate only under the profile.**
   `DmcChannel::reload_countdown` is the example: `if P::DMC_LENGTH_LAGS_THE_FETCH {
   writer.write_i32(…) }`, so a standard block stays byte for byte what it always was and
   `NesCore::STATE_VERSION` does not move.
5. **If the state changes between cycles the APU may batch over**, make `quiet_clocks` answer 0
   while it is pending — again behind the constant. Forgetting this is the one error that produces a
   difference only on some games.
6. **Write a differential test** in `crates/zax-nes/tests/neshawk_profile_tests.rs`: measure the
   same thing on both profiles and assert the gap the item predicts. A test that pins only the
   NesHawk number cannot tell a regression of the standard profile from a change of yours.
7. **Re-run the two suites that say you did not break the standard path**, and no others count:
   * `golden-core` over the originals must stay
     `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773`;
   * `test-roms --profile standard` over `dmc_dma_during_read4`, `sprdma_and_dmc_dma`, `apu_test`,
     `dmc_tests` and `AccuracyCoin` must be unchanged — 24 pass, 6 manual, 128/147.
   Then record what `--profile neshawk` scores and which item moved it — and then do the one thing
   NESHAWK-PROFILE-4 added, because it is what would have caught item 8: **run the public ROMs
   that measure the quantity your item is about on NesHawk itself.** If NesHawk passes one that
   your item makes the profile fail, stop. The item is a compensation and the difference is
   somewhere else.
### Where the suites live, and the one invocation trap

Written down by NESHAWK-PROFILE-10 because finding them cost it time:

| | |
|---|---|
| `golden-core`'s 98 originals | `G:/3dsen/test-originals` — **not** `demo/roms`, which is the 18 of `ZAXNES_ROMS` |
| `test-roms` | `G:/3dsen/cleanroom/assets-free/test-roms` (331 ROMs, ~3 min a profile with `--no-shots --quiet`) |
| *Faxanadu* | movie `G:/3DSen/test-movies/bk2/invariel-faxanadu.bk2`, ROMs `Faxanadu [USA][!!].nes` and `Faxanadu [USA][Rev 1][!!].nes` in `G:/3DSen/test-movies/roms`, held against `bizhawk-runs/fax-usa.txt` and `fax-rev1.txt` with **`--trace-ref`** (not `--ram-ref`) |
| the bench ROMs | `Super Mario Bros. 3 [USA][!!].nes` and `Mega Man 3 [USA][!!].nes` in `G:/3dsen/test-originals` |
| the whole suite | `cargo test --release --workspace --exclude zax-app --exclude zax-platform` — those two do not **link** here (`LNK1181: SDL2.lib`), and without the excludes the run aborts and reports a short total |

**The trap**: the harness takes `G:/...`-style paths and a `/g/...` MSYS path for the *movie or the
ROM* fails with `The system cannot find the path specified. (os error 3)` — even though `--ram-ref`
and `--trace-ref` accept either. Use `G:/` for every argument and the question does not arise.

8. **Measure the movie.** `movie-sync --profile neshawk` with `--ram-ref` against a per-frame
   reference, and `--profile standard` beside it so the two columns are from the same command.

### Twelve traps that cost these five lots time, so they are written down

* **Every `timer % 2` from NesHawk's source must be inverted.** Its DMC timer is this core's plus
  one: `timer_reload = DMC_RATE[i]` reloading to 428 against this core's `rates[i] - 1` reloading
  to 427, both read after the same cycle's clock. Item 3 shipped with the source's own text and the
  wrong convention for a whole lot.
* **The arithmetic does not decide; the measurement does.** Twice now an item has been sized in
  advance, found far too small, and closed the frame anyway: item 1a against a "wrong sign" of 45
  to 55 cycles at frame 926, and item 1c worth **six** cycles in a frame whose miss was measured at
  **about two hundred and twenty**. What these items move is *which cycle* each fetch lands on and
  therefore the interleaving of the whole frame, which is not a sum over fetches. Size an item to
  decide what to read next, never to decide whether to implement it.
* **A differential test of one item can be an instrument for another.** Item 1b's test measured
  slips 1 and 2 and asserted `[5, 5]`; item 1c made slip 1 cost 4, so the test had to move to the
  **even** slips 2 and 4 to go on testing item 1b's own claim. That the test failed is the system
  working: hold each item's test to a condition the *other* items do not vary. (With item 1c
  retracted every slip answers 5 again, and the test stays on the even slips so that it and
  `the_reload_costs_four_cycles_at_every_slip_item_1c_retracted` keep failing for different
  reasons.)

* **Three readings of the source have now lost to a measurement of the program, and all three were
  verbatim.** The branch-interrupt quirk, item 8 and item 1c. Each was read off the 2.11.1 tag,
  re-checked, and wrong about what NesHawk *does* — item 1c because `RunCpuOne`'s closing
  `cpu.RDY = true` undoes the `RDY` the request cycle lowered, which is three lines below the two
  the derivation quoted. The lesson is not "do not read the source": it is that a reading
  predicts and a measurement decides, and **the experiment is usually cheaper than the third
  re-read**. Item 1c's cost two lots of a profile fitted around it.

* **Measure the quantity in the frame where the two machines are still in step.** Item 1c's
  retraction was nearly missed because the stall-by-slip table *disagreed with itself*: slip 1
  came out 4 six times and 3 ten times. A slip is read off this core's own census, so it means
  something only while the two CPU streams are aligned — and they are aligned only up to the
  **first** fetch the two machines charge differently, after which the stream drifts and every
  later slip in that frame is read against the wrong cycle. Frame 11 is the one frame known to
  *start* in step; its first slip-1 fetch is the only trustworthy row in the table, and it says 4.

* **An item whose two arms are the same function is not an item.** NesHawk expresses an OAM
  transfer's alignment on the parity of the DMC's timer and this core on the parity of the cycle,
  and the two are **the same predicate on every reachable state** because every one of the thirty-two
  DMC rate values is even and the timer's parity is therefore locked to the cycle's from power-on.
  The tell was that no differential test could be written for it: a test whose two columns are
  provably equal cannot fail, so there is nothing to ship. Ask that question *before* writing the
  constant.

* **An item can be real on one movie and not be a difference at all.** Item 8 took *Kirby* 8 968
  frames further and made 213 frames agree to the byte where none had, and it is **not** a NesHawk
  difference: the public ROM that asks its question directly, `ppu_vbl_nmi` `05-nmi_timing`, is one
  NesHawk passes. An item that moves a movie is evidence that *something* of that size is missing,
  and no evidence at all about **what**. Before shipping one, find the public ROM that measures the
  quantity and run it on the other machine; if the other machine passes it and your item makes the
  profile fail it, the item is a compensation and the real difference is elsewhere. Here it was two
  numbers away — the boot, item 10 — and the compensation cost eleven ROMs for two lots.
* **An item can be exactly right and still not shippable.** Item 11 is NesHawk's own constructor
  line, it predicts the measured fetch cycle to the cycle, and it puts all thirteen fetches of the
  frame on NesHawk's phase where this core drifted 211 cycles out of it. It also takes the one
  movie the profile exists for from frame 8 005 back to frame 925 and improves nothing anywhere
  else, because the error it fixes was cancelling a second error of the opposite sign. **Correct in
  isolation is not the same as better**: measure an item against the whole movie before enabling
  it, and if it is right but costs, ship it *off* with the measurement and the gate written down
  rather than either forcing it through or deleting the knowledge. The constant, its note, its test
  and Q-182 are that shape.
* **When two errors cancel, fixing one makes things worse — and that is progress, not a setback.**
  The lots before this one tuned the DMC's fetch-cost items against a power-on phase that was 592
  cycles wrong. Each item was measured honestly and each helped, and some of what they were worth
  was compensation. Expect this of any item set fitted to one movie: when a newly-exact item
  regresses the metric, the right conclusion is that a *second* difference has just become visible,
  not that the item is wrong. The way out is to isolate the new residual to one event, as step 4
  did, rather than to re-tune.
* **A suite figure is only as good as the invocation that produced it.** `cargo test` **aborts the
  remaining targets** when one fails, so a lot that reports a single total can report a short one
  and call it a pass: NESHAWK-PROFILE-4's "1 142 passed, 0 failed" was a target that had already
  failed plus the targets that never ran. Grep the output for `failed` and for `FAILED`, and sum
  the per-target `test result:` lines, rather than reading the last one.
* **A ledger already taken is worth reading before a new one is taken.** Item 10 came out of
  `kirby-jp-cyc0.txt`, frames 2 to 1 201, which lot NESHAWK-PROFILE-3 wrote as a by-product and
  never analysed. Its first six lines carry the whole answer.
* **A reading can be wrong and the measurement is what says so.** NesHawk's `branch_irq_hack` is set
  for every taken non-page-crossing branch and `Fetch1` skips the interrupt test on the instruction
  after it — read verbatim from the 2.11.1 tag, three times, in three phrasings. It predicts that
  NesHawk can never take the NMI at the boundary after such a branch. It does, on 20 of 64 frames of
  a ROM written to ask. **Build the experiment before implementing what the source seems to say**;
  this one was forty lines of 6502 and it saved a round.

* **A quantity read off one machine and compared against the other is only valid while the two are
  in step.** Lot 6 retracted item 1c on a table of "the stall by slip" whose stall came from
  BizHawk's bus stream and whose **slip came from this core's own census** — and the two CPUs have
  not been in step *inside an instruction* since the movie's fourth frame, so a fetch this core
  calls slip 1 is one NesHawk calls slip 0 or 2. The same table, built entirely out of BizHawk
  numbers (the stall as an instruction's duration minus the shortest duration that PC is ever seen
  to take, the slip as the cycles carrying a `w` **and** an `r`), answers 705 fetches with no
  exceptions and says the opposite. **Before trusting a cross-machine table, ask which machine each
  column came from**; a column that can be computed inside one machine should be.
* **Find the FIRST difference before explaining the one you can see.** The movie's work-RAM
  reference says *Kirby* agrees to frame 45 095, and two lots read that as "the machines are in step
  until 45 095". They are not: they part at frame **55**, and the reference cannot see it because the
  difference is an extra pass of a controller read that returns the **same value**, and because
  Kirby's sound driver keeps its state in the cartridge's **PRG-RAM**, which `--ram-ref` does not
  compare at all. A window placed at the first *visible* difference was 45 041 frames downstream of
  its cause, and everything measured in it was a comparison of two unrelated events. The cheap fix
  is a whole-prefix ledger of the subsystem under suspicion — `q182e.lua` dumps the DPCM schedule of
  45 100 frames in four minutes — taken **before** a window is chosen.
* **One Lua callback per address, not one per access.** BizHawk's
  `event.onmemoryread/write(func, address, name, domain)` hooks the address you name; passing `nil`
  hooks every access and costs a Lua call for each. That is the whole difference between a 14-frame
  window at frame 45 000 and the movie's first 45 100 frames: `q182c.lua` could not reach frame 309
  affordably, `q182e.lua` reaches 45 100 in 4m 20s. Two lots' worth of "that window is unaffordable"
  was a registration argument.
* **A census column is an instrument, not a measurement.** `--dmc-census`'s `cost` is
  `fetch - halt + 1`, and the census dates the halt a cycle after the request on 113 of 1 524
  fetches — so 74 reloads read as costing **three** cycles where the CPU loses exactly the same
  four. Before calling a column a difference, check it against the other machine's own stream: all
  thirteen checked resume the CPU on the **same cycle** on both machines.
* **A hot entry point must be concrete per profile.** Rust monomorphises a generic function in the
  crate that *calls* it, and a generic body with one call site is inlined into its wrapper. A
  generic `run_frame` cost **+3.8 %** on Super Mario Bros. 3 for both reasons together, with no
  emulation changed. `NesCoreOf<Standard>::run_frame` and `NesCoreOf<NesHawkCompat>::run_frame` are
  therefore declared separately, each a one-line forward to an `#[inline(never)]` generic body. A
  new hot entry point needs the same treatment; a cold one does not. `nes_core.rs` carries the
  measurements beside the code.
* **A default type parameter does not help callers.** Rust applies a type parameter's default when
  it resolves a type and **not** when it resolves a path through one, so `NesCore::STATE_VERSION`
  and `NesCore::new()` both stop compiling behind `NesCore<P = Standard>`. The generic structs are
  therefore `NesCoreOf<P>` and `NesBusOf<P>`, with `pub type NesCore = NesCoreOf<Standard>` and
  `pub type NesBus = NesBusOf<Standard>` — plain aliases to a concrete instantiation, which resolve
  everywhere. That is why not one line outside `zax-nes` had to change.

* **A residual of −1 between the two instruments is the two machines *in step*, and the proof is
  inside BizHawk.** `emu.totalexecutedcycles()` is a **1-based** count of the cycles that have run,
  where `frame-trace`'s `CYC` is the **0-based** index of the cycle about to run — so the same
  physical cycle is `N` here and `N + 1` there, and the parity of a cycle is *opposite* in the two
  counts. NesHawk's own OAM rule is what pins it: `cpu_deadcounter` is 2 when `apu.dmc.timer % 2 ==
  0`, the timer powers on at 1 020 and every rate is even, so that parity is the parity of the
  cycles **elapsed** — and the first transfer's leading dead cycle lands where that makes it.
  Without this, a one-cycle counter convention and a one-cycle difference in the boot are
  indistinguishable, and lot NESHAWK-PROFILE-9's whole round turns on which of the two it is.
* **A sweep in steps of three never varies the sub-cycle phase, and four lots' worth of "the
  residual is not a phase constant" rested on sweeps that did exactly that.** `PPU_START_UP_DOTS_SKIPPED`
  moves the PPU against the CPU **in dots**, and a CPU cycle is three dots — so lot 7's 7 156…7 174
  in steps of 3 and lot 8's reset biases 0, 3, 6, 9, 12 are **twenty values in one residue class
  mod 3**, every one of them a whole-cycle move. NESHAWK-PROFILE-10 swept the thirteen values
  7 158-7 172 at **one-dot** steps and the metric is sharply peaked: 7 163 takes the four marks'
  in-step frames from 20.0 % to **42.0 %** of 9 336 and the spin-loop residue from 71.9 % to
  **90.2 %**. It is still refused — no reading, and the wall does not move — but the *question* had
  never been asked. **Before concluding that a knob is not the answer, check that the sweep's step
  can reach the values that matter**; and note the second half, that the constant is applied at the
  power-on **and** at the reset, so every value of it gives an **even** net offset and ±1 dot needs
  a second constant.

* **The byte-identical count can be moved eighteen-fold by a constant with no reading and no effect
  on the parting frame.** `PPU_START_UP_DOTS_SKIPPED = 7163` takes `step1c` from 237 frames
  identical to the byte to **4 392** of 8 901 and costs `step1` 98 of its 318, leaving the parting
  frame at 9 353 exactly. So that column measures *when* each machine wrote the RAM down as much as
  whether the two ran the same frame, and it is not a safe target to optimise against. **The
  parting frame and the first differing instruction are the figures that survive a knob.**

* **An inert result needs a control before it is a result.** A post-reset-only dot bias of -2, -1,
  +1 and +2 gives *byte-identical* answers — the same 1 866 in-step frames, the same first
  disagreeing frame — which is only evidence once a bias of **+300** has been shown to take the
  same metric to **0 of 9 336**. Without that the four rows are indistinguishable from a patch that
  did not apply, and NESHAWK-PROFILE-10 nearly recorded them as one. The cause is real and lot 8
  named it: the post-reset boot's 7-cycle `$2002` wait re-quantises any bias smaller than a loop
  iteration.

* **A differential test can be pinning a *different* item's cycle.** Item 7's test took the base
  cost of an OAM transfer from the **`Standard`** profile and asserted NesHawk was two cycles above
  it — which reads as "an inside fetch costs NesHawk one cycle more than this core". It was **item
  13's** cycle: the two profiles' transfers are 513 and 514 on opposite phases of the `$4014` write,
  so a base taken from the other profile is the wrong figure by one. Measured against each
  profile's **own** transfer at the same phase, an inside fetch costs **both** profiles two cycles
  (for different reasons: `Standard`'s fetch may only read on a get cycle, so the pair that follows
  moves by two), which is exactly why lot 2 measured item 7 *inert* over 81 frames. A test whose
  base comes from the other column measures the sum of every difference between the columns.

## Where the profile is chosen, and the one place to change it

| | |
|---|---|
| the rule | `MovieFile::recorded_profile` in `zax-harness/src/movie_file.rs`: a `.bk2` whose `Core` header is exactly `NesHawk`, and nothing else — not `QuickNes`, not an `.fm2`, not a header with no `Core` line |
| the override | `--profile standard\|neshawk` on `movie-sync`, `frame-trace` and `test-roms`, through `zax-harness/src/profile_choice.rs`. An unknown value is a usage error, never a silent fall-back |
| where it is read | `movie_sync.rs` `zaxnes_machine` (one `match`, then a `Box<dyn Machine>`) and `frame_trace.rs` `run` → `trace_on::<P>` |
| in a report | `ZaxnesMachine::name()` answers `zaxnes` or `zaxnes/neshawk`, so no figure can be mistaken for the other chip's |
| in a savestate | `TimingProfile::STATE_CORE_NAME` — `"Zaxnes"` or `"Zaxnes-NesHawk"` — at the head of the block, so a block of one profile is refused by the other with the message REQ-CORE-120 fixes, and `STATE_VERSION` never moved |

**The application cannot open a `.bk2` at all** (`grep -rn bk2 crates/zax-app/src/` finds nothing;
the movie dialog admits `.fm2`, `.fcm` and `.mv`), so there is nothing in it that selects a profile
today. The day it gains a `.bk2` importer, `Engine` is already implemented for both profiles and the
rule above is one function call — and the switch must be **for that playback only**, never written
to the game's own engine setting, and said on screen, which is what lot FCEUX-48 settled for `.fm2`.

## The cost of having two chips

| | |
|---|---|
| speed, standard profile | **NESHAWK-PROFILE-9: −3.0 % on Super Mario Bros. 3 and −2.8 % on Mega Man 3 against `428c50e6` itself, three interleaved rounds of 600 frames** — and +9 728 bytes of binary for three more items and the const-generic frame entry. Within the +1 % budget all five earlier lots were held to **cumulatively**: NESHAWK-PROFILE's `bench-core` on five ROMs read -3.0 %, -3.0 %, -2.2 %, +0.5 %, +0.6 % against its parent, NESHAWK-PROFILE-2's on Super Mario Bros. 3 and Mega Man 3 read **-2.24 %** and **-1.04 %**, NESHAWK-PROFILE-3's read **+0.77 %** and **+0.40 %**, NESHAWK-PROFILE-4's **-1.25 %** and **+0.27 %**, NESHAWK-PROFILE-5's **+2.68 %** and **+1.24 %** against its own parent, and NESHAWK-PROFILE-6's **-0.86 %** and **-0.35 %** against `428c50e6` itself over three interleaved rounds, with `golden-core` byte-identical throughout. **Cumulatively since `428c50e6`, the parent of the last five lots: -0.86 % and -0.35 %** — the whole series is still faster than it started. **The amplitude of the last two lots' figures, in opposite directions, is the honest measure of what this bench can resolve**: neither lot changed a per-frame path at all, so both numbers are code layout on a machine that drifts within a run (`428c50e6`, unchanged, rose 2 % across NESHAWK-PROFILE-5's own six rounds). The cumulative figure against a fixed reference is the one to trust |
| speed, standard profile — **NESHAWK-PROFILE-10, over the budget and flagged** | **+2.32 % on Super Mario Bros. 3 and +1.43 % on Mega Man 3**, six interleaved rounds of 600 frames against the lot's own parent `08a881b8` — three with the items-off binary first and three with it second, which is the control the amplitude needed. **Reported as over the +1 % budget rather than absorbed**, and the evidence that it is layout and not emulation is unusually strong: the order control *inverts the ranking* (Super Mario Bros. 3 is +3.83 % off-first and +0.82 % on-first, Mega Man 3 +0.32 % and +2.54 %), **one binary's own spread over its six rounds is as large as the difference** (Mega Man 3's items-off binary ranges 0.8608-0.8848 ms, +2.79 %, against a measured +1.43 %), and the two *deterministic* instruments both say nothing changed — `golden-core` is byte-identical and **the whole source diff is three `const … = false` → `true` inside `impl TimingProfile for NesHawkCompat`**, a profile `bench-core` never instantiates. What moved is where `NesCoreOf<NesHawkCompat>`'s three newly-live arms put `NesCoreOf<Standard>`'s hot code. An owner who wants this inside +1 % should ask for a **layout** round, not a re-measurement |
| behaviour, standard profile | `golden-core` over 98 ROMs x 1 200 frames **byte-identical** in all ten lots (`8d508070...`); `AccuracyCoin` 128/147 unchanged; 24 of 24 DMC/DMA ROMs unchanged; `ppu_vbl_nmi` 11/11, `vbl_nmi_timing` 7/7 and `cpu_interrupts_v2` 6/6 unchanged; `test-roms --profile standard` 212 pass / 7 fail unchanged; both *Faxanadu* dumps 207 and 220 unchanged |
| memory | none: a profile is a zero-sized type, and the two cores are the same size |
| binary | **+147 408 bytes of `.text`** (+3.0 %) for a whole second CPU, bus, APU and DMA — a fourteenth of the 2 MB that would have put it behind a feature, so it is behind none |
| savestates | unchanged for the standard profile, to the byte and to the version |
| tests | `zax-nes` passing with the ignored ones, of which **fourteen** of `tests/neshawk_profile_tests.rs`'s are the items' own differential tests and six the architecture's in `tests/timing_profile_tests.rs`. NESHAWK-PROFILE-10 **deleted the `ItemsWaiting` test profile**: it was `NesHawkCompat` plus the three waiting items, so with them on it *was* `NesHawkCompat`, and the three tests measure the shipped profile directly as item 11's did when it shipped. A test profile that stands in for a waiting item must go when the item ships, or it silently stops testing anything. The whole suite is **2 500 passed, 0 failed, 88 ignored** over 144 targets, summed per target rather than read off the last line. The earlier lots' "eleven" targets' worth are the items' own differential tests in `tests/neshawk_profile_tests.rs` and six the architecture's in `tests/timing_profile_tests.rs`. The earlier lots' "1 142" was short: `timing_profile_tests.rs`'s no-DPCM invariant **had been failing since NESHAWK-PROFILE-4 added a PPU item** and the aborted target took the rest of its count with it, which NESHAWK-PROFILE-5 step 3 found and fixed — the test now asserts the two profiles differ by **exactly** item 10's start-up frame, so it still fails the moment a DMC item reaches the common path |
