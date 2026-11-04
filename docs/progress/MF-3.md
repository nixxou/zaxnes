# Lot MF-3 — the mapper failures of the test-ROM campaign, Q-117, and the first new boards

Role R3 (clean-side implementer), worktree `wt-mappers`, branch `mapper-review`.

Sources of truth for this lot:

- `docs/review/MF-T-test-roms.md` (the campaign: which ROMs fail and what they say),
- `spec/changes/Q-117.md` **with its R0 answer** (the UNROM 512 bank latch),
- `REQ-CORE-100` (the mapper table and "le comportement documenté sur nesdev"),
  `REQ-CORE-102` ("Ajouter des mappers au-delà de la liste est permis"), `AMD-061` §1 (public
  documentation prevails over the original program), `AMD-054` / `REQ-CORE-075` (expansion-audio
  mixing),
- `docs/review/MR-1-mappers.md`, the **MR-1b** section, finding **MR-4-3** (the Acclaim MC-ACC),
- `docs/progress/MF-2.md` (what the previous lot did and the cost method it established),
- the NESdev wiki pages named per step, `holy-mapperel/README.md`, and `third_party/mesen2`
  where it agrees with them.

Build environment for every cargo command (the C: drive is nearly full):

    TMP=<worktree>/rust/target-mf/tmp  TEMP=<worktree>/rust/target-mf/tmp
    CARGO_TARGET_DIR=<worktree>/rust/target-mf

`zax-app` cannot build in this worktree (its demo ROM is git-ignored), so clippy is scoped to
`-p zax-nes -p zax-harness`.

## Steps

- [x] 1/9 — MMC1 SNROM: bit 4 of the CHR bank register disables the work RAM
- [x] 2/9 — Q-117: the UNROM 512 bank latch is decoded by configuration
- [x] 3/9 — mapper 69, Sunsoft FME-7 (banks, CPU-cycle IRQ, $6000 RAM/ROM switching)
- [x] 4/9 — Sunsoft 5B audio: the three AY-3-8910 squares, envelope and noise
- [x] 5/9 — mapper 118, TxSROM
- [x] 6/9 — mapper 180, UNROM with the switchable bank at $C000
- [x] 7/9 — mapper 78, submappers 1 and 3
- [x] 8/9 — MR-4-3, the Acclaim MC-ACC (mapper 4 submapper 3)
- [x] 9/9 — re-run the suites, the campaign document, and the close of the lot

## Out of scope (from the brief)

VRC6, VRC7, Namco 163, FDS, mapper 99 / VS. System, and anything in the CPU, PPU or APU core
(DMA, APU PAL timing, `AccuracyCoin`): lot MF-4.

**Registers in `$4020-$5FFF` (brief item 5).** None of this lot's boards has any, and each step
below says so for its own board. That range is shared with the hack packs' extension registers
and code zone (`zone_data`), which this branch does not carry; a board that needed it would stop
the lot and open a Q ticket.

## How the cost of each step is measured

The method MF-2 step 0 established, because there is no callgrind on this host:

    touch rust/crates/zax-nes/src/lib.rs
    cargo rustc --release -p zax-nes --lib -- --emit=asm

then the static instruction count of **every** symbol of the crate's `.s`, diffed against the
same dump taken before the step. The four functions the lot's rules name —
`Board::ppu_peek`, `Board::ppu_read`, `Ppu2C02::read_pattern_planes`,
`Ppu2C02::read_nametable_as` — are reported for every step; the full diff is what says whether
anything *else* moved.

Before the lot (`7cbd9c52`):

| function | instructions |
| --- | --- |
| `Board::ppu_peek` | 240 |
| `Board::ppu_read` | 241 |
| `Ppu2C02::read_pattern_planes` | 106 |
| `Ppu2C02::read_nametable_as` | 107 |

## Findings → fix → test → hot path (for R0)

