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
bit, and the FM core has **five** quantities no allowed source gives numerically, plus a sixth
question that is not a number (Q-129).

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

## Step 2 — the VRC7 audio: the OPLL subset, from the documentation alone

### The code

`apu/vrc7_audio.rs`, **new**: the two tables, the twelve operators, the six channels and the chip.
`apu/mixer.rs` gains `VRC7_CHANNELS`, `VRC7_UNIT`, `VRC7_CHANNEL_DC`, `VRC7_MAX` and
`vrc7_level`. `vrc7.rs` carries the chip, routes `$9010` and `$9030` to it, clocks it from
`on_cpu_cycle` and answers `expansion_audio_output`.

**No Mesen 2 header on the new file, and that is the whole story of this step.**
`Audio/Vrc7Audio.h` is 91 lines around `OPLL_calc`; `emu2413` lives in `Core/Shared/Utilities`,
which `third_party/README.md` says was not copied. So this is the first expansion chip in the
build with **one** source instead of two, and the five quantities no allowed source gives
numerically are **Q-129**.

### The unit, which is the one design decision everything else follows from

**1/256 of an octave of amplitude**, about 0.0235 dB. Not arbitrary: it is the coarsest unit in
which every documented quantity is a whole number —

| documented quantity | units |
| --- | ---: |
| one envelope step, 0.375 dB (the reverse-engineering notes' measurement) | 16 |
| one total-level step, 0.75 dB | 32 |
| one sustain-level step, 3 dB | 128 |
| one carrier volume step, 3 dB | 128 |
| KSL at 1.5 / 3 / 6 dB per octave | 64 / 128 / 256 per octave |

— and in which the exponential becomes a 256-entry table plus a shift, so **there is no division
anywhere on the per-sample path**. The 0.375 dB step comes out at 0.37629 dB, 0.3 % high: over
the whole 128 levels that is 48.2 dB where the notes measured 48.0.

The two tables are built by `const fn` from `e^x`, `sin x` and `ln y` written as their own series,
because `f64::exp`, `f64::sin` and `f64::ln` are not `const fn` — the same reason
`mixer::SUNSOFT_5B_RATIO` is a written-out literal. `f64::from` is not `const` either, so the
casts are `as f64` on `u32` and `i32`, both of which are lossless and so draw no lint.

### Three things the documents pin that look like they would need a guess

1. **The envelope's advance patterns.** The reverse-engineering notes give the law
   (`eg_shift = 13 - rate/4`, four densities of 4/8, 5/8, 6/8 and 7/8) and, separately, a
   duration formula `cycles = (1 << (14 - rate/4)) × s[rate & 3]` with `s = {127, 102, 85, 73}`
   that "reconstructs the datasheet table precisely". A 127-level decay at density `d` takes
   `127 × 2^eg_shift / d` samples, so `s[n] = 127 × 0.5 / d[n]` — and 127 × 0.8, 127 × 2/3 and
   127 × 4/7 are **102, 85 and 73 to the unit**. That fixes the count of ones in each row; the
   notes' other measurement, "Rate 14: pattern of 2048, 1024, 1024 samples", fixes the positions,
   because `0,1,1,1,0,1,1,1` is the only 6-in-8 arrangement whose gaps repeat 1, 1, 2. The
   derivation is on `EG_PATTERNS` so the next reader does not take the table on trust.
2. **The two LFO rates.** The page's test register says the LFOs are normally stepped "once every
   several samples" and that bit 3 makes them "64x faster" for the tremolo and "1024x faster" for
   the vibrato. With the manual's 3.7 Hz, `49716 / (64 × 210) = 3.699 Hz` **fixes the tremolo's
   triangle at 210 steps**; the vibrato comes out at 6.07 Hz against the manual's 6.36 at the
   VRC7's own 3.58 MHz.
3. **The phase accumulator's width.** The page's "49716 Hz * freq / 2^(19 - octave)" says it
   outright: 19 bits, and the increment `(fnum × 2 × multiplier) << block >> 1` is an integer with
   no rounding at all.

### The envelope's floor is silence, and a test is what established it

The envelope has 128 levels of 0.375 dB, which runs out at 48 dB and not at nothing. Taken
literally, a released note — and a note whose attack rate is `$0`, which the page calls a "halt" —
would sound faintly for ever: `req_core_075_a_vrc7_attack_of_zero_never_sounds` measured it at 18
units of swing where it expected none. The bottom level is the converter running out of bits, so
`Operator::attenuation` answers full attenuation there, and both tests now see exact silence.

### Q-128: the level, and the save state that corrected the first version of it

The VRC7's page gives **no** level at all — not a ratio, not a resistor value, not even the
qualitative "very loud" the Sunsoft 5B's page offers. So the anchor is Q-118's rule, which R0
approved for the 5B and which the VRC6's page then confirmed in its own words, applied the one way
an FM channel allows: **one channel at full output has the peak-to-peak swing of one APU pulse at
volume 15**. A pulse at volume 15 alternates between 0 and 4 890, so its amplitude about its own
mean is 2 445; a sine of amplitude 2 445 has the same swing. Six channels reach 29 340.

**An FM carrier is bipolar, and `M` has to be positive** (AMD-061 §6.2 and Q-118: `clip`
implements only the lower bound *because* `M` cannot go negative). The first version put a
**constant** offset of 14 670 on `M` — and
`req_core_090_the_mapper_state_survives_a_save_and_restore` failed on mapper 85, with three bytes
of an `f64` at −0.4476 against 0. That `f64` is the high-pass filter's own state, −14 670 / 32 768:
**REQ-CORE-074 clears the audio pipeline on a restore**, so a standing offset makes
save-restore-save differ. The offset is therefore carried **per sounding channel** — an idle
channel contributes nothing — so a chip with no note playing has an `M` of exactly 0, like every
other expansion chip in the build, and what offset remains while a note sounds is removed by the
high-pass filter REQ-CORE-073 puts at the end of the chain. At the worst `M`, `A − M` has 3 428
units of margin under `MIN_SCALE`.

That test is worth naming as a finding: **it catches a class of bug no audio test would**, because
it is about the savestate and not about the sound.

### The two attributes, each with its measurement

| attribute | what it buys |
| --- | --- |
| `#[inline(never)]` on `Vrc7Audio::write` | `NesBus::write_internal`'s frame went from `subq $56, %rsp` to `subq $104, %rsp` — the same **one** instruction, so no count moved, but 48 more bytes of stack touched on every CPU write of every board. MF-3b rule 3's shape. With the attribute the arm is a tail call and the frame is step 1's |
| `#[inline(never)]` on `Vrc7Audio::run_sample`, with `step_cpu_cycle` left `#[inline]` | the shape MF-5a's note on `Vrc6Audio` asks for. The VRC6 had no cheap early-out — its three oscillators run on the CPU clock itself — so its whole body had to leave; this chip runs at the board clock **divided by 36**, so the divider inlines as three instructions and the sample does not |

### Callgrind: thirteen ROMs, 300 frames

| ROM | board | `a577f336` | MF-5b step 2 | delta | | vs step 1 |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `oamstr` | NROM | 3 339 425 956 | 3 318 333 247 | −21 092 709 | **−0.632 %** | +0 |
| `smb1` | NROM | 6 032 357 668 | 6 000 553 684 | −31 803 984 | **−0.527 %** | +19 213 |
| `mm2` | MMC1 | 6 038 205 780 | 6 017 246 350 | −20 959 430 | **−0.347 %** | −11 961 |
| `m4` | MMC3 | 5 500 894 936 | 5 482 284 501 | −18 610 435 | **−0.338 %** | −12 246 |
| `m1` | MMC1 | 4 914 873 709 | 4 900 174 720 | −14 698 989 | **−0.299 %** | −4 808 |
| `readbuf` | CNROM | 6 852 861 485 | 6 835 939 505 | −16 921 980 | **−0.247 %** | +11 433 |
| `duelito` | NROM | 7 398 916 294 | 7 380 753 621 | −18 162 673 | **−0.245 %** | +8 544 |
| `m0` | NROM | 5 544 075 747 | 5 533 139 316 | −10 936 431 | **−0.197 %** | +5 484 |
| `vblnmi` | MMC1 | 3 328 805 321 | 3 322 245 015 | −6 560 306 | **−0.197 %** | −11 719 |
| `mmc5` | MMC5 | 11 331 592 771 | 11 311 616 787 | −19 975 984 | **−0.176 %** | −8 917 325 |
| `smb3` | MMC3 | 7 967 683 371 | 7 960 904 211 | −6 779 160 | **−0.085 %** | −3 171 |
| `metroid` | MMC1 | 5 154 491 016 | 5 151 068 445 | −3 422 571 | **−0.066 %** | +15 822 |
| `z2` | MMC1 | 6 312 926 031 | 6 312 078 696 | −847 335 | **−0.013 %** | −424 |
| **total** | | **79 717 110 085** | **79 526 338 098** | **−190 771 987** | **−0.239 %** | **−8 901 398** |

**All thirteen are below the lot's baseline**, and against step 1 the six that move up do so by
424 to 19 213 instructions — one to two orders of magnitude inside the ~10⁵ whole-process floor
PR-1 step 3 measured. The one symbol that moves by 1.5 M or more anywhere in the set is
`NesBus::push_expansion_audio` on `mmc5`, **−8 934 160**, which is a gain.

### Hot path: the generated code after LTO

`cmp4c.py` and `prologues.py`, `lin-5b-s1f.s` against `lin-5b-s2b.s`. **Exactly two symbols
differ, and every one of the twenty pinned prologues and stack frames is instruction-identical
(`prologues.py` reports twenty `SAME PROLOGUE` and no `MOVED`):**

| symbol | step 1 | step 2 | what moved |
| --- | ---: | ---: | --- |
| `NesBus::write_internal` | 3 012 | 3 020 | +8, the tail call into `Vrc7Audio::write` inside the mapper-85 arm |
| `Board::irq_asserted` | 32 | 32 | same size, arms re-ordered |
| everything else pinned | | | `Cpu6502::step` 2 529, `branch` 406, `resolve` 1 847, `read_cycle` 181, `read_internal` 175, `tick` 82, `catch_up_ppu` 216, `run_dma` 992, `dma_read` 75, `run_cpu_cycles` 950, `quiet_cycles` 87, `run_dots` 1 906, `background_fetch` 196, `read_nametable_inner` 114, `read_pattern_planes` 107, `Board::ppu_read` 336, `ppu_peek` 252, `on_cpu_cycle` 396, `on_ppu_address_bus` 156 — **all IDENTICAL** |

**Bounds checks: 911 against the baseline's 911, identical.** No watched callee moved.

### Tests

| check | result |
| --- | --- |
| `cargo fmt --check` | clean |
| `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings` | clean, **no new `allow`** beyond one `struct_excessive_bools` on `OperatorPatch`, whose six bits the page names one by one |
| `cargo test --release -p zax-nes -p zax-harness -- --test-threads=1` | **809 passed, 0 failed, 8 ignored** |

809 is step 1's 794 plus this step's fifteen, in `tests/vrc7_audio_tests.rs`:

| test | what it pins |
| --- | --- |
| `q_128_a_vrc7_channel_at_full_output_swings_like_an_apu_pulse_at_volume_15` | the Q-128 level, all four constants recomputed from `PULSE_TABLE`, and that nothing sounding is an `M` of **exactly 0** |
| `req_core_075_a_vrc7_is_silent_until_a_channel_is_keyed_on` | power-on silence, and that a volume without a trigger bit is still silence |
| `req_core_075_a_keyed_vrc7_channel_makes_a_tone_that_swings_both_ways` | the key-on, and the carrier's sine going both sides of its own offset |
| `req_core_075_a_vrc7_volume_of_f_is_the_quietest_and_0_the_loudest` | the inverted four-bit volume at 3 dB a step |
| `req_core_075_a_vrc7_note_releases_to_silence_when_the_trigger_clears` | the key-off, and that the release reaches exact silence rather than a floor of 47.6 dB |
| `req_core_075_a_vrc7_attack_of_zero_never_sounds` | "attack $0 = halt", through `RATE = 0`: the envelope never leaves the floor |
| `req_core_075_a_faster_vrc7_attack_reaches_its_peak_sooner` | attack `$F` against `$5`, measured in cycles to a quarter of the peak |
| `req_core_075_the_vrc7_sustain_bit_of_2x_forces_the_release_rate_to_5` | `$2X`'s `S` overriding instrument 1's own release of `$7` with the slower rate 5 |
| `req_core_075_the_fifteen_vrc7_instruments_are_the_pages_own_bytes` | all fifteen sound, and no two produce the same 256 samples — which is what a duplicated row of the patch table would do |
| `req_core_075_instrument_0_is_the_custom_patch_and_follows_00_to_07_live` | instrument 0 reading `$00`-`$07`, and a change to `$03`'s `Q` bit heard **while the note sounds** |
| `req_core_075_the_vrc7_tremolo_runs_at_3_7_hz_and_the_vibrato_at_6_07` | the two cadences by their arithmetic, so a change to any of the three constants has to come and change this test |
| `req_core_075_the_vrc7_tremolo_modulates_the_amplitude_and_the_vibrato_the_pitch` | the tremolo lowering the energy of a whole LFO period, and the vibrato changing the waveform |
| `req_core_075_the_vrc7_sums_its_six_channels_into_one_dac` | six channels far louder than one, and `$16`/`$26`/`$36` not being a seventh |
| `req_core_075_e000_bit_6_silences_the_vrc7_and_disregards_its_writes` | `M` going to 0, the writes being dropped while it is set, and the chip resuming where it was |
| `req_core_075_a_vrc7_subtracts_its_m_from_the_apu_mix` | `S = A − M` at the loudest `M`, through the mixer's own test point |

Three of those fifteen were **wrong when first written**, and each failure was a lesson worth
keeping: the extremes of a long window are the same with and without tremolo (so the test measures
energy over an LFO period instead); two instruments at the same note and volume reach the **same**
peak, the carrier's level being the channel volume and not a patch byte (so the test compares 256
samples instead); and the vibrato's first LFO step bends a nine-bit F-number of 384 by `384 >> 9`,
which is nothing at all, so a window shorter than a few LFO steps compares two identical stretches
(the window is now 4 096 samples).

`mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched` **did** need editing, and what it caught
is the same thing MF-5a's step 4 caught: the chip's sample divider counts down from power-on with
no register written, like the FME-7's 5B prescaler and the Namco 163's multiplexer, so 85 joins
its expected list — which is now `[5, 5, 19, 19, 69, 69, 85, 85]`.

**Save states**: `Vrc7Audio::save_state` is appended after the IRQ, the shape `fme7.rs`,
`vrc6.rs` and `namco163.rs` established, and `load_registers` tolerates a block without it. The
two `OperatorPatch`es per channel are **not** serialised — they are decoded from `$3X` and the
custom patch, both of which are, so `load_state` rebuilds them exactly as a write would, and
there is one fewer thing that can come back out of step with itself. `level` **is** serialised,
for the reason `Namco163Audio::level` records: recomputing it would need the waveform evaluated
without advancing it, and one `i32` is cheaper than that.

## Step 3 — the save states, and the one existing test that found a real bug

Steps 1 and 2 each wrote their own half of the block; this step is where the **behaviour** of it
is pinned, because the format being right and the machine resuming are two different claims. **No
production code changed**, so the measured binary of step 2 stands and no new instruction count
was taken.

### Nothing in the format moved

The board's block is new. No existing block gained, lost or reordered a field. Three things are
deliberately *not* serialised, each with its reason in the code:

