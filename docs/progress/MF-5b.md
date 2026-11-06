# Lot MF-5b — the VRC7 board, with its FM audio

Baseline `a577f336` (MF-5a's end). **782 tests**, 0 failed, 8 ignored, measured at the lot's start
and reproduced here before anything was written.

No file under `crates/zax-nes/src/cartridges/mappers/` handles iNES mapper 85, and
`SUPPORTED_MAPPERS` has not got it. REQ-CORE-075 makes expansion audio optional (P3) and Q-118
lists the VRC7 among the optional chips; MF-5a's VRC6 (`VrcIrq`, `expansion_audio_output`, the
`Board::on_cpu_cycle` latch) and the Sunsoft 5B are the patterns this lot follows.

## Steps

1. The VRC7 board (mapper 85), with the VRC IRQ **shared** with `vrc2_vrc4.rs` and `vrc6.rs`.
2. The VRC7 audio: the OPLL subset — six two-operator FM channels, the custom patch and the
   fifteen built-in ones, the envelope generator, the two LFOs.
3. Save states for the board and the audio block, behind the existing block versioning.
4. The ROM check.
5. The suites, the campaign ROM by ROM against MF-5a's, and the campaign document.

## The ROM situation, settled first because it shapes every step

**There is no mapper-85 image anywhere in `assets-free/test-roms`.** Every `.nes` under the tree
was opened and its iNES header decoded, the NES 2.0 submapper byte included: **330 images carry a
valid header**, and between them

    mappers 0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 22, 28, 34, 66, 69, 78, 118, 180

which is MF-5a step 7's list to the image. Mapper 85: **zero images**. `holy-mapperel` has no
`M85` target — its 43 files stop at `M180` — and no file anywhere in the tree has `vrc` or `85`
in its name.

So this lot cannot be driven by a ROM at any step, and that is stated here rather than discovered
at step 4: **every behavioural claim below rests on a unit test written against the nesdev pages
and the YM2413 application manual.** R0 runs the two commercial titles: *Lagrange Point*
(submapper 2, the VRC7a) and *Tiny Toon Adventures 2* (J) (submapper 1, the VRC7b).

## The sources, and the one that is missing

| source | what it gives |
| --- | --- |
| nesdev, "VRC7" | the whole board: the register map, the two address lines, `$E000`'s four bits |
| nesdev, "VRC7 audio" | the register map of the chip, the semantics of every operator field, the 26 internal registers, the fifteen built-in patches as hex, the multiplier table, the feedback table, the two waveforms, the frequency law, 49716 Hz |
| nesdev, "VRC IRQ" | the counter, already implemented as `VrcIrq` (MF-5a step 1) |
| YM2413 application manual (smspower, bitsavers) | `RATE = 4 × R + Rks`, the TL and SL step sizes in dB, the KSL law in dB/octave, the two LFO **frequencies**, "rises exponentially during attack time and varies linearly everywhere else" |
| YM2413 reverse-engineering notes 2015-03-20 (smspower) | the envelope's **0.375 dB per step over 128 levels**, the shared envelope counter, and the `eg_shift`/`eg_select` advance tables for rates 4-59 |
| `third_party/mesen2` | `Konami/VRC7.h`, the board — adapted. **Its `Audio/Vrc7Audio.h` is a 91-line shell around `emu2413`, and `Shared/Utilities/` is not in the copied tree** (`third_party/README.md`: only `Core/NES` and `Core/Shared/Audio` were copied). So there is **no OPLL reference implementation in reach**, and the FM core below is built from the documentation alone. |

That last row is why this lot opens two tickets rather than one: the board is documented to the
bit, and the FM core has four quantities no allowed source gives numerically.

## Step 1 — the VRC7 board, mapper 85

### The code

`cartridges/mappers/vrc7.rs`, **new**. Three switchable 8 KiB PRG windows with the last one
wired, eight 1 KiB CHR windows, `$E000`'s four bits (two of arrangement, the silence bit and the
WRAM enable), and the IRQ through the shared [`VrcIrq`].

`registry.rs`: `85 => Board::Vrc7(...)`, and `SUPPORTED_MAPPERS` gains 85. `board.rs`: the variant
is **appended**, as that file's own note requires. `cartridges/mod.rs`: `clocks_on_cpu_cycle`
gains 85 (the IRQ counter). `nametable_pages_vary` does **not**: a VRC7 drives CIRAM A10 from
`$E000` and never gives a page per table.

### The select line, and the two sound ports that ignore it

The page gives the variants in one sentence — "VRC7b uses A3 to select registers ($x008) …
VRC7a uses A4 to select registers ($x010)" — so `register_index` is two `bool`s latched at load
(`select_a3`, `select_a4`), the shape `Vrc2Vrc4::index_lines` has, and a dump with no submapper
answers to **both** lines. The sound ports are decoded **before** that rewrite and are at `$9010`
and `$9030` on every submapper, because both pages print them there with no submapper clause and
Mesen exempts exactly those two addresses from its A4-to-A3 fold (`VRC7.h:91-94`). A5 is what
separates them, so the mask is `$F030` and not `$F010`.

