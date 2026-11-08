# MF-UNL-9 — Cheapocabra (GTROM), mapper 111

Lot **MF-UNL-9** of `docs/review/UNL-census.md` §7.2, the ninth of the twelve MF-UNL lots:
**one number, one board, one Cargo feature, 7 original titles.**

Base commit: **`7972e5f3`** (`MF-UNL-8 (8/8)`), the branch tip when the lot opened.
Branch: `mapper-review`. Worktree: the `wt-mappers` worktree.

The seven titles the census lists, all commercially released homebrew: *Anguna: Scourge of the
Goblin King*, *Candelabra: Estoscerro*, *The Incident*, *NEScape*, *Project Blue*, *Spook-o-tron*,
*Storied Sword*.

## Step 0 — the inventory, which is what this lot's instruction asks for first

**Mapper 111 already exists.** `board-111` was built by **MF-6c step 9** (`Cheapocabra`,
`rust/crates/zax-nes/src/cartridges/mappers/cheapocabra.rs`), and the instruction asks that step 0
establish its *exact current state* from the Cargo.toml comments and the feature table rather than
from the lot brief. Done, and the brief is out of date on one point:

### The board is **not** parked

| source | what it says |
| --- | --- |
| `rust/crates/zax-nes/Cargo.toml` `default` | contains **`board-111`** (line 44) |
| `rust/crates/zax-nes/Cargo.toml` `parked-boards` | `parked-boards = ["board-vs-system"]` — **`board-111` is not in it** |
| `board-111`'s own comment | "**Parked by MF-6c step 9 and un-parked by MF-CART step 5**" |
| `docs/review/BOARD-FEATURES.md` line 151 | decision column **`on`**; "**un-parked by MF-CART step 5**" |

MF-6c step 9 measured the board at `mmc5` **+7 423 140 (+0.066 %)** and parked it. That instruction
was **not the board's**: it was one unit of `Cartridge::default_cpu_read`'s quantum, which MF-CART
step 4 deleted by inlining that function into `Cartridge::try_cpu_read`. Re-measured on the fixed
tree, MF-CART step 5 got `mmc5` **−12 564 290 (−0.111 %)** with a worst ROM of **+0.002 %**, and
put the feature back in `default`.

So the lot brief's "parked boards stay parked" clause **has no subject here**. The board is in
`default` and this lot's duty is the other half of the same rule: what it adds must stay inside the
+0.02 %-per-ROM tolerance, or the *new mechanism* goes behind its own feature (the `board-68` /
`board-68-chr-nametables` precedent) — not the board.

### What the existing board already does, against every step and test of §7.2's MF-UNL-9

| census item | state | where |
| --- | --- | --- |
| **step 1** — a `FlashRom` memory kind in `CartridgeMemory`: command sequence, **dirty-sector set**, **a save file that is the changed sectors** | **missing**, and it is the whole of this lot | — |
| step 2 — the control register's PRG bank | **done** | `Cheapocabra::update_banks`, `self.register & 0x0F` over sixteen 32 KiB banks |
| step 2 — bit 5 selecting between two 8 KiB CHR-RAM pages used as nametable RAM | **done** | `Cheapocabra::nametable_offset`, `nametable_page` → `SERVED_BY_CARTRIDGE` |
| step 2 — four-screen mirroring forced with no alternative | **done** | `Cheapocabra::reset` sets `NametableMirroring::FourScreen` unconditionally |
| step 3 — bits 6-7 (the LEDs) decoded and discarded, with the page's "currently universally unsupported" quoted | **partly**: they are decoded and discarded and the gap is argued at length, but the comment quotes the page's *other* sentence, not that one | `Cheapocabra`'s struct note |
| step 4 — §7.1's three common things | **done for the board as it stands** (it is in the four sweeps and the save-state sweep) | `mapper_tests.rs` |
| step 4 — the save-state round trip extended to cover a **flashed sector** | **missing**: MF-6c recorded flash persistence as a gap, so a flashed byte is lost by a state round trip today | — |
| test — unlock-then-program changes a ROM byte | **done** | `req_core_100_the_gtrom_programs_and_erases_its_own_prg_rom` |
| test — an **incomplete** sequence does not | **done** | same test, the wrong-unlock-value case |
| test — a sector erase sets a whole sector to `$FF` | **done**, with the next sector asserted untouched | same test |
| test — a flashed byte survives a save-state round trip **and appears in the save file** | **missing** | — |
| test — bit 5 switches the nametable between the two pages; a write to one is not visible in the other | **done** | `req_core_100_the_gtrom_serves_two_banks_of_four_distinct_nametables` |
| test — mirroring is four-screen **regardless of the header's flag** | **partly**: `Nm::FourScreen` is asserted, but only on an image whose header flag is the default | same test |
| test — the LED bits change nothing observable | **partly**: the PRG test asserts they disturb neither the PRG windows nor CHR; the nametable bank and the mirroring are not asserted | `req_core_100_the_gtrom_banks_32_kib_of_prg_from_either_register_window` |