| item | fix (file:line) | test | hot path touched, and why other games do not pay | commercial games whose output changes |
| --- | --- | --- | --- | --- |
| MF-T, `M1_P128K` / `M1_P128K_CR8K` detailed `4000` (SNROM work-RAM disable) | `mmc1.rs:29-38` (two fields), `:72-104` (`is_snrom`), `:105-116` (`refresh_prg_ram_disable`), `:118-119` (`update_banks`), `:200-201` (`load_registers`), `:211-215` (`prg_ram_enabled`) | `req_core_100_mmc1_snrom_disables_prg_ram_with_bit_4_of_the_chr_register`, `req_core_100_mmc1_only_an_snrom_reads_bit_4_of_the_chr_register_as_a_ram_enable`; the two Holy Mapperel rows in `test_roms_tests.rs` | **none.** The whole-crate asm diff is three symbols, all off every hot path: `Mmc1::update_banks` 137 → 148, `Mmc1::load_registers` 89 → 118, `Cartridge::create` 3435 → 3471 (the load-time decision). `Board::prg_ram_enabled` is **byte-identical** — the answer is a precomputed `bool`, so the $6000-$7FFF decode is the same load and test as before | mapper-1 images on an SNROM-shaped board (CHR-ROM ≤ 8 KiB, PRG-RAM 8 KiB, PRG-ROM ≤ 256 KiB) **that set bit 4 of `$A000`**: on real SNROM hardware that disables the RAM, so no shipped game does it while using the RAM. The golden-core set has mapper 1; *Mega Man 2* and *Zelda II* are both on this shape and neither writes that bit with the RAM live |
| Q-117 (R0's answer): the UNROM 512 latch is at `$C000-$FFFF` on a self-flashable configuration, `$8000-$BFFF` being the flash write window, and the chip recognises its sequences on **chip** addresses | `discrete.rs:422-446` (the type doc and the `latch_base` field), `:496-503` (the load-time decision), `:514-526` (`chip_address`, the two unlock constants), `:528-585` (`step_flash`), `:688-698` (`write_register`) | `q_117_unrom512_programs_a_byte_of_its_own_flash` (was `mr_30_4_…`, rewritten with the page's sequences verbatim), `q_117_unrom512_flashes_the_bank_the_driver_selects`, `q_117_unrom512_unlocks_only_on_the_chips_own_command_addresses`, `q_117_unrom512_without_self_flashing_keeps_the_latch_over_8000_ffff`, `mr_30_4_unrom512_an_incomplete_flash_sequence_changes_nothing`, `mr_30_4_unrom512_with_bus_conflicts_cannot_flash`, `mr_30_5_unrom512_submapper_4_does_not_bank_through_the_led_register` (unchanged, and now implied by the new rule) | **none, and it is a win.** `UnRom512::write_register` 116 → **106** (one compare replaces a `bool` test plus a second compare) and `Cartridge::create` 3471 → **3462** (one `u16` replaces two `bool`s). Nothing else in the crate moved. The read path is untouched: `rom_windows` is still a plain window, so no board pays anything on a read | self-flashing UNROM 512 homebrew, and **every** mapper-30 image on a no-bus-conflict configuration: its bank latch has moved from `$8000-$FFFF` to `$C000-$FFFF`. No golden-core entry is on mapper 30 |
| mapper 69, Sunsoft FME-7 / 5A / 5B — a **new board** (REQ-CORE-102) | `fme7.rs` (new, 240 lines), `board.rs:12-30` (the append rule) + `:70-72` + `:105` (the variant), `registry.rs:20-23` (`SUPPORTED_MAPPERS`) + `:181` (`create_board`), `mappers/mod.rs:26` + `:40` | `req_core_100_fme7_*` x4 and `req_core_090_the_fme7_state_survives_a_save_and_restore` in `mapper_tests.rs`, `req_core_100_the_fme7_*` x3 and `req_core_100_a_write_to_fme7_command_d_acknowledges_the_irq` in `mapper_irq_tests.rs`; `M69_P128K_C64K_S8K` and `_W8K` now **pass** with `detailed 0000` | `Board::ppu_peek` 240 → **251**, `ppu_read` 241 → **252**; `read_pattern_planes` and `read_nametable_as` **byte-identical**. The +11 is LLVM switching the function's shared bounds-check tail from tail-*merging* to tail-*duplication*: the arms' own code is unchanged and the executed path of the arms concerned is **one instruction and two taken branches shorter**. Every other hot dispatch (`on_cpu_cycle`, `irq_asserted`) has its existing arms **byte-identical** with the FME-7's appended. See "Step 3" for the wall-clock measurement and what it is worth | **none.** No commercial game moves: this is a board that did not load before. The golden-core set (mappers 0-4, 156) has no mapper-69 entry |
| Sunsoft 5B audio (brief item 3, REQ-CORE-075 optional, **Q-118** for the level and the polarity) | `apu/sunsoft5b_audio.rs` (new, 320 lines), `apu/mixer.rs:145-215` (`SUNSOFT_5B_TABLE`, `sunsoft_5b_level`), `apu/mod.rs:10-15` + `:23` + `:38`, `fme7.rs:40-44` (the field) + `:190-192` (`expansion_audio_output`) + `:258` (the register writes) + `:269` (the clock) + `:143-160` (the state) | `sunsoft5b_audio_tests.rs`, 15 tests | **none.** The four named functions are **byte-identical**. The whole-crate diff is thirteen symbols, all inside `Fme7` / `Sunsoft5bAudio` or on the expansion-audio path that only an MMC5 or a mapper-69 cartridge reaches — `NesBus::push_expansion_audio` is still `#[inline(never)]` behind the load-time `expansion_audio` bool (RU-3a). `Board::on_cpu_cycle` is 7 instructions **smaller**: the FME-7's arm left the shared dispatch for a call | **none.** No commercial game on any other board moves; mapper-69 games gain their sound, which they did not have one commit ago. No golden-core entry is on mapper 69 |
| mapper 118, TxSROM — **a variant of the MMC3 board chosen at load**, no new `Board` variant | `mmc3.rs:12-41` (the doc and three fields), `:56-78` (the load-time facts), `:136-156` (`refresh_nametable_pages`), `:158-159` (`update_banks`), `:260-270` (`mapper_number`, `nametable_page`), `:374` (`ignore_a000`), `registry.rs:20-23` + `:160`, `cartridges/mod.rs:194-196` + `:256` (`nametable_pages_vary`) | `req_core_100_txsrom_*` x4 and `req_core_090_the_txsrom_nametables_come_back_with_its_banks`; `M118_P128K_C64K` now **passes** with `detailed 0000` | **none, and the two fetch dispatches came out smaller than before the lot**: `Board::ppu_peek` 251 → **235**, `ppu_read` 252 → **236** (240/241 at `7bcd...`), prologue and niche unchanged, `read_pattern_planes` / `read_nametable_as` byte-identical, `Ppu2C02::step_dot` back to its pre-lot 1409. `Board::nametable_page` 28 → 33 for the new arm, which is a clamped load from a field — and `Cartridge` caches the four answers, re-reading them after a CPU write only for mappers 5 and 118 | **none.** Mapper 4 is byte-identical in behaviour: `is_txsrom` is false, the four pages stay `FOLLOW_MIRRORING`, `$A000` still drives the arrangement, and no savestate byte moved. The golden-core set has mapper 4 and it does not move |
| mapper 180, UNROM with the fixed **first** bank — **a wiring of the UxROM board chosen at load**, no new `Board` variant | `discrete.rs:8-49` (the doc, two fields, `update_banks`), `:63-65` (`mapper_number`), `registry.rs:20-23` + `:144-148` (`has_bus_conflicts`) + `:158` | `req_core_100_mapper_180_fixes_the_first_bank_and_switches_at_c000`, `req_core_100_mapper_2_keeps_the_fixed_last_bank`; `M180_P128K_CR8K_H` and `M180_P128K_H` now **pass** with `detailed 0000` | **none, and a small win.** All four named functions **byte-identical**. Seven symbols moved and five shrank: the branch made `UxRom::update_banks` worth outlining, which took 36 instructions out of `Board::write_register` and 30 out of `Board::reset` | **none.** Mapper 2 does not move: `fixed_first` is false, its conflicts stay gated on submapper 2 (MR-BUS-1), and no savestate byte moved. No golden-core entry is on mapper 180 |
| mapper 78, both wirings (submapper 1 *Uchuusen*, submapper 3 *Holy Diver*) with the iNES-1.0 heuristic — a **new board** and the lot's second and last new `Board` variant | `discrete.rs:185-283` (`Mapper078`, new), `board.rs:72-73` + `:107` (the appended variant), `registry.rs:20-23` + `:144-150` (`has_bus_conflicts`) + `:183`, `mappers/mod.rs:39-41` | `req_core_100_mapper_78_*` x4; `M78.3_P128K_C64K` now **passes** with `detailed 0000` | **none.** All four named functions **byte-identical**, same prologue and niche — the second appended variant cost the fetch dispatches nothing where the first cost +11 and then gave it back, which is what settles step 3's measurement as placement. +2 to +7 on the five `Board` dispatches that gained an arm; the crowd of renamed `discrete.rs` symbols is the mangled disambiguator shifting, not a change | **none.** A board that did not load before. No golden-core entry is on mapper 78 |
| MR-4-3, the Acclaim MC-ACC (mapper 4 submapper 3) — **the MMC3's own code, monomorphised**: `Mmc3Board<const MC_ACC: bool>`, `Mmc3 = Mmc3Board<false>`, `McAcc = Mmc3Board<true>` | `mmc3.rs:11-33` (the doc), `:46-49` (the prescaler field), `:58-62` (the two aliases), `:69-76` (the watcher), `:233-236` + `:248-251` (the savestate byte), `:316-327` (`ppu_irq_horizon`), `:430-437` (`$C001`), `:443-471` (`on_ppu_address_bus`), `a12_watcher.rs:19-30` + `:72-80` (`fell`), `board.rs:74-77` + `:109`, `registry.rs:155-162` | `mr_4_3_the_mc_acc_clocks_once_per_eight_a12_falls`, `mr_4_3_a_plain_mapper_4_image_is_not_an_mc_acc`, `mr_4_3_writing_c001_resets_the_mc_acc_prescaler`, `req_core_090_the_mc_acc_prescaler_survives_a_save_and_restore` | **none, and the one claim in the lot that had to be exact.** `Board::on_ppu_address_bus` runs on every PPU fetch: the MMC3's arm is **character for character** what it was, all three of its paths, and the dispatch that reaches it got **one instruction shorter** (a jump table replaced a bitset-plus-compare chain). The four named functions are byte-identical. A field-based mode was written first, measured at **+2 instructions per fetch** for mappers 4 and 64, and thrown away — which is why `MC_ACC` is a `const` parameter | **none.** A plain mapper-4 or mapper-118 image takes the same instructions and writes the same savestate bytes; only an NES 2.0 submapper-3 dump changes, and it did not run as an MC-ACC before. *The Incredible Crash Dummies* and *Mickey's Safari in Letterland* are the titles; neither is in the golden-core set |
| step 9: the suites, the campaign document, the close | `docs/review/MF-T-test-roms.md` (totals, the eight changed rows, a "What lot MF-3 changed" section), `test_roms_tests.rs` (all forty `holy-mapperel` rows now `Pass`) | the whole `holy-mapperel` suite, `mmc3_test`, `mmc3_test_2`, `mmc3_irq_tests`, `MMC1_A12` | **the lot end to end:** `Board::ppu_peek` 240 → **235**, `ppu_read` 241 → **236**, `read_pattern_planes` and `read_nametable_as` **byte-identical**, `Ppu2C02::step_dot` 1409 → **1403**, and the MMC3's arm of `on_ppu_address_bus` character for character what it was. **But `bench-core` says +4 % on ROMs that touch no new board**, outside this host's noise — the lot's one open item, localised and documented in "Step 9" for R0's callgrind | **no commercial game on an existing board changes behaviour.** Golden-core has mappers 0-4 and 156: mapper 1 changes only for an SNROM-shaped image that sets bit 4 of `$A000` with the RAM live (no shipped game does), mapper 4 not at all unless the dump declares submapper 3, and 0, 2, 3 and 156 not at all |

## Step 1 — the MMC1 SNROM work-RAM disable

### What the ROM says

