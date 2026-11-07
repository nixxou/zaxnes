# Lot MF-VS-fix: the VS table corrects a wrong iNES 1.0 header

**Baseline and performance reference: `0aee4241`**, the end of MF-4e.

## The defect R0 found

Common iNES 1.0 dumps of *Vs. Castlevania* carry a header announcing **mapper 99 with 0 KiB of
CHR-ROM** (128 KiB PRG, byte 7 bit 0 set). The game is a **UxROM (mapper 2) board with CHR-RAM**,
and `rust/crates/zax-nes/src/vs/roms.rs` already knows it — row `0xFFBE_F374`, "Vs. Castlevania",
with the comment `// mapper 2, Vs. System`. **But the comment is all it is**: loading follows the
header, so the image comes up as mapper 99 with no CHR at all and the screen stays empty. Many VS
dumps carry mapper 99 by default in old header sets, so the defect is not one dump's.

## What this lot does, in one sentence

The 74-row table stops being a PPU/cabinet table with mapper numbers in its comments and becomes a
**board** table: each row carries the mapper, the submapper, the PRG-ROM, CHR-ROM and CHR-RAM sizes
and the mirroring the public NES 2.0 header database records for it, and at load a VS image whose
data CRC is in the table is loaded as **that** board when its header disagrees.

## Sources

The one source for the new columns is the one the table was built from and the brief names: the
public **NES 2.0 ROM header database** (`nes20db.xml`, `NewRisingSun`), build **2026-09-26**, 21 339
games of which exactly **74** carry `<console type="1">` — the same build and the same 74 rows MF-VS
step 1 generated the table from. Each row's new fields are its `<pcb mapper= submapper= mirroring=
/>`, `<prgrom size= />`, `<chrrom size= />` and `<chrram size= />`, keyed as before by the same
entry's `<rom crc32= />`, the CRC of the data without the header.

Before writing a line, the committed table was re-derived from that database and compared field by
field: **74 rows parsed, 0 differences** on the CRC, the PPU model, the hardware type, the wiring
*and the mapper number in each row's comment*. So the new columns are not a second opinion about the
table — they are the columns the comments were already quoting.

## The design, settled before step 1

1. **The row carries a board, not six loose fields.** `VsRom` gains one `board: VsBoardSpec`, whose
   six fields are the database's. The row's `// mapper 99, Vs. System` comment keeps its *category*
   half (`Vs. System`, `Bad Dumps`, `Homebrew`…), which is the part no field holds; the mapper half
   becomes data, which is the brief's "so the comments become data".
2. **The correction is a header rewrite between `RomImage::try_load` and `Cartridge::create`**, and
   nowhere else. That is the only point where the header has been parsed and nothing has been built
   from it yet. `Cartridge::create` then picks the board, the bus-conflict rule and the memory sizes
   from the corrected header exactly as it does for any image, so **no board, no registry arm and no
   mapper learns that a table exists**.
3. **Detection runs on the *original* header and the correction after it.** `VsSystem::detect` reads
   byte 7 bit 0, the NES 2.0 console type **and the mapper number** (99 or 151 make an image a VS
   image whatever the console byte says). A header whose only VS signal is mapper 99 would stop
   being a VS image the moment the table rewrote that number, so the order is forced: detect, then
   correct.
4. **Who wins.** An **iNES 1.0** header is a 1986 description of a board that the table knows
   better: every field of the row that disagrees wins. A **NES 2.0** header is trusted — it has
   fields for all of this and a dumper filled them — *unless* it contradicts the table on the
   mapper, in which case the mapper and the submapper are the table's and the load message says so.
   Nothing else of a NES 2.0 header is touched, because a header that got the mapper wrong has not
   thereby been shown wrong about a size (**Q-138** records the reading).
5. **The submapper has to become authoritative with the mapper.** Everything that reads
   `InesHeader::submapper` gates on `is_nes20`, which is the proxy for "this field is meaningful";
   after a table correction it is meaningful on an iNES 1.0 header too. So `InesHeader` gains a
   `board_from_vs_table` flag **behind the feature** and `registry::has_bus_conflicts` asks
   `header.has_submapper()` instead of `header.is_nes20` — a `const fn` that is `self.is_nes20` and
   nothing else in the default build. Without it the two rows the database gives mapper 2 submapper
   2, *Vs. Castlevania* and *Vs. Top Gun*, would carry a submapper no code could read.