So the lot is **the save path and four tests**, which is exactly what the census predicted when it
graded one number an L: "the save path … is the whole of this lot's difficulty". Nothing already
here is rewritten; `Cheapocabra`'s register decode, its bank maths, its nametable bodies and its
existing three tests are left as the base commit shipped them.

### The two gaps MF-6c recorded that this lot closes, and the two it does not

`Cheapocabra`'s struct note records four gaps. This lot closes the first and leaves three:

* **closed** — "**persistence is out of scope.** The flash is changed in memory and a reload loses
  it (Q-117, to be settled with REQ-CORE-095)."
* kept — the software-ID commands `$90`/`$F0`, which need a body in `Board::cpu_read_override`.
* kept — the two LEDs, for want of a host surface for an indicator on a cartridge.
* kept — the register's read side ("reading from the register effectively writes the value of open
  bus"), which needs the open-bus value inside `Mapper::cpu_read_override`.

## The design of step 1, which the census asks be settled before coding

> **This is the step to design before coding** — it is the first time this build persists something
> that the ROM window reads.

Four decisions, each with its reason:

1. **The flash does not touch any read path.** A programmed byte is written into `prg_rom` as the
   board already writes it, so every PRG read stays the plain window read it is today. The new
   object records *which sectors changed*, nothing more. That is what keeps the measurement of a
   board in `default` honest: there is no new test on a hot path.
2. **The dirty-sector set and the undo data are one thing.** A sector that has been written keeps a
   copy of its **pristine** contents, taken on the first write to it. The set of sectors with a copy
   *is* the dirty set, and the copy is what lets a save state load into a core whose flash was
   written differently (revert, then apply) without a second copy of the whole ROM.
3. **The save file is the changed sectors**, per the census: a small self-describing container of
   `(sector index, sector contents)` pairs, and not a RAM image. A game that has flashed one 4 KiB
   sector of a 512 KiB chip writes about 4 KiB, not 512.
4. **The save state carries the changed sectors too, behind a format-version check.** With no flash
   on the board the new code writes **zero bytes**, so every existing board's state block is byte
   for byte what it is today and the only difference is the version number in the header; a state
   written before this lot still loads, with the flash left unprogrammed, which is also what a
   fresh power-on gives.

## Steps

**The step list was renumbered from eight to six after step 0**, and the commit that opened the lot
is the one `MF-UNL-9 (0/8)` in the log. The reason is that the plan's steps 1-5 do not compile
apart: the module, the board on it, the savestate section and the save-file API are one object
seen from four sides — `Cheapocabra` cannot program a sector before `FlashRom` exists, `FlashRom`
has no caller before `Cheapocabra` uses it, and neither has a test before the API is there. Rather
than commit four trees that do not build, they are one step. Nothing was dropped: the table at the
end of step 1 below lists each of the five against what it produced.

- [x] **0/8** — the plan: this file, the inventory above and the base commit. (Numbered 0/8; the
  list below is the renumbered one.)