`M1_P128K.nes` and `M1_P128K_CR8K.nes` (identical headers — the suite's ROMs "differ only by
header", and these two do not even differ there) print

    SNROM (MMC1): detailed 4000 (WRAM 4: $A000 bit 4 does not disable WRAM (SNROM), or
    disables it (all but SNROM)); 128K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK

The other ten MMC1 rows — SJROM, SKROM, SUROM, SXROM — were already `0000`, which is what
localised the defect to the CHR-register line of one board.

### The hardware

NESdev, *MMC1*, SNROM: the board, "which only supports 8 KiB of CHR-ROM/-RAM, uses the upper CHR
bank select line coming out of the mapper (CHR A16; bit 4 of bank number) as an additional chip
enable for the PRG-RAM". Both CHR bank registers carry it as `E`, "PRG-RAM disable (0: enable,
1: open bus)", CHR bank 1's being "ignored in 8 KiB mode"; and the page then requires that
"SNROM's `E` bit and SO/U/XROM's `P` and `S` bits in both CHR bank registers must be set to the
same values". So CHR bank 0 is the whole answer for any program the page admits, and it is also
the register the test ROM names.

**Mesen 2 does not implement this at all** — `MMC1.h:87` latches `_wramDisable` from `$E000`
only, and `UpdateState` never looks at bit 4 of a CHR register for the RAM. AMD-061 §1 makes the
public documentation prevail, so the page is followed and the divergence from Mesen is
deliberate.

### Identifying the board

The same line means something else on every other MMC1 board that has it: `P`, the 256 KiB outer
PRG bank, on SUROM and SXROM; CHR A16, a real bank line, on SJROM / SKROM / SLROM. The board page's
table separates them by size alone:

| board | PRG-ROM | PRG-RAM | CHR |
| --- | --- | --- | --- |
| SNROM | 128/256 KiB | 8 KiB | 8 KiB RAM/ROM |
| SOROM | 128/256 KiB | **16 KiB** | 8 KiB RAM/ROM |
| SUROM | **512 KiB** | 8 KiB | 8 KiB RAM/ROM |
| SXROM | 128/256/512 KiB | **32 KiB** | 8 KiB RAM/ROM |

and those are exactly the three quantities Holy Mapperel's driver measures (with its bank tags)
before it decides which way the `4xxx` digit goes. So `Mmc1::is_snrom` is those three
comparisons, taken at load.

**One trap, and it decides the iNES-1.0 half of the question.** The CHR test is on the header's
CHR-**ROM** size, not on the CHR-RAM buffer: an NES 2.0 image gives its CHR-RAM exactly
(`M1_P128K.nes` declares 8 KiB in byte 11), but REQ-CORE-012 hands an iNES 1.0 image with no
CHR-ROM **32 KiB** of CHR-RAM — a size no MMC1 board carries. Every CHR-RAM MMC1 board (SNROM,
SOROM, SUROM, SXROM, SGROM) has 8 KiB and no more, so a declared CHR-ROM of 0 *is* the 8 KiB
board. Reading the padded buffer instead would have passed the two test ROMs and left every
iNES 1.0 SNROM dump — The Legend of Zelda, Zelda II, Final Fantasy — on the wrong wiring. The
PRG-RAM size is the effective one, which for iNES 1.0 is the 8 KiB default: a dump that cannot
be told from an SNROM is an SNROM here, as it is for the test ROM's own driver. A 512 KiB
PRG-ROM is excluded by the same comparison `update_banks` already uses for `outer`, so the two
readings of bit 4 can never both be live.

### The shape, and why it costs nothing

`prg_ram_enabled` needs *both* registers on an SNROM. Written as an expression
(`(self.prg | (self.chr0 & mask)) & 0x10 == 0`) it would put a second load and a mask on the CPU's
$6000-$7FFF decode for every MMC1 game, so the answer is a `bool` field recomputed where the two
registers commit — `update_banks`, which every register write already ends with. It is derived
state, so nothing new is written to the savestate and **every mapper-1 state from before this lot
loads unchanged**; `load_registers` recomputes the bool after reading the registers back
(REQ-CORE-091).

Registers in `$4020-$5FFF`: none — the MMC1 decodes `$8000-$FFFF` only.

### Result

* `cargo fmt --check`, `cargo clippy -p zax-nes -p zax-harness --all-targets -D warnings`: clean.
* `cargo test --release -p zax-nes -p zax-harness`: 0 failed.
* `ZAXNES_TEST_ROMS=…/assets-free/test-roms cargo test --release -p zax-harness --test
  test_roms_tests`: 3 passed — the 40 Holy Mapperel rows now include `M1_P128K` and
  `M1_P128K_CR8K` as `pass`, the other ten MMC1 boards still `pass`.
* Generated code: `Board::ppu_peek` 240, `Board::ppu_read` 241,
  `Ppu2C02::read_pattern_planes` 106, `Ppu2C02::read_nametable_as` 107 — all four unchanged, and
  the whole-crate diff is the three load-time / register-commit symbols in the table above.

## Step 2 — Q-117, the UNROM 512 bank latch

### What R0 answered

Q-117 asked which bank the flash chip sees, given that — as MF-2 read the page — every byte of
a command sequence is also a write to the bank latch. R0 reread the NESdev *UNROM 512* page and
none of the ticket's three readings was right:

> **sur une configuration reprogrammable (sous-mappeur 0 avec le bit de pile, sous-mappeurs 1, 3
> et 4), le verrou de banque est en `$C000-$FFFF` seulement**, et `$8000-$BFFF` est la fenêtre
> d'écriture de la flash.

The page says it in the register diagram itself: `D~[NCCP PPPP] A~[1... .... .... ....]`, and
then "For self-flashable configurations, the address range shifts to `$C000-$FFFF`" with
`A~[11.. .... .... ....]`. And the sequences it gives put a bank write in front of **every**
command byte:

* erase — `$C000:$01, $9555:$AA`; `$C000:$00, $AAAA:$55`; `$C000:$01, $9555:$80`;
  `$C000:$01, $9555:$AA`; `$C000:$00, $AAAA:$55`; `$C000:BANK, ADDR:$30`;
* program — `$C000:$01, $9555:$AA`; `$C000:$00, $AAAA:$55`; `$C000:$01, $9555:$A0`;
  `$C000:BANK, ADDR:DATA`.

The chip sees `bank × $4000 + (ADDR & $3FFF)`, so bank 1 at `$9555` is chip `$5555` and bank 0
at `$AAAA` is chip `$2AAA` — the SST39SF040's own command addresses — and the **last** write of
each sequence reaches the bank the driver named. The interaction MF-2 had to reason about does
not exist: the bank writes are at `$C000`, outside the window, so they never touch the state
machine.

### What changed

Three things, and the first pays for the other two:

1. **`can_flash` and `has_led_register` collapse into one `latch_base: u16`**, $C000 on a
   self-flashable configuration and $8000 otherwise. "Self-flashable" is "no bus conflicts",
   since they "interfere with the self-flashing procedure" — the set
   `registry::has_bus_conflicts` already decides, so there is no new load-time predicate. The
   whole of `write_register` becomes

       if address < self.latch_base { self.step_flash(…); return; }
       self.register = value;
       self.update_banks(memory);

   One compare answers both "is this the flash window?" and "is this the latch?", and on a
   non-flashable board `address < 0x8000` is unreachable after the guard above it, so
   `step_flash` is simply never entered.
2. **`step_flash` matches on `chip_address(address)`** — `(register & $1F) × $4000 +
   (address & $3FFF)` — against `$5555` and `$2AAA` instead of on the CPU addresses `$9555` and
   `$AAAA`.
3. **MR-30-5 falls out of the new rule.** Submapper 4's LED register is at `A~[10..]`, which is
   where its flash window is; its bank register is at `$C000-$FFFF` like every other
   self-flashable board's. The special case is gone and
   `mr_30_5_unrom512_submapper_4_does_not_bank_through_the_led_register` passes unchanged.

A side effect of the page's own sequences, worth writing down: `$C000:$01` and `$C000:$00` also
set the CHR-RAM bank bits to 0 and the arrangement bit to 0, because they are writes to the
whole register. A real driver lives with that; so does ours.

### The tests

`mr_30_4_unrom512_programs_a_byte_of_its_own_flash` is replaced by
`q_117_unrom512_programs_a_byte_of_its_own_flash`, which drives the page's sequences verbatim
through two helpers (`erase_sector`, `program_byte`) that write the `$C000` bank in front of
every byte. Two new tests pin what the answer buys: the sequences reach **the bank the driver
selects** (erase the same offset in banks 1 and 3, program into 1 and not 3), and an unlock on
the wrong chip address — `$9555` latched to bank 2, chip `$1555` — programs nothing. A fourth,
in `mapper_tests.rs`, pins the other side: a configuration that is not self-flashable keeps the
latch over the whole of `$8000-$FFFF`.

Four existing tests moved their bank writes from `$8000` to `$C000`, because their images are
self-flashable configurations: `req_core_100_unrom512_switches_prg_chr_ram_and_mirroring`,
`mr_30_1_unrom512_byte6_08_is_switchable_one_screen`,
`mr_30_3_unrom512_submapper_3_switches_horizontal_and_vertical`, and the battery arm of
`mr_bus_1_unrom512_conflicts_follow_the_submapper_and_the_battery_bit`. The non-flashable arms
of that last one still write `$8100`, which is the point.

### Save states and the $4020-$5FFF rule

`latch_base` is derived at load, as `can_flash` and `has_led_register` were, so **no savestate
byte changed**: a mapper-30 state written before this lot loads unchanged (`register`,
`flash_step`, `flash_erasing`, in that order).

Registers in `$4020-$5FFF`: none. The UNROM 512 decodes `$8000-$FFFF` only, and REQ-CORE-100
gives the board no PRG-RAM at all (MR-30-2).

### Result

* `cargo fmt --check`, `cargo clippy -p zax-nes -p zax-harness --all-targets -D warnings`: clean.
* `cargo test --release -p zax-nes -p zax-harness`: 0 failed.
* Generated code: `Board::ppu_peek` 240, `Board::ppu_read` 241,
  `Ppu2C02::read_pattern_planes` 106, `Ppu2C02::read_nametable_as` 107 — unchanged. The only two
  symbols in the whole-crate diff both **shrank**: `UnRom512::write_register` 116 → 106,
  `Cartridge::create` 3471 → 3462.

## Step 3 — mapper 69, the Sunsoft FME-7

### The board

NESdev, *Sunsoft FME-7*. Everything goes through two addresses: a **command register** at
`$8000-$9FFF` (`.... CCCC`, four bits naming one of sixteen commands) and a **parameter
register** at `$A000-$BFFF`, where "Writing here invokes the command with the specified parameter
byte". The sixteen commands:

| command | effect |
| --- | --- |
| `$0`-`$7` | 1 KiB CHR bank at PPU `$0000-$03FF` … `$1C00-$1FFF`, `BBBB BBBB` |
| `$8` | the `$6000-$7FFF` window, `ERbB BBBB` |
| `$9`-`$B` | 8 KiB PRG bank at `$8000`, `$A000`, `$C000`, `..bB BBBB` |
| `$C` | nametable arrangement, `.... ..MM` |
| `$D` | IRQ control, `C... ...T` |
| `$E`, `$F` | low and high byte of the IRQ counter |

`$E000-$FFFF` is absent from the PRG table because the last bank is wired there. Mapper 69 is
outside REQ-CORE-100's table; REQ-CORE-102 says "Ajouter des mappers au-delà de la liste est
permis", and Holy Mapperel's `README.md` names the boards: "069: JLROM, JSROM (Batman: Return of
the Joker, Gimmick!, Hebereke)".

**Command `$8`, the one window that is not plain.** `E` is the RAM chip's `+CE` and `R` picks
"0=ROM, 1=RAM", so the window has three states: RAM (both set), **open bus** (`R` set, `E`
clear — the chip deselected), and a plain 8 KiB PRG-**ROM** bank (`R` clear, whatever `E` says).
`Cartridge::default_cpu_read` reads `$6000-$7FFF` from PRG-RAM when `prg_ram_enabled` says so and
otherwise asks the board, so the ROM and open-bus states are served by a `try_read_prg` override
and nothing else changes: the `$8000-$FFFF` fast path stays `Cartridge::read_rom_window` over the
default `rom_windows`.

`ram_readable` and `rom_offset_6000` are precomputed from the register, so `prg_ram_enabled` — on
the CPU's `$6000-$7FFF` decode — is one load and one test like every other board's. The bank count
for the wrap comes from `BankWindows::prg_bank_count_8k()` rather than from the ROM, which lets
`load_registers` (handed a reader and no memories) bring both derived halves back with the
register; the first version read the ROM length, and the savestate test caught it at once.

**Command `$C` and the arrangement vocabulary.** The page names the four values "0: Horizontal
(A10), 1: Vertical (A11), 2: 1scA (GND), 3: 1scB (VCC)" — the *arrangement* wording of the iNES
header, where "(A10)" means CIRAM A10 is driven by PPU A10, i.e. the horizontal arrangement, which
every board page in this crate calls **vertical mirroring**. Mesen 2 reads it the same way
(`SunsoftFme7.h:103-110`: 0 → `Vertical`), the two sources agree, and the Holy Mapperel ROM is the
witness: it "detects what mapper it's running on through mirroring", so the wrong reading would
have beeped `MIR` instead of printing `J*ROM (FME-7)`.

**The IRQ.** "When the IRQ counter is decremented from `$0000` to `$FFFF` an IRQ is generated",
the 16-bit counter running off M2 — one step per CPU cycle — while `$D`'s `C` bit allows it, and
"Writes to the IRQ counter registers directly set the lower or upper eight bits of the counter":
`$E` and `$F` are halves of the counter, not a latch and a reload. The line is only pulled if `T`
was set *at the moment of the wrap* — the wrap is not latched into a flag a later `T` could
reveal, which is how Mesen reads the page too (`SunsoftFme7.h:59-66`) and what
`req_core_100_the_fme7_needs_both_command_d_bits` pins. "All writes to this register acknowledge
an active IRQ" is command `$D` and nothing else. No `ppu_irq_horizon` override: nothing the PPU
does can move this line, which is the trait's default.

Registers in `$4020-$5FFF`: **none** — the board decodes `$6000-$FFFF` only, so the brief's
item 5 does not bite.

The 5B's three sound channels are at `$C000-$DFFF` and `$E000-$FFFF`; a write there is ignored for
now, which is exactly what a 5A — the same chip without the sound — does. Step 4 adds them.

### The cost of a new `Board` variant, measured properly

This is the first step of the lot that adds a variant, so it is where the method is set down.

**`board.rs` now says that a board is appended, never inserted**, and the note there gives the two
measured reasons: `Mmc5` must stay at index 5 (the niche-filling tag lives in its last `bool`, and
every dispatch prologue is `movzbl 1355(%rcx); subb $2; movl $5; cmovael` — a literal 5), and
appending leaves the existing discriminants, and so the existing jump-table slots, where they are.
Checked both ways on `Board::ppu_peek`: appended, the arms before the new one are
instruction-identical; inserted after `GxRom`, four arms move and their operand offsets move with
them.

**The four functions the lot's rules name:**

| function | step 2 | step 3 |
| --- | ---: | ---: |
| `Board::ppu_peek` | 240 | **251** |
| `Board::ppu_read` | 241 | **252** |
| `Ppu2C02::read_pattern_planes` | 106 | 106 (byte-identical) |
| `Ppu2C02::read_nametable_as` | 107 | 107 (instruction-identical; one immediate widens by a bit, being a bitset over the variants) |

The +11 on the two `Board` fetch functions is **not added work**. The prologue, the jump table's
shape and every existing arm's own instructions are unchanged; what changed is that LLVM stopped
tail-*merging* the shared "offset in range → load → return" tail and tail-*duplicated* it into
four arms instead. For those arms the executed path goes from
`jmp; cmpq; jb(taken); movzbl; retq` to `cmpq; jae(not taken); movzbl; retq` — **one instruction
and two taken branches fewer**. The same check on the two dispatches that run every CPU cycle,
`Board::on_cpu_cycle` and `Board::irq_asserted`, shows the existing arms **byte-identical** with
the FME-7's appended at the end.

**Wall clock, because the static count is only a proxy.** `zax-harness bench-core`, 3600 frames
after 300 of warm-up, interleaved runs of two binaries that differ only by this step, on
`mmc3_test/5-MMC3.nes` (mapper 4) and `holy-mapperel/M0_P32K_C8K_V.nes` (mapper 0) — neither can
execute a line of FME-7 code:

| build | 5-MMC3, min of 6 | NROM, min of 4 |
| --- | ---: | ---: |
| before the step | 0.9932 ms/frame | 0.8226 |
| the variant with an **empty** board (control) | 1.0079 | 0.8225 |
| the step as committed | 1.0243 | 0.8345 |

So the 27th variant with no code behind it costs **0.0 %** on NROM, and the step as committed
about 1.4 % — on a ROM that never reaches the board. That is code placement and register
allocation, not work: a second run of the same three binaries in a later session put *before* at
0.8172-0.8387 (a ±1.5 % spread of its own), and a behaviour-neutral perturbation of the FME-7
(`#[inline(never)]` on two of its own private functions, which no other board can reach) landed
between the two at 0.8323. Two things did change in the PPU and the CPU without being asked to —
`Ppu2C02::step_dot` is the same instructions with `%r15` renamed to `%r13`, and
`Cpu6502::execute` stopped being a symbol of its own because `execute_without_operand` left it,
which let it inline into `step` — and neither followed from a source change on those paths.
`#[inline]` on `execute_without_operand` does not move it back, and the CPU is out of this lot's
scope, so no attribute was added for it.

**For R0:** the instrument for this question is callgrind, whose instruction counts are
deterministic, not this host's wall clock. The claim this step makes is the narrow one that can be
checked statically and was: no existing board's dispatch arm gained an instruction, and the two
fragile PPU functions are unchanged. Three of this lot's remaining boards go **into existing
boards** rather than into new variants — mapper 118 and the MC-ACC because the brief asks for them
"as a variant of the MMC3 board" and "confined to the MMC3 board", mapper 180 because it is a
UNROM wiring — so mapper 69 and mapper 78 are the only two variants the lot adds.

### Result

* `cargo fmt --check`, `cargo clippy -p zax-nes -p zax-harness --all-targets -D warnings`: clean.
* `cargo test --release -p zax-nes -p zax-harness`: 0 failed. Three existing tests had to move
  with the registry: `req_core_100_every_required_mapper_is_registered` now asserts REQ-CORE-100's
  table is a **subset** of `SUPPORTED_MAPPERS` and names the two numbers REQ-CORE-102 lets this
  build add (69 and 156), and the two "an unimplemented mapper is refused" tests use 68 where they
  used 69.
* `zax-harness test-roms --filter M69`: **2 pass**, `J*ROM (FME-7): 128K PRG ROM, 8K PRG RAM OK,
  64K CHR ROM OK, detailed 0000` — the IRQ digit included, which is the FME-7 interval timer Holy
  Mapperel's README says it measures.
* `ZAXNES_TEST_ROMS=… cargo test --release -p zax-harness --test test_roms_tests`: 3 passed, with
  the two mapper-69 rows flipped from `unsupported` to `pass`.

## Step 4 — the Sunsoft 5B's audio

### What the sources give, and what they do not

NESdev, *Sunsoft 5B audio*: the chip carries a YM2149F (an AY-3-8910) reached through two
addresses — `$C000-$DFFF` selects one of sixteen registers, `$E000-$FFFF` writes it — with three
square channels, a noise generator and an envelope generator. The register map, the `$07` mixer,
the 17-bit LFSR "with taps at bits 16 and 13", the eight envelope shapes of `$0D` and the
"1.5db logarithmic steps" of the envelope's 32 levels are all there, and all implemented.

**Mesen 2 was not adaptable.** Its `Sunsoft5bAudio` keeps the sixteen registers and the three
tone generators, but `UpdateOutputLevel` (`Sunsoft5bAudio.h:63-74`) only ever looks at the tone:
it has **neither the envelope nor the noise**, which the brief asks for and the page describes. The
one place the two sources meet is the volume law — Mesen builds its table from the same 1.5 dB
figure, twice per volume step — and that agreement is recorded on
`mixer::SUNSOFT_5B_RATIO`. Nothing else was taken from it, so `sunsoft5b_audio.rs` carries no
adaptation header.

### The clock, and why one prescaler drives all three generators

The page gives the tone and the noise as `Clock / (32 × Period)` and the envelope **step** as
`Clock / (16 × Period)`, with the clock at 1.789773 MHz — the CPU's. Those three are exactly the
AY's own datasheet relations (`fT = fCHIP / (16 × TP)`, `fN = fCHIP / (16 × NP)`,
`fE = fCHIP / (256 × EP)` for the complete 32-step cycle) **for a chip clocked at half the CPU
clock**, which is what reconciles the page's /32 with the datasheet's /16 and makes its envelope
formula the step rate rather than the cycle rate. So:

