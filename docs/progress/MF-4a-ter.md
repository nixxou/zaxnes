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

## Step 3 — measured, and the other half of the bookkeeping

### The ROMs this is measured on, and why they are the right ones

R0's two games are commercial and not in `assets-free/`, so the lot measures the public ROMs that
reproduce their *shape*. The shape is two numbers, and this lot counted them instead of guessing
them — a temporary `DMACOUNTS` line in `bench-core`, built in a throwaway WSL tree and not in this
one, printing `NesBus::oam_dma_count` and `dmc_dma_count` after 300 frames:

| ROM | OAM transfers | DMC fetches | what it stands for |
| --- | ---: | ---: | --- |
| `spritecans-2011/spritecans` | **295** | **0** | one OAM transfer a frame and no DPCM — the lot's own target, isolated |
| `dpcmletterbox` | **297** | **19 051** | one OAM transfer a frame *and* DPCM played continuously |
| `apu_test/8-dmc_rates` | 0 | 289 | the DMC fetch alone |
| `M1_P128K` | 1 | 0 | MF-4a-bis's mapper-1 stand-in, for the no-regression check |
| `M0_P32K_C8K_V` | 11 | 0 | likewise, NROM |

`spritecans` is the first public ROM found here that runs a full OAM transfer every frame, which
MF-4a-bis had to record as missing ("no public ROM available here does an OAM transfer every
frame"). It is what makes this lot's claim measurable rather than extrapolated.

Procedure exactly as `PERFORMANCE-HISTORY.md` §3: `git archive` of each commit into its own fresh
directory with its own `CARGO_TARGET_DIR`, `CARGO_PROFILE_RELEASE_STRIP=false`, then
`valgrind --tool=callgrind --dump-instr=no --cache-sim=no --branch-sim=no ./zax-harness bench-core
<rom> --frames 300 --warmup 0`. Three commits built: `b39eaba4`, `d4151eae` and this lot's end.

### The brief's target metric

`run_dma` + `dma_read` + `write_internal`, against `b39eaba4` — with `dmc_only_dma` counted into
`run_dma`, since it is `run_dma`'s DMC case split out of it:

| ROM | `b39eaba4` | `d4151eae` | **this lot** |
| --- | ---: | ---: | ---: |
| `spritecans` (295 OAM, 0 DMC) | 3 603 384 | +5 700 859 | **−27 208** |
| `M0_P32K_C8K_V` (11 OAM) | 5 407 336 | +213 371 | **−254** |
| `M1_P128K` (1 OAM) | 13 286 171 | +20 958 | **+1 537** |
| `8-dmc_rates` (0 OAM, 289 DMC) | 965 015 | +54 989 | **+22 772** |
| `dpcmletterbox` (297 OAM, 19 051 DMC) | 4 763 150 | +9 326 690 | **+1 446 877** |

**The OAM bookkeeping the lot set out to trim is gone, and then some.** Per stolen cycle on
`spritecans`, over its 295 × 513.5 = 151 483 stolen cycles:

| | instructions per stolen cycle |
| --- | ---: |
| `b39eaba4`, the old `$4014` loop | 23.8 |
| `d4151eae`, the state machine | **61.4** |
| this lot, the fast loop | **23.6** |

The +37.6 per cycle R0 measured is the 50-per-cycle the brief estimated, and it is given back in
full: the fast loop is 0.2 instructions a cycle *under* the loop that lived inside the `$4014`
write before MF-4a, while doing the halt, the alignment and the get/put parity the old loop did
not do at all.

### What is left, and it is not the OAM transfer

The one row above the target is `dpcmletterbox`, at +1.45 M, and it is **19 051 DMC fetches**, not
297 OAM transfers. Taking `spritecans`'s measured 2 296 263 / 295 = 7 784 instructions per
transfer, `dpcmletterbox`'s 297 transfers account for 2.31 M of its 3.67 M, leaving 1.36 M over
19 051 fetches = **71 instructions per DMC fetch**. `8-dmc_rates` gives the same figure
independently: 20 122 / 289 = **69.6**.

That is why this step also split the lone DMC fetch out (see below): at `d4151eae` the same fetch
cost **181** instructions, and at `b39eaba4` about **45**, inlined into `tick`. The remaining ~25 a
fetch is the state the halt has to set up and the fetch's own `sync_apu`, and it is charged in full
against a `b39eaba4` whose cost for it sat in `tick` and so was never in these three rows. On a
game that plays no DPCM it is zero.

### The lone DMC fetch, split out

A DMC fetch with no OAM transfer beside it is **most** of what a DPCM game asks of this path —
`dpcmletterbox` makes 64 of them a frame against its one transfer — and it is a sequence that is
fixed before it starts. `NesBus::dmc_fetch_cycles` (`nes_bus.rs:703-739`) runs it straight: the
dummy cycle, an alignment cycle when the next one is not a get cycle, and the fetch. Nothing is
asked per cycle, and the reasoning is in four lines rather than a horizon:

* **no second DMC request can appear inside it** — `poll_dmc_dma` raises one only while
  `(dma & (DMA_DMC | DMA_DMC_WAIT)) == 0`, and `DMA_DMC` stays set until the byte is supplied, so
  `promote_dmc_wait` would have nothing to do on any of these cycles;
* **no OAM transfer can appear** — `$4014` is a write and the CPU is halted;
* `DMA_HALT` is consumed by the caller and `DMA_DUMMY` by the first cycle, so `dma_cycle`'s two
  tests have one answer each;
* the entry condition's `!internal` makes `input_port` false too (`$4016` and `$4017` are inside
  `$4000`-`$401F`), so every idle cycle repeats the CPU's read unconditionally, and
  `halt_is_invisible` — which needs `internal` — was false.