- [x] **1/6** — the flash, whole: `FlashRom` and `FlashCommandState` in a new module, the
  `CartridgeMemory` field and its five methods, the `board_memory_sizes` row, `Cheapocabra` on it,
  the savestate section behind the version check, the core's save-file API, and the four tests the
  census lists that were not here.
- [ ] **2/6** — the four sweeps, the feature configurations, `fmt` and clippy in the four
  configurations the instruction names.
- [ ] **3/6** — the callgrind campaign against the base commit, and the feature decision.
- [ ] **4/6** — golden-core against the base commit, and the full `--include-ignored` run with both
  ROM roots.
- [ ] **5/6** — `BOARD-FEATURES.md`, the census note and the journal.
- [ ] **6/6** — the final pass.

## Step 1 — the flash, whole

### What each of the plan's five steps became

| plan step | where it landed |
| --- | --- |
| 1 — a `FlashRom` memory kind: command sequence, dirty-sector set, save file of the changed sectors | `rust/crates/zax-nes/src/cartridges/flash.rs`, a new module: `FlashCommandState` (the chip's command machine), `FlashCommand` (what a finished sequence asks for) and `FlashRom` (the sectors). `CartridgeMemory::flash`, `flash_program`, `flash_erase_sector`, `export_flash_save`, `import_flash_save`, `save_flash_state`, `load_flash_state`. `BoardMemorySizes::flash_sector`, and mapper 111's row giving it `FlashRom::SST39SF040_SECTOR` |
| 2 — `Gtrom`: the PRG bank, bit 5's two nametable pages, forced four-screen | **already shipped by MF-6c step 9** and not rewritten; `Cheapocabra::step_flash` now delegates the decode and the two commands, and that is the only change to the board's behaviour-bearing code |
| 3 — bits 6-7 decoded and discarded with the page's "currently universally unsupported" quoted | the sentence is now in `Cheapocabra`'s struct note, in the page's own words ("The LED lights on the board are universally unsupported at this time"), and `req_core_100_the_gtrom_led_bits_change_nothing_observable` is the test |
| 4 — §7.1's three common things, with the round trip extended to a **flashed** sector | `req_core_090_the_gtrom_flashed_sectors_come_back_with_its_registers`; the sweeps are step 2 of this list |
| the save-file API | `NesCore::export_flash_save` / `NesCore::import_flash_save`, beside `export_battery_ram` / `import_battery_ram` |

### The four decisions of the plan, as built

1. **Nothing is on a read path.** A programmed byte goes into `prg_rom` where every PRG read
   already finds it. `FlashRom` is read when a state or a save file is written and never on a bus
   access, and the field is `Option<Box<FlashRom>>` — one null pointer on the 198 other numbers.
2. **The dirty set and the undo data are one `Vec`.** `FlashRom::original` holds the pristine
   contents of every sector a command has touched, taken on the first write to it, and the sectors
   with an entry *are* the dirty set. The undo half is not decoration: `load_state` has to load a
   state into a core whose flash was written *differently*, which is revert-then-apply, and the
   last assertion of the round-trip test is exactly that case.
3. **The save file is the changed sectors.** `"ZXFL"`, a version, the sector size, then
   `(index, contents)` per changed sector. Three flashed sectors of a 512 KiB chip measure 12 KiB
   and a header, which the test asserts as a bound rather than hoping for.
4. **A sector is the chip's sector**, 4 KiB, and it is the unit of the erase command, of the dirty
   set and of the file at once.

### The savestate format, and the one place the bump could have broken a file

The flash goes at the **end of the cartridge's section**, after the board's own registers, and
`CartridgeMemory::save_flash_state` writes **zero bytes** when the board has no flash — so every
other board's block is byte for byte what it was, and the only difference in their files is the
version number in the header. `Cheapocabra`'s own block is unchanged too: its three fields are
still a `u8`, a `u8` and a `bool` in that order, which is why the command machine stayed on the
board (`FlashCommandState`'s own note) instead of moving into `CartridgeMemory` with the sectors.

`STATE_VERSION` went **8 → 10** and `WIDE_STATE_VERSION` **9 → 11**, with 8 and 9 kept as
`STATE_VERSION_8` and `WIDE_STATE_VERSION_9` and still read. Two consequences had to be handled
rather than inherited:

* **`state_version_has_mmc5_audio` was `version >= STATE_VERSION`**, which was correct only while
  that constant *was* the threshold. Left alone it would have stopped reading the MMC5's sound
  registers out of every version-8 and version-9 block the moment the constant moved. It now names
  `STATE_VERSION_8`, the version the registers appeared in.
* **`state_uses_widescreen` matched the version numbers literally**, so after the bump a state
  written by the build before this lot would have answered `None` and the host would have stopped
  recognising it. The two arms for 8 and 9 are added. (That function still has no arm for
  `WIDE_STATE_VERSION_7`, which `is_wide_state_version` does accept — a pre-existing asymmetry,
  recorded here and deliberately not touched by this lot.)

### The four tests the census asked for and the one it did not

| test | the census item |
| --- | --- |
| `req_core_095_the_gtrom_flash_save_is_the_changed_sectors` | "a flashed byte … appears in the save file", and the file being the changed sectors: the empty file of a chip nothing wrote, the 12 KiB bound against a 512 KiB chip, both programmed bytes found inside the `$FF` of their erased sectors, a reload into a fresh machine, a reset keeping it, and three refusals that leave the chip as it was |
| `req_core_090_the_gtrom_flashed_sectors_come_back_with_its_registers` | "a flashed byte survives a save-state round-trip", plus the revert case |
| `req_core_100_the_gtrom_is_four_screen_whatever_the_header_says` | "Mirroring is four-screen regardless of the header's flag" — all four values of byte 6's two mirroring bits, six values of the register, and after a reset |
| `req_core_100_the_gtrom_led_bits_change_nothing_observable` | "The LED bits change nothing observable" — the PRG windows, the CHR bank, the nametable bank and the mirroring across all three LED combinations, and an LED write in the middle of a flash sequence not aborting it |
| `req_core_095_a_board_without_a_flash_chip_exposes_nothing` | not asked for: the counterpart of `req_core_095_a_cartridge_without_a_battery_exposes_nothing` for the second save medium |

**A finding worth recording, because the first draft of the save-file test was wrong on it.** The
chip's unlock writes are on chip addresses `$5555` and `$2AAA`, and
`chip_address = (bank << 15) | (addr & 0x7FFF)` can only produce those with **bank 0** — so the
unlock pair is reachable at CPU `$D555`/`$AAAA` and *nowhere else*. A game that flashes a sector of
another bank must unlock in bank 0 and move the register **before the last write of the sequence**,
which is a CPU-bus write the chip never sees. `gtrom_flashed` does exactly that for its third
sector, so the suite now covers the only route by which fifteen of the sixteen banks can be
written at all.

The three tests MF-6c step 9 wrote are untouched and still pass.

## Sources consulted

* `docs/review/UNL-census.md` §7.1 and §7.2's MF-UNL-9 section.
* nesdev wiki, "GTROM": the register layout `GRNC PPPP`, "The register is mapped to the CPU
  addresses $5000-$5FFF and $7000-$7FFF and writing a byte to any of these addresses all have the
  same result", "This means the mapper always uses 4-screen mirroring and can neither use nor
  benefit from a switch to control the nametable arrangement", "The LED lights on the board are
  universally unsupported at this time", "a Microchip SST39SF040 flash chip as the PRG ROM storage
  device", and "Both of these mappers lack PRG RAM for game saves but the configuration of both
  boards make it possible to erase and rewrite sections of the chip using NES software".
* `docs/progress/MF-6c.md` step 9, `docs/progress/MF-CART.md` step 5,
  `docs/review/BOARD-FEATURES.md` line 151, and `rust/crates/zax-nes/Cargo.toml`'s own comments.
