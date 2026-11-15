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

## DMC and OAM DMA, step 4: two power-on phases, then 0 cycles (2026-11-15)

* `oam0` / `oam1` (OAM DMA started one cycle apart): the netlist takes 513 and 514 cycles, Zaxnes
  514 and 513. Together with step 3 this is one thing: **the CPU / APU get-put parity after
  reset is the other one in Zaxnes**. `cpu-bus-trace --flip-get-put` (`NesBus::
  diagnostic_flip_get_put`: the bus's cycle parity and the APU's together, as the one clock they
  are on the chip) gives the netlist's parity: OAM DMA 513 / 514 and the DMC load halts land
  exactly where the netlist's do.
* The DMC output unit's power-on phase (`--dmc <timer>,<bits>`) is the second free value: a scan
  of the timer (380-470) and the bit counter (1-8) over `dmc`, `dmc0` and `dmc1` together finds
  one value, **timer 431, 8 bits, that makes all three identical** (3 x 2 990 cycles), and nothing
  else near it.
* With those two settings (and the PPU's phase of step 1), the whole corpus, every cycle:

| Program | Cycles | DMA cycles (Zaxnes) | Differing cycles |
|---|---|---|---|
| `loop`, `modes`, `allops`, `branch` | 3 990 each | 0 | 0 |
| `dma`, `oam0`, `oam1` | 3 999 each | 513 / 513 / 514 | 0 |
| `nmi` | 29 990 | 0 | 0 |
| `irq` | 59 990 | 0 | 0 |
| `dmc`, `dmc0`, `dmc1` | 29 999 each | 272 / 274 / 273 | 0 |

  The DMC programs' DMAs land on reads, on writes (the halt waits for the next read) and inside
  read-modify-writes (`INC abs`), with 3- and 4-cycle stalls; the netlist's stalls repeat the
  CPU's read as Zaxnes's model says (`0202R 0202R 0202R C000R`).
* Neither value is a model difference: both are states the console leaves to power-on. The
  netlist simulation sets them at the end of its reset; whether a console's get / put parity is
  random at power-on (as the CPU / PPU alignment is) or fixed by the reset is not settled here. A
  console-model choice "CPU / APU alignment" (random, either) would cover both; recorded as
  FIDELITY-LEADS G5.

## Step 5: the hard cases (2026-11-15)

`mkprog3.py`; same settings as step 4 (alignment 3, phase -8, `--flip-get-put`, `--dmc 431,8`).
`cmp2.py` splits the comparison: cycles with another address or R/W (timing), and cycles where
only the byte differs.

| Program | What | Cycles | Timing differences | Byte-only differences |
|---|---|---|---|---|
| `dmcoam` | DMC fetches colliding with OAM DMAs every loop | 29 999 (29 237 DMA) | 0 | 0 |
| `dmc2007` | `$2007` reads halted by DMC fetches | 29 999 | 0 | 0 |
| `illegal` | every stable illegal opcode in every mode | 3 990 | 0 | 0 |
| `dmc4016` | controller strobes and reads under DMC fetches | 29 999 | 0 | `$4017` reads `$10` on the netlist (VisualNes has no controller and floats the bus, step 6); 7 `$4016` reads right after a fetch (G6) |
| `intbranch` | frame IRQ and NMI against taken branches | 59 990 | 0 | `$2002` bit 5 (sprite overflow from the PPU's power-on OAM); `$4015` on the pins (below) |
| `fc5` | `$4017` five-step then four-step with IRQ | 59 990 | 0 | `$4015` on the pins (below) |

`$4015` is read inside the 2A03 and does not drive the pins, so the bench, which prints the pins,
shows the open bus there (`$40`) while Zaxnes prints what the CPU got (`$00` or `$40`); the
programs store each value read, and those stores are identical on both sides.

**Across all 18 programs and 353 000 cycles, no cycle has another address or another R/W.**

## Step 6: the DMA cycles themselves, and the controller ports (2026-11-15)

* Feature `bus-trace` (`zax-nes`, `zax-harness`, never in `default`; commit `bf1c75f5`): the bus
  records every access a DMA unit makes (halt and idle re-reads, DMC fetch, OAM get and put), and
  `cpu-bus-trace` prints each stolen cycle (`D`, or `I` for an idle re-read the controller does not
  see) instead of one `DMA <n>` line. Without the feature the hook is an empty inline function.
* `cmp3.py` compares every cycle, DMA included. With the settings of step 4, **all 18 programs,
  421 901 cycles of which 32 131 are DMA cycles: 0 timing differences, and 0 byte differences on the
  DMA cycles** (`final5.sh`). Byte-only differences stay where step 5 found them.
* The controller ports: VisualNes has no controller hardware; any read in `$2000`-`$7FFF` that is
  not a chip floats the external bus (`Core/macros.cpp`, `mCpuRead`, then `floatBits`). Its
  `$4016` / `$4017` bits are therefore the switch-level simulation of a floating bus, not a
  measurement. Program `g6jmp` (`mkg6.py`: opcode fetched from `$4015`, operand from `$4016`)
  shows the netlist taking the open bits from the pins (operand `$40`, branch to `$4057`) where
  Zaxnes takes the internal bus (`$00`); `AccuracyCoin`'s `Implied Dummy Reads`, verified on
  hardware, expects the internal bus in that very situation (`[Read $4016: 21]` for every implied
  opcode with bit 5 set, `$E1` by the pins for `$EA`). So the netlist is no judge for these bits,
  and G6 stays as recorded in `FIDELITY-LEADS.md`.
* `sweep.sh`: the PPU phase gives 0 differences on `nmi` for -12 to -4 and 14-17 outside (the NMI
  taken one instruction earlier or later); without `--flip-get-put`, `oam0` 2 775 and `dmc0`
  27 861 timing differences; without `--dmc 431,8`, `dmc` 27 836.

The owner's proof document built from this lot: `docs/review/SUBCYCLE-PROOF.md` (in French), traces
in `docs/review/subcycle-proof/traces.zip`.

## Meaning for A-bis

Nothing measured needs the CPU on half-cycles: every bus cycle the CPU and the DMA units make is the
netlist's, and the remaining differences are power-on states and the bench's floating bus. A-bis
stays a proposal without a measured case to fix.