* the two `OperatorPatch`es of each channel — decoded from `$3X` and from the custom patch, both
  of which *are* in the block, so `load_state` rebuilds them exactly as a write would;
* `Cartridge::clocks_on_cpu_cycle` — derived from the mapper number at load, like its five
  neighbours;
* the four `bool`s the board derives from `$E000` (`prg_ram_on`, `silenced`) and the mirroring,
  which `Cartridge::load_state` has already restored from the state — recomputing the arrangement
  here would overwrite the saved one with whatever `$E000` implies.

One thing *is* serialised although it is derived — `Vrc7Audio::level` — for the reason
`Namco163Audio::level`'s field doc gives: recomputing it would need the whole waveform evaluated
without advancing it, and one `i32` makes the restored `M` exactly the saved one.

### What the seven tests prove rather than assert

| test | what it pins |
| --- | --- |
| `req_core_090_a_vrc7_state_round_trips_through_the_fm_chip` | **600 cycles of `M` identical** after a restore, with four channels on four instruments (one of them the custom patch) and the machine deliberately driven elsewhere in between: twelve phase accumulators, twelve envelopes and their phases, two LFO counters and their dividers, the shared envelope counter, the sample divider and the cached `M`, in one assertion |
| `req_core_091_a_vrc7_state_brings_back_the_custom_patch_and_the_port` | the eight custom bytes through 300 cycles of `M`, and `$9010`'s latched address checked by **writing through it** after the restore |
| `req_core_091_a_block_written_before_the_vrc7_sound_still_loads` | the tolerance `load_registers` claims, tested by **truncating** the block |
| `req_core_090_a_vrc7_state_round_trips_its_arrangement_and_banks` | the three PRG banks, the eight CHR banks, the arrangement coming back **from the state** and not from `$E000`, and the PRG-RAM enable |
| `req_core_091_a_vrc7_state_brings_back_the_irq_counter` | the shared `VrcIrq` mid-count, and that it fires from where the state left it and not from the latch |
| `req_core_092_a_silent_vrc7_saves_and_restores_byte_for_byte` | the Q-128 property: a silent chip has an `M` of exactly 0, so the round trip is byte-identical |
| `mf_5b_a_sounding_expansion_chip_is_not_byte_identical_on_any_board` | that the *sounding* case is the APU's own load path and not this board's, demonstrated on a **VRC6 as well as** the VRC7 |