`NesBus::dmc_only_dma` (`nes_bus.rs:683-701`) is its entry, reached from `halted_read_cycle`
(`nes_bus.rs:1056-1061`) on `dma == (DMA_DMC | DMA_DUMMY | DMA_HALT) && (address & 0xFFE0) !=
0x4000` rather than through `run_dma`, so the fetch pays neither `run_dma`'s prologue — eight
general registers and two XMM registers, the price of the OAM state machine's working set — nor
the three classifications of the halted address that only the `internal` case reads. Measured, per
fetch: inside `run_dma` **92**, through its own entry **71**.

Anything else — a fetch during an OAM transfer, a halted `$4016` read, a fetch after a state load
— is the exact state machine. `req_core_113_a_lone_dmc_fetch_takes_three_cycles_or_four_by_its_alignment`
(`tests/dmc_dma_timing_tests.rs:69-103`) pins the new path's three and four cycles at both
parities, and `only_the_halt_cycle_of_a_dma_reaches_the_controller` pins the same counts for the
`internal` case that still goes through the state machine, so both halves are covered.

### No regression elsewhere: the generated code, and the executed instructions

**Executed instructions.** Between `d4151eae` and this lot, across all five ROMs, exactly four
`zax_nes` symbols moved — and they are the four this lot touches:

| ROM | `run_dma` | `dma_read` | `dmc_only_dma` | `halted_read_cycle` | any other `zax_*` symbol |
| --- | ---: | ---: | ---: | ---: | --- |
| `spritecans` | −3 764 547 | −1 963 520 | — | +9 145 | **none** |
| `dpcmletterbox` | −6 770 109 | −2 472 158 | +1 362 454 | +155 081 | **none** |
| `8-dmc_rates` | −44 819 | −7 520 | +20 122 | +2 315 | **none** |
| `M1_P128K` | −12 765 | −6 656 | — | — | **none** |
| `M0_P32K_C8K_V` | −140 409 | −73 216 | — | — | **none** |

