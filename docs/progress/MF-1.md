# Lot MF-1 — the zero-cost mapper fixes of the MR-1/MR-1b review

Role R3 (clean-side implementer), worktree `wt-mappers`, branch `mapper-review`.

Source of truth for the findings: `docs/review/MR-1-mappers.md`, section
**"## MR-1b — the authoritative findings table"** (the MR-1 sections before it are superseded
wherever MR-1b disagrees). The failing tests live in
`rust/crates/zax-nes/tests/mr1_review_tests.rs`.

Build environment used for every cargo command (the C: drive is nearly full):

    TMP=<worktree>/rust/target-mf/tmp  TEMP=<worktree>/rust/target-mf/tmp
    CARGO_TARGET_DIR=<worktree>/rust/target-mf

`zax-app` cannot build in this worktree (its demo ROM is git-ignored), so clippy is scoped to
`-p zax-nes`.

## Steps

- [x] 1/10 — Caltron 6-in-1: MR-41-1, MR-41-2, MR-41-3
- [x] 2/10 — MMC6: MR-4-1, MR-4-7, MR-4-6, MR-4-8
- [x] 3/10 — RAMBO-1 PRG mode: MR-64-7
- [x] 4/10 — MMC5: MR-5-1, MR-5-3, MR-5-5
- [x] 5/10 — MMC1: MR-1-1, MR-1-3, MR-1-4, MR-1-5
- [x] 6/10 — VRC2/VRC4: MR-21-1, MR-21-2, MR-21-3
- [x] 7/10 — mapper 34: MR-34-1 and the work-RAM half of MR-34-2
- [x] 8/10 — MR-32-1, MR-156-1, MR-16-1
- [x] 9/10 — MR-13-1, MR-13-2, MR-15-1, MR-225-1, MR-4-9, MR-30-1, MR-30-2, MR-30-3, MR-30-5
- [x] 10/10 — full suite, the `--ignored` list, the R0 table and the review annotations

## Out of scope (lot MF-2 or later), left untouched

MR-BUS-1, MR-66-1, MR-64-3, MR-64-4, MR-5-2, MR-5-4, MR-30-4, MR-30-6, MR-4-3 (MC-ACC is a
new board), the three remaining unknowns (MR-64-1, MR-64-5, MR-18-1), and everything under
Q-110, Q-111 and Q-112 (mapper 15's banking modes, mappers 28 and 101, MMC5 audio, the reset
state): those wait for amendment AMD-061. The MR-1 fixes MR-1b withdrew (MR-64-1, MR-64-2,
MR-11-1, MR-4-2 and the rest of its "Left `our bug`" table) are **not** applied.

## Findings → fix → test → games (for R0)

The `Fixed in MF-1 (<hash>)` lines of `docs/review/MR-1-mappers.md` are appended in step 10,
because a step cannot write its own commit hash: 32 of them, one per fixed finding.

