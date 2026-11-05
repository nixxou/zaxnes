# Lot MF-5a — the VRC6 and Namco 163 boards, with their expansion audio

Baseline `ad1ad765` (MF-4d's end). **716 tests**, 0 failed, 7 ignored, measured at the lot's start
and reproduced here before anything was written.

Neither board existed: nothing under `crates/zax-nes/src/cartridges/mappers/` handled iNES mappers
24, 26, 19 or 210, and `SUPPORTED_MAPPERS` had none of them. REQ-CORE-075 makes expansion audio
optional (P3) and Q-118 lists the VRC6 and the Namco 163 among the optional chips; the Sunsoft 5B
(`fme7.rs` + `apu::Sunsoft5bAudio` + `Mapper::expansion_audio_output`) is the pattern this lot
follows.

## Steps

1. The VRC6 board (mappers 24 and 26), with the VRC IRQ counter **shared** with `vrc2_vrc4.rs`.
2. The VRC6 audio: two pulses, the sawtooth, and `$9003`.
3. The Namco 163 board (mapper 19; mapper 210 submappers 1 and 2, no audio).
4. The Namco 163 audio: up to eight channels, time-multiplexed, one every 15 CPU cycles.
5. Save states for both boards and both audio blocks, behind the existing block versioning.
6. The ROM check.
7. The suites, the campaign ROM by ROM against MF-4d's, and the campaign document.

## The ROM situation, settled first because it shapes every step

**There is no VRC6 and no Namco 163 ROM anywhere in `assets-free/test-roms`.** Not a naming
accident: every `.nes` under that tree was opened and its header decoded —

    330 images, mappers 0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 22, 28, 34, 66, 69, 78, 118, 180

Mappers 19, 24, 26 and 210: **zero images each**. `holy-mapperel` has no `M24`/`M26`/`M19` target
(its highest is `M180`), and `m22chrbankingtest` is a VRC2a, which mapper 22 already covers.

So this lot cannot be driven by a ROM at any step, and that is stated here rather than discovered
at step 6: **every behavioural claim below rests on a unit test written against the nesdev page,
cross-checked against `third_party/mesen2`.** R0 runs the commercial titles (*Akumajou Densetsu*,
*Madara*, *Esper Dream 2* for the VRC6; *Megami Tensei II*, *Erika to Satoru no Yume Bouken*,
*King of Kings* for the 163).

## Step 1 — the VRC6 board, mappers 24 and 26

## Instrument

Three measures, and the lot needs all three because each is blind to what the others see.

1. **Post-LTO generated code** (MF-3b rule 2), `cargo rustc --release -p zax-harness --bin
   zax-harness -- --emit=asm`, compared with MF-4c's `cmp4c.py`. The baseline dump is
   `bin-4d-s6b.s`, and it **is** `ad1ad765`'s: `git diff --stat a758f923 ad1ad765` touches
   `docs/progress/MF-4d.md`, `docs/review/MF-T-test-roms.md`,
   `rust/docs/PERFORMANCE-HISTORY.md` and `zax-harness/tests/test_roms_tests.rs` — four files,
   three of them documents and the fourth a test, none of them compiled into the `zax-harness`
   **binary**. So no baseline rebuild was needed for the code comparison.
2. **Call sites per callee** (MF-3b rule 1), `target-mf/asm/callsites.py`, added by this lot:
   a callee going from 0 call sites to N is a regression whose instruction count does not move.
3. **Callgrind**, PR-1's `rust/tools/perf/`, thirteen ROMs, 300 frames, no warm-up, both sides
   built by `build-clean.sh` from a `git archive`/`git ls-files` tar into a brand-new directory
   with its own `CARGO_TARGET_DIR`. This is the measure the bar is written against.

Two helpers were added beside `cmp4c.py`, because the verdict column says *that* something moved
and never *what*: `showdiff.py` (the instruction-level hunks of one symbol) and `prologues.py`
(the first N instructions of each pinned symbol — a `pushq` or a bigger `subq $N, %rsp` is a cost
to every caller whatever branch it then takes, where extra arm blocks further down are not).

The baseline callgrind run reproduces MF-4d's own table: `smb3` 8 103 531 708 against its
8 103 504 411, `duelito` 7 198 774 119 against 7 198 718 334, `mmc5` 11 402 398 394 against
11 402 403 015 — all inside the ~10⁵ whole-process noise floor PR-1 step 3 measured.

## The code

`vrc_irq.rs`, **new**: `VrcIrq`, the counter Konami put in the VRC4 and the VRC6 — the seven
fields and three methods `vrc2_vrc4.rs` carried inline. The brief asked for it to be shared
rather than copied, and the save-state layout is the one `Vrc2Vrc4::save_registers` already
wrote — latch, counter, prescaler, then the four flags, in that order — so **a mapper
21/22/23/25/27 state written before this lot loads byte for byte** (REQ-CORE-091). The decode
stays on each board: a VRC4 reaches the four registers through two address lines and writes the
latch in nibbles, a VRC6 has them at `$F000`/`$F001`/`$F002` and writes it whole.

`vrc6.rs`, **new**: mappers 24 and 26. PRG (a 16 KiB window and an 8 KiB one, the last bank
wired), CHR (the four arrangements of `$B003`'s `D` and `P`), the nametables, the PRG-RAM enable
(`W`), and the IRQ. Mapper 26 is one `address` rewrite — "VRC6b (mapper 026) has A0 and A1
swapped" — decided by a load-time `bool`, not a `match` on the number.

`registry.rs`: `24 | 26 => Board::Vrc6(...)`, and `SUPPORTED_MAPPERS` gains 24 and 26.
`board.rs`: the variant is **appended**, as that file's own note requires.
`cartridges/mod.rs`: `nametable_pages_vary` gains 24 and 26, because `$B003` and the CHR
registers between them pick a CIRAM page per table.

### `$B003`, and the two vocabularies

nesdev's `$B003` tables name the four fixed arrangements "vertical / horizontal / 1-screen A /
1-screen B" *and* give the CIRAM A10 source of each. For mode 0 the two namings agree with this
crate's. For **mode 3** they do not, and the page's own table rows are what settle it — re-derived
here one column at a time from its even/odd entries:

| `$B003 & $2F` | the page's rows | CIRAM pages | this crate |
| --- | --- | --- | --- |
| $20, $27 | R6e R6o R7e R7o | 0 1 0 1 | `Vertical` |
| $23, $24 | R6e R7e R6o R7o | 0 0 1 1 | `Horizontal` |
| $28, $2F | R6e R6e R7e R7e | 0 0 0 0 | `SingleScreenA` |
| $2B, $2C | R6o R7o R6o R7o | 1 1 1 1 | `SingleScreenB` |

So the page calls `$23` "vertical pairs 2 KiB spread" and this crate calls the same wiring
`Horizontal`; `fme7.rs` carries the same note about the same inversion. Mesen 2 reads every row
the same way (`VRC6.h:174-194`), which is two independent sources agreeing, and the derivation
above is the third.

### Q-123: `$B003` bit 4 is decoded and ignored, for a measured reason

`N` set means "Nametables come from CHRROM". **It is not implemented**, and the reason is not a
reading of the page: a *third* board overriding `Mapper::read_nametable` costs every other board
two instructions and 8 bytes of stack **per nametable fetch**.

`Ppu2C02::read_nametable_inner` folds that `dispatch!` in behind its
`page == SERVED_BY_CARTRIDGE` test. With two distinct arm bodies — the MMC5's `ExRAM`/fill and
the UNROM 512's four-screen CHR-RAM — LLVM answers "not me" with a bitset. Five shapes were
built and measured:

| | `read_nametable_inner` |
| --- | ---: |
| baseline (MF-4d) | **132**, bitset |
| the VRC6's arm as first written | 83 — the dispatch leaves the function by a tail call |
| `#[inline(always)]` on `<Board as Mapper>::read_nametable` | 80 — the boundary moves to `Cartridge::read_nametable` |
| `#[inline(always)]` on both | 109 — folded back in, but as a **jump table**, and the MMC5's arm goes out of line |
| the arm shrunk to one indexed load (the UNROM 512's own shape) | 116, and the prologue grows to `pushq %rsi / pushq %rbx / subq $40` |
| **no arm for the VRC6** | **132, `IDENTICAL` to the baseline** |

The trigger is the **third distinct body, not its size** — the fifth row is what proves that, and
it is why no amount of shrinking the arm would have worked. The attributes were taken back out;
the measurement is written on `<Board as Mapper>::read_nametable` so the next lot reads it before
adding a fourth board, and the long form is in **Q-123**.
`q_123_vrc6_keeps_its_nametables_on_ciram_when_n_asks_for_chr_rom` pins the provisional choice —
`N` changes nothing, and the answer is never `SERVED_BY_CARTRIDGE` — so a lot that implements the
mode has to come and rewrite it.

### The bug the reset sweep caught

`amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` failed on mapper 24
the first time it ran: `[0, 1, 2, 15]` at power-on against `[6, 7, 6, 15]` after a reset. The
first draft mapped the PRG windows straight from `write_register` and kept no register, so
`Mapper::reset` had nothing to put back — `BankWindows` has no memory of what was written into
it. `prg_16k` and `prg_8k` are now fields and `update_banks` maps all four windows from them.
Their power-on values (0 and 2) are the banks the windows would show with no board logic at all,
which is what Mesen comes up with too; nothing on the page gives them, and every VRC6 game writes
both registers in its reset handler.

## The latch that paid for the board: `Cartridge::mapper_number`

The post-LTO diff put `Ppu2C02::run_dots` at 1 936 → 1 948, **+12**, and callgrind named the
path: `run_dots` up on eleven of the thirteen ROMs and **exactly 0 on `oam_stress` and
`ppu_vbl_nmi`**, the two that never emit a sprite. One instruction per sprite emitted, on every
board.

The cause is `Ppu2C02::capture_sprites`, which asks `Cartridge::mapper_number()` once per sprite
batch — mappers 9 and 10 sort their sprites by x (REQ-CORE-044) and nothing else does — and
`Board::mapper_number` is the one dispatch whose arms return a **per-variant constant**. LLVM
compiles it as a bitset over the boards answering with a literal, plus a field read at the offset
of `mapper_number` for the boards that carry one. Two distinct field offsets fit that shape;
`Vrc6` was a third, and the bitset became a jump table. In the generated code, inside `run_dots`:

    movzbl  1600(%r15), %eax        movzbl  1600(%r15), %eax
    subb    $2, %al                 subb    $2, %al
    movzbl  %al, %eax               movzbl  %al, %eax
    movl    $5, %ecx                movl    $5, %ecx
    cmovbl  %ecx, %eax              cmovbl  %ecx, %eax
    movzbl  %al, %ecx               movzbl  %al, %eax
    movl    $352, %eax              leaq    .LJTI_2(%rip), %rcx
    movl    $264232811, %edx        movslq  (%rcx,%rax,4), %rax
    btl     %ecx, %edx              addq    %rcx, %rax
    jae     .LBB                    jmpq    *%rax

The fix is MF-3b rule 5, and it is the fifth latch on `Cartridge` beside `rom_windows`,
`nametable_pages_vary`, `bus_conflicts` and the two PPU-bus ones: a `mapper_number: i32` field
set in `Cartridge::create` from the number the registry was already handed, and
`Cartridge::mapper_number` is now `const fn` returning it. The question cannot change answer, so
nothing is cached that could go stale, and no savestate byte moves.

It takes the board **below** parity rather than level with it, because the dispatch it removes was
there before this lot: `run_dots` 1 936 → **1 932**, four instructions smaller, with `btl` 8 → 6,
`cmovbl` 3 → 2, `jae` 14 → 13, `jb` 5 → 4, `subb` 2 → 1 and `movzbl` 84 → 81 — *both* of the
niche-tag-plus-bitset sequences gone, the second being the MMC2 `mapper_number` read the same
function made.

## Hot path: the generated code after LTO

`cmp4c.py`, `bin-4d-s6b.s` against `bin-5a-s1f.s`.

**Prologues: every pinned symbol's prologue and stack frame is instruction-identical**
(`prologues.py`, normalising basic-block labels and the crate disambiguator) — `Cpu6502::step`'s
seven pushes, `NesBus::tick`'s `subq $40`, `Ppu2C02::run_dots`' `subq $616`,
`read_nametable_inner`'s `pushq %rsi / subq $32`, `background_fetch`'s five pushes and
`subq $32`. Nothing got a bigger frame or an extra saved register, which is the test for a cost
every game pays whatever branch it takes.

| symbol | baseline | step 1 | what moved |
| --- | ---: | ---: | --- |
| `Apu2A03::quiet_cycles` | 90 | 90 | **IDENTICAL, raw** |
| `Ppu2C02::read_nametable_inner` | 132 | **132** | IDENTICAL; the Q-123 decision is what keeps it there |
| `NesBus::read_cycle`, `tick`, `read_internal` | 182, 75, 176 | same | IDENTICAL (offs+hash) |
| `Apu2A03::run_cpu_cycles` (4 defs) | 875 | 875 | IDENTICAL — the APU is untouched |
| `NesBus::run_dma`, `dma_read` | 511, 76 | same | IDENTICAL |
| `Ppu2C02::read_pattern_planes` | 106 | 106 | IDENTICAL — MF-2's shield intact |
| `Ppu2C02::background_fetch` | 199 | 199 | same size, blocks re-laid-out |
| `NesBus::catch_up_ppu` | 219 | 219 | same size; one block moved, and callgrind makes it a **gain** |
| `Ppu2C02::run_dots` (2 defs) | 1 936 | **1 932** | −4: the `mapper_number` latch |
| `Cpu6502::step` | 897 | 916 | +19 |
| `Cpu6502::branch` | 366 | 377 | +11 |
| `Cpu6502::resolve` (2 defs) | 1 725 | 1 772 | +47 |
| `Board::ppu_read` | 312 | 328 | +16 |
| `Board::ppu_peek` | 235 | 253 | +18 |
| `Board::on_cpu_cycle` | 363 | 388 | +25 |
| `Board::on_ppu_address_bus` | 156 | 156 | same size, arms re-ordered |
| `NesBus::write_internal` | 2 643 | 2 784 | +141 |

The five that grow are all **one more arm in a dispatch**, and the executed path of a board the
lot did not touch is unchanged in each:

* `step`, `branch` and `resolve` carry the inlined `Board::irq_asserted`. Its arms are
  `cmpb $0, N(%rdi) / jne / jmp` before and after, and the +19/+11/+47 are extra copies of that
  three-instruction block (tail duplication) plus the jump table's thirty-first slot. The niche
  prologue is unchanged, so an MMC1 executes what it executed.
* `Board::ppu_read`, `ppu_peek`, `on_cpu_cycle` and `on_ppu_address_bus` keep the same niche
  prologue and the same jump-table shape, with one more entry and one more arm body.
* `write_internal`'s +141 is the VRC6's whole register decode inlined into its own arm, reached
  only by a CPU write to a mapper-24 or -26 cartridge. Its prologue, its RAM fast path and its
  twenty-three jump tables are unchanged.

**MF-3b rule 1, call sites: no callee gained one.** `Cpu6502::end_cycle` 0 → 0,
`Ppu2C02::read_nametable_as` 0 → 0, `read_nametable_inner` 15 → 15, `read_pattern_planes` 3 → 3,
`run_dots` 2 → 2, `Board::ppu_read` 4 → 4, `ppu_peek` 4 → 4, `on_ppu_address_bus` 14 → 14,
`irq_asserted` 0 → 0, `on_cpu_cycle` 2 → 2, `cpu_read_override` 0 → 0,
`Cartridge::try_cpu_read` 2 → 2, `Ppu2C02::read` 2 → 2, `Apu2A03::run_cpu_cycles` 16 → 16,
`NesBus::push_expansion_audio` 15 → 15. `panic_bounds_check` 1 260 → 1 261, and the one that
moved is **not** in zax-nes's core: with the crate disambiguator normalised, twenty of the
twenty-one symbols that moved are `mlua`/`zax_scripting`/`glow` shims whose backreference indices
shifted with the hashes, and the twenty-first is a closure vtable shim of
`zax_nes::ppu::tiles::Ppu2C02::decode_margins`, in the 3D tile path and not on any per-cycle or
per-fetch path. No bounds check appeared in `write_internal`, `step`, `run_dots` or any board
dispatch, which is MF-4d step 2's stop-the-lot test.

## Callgrind: thirteen ROMs, 300 frames

| ROM | board | `ad1ad765` | MF-5a step 1 | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 426 817 394 | 3 426 812 449 | −4 945 | −0.000 % |
| `oamstr` | NROM | 3 437 722 453 | 3 437 722 357 | −96 | −0.000 % |
| `m1` | MMC1 | 5 014 160 791 | 5 014 192 805 | +32 014 | +0.001 % |
| `metroid` | MMC1 | 5 252 853 203 | 5 252 836 741 | −16 462 | −0.000 % |
| `m4` | MMC3 | 5 600 693 086 | 5 600 719 911 | +26 825 | +0.000 % |
| `m0` | NROM | 5 643 153 338 | 5 643 159 655 | +6 317 | +0.000 % |
| `smb1` | NROM | 6 130 637 413 | 6 130 647 186 | +9 773 | +0.000 % |
| `mm2` | MMC1 | 6 136 429 450 | 6 136 443 669 | +14 219 | +0.000 % |
| `z2` | MMC1 | 6 409 332 652 | 6 409 275 719 | −56 933 | −0.001 % |
| `readbuf` | CNROM | 6 950 211 191 | 6 950 139 082 | −72 109 | −0.001 % |
| `duelito` | NROM | 7 198 774 119 | 7 198 772 656 | −1 463 | −0.000 % |
| `smb3` | MMC3 | 8 103 531 708 | 8 103 339 481 | −192 227 | −0.002 % |
| `mmc5` | MMC5 | 11 402 398 394 | 11 402 384 567 | −13 827 | −0.000 % |
| **total** | | **80 706 715 192** | **80 706 446 278** | **−268 914** | **−0.000 %** |

Every figure is inside the ~10⁵ whole-process floor, and the total is negative. The per-symbol
counts, which have no floor, are what the bar actually rests on:

**Identical to the instruction on all thirteen ROMs**, every one of them: `Cpu6502::step`,
`resolve`, `branch`, `execute_without_operand`, `NesBus::tick`, `read_cycle`, `read_internal`,
`halted_read_cycle`, `write_internal`, `run_dma`, `dma_read`, `Ppu2C02::read`,
`Cartridge::try_cpu_read`, `default_cpu_read`, `read_rom_window`, `Mapper::cpu_read_override`,
`Apu2A03::run_cpu_cycles`, `quiet_cycles`, `Ppu2C02::read_nametable_inner`,
`read_pattern_planes`, `run_sprite_evaluation`, `background_fetch`, `paint_sprite`,
`emit_sprite`, `Board::ppu_read`, `ppu_peek`, `on_cpu_cycle`, `on_ppu_address_bus`.

**Two symbols move**, and both are named:

* `NesBus::catch_up_ppu` — **−157 741 on `smb3`** and −6 612 on `m4`, 0 on the other eleven. A
  gain, from the one basic block that changed place; 219 instructions before and after.
* `Ppu2C02::run_dots` — up, on the sprite path only: `duelito` +43 662 (+0.0006 %), `metroid`
  +9 622, `readbuf` +12 519, `smb1` +7 504, `mmc5` +8 260, `z2` +4 930, `m0` +6 630, `m1` +4 470,
  `m4` +4 440, `smb3` +3 550, and **exactly 0 on `oam_stress` and on `ppu_vbl_nmi`**, the two
  ROMs of the set that emit no sprite at all.

**That last one is not free, and saying so is the honest reading of the bar.** What it is *not* is
added work or an added dispatch: `run_dots` is four instructions **smaller** than the baseline and
both of its niche-tag bitsets are gone. The opcode mix says where the executed instructions went
instead — `movdqu` 40 → 43, `movups` 8 → 9, `leaq` +2, `cmoveq` +2 — LLVM re-vectorising the tile
record store now that the two dispatches no longer hold registers. It is the tail-merging /
vectorisation flip MF-1, MF-2 and MF-3 all met on this family of functions, and MF-4d accepted the
same class of move on `Board::ppu_read` ("placement, not work"). The size of it: **+0.0006 % on
the worst ROM of thirteen**, against −0.002 % on `smb3`, −0.001 % on `readbuf` and on `z2`, and
−0.000 % on the set. Chasing it further means fighting the register allocator over three `movdqu`,
which is not a trade this step took; it is recorded here so R0 sees it in the same table as the
gains.

## Tests

`cargo fmt --check` clean. `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean, **no new `allow`** — the two `#[allow(clippy::inline_always)]` the attempts above needed
went out with the attributes. `cargo test --release -p zax-nes -p zax-harness`: **726 passed,
0 failed, 7 ignored** — the baseline's 716 plus this step's ten.

The ten, each against a register of the page:

| test | what it pins |
| --- | --- |
| `req_core_100_vrc6_switches_a_16_kib_and_an_8_kib_prg_bank` | `$8000` four bits at 16 KiB, `$C000` five at 8 KiB, the fixed last bank, all four mirrors of each register, and that nothing above either field latches |
| `req_core_100_vrc6b_has_a0_and_a1_swapped` | `$D001` latches R1 on mapper 24 and R2 on mapper 26 — the one place the swap is visible, `$C000-$C003` being one register |
| `req_core_100_vrc6_arranges_chr_in_the_four_banking_modes` | the four `D`/`P` arrangements, including `P` clear putting the whole register in both halves of a 2 KiB window |
| `req_core_100_vrc6_b003_fixes_four_arrangements_when_p_is_set` | all eight `$B003 & $2F` values of the table above |
| `req_core_100_vrc6_picks_a_ciram_page_per_table_from_the_latched_bit` | modes 0, 1 and 2 taking a page per table, and only the register's low bit being it |
| `q_123_vrc6_keeps_its_nametables_on_ciram_when_n_asks_for_chr_rom` | the Q-123 provisional choice |
| `req_core_100_vrc6_enables_prg_ram_with_bit_7_of_b003` | `W` gating $6000-$7FFF, and that disabling hides without losing |
| `req_core_100_the_vrc6_counter_counts_up_on_the_cpu_clock` | the whole latch written at once, cycle mode, and the acknowledge copying `A` to `E` |
| `req_core_100_the_vrc6_counter_runs_on_the_scanline_prescaler_and_keeps_a_set` | the ÷341/3 prescaler, and `A` set leaving the counter armed |
| `mf_5a_the_vrc6_and_the_vrc4_share_one_irq_counter` | a mapper-23 VRC4 and a mapper-24 VRC6 fire on the **same cycle** from the same latch through their two different decodes — the check that the extraction is a share and not a copy |

`amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` and
`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` sweep `SUPPORTED_MAPPERS`, so
both now cover mappers 24 and 26 without being edited;
`req_core_100_every_required_mapper_is_registered` has 24 and 26 added to the list of numbers
this build adds beyond REQ-CORE-100's table, so an accidental addition is caught as surely as an
accidental removal.

**Save states**: the VRC6's own block is new. `Vrc2Vrc4`'s is **unchanged**, which is the one
compatibility question the extraction raised, and `VrcIrq::save_state` writes the seven fields in
the order that board already used.

## Step 2 — the VRC6 audio

## The code

`apu/vrc6_audio.rs`, **new**: `Vrc6Pulse` twice, `Vrc6Saw`, and `Vrc6Audio` holding them with
`$9003`'s halt and frequency scaling. `apu/mixer.rs` gains `VRC6_LEVELS` and `VRC6_TABLE`.
`vrc6.rs` carries the chip, routes `$9000`-`$9003`, `$A000`-`$A002` and `$B000`-`$B002` to it,
clocks it from `on_cpu_cycle` and answers `expansion_audio_output`.

### The level, and why Q-118 does not reopen here

Q-118 had to pick an anchor for the Sunsoft 5B because its page gives none. **The VRC6's page
gives both the anchor and the law**, in two sentences:

* "At maximum volume, the pulse channels of the VRC6 are roughly equivalent to the pulse channels
  of the 2A03 (except inverted)" — so one pulse at `V = 15` is `PULSE_TABLE[15]`, which is
  exactly the anchor R0's answer to Q-118 approved for the 5B;
* "The DAC of the VRC6, unlike the 2A03, appears to be linear", and "The final mix is a 6-bit DAC
  summing the two 4-bit pulse outputs and the high 5 bits of the saw accumulator" — so the table
  is a straight line of 62 steps and `M` caps at 61 × 4 890 / 15 = **19 886**, 61 % of full
  scale.

The polarity is the page's too — "(except inverted)" — and it is the polarity `mixer::clip`
already implements (AMD-061 §6.2), so nothing in the mixer moved. **No new Q ticket**: the
question Q-118 asked is answered by the document here, which is what that ticket said would
settle it.

`VRC6_TABLE` is built from the unrounded anchor rather than by multiplying the rounded
`PULSE_TABLE[15]`, so the step is not quantised twice, and
`req_core_075_the_vrc6_dac_is_linear_through_an_apu_pulse` recomputes all 62 entries at runtime
and compares.

### The three generators

The pulses are the page's twelve-bit divider, its sixteen-step duty generator and its
`step <= D` rule, with `M` bypassing the generator. The sawtooth is the one place **Mesen is the
deciding source**: the page says "the accumulator only reacts on every 2 clocks" and gives six
additions and a reset per phase, without saying *which* of the fourteen steps add;
`Vrc6Saw.h:53-68` adds on the even ones and resets at step 0, which is the only reading that
produces the page's six-and-one, and `req_core_075_the_vrc6_saw_accumulates_six_times_then_resets`
pins the resulting output sequence (1, 2, 3, 4, 5, 6, then 0, for a rate of 8).

`refresh_level` runs only when a generator actually stepped — each `step_cpu_cycle` returns
whether it did — because all three run off the CPU clock itself, with no prescaler to hide behind
like the 5B's divide-by-16.

## The two attributes, and the lesson about the two dumps

### `Board::on_cpu_cycle`'s prologue is shared by thirty arms

The audio inlined into the VRC6's arm took the dispatch's prologue from
`pushq %rsi / subq $32, %rsp` to `pushq %rsi / pushq %rdi / subq $40, %rsp` — **one more push and
pop per CPU cycle for every board**, an NROM with no expansion audio included. That is MF-3b
rule 3 in its exact form: the arm that matters is the empty one.

`#[inline(never)]` on `Vrc6Audio::step_cpu_cycle` put the Windows prologue back. **Callgrind then
showed it had done nothing at all for the Linux build**: `Board::on_cpu_cycle` +17 868 320
instructions, +2 per CPU cycle, on **all thirteen ROMs**, twelve of which have no expansion audio.

**This is the first place in these lots where the two instruments disagree, and the rule it
yields is worth writing down.** `cargo rustc --emit=asm` runs on the Windows host, callgrind on
the WSL2 Linux one. Same rustc, same LLVM — but not the same ABI: Windows x64 passes the first
argument in `%rcx` and has a different callee-saved set from the SysV ABI, so an attribute chosen
to fix a register-allocation symptom in one dump is *not evidence* about the other.
**PERFORMANCE-HISTORY.md §2's "the Linux binary's hot code is the Windows binary's" holds for
what a function computes and not for which registers it saves.** Register pressure and prologue
size have to be read on the dump of the platform being measured, or settled by a change that is
ABI-independent.

The fix that works on both is `#[inline(never)]` on `Vrc6::on_cpu_cycle` **itself**, not only on
the audio's body: the arm then becomes a tail call and holds nothing across it. With it,
`Board::on_cpu_cycle` is 363 → **366** against the lot's baseline (+3, all of it inside the
VRC6's own arm) and the per-cycle cost of the dispatch for every other board is the baseline's.

### The latch: `Board::on_cpu_cycle` disappears for 26 of the 38 mapper numbers

Chasing the above exposed something larger. `Cartridge::step_cpu_cycle` dispatched
**unconditionally**, so a board with nothing to do paid, every CPU cycle: a `callq`, a
twelve-instruction niche prologue, a thirty-entry jump table, a three-instruction `retq` arm and
the return. Callgrind priced it exactly — `Board::on_cpu_cycle` is **98 275 760 instructions**
over 300 frames on `duelito`, on `z2`, on `smb1`, on every ROM of the set whatever its board,
which is **eleven instructions per CPU cycle**.

Twelve of the thirty-eight supported mapper numbers override `Mapper::on_cpu_cycle`: 5 (the
MMC5's PPU-idle counter), 16 and 18 (the FCG and SS88006 IRQ counters), 21, 22, 23, 25 and 27
(Konami's VRC IRQ), 24 and 26 (the VRC6's IRQ and sound), 64 (the RAMBO-1's cycle mode) and 69
(the FME-7 and its 5B). The other twenty-six inherit the empty default. So `Cartridge` gains
`clocks_on_cpu_cycle`, latched from the mapper number at load and tested **before** the dispatch
— the sixth such latch beside `rom_windows`, `nametable_pages_vary`, `bus_conflicts`,
`watches_ppu_reads` and `watches_ppu_bus_without_read`, and the same shape MF-3b step 2 gave
`on_ppu_address_bus`.

**MF-3b rule 5's warning applies and is tested, not trusted.** A board whose number nobody adds
to the list would have its counter silently never clocked and its sound silently never played —
a correctness bug, not a slow path. `mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched`
sweeps `SUPPORTED_MAPPERS` × submappers 0 and 3, drives 400 bare CPU cycles straight at each
board and requires the latch wherever the board moved (`Board` derives `PartialEq`, so "moved" is
the whole state). Its own expected list is short on purpose and the comment says why: only the
MMC5's idle counter and the FME-7's 5B prescaler move with **no register written at all**, the
other ten needing an enable bit first — which is what `mapper_irq_tests` and the two audio suites
cover, since they drive whole machines through `step_cpu_cycle`.

## Callgrind: thirteen ROMs, 300 frames, against `ad1ad765`

| ROM | board | `ad1ad765` | MF-5a step 2 | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 426 817 394 | 3 328 505 294 | −98 312 100 | **−2.869 %** |
| `oamstr` | NROM | 3 437 722 453 | 3 339 414 348 | −98 308 105 | **−2.860 %** |
| `m1` | MMC1 | 5 014 160 791 | 4 915 912 247 | −98 248 544 | **−1.959 %** |
| `metroid` | MMC1 | 5 252 853 203 | 5 154 567 576 | −98 285 627 | **−1.871 %** |
| `m4` | MMC3 | 5 600 693 086 | 5 502 441 098 | −98 251 988 | **−1.754 %** |
| `m0` | NROM | 5 643 153 338 | 5 544 850 455 | −98 302 883 | **−1.742 %** |
| `smb1` | NROM | 6 130 637 413 | 6 032 345 169 | −98 292 244 | **−1.603 %** |
| `mm2` | MMC1 | 6 136 429 450 | 6 038 176 841 | −98 252 609 | **−1.601 %** |
| `z2` | MMC1 | 6 409 332 652 | 6 310 995 722 | −98 336 930 | **−1.534 %** |
| `readbuf` | CNROM | 6 950 211 191 | 6 851 870 163 | −98 341 028 | **−1.415 %** |
| `duelito` | NROM | 7 198 774 119 | 7 100 492 549 | −98 281 570 | **−1.365 %** |
| `smb3` | MMC3 | 8 103 531 708 | 8 005 101 664 | −98 430 044 | **−1.215 %** |
| `mmc5` | MMC5 | 11 402 398 394 | 11 420 241 676 | **+17 843 282** | **+0.156 %** |
| **total** | | **80 706 715 192** | **79 544 914 802** | **−1 161 800 390** | **−1.440 %** |

The symbol diff is one line per ROM and leaves nothing unaccounted for. On the twelve:

    -98,275,760   98,275,760 -> 0   <Board as Mapper>::on_cpu_cycle

and on `mmc5`:

    +17,868,320  384,168,880 -> 402,037,200   <NesBus>::tick

That is the whole of it: the dispatch **goes to zero** on every board that does not need it, and
the latch's two instructions per cycle land in `tick`. Every other per-symbol count is identical
to the instruction on all thirteen — `Cpu6502::step`, `resolve`, `branch`,
`execute_without_operand`, `read_cycle`, `read_internal`, `write_internal`, `run_dma`, `dma_read`,
`Ppu2C02::read`, `run_dots`, `read_nametable_inner`, `read_pattern_planes`, `background_fetch`,
`run_sprite_evaluation`, `emit_sprite`, `paint_sprite`, `Apu2A03::run_cpu_cycles`,
`quiet_cycles`, `Board::ppu_read`, `ppu_peek`, `on_ppu_address_bus`, `Cartridge::try_cpu_read`,
`default_cpu_read` — with `catch_up_ppu` keeping step 1's −157 741 on `smb3`.

### `mmc5` is +0.156 %, and that is a trade the lot is making knowingly

**Stated plainly: this step does not meet "identical to the instruction or lower on every ROM".**
One ROM of thirteen is slower, by two instructions per CPU cycle, and the reason is not the new
board — `mmc5exram` runs no line of the VRC6 — but the latch that pays for it. A board that *is*
latched now tests the `bool` and then dispatches as before.

The trade, priced: **twenty-six of the thirty-eight mapper numbers gain eleven instructions per
CPU cycle, and twelve pay two.** On the measured set that is twelve ROMs at −1.2 % to −2.9 %,
including all five of R0's commercial games, against one at +0.156 %.

It is also **the shape MF-3b step 2 shipped**, on the other clock, and that step's own words
apply unchanged: it made twenty-six boards skip `on_ppu_address_bus` entirely and cost mappers 4,
64 and 118 "about +2 instructions per PPU fetch", recording that "Neither of R0's callgrind games
is on those mappers". Neither is on an MMC5 here either; the MMC5's commercial title is
*Castlevania III*, which PR-1 step 1 deliberately left out of this set.

**The revert is one line** — drop the `if self.clocks_on_cpu_cycle` in
`Cartridge::step_cpu_cycle` — and it would leave the step at parity on twelve ROMs and the MMC5
exactly on the baseline, at the cost of the −1.44 %. The call belongs to R0 and the numbers above
are what it needs; PortMaster and handhelds coming first is the argument for keeping it.

## Hot path: the generated code after LTO

`cmp4c.py`, `bin-4d-s6b.s` against `bin-5a-s2c.s`. Against **step 1** every pinned symbol is
`IDENTICAL, raw` except three, and that is the cleanest statement of what this step costs:

| symbol | step 1 | step 2 | what moved |
| --- | ---: | ---: | --- |
| `NesBus::tick` | 75 | **79** | +4, the latch's `cmpb`/`jne` and its re-laid-out neighbours |
| `NesBus::write_internal` | 2 784 | 2 805 | +21, inside the `$9000`-`$B002` arm |
| `Board::on_cpu_cycle` | 388 | 393 | +5, inside the VRC6's arm: the tail call plus the VRC IRQ's register renaming |
| everything else | | | `step` 916, `branch` 377, `resolve` 1 772, `read_cycle` 182, `read_internal` 176, `catch_up_ppu` 219, `run_dma` 511, `dma_read` 76, `run_cpu_cycles` 875, `quiet_cycles` 90, `run_dots` 1 932, `background_fetch` 199, `read_nametable_inner` 132, `read_pattern_planes` 106, `Board::ppu_read` 328, `ppu_peek` 253, `on_ppu_address_bus` 156 — **all IDENTICAL raw** |

`Ppu2C02::run_dots` reports `DIFFERS` on the raw column for one reason only, and it is worth
naming so the next lot does not chase it: the two `switch.table.…run_dots.1004` / `.1005` symbol
**suffixes** become `.1005` / `.1006` because one more switch table exists in the module. Same
instructions, same operands, a different symbol name in a `leaq`.

**Prologues** (`prologues.py`): `Board::on_cpu_cycle` is back to `pushq %rsi / subq $32, %rsp`,
and every other pinned symbol's prologue and frame is instruction-identical to step 1's.

**MF-3b rule 1, call sites**: `callsites.py` reports **no watched callee moved at all** between
step 1 and step 2 — not one entry in the table differs, `panic_bounds_check` included.

## Tests

`cargo fmt --check` clean, `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean, **no new `allow`**. `cargo test --release -p zax-nes -p zax-harness`: **741 passed,
0 failed, 7 ignored** — step 1's 726 plus fourteen in `vrc6_audio_tests.rs` and the latch
invariant.

| test | what it pins |
| --- | --- |
| `req_core_075_the_vrc6_dac_is_linear_through_an_apu_pulse` | all 62 entries against the page's anchor and its linear law, and `M`'s ceiling of 19 886 |
| `req_core_075_a_vrc6_is_silent_until_a_channel_is_enabled` | power-on silence, and that a volume without `E` is still silence |
| `req_core_075_a_vrc6_pulse_in_mode_m_outputs_its_volume_flat` | `M` bypassing the duty generator, at two volumes |
| `req_core_075_a_vrc6_pulse_duty_is_d_plus_one_sixteenths` | `D` = 0, 3 and 7 giving 1/16, 4/16 and 8/16 of the cycle high |
| `req_core_075_the_two_vrc6_pulses_add_into_the_same_dac` | 15 + 15 = step 30 = twice step 15, and `$A000` being a second channel and not a mirror |
| `req_core_075_clearing_e_forces_a_vrc6_pulse_to_zero_and_resets_its_duty` | the forced 0, the halted generator, and the resume from step 0 |
| `req_core_075_the_vrc6_saw_accumulates_six_times_then_resets` | the output sequence 1-6 then 0 for rate 8, and that there is no seventh addition |
| `req_core_075_a_vrc6_saw_rate_above_42_wraps_the_accumulator` | rate 43 giving 5, 10, 16, 21, 26 and a wrap, never 31 |
| `req_core_075_clearing_e_zeroes_the_vrc6_saw_accumulator` | the accumulator forced to 0 |
| `req_core_075_the_vrc6_saw_divides_the_clock_by_14_and_the_pulses_by_16` | the two different divisors from the same period register |
| `req_core_075_9003_bit_0_halts_every_vrc6_oscillator_in_place` | the halt freezing the level mid-duty, and releasing it |
| `req_core_075_9003_scales_every_frequency_by_16_or_256` | the two shifts by edge count, 256x overriding 16x, and halt overriding both |
| `req_core_075_the_vrc6_sums_its_three_channels_into_one_m` | the DAC's ceiling reached by all three together, and `M` positive |
| `req_core_075_a_vrc6_subtracts_its_m_from_the_apu_mix` | `S = A − M` through the mixer's own test point |
| `mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched` | the latch's invariant, over every supported mapper and two submappers |

**Save states**: `Vrc6Audio::save_state` is appended after the IRQ, the shape `fme7.rs`
established, and `load_registers` tolerates a block without it. `clocks_on_cpu_cycle` is derived
from the mapper number at load, like its five neighbours, and is not written.

`THIRD-PARTY-NOTICES.md` now lists the VRC6 and `VrcIrq` adaptations — and the FME-7's `$C`
command, which lot MF-3 added the file header for without adding the entry.

## Status

- [x] step 1 — the VRC6 board, mappers 24 and 26 (Q-123 opened: `$B003` bit 4)
- [x] step 2 — the VRC6 audio (and `Board::on_cpu_cycle` latched: the set is **-1.44 %**, `mmc5` +0.156 %)
- [ ] step 3 — the Namco 163 board, mappers 19 and 210
- [ ] step 4 — the Namco 163 audio
- [ ] step 5 — save states
- [ ] step 6 — the ROM check
- [ ] step 7 — the suites and the campaign
