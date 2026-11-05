# Lot MF-4a-ter — trimming the OAM DMA bookkeeping

MF-4a-bis gave back everything MF-4a cost on the cycles that run **no** DMA: `Apu2A03::quiet_cycles`
and `run_cpu_cycles` byte-identical to `b39eaba4`, every `Cpu6502` function back to its baseline
instruction count to the instruction. What it did not give back is the cost of the stolen cycles
themselves. R0's measurement of `d4151eae`:

| game | stage | `b39eaba4` | `d4151eae` | delta |
| --- | --- | ---: | ---: | ---: |
| Mega Man 2 | core | 7 639 627 407 | 7 648 576 037 | +8.9 M (+0.12 %) |
| Zelda II | core | 7 783 867 550 | 7 795 720 235 | +11.8 M (+0.15 %) |
| both | PPU | — | identical | 0 |

of which the DMA machinery is **`run_dma` +8.4 M, `dma_read` +2.7 M, `write_internal` −3.2 M ≈
+7.9 M** on both games. One OAM DMA per frame, 300 frames, 513-514 cycles each ≈ 154 k stolen
cycles: about **50 instructions per stolen cycle** more than the old `$4014` loop. That is this
lot's target; the read-cycle gate (`read_cycle` +36.3 M against `tick` −36.6 M) and the DMC phase
(`run_cpu_cycles`) are out of scope.

## Steps

1. The DMC's fetch horizon: the cycle its memory reader could next ask for a byte, computed from
   the channel's own timer and bit counter.
2. The fast path: an OAM transfer no DMC fetch interrupts copies its bytes in a tight get/put
   loop, with the exact state machine taking over from the cycle a request can appear.
3. Measured — callgrind on clean builds from `git archive`, and the generated code of the
   functions the brief pins.
4. The suites end to end.

## Step 1 — the DMC's fetch horizon

The fast path of step 2 skips the DMA state machine, and with it the per-cycle question "has the
DMC asked for a byte?". What makes that safe is a bound, not an assumption: the number of CPU
cycles before the memory reader *could* ask.

`DmcChannel::cycles_until_fetch` (`apu/dmc_channel.rs:98-137`) computes it from the channel's own
state, in one line of arithmetic and no loop:

* `bytes_remaining <= 0` → **`None`**, "never". And it is a claim about the future, not only about
  now: `bytes_remaining` grows again only through a `$4015`/`$4013` write or through
  `supply_byte`, and a halted CPU writes nothing while a DMA that takes no fetch supplies nothing.
  That is the whole of the common case — a game playing no DPCM gets the whole transfer in one
  batch off one test.
* `buffer_empty` → **0**, "now" (this state is `needs_sample_fetch`, which the bus has already
  turned into a held request).
* otherwise the reader asks on the cycle the output unit takes the **last** bit out of the shift
  register and finds only the sample buffer to refill it with: `timer + 1` cycles to the next bit
  — the timer counts down once per CPU cycle and the bit plays on the cycle it is already 0 on —
  and a whole period for each of the `bits_remaining - 1` bits after it.

`Apu2A03::cycles_until_dmc_fetch` (`apu/mod.rs:432-443`) passes it on. The count is measured from
the **APU's** position, not the bus's: on a quiet cycle the DMC's timer has not been counted down
yet, so a caller whose APU lags has to take `apu_lag` off the answer. Step 2 does.

### Pinned in both directions

`req_core_113_the_dmc_fetch_horizon_names_the_cycle_the_reader_asks_on`
(`tests/dmc_dma_timing_tests.rs:53-101`) takes a sample's load fetch, asks the horizon at each of
sixteen phases of the timer, and then:

* runs `horizon - 1` **write** cycles — which a DMA may not halt — and a read cycle: the read runs
  plainly, `dmc_dma_count` stays at 1. A horizon one cycle **too short** fails here;
* runs `horizon` write cycles and a read cycle: that read is halted and the fetch runs,
  `dmc_dma_count` is 2. A horizon one cycle **too long** fails here.

So the bound is exact, not conservative, and the two failure modes it could have are each an
assertion. The second test pins the `None` answer across a `$4015` start and stop.

Nothing per-cycle changed: both functions are new, `const`, and called from nowhere yet.
`cargo fmt --check` clean, clippy clean (**no new `allow`**), `cargo test --release -p zax-nes -p
zax-harness` **692 passed, 0 failed, 7 ignored** — MF-4a-bis's 690 plus these two tests. No
`unsafe`, no savestate field.

## Status

- [x] step 1 — the DMC's fetch horizon
- [ ] step 2 — the fast path
- [ ] step 3 — measured
- [ ] step 4 — the suites