| finding | fix (file:line) | test | commercial games it changes |
| --- | --- | --- | --- |
| MR-41-1 | `multicart.rs:329` (`outer & 0x07`) | `mr_41_1_caltron_prg_field_is_three_bits` | Caltron 6-in-1: games 5 and 6 become reachable at all |
| MR-41-2 | `multicart.rs:332` (`outer & 0x20`) | `mr_41_2_caltron_mirroring_comes_from_address_bit_5` | Caltron 6-in-1 |
| MR-41-3 | `multicart.rs:383` (gate on `outer & 0x04`) | `mr_41_3_caltron_inner_chr_is_gated_on_the_prg_bank` | Caltron 6-in-1 |
| MR-4-1 | `mmc3.rs:80`, `:84` (the two masks swap) | `mr_4_1_mmc6_a001_read_and_write_bits_are_not_swapped` | StarTropics, Zoda's Revenge (their saves) |
| MR-4-7 | `mmc3.rs:303` (the $A001 arm gated on $8000 bit 5) | `mr_4_7_mmc6_a001_is_held_at_zero_while_ram_is_disabled` | StarTropics, Zoda's Revenge |
| MR-4-6 | `registry.rs:74` + `mmc3.rs:70` (1 KiB) | `mr_4_6_mmc6_carries_one_kib_of_prg_ram`, `mr_4_6_a_plain_mmc3_keeps_its_eight_kib` | StarTropics, Zoda's Revenge: the battery-RAM and save-state size only, no byte a game reads |
| MR-4-8 | `mmc3.rs:236-244` (`Some(None)`) | `mr_4_8_mmc6_window_is_open_bus_when_no_half_is_readable` | none known |
| MR-4-9 (submapper 2 only) | `mmc3.rs:38-42` | `mr_4_9_mmc3_submapper_2_has_hard_wired_mirroring` | submapper-2 mapper-4 dumps only. Submapper 4 is Q-114 |
| MR-64-7 | `rambo1.rs:44-56` (RF, R7, R6) | `mr_64_7_rambo1_prg_mode_swaps_r6_and_r15`, `req_core_100_rambo1_maps_three_prg_banks_and_a_1_kib_chr_mode` | the five Tengen mapper-64 titles, each one that sets $8000 bit 6 |
| MR-5-1 | `mmc5.rs:177-179` (the three clauses) | `mr_5_1_mmc5_8x8_sprites_always_use_chr_set_a`, `req_core_100_mmc5_serves_background_fetches_from_the_second_set_with_tall_sprites` | any 8x8-sprite MMC5 title that writes $5128-$512B |
| MR-5-3 | `mmc5.rs:515-532` (arm, then increment) | `mr_5_3_mmc5_scanline_target_one_fires_on_the_second_line`, `req_core_100_the_mmc5_scanline_irq_fires_on_the_line_5203_names` | **Castlevania III, Just Breed** and every other MMC5 split: each one moves down by exactly one scanline |
| MR-5-5 | `mmc5.rs:413-422` (`None`) | `mr_5_5_mmc5_exram_reads_are_open_bus_in_the_nametable_modes` | none known |
| MR-1-1 | `mmc1.rs:170-186` (bit 7 above the filter) | `mr_1_1_mmc1_reset_bit_bypasses_the_consecutive_write_filter`, `req_core_100_mmc1_ignores_a_write_on_the_cycle_after_another` | **Shinsenden**; any MMC1 game whose reset arrives as a read-modify-write |
| MR-1-3 | `mmc1.rs:33-45` + `:155-161` (the shift and mask at load) | `mr_1_3_mmc1_sorom_selects_its_ram_chip_with_bit_3_alone`, `mr_1_3_an_sxrom_still_uses_both_s_bits` | **Genghis Khan, Nobunaga's Ambition** (SOROM saves) |
| MR-1-4 | `mmc1.rs:54` + `:68-73` (`fixed_prg`) | `mr_1_4_mmc1_submapper_5_has_fixed_prg` | **Dr. Mario** and the other SEROM/SHROM dumps, only if headered NES 2.0 submapper 5 |
| MR-1-5 | `registry.rs:106-112` + `mod.rs:49`, `:333-348` | `mr_1_5_only_the_second_sorom_ram_chip_is_battery_backed`, `mr_1_5_an_sxrom_keeps_its_whole_ram_battery_backed` | **Genghis Khan, Nobunaga's Ambition**: the battery file shrinks from 16 KiB to 8. See Q-113 |
| MR-21-1 | `vrc2_vrc4.rs:155-162` (the variant's own lines) | `mr_21_1_vrc2a_uses_the_exact_register_wiring`, `mr_21_3_every_vrc_variant_decodes_its_own_address_lines` | **Twinbee, Gradius** (mapper 22) |
| MR-21-2 | `vrc2_vrc4.rs:288-296` (`is_vrc2`) | `mr_21_2_vrc2a_has_no_prg_swap_mode`, `req_core_100_vrc_switches_two_prg_banks_and_fixes_the_last_two` | **Twinbee, Gradius**: any VRC2a title writing $9002-$9003 |
| MR-21-3 | `vrc2_vrc4.rs:11-69` (the variant table), `:100-129` | `mr_21_3_every_vrc_variant_decodes_its_own_address_lines`, `mr_21_3_submapper_zero_keeps_the_ord_heuristic`, `mr_21_3_a_declared_vrc2b_has_no_irq_counter` | submapper-tagged 21/23/25 dumps only; mapper 27 now takes VRC4_27's own A0/A1, the same pair the mapper-23 heuristic gave it, so no mapper-27 dump moves |
| MR-34-1 | `registry.rs:45-58` + `multicart.rs:31-52`, `:81-103` | `mr_34_1_bnrom_has_no_register_at_7ffd`, `mr_34_1_the_board_comes_from_the_submapper_then_from_the_chr_size` | BNROM titles that write the last three bytes of $7FFF; and any 8 KiB-CHR-ROM mapper-34 dump, which now resolves to BNROM rather than NINA-001 |
| MR-34-2 (work-RAM half only) | `registry.rs:64-69` (`prg_ram: Some(0)`) | `mr_34_1_bnrom_has_no_register_at_7ffd` | BNROM titles reading $6000-$7FFF |
| MR-32-1 | `iremg101.rs:30` + `:76-97` | `mr_32_1_irem_g101_submapper_1_is_fixed_one_screen` | **Major League**, NES 2.0 submapper-1 dumps only |
| MR-156-1 | `daou306.rs:74-81` | `mr_156_1_daou306_mirroring_is_a_two_bit_field` | the Daou Infosys DIS23C01 titles |
| MR-16-1 | `bandai_fcg.rs:32` + `:40-55`, `:142-152` | `mr_16_1_bandai_submapper_4_writes_the_counter_directly` | **Famicom Jump II** and the other FCG-1/2 dumps, NES 2.0 submapper 4 only |
| MR-13-1 | `registry.rs:82-87` + `discrete.rs:230` | `mr_13_1_cprom_has_16_kib_of_chr_ram` | **Videomation**: the CHR-RAM and the save-state blob shrink by 16 KiB, no pattern byte moves |
| MR-13-2 | `discrete.rs:263-266` | `mr_13_2_cprom_mirroring_is_wired_vertical` | **Videomation**, and only on a mis-headered dump |
| MR-15-1 | `multicart.rs:192-204` | `mr_15_1_mapper_15_write_protects_chr_ram_in_modes_0_and_3` | the mapper-15 multicarts, when a game writes CHR-RAM in mode 0 or 3 |
| MR-225-1 | `multicart.rs:215`, `:264-296` | `mr_225_1_multicart_225_has_four_nibbles_of_ram` | none known |
| MR-30-1 | `ines_header.rs:35-40` + `discrete.rs:403-455` | `mr_30_1_unrom512_byte6_09_is_four_screen`, `mr_30_1_unrom512_byte6_08_is_switchable_one_screen` | four-screen UNROM 512 homebrew (byte 6 = $09) |
| MR-30-2 | `registry.rs:88-93` (`prg_ram: Some(0)`) | `mr_30_2_unrom512_does_not_drive_6000_7fff` | UNROM 512 homebrew reading $6000-$7FFF |
| MR-30-3 | `discrete.rs:438-439`, `:468-473` | `mr_30_3_unrom512_submapper_3_switches_horizontal_and_vertical` | submapper-3 UNROM 512 homebrew |
| MR-30-5 | `discrete.rs:450` + `:498-507` | `mr_30_5_unrom512_submapper_4_does_not_bank_through_the_led_register` | submapper-4 UNROM 512 homebrew |

**Boards not in that table do not change at all.** The only shared files touched are four, each
because a fixed board needed it: `cartridges/registry.rs` (the load-time memory sizes, the
mapper-34 board resolution, the battery offset), `cartridges/mod.rs` (`CartridgeMemory::new`'s
parameter, the battery sub-slice, the two tolerant blob reads), `state.rs` (one new reader
method) and `rom/ines_header.rs` (one new `u8` field). None of the four is on a per-cycle,
per-CPU-access or per-PPU-fetch path: `board_memory_sizes`, `mapper_34_is_nina` and
`battery_offset` run once in `Cartridge::create`, `read_blob_prefix_into` runs on a savestate
load, and `byte6` is read once in `UnRom512::new`.

**What R0 should expect to move in the golden-core campaign.** Only the MMC5 titles, and only
because of MR-5-3: every split moves down by exactly one scanline, so **Castlevania III** and
**Just Breed** are the entries to re-bless. Everything else in the table is either a board with
no golden-core entry (the multicarts, the homebrew, the Korean and Daou titles), a change gated
on an NES 2.0 submapper no entry declares, or a change to a memory size rather than to a byte
any game reads. The boards **not** in the table must not move at all.

### Step 1 notes

- The outer register now keeps six address bits instead of five
  (`multicart.rs:305`); the saved state is still the same two bytes, so a state written
  before this lot loads unchanged — only the meaning of the bits in `outer` moves, which is
  the fix itself.
- `req_core_100_caltron_takes_the_prg_bank_from_the_address_and_the_chr_from_the_data`
  (`mapper_tests.rs:701`) encoded the old layout in its address and had to be re-written to
  the `..MC CPPP` one; it asserts the same windows as before. No other test touches mapper 41.
- `SPEC-02-coeur-nes.md:414` lists mapper 41 with an empty behaviour column, so REQ-CORE-100's
  "comportement documenté sur nesdev" governs and no Q ticket is needed.

### Step 2 notes

- **The per-board RAM-size hook.** `registry::board_memory_sizes(mapper, header)` replaces
  `minimum_prg_ram_size`: it returns an *exact* PRG-RAM or CHR-RAM size for a board whose RAM
  the header cannot describe, and keeps the old floor for the MMC5. It is read once in
  `CartridgeMemory::new`. This is MR-1's shared-infrastructure item 2, and step 9's MR-13-1
  and MR-30-2 reuse it.
- **Save states.** The MMC6's PRG-RAM goes from 8 KiB to 1 KiB, so a state written before this
  lot stores a larger blob than the board now holds. `Cartridge::load_state` therefore reads
  its two memory blobs with the new `StateReader::read_blob_prefix_into` (`state.rs:447`)
  instead of `read_blob_into`: the prefix that fits is kept and the excess dropped, in both
  directions. Pinned by `mf_1_a_state_written_with_a_larger_board_ram_still_loads` and its
  twin. No save-state version bump: no board gained a register.
- **The MMC6 power-on $A001 is now 0**, not the MMC3's 0x80 (`Mmc3::power_on_protect`). MR-4-5
  says the MMC6's value *is* documented — $8000 powers on at 0, so the RAM is disabled, so the
  chip holds $A001 at $00 — and without this MR-4-7's gate would leave 0x80 standing as "high
  half readable" the moment a game enabled the RAM. The MMC3 proper keeps 0x80 and its
  rationale.
- `mmc6_offset` lost its `% memory.prg_ram().len()`: on a 1 KiB allocation the mask is the
  offset. That is one division *removed* from the MMC6 access path, as MR-4-6 predicted.
- MR-4-1 and MR-4-8 move no byte any other board reads: both functions are behind
  `self.is_mmc6`, which is a load-time field.

### Step 3 notes

- One index pair exchanged in `Rambo1::update_banks`; R7 stays at $A000 in both modes. Mesen
  cannot be the oracle here — MR-1b records that it carries the same old wiki table — so the
  authority is the corrected nesdev page plus the hardware tests of the "RAMBO-1 Mapper
  Investigation" thread that produced the correction.
- `req_core_100_rambo1_maps_three_prg_banks_and_a_1_kib_chr_mode` (`mapper_tests.rs:714`)
  asserted the rotation and now asserts the swap; its CHR half is untouched. MR-1b's withdrawn
  MR-64-2 (the 2 KiB CHR window) and MR-64-1 (the counter) are **not** applied, and the two
  tests that asserted them stay removed.
- No state, no new branch: `select & 0x40` was already the condition.

### Step 4 notes

- **MR-5-1** is Mesen's `chrA` expression (`MMC5.h:192`) read back as "use set B": set B only
  with 8x16 sprites, only on a background fetch, and only when either the scan is in a frame
  or set B was written last. The three nesdev rules fall out of it, and the clause that was
  missing — the last-written set deciding *only* outside rendering — is the third. It is
  branchless now where it used to be an `if`, so nothing is added to the per-fetch path.
  MR-1b's caveat stands and is not fixed here: the wiki narrows the last-written rule to $2007
  access, which our one `read_chr` path does not distinguish from a fetch. That is the same
  in-frame-detection question as MR-5-4, which is out of this lot.
- **MR-5-3 moves every MMC5 split down one scanline**, so R0 should expect the golden-core
  entries of the MMC5 titles to move — **Castlevania III and Just Breed above all** — and no
  other board's to. `req_core_100_the_mmc5_scanline_irq_fires_on_the_line_5203_names` now
  asserts line 60 for a target of 60, where it asserted 59.
- The counter is masked to 8 bits on the increment, which the old `scanline + 1` never needed:
  nesdev calls it "the internal 8-bit scanline counter", and 240 lines cannot reach the wrap,
  but the register is what it is.
- MR-5-2 (writes outside rendering) and MR-5-4 (the in-frame detection) are **not** touched:
  both are out of this lot and their tests stay ignored.

### Step 5 notes

- **MR-1-1** keeps `last_write_cycle` updated on a reset write, which is also what Mesen does
  (`MMC1.h:210`, the assignment sits outside the `if`): the reset is processed whatever the
  cycle, and the *next* write is still measured from it.
  `req_core_100_mmc1_ignores_a_write_on_the_cycle_after_another` (`mapper_tests.rs:106`) used
  0xFF for the write it expects to be dropped, which now carries the reset bit; it uses 0x7F
  instead and asserts the same windows.
- **MR-1-3 adds no runtime branch.** `ram_bank_shift`/`ram_bank_mask` are chosen in
  `Mmc1::new` from `memory.prg_ram().len()`, exactly as MR-1b asked, so `prg_ram_offset` still
  does one shift, one mask and one modulo on the $6000-$7FFF access path. The 8 KiB case takes
  mask 0, which is the old `bank = 0` without the `if`.
- **The fixture gained the PRG-RAM knob** MR-1b named as the blocker for MR-1-3 and MR-1-5:
  `MapperRom::prg_ram(volatile, non_volatile)` writes NES 2.0 byte 10. A byte 10 of zero reads
  back as the 8 KiB default, so the images that only asked for a submapper are unchanged.
- **MR-1-5 opened Q-113** (`spec/changes/Q-113.md`). The nesdev sentence names SOROM and SZROM
  only, both 16 KiB boards; MR-1b's prose extends it to the 32 KiB SXROM, which the source
  does not say and Mesen contradicts. Applying that extension would cut *Final Fantasy I·II*
  and *Best Play Pro Yakyuu Special* saves to a quarter, so the fix applies the quoted sentence
  and nothing more: 16 KiB board -> the second 8 KiB chip only, 32 KiB board -> the whole RAM.
  The ticket also asks whether SOROM's "second chip" is the bit-3-set bank (our reading, from
  the wiki) or bank 0 (Mesen's), and what to do with battery files already written.
- **For R0, a host-side consequence, not a save-state one:** on a SOROM the battery file goes
  from 16 KiB to 8 KiB, and `NesCore::import_battery_ram` copies a prefix, so an existing
  16 KiB file would load its volatile half into the retained one. Two titles are concerned;
  question 3 of Q-113 carries it.
- MR-1-2 is **not** applied: MR-1b withdrew it, and its test stays removed.

### Step 6 notes

- **The variant table is in the code now** (`VrcVariant`, with `resolve` from the mapper number
  and the NES 2.0 submapper, `index_lines` straight from the nesdev table, and `is_vrc2`).
  That is MR-1's infrastructure item 3 for this board, and it closes MR-21-1, MR-21-2 and
  MR-21-3 together.
- **`register_index` lost its `match` and gained nothing.** The two line pairs are load-time
  fields; a known variant has them equal, so the OR is a no-op and the expression is the same
  four shifts the heuristic used to cost, with no branch on the mapper number.
- **The heuristic survives exactly where the variant is unknown**: submapper 0 of mappers 21,
  23 and 25. Mapper 22 is always VRC2a and mapper 27 always the pirate board, so neither ever
  reaches it; mapper 27 used to take the mapper-23 heuristic and now takes VRC4\_27's own
  A0/A1, which happen to be the same pair — so no mapper-27 dump moves.
- **An unknown variant keeps the VRC4 features** (IRQ counter and PRG swap mode), as our code
  did before and as Mesen does for the IRQ. Mesen is inconsistent here: its submapper-0
  mapper-23 default is VRC2b, which takes the VRC2 arm for $9000-$9003 yet still gets the IRQ.
  Keeping VRC4 is the safe reading — most mapper-23 dumps are VRC4e or VRC4f — and a real
  VRC2b or VRC2c declares itself with submapper 3, which the new table honours.
- **VRC4f** (mapper 23 submapper 1) is a row Mesen does not have at all: it folds submapper 1
  into its VRC2b default and so denies a VRC4f its IRQ. The nesdev table gives it VRC2b's
  address lines with VRC4 capabilities, which is what `resolve` returns.
- `chr_high_mask` still keys on mapper 27 and the VRC2 mirroring stays a one-bit field: both
  are **AMD-021 R2-004** accepted divergences, not findings.
- No state added, so save states are unaffected: the new fields are derived from the ROM.

### Step 7 notes

- The two mapper-34 boards are now distinguished by a load-time field, from
  `registry::mapper_34_is_nina`: **the submapper first** ("Submapper 1 denotes the
  NINA-001/NINA-002 circuit board", "Submapper 2 denotes the I-IM/BNROM circuit board"), then
  the CHR-ROM size, "BNROM when the CHR-ROM size is 0-8 KiB". Both refinements MR-1b added to
  MR-1's proposal are in: the threshold is 8 KiB rather than "empty", and the submapper wins.
  An 8 KiB-CHR-ROM mapper-34 dump therefore changes board, which is the point of the fix.
- `update_banks` keys its CHR layout on the board rather than on `chr_rom().is_empty()`, so a
  BNROM with 8 KiB of CHR-ROM is unbanked as the board is ("fixed 8 KiB CHR-ROM/RAM").
- **The work-RAM half of MR-34-2 is done; its bus-conflict half is not** — that one extends
  MR-BUS-1 and belongs to lot MF-2, as the lot brief says. A BNROM now allocates no PRG-RAM, so
  `Cartridge::default_cpu_read` falls through to `BankWindows::read_prg`, which answers `None`
  below $8000: the window is open bus.
- **MR-1's `mr_34_1_bnrom_has_no_register_at_7ffd` had an assertion MR-34-2 contradicts.** It
  ended with "the write still reaches RAM" at $7FFD; the board has no RAM there at all, so that
  line now asserts open bus instead. MR-1b did not reconcile the two findings, and this lot
  fixes both.
- No state added. A BNROM save state written before this lot carries an 8 KiB PRG-RAM blob,
  which `read_blob_prefix_into` (step 2) drops into a zero-length buffer without complaining.

### Step 8 notes

- **MR-32-1 is one condition, not two.** The nesdev sentence makes the hard-wired one-screen
  mirroring and the disabled $9000 the same fact, so submapper 1 sets `SingleScreenA` at reset
  and then returns from the $9000 arm without touching either the mirroring or `swap_prg` —
  which leaves the board in PRG mode 0, the only mode the sentence allows.
- **MR-156-1** widens $C014 from one bit to two. Mesen reads bit 0 only and is wrong here as
  well, which MR-1b records; the authority is the register diagram.
- **MR-16-1** routes $x00B/$x00C through `write_irq_value`, which picks the counter on an
  FCG-1/2 and the latch on an LZ93D50, and makes $x00A's latch-to-counter copy conditional.
  An undeclared mapper-16 dump stays an LZ93D50, which is both the common board and what we
  did before, so no existing dump moves.
- All three flags are load-time; the three existing tests of these boards use submapper 0 and
  are untouched.

### Step 9 notes

- **`InesHeader` gained `byte6`** (one `u8` on a struct built once per load, which is exactly
  what MR-1b sanctioned). The generic iNES decode of `mirroring` is **unchanged** — changing it
  would move every board — and only UNROM 512 reads the two bits as the pair its own page
  documents. `UnRom512Mirroring` has the three cases: `Fixed` (vertical, horizontal, or the
  genuine four-screen of `%....1..1`), `OneScreen` (`%....1..0`, which is what any FourScreen
  header used to mean on this board) and `HorizontalVertical` (submapper 3).
- **MR-30-6 is still out of scope**, so a four-screen UNROM 512 now keeps four nametables but
  takes them from console VRAM rather than "the last 8 KiB of CHR-RAM". MR-30-1 is the
  prerequisite MR-1b said MR-30-6 needed, and it is now in place.
- **MR-30-5**: the LED register is `D~[BYGR bygr] A~[10.. .... .... ....]`, i.e. $8000-$BFFF,
  so on submapper 4 the bank register only answers $C000-$FFFF. Nothing in the engine lights,
  so the LED value is not stored and no state is added.
- **MR-15-1** is one `self.mode` test in `Mapper::ppu_write`. Q-110's neighbours — how `mode`
  itself is decoded, mode 1's fixed bank, the 6-bit `B` field, the missing XOR — are untouched,
  as the lot brief requires.
- **MR-225-1 adds state**, the only finding in the lot that does: four nibbles. They are
  written **last** in `save_registers`, and `load_registers` reads them only when the block
  still has bytes (`StateReader::remaining`), so a mapper-225 state written before this lot
  loads with the nibbles at their power-on zeros — the same pattern the MMC5's sound registers
  use for format versions 4 to 7. No version bump.
- **MR-4-9 is half-done, and Q-114 says why.** Submapper 2, "MMC3C with hard-wired mirroring",
  is in: it widens the existing `four_screen` flag. Submapper 4, "NEC MMC3 (Loading latch with
  0 disables IRQ)", is the rev A counter, which **REQ-CORE-100 pins to rev B** for this board
  and which REQ-CORE-114's `mmc3_test_2` test 6 then fails. That is a spec-versus-submapper
  question, filed as `spec/changes/Q-114.md`, which also carries MR-4-3's MC-ACC (submapper 3)
  since it raises the same priority question. Submapper 5 is a scrambling chip, outside
  REQ-CORE-100.
- **MR-13-1 and MR-30-2 reuse step 2's `board_memory_sizes` hook**; both shrink a memory, so
  both rely on step 2's `read_blob_prefix_into` to keep older save states loadable.

## Step 10 — the close of the lot

### The Q tickets opened

- **`spec/changes/Q-113.md`** — which half of the PRG-RAM the battery retains on the MMC1
  boards that carry two chips. The nesdev sentence MR-1-5 quotes names SOROM and SZROM, the two
  16 KiB boards, and says nothing about the 32 KiB SXROM; the text of MR-1-5 extends it to
  SXROM anyway, which Mesen contradicts and which would cut *Final Fantasy I·II* and *Best Play
  Pro Yakyuu Special* saves to a quarter. The fix applies the quoted sentence and nothing more.
  The ticket also asks whether SOROM's "second chip" is the bit-3-set bank (our reading, from
  the wiki) or bank 0 (Mesen's), and what to do with battery files already written. Referenced
  in `cartridges/registry.rs`, `battery_offset`.
- **`spec/changes/Q-114.md`** — mapper 4 submapper 4, "NEC MMC3 (Loading latch with 0 disables
  IRQ)", is the rev A counter, and **REQ-CORE-100 pins rev B** for this board while
  REQ-CORE-114's `mmc3_test_2` test 6 depends on rev B. That is the spec-versus-submapper
  question MR-4-9 identified; submapper 2 is implemented, submapper 4 is not. The ticket also
  carries MR-4-3's MC-ACC (submapper 3), which raises the same priority question.

