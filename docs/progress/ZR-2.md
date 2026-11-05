# Lot ZR-2 — the DMA halt's re-read must not reach the PPU from the hack zone either

Baseline: `1f3d06c1` (ZR-1's end). Reference picture: `c4577878` (MF-4a 5/5).

ZR-1 removed the **CPU's own** read of a zone address and restored the picture up to MF-4d step 5.
What is left is the **halt's and the dummy cycles' re-read** of the same address inside
[`NesBus::run_dma`] and [`NesBus::dmc_only_dma`], which R0's ZR-1 instruction said to keep. That
instruction was wrong: the hack zone does not exist on real hardware, so no hardware behaviour is
preserved by reading `$2002`/`$2007` there. The rule becomes **code running from the zone never
reaches the PPU registers through the CPU's fetch path — not by the fetch, and not by any DMA halt
or dummy re-read of the fetch address.** The hack's own explicit loads and stores are not fetches
and are out of the rule.

## Steps

1. Confirm: count the PPU-register reads `run_dma` makes on a zone address, SMB3 and Ninja Gaiden III.
2. The fix: the re-reads take the zone path too, and the MF-3b check on the five functions.
3. The unit tests: an OAM-halted and a DMC-halted zone fetch during rendering move nothing.
4. Re-pin the harness regression test of ZR-1 step 4 on the restored picture.
5. The pictures: frame 1 500 against `c4577878`, then every 5th frame of both games.
6. The suites, the campaign, golden-core and the instruction count.

## Step 1 — R0's diagnosis is confirmed, with the numbers

### The two commands

ZR-1's, run on a build of the branch tip:

    zax-harness render --rom "<demo/roms>/Super Mario Bros. 3 (USA).nes" --run 1500 \
      --press <R0's smb3 script> --poke 1000:7CFF=01 --advanced --widescreen 128 \
      --out <dir> --png-nes <file>.png

    zax-harness render --rom "<demo/roms>/Ninja Gaiden III-TASoD [USA][!].nes" --run 700 \
      --press <R0's ng3 script> --advanced --widescreen 128 --out <dir> --png-nes <file>.png

### The counter

A throwaway `[AtomicU64; 12]` in `nes_bus.rs` and a throwaway `zr2_zone` flag that
`halted_zone_read_cycle` raises around its `run_dma` / `dmc_only_dma` call, incremented at **all
five** sites where either of those two reads the halted CPU address — `run_dma`'s halt read and
its idle-cycle repeat, `dmc_only_dma`'s halt read, and `dmc_fetch_cycles`' dummy and alignment
reads — indexed by the register the address decodes to, printed at the end of `rom::capture` and
then reverted. It is in no commit, and the frame-1500 PNG it produced is byte-identical to ZR-1's
(`a1516fbf96fa8de7…`), so the instrument changes nothing.

| at the branch tip, with the hack | SMB3, 1 500 frames | Ninja Gaiden III, 700 frames |
| --- | ---: | ---: |
| zone fetches a DMA unit **halted** | 1 077 | 1 312 |
| **re-reads of the fetch address** those halts made | **3 166** | **3 810** |
| of them absorbed by the extension's `$3000`-`$3FFF` window | **0** | **0** |
| of them reaching `Ppu2C02::read` | **3 166** | **3 810** |

and by register:

| | `$2000` | `$2001` | `$2002` | `$2003` | `$2004` | `$2005` | `$2006` | `$2007` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| SMB3 | 346 | 460 | **441** | 348 | 389 | 384 | 356 | **442** |
| NG3 | 502 | 413 | **432** | 440 | 510 | 482 | 480 | **551** |

**R0's diagnosis is confirmed.** ZR-1 took away the CPU's own read and left the DMA's: 3 166 reads
on SMB3's 1 500 frames, every one of them reaching `Ppu2C02::read`, 441 of `$2002` clearing `w`
and 442 of `$2007` moving `v` and the read buffer — and, since MF-4d step 6, moving `v` by a coarse
X and a Y at once when they land during rendering. Nothing comes from elsewhere: there is no
remaining change to find.

Why ZR-1 counted 3 301 and this counts 1 077 halts: ZR-1's counter sat in `zone_read_cycle` on the
`dma != 0` side, which also counts the fetches where a **held** DMC request makes `dma` non-zero
and no unit may take the bus. This one counts after `halted_zone_read_cycle`'s early return, so it
counts the fetches a unit actually halted. 3 166 / 1 077 ≈ 2.94 reads a halt, which is the halt,
the dummy and (usually) the alignment cycle of a lone DMC fetch.

### The picture that is left (step 1)

Frame 1 500, the reference `c4577878` (`a923573ab7b9b3f8…`) against the tip
(`a1516fbf96fa8de7…`): **51 scanlines differ, y = 169 to 220**, and the harness's own summary line
counts **22 colours against 21**. That is the same footprint, to the scanline, that ZR-1 step 1
measured for the whole regression — R0's three symptoms by eye (the band of sky cutting the ground,
the truncated right platform, the overflowing status-bar band) are what is left of it.