* a tone toggles every 16 × period CPU cycles,
* the noise shifts every 32 × period,
* the envelope steps every 16 × period,

and a single divide-by-16 of the CPU clock drives everything: on each of its ticks a tone counter
takes one step, the noise counter two, the envelope counter one. Writing it that way is both the
hardware's shape and the cheap one — the real work happens once per sixteen CPU cycles.

### The envelope's shape machine

`$0D` is "envelope reset and shape: continue (C), attack (A), alternate (a), hold (H)". A write
restarts the ramp at the end `A` names and releases a held envelope. When a ramp reaches an end of
the 0-31 range:

* `C = 0`: the level goes to 0 and stays, whichever way the ramp went (the four `\___` and `/___`
  shapes);
* `C = 1`, `H = 1`: stop, **after** `a` has had its chance to flip the direction — which is what
  makes `\‾‾` and `/___` differ from `\___` and `/‾‾`;
* `C = 1`, `H = 0`: go again, reversed if `a` and from the same end if not.

All eight shapes fall out of those three lines, which is why the code is three branches and not a
table.

### Level and polarity: Q-118

REQ-CORE-075 leaves this board's sound **optional** and gives it no level, and the page says only
that "It is very loud compared to other audio expansion carts" — no ratio, no resistor values,
nothing about polarity. So the lot opens **Q-118** with a provisional choice, taken the way
AMD-061 §6.4 took the MMC5's: the one quantitative sentence available is "maximum level 31
corresponds to the 4-bit volume register level 15", so **level 31 of one channel is one APU pulse
at volume 15** — `PULSE_TABLE[15]` = 4 890 — and the other levels follow at 1.5 dB apart.
`mixer::SUNSOFT_5B_TABLE` is built downwards from that anchor so the top entry is exact.