No other finding of this lot contradicted the spec: `SPEC-02-coeur-nes.md:414` lists mapper 41
with an empty behaviour column, AMD-021 R2-004 already accepts the two VRC divergences the fix
keeps, and AMD-021 R2-002 is silent on MR-15-1 rather than contradicting it.

### The out-of-scope tests still failing

`cargo test --release -p zax-nes --no-fail-fast -- --ignored` leaves exactly four failures, all
of them findings this lot does not touch:

| test | finding | why it waits |
| --- | --- | --- |
| `mr_bus_1_color_dreams_write_is_anded_with_the_rom_byte` | MR-BUS-1 | lot MF-2: a shared bus-conflict path, `negligible` rather than `none` |
| `mr_bus_1_uxrom_submapper_2_write_is_anded_with_the_rom_byte` | MR-BUS-1 | same |
| `mr_64_3_rambo1_a12_filter_is_sixteen_m2_cycles` | MR-64-3 | lot MF-2: a per-board A12 threshold, `negligible` |
| `mr_5_2_mmc5_exram_write_outside_rendering_does_not_store_the_value` | MR-5-2 | lot MF-2: it needs the in-frame flag MR-5-4 governs, `negligible` |

The other seven `#[ignore]`d tests in the crate all pass under `--ignored`; they are the
long-running golden and timing ones, not failures.
`mr_bus_1_uxrom_without_a_submapper_has_no_conflict`, the one unignored guard MR-1b left
against MR-BUS-1 over-reach, still passes in the normal suite.