## Step 2 — the fix

### What it is

One new function, `NesBus::dma_repeat_read`, which is the read a DMA unit makes of the **CPU's**
address while it holds the bus — the halt cycle's own read and the dummy and alignment cycles'
repeats of it. It carries the rule and its measurements, and its body is

    if !ZONE { let value = self.read_internal(read_address); self.data_bus = value; }

The five sites that made that read now call it: `run_dma`'s halt read and its idle-cycle repeat,
`dmc_only_dma`'s halt read, and `dmc_fetch_cycles`' dummy and alignment reads. Those three
functions take `const ZONE: bool`; `halted_read_cycle` calls them `::<false>` and
`halted_zone_read_cycle` `::<true>`.

Three things are deliberately *not* done:

* **the units' own reads stay.** An OAM transfer's source bytes and the DMC's sample fetch are the
  units' reads of the units' addresses, not of the CPU's, and they go on exactly as before — the
  transfer lands in OAM byte for byte and the sample byte reaches the DMC;
* **every cycle stays.** Only the read is suppressed, never a `tick` — the table below counts them;
* **`data_bus` is not loaded.** That is the same statement, not an extra one: a read that does not
  happen drives nothing onto the bus. The byte the fetch takes comes from the pack's code buffer
  and `read_code` loads it there, as it always did.

### Why `ZONE` is a const parameter

`run_dma` and `dmc_only_dma` are on the DMA path of **every** game, so the brief's "the ordinary
path must not change" is the binding constraint. A `bool` argument would put an argument move at
each ordinary call site and a test in each body. A const parameter is monomorphised instead: the
`false` copy is the old function and the `true` copy is a second, cold one, reached only from a
zone fetch. What it buys is measured below; what it costs is 572 instructions of cold code.

### The MF-3b check on the five functions