The three channels are added **after** the table, not through a shared index as the MMC5's two
squares are: the page requires it ("The three output channels are mixed together linearly") and the
reason is physical — the MMC5 sits in the APU's own non-linear network, the 5B is a separate chip
whose single output enters by the cartridge's audio pin. `M` therefore caps at 3 × 4 890 =
**14 670**, 45 % of full scale, which is consistent with "very loud".

`M` is **positive**, so the APU subtracts it as it subtracts the MMC5's. That is Q-118's
provisional answer on polarity too, and the ticket says why it is not free to reverse:
`mixer::clip` only implements the lower bound *because* `M` cannot be negative, and that is what
leaves a game without expansion audio the three instructions AMD-061 §6.2 left it. One constant
changes the level if R0 wants another; reversing the polarity would reopen §6.2.

### Cost

`Sunsoft5bAudio` is the board's own type, held by `Fme7`, and the level is cached in a field
recomputed only where a generator or a register moves — so `expansion_audio_output`, which the bus
reads every CPU cycle of a mapper-69 game, is one load. The bus's own gate is unchanged:
`NesBus::push_expansion_audio` is `#[inline(never)]` and only called when the load-time
`expansion_audio` bool is set, which is RU-3a's arrangement and is why the second board costs the
other thirty mapper numbers nothing.

| function | step 3 | step 4 |
| --- | ---: | ---: |
| `Board::ppu_peek` | 251 | 251 (byte-identical) |
| `Board::ppu_read` | 252 | 252 (byte-identical) |
| `Ppu2C02::read_pattern_planes` | 106 | 106 (byte-identical) |
| `Ppu2C02::read_nametable_as` | 107 | 107 (byte-identical) |

The whole-crate diff is thirteen symbols, every one of them either inside `Fme7` /
`Sunsoft5bAudio` or on the expansion-audio path that only an MMC5 or a mapper-69 cartridge
reaches: `Board::expansion_audio_output` 10 → 26, `Cartridge::expansion_audio_output` 11 → 27,
`NesBus::push_expansion_audio` 18 → 34, `NesBus::attach` 54 → 68,
`Cartridge::has_expansion_audio` 12 → 21, `Cartridge::create` 3565 → 3574, `Board::reset`
482 → 489. **`Board::on_cpu_cycle` got 7 instructions smaller** (107 → 100): the FME-7's arm grew
past the inlining threshold and became a call to `Fme7::on_cpu_cycle`, which takes its body out of
the dispatch every other board shares.

### Save states

The audio block is written **last** in `Fme7::save_registers` and read only if
`reader.remaining() > 0`, so a mapper-69 state written between step 3 and this step loads with a
silent chip — which is the state of a 5A, and of a 5B before its first register write
(REQ-CORE-091). `load_state` also refuses an impossible prescaler or an envelope level above 31,
neither of which this build can write, so a corrupt block cannot index past the table.

### Result

* `cargo fmt --check`, `cargo clippy -p zax-nes -p zax-harness --all-targets -D warnings`: clean.
* `cargo test --release -p zax-nes -p zax-harness`: 0 failed, including the 15 new tests of
  `sunsoft5b_audio_tests.rs` (the volume law recomputed from 1.5 dB with `powf`, the linear sum of
  three channels, the 32-cycles-per-period square, the `$07` gate including the "constant signal
  at the specified volume" case, the noise gating a channel, four envelope shapes, the `$0D`
  restart, the subtraction in the mixer, and a select above `$0F` writing nothing).
* `ZAXNES_TEST_ROMS=… cargo test --release -p zax-harness --test test_roms_tests`: 3 passed. The
  Holy Mapperel mapper-69 ROMs do not exercise the sound — the suite has no audio test — so they
  are unchanged at `pass`.

## Step 5 — mapper 118, the TxSROM

### The board

nesdev, *INES Mapper 118*: TKSROM and TLSROM, running *Armadillo*, *Goal! Two*, *NES Play Action
Football* and *Pro Sport Hockey*. It is an MMC3 with **CHR A17 — bit 7 of the bank number — wired
to CIRAM A10**, so "the CHR bankswitching directly affects the mirroring mapping the nametable
RAM", and the registers that drive it are the ones mapped to **the first pattern table**, which
bit 7 of `$8000` chooses between:

* bit 7 clear: the two 2 KiB registers R0 and R1 sit at PPU `$0000-$0FFF`, and bit 7 of each takes
  **two** nametables (R0 → 0 and 1, R1 → 2 and 3);
