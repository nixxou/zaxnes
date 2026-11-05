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

## Step 2 — the fast path

### Where the 50 instructions per stolen cycle go

`run_dma`'s loop is a state machine asked, on **every** stolen cycle, questions whose answers an
OAM transfer does not change:

| per cycle | what it is | true for a whole transfer |
| --- | --- | --- |
| `promote_dmc_wait(self.cycles + 1)` | has a held DMC request come due? | no request can be held |
| `(self.cycles & 1) != 0` | is this a get cycle? | get and put simply alternate |
| the four-arm `if`/`else if` chain on `dma` and `oam_steps` | which unit does what on this cycle | read, write, read, write |
| `dma_cycle`'s `DMA_HALT` and `DMA_DUMMY` tests | is a halt or dummy cycle still owed? | neither, `dma == DMA_OAM` |
| `dma_read(address, internal, &mut decoded_before)` | a DMA unit's read, with the 2A03's own decoder | a plain `read_internal` |

`dma_read` is the expensive one on its own: it is an out-of-line call with four arguments and a
frame of its own, and its exclusive 2.7 M over 300 frames is **35 instructions for each of the
76 800 source bytes** — a `!internal` test, a range check, a `read_internal`, two stores and a
prologue and epilogue around them. `run_dma`'s own 8.4 M is **55 per stolen cycle** for the rest of
the table.

### The change

`NesBus::copy_oam_pairs` (`nes_bus.rs:669-695`) is that table with every row answered once: a
`for` loop over byte pairs, `tick` / `read_internal` / `tick` / `sync_ppu` / `write_oam_dma`, which
is the shape the old `$4014` loop had. `NesBus::oam_fast_pairs` (`nes_bus.rs:612-667`) says how
many pairs may run in it, and `run_dma`'s loop (`nes_bus.rs:564-581`) enters it when four things
hold:

1. `self.dma == DMA_OAM` — nothing else is owed, so no halt, no dummy, no held DMC request;
2. `(self.cycles & 1) != 0` — the next cycle is a get cycle. When it is not, the state machine's
   own `else` arm runs the one alignment cycle and the next turn of the loop qualifies;
3. `(oam_steps & 1) == 0` — the transfer stands on a byte boundary, so a batch is whole pairs and
   `oam_byte` never has to cross in or out of the fast loop;
4. `fast`, asked **once** before the loop: `!internal` (the halted address is not in
   `$4000`-`$401F`, so `dma_read`'s 2A03-decoder half and `decoded_before` are dead and
   `input_port` is false) **and** `dma_oam_page != 0x40` (no source address is one of the
   open-bus `$4000`-`$401F` ones). The opcode fetch after a `STA $4014` is in PRG and the page is
   a RAM page, so this is every ordinary OAM DMA.

Anything else — a DMC fetch due, an odd cycle, a halted `$4016` read, a source page of `$40` — is
the exact state machine, untouched.

### Why the bound closes, and why `free / 2` and not `free`

The one thing the fast loop drops that could matter is `promote_dmc_wait`. A DMC request can only
appear from `poll_dmc_dma`, inside one of the `tick`s the loop itself runs, on the cycle the memory
reader runs dry — and step 1's horizon names that cycle. Call it `dry = cycles + free`. The request
raised there is *held* until `dry + 1`, so on every cycle from `cycles + 1` to `dry` inclusive
`promote_dmc_wait` would have returned without doing anything. Skipping it there is not an
approximation; it is the same machine.

`free / 2` is the pairs that fit in those cycles, and it is also what keeps the boundary honest. A
pair is an even get cycle and the odd put after it, so a batch's last cycle is always a **put**;
when `free` is even that put is `dry` itself, and when `free` is odd the batch stops one cycle
short of it. Were a batch allowed to run one cycle past a get cycle on which the request rose, the
put after it would have consumed the DMC's halt cycle in `dma_cycle` — "an OAM transfer's own
cycles stand in for its halt and dummy cycles" — and the fetch would land a cycle earlier than the
fast loop puts it. That is the single failure mode this arithmetic exists for, and the test below
is built around it.

