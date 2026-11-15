# HALF-CYCLE-FULL — lot A-bis (proposal, 2026-11-15, awaits the owner)

FIDPLUS-A put the PPU and the CPU-PPU register edges on their measured master half-cycle (0 dots
of difference against the 2C02 netlist, AccuracyCoin 147/147 in the four alignments). The CPU, the
APU, the DMAs and the mapper's M2 still advance one CPU cycle at a time: their effects are right to
the cycle, but not proved to the half-cycle. A-bis would run the whole machine on the master clock
in Fidélité+, as BeesNES does, and judge the CPU and APU against netlists as the PPU was.

## BeesNES's shape (MIT, read for structure, `ext-tools/bees-src`)

* `CSystem::Tick` (`Src/System/LSNSystem.h:147`): real time converted to a master counter, then a
  ring of hardware slots ticked in counter order, each slot advanced by its own increment (CPU
  phases, PPU dots, APU).
* The CPU is split in two updates, `Tick` (PHI1) and `TickPhi2` (`Src/Cpu/LSNCpu6502.h:230`): the
  address and RDY / DMA halt checks on PHI1 (`LSN_INSTR_START_PHI1`), the data on PHI2; IRQ and
  NMI latched on PHI1 flags (`m_bIrqStatusPhi1Flag`, `LSN_CHECK_INTERRUPTS`).
* The APU is ticked as one slot per CPU cycle ("assuming the APU is not divided into PHI1 and
  PHI2").

## What A-bis would add to Zaxnes

| Piece | Today | A-bis |
|---|---|---|
| CPU | one bus access per cycle, PPU caught up around it | PHI1 / PHI2 halves: address out on PHI1, data on PHI2, RDY / DMA halt sampled on its half |
| IRQ / NMI | polled per cycle at the documented point | latched on the PHI1 / PHI2 edge the 2A03 netlist shows |
| OAM / DMC DMA | per-cycle get / put model (measured against blargg and AccuracyCoin) | halt, alignment and put / get cycles on their halves; DMC-OAM conflicts at the half-cycle |
| APU | one step per CPU cycle | unchanged (BeesNES keeps it whole too) unless the 2A03 netlist shows a half-cycle effect |
| Mappers | clocked per CPU cycle | M2 rise / fall visible to the boards that use it (MMC5's, the FDS's, VRC IRQ prescalers) |

Placement: Fidélité+ only (it costs speed), through the compile-time `Fid<K>` copy so the standard
core is untouched (lesson of FIDPLUS-A, memory `optional-path-inlining`).

## Judge

* A 2A03 netlist differential, as `fidjudge.sh` does for the 2C02: CPU bus traces (address, R/W,
  data, SYNC, RDY, M2) of every test program run on both, zero difference in every frame.
* blargg's CPU, APU and DMA suites, AccuracyCoin (147/147 x4 kept), `dmc_dma_during_read4`,
  `sprdma_and_dmc_dma`, the campaign and the golden equal to the standard core's where the
  standard core is right.

## Plan and size

1. Bench: a 2A03 netlist runner (the visual6502 2A03 netlist, its own licence checked first),
   trace format, first diff on a few programs. 1-2 days.
2. CPU PHI1 / PHI2 in the Fidélité+ copy, IRQ / NMI on their edges. 1-2 days.
3. DMAs on halves. 1 day.
4. M2 for the boards that need it. 1 day.
5. Validation and perf (the Fidélité+ frame cost measured, the standard core within ±0.01 %).

Several days in all; the owner chooses whether it comes before or after FIDPLUS-C and D.