* bit 7 set — "mapping the 1KB CHR banks to the first pattern table by setting bit 7 of `$8000`" —
  the four 1 KiB registers R2-R5 sit there, one nametable each.

And the mirroring register "is bypassed by the configuration described above, so writing here has
no effect": `$A000` does nothing. Mesen 2 reads the register the same way (`TxSRom.h:16-39`, and
its `UpdateMirroring` is empty with the comment "8001 writes are used to setup mirroring
instead"), so the two sources agree.

### Where it lives

**Inside the MMC3 board, chosen at load**, which is what the brief asks for and what the board is:
`Mmc3::new` now takes the mapper number, `registry::create_board` maps `4 | 118` to the same
variant, and `SUPPORTED_MAPPERS` gains 118 (REQ-CORE-102). **No new `Board` variant**, so none of
step 3's enum arithmetic applies here. Three load-time facts fall out of the number:

* `is_txsrom`, which `refresh_nametable_pages` reads;
* `ignore_a000`, which the four-screen flag and NES 2.0 submapper 2 already set (MR-4-9) and which
  a TxSROM now sets too — one condition instead of a second test in `write_register`;
* a TxSROM is neither an MMC6 (submapper 1) nor a revision-A MMC3 (submapper 4), since those
  submappers belong to mapper 4; both are now `!is_txsrom && …`.

The four answers live in a `[i32; 4]` field, `FOLLOW_MIRRORING` from one end of a plain MMC3's run
to the other — exactly what the trait's default answers — and the pages bit 7 names on a TxSROM.
A field and not a `match`, for the reason `UnRom512::nametable_answer` gives: `Board::nametable_page`
has to stay one load per arm. It is recomputed in `update_banks`, which every register write
already ends with, and in `load_registers` through it.

`Cartridge::nametable_pages_vary` now names mapper 118 beside mapper 5: the TxSROM is the second
board whose pages a register write can move, so the four answers are re-cached after every CPU
write to those two cartridges and left alone for the other thirty-one mapper numbers.

### Cost

| function | step 4 | step 5 |
| --- | ---: | ---: |
| `Board::ppu_peek` | 251 | **235** |
| `Board::ppu_read` | 252 | **236** |
| `Ppu2C02::read_pattern_planes` | 106 | 106 (byte-identical) |
| `Ppu2C02::read_nametable_as` | 107 | 107 (byte-identical) |

The two fetch dispatches came out **16 instructions smaller than at step 4 and 5 smaller than
before the lot** (240/241), with the prologue and the niche unchanged — `movzbl 1355(%rcx)`,
`movl $5` — so the tail-duplication step 3 measured has been undone by the same heuristic going
the other way. `Ppu2C02::step_dot` is back to its pre-lot 1409. Nothing in this step touched any of
those four functions; this is the register-allocation and block-placement lottery step 3
documented, and it is the clearest possible evidence that the numbers in that step were not added
work.

Where real code moved: `Mmc3::update_banks` 144 → 167 (the page refresh),
`Board::nametable_page` 28 → 33 (the new arm: a bounds-clamped load from the field),
`Cartridge::create` 3574 → 3667 (the load-time decision). Several symbols shrank for the same
lottery reason — `Cartridge::cpu_write` 181 → 120, `Cartridge::reset` 153 → 30,
`Board::rom_windows` 239 → 213 — which is worth reporting but not worth claiming.

Registers in `$4020-$5FFF`: **none**. The TxSROM decodes what the MMC3 decodes, `$6000-$FFFF`.

### Save states

`nametable_pages` is derived from the bank registers and nothing was added to
`Mmc3::save_registers`, so **every mapper-4 savestate written before this lot loads byte for
byte**, and a mapper-118 state brings its nametables back with its banks
(`req_core_090_the_txsrom_nametables_come_back_with_its_banks`).

### Result

* `cargo fmt --check`, `cargo clippy -p zax-nes -p zax-harness --all-targets -D warnings`: clean.
* `cargo test --release -p zax-nes -p zax-harness`: 0 failed, with five new tests — the two
  register sets, the switch between them, `$A000` doing nothing on a TxSROM while it still drives
  the arrangement on a plain MMC3, and the savestate.
* `zax-harness test-roms --filter M118`: **1 pass**, `T*SROM (MMC3): 128K PRG ROM, 8K PRG RAM OK,
  64K CHR ROM OK, detailed 0000`.
* `ZAXNES_TEST_ROMS=… cargo test --release -p zax-harness --test test_roms_tests`: 3 passed, with
  the mapper-118 row flipped from `unsupported` to `pass`.

## Step 6 — mapper 180, the other wiring of the UxROM board

nesdev, *INES Mapper 180*, is three sentences and they decide everything. The board name is
**UNROM**, the game is *Crazy Climber* and it is the only one, the register at `$8000-$FFFF`
selects "16 KB PRG ROM bank for CPU `$C000-$FFFF`" with `$8000-$BFFF` "fixed to the first bank",
and the reason is spelled out: the board "uses AND logic (74HC08) instead of OR logic (74HC32),
producing a fixed first bank at `$8000` rather than a fixed last bank at `$C000`".

So it is the UxROM board with the other half of the 74 series, and it goes **inside `UxRom`** as a
load-time `fixed_first` flag rather than into a `Board` variant of its own — which is both what the
page says it is and what `board.rs`'s note asks for. `registry::create_board` maps `2 | 180` to the
same variant and `SUPPORTED_MAPPERS` gains 180 (REQ-CORE-102).

**Bus conflicts.** The page's characteristics table gives the board "Bus conflicts: Yes" with no
submapper clause — the AND logic that fixes the first bank is exactly what leaves the ROM driving
the data bus during a write — so `registry::has_bus_conflicts` puts 180 with 11 and 66 rather than
with the submapper-gated 2, 3 and 7 (MR-BUS-1). Mapper 2 is untouched: a submapper-0 dump still has
none, which `mr_bus_1_uxrom_without_a_submapper_has_no_conflict` pins.

Registers in `$4020-$5FFF`: **none**; `$8000-$FFFF` only.

### Cost

| function | step 5 | step 6 |
| --- | ---: | ---: |
| `Board::ppu_peek` | 235 | 235 (byte-identical) |
| `Board::ppu_read` | 236 | 236 (byte-identical) |
| `Ppu2C02::read_pattern_planes` | 106 | 106 (byte-identical) |
| `Ppu2C02::read_nametable_as` | 107 | 107 (byte-identical) |

Seven symbols in the whole-crate diff, and five of them **shrank**: the branch made
`UxRom::update_banks` worth outlining (new, 76 instructions), which took its body out of
`Board::write_register` (633 → 597) and `Board::reset` (489 → 459). `Cartridge::create` 3667 →
3701 for the load-time decision. Nothing else.

### Save states

No new field is written: `fixed_first` and `mapper_number` are derived from the ROM at load, and
`UxRom::save_registers` still writes the one bank byte. **Every mapper-2 savestate loads
unchanged.**

### Result

* `cargo fmt --check`, `cargo clippy -p zax-nes -p zax-harness --all-targets -D warnings`: clean.
* `cargo test --release -p zax-nes -p zax-harness`: 0 failed, with two new tests — mapper 180's
  fixed first bank and switchable `$C000`, written through a `$FF`-filled bank because the board
  has conflicts, and mapper 2 keeping the opposite wiring.
* `zax-harness test-roms --filter M180`: **2 pass**, `U*ROM (7408): 128K PRG ROM, 8K PRG RAM OK,
  8K CHR RAM OK, detailed 0000`.
* `ZAXNES_TEST_ROMS=… cargo test --release -p zax-harness --test test_roms_tests`: 3 passed, with
  both mapper-180 rows flipped from `unsupported` to `pass`.

## Step 7 — mapper 78, the Irem IF-12 and the Jaleco board of *Uchuusen*

### The board

nesdev, *INES Mapper 078*: the two cartridges are *Holy Diver* and *Uchuusen - Cosmo Carrier*,
both "a CNROM and a UNROM" in discrete logic, with one register at `$8000-$FFFF` doing all three
jobs — `CCCC MPPP`: bits 0-2 the 16 KiB PRG bank at `$8000-$BFFF`, bit 3 the mirroring, bits 4-7
the 8 KiB CHR bank, and `$C000-$FFFF` fixed to the last 16 KiB. "Subject to bus conflicts: Yes",
unconditionally, which puts it with mappers 11, 66 and 180 in
`registry::has_bus_conflicts` rather than with the submapper-gated 2, 3 and 7.

**Bit 3 means one thing on each board**, and that is the whole of the two wirings:

* *Holy Diver*, NES 2.0 submapper 3: "0 = H, 1 = V", through "a 74HC00, which serves as a 2:1 mux
  of PPU A10 and PPU A11 into CIRAM A10";