### The one board whose power-on overrides the header

`$E000` = 0 at power-on is **vertical** mirroring, and the board has no pass-through mode, so a
mapper-85 dump whose header says horizontal comes up vertical. Mesen does the same
(`UpdateState()` from `InitMapper`). The VRC6's mode 0 is not an analogue — that one leaves the
header's arrangement alone — so this is noted on `Mapper::reset` and pinned by
`req_core_100_a_vrc7_is_vertical_at_power_on_whatever_the_header_says`.

### The two bounds checks the first draft put on the hot path

`panicsites.py`, 911 → 914, and two of the four were the step's own: `BankWindows::map_chr_1k`
gained a `panic_bounds_check` of its own and a **fifth one appeared in `NesBus::write_internal`**,
which is MF-4d step 2's stop-the-lot test. Both came from indices LLVM could not prove in range —
`update_banks` written with `iter().enumerate()`, and `let slot = (page - 0xA) * 2 + index`.
Indexed loops over constant ranges (the shape `vrc6.rs` already uses) and a `& 7` that changes no
value fixed both: **911 → 911, identical**.

## The regression this step found, and the four instructions that paid for it

The board is twenty-two lines of banking and a shared IRQ counter. Adding it cost **+0.542 % on
all thirteen ROMs of the set**, and the cause has nothing to do with the VRC7:

| ROM | `a577f336` | the board as first written | |
| --- | ---: | ---: | ---: |
| `vblnmi` | 3 328 805 321 | 3 363 159 396 | **+1.032 %** |
| `mmc5` | 11 331 592 771 | 11 410 669 888 | +0.698 % |
| `m1` | 4 914 873 709 | 4 947 964 876 | +0.673 % |
| … every one of the thirteen slower … | | | +0.241 % to +1.032 % |
| **total** | **79 717 110 085** | **80 149 185 443** | **+0.542 %** |

The symbol diff names it in one line, on every ROM:

    +169,739,749    0 -> 169,739,749   <NesBus as CpuBus>::irq_asserted

**This is MF-3b step 1's regression, one level up.** `Cpu6502::end_cycle` samples /IRQ once per
CPU cycle through `NesBus::irq_asserted`, which inlines `Cartridge::irq_asserted` and with it the
whole `Board::irq_asserted` dispatch — **thirty-two arms, twenty of which only answer "no"**.
MF-3b pinned `end_cycle` itself so the sampling would stay inlined, and nothing pinned the chain
below it: the thirty-second arm grew the callee past the inliner's budget and LLVM outlined
`NesBus::irq_asserted` instead, calling it every cycle. About **19 instructions per CPU cycle, on
twelve ROMs of thirteen whose board cannot assert an IRQ at all.**

### MF-3b rule 1 could not see it, and the tool is fixed

`callsites.py` reported `Board::irq_asserted` **0 → 0** while this was happening: the regression
had moved one link up the chain and the watched list only had the last link.
`target-mf/asm/callsites.py` now watches `NesBus::irq_asserted` and `Cartridge::irq_asserted`
too, with the story on it. **This is the second time the lots have found the watched list blind
to a real outlining** (MF-3b step 3 was the first, `read_nametable_as` in a pre-LTO dump), and
both times the fix was to widen the instrument rather than to trust it.

### The load-time latch was built and refuted, and the numbers are why

MF-3b step 1 costed a `Cartridge::irq_capable` latch on paper and declined it. This step
**built** it — the latch plus `#[inline(never)]` on the dispatch — and measured it, because the
paper argument could not say how big either side was:

| ROM | board | vs `a577f336` with the latch |
| --- | --- | ---: |
| `m1` | MMC1 | **−1.526 %** |
| `metroid` | MMC1 | **−1.428 %** |
| `m0` | NROM | **−1.300 %** |
| `z2` | MMC1 | **−1.069 %** |
| `m4` | MMC3 | **+1.805 %** |
| `mmc5` | MMC5 | **+1.449 %** |
| `smb3` | MMC3 | **+1.340 %** |

Exactly the trade MF-3b predicted: a board that cannot assert an IRQ stops paying the dispatch,
and a board that can pays a `callq` per CPU cycle instead of an inlined bitset. **Three of R0's
thirteen get 1.3-1.8 % slower, two of them MMC3s**, so it is out — and the measurement is
recorded here so the next lot does not have to build it again.

### What shipped: four `#[inline(always)]`, each with its number on it