`NesBus::tick`, `read_cycle`, `write_internal`, `read_internal`, `Cpu6502::step`, `branch`,
`resolve`, `apply_read`, `Apu2A03::run_cpu_cycles`, `quiet_cycles` and every PPU symbol read the
**same number to the instruction** at `d4151eae` and here, on every one of the five ROMs. That is
the strongest form the no-regression check can take, and it is why the `halted_read_cycle` row is
worth naming: its +155 081 on `dpcmletterbox` is the new entry test, 8 instructions on each of the
19 348 cycles that reach it, and 0 on a cycle that reaches no DMA at all.

**Generated code**, post-LTO dump of the whole `zax-harness` binary
(`cargo rustc --release -p zax-harness --bin zax-harness -- --emit=asm`). `d4151eae` changed only
`docs/`, so MF-4a-bis's `bin-bis2-final.s` *is* its code and is the baseline here;
`rust/target-mf/asm/bodies.py` extracts a symbol's body and diffs it instruction for instruction:

| symbol | instructions | verdict |
| --- | ---: | --- |
| `Cpu6502::step` | 893 | **IDENTICAL** |
| `Cpu6502::branch` | 368 | **IDENTICAL** |
| `Cpu6502::resolve` (2 defs) | 1 725 | **IDENTICAL** |
| `NesBus::read_cycle` | 3 | **IDENTICAL** |
| `NesBus::tick` | 75 | **IDENTICAL** |
| `NesBus::catch_up_apu` | *inlined into `tick`, no symbol in either* | — |
| `NesBus::write_internal` | 2 565 | **IDENTICAL** |
| `NesBus::read_internal` | 729 | **IDENTICAL** |
| `Apu2A03::run_cpu_cycles` (4 defs) | 873 | **IDENTICAL** |
| `Apu2A03::quiet_cycles` | 90 | **IDENTICAL** |

Three of those — `run_cpu_cycles`, `write_internal`, `read_internal` — first came out DIFFERS, and
the difference was worth running down rather than waving away. It is **names**: the crate-wide
anonymous-constant hash, which any source edit moves, and two anonymous-constant *ordinals*
shifted by one because the crate gained functions (`anon.…498` → `anon.…499` in `run_cpu_cycles`,
`anon.…520` → `anon.…521` in `read_internal`). The two shifted constants were read out of both
dumps and are byte-identical: a decode table, and a panic `Location` record naming the same file,
line 383 and column 13. `bodies.py -o` normalises the ordinal, and the table above is that run.

**MF-3b rule 1**, the stop-the-lot call-site check — none of the named callees gains a first call
site: `end_cycle` 0 → 0, `read_nametable_as` 0 → 0, `read_nametable_inner` 15 → 15,
`read_pattern_planes` 3 → 3, `Board::ppu_read` 4 → 4, `ppu_peek` 4 → 4, `on_ppu_address_bus`
12 → 12, `irq_asserted` 0 → 0, `on_cpu_cycle` 1 → 1. What does gain call sites is `tick` 79 → 84,
`read_internal` 7 → 13, `catch_up_ppu` 8 → 9 and `run_cpu_cycles` 16 → 17, and every one of them is
inside `copy_oam_pairs`, `dmc_fetch_cycles` or `dmc_only_dma` — all `#[inline(never)]` or reached
only from one, so no cycle that runs no DMA encounters them. The bodies table above is the proof
that their callers did not move.

### Whole-process instructions