* *Cosmo Carrier*, submapper 1: "0 = 1scA, 1 = 1scB".

Mesen 2 reads bit 3 the same way for each (`JalecoJf16.h:25-31`), so the two sources agree on the
direction — and the Holy Mapperel ROM, which identifies the board by its mirroring, is the
witness: it prints `HOLY DIVER`.

### The iNES 1.0 heuristic

"Emulators SHOULD support NES 2.0 submappers: 1 for Uchuusen and 3 for Holy Diver", and for an
image that carries neither the page gives the rule: "iNES1 ROM image headers often set the
'alternative nametables' flag for Holy Diver and cleared it for Cosmo Carrier". So byte 6 bit 3
decides, with the submapper winning where there is one. **Mesen 2 has no equivalent** — it reads
`SubMapperID` alone, so an iNES 1.0 *Holy Diver* gets Cosmo Carrier's one-screen wiring there —
and AMD-061 §1 makes the documentation prevail.

Our loader reads byte 6 bit 3 as a four-screen header, and nothing comes of that: `update_banks`
sets the arrangement from the register on the first reset, so `memory.mirroring()` is never left
at `FourScreen`. `req_core_100_mapper_78_never_keeps_the_four_screen_header` pins it.

Registers in `$4020-$5FFF`: **none**.

### Where it lives

A genuinely new board — not a wiring of an existing one — so it is the lot's **second and last**
new `Board` variant, appended after `Fme7` per `board.rs`'s note. It sits in `discrete.rs`, which
is where "a CNROM and a UNROM" belongs.

### Cost

| function | step 6 | step 7 |
| --- | ---: | ---: |
| `Board::ppu_peek` | 235 | 235 (byte-identical, same prologue and niche) |
| `Board::ppu_read` | 236 | 236 (byte-identical) |
| `Ppu2C02::read_pattern_planes` | 106 | 106 (byte-identical) |
| `Ppu2C02::read_nametable_as` | 107 | 107 (byte-identical) |

So the second appended variant cost the two fetch dispatches **nothing at all**, where the first
cost +11 and then got it back at step 5 — which is the behaviour of a placement heuristic and not
of a cost that scales with the number of arms. The rest of the diff is the new board's own symbols,
+2 to +7 on the `Board` dispatches that gained an arm (`write_register`, `reset`, `save_registers`,
`try_read_prg`, `expansion_audio_output`), `Cartridge::create` 3701 → 3734, and a crowd of
`discrete.rs` symbols whose mangled disambiguator shifted because the new type was inserted before
them in the file — those are renames, not changes. `Ppu2C02::step_dot` went 1409 → 1419, the same
±10 lottery step 3 documented.

### Save states

One new board with one register byte, and nothing added to any existing board's block.

### Result

* `cargo fmt --check`, `cargo clippy -p zax-nes -p zax-harness --all-targets -D warnings`: clean.
* `cargo test --release -p zax-nes -p zax-harness`: 0 failed, with four new tests — the register's
  three fields, bit 3 on each of the two boards, the byte-6 heuristic with the submapper winning
  over it, and the four-screen header never surviving.
* `zax-harness test-roms --filter M78`: **1 pass**, `HOLY DIVER: 128K PRG ROM, 8K PRG RAM OK,
  64K CHR ROM OK, detailed 0000`.
* `ZAXNES_TEST_ROMS=… cargo test --release -p zax-harness --test test_roms_tests`: 3 passed. **All
  forty Holy Mapperel rows are now `pass`**, which is what this lot set out to do for that suite.

## Step 8 — MR-4-3, the Acclaim MC-ACC

### The finding

nesdev, *MMC3*: "Acclaim's MC-ACC chip is their own variant of the MMC3 ... The only known
difference is that the scanline counter uses a ÷8 prescaler instead of filtering A12, clocked by
falling edges", and *NES 2.0 submappers* gives mapper 4 "Submapper 3: MC-ACC chip variant".
MR-1b's own analysis adds the two details that make it implementable, from krikzz's research by
way of Mesen 2's `McAcc`: the clock lands on the **first** fall of each group of eight ("Counter
clocking happens once per 8 A12 cycles at first cycle"), and "writing to `$C001` resets pulse
counter". *The Incredible Crash Dummies* and *Mickey's Safari in Letterland* are the titles named
in public discussion of the chip; a dump is recognised by its submapper.

Everything else is the MMC3's, revision B included — submapper 3 is neither submapper 1 (MMC6)
nor submapper 4 (the NEC/Sharp revision A of AMD-061 §8), so those two stay `false`.

### `MC_ACC` is a type parameter, not a field

This is the one decision of the step, and MR-4-3 had already stated the requirement: the right
shape must be one where "no MMC3 game pays a branch", because `on_ppu_address_bus` runs on
**every PPU fetch** and the MMC3 is in the golden-core set.

A field would have cost two instructions there. `A12Watcher::rose` opens with
`let high = (address & 0x1000) != 0`, which compiles to a single `andw $4096, %r8w` whose own
flags answer the question; comparing against a field instead — `== self.high_value`, the obvious
way to make one watcher count either edge — needs the field loaded and a `cmpw` as well. That
version was written, measured at **+2 instructions per fetch for every mapper-4 and mapper-64
game**, and thrown away.

What is committed instead is `Mmc3Board<const MC_ACC: bool>`, with `Mmc3 = Mmc3Board<false>` and
`McAcc = Mmc3Board<true>`. Both boards share **every method body**; the four places the clone
differs are `if MC_ACC` branches that the `const` parameter folds away at compile time:

* the watcher it is built with, and whether `on_ppu_address_bus` asks it for a rise or a fall
  (`A12Watcher::fell`, a second method rather than a mode inside `rose`, for the reason above);
* the ÷8 prescaler around `clock_irq_counter`;
* the `$C001` reset of that prescaler;
* `ppu_irq_horizon`'s dots-per-clock, and the prescaler byte in the savestate.

Mesen 2 writes a separate class for the same reason (`MapperFactory.cpp:285-290` → `McAcc`); the
difference is that this shares the code instead of copying it, so the TxSROM, the MMC6, the two
IRQ revisions and the MC-ACC cannot drift apart.

### What the generated code says

| function | step 7 | step 8 |
| --- | ---: | ---: |
| `Board::ppu_peek` | 235 | 235 (byte-identical) |
| `Board::ppu_read` | 236 | 236 (byte-identical) |
| `Ppu2C02::read_pattern_planes` | 106 | 106 (byte-identical) |
| `Ppu2C02::read_nametable_as` | 107 | 107 (byte-identical) |

And the function this step is actually about, `Board::on_ppu_address_bus`, **where it matters**:
the MMC3's own arm is

    movzbl 40(%rcx), %eax    ; self.high
    andw   $4096, %r8w
    jne    L
    testb  %al, %al
    je     L
    movq   %r9, 24(%rcx)     ; low_since = ppu_clock
    movzwl %r8w, %eax
    shrl   $12, %eax
    movb   %al, 40(%rcx)
    retq

**character for character what it was at step 7**, and so are its two other paths (the filtered
rise and the clock). The dispatch that reaches it got *shorter*: with a fourth real arm LLVM
switched from a bitset-plus-compare chain to a jump table, so an MMC3 fetch now runs
6 prologue + 4 table instructions where it ran 6 + 5, and the two correctly-predicted conditional
branches became one indirect jump whose target is constant for the life of a cartridge. The
function as a whole is 106 → 155 because the MC-ACC's arm is new code beside it.

The usual lottery moved elsewhere in the same build — `Ppu2C02::step_dot` 1419 → 1403, and the CPU
interpreter reshuffled its inlining again (`Cpu6502::resolve` 1293 → 584, `step` 829 → 1331, four
helpers folded in). Step 9 measures the lot end to end with `bench-core` rather than arguing about
those.

### Save states

The prescaler byte is written **only** when `MC_ACC`, so a mapper-4 or mapper-118 block is
byte-identical to one written before this step — `req_core_090_the_mapper_state_survives_a_save_and_restore`
compares the re-saved bytes and holds — and an MC-ACC state carries its prescaler
(`req_core_090_the_mc_acc_prescaler_survives_a_save_and_restore`).

Registers in `$4020-$5FFF`: **none**, as for every MMC3.

### Result

* `cargo fmt --check`, `cargo clippy -p zax-nes -p zax-harness --all-targets -D warnings`: clean.
* `cargo test --release -p zax-nes -p zax-harness`: 0 failed, with four new tests — the ÷8
  prescaler (eight falls where the MMC3 needs one rise), a plain mapper-4 image not being one, the
  `$C001` reset, and the savestate. The MC-ACC still reports mapper number 4.
