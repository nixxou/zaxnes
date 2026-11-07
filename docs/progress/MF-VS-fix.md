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
* [x] **1/6** — the table carries the board: `VsBoardSpec`, the 74 rows regenerated from the
  database, the accessors, and a test that pins every row's board field by field.
* [x] **2/6** — the override at load: `vs::corrected_board`, `RomImage::correct_vs_board`, the
  `load_rom` order, `has_submapper`, the load message.
* [x] **3/6** — the tests: the brief's four cases, plus the regression itself as a synthetic image.
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

### Step 1 — the board is a column

`VsRom` gains one field, `board: VsBoardSpec`, and `VsBoardSpec` is six: the mapper, the
submapper, the PRG-ROM, CHR-ROM and CHR-RAM sizes and the mirroring, each one attribute of the row's
database entry and each with its own accessor. The 74 rows were **regenerated**, not edited: a script
matched every `VsRom::new` literal in the committed file and inserted the `VsBoardSpec::new` call
built from the database entry of that row's CRC, so no title, PPU model, hardware type or wiring was
retyped and none changed. The trailing comment lost its `mapper NN,` half — that half is now the
board's first field — and kept the database's own category (`Vs. System`, `Bad Dumps`, `Homebrew`,
`Bootleg Hacks`, `Compatibility Hacks`), which no column holds.

**The proof the columns are the database's**, re-run on the committed file after `cargo fmt`:
`scratchpad/check_boards.py` parses all **74** rows out of `vs/roms.rs` and compares **nine**
columns each — CRC, PPU model, hardware type, wiring, mapper, submapper, PRG-ROM, CHR-ROM, CHR-RAM
and mirroring — against the rows it re-derives from `nes20db.xml`. **0 differences.**

Three things the data says that the step did not assume:

* **Two rows, not one, are the defect.** `FFBEF374` *Vs. Castlevania* and `8C0C2DF5`
  *Vs. Top Gun* are both `mapper="2" submapper="2"`, 128 KiB of PRG, **no CHR-ROM** and 8 KiB of
  CHR-RAM. They are the only two rows of the 74 with any CHR-RAM, and they are exactly the two an
  iNES 1.0 header announcing mapper 99 and CHR 0 breaks.