| attribute | what it buys |
| --- | --- |
| `<NesBus as CpuBus>::irq_asserted` | the 170 M itself |
| `Cartridge::irq_asserted` | the boundary that moved here once the outer one was pinned: 6.6 M on `m1`, 12.9 M on `mmc5` |
| `Cpu6502::execute` | the interpreter's single out-of-line boundary, kept where MF-3b left it: the flip to `execute` cost **about +6 M on every ROM** (`execute` 0 → 192.5 M, `step` −93.5 M, `execute_without_operand` −92.7 M). One call site, so the attribute adds no code |
| `Board::try_read_prg` | PR-1's story on `cpu_read_override`, on the next method along: it left `Cartridge::default_cpu_read` (−283 M, to **zero**) for a call from `try_cpu_read` (+133 M), **net +66 M on `mmc5exram`**, 0.58 % of that ROM |

### Callgrind: thirteen ROMs, 300 frames, every one below the baseline

`tools/perf/make-tree.sh` then `build-clean.sh` — a `git archive`/`git ls-files` tar extracted
into a brand-new directory with its own `CARGO_TARGET_DIR`, never reused — then
`callgrind-set.sh`, 300 frames, no warm-up, 7 jobs. The baseline run reproduces MF-5a step 4's
own table (`vblnmi` 3 328 805 321 against its 3 328 806 010, `mmc5` 11 331 592 771 against
11 331 624 233, `smb3` 7 967 683 371 against 7 967 706 566), which is the check that the
instrument has not drifted between the two sessions.

| ROM | board | `a577f336` | MF-5b step 1 | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `oamstr` | NROM | 3 339 425 956 | 3 318 333 487 | −21 092 469 | **−0.632 %** |
| `smb1` | NROM | 6 032 357 668 | 6 000 534 471 | −31 823 197 | **−0.528 %** |
| `mm2` | MMC1 | 6 038 205 780 | 6 017 258 311 | −20 947 469 | **−0.347 %** |
| `m4` | MMC3 | 5 500 894 936 | 5 482 296 747 | −18 598 189 | **−0.338 %** |
| `m1` | MMC1 | 4 914 873 709 | 4 900 179 528 | −14 694 181 | **−0.299 %** |
| `readbuf` | CNROM | 6 852 861 485 | 6 835 928 072 | −16 933 413 | **−0.247 %** |
| `duelito` | NROM | 7 398 916 294 | 7 380 745 077 | −18 171 217 | **−0.246 %** |
| `m0` | NROM | 5 544 075 747 | 5 533 133 832 | −10 941 915 | **−0.197 %** |
| `vblnmi` | MMC1 | 3 328 805 321 | 3 322 256 734 | −6 548 587 | **−0.197 %** |
| `mmc5` | MMC5 | 11 331 592 771 | 11 320 534 112 | −11 058 659 | **−0.098 %** |
| `smb3` | MMC3 | 7 967 683 371 | 7 960 907 382 | −6 775 989 | **−0.085 %** |
| `metroid` | MMC1 | 5 154 491 016 | 5 151 052 623 | −3 438 393 | **−0.067 %** |
| `z2` | MMC1 | 6 312 926 031 | 6 312 079 120 | −846 911 | **−0.013 %** |
| **total** | | **79 717 110 085** | **79 535 239 496** | **−181 870 589** | **−0.228 %** |

**The bar is met on every ROM**, and the worst of the thirteen is `z2` at −0.013 %. All five of
R0's commercial games are below the baseline.

### Hot path: the generated code after LTO, and the flag the measurement overrules

`cmp4c.py` and `prologues.py`, `lin-5b-base.s` against `lin-5b-s1f.s`, both from
`cargo rustc --release -p zax-harness --bin zax-harness -- --emit=asm` on clean trees in the WSL2
Ubuntu.

**No pinned symbol's prologue or stack frame moved at all** (`prologues.py` reports not one
`PROLOGUE MOVED`), which is the test for a cost every game pays whatever branch it takes.
`read_cycle` 181, `read_internal` 175, `tick` 82, `run_cpu_cycles` 950, `quiet_cycles` 87,
`run_dma` 992, `dma_read` 75, `background_fetch` 196, `read_pattern_planes` 107 and
`Board::ppu_read` 336 are **IDENTICAL raw**.

| symbol | baseline | step 1 | what moved |
| --- | ---: | ---: | --- |
| `Cpu6502::step` | 932 | 2 529 | absorbed `execute_without_operand`, which has no symbol now |
| `Cpu6502::branch` | 395 | 406 | +11 |
| `Cpu6502::resolve` (2 defs) | 1 800 | 1 847 | +47 |
| `Board::on_cpu_cycle` | 392 | 396 | +4, inside the VRC7's own arm |
| `NesBus::write_internal` | 2 874 | 3 012 | +138, the VRC7's whole register decode in its own arm |
| `Board::ppu_peek`, `on_ppu_address_bus`, `catch_up_ppu` | 252, 156, 216 | same | same size, arms re-ordered |
| `Ppu2C02::run_dots` | 1 907 | 1 906 | −1 |
| `Board::irq_asserted` | *no symbol* | **32, 25 call sites** | see below |

