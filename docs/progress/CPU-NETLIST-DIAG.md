# CPU-NETLIST-DIAG — the CPU against the 2A03 netlist (owner's "diagnostic" instead of A-bis)

Owner, 2026-11-15: rather than the whole machine on master half-cycles (A-bis,
`docs/design/HALF-CYCLE-FULL.md`), measure first whether the CPU differs from the transistor
netlist at all.

## Tools

* Netlist side: `ext-tools/visualnes-bench/benchcpu.cpp` (outside the repo, VisualNes is for
  measurement only): VisualNes's 2A03 + 2C02, one line per CPU cycle printed on the falling edge
  of its phi2 from the address, R/W and data held during that phi2.
* Zaxnes side: `zax-harness cpu-bus-trace <program.bin> --cycles <n> [--alignment] [--phase]
  [--dmc <timer>,<bits>]`, every CPU read and write through a recording `CpuBus`; a DMA stealing
  cycles inside the bus shows as `DMA <n>`. RAM powered on at zero, as the netlist's.
* Programs at `$8000`, RESET `$8000`, NMI / IRQ `$9000`; each starts with `LDX #$FF / TXS` (the
  power-on stack pointer is undefined). The PPU's start against the CPU's (`--phase`) is the same
  free parameter as in the FIDPLUS-2 dot differential; netlist reset hold 4 half-steps with
  alignment 3, phase -12 to -4.

## Results (2026-11-15)

| Program | Cycles | What it covers | Differing cycles |
|---|---|---|---|
| `loop` | 2 990 | abs,X reads across a page, stores, INX/BNE, JMP | 0 |
| `modes` | 2 990 | ADC in every mode (page crossings), RMW, stack, JSR/RTS | 0 |
| `allops` | 3 990 | every legal opcode once, JMP indirect | 0 |
| `branch` | 3 990 | taken / not taken / page-crossing branches | 0 |
| `nmi` | 30 000 | BRK, NMI entry at vblank, RTI | 0 (phase -12 to -4) |
| `irq` | 60 000 | frame-counter IRQ, `$4015` read in the handler, RTI | 0 |
| `dma` | 3 000 | OAM DMA length and alignment (the DMA's own bus not traced on the Zaxnes side) | 0 |
| `dmc` | 12 000 | looping DMC sample (rate 15) under a mix of reads and writes | first fetch same cycle; later fetches ~10 cycles apart, same 432-cycle period |

So far the CPU makes the netlist's bus cycle for cycle: instructions, interrupts and their entry
cycle, OAM DMA. What remains open is the DMC: after the first byte, its fetches come about 10 CPU
cycles earlier in Zaxnes than in the netlist, at the same period. Setting the output unit's
power-on timer (0-427) and bit counter (1, 7, 8) by hand does not align them, so it is not only an
undefined power-on value; next: trace the netlist's DMC timer, bit counter and rate nodes around
the `$4010` write and the first output cycle.

## Meaning for A-bis

Nothing measured needs the CPU on half-cycles: where the CPU differs, it is a timing value of the
DMC unit, which a per-cycle model can carry. The DMA bus contents (which cycles are halt, dummy
and fetch, and what a DMA does on a write cycle) still have to be traced on the Zaxnes side to be
compared.