### The suite

Two files carry the header `// Portions adapted from Mesen 2 (c) Sour, GPL-3.0`: `mmc5.rs`,
whose `use_b` is Mesen's `chrA` expression read back (MR-5-1), and `vrc2_vrc4.rs`, whose
submapper-0 fallback is Mesen's OR'd heuristic (MR-21-3, and it was already ours before this
lot). A **Mesen 2** entry was added to `THIRD-PARTY-NOTICES.md` (GNU GPL v3, local repository
`third_party/mesen2`, commit `b9fa69d`), naming both sites. Every other fix in the lot follows
the NESdev pages REQ-CORE-100 designates, several of them *against* Mesen: MR-64-7, MR-156-1
and MR-5-3's convention are findings where MR-1b records Mesen as sharing our error or holding
an outdated table.

- `cargo fmt --check` — clean.
- `cargo clippy -p zax-nes --all-targets -- -D warnings` — clean.
- `cargo test --release -p zax-nes` — 40 test binaries, **559 passed, 0 failed**, 11 ignored
  (the four above plus seven long-running golden and timing tests, all of which pass under
  `--ignored`).
- `cargo check --workspace --exclude zax-app --all-targets` — clean, so the three shared files
  this lot touched break no other crate. `zax-app` cannot build in this worktree; its only
  contact with the changed API is `NesCore::export_battery_ram` / `import_battery_ram`, whose
  signatures are unchanged.