6. **The feature gates all of it** (`board-vs-system`, which is in `parked-boards` since MF-VS step
   7). The default build must come out at `0aee4241`'s figures and byte-identical goldens, and a
   non-VS image is never looked up in this table at any build.

## Steps

* [x] **0/6** — the plan: the baseline read, the database re-derived and compared to the committed
  table (74 rows, 0 differences), the design above settled, this file. No code.
* [ ] **1/6** — the table carries the board: `VsBoardSpec`, the 74 rows regenerated from the
  database, the accessors, and a test that pins every row's board field by field.
* [ ] **2/6** — the override at load: `vs::corrected_board`, `RomImage::correct_vs_board`, the
  `load_rom` order, `has_submapper`, the load message.
* [ ] **3/6** — the tests: the brief's four cases, plus the regression itself as a synthetic image.
* [ ] **4/6** — the measurement: the default build at `0aee4241`'s figures (the bar and golden-core),
  the three `Cpu6502::step` inlinings 0 / 0 / 0, `zone_offset` not dearer.
* [ ] **5/6** — the documents: Q-138, `MF-VS.md`'s "For R0" paragraph answered, the journal.
* [ ] **6/6** — the check: fmt; clippy `-D warnings` in both configurations; the suites in both; the
  ignored suite with the two ROM variables; the campaign, 331 rows identical to MF-4e's.

## What each step found

### Step 0 — the plan

Read: `docs/progress/MF-VS.md` whole, `rust/crates/zax-nes/src/vs/mod.rs` and `vs/roms.rs`,
`rom/ines_header.rs`, `rom/rom_image.rs`, `rom/rom_load_result.rs`, `nes_core.rs`'s `load_rom`,
`cartridges/registry.rs` (`board_memory_sizes`, `has_bus_conflicts`, `create_board`) and
`cartridges/mod.rs`'s `Cartridge::create`.

**The database the table was built from is still the same build.** Re-downloaded as
`nes20db.xml` dated **2026-09-26** — MF-VS step 1's build — with 21 339 games and exactly 74 of
`<console type="1">`, and the checker reports **0 differences** against the committed table on every
field it already had, the mapper in each comment included. The one cosmetic difference is in the
*titles*: this build writes them in their own scripts (`Vs. 麻雀`, `忍者じゃじゃ丸くん`,
`Super Xevious꞉ Gampの謎`) where MF-VS's generated table romanised them. **The titles are not
touched by this lot**: they are what the load note prints and what a test pins, and romanised is
what the committed file has.

**The defect, confirmed in the data rather than inferred.** The row the brief names is
`FFBEF374` — `mapper="2" submapper="2" mirroring="4"`, `prgrom 131072`, **`chrrom 0`**, `chrram
8192`. A circulating iNES 1.0 header for it says mapper 99 and CHR 0, and REQ-CORE-012's own rule
then gives that image 32 KiB of CHR-RAM and the mapper-99 board, which fetches its patterns from a
CHR-ROM that is not there. Two rows are in that position and not one: **`8C0C2DF5` *Vs. Top Gun***
is the same board, mapper 2 submapper 2 with 8 KiB of CHR-RAM, and the same header mistake makes it
the same empty screen. Every one of the 74 rows is `mirroring="4"`, which is the four-screen VRAM
every VS board has and which step 1 stores as data all the same.

**Three things the plan deliberately does not do.** The row's `prgram`/`prgnvram` (2 KiB on most
rows, which is the VS board's work RAM) is **not** carried: `registry::board_memory_sizes` already
answers that from the mapper number, which is where every other board's RAM size comes from, and the
brief's list of columns does not include it. Byte 6 is **not** rewritten when the mirroring is —
only mapper 30's board reads that byte's mirroring bits as a pair (MR-30-1) and no VS row is mapper
30. And the **region** is not a column: `rom::region_exception` is the table for that and it is keyed
by the same CRC.

Free space on G: at the start of the lot: **17 GB**, against the brief's floor of 5 GB.