* **Only three rows have a submapper at all**: those two (`2`/`2`, which nesdev's "NES 2.0
  submappers" makes "Bus conflicts present") and `21A653C7` *Super Sky Kid* (`206`/`1`). The 206
  submapper reaches nothing in this core — `Namcot108` does not read one — so the submapper's one
  live consequence is the bus-conflict rule of the two UxROM boards, which is why step 2 has to make
  the field authoritative rather than merely present.
* **All 74 rows are `mirroring="4"`.** The four-screen VRAM was already an assumption of the lot
  (`VsBoard` has no mirroring register, and the VRC1 ignores its own bit on a VS board); it is now a
  column, and a test asserts the invariant over the whole table rather than trusting the sentence.

Two tests: `the_rows_carry_the_board_the_database_records` writes out six rows field by field — the
two with CHR-RAM, the two 40 KiB PRG images of nesdev's "INES Mapper 099", the one row with a
non-bus-conflict submapper and the mapper-75 VRC1 — so that a change to the generated table shows up
in a diff; and `every_board_of_the_table_is_one_this_build_can_run` holds the four invariants over
all 74 rows: the mapper is in `NesCore::SUPPORTED_MAPPERS`, the mirroring is four-screen, the PRG-ROM
is a non-zero multiple of 8 KiB (40 KiB and 24 KiB rows exist, so 16 KiB would be the wrong unit),
and `chr_rom_size == 0` **if and only if** `chr_ram_size != 0`.

Green: `zax-nes` **47 passed, 0 failed** on `vs_system_tests` with `--features parked-boards` (45 at
the baseline plus this step's 2); fmt clean; clippy `-D warnings --all-targets` clean on `zax-nes`
by default and with `--features parked-boards`. One lint needed a decision: `similar_names` rejects
`chr_rom_size` beside `chr_ram_size`, which are `InesHeader`'s own two names for the same two
fields, so the `allow` is on `VsBoardSpec::new` with the reason written there.

### Step 2 — the override, in four pieces and one order

**`vs::corrected_board(header, data_crc, image_len) -> Option<(InesHeader, String)>`** is the
decision, and it is `pub(crate)`: it refuses an image `VsSystem::is_vs_image` does not hold for, it
refuses a CRC no row matches, and it refuses a header that already agrees with the row. What it
returns is a whole corrected header plus the sentence the load message carries. The two readings the
brief states are the two branches:

* **iNES 1.0** — the mapper, the submapper, the PRG-ROM and CHR-ROM sizes, the CHR-RAM size and the
  mirroring are the row's wherever they disagree.
* **NES 2.0** — returns `None` unless `header.mapper != row.board().mapper()`, and then changes the
  mapper and the submapper and nothing else.

**`RomImage::correct_vs_board(data_crc)`** applies it: the header is replaced and the two bank
vectors are **re-split from `raw`**, because a header can be right about a 128 KiB total and wrong
about where it divides. It is the only writer of the new header, `#[cfg]`-gated, and it is called
from exactly one place.

**`load_rom`'s order is now explicit, and it had to change.** The old body was
`RomImage::try_load(loaded, name).and_then(Cartridge::create)`, one chained expression — and the
correction has to sit *between* those two calls, which is the only moment the header exists and
nothing has been built from it. So the chain became two `match`es with the same two error arms, with
the data CRC computed above them (it only ever needed `image`) and the VS block between them. The
**detection** moved up with it and now reads `rom.header()` *before* the correction, which is forced
and not cosmetic: `VsSystem::is_vs_image` treats mapper 99 and mapper 151 as VS signals in their own
right, so an image whose console byte says nothing would stop being a VS image the instant the table
rewrote that number. Detect, then correct.

**`InesHeader::has_submapper()`**, and why the flag exists. Every submapper-dependent rule in this
core is written `if header.is_nes20 { header.submapper } else { 0 }`, because an iNES 1.0 header has
no such field. After a correction it has one, from a source that does. So `InesHeader` gains
`board_from_vs_table: bool` **behind the feature**, `has_submapper()` is
`is_nes20 || board_from_vs_table` with the feature and `is_nes20` without it, and
`registry::has_bus_conflicts` is the one call site that asks it — the one rule the table's submapper
reaches, since the database gives *Vs. Castlevania* and *Vs. Top Gun* mapper 2 **submapper 2**, "Bus
conflicts present". The other two submapper call sites are deliberately untouched: mapper 34's two
boards and the MMC6 are not VS numbers, so asking there would be dead code.

**One refactor the lint forced, and it is an improvement.** The two added lines pushed
`InesHeader::try_parse` to 101 of clippy's 100 allowed, so the CHR-RAM block came out as a free
`chr_ram_size(is_nes20, byte11, chr_rom_size)` beside the `nes20_prg_ram_size` and `nes20_rom_size`
that were already there. Same arithmetic, REQ-CORE-012's default and its floor in one place.

Green: `zax-nes` + `zax-harness` **926 passed, 0 failed, 8 ignored** by default — *exactly* MF-4e's
926 — and **980 / 0 / 8** with `parked-boards` (978 at MF-4e plus step 1's two). fmt clean; clippy
`-D warnings --all-targets` clean on `zax-nes` by default, with `--features parked-boards` and with
`--no-default-features`.

### Step 3 — the tests, and the fixture that made them possible

**The problem the step had to solve first.** The table is keyed by the CRC-32 of a real dump's PRG
and CHR bytes, and no commercial ROM may appear in a test — so a *synthetic* image has to be given a
CRC that is **in the table**. Searching for one is 2³² work. But CRC-32 is affine over its 32-bit
register, so it is arithmetic instead: `force_crc32(data, at, wanted)` sets four bytes of `data` at
`at` so that the whole slice hashes to `wanted`.

> Walk the register **backwards** from `wanted` through every byte after the four — each backward
> step is determined, because the register's top byte names the one table entry that was used
> (the 256 entries of the `$EDB88320` table have 256 distinct top bytes, which the helper's
> `expect` states) — and that gives the state the four bytes must produce. Four more backward steps
> give the four table **indices** they must use: the low byte of each intermediate state is unknown,
> but it never reaches the next step's top byte, so the indices fall out all the same. Then walk
> **forwards** from the state before `at`: byte *k* is `index_k ^ (state_k & 0xFF)`.

Every caller asserts `data_crc(image) == wanted` afterwards, so an error in the helper fails loudly
instead of quietly weakening a test. The algorithm was checked against `zlib.crc32` at three offsets
before it was written in Rust.

**One bug worth recording.** The first version forced the CRC over the whole image, header included
— and `data_crc` hashes the data *without* the 16-byte header, so every fixture missed its row.
The helper's doc now says what the caller must pass (`image[16..]`), because the two slices are easy
to confuse and the failure is a silent "the table does not know this dump".

The six tests, the brief's four among them:

1. **`a_mapper_99_chr_0_header_loads_as_the_table_says`** — the defect itself. An iNES 1.0 image
   with 128 KiB of PRG, byte 5 = 0 and mapper 99 in bytes 6/7, hashing to `0xFFBEF374`. Before the
   load the header says mapper 99, no CHR-ROM and REQ-CORE-012's 32 KiB of CHR-RAM; after it the
   cartridge is **mapper 2**, the header is submapper 2 with 128 KiB of PRG, no CHR-ROM, **8 KiB**
   of CHR-RAM and four-screen, `has_submapper()` is true, and a pattern byte written at `$0000` is
   read back — which the mapper-99 board could not do at all. The VS. System is still the row's
   (`VsSource::RomTable`, RP2C04-0002, "Vs. Castlevania"), which is the proof detection ran *before*
   the correction, and the note names the correction field by field.
2. **`the_tables_submapper_reaches_the_bus_conflict_rule`** — the submapper's one live consequence,
   proved through the conflict and not through the field: `$A000` holds `$11`, a write of `$03`
   there latches `$03 & $11 = $01`, and `$8000` comes up bank 1. Without
   `InesHeader::has_submapper` the write would latch `$03` and the bank would be 3.
3. **`a_correct_header_is_not_altered`** — *Battle City*'s row under an iNES 1.0 header that already
   says mapper 99, 32 KiB of PRG, 16 KiB of CHR and four-screen: the header after the load is
   `assert_eq!`-equal to the header before it, `has_submapper()` is still false, and the note
   carries the title but no correction.
4. **`an_unknown_vs_crc_keeps_its_header`** — one byte of the data flipped, so the CRC is no row's:
   the header stands and the VS layer falls back to `VsSource::Default`, the "unknown VS image …
   plain NES palette" rule.
5. **`a_non_vs_image_with_a_colliding_crc_is_not_touched`** — the same data hashing to
   *Vs. Castlevania*'s CRC exactly, with byte 7 bit 0 **clear** and mapper 2 (no VS number). The
   header is untouched, there is no VS. System, there is no note, the image keeps its 32 KiB of
   CHR-RAM, and the row's submapper 2 never reaches it: a write of `$03` at `$A000` selects bank 3
   with no conflict, which is MR-1b's rule for a submapper-0 dump of mapper 2.
6. **`a_nes20_header_is_trusted_except_on_the_mapper`** — *Battle City*'s data under a NES 2.0
   header twice. Saying mapper 99 with 8 KiB of CHR-RAM and **vertical** mirroring, all three of
   which the table disagrees with: all three stand. Saying **mapper 2**: the mapper becomes the
   table's 99, the header's CHR-RAM stays its own 8 KiB, and the note is "the NES 2.0 header says
   mapper 2, the VS table says mapper 99 for Battle City and the table wins".

Green: `zax-nes` + `zax-harness` **926 / 0 / 8** by default — still exactly MF-4e's — and
**986 / 0 / 8** with `parked-boards` (980 after step 2 plus this step's 6). fmt clean, clippy
`-D warnings --all-targets` clean in both configurations.