### Performance

Every fix in the lot is a load-time decision or arithmetic on a register write, as MR-1b
classed them. Three of them *remove* work rather than add it:

- `Mmc3::mmc6_offset` lost its `% memory.prg_ram().len()` — one division gone from the MMC6
  access path (MR-4-6).
- `Mmc1::prg_ram_offset` lost its `if len > PRG_WINDOW_SIZE` — the shift and the mask are now
  load-time fields, so the $6000-$7FFF path is one branch shorter (MR-1-3).
- `Vrc2Vrc4::register_index` lost its `match self.mapper_number` — the address lines are
  load-time fields and the expression is branchless (MR-21-1, MR-21-3).

And one is branchless where it used to branch: `Mmc5::chr_address`'s `use_b` is three bool
reads and two ANDs instead of an `if` (MR-5-1).

Nothing was added to the `Board` dispatch, to a `Mapper` hot method, or to any per-cycle,
per-CPU-access or per-PPU-fetch path. The one new `Mapper` override is `Mapper015::ppu_write`,
which replaces the trait default on that board alone and runs only when the CPU writes $2007 on
a mapper-15 cartridge; `Multicart225` gains `cpu_peek_override` and `intercept_cpu_write`, both
on a board whose games already went through `intercept_cpu_write`.

### Save states