`None` from the horizon — no sample playing — is the whole rest of the transfer in one batch off
one test, which is every OAM DMA of a game that plays no DPCM. Measured with a temporary
`eprintln` in the loop: a plain transfer is **`pairs=256`, one batch**; with a sample at rate 15
(432 cycles a byte, so a fetch inside every transfer) the 128 cases of the test below take 232
batches in all, the state machine getting only the cycles around each fetch.

No `// Q-NNN`: nothing here was ambiguous. The horizon is derivable from the DMC's own documented
timer, so the brief's fallback — asking once per get/put pair because the page says the interleave
is decided at that granularity — was not needed.

### The fast loop is the state machine, cycle for cycle

`req_core_113_the_fast_oam_loop_is_the_state_machine_cycle_for_cycle`
(`tests/oam_dma_timing_tests.rs:125-180`) drives an OAM transfer with a DPCM sample playing and the
DMC's request **aimed** at a chosen point of it: `lead` is how many cycles short of the reader's
own question the `$4014` write lands. Four rates × 32 leads = 128 rows, recording the cycles
stolen, the fetches that landed inside the transfer, and a hash of the OAM left behind.

The expected table was produced by this same test at `dadb94c5` (step 1, whose only change is the
horizon function the test's *setup* calls), where the state machine ran every one of these cycles
alone. It is asserted character for character, and it is not a flat table: stalls come out 513,
514, 515, 516, 517 and 518, and the fetch count 0, 1 or 2 —

| leads | what they exercise | rows |
| --- | --- | --- |
| 0-15 | the fetch in the transfer's first cycles, both parities eight times over | 64 |
| 254-257 | the fetch in the middle | 16 |
| 504-515 | the fetch across the transfer's end and just after it | 48 |
| rate 15, leads 0-15 | **two** fetches in one transfer (432 cycles a byte) | 16 |

The OAM hash is `DCF93400` on all 128 — which is the other half of the check: whatever the
interleave did to the timing, not one of the 256 bytes was read out of step, written twice or
missed.

### The ROMs, re-run

| suite | `d4151eae` | step 2 |
| --- | --- | --- |
| `sprdma_and_dmc_dma` | 2 / 2 (`0F 526`, `0F 528`) | **2 / 2 (`0F 526`, `0F 528`)** |
| `dmc_dma_during_read4` | 3 pass, 0 fail, 2 manual | **3 pass, 0 fail, 2 manual** |
| `pal_apu_tests` | 10 / 10 | **10 / 10** |
| `apu_test` | 19 / 19 | **19 / 19** |
| `blargg_apu_2005.07.30` | 11 / 11 | **11 / 11** |

Every message is `d4151eae`'s character for character, the two `manual` CRCs included:
`dma_4016_read` `08 08 07 08 08`, `dma_2007_read` `11 22 / 11 22 / 44 55 / 11 22 / 11 22 /
5E3DF9C4`, `double_2007_read` `22 33 44 55 66 / 33 44 55 66 77 / D84F6815`. Those CRCs are what
makes this a check of the ordering and not only of the byte count: a halt cycle one cycle out moves
them.

`cargo fmt --check` clean, clippy clean (**no new `allow`**), `cargo test --release -p zax-nes -p
zax-harness` **693 passed, 0 failed, 7 ignored**. No `unsafe`.

**Savestates.** Nothing added, nothing moved: `copy_oam_pairs` and `oam_fast_pairs` read the same
`dma`, `dma_oam_page` and APU state the state machine read, and `oam_steps`/`oam_index` are
`run_dma`'s locals as before. A DMA in flight at save time is still saved the way MF-4a-bis saved
it — `dma` and `dma_oam_page` are transient and `save_state` does not write them — so a state
written before MF-4a loads byte for byte.

## Status

- [x] step 1 — the DMC's fetch horizon
- [x] step 2 — the fast path
- [ ] step 3 — measured
- [ ] step 4 — the suites