Clean `git archive` builds of the baseline `2d714d82` (ZR-1's end) and of this tree, each
extracted into its own directory with its own `CARGO_TARGET_DIR`, never reused. Instructions per
symbol and call sites per callee, MF-3b rule 1, on the pre-LTO lib dump
(`cargo rustc --release -p zax-nes --lib -- --emit=asm`) and — MF-3b rule 2, this being the CPU
read path — on the **post-LTO** dump of the shipped binary
(`cargo rustc --release -p zax-harness --bin zax-harness -- --emit=asm`). The two dumps agree
symbol for symbol; the post-LTO figures:

| symbol | ins, baseline | ins, fixed | call sites, baseline | call sites, fixed |
| --- | ---: | ---: | ---: | ---: |
| `NesBus::read_cycle` | 182 | **182** | 65 | **65** |
| `NesBus::halted_read_cycle` | 53 | **53** | 0 | **0** |
| `NesBus::zone_read_cycle` | 18 | **18** | 36 | **36** |
| `NesBus::halted_zone_read_cycle` | 43 | **43** | 0 | **0** |
| `NesBus::run_dma` | 511 | — | 2 | — |
| `NesBus::run_dma::<false>` | — | **511** | — | 1 |
| `NesBus::run_dma::<true>` | — | 483 | — | 1 |
| `NesBus::dmc_only_dma` | 102 | — | 2 | — |
| `NesBus::dmc_only_dma::<false>` | — | **102** | — | 1 |
| `NesBus::dmc_only_dma::<true>` | — | 89 | — | 1 |
| `NesBus::read_internal` | 176 | **176** | 14 | 16 |
| `NesBus::tick` | 79 | **79** | 84 | 93 |
| `NesBus::dma_read` | 76 | **76** | 2 | 4 |
| `NesBus::catch_up_ppu` | 219 | **219** | 10 | 12 |
| `Cpu6502::step` | 916 | **916** | 2 | **2** |
| `Cpu6502::resolve` | 1 402 | **1 402** | 3 | **3** |
| `Cpu6502::branch` | 377 | **377** | 0 | **0** |

`CpuBus::read_code` and `CpuBus::read` have no symbol in either dump — both are inlined
everywhere — and `zone_read_cycle`'s **36 call sites** and `read_cycle`'s **65**, both unchanged,
are their fetch and read sites seen from the callee's side.

**The ordinary path is not merely the same size, it is the same code.** Each function's
instruction stream was extracted from both dumps and compared line by line, with only the local
block labels renumbered (they are per-monomorphisation and name the same blocks):

| function | lines | differing lines |
| --- | ---: | --- |
| `run_dma` → `run_dma::<false>` | 568 | **0** |
| `dmc_only_dma` → `dmc_only_dma::<false>` | 112 | **0** |
| `read_cycle` | 209 | **0** |
| `zone_read_cycle` | 21 | **0** |
| `read_internal` | 202 | **0** |
| `tick` | 88 | **0** |
| `dma_read` | 87 | **0** |
| `halted_read_cycle` | 62 | 2 — the two `callq`s, same instruction and same place, naming `run_dma::<false>` and `dmc_only_dma::<false>` instead of `run_dma` and `dmc_only_dma` |
| `halted_zone_read_cycle` | 51 | 2 — the same two, naming the `::<true>` copies |

**Where the new call sites are: all of them inside the two cold copies, and nowhere else.** Over
the whole binary dump, **903 733 → 904 305 instructions, +572**, which is 483 + 89 and nothing
else. Exactly eight callees moved a call-site count, and the calls the two new copies contain
account for every one of them:

| callee | delta over the binary | in `run_dma::<true>` | in `dmc_only_dma::<true>` |
| --- | ---: | ---: | ---: |
| `NesBus::tick` | +9 | 6 | 3 |
| `NesBus::read_internal` | +2 | 1 | 1 |
| `NesBus::catch_up_ppu` | +2 | 2 | 0 |
| `NesBus::dma_read` | +2 | 2 | 0 |
| `NesBus::push_expansion_audio` | +2 | 1 | 1 |
| `Apu2A03::run_cpu_cycles` | +2 | 1 | 1 |
| `memcpy` | +2 | 2 | 0 |
| `core::panic_bounds_check` | +1 | 1 | 0 |

No callee gains its **first** call site, so MF-3b's stop-the-lot event does not occur, and nothing
outside the two cold copies is reachable from a read that is not a zone fetch.

**And the same table shows the fix did what it says.** Compare the copies with the originals:

| | `read_internal` calls | `tick` calls |
| --- | ---: | ---: |
| `run_dma` / `run_dma::<false>` | 4 | 6 |
| `run_dma::<true>` | **1** | **6** |
| `dmc_only_dma` / `dmc_only_dma::<false>` | 4 | 3 |
| `dmc_only_dma::<true>` | **1** | **3** |

Every `tick` survives — not one cycle is lost — while the reads of the CPU's address are gone.
The single `read_internal` left in each copy cannot be one of them: `dma_repeat_read::<true>` has
an empty body, so no call can come from it. What is left is the unit's read of its **own**
address.

The cost is therefore **zero instructions on every path that is not a halted zone fetch**, which
is every instruction of every game without hack code; and on a halted zone fetch it is one `callq`
into a 483- or 89-instruction function in place of one `callq` into a 511- or 102-instruction one
— which is lower, and which only a game running hack code ever pays.

### The suites at this step

`cargo fmt --check` clean. `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean. `cargo test --release -p zax-nes -p zax-harness`: **746 passed, 0 failed, 8 ignored** —
ZR-1's 744 plus the two tests step 3 adds.

No `unsafe`. No state field added, removed or reordered, and nothing new is serialised: `ZONE` is
a compile-time parameter and lives in no value. The save format does not move.

### The picture, already

The decisive check, out of order because it is one command: frame 1 500 of Super Mario Bros. 3
with `--advanced --widescreen 128` is now **`a923573ab7b9b3f8…`** — byte for byte `c4577878`'s
reference picture. Step 5 does the dense A/B and the control.

## Step 3 — the unit tests

In `exec_zone_tests.rs` (REQ-CORE-113), and in step 2's commit rather than their own: the fix makes
two of ZR-1's three assertions **false**, so the code and the tests could not be committed apart.

### What changed, and what was added

ZR-1's three tests were built on the premise that the halt's own repeated read legitimately moves
the PPU. Two of them therefore asserted a **one-read-fewer** relation against the data read the
fetch stands for. That relation is now wrong — the fetch makes **none** of those reads — so both
are re-pinned on the sharper claim, and two tests are added for the state the picture broke in.

| test | the state | what it asserts |
| --- | --- | --- |
| `…_under_a_held_dmc_request_touches_no_ppu_register` | `dma` non-zero, no unit may halt | unchanged from ZR-1, and still passing: `v`, the read buffer and `w` are a control's |
| `…_a_halted_zone_fetch_makes_none_of_the_reads_the_data_read_it_stands_for_makes` | a DMC fetch due | **re-pinned**: `v` is **exactly** where it was, while the data read moved it |
| `…_halted_by_an_oam_transfer_lets_it_finish_and_reads_nothing_itself` | a `$4014` transfer pending | **re-pinned** the same way, and the 256 bytes still land in OAM byte for byte |
| `…_halted_by_a_dmc_fetch_during_rendering_moves_no_ppu_register` | **new** | during rendering, the fetch is a control's halted read of `$8000` in `v`, `w`, the read buffer **and** the cycles |
| `…_halted_by_an_oam_transfer_during_rendering_moves_no_ppu_register` | **new** | the same, the other unit, 512 cycles of repeats |

The two new ones run **during rendering**, which is where the damage was: rendering off, a `$2007`
read steps `v` by the control register's stride, but during rendering it moves a coarse X and a Y
at once (MF-4d step 6), and that is what broke the status-bar split.

Three notes on how they are written:

* **the counterpart is a control, not the data read.** A halted read of `$8000` is the same halt,
  the same unit and the same cycles on an address that is not a register, so a halted zone fetch
  must be indistinguishable from it in the PPU — which is the rule itself, stated as a test.
* **it pins the cycles too, for free.** During rendering the PPU moves `v` by itself, so two
  machines only agree on `v` if they stopped on the same dot; the test also compares
  `(scanline, dot)` outright.
* **each address probes its own observable.** Each test fetches at a `$2007` mirror and at a
  `$2002` mirror: `$2007` moves `v` and refills the read buffer, `$2002` clears the write toggle
  (probed with the `$2006` double write REQ-CORE-112's test already uses). Those are the two reads
  with something to lose, and 442 and 441 of step 1's SMB3 count.

Each also asserts what must **not** have changed: the DMC fetched, the transfer ran, and OAM is
the control's byte for byte.

### All four halted tests fail on the baseline

`2d714d82`'s `nes_bus.rs` put back under the new tests, nothing else touched:

| test | baseline | this tree |
| --- | --- | --- |
| `…_under_a_held_dmc_request_…` | passes | passes |
| `…_makes_none_of_the_reads_…` | **fails**, `v` 9 032 against 9 030 | passes |
| `…_halted_by_an_oam_transfer_lets_it_finish_…` | **fails**, the same two | passes |
| `…_by_a_dmc_fetch_during_rendering_…` | **fails** | passes |
| `…_by_an_oam_transfer_during_rendering_…` | **fails** | passes |

and the two rendering failures print the damage itself. At the `$2007` mirror, control against
baseline fetch:

    position: (16, 53)   vram: 4493   buffered: 17   write_toggle_probe: 4505
    position: (16, 53)   vram: 16784  buffered: 0    write_toggle_probe: 4505

`v` has gone from `$118D` to `$4190` — a coarse X **and** a Y, MF-4d step 6's rule applied to a
read the hardware never makes — and the read buffer has been refilled. At the `$2002` mirror the
`vram` and `buffered` columns agree and `write_toggle_probe` is 4 505 against 8 591: the toggle
cleared. **`position` is identical in every case**, which is what makes the claim about reads and
not about cycles: the baseline spent the same cycles and merely read.

`cargo fmt --check` clean. `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean. `cargo test --release -p zax-nes -p zax-harness`: **746 passed, 0 failed, 8 ignored**.