No format version bump: no board gained a register that an older block would have to carry
positionally, and the one board that gained state writes it last.

- **`Multicart225`** gained four nibbles (MR-225-1), written last in `save_registers` and read
  only when `StateReader::remaining` says the block still has bytes. An older block loads with
  them at their power-on zeros.
- **Three boards' volatile RAM shrank** — the MMC6's PRG-RAM to 1 KiB (MR-4-6), CPROM's CHR-RAM
  to 16 KiB (MR-13-1), UNROM 512's and BNROM's work RAM to nothing (MR-30-2, MR-34-2) — so
  `Cartridge::load_state` reads its two memory blobs with the new
  `StateReader::read_blob_prefix_into` instead of `read_blob_into`: the prefix that fits is
  kept, in both directions. Pinned by `mf_1_a_state_written_with_a_larger_board_ram_still_loads`
  and `mf_1_a_state_written_with_a_smaller_board_ram_still_loads`.
- **Two boards' register meanings moved** without the blob changing size: `Caltron6In1::outer`
  keeps six address bits instead of five (MR-41-1/2), and `Mmc3::protect` has its MMC6 bit
  roles swapped (MR-4-1). Both load; what the bits mean is the fix.
- **One host-side file shrinks, and it is not a save state**: on a SOROM the battery file goes
  from 16 KiB to 8 (MR-1-5), and `NesCore::import_battery_ram` copies a prefix, so an existing
  16 KiB file would load its volatile half into the retained one. *Genghis Khan* and
  *Nobunaga's Ambition* are the two titles concerned; question 3 of Q-113 carries it.

