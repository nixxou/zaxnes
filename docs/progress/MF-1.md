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
- [ ] 7/10 — mapper 34: MR-34-1 and the work-RAM half of MR-34-2
- [ ] 8/10 — MR-32-1, MR-156-1, MR-16-1
- [ ] 9/10 — MR-13-1, MR-13-2, MR-15-1, MR-225-1, MR-4-9, MR-30-1, MR-30-2, MR-30-3, MR-30-5
- [ ] 10/10 — full suite, the `--ignored` list, the R0 table and the review annotations

## Out of scope (lot MF-2 or later), left untouched

MR-BUS-1, MR-66-1, MR-64-3, MR-64-4, MR-5-2, MR-5-4, MR-30-4, MR-30-6, MR-4-3 (MC-ACC is a
new board), the three remaining unknowns (MR-64-1, MR-64-5, MR-18-1), and everything under
Q-110, Q-111 and Q-112 (mapper 15's banking modes, mappers 28 and 101, MMC5 audio, the reset
state): those wait for amendment AMD-061. The MR-1 fixes MR-1b withdrew (MR-64-1, MR-64-2,
MR-11-1, MR-4-2 and the rest of its "Left `our bug`" table) are **not** applied.

## Findings → fix → test → games (for R0)

The `Fixed in MF-1 (<hash>)` lines of `docs/review/MR-1-mappers.md` are appended in step 10,
because a step cannot write its own commit hash.

| finding | fix (file:line) | test | commercial games it changes |
| --- | --- | --- | --- |
| MR-41-1 | `multicart.rs:264` (`outer & 0x07`) | `mr_41_1_caltron_prg_field_is_three_bits` | Caltron 6-in-1 (games 5-6 become reachable) |
| MR-41-2 | `multicart.rs:267` (`outer & 0x20`) | `mr_41_2_caltron_mirroring_comes_from_address_bit_5` | Caltron 6-in-1 |
| MR-41-3 | `multicart.rs:318` (gate on `outer & 0x04`) | `mr_41_3_caltron_inner_chr_is_gated_on_the_prg_bank` | Caltron 6-in-1 |

| MR-4-1 | `mmc3.rs:63`, `:67` (the two masks swap) | `mr_4_1_mmc6_a001_read_and_write_bits_are_not_swapped` | StarTropics, Zoda's Revenge (their saves) |
| MR-4-7 | `mmc3.rs:294` ($A001 gated on $8000 bit 5) | `mr_4_7_mmc6_a001_is_held_at_zero_while_ram_is_disabled` | StarTropics, Zoda's Revenge |
| MR-4-6 | `registry.rs:53` + `mmc3.rs:55` (1 KiB) | `mr_4_6_mmc6_carries_one_kib_of_prg_ram`, `mr_4_6_a_plain_mmc3_keeps_its_eight_kib` | StarTropics, Zoda's Revenge (battery-RAM and save-state size only) |
| MR-4-8 | `mmc3.rs:236-243` (`Some(None)`) | `mr_4_8_mmc6_window_is_open_bus_when_no_half_is_readable` | none known |
| MR-64-7 | `rambo1.rs:46-48` (R15, R7, R6) | `mr_64_7_rambo1_prg_mode_swaps_r6_and_r15` | the five mapper-64 titles, whichever of them sets $8000 bit 6 (Klax, Rolling Thunder, Shinobi, Skull & Crossbones, Xybots) |
| MR-5-1 | `mmc5.rs:170-172` (three clauses) | `mr_5_1_mmc5_8x8_sprites_always_use_chr_set_a`, `req_core_100_mmc5_serves_background_fetches_from_the_second_set_with_tall_sprites` | any 8x8-sprite MMC5 title that writes $5128-$512B |
| MR-5-3 | `mmc5.rs:511-527` (arm, then increment) | `mr_5_3_mmc5_scanline_target_one_fires_on_the_second_line`, `req_core_100_the_mmc5_scanline_irq_fires_on_the_line_5203_names` | **Castlevania III, Just Breed** and every other MMC5 split: each one moves down by exactly one scanline |
| MR-5-5 | `mmc5.rs:410-416` (`None`) | `mr_5_5_mmc5_exram_reads_are_open_bus_in_the_nametable_modes` | none known |
| MR-1-1 | `mmc1.rs:178-190` (bit 7 above the filter) | `mr_1_1_mmc1_reset_bit_bypasses_the_consecutive_write_filter` | **Shinsenden**; any MMC1 game whose reset arrives as a read-modify-write |
| MR-1-3 | `mmc1.rs:36-43` + `:170-175` (shift/mask at load) | `mr_1_3_mmc1_sorom_selects_its_ram_chip_with_bit_3_alone`, `mr_1_3_an_sxrom_still_uses_both_s_bits` | **Genghis Khan, Nobunaga's Ambition** (SOROM saves) |
| MR-1-4 | `mmc1.rs:67-72` (`fixed_prg`) | `mr_1_4_mmc1_submapper_5_has_fixed_prg` | **Dr. Mario** and the other SEROM/SHROM dumps, only if headered NES 2.0 submapper 5 |
| MR-1-5 | `registry.rs:75-81` + `mod.rs:330-345` | `mr_1_5_only_the_second_sorom_ram_chip_is_battery_backed`, `mr_1_5_an_sxrom_keeps_its_whole_ram_battery_backed` | **Genghis Khan, Nobunaga's Ambition**: the battery file shrinks from 16 KiB to 8 |
| MR-21-1 | `vrc2_vrc4.rs:138-147` (the variant's lines) | `mr_21_1_vrc2a_uses_the_exact_register_wiring`, `mr_21_3_every_vrc_variant_decodes_its_own_address_lines` | **Twinbee, Gradius** (mapper 22) |
| MR-21-2 | `vrc2_vrc4.rs:229-241` (`is_vrc2`) | `mr_21_2_vrc2a_has_no_prg_swap_mode`, `req_core_100_vrc_switches_two_prg_banks_and_fixes_the_last_two` | **Twinbee, Gradius**; any VRC2a title writing $9002-$9003 |
| MR-21-3 | `vrc2_vrc4.rs:6-64` (the variant table) | `mr_21_3_every_vrc_variant_decodes_its_own_address_lines`, `mr_21_3_submapper_zero_keeps_the_ord_heuristic`, `mr_21_3_a_declared_vrc2b_has_no_irq_counter` | only submapper-tagged 21/23/25 dumps, and mapper 27 (now the exact VRC4\_27 wiring instead of the mapper-23 heuristic) |

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