* No test ROM covers this board: Holy Mapperel has no mapper-4 submapper-3 image and
  `mmc3_test` is an iNES 1.0 suite, so the behaviour is pinned by the unit tests alone, which is
  what MR-4-3 expected ("the fix is a new board, not a branch, so the test belongs to the lot that
  adds it").

## Step 9 — the suites, the campaign document, and the close of the lot

### The suites, re-run

`zax-harness test-roms <dir> out.json --filter … --no-shots`, against
`assets-free/test-roms`:

| suite | before MF-3 | after MF-3 |
| --- | --- | --- |
| `holy-mapperel` | 32 / 40, 2 fail, 6 unsupported | **40 / 40** |
| `nes-test-roms/mmc3_test` | 5 / 6 | 5 / 6, unchanged |
| `nes-test-roms/mmc3_test_2` | 5 / 6 | 5 / 6, unchanged |
| `nes-test-roms/mmc3_irq_tests` | 5 / 6 | 5 / 6, unchanged |
| `nes-test-roms/MMC1_A12` | manual | manual, unchanged |

The three MMC3 rows that still fail are the three the campaign marked **required to fail**: each
is the revision A half of a contradictory pair whose revision B half passes, and REQ-CORE-100 pins
mapper 4 to revision B (AMD-061's answer to Q-114 restates it). Adding the TxSROM and the MC-ACC to
that board moved none of them, which is the point — an iNES 1.0 mapper-4 image, which is what all
three suites are, is still a plain revision B MMC3.

Campaign totals: pass **196 → 204**, fail **17 → 15**, unsupported **7 → 1**, manual 111 of 331.
The one `unsupported` left in the whole campaign is `nes-test-roms/other/LINUSMUS.NES`, and it is
not a mapper: "Corrupt header: missing iNES signature".

`docs/review/MF-T-test-roms.md` carries the new totals, the eight changed `holy-mapperel` rows,
and a "What lot MF-3 changed" section with the before/after table and the commands.
`rust/crates/zax-harness/tests/test_roms_tests.rs` — the env-gated guard — now expects
`Outcome::Pass` for all forty `holy-mapperel` rows and still expects the three MMC3 failures.

### The lot's performance, measured end to end — and the one thing left open

**Static, which is what the lot's rules ask for.** Pre-lot (`7cbd9c52`) against the finished lot,
whole-crate assembly:

| function | before the lot | after | |
| --- | ---: | ---: | --- |
| `Board::ppu_peek` | 240 | **235** | −5 |
| `Board::ppu_read` | 241 | **236** | −5 |
| `Ppu2C02::read_pattern_planes` | 106 | 106 | byte-identical |
| `Ppu2C02::read_nametable_as` | 107 | 107 | byte-identical |
| `Ppu2C02::step_dot` | 1409 | **1403** | −6 |

And the function that mattered most, `Board::on_ppu_address_bus`: the MMC3's arm is **character
for character** what it was, all three of its paths, and the dispatch that reaches it is one
instruction shorter. The per-CPU-cycle dispatches — `Board::irq_asserted`,
`Board::on_cpu_cycle` — differ from their pre-lot selves **only** in field displacements (the
boards' structs grew) and in the order LLVM emitted the arms; no arm gained an instruction.

**Wall clock, which is not.** `zax-harness bench-core`, 3600 frames after 300 of warm-up, two
binaries differing only by this lot's code, interleaved runs, `core run_frame`:

| build | 5-MMC3 (mapper 4) | NROM | `M1_P128K` (mapper 1) |
| --- | ---: | ---: | ---: |
| before the boards (step 2) | 0.997 ms/frame | 0.819 | 0.826 |
| step 7 (five boards in) | 1.033 | — | — |
| the finished lot | 1.042 | 0.860 | 0.861 |

**That is about +4 %, on ROMs that cannot execute a line of any new board, and it is outside this
host's noise** — twelve interleaved pairs, every one of them the same way round. It is the lot's
one open item and it is reported here rather than explained away.

What is known about it:

* it **accumulates with the amount of new code**, not with any one step: ~+3 % by step 7 and
  ~+4 % after step 8, roughly in proportion to the ~1 500 instructions of new board code;
* it is **not the `Board` enum**: a control build with a 27th variant whose board does nothing at
  all measured **0.0 %** on NROM (step 3);
* it is **not an executed path**: see the static table above and the per-step sections;
* the compiler reshuffled the CPU interpreter's inlining twice during the lot without being asked
  to (`Cpu6502::step` 240 → 1331, `execute` and four helpers folded in, `resolve` 1293 → 584),
  which is the most plausible home for it — a hot loop five times the size is an I-cache change
  that instruction counts do not show. **Forcing the pre-lot shape back made it worse**:
  `#[inline(never)]` on `Cpu6502::execute` measured 1.078 ms/frame against the lot's 1.042 and the
  baseline's 0.997, so the inliner's new choices are not themselves the regression, and no
  attribute was left in the CPU (which is out of this lot's scope anyway);
* it is **not the reused-build-folder artefact R0 found in MF-2** ("Le +0,3/+0,9 % du premier
  passage `mf2/l1` venait du dossier de build `/root/zaxmf`, réutilisé entre branches"). Both
  binaries were rebuilt back to back after `cargo clean --release` of the five workspace crates —
  a gigabyte of artefacts removed — and the figures came back 0.997-1.002 against 1.042-1.049,
  the same +4.6 %. **The one thing that was not done is R0's own prescription**, a `git archive`
  of each commit into a brand-new folder: the C: and G: drives are at 100 % and 99 %, and filling
  G: to do it was not worth the risk. R0 should re-measure that way.

**For R0.** The instrument for this is callgrind, which counts executed instructions
deterministically, and R0 runs it on Mega Man 2 (mapper 1) and Zelda II (mapper 1) plus the
golden-core campaign. The prediction this lot makes is that **callgrind will show no board paying
instructions it did not pay before** — every claim above is checkable in the generated code — and
that if callgrind nonetheless shows a regression, the lever is the layout and inlining of the CPU
and PPU, not any of the seven boards this lot touched. Neither is in MF-3's scope; MF-4 has the
CPU, the PPU and the APU.

### The lot's state at the close

* `cargo fmt --check`: clean. `cargo clippy -p zax-nes -p zax-harness --all-targets -D warnings`:
  clean. `cargo test --release -p zax-nes -p zax-harness`: **0 failed**, no test left `#[ignore]`d
  by this lot.
* `ZAXNES_TEST_ROMS=… cargo test --release -p zax-harness --test test_roms_tests`: 3 passed.
* **Savestates**: no existing board's block changed size or order. Mapper 1, 2, 4, 30 and 118
  states written before this lot load byte for byte — mapper 4's block is identical because the
  MC-ACC's prescaler byte is written only by the clone, and mapper 1's, 2's and 30's new facts are
  all derived at load. The two new boards (69, 78) and the MC-ACC write their own.
* **Tickets**: one opened, **Q-118** (the Sunsoft 5B's mixing level and polarity, with a
  provisional choice from the one quantitative sentence the page gives). **Q-117** is implemented
  as R0 answered it. No other ambiguity needed one.
* **The `$4020-$5FFF` rule** (brief item 5): **no board of this lot has a register there**. Each
  step says so for its own board, and the type-level documentation of each board repeats it. The
  rule was never triggered, so nothing was opened for it.

### What the brief asked for and what is here

| brief item | state |
| --- | --- |
| 1. MMC1 SNROM work-RAM disable, both ROMs passing, every other MMC1 board still passing | **done** (step 1) |
| 2. Q-117, UNROM 512 latch decoding, MR-30-4 tests rewritten with the page's sequences | **done** (step 2) |
| 3. mapper 69 + Sunsoft 5B audio | **done** (steps 3 and 4), with Q-118 for the level |
| 3. mapper 118 TxSROM, as a variant of the MMC3 board chosen at load | **done** (step 5) |
| 3. mapper 180 | **done** (step 6), as a wiring of the UxROM board chosen at load |
| 3. mapper 78, both wirings, with the iNES-1.0 heuristic | **done** (step 7) |
| 3. for every new board: registry entry, `SUPPORTED_MAPPERS`, load-compatible save state, the Holy Mapperel ROM passing, tests on a synthetic ROM | **done** for all four numbers |
| 4. MR-4-3, the MC-ACC, chosen at load, confined to the MMC3 board | **done** (step 8) |
| 5. the `$4020-$5FFF` rule | **not triggered**; checked per board |
| 6. suites re-run, before/after in `MF-T-test-roms.md`, the newly passing ROMs in the env-gated test | **done** (step 9) |

Nothing the brief asked for is missing. The one item handed on is the +4 % wall clock above, which
is a measurement and not an undelivered piece of work.
