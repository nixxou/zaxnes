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

## DMC, step 2 (2026-11-15)

* The netlist's DMC timer is a 9-bit LFSR (`cpu_pcm_t0-8`), with `cpu_pcm_bits0-2` and
  `cpu_pcm_freq0-3` (the bench prefixes the 2A03's node names with `cpu_`). On `dmc.bin`, after the
  rate-15 write the bit clocks fall at 432, then every 54 cycles; the bit counter wraps at 810 and
  the fetch is at 813.
* The recorder first logged a halted read at the cycle its call began; a DMA runs inside the call
  before the read, so the read is now logged at the call's last cycle, the DMA before it.
* With that, the first fetch after the `$4015` write differs too: the netlist halts on the data
  read `$0200` (3 cycles after the write), Zaxnes on the operand fetch `$8019` before it (2
  cycles). Two free power-on phases are in play: the DMC output unit's (`--dmc <timer>,<bits>`) and
  the APU's get / put parity against the CPU (random per console). Next: a parity knob, a scan of
  both, then the DMA's own cycles (halt, dummy, fetch addresses) traced on the Zaxnes side, before
  anything is called a difference of the model.

## DMC, step 3: the load's halt (2026-11-15)

Programs `dmc0` / `dmc1` (`docs/progress/cpu-diag/`, built by `mkdmc.py`): the same DMC start with
the `$4015` write on one CPU cycle or the next (a page-crossing `LDA abs,X` costs one more), so both
parities are judged. Cycles counted from the write (W = 0):

| | netlist | Zaxnes |
|---|---|---|
| `dmc0` | +1 `801C`, +2 `801D`, +3 `801E`, **+4 `0200` halt**, +5 `0200` dummy, +6 `C000` fetch, +7 `0200`, +8 `801F` | +1, +2, **+3 halt (`801E`)**, +4, +5 fetch, +6 `801E`, +7 `0200`, +8 `801F` |
| `dmc1` | +1, +2, **+3 `801E` halt**, +4 dummy, +5 `C000`, +6 `801E`, +7 `0200`, +8 `801F` | +1, +2, +3 `801E`, **+4 halt (`0200`)**, +5, +6 fetch, +7 `0200`, +8 `801F` |

Both make the load DMA 3 cycles and resume on the same cycle; the halt lands one read apart, the
other way round for each parity. Zaxnes's delay rule (`DmcChannel::set_enabled`: 2 cycles when
`apu_cycle`, 3 otherwise, the Mesen 2 rule) is the inverse of the netlist's for this core's
parity. Inverting it alone makes Zaxnes's halt land where the netlist's does but adds an alignment
cycle (4-cycle DMA): this core takes the get / put parity from the bus's cycle count and the delay
from `apu_cycle`, and against the netlist one of the two relations is the other way round. A longer
netlist reset (+24, +48 half-steps) changes nothing: the netlist's APU parity is set by the end of
reset, not free. Open: trace the DMA's own cycles in Zaxnes (halt, dummy, alignment, fetch
addresses) and the OAM DMA's get / put parity against the netlist, then decide which relation to
correct; validate with blargg's `sync_dmc`, `sprdma_and_dmc_dma`, `dmc_dma_during_read4` and
`AccuracyCoin`. Bus-visible effect today: which address the stall repeats (open bus and
side-effect reads such as `$2007` / `$4016`), not the cycle count.

## Meaning for A-bis

Nothing measured needs the CPU on half-cycles: where the CPU differs, it is a timing value of the
DMC unit, which a per-cycle model can carry. The DMA bus contents (which cycles are halt, dummy
and fetch, and what a DMA does on a write cycle) still have to be traced on the Zaxnes side to be
compared.