### Tests added beyond the un-ignored ones

MR-1b recorded eight findings as "test missing" for want of a hook. Both hooks exist now — the
per-board memory sizes (`registry::board_memory_sizes`) and the fixture's PRG-RAM knob
(`MapperRom::prg_ram`) — so the lot wrote the tests instead of leaving the gap:
`mr_4_6_*` (2), `mr_4_8_*`, `mr_5_5_*`, `mr_1_3_*` (2), `mr_1_5_*` (2), `mr_13_2_*`,
`mr_15_1_*`, `mr_225_1_*`, `mr_4_9_*`, `mr_21_3_*` (3), `mr_30_1_unrom512_byte6_08_*`,
`mr_30_3_*`, `mr_30_5_*`, `mr_34_1_the_board_comes_from_*`, and the two `mf_1_a_state_*`
savestate guards.

### Six existing tests asserted behaviour a finding declares wrong

Each was re-written to the finding's behaviour, with the finding cited in the test:

| test | finding | what changed |
| --- | --- | --- |
| `req_core_100_caltron_takes_the_prg_bank_from_the_address_and_the_chr_from_the_data` | MR-41-1/2/3 | its address encoded the old five-bit layout; same windows asserted |
| `req_core_100_rambo1_maps_three_prg_banks_and_a_1_kib_chr_mode` | MR-64-7 | asserted the rotation, now asserts the swap |
| `req_core_100_the_mmc5_scanline_irq_fires_on_the_line_5203_names` | MR-5-3 | asserted line 59 for a target of 60, now line 60 |
| `req_core_100_mmc5_serves_background_fetches_from_the_second_set_with_tall_sprites` | MR-5-1 | its 8x8 half asserted "the set written last serves everything"; a third case pins the in-frame clause |
| `req_core_100_mmc1_ignores_a_write_on_the_cycle_after_another` | MR-1-1 | used 0xFF for the write it expects dropped, which carries the reset bit; uses 0x7F |
| `req_core_100_vrc_switches_two_prg_banks_and_fixes_the_last_two` | MR-21-2 | expected the swap mode on mapper 22, which is a VRC2 and has none |

And one test of the review file itself had an assertion a later finding contradicts:
`mr_34_1_bnrom_has_no_register_at_7ffd` ended with "the write still reaches RAM" at $7FFD,
where MR-34-2 says the BNROM has no work RAM at all. It asserts open bus instead. MR-1b did not
reconcile the two findings; this lot fixes both.

### What was deliberately not done

- **The four MR-1 fixes MR-1b withdrew** — MR-64-1, MR-64-2, MR-11-1, MR-4-2 — are not applied,
  and the tests that asserted them stay removed. Two of them would have broken a board that is
  right today.
- **The bus-conflict half of MR-34-2** (it extends MR-BUS-1), **MR-66-1**, **MR-64-4**,
  **MR-5-4**, **MR-30-4**, **MR-30-6** and **MR-4-3** (the MC-ACC is a new board) are lot MF-2
  or later.
- **Q-110's points** — mapper 15's banking modes and `mode` decode, mappers 28 and 101 — are
  untouched; MR-15-1's fix is the one condition the ticket does not cover.
- **Q-111** (MMC5 audio) and **Q-112** (the reset state) need no code and got none.
- The three remaining `unknown`s (**MR-64-1, MR-64-5, MR-18-1**) are left as MR-1b left them,
  all three leaning our way.
