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

## Status

- [x] step 1 — the VRC6 board, mappers 24 and 26 (Q-123 opened: `$B003` bit 4)
- [ ] step 2 — the VRC6 audio
- [ ] step 3 — the Namco 163 board, mappers 19 and 210
- [ ] step 4 — the Namco 163 audio
- [ ] step 5 — save states
- [ ] step 6 — the ROM check
- [ ] step 7 — the suites and the campaign