| ROM | `b39eaba4` | `d4151eae` | **this lot** | vs `d4151eae` | vs `b39eaba4` |
| --- | ---: | ---: | ---: | ---: | ---: |
| `spritecans` | 6 338 757 419 | 6 350 741 097 | **6 344 970 479** | **−5.77 M** | +6.21 M (+0.098 %) |
| `dpcmletterbox` | 6 282 116 622 | 6 318 559 543 | **6 310 857 904** | **−7.70 M** | +28.73 M (+0.457 %) |
| `M0_P32K_C8K_V` | 5 697 405 530 | 5 703 831 793 | **5 703 620 142** | −0.21 M | +6.21 M (+0.109 %) |
| `M1_P128K` | 5 069 741 221 | 5 075 587 411 | 5 075 644 636 | +0.06 M | +5.90 M (+0.116 %) |
| `8-dmc_rates` | 5 789 425 327 | 5 810 430 566 | 5 810 463 953 | +0.03 M | +21.04 M (+0.363 %) |

The two small positives are not core code. On `M1_P128K` the core rows are −19 421 and the whole
process is +57 225; the difference is `getgeneric` and the `luaH_*` rows moving ±40 k, the Lua
hash-order and placement noise every MF lot has recorded. Same on `8-dmc_rates`: core −29 902,
Lua +63 k.

The residues against `b39eaba4` are the two things the brief puts out of scope, and they are the
same ones MF-4a-bis measured:

* `spritecans`, +6.21 M: `read_cycle` +26 211 128 against `tick` −22 059 941 — the read-cycle gate,
  three instructions a read cycle against the two and a half `tick` gives back, which the brief
  says to keep and Q-119 asks R0 to rule on. The DMA rows are **−27 208**, i.e. negative;
* `dpcmletterbox`, +28.73 M, and `8-dmc_rates`, +21.04 M: the gate plus **MF-4a's DMC output-unit
  phase** (`run_cpu_cycles` +12 459 131 and `quiet_cycles` −1 232 022 on `dpcmletterbox`), which is
  the behaviour change `sync_dmc` requires and which MF-4a-bis measured at +25.5 M on
  `8-dmc_rates` on its own. No arrangement of code returns it.

### What R0 should see on Mega Man 2 and Zelda II

R0's breakdown reads `run_dma` +8.4 M, `dma_read` +2.7 M, `write_internal` −3.2 M on **both**
games, and attributes all of it to the OAM transfer. The `dma_read` row says how much of that is
really so: `dma_read` costs 26 instructions a call, called once per OAM source byte and once per
DMC fetch, and 300 transfers are 76 800 source bytes. So the OAM transfer accounts for essentially
the whole of the +2.7 M, and both games being within 0.1 M of each other says the term they share —
the transfer — dominates. On that reading both games are near `spritecans`'s profile and should
land **at or just under `b39eaba4`** on the three rows.

Where they do play DPCM, the arithmetic is on the table above: each DMC fetch is 71 instructions
here against 181 at `d4151eae`, so a game making *n* fetches in 300 frames should come out at
about `n × 71` on top of an OAM figure that is itself slightly negative.

| R0's measurement | expected |
| --- | --- |
| `run_dma` + `dma_read` + `write_internal` | **−27 k** if the games play no DPCM in those 300 frames; `n × 71` above that otherwise, with `dmc_only_dma` to be added to `run_dma` |
| `NesBus::dmc_only_dma` | new symbol, the lone DMC fetch; 0 on a game with no DPCM |
| `NesBus::halted_read_cycle` | +8 instructions per cycle that reaches a DMA, 0 elsewhere |
| `NesBus::tick`, `read_cycle`, `write_internal`, `read_internal` | **0**, to the instruction |
| `Cpu6502::step`, `branch`, `resolve`, `apply_read` | **0**, to the instruction |
| `Apu2A03::run_cpu_cycles`, `quiet_cycles` | **0**, to the instruction |
| **PPU** | **0**. No PPU symbol moved in either dump and no PPU row moved in any run |
| golden-core | the DMA timing is cycle-identical to `d4151eae` (128-row table, `526`/`528`, `08 08 07 08 08`, CRC `5E3DF9C4`), so no game moves |

## Status

- [x] step 1 — the DMC's fetch horizon
- [x] step 2 — the fast path
- [x] step 3 — measured
- [ ] step 4 — the suites