The `branch` +11 and `resolve` +47 are MF-5a step 1's figures to the instruction — extra copies
of the inlined `irq_asserted` block (tail duplication) plus the jump table's thirty-second slot.

**`Board::irq_asserted` gains its first call site, which `callsites.py` prints as
`FIRST CALL SITE -- stop the lot`, and callgrind overrules it.** All twenty-five sites are inside
`Cpu6502::step` and the arithmetic on `m1` closes exactly:

    execute_without_operand   -92,725,930  (folded into step)
    step                      +58,425,018
    Board::irq_asserted       +18,811,584  (0 -> out of line, ~2.1 instructions per CPU cycle)
    net                       -15,489,328  against the -14,694,181 measured

Trading twenty-five *inlined* copies of a thirty-two-arm dispatch for twenty-five calls to one
shared copy is what let `execute_without_operand` fold back into `step`, and that is worth 34 M
more than the calls cost. The rule did its job — it named a real change in the generated code —
and the instrument that decides is the instruction count.

### Tests

| check | result |
| --- | --- |
| `cargo fmt --check` | clean |
| `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings` | clean. **Four new `allow(clippy::inline_always)`**, one per attribute above, each with its measured reason on the function |
| `cargo test --release -p zax-nes -p zax-harness -- --test-threads=1` | **794 passed, 0 failed, 8 ignored** |

794 is the baseline's 782 plus this step's twelve, in `tests/vrc7_tests.rs`:

| test | what it pins |
| --- | --- |
| `req_core_100_vrc7_switches_three_8_kib_prg_banks_and_wires_the_last` | `$8000`, `$8010`, `$9000` at 8 KiB, six bits, the wired last bank, and that `$E000` is not a bank register |
| `req_core_100_vrc7_ignores_a5_to_a11_in_a_register_address` | `$8F60` and `$8EC0` are `$8000`, on both variants |
| `req_core_100_a_vrc7a_selects_registers_with_a4_and_a_vrc7b_with_a3` | the one place the two variants differ without the sound: `$8008` and `$8010` swap roles |
| `req_core_100_a_vrc7_without_a_submapper_answers_to_both_lines` | the OR'd reading, including A3 and A4 both set being the second register and not a third |
| `req_core_100_vrc7_switches_eight_1_kib_chr_banks` | the eight registers, two per page, on both variants |
| `req_core_100_a_vrc7_chr_register_takes_the_whole_byte` | bits 4-7 of a CHR register reaching the window |
| `req_core_100_vrc7_picks_its_arrangement_from_the_two_low_bits_of_e000` | the page's four mirroring rows, and the high bits changing none of them |
| `req_core_100_a_vrc7_is_vertical_at_power_on_whatever_the_header_says` | the power-on override of the header, and a reset putting it and the banks back |
| `req_core_100_vrc7_enables_prg_ram_with_bit_7_of_e000` | `R` gating $6000-$7FFF, and that disabling hides without losing |
| `req_core_100_the_vrc7_reaches_the_vrc_irq_through_its_own_three_registers` | `$E010`, `$F000` and `$F010`, and that `$E000` is the control register and not the latch |
| `mf_5b_the_vrc7_and_the_vrc6_share_one_irq_counter` | a mapper-24 VRC6 and a mapper-85 VRC7 firing on the **same cycle** from the same latch through two different decodes |
| `req_core_100_the_vrc7_counter_runs_on_the_scanline_prescaler_too` | the ÷341/3 prescaler through this board's decode |

`amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state`,
`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`,
`mf_4c_the_write_latch_is_the_read_latch_minus_the_boards_that_wait_on_rd` and
`mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched` sweep `SUPPORTED_MAPPERS`, so all four
cover mapper 85 without being edited — and the last one passes with 85 **absent from its expected
list**, because a VRC7 whose IRQ has not been enabled moves on no CPU cycle. Step 2 changes that.
`req_core_100_every_required_mapper_is_registered` has 85 added to the list of numbers this build
adds beyond REQ-CORE-100's table.

**Save states**: the board's block is new, and no existing block moved. No `unsafe`.

## Status

- [x] step 1 — the VRC7 board, mapper 85. **Every one of the thirteen ROMs is below `a577f336`**,
      total **−0.228 %**, after the `NesBus::irq_asserted` outlining this step found and the four
      attributes that undo it
- [ ] step 2 — the VRC7 audio
- [ ] step 3 — save states
- [ ] step 4 — the ROM check: **no mapper-85 image exists** (the sweep is above)
- [ ] step 5 — the suites and the campaign