### The tolerance test had to be rewritten, and the reason is worth recording

`vrc6_audio_tests.rs` tests the same claim by relabelling the block `STATE_VERSION_4`. That does
not work here, and thinking about why is what produced the right test: a VRC7 state written at
step 1 is not a differently-*versioned* block, it is a **shorter** one — no format version sits
between the two steps, and what distinguishes them is that step 1's `save_registers` stopped
after the IRQ. `reader.remaining() > 0` is exactly the right test for that, and it works because
the mapper's registers are the last thing in the whole state. So the test truncates the block by
the 253 bytes `Vrc7Audio::save_state` writes and requires the load to **succeed** with a silent
chip — a truncation there is not a corruption.

### The bug an existing test found, which no audio test would have

`req_core_090_the_mapper_state_survives_a_save_and_restore` sweeps every supported mapper and
asserts that a save, five frames, a restore and a second save give the **same bytes**. It failed
on mapper 85 with three bytes of an `f64` at **−0.4476** against 0 — and −0.4476 is
−14 670 / 32 768.

That `f64` is the APU's cached mixer level, and `Apu2A03::load_state` re-derives it with `M`
forced to 0, deliberately and with its reason on the line
(`self.mixer_output = mixer::clip(self.mixer_apu, 0)`, "`M` … not in the block … the bus pushes
`M` again before the next cycle"). The first version of `vrc7_level` put a **standing** DC offset
of 14 670 on `M`, so a VRC7 making no sound at all still reported one — and the round trip
differed.

**The fix made the sound better, not just the test green**: the offset is carried per *sounding*
channel, so a chip with nothing playing has an `M` of exactly 0, like the other four expansion
chips in the build, and the offset that remains while a note sounds is what REQ-CORE-073's
high-pass filter is for. The whole argument is Q-128.

Two things are worth taking from it. First, **a savestate test caught an audio design error**,
which is not where anyone would have looked. Second, the asymmetry it exposed is *general*: a
state saved while any expansion chip is sounding is not byte-identical on an immediate re-save,
on the MMC5, the VRC6, the Namco 163 and the 5B alike. The second test above demonstrates it on a
VRC6 so that nobody reads the VRC7's silence requirement as a stronger invariant than it is.

| check | result |
| --- | --- |
| `cargo fmt --check` | clean |
| `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings` | clean, no new `allow` |
| `cargo test --release -p zax-nes -p zax-harness -- --test-threads=1` | **816 passed, 0 failed, 8 ignored** |

## Step 4 — the ROM check: there is none to run, re-established independently

The sweep at the head of this file was **re-run from scratch** rather than taken from MF-5a on
trust, because the claim is load-bearing: every behavioural assertion in this lot rests on it.
Every `.nes` under `assets-free/test-roms` was opened and its iNES header decoded, the NES 2.0
submapper byte included. **330 images carry a valid header**, and between them

    mappers 0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 22, 28, 34, 66, 69, 78, 118, 180

which is MF-5a step 7's list to the image.

| mapper | images |
| --- | ---: |
| 85 (VRC7) | **0** |
| 19, 24, 26, 210 (MF-5a's four) | 0 each, unchanged |

`holy-mapperel` has no `M85` target — its 43 files stop at `M180` — and no file anywhere in the
tree has `vrc` or `85` in its name.

**So this lot cannot be accepted by a test ROM.** What stands in place of one: the thirty-four
unit tests of steps 1 to 3, written against the nesdev pages, the YM2413 application manual and
the YM2413 reverse-engineering notes, with the two provisional choices pinned by name; and
**R0's two commercial titles**, which are the only images in reach that run this board —
*Lagrange Point* (submapper 2, the VRC7a, the one game with FM music) and *Tiny Toon Adventures 2*
(J) (submapper 1, the VRC7b). Q-128 and Q-129 both name what those runs would settle, and Q-129's
sixth point is specifically about the second of them.

## Step 5 — the suites, the campaign ROM by ROM, and the document

### The suites

| check | result | against `a577f336` |
| --- | --- | --- |
| `cargo fmt --check` | clean | same |
| `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings` | clean | **five new `allow`**: the four `inline_always` of step 1 and one `struct_excessive_bools` on `OperatorPatch` |
| `cargo test --release -p zax-nes -p zax-harness -- --test-threads=1` | **816 passed, 0 failed, 8 ignored** | the baseline's 782 plus this lot's 34 |
| `cargo test --release -p zax-harness --test test_roms_tests -- --test-threads=1` | **14 passed** | the same 14 groups, the file untouched |
| `--ignored`, with both ROM folders | **7 of 8 pass** | the **same 7**, test for test |

The CPU, PPU, APU and mapper suites are all inside the third line; nothing in them was edited
except the two sweeps that had to learn about mapper 85 (step 1's and step 2's notes name both).

**The one `--ignored` failure is not this lot's**:
`builtin_patches::tests::the_zelda_ii_patch_applies_to_the_usa_rom`, which MF-4d-fix named as out
of scope and which MF-5a reported identically. The seven that pass include the two that matter
most here — `golden_pixels_tiles_audio_cycles_and_blargg_match_up_to_frame_1200`, which compares
pixels, tiles, audio **and** cycle counts over 1 200 frames of the commercial set, and
`amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line`, the P1 criterion MF-4d-fix exists
to have restored. **Golden-core is the brief's "no game on another board may move", and it passes
unchanged** — which, for a lot that had to pin four `#[inline(always)]` attributes to reach
parity, is not a formality.

### The campaign, ROM by ROM

`zax-harness test-roms` over `assets-free/test-roms`, against MF-5a's own run:

    331 ROMs, 214 pass, 5 fail, 0 timeout, 1 unsupported, 111 manual

**Identical totals, and identical rows.** `campdiff.py` reports *rows whose verdict, code or
message differs: 0*. Comparing all 331 rows **field by field** — ignoring `.ms`, which is wall
clock and moves on most rows in any two runs, and `.shot`, because this run used `--no-shots` —
**zero rows differ on any field**: verdict, code, message, mapper and detail alike. Inside the
`AccuracyCoin` row the tally stays at **128/147** with the same failing rows in the same order,
which is MF-4d-fix's figure after Q-125.

As in MF-5a, that is stronger than the brief asked for. It asked for "every row identical except
the new board's", and **there are no rows of the new board**: step 4's sweep says the campaign
contains no mapper-85 image, so the expected exception set is empty and the campaign is identical
outright.

### Callgrind on the committed tree

The step-2 measurement was taken on a tree that is **code-identical** to the committed one
(`git diff HEAD -- rust/crates/*/src` is empty from step 3 on, both later steps being tests and
documents), but the lot's end was built and re-run anyway, because that is the number R0 compares
against:

| ROM | board | `a577f336` | MF-5b's end | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `oamstr` | NROM | 3 339 425 956 | 3 318 352 390 | −21 073 566 | **−0.631 %** |
| `smb1` | NROM | 6 032 357 668 | 6 000 539 662 | −31 818 006 | **−0.527 %** |
| `mm2` | MMC1 | 6 038 205 780 | 6 017 224 822 | −20 980 958 | **−0.347 %** |
| `m4` | MMC3 | 5 500 894 936 | 5 482 275 947 | −18 618 989 | **−0.338 %** |
| `m1` | MMC1 | 4 914 873 709 | 4 900 165 436 | −14 708 273 | **−0.299 %** |
| `readbuf` | CNROM | 6 852 861 485 | 6 835 942 036 | −16 919 449 | **−0.247 %** |
| `duelito` | NROM | 7 398 916 294 | 7 380 740 398 | −18 175 896 | **−0.246 %** |
| `m0` | NROM | 5 544 075 747 | 5 533 123 239 | −10 952 508 | **−0.198 %** |
| `vblnmi` | MMC1 | 3 328 805 321 | 3 322 236 008 | −6 569 313 | **−0.197 %** |
| `mmc5` | MMC5 | 11 331 592 771 | 11 311 618 977 | −19 973 794 | **−0.176 %** |
| `smb3` | MMC3 | 7 967 683 371 | 7 960 927 909 | −6 755 462 | **−0.085 %** |
| `metroid` | MMC1 | 5 154 491 016 | 5 150 991 122 | −3 499 894 | **−0.068 %** |
| `z2` | MMC1 | 6 312 926 031 | 6 312 078 643 | −847 388 | **−0.013 %** |
| **total** | | **79 717 110 085** | **79 526 216 589** | **−190 893 496** | **−0.239 %** |

Reproduces step 2's table inside the ±5×10⁴ per-ROM floor on every row, which is the check that
the instrument has not drifted across the lot's four runs.

### The document

`docs/review/MF-T-test-roms.md` gains a "What lot MF-5b changed" section: the identical campaign,
the header sweep with the mapper list, the thirty-four unit tests and the two commercial titles
that stand in for the ROM this tree has not got, the eight ignored tests with their results, and
the regression guard untouched at MF-4d-fix's floor of 128. It also records the thing that makes
this board different from the other four expansion chips — **one source instead of two** — and
that Q-129 is the list of what that costs. The totals table's own note now says MF-4d-fix, MF-5a
and MF-5b moved no row.

## What the lot delivered, against the brief

| brief item | state |
| --- | --- |
| the VRC7 board, mapper 85, submappers 1 and 2 selecting the register address line and 0 decoding both | step 1: `vrc7.rs`, two latched `bool`s and no `match` on the number |
| PRG and CHR banking, mirroring, the PRG-RAM enable, the VRC IRQ **shared**, the audio-silence bit | step 1, with `mf_5b_the_vrc7_and_the_vrc6_share_one_irq_counter` proving the share |
| a unit test for each register | step 1: twelve, one per register group plus the two variants and the power-on override |
| the VRC7 audio: six FM channels, two operators each, fifteen instruments plus the custom one, the envelope, both LFOs, key-on, sustain, no rhythm mode | step 2, built from the documentation alone — **there is no OPLL reference implementation in the copied tree** |
| register writes through `$9010` and `$9030` | step 2, decoded before the register-select rewrite, on every submapper |
| mix through `expansion_audio_output` at the level NESdev gives relative to the APU | **the page gives no level at all**, so **Q-128** opened, pinned by name in a test, exactly as MF-5a did with Q-127 |
| a Q ticket if the sources disagree on the level or the patch set | **Q-128** for the level, **Q-129** for the five quantities no allowed source gives. The patch set is not in doubt: the page tabulates all fifteen in hex |
| the chip at the board clock divided by 36, with as little work per CPU cycle as possible and none at all on other boards | step 2: three instructions and a call, `#[inline(never)]` on the sample; `clocks_on_cpu_cycle` gains 85 so no other board pays anything |
| unit tests on the operator, the envelope and a channel's output | step 2: fifteen, including the attack's halt, the attack's rate ordering, the sustain-bit release rate, and the fifteen instruments proved distinct over 256 samples each |
| save states behind the existing block versioning, with older states still loading | step 3: seven tests, 600 cycles of `M` identical after a restore, and the tolerance tested by **truncating** the block |
| the ROM check | step 4: **no mapper-85 image exists**, re-established by decoding all 330 headers |
| the suites and the campaign ROM by ROM | step 5: **331 rows identical on every field**, 816 / 0 / 8, 7 of 8 ignored, golden-core unchanged |
| no performance regression, none at all | **every one of the thirteen ROMs is below `a577f336`**, total **−0.239 %**; the worst is `z2` at −0.013 % and all five of R0's commercial games are below the baseline |

**Two things are left for R0**, each with a one-constant or one-`bool` fix: **Q-128** (the level,
where one `/ 2` changes the reading) and **Q-129** (the five OPLL quantities, and the VRC7b's
missing resonator). Both need *Lagrange Point* to settle, and both say so.

**One finding is reusable and belongs to no step**: `NesBus::irq_asserted` outlining under a
thirty-second `Board` variant, which cost **+0.542 % on all thirteen ROMs** before four
attributes undid it — the third time in these lots that one more enum variant has tipped an
inliner budget on a per-cycle path (MF-3b step 1, MF-5a step 1, here), and the second time that
`callsites.py`'s watched list has been blind to it.

## Status

- [x] step 1 — the VRC7 board, mapper 85. **Every one of the thirteen ROMs is below `a577f336`**,
      total **−0.228 %**, after the `NesBus::irq_asserted` outlining this step found and the four
      attributes that undo it
- [x] step 2 — the VRC7 audio (**Q-128** opened: the level, which this chip's page does not give
      at all; **Q-129** opened: the five OPLL quantities no allowed source gives, there being no
      reference implementation in reach). **All thirteen below the baseline**, total **−0.239 %**
- [x] step 3 — save states (seven tests; 600 cycles of `M` identical after a restore, and the
      round-trip test that found the standing DC offset of Q-128)
- [x] step 4 — the ROM check: **no mapper-85 image exists**, re-established by decoding all 330 headers
- [x] step 5 — the suites (**816 / 0 / 8**, 7 of 8 ignored, golden-core unchanged) and the campaign (**331 rows identical on every field**), `MF-T-test-roms.md` updated
