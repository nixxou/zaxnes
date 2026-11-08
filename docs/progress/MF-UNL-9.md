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

- [x] **0/8** — the plan: this file, the inventory above and the base commit.
- [ ] **1/8** — `FlashRom` in `CartridgeMemory`: the SST39SF040's command sequence, the
  dirty-sector set with its pristine copy, and the save-file encoding.
- [ ] **2/8** — `Cheapocabra` on it: the command machine delegates, the written sectors are
  recorded, and the board's emulation is unchanged to the instruction.
- [ ] **3/8** — the flash in the save state, behind the version check, with states written before
  this lot still loading.
- [ ] **4/8** — the core's save-file API, and what is handed to the host.
- [ ] **5/8** — the four tests the census lists that are not here: the save file, the save-state
  round trip of a flashed sector, four-screen regardless of the header's flag, and the LED bits.
- [ ] **6/8** — the four sweeps, the feature configurations, `fmt` and clippy.
- [ ] **7/8** — the callgrind campaign against the base commit, and the feature decision.
- [ ] **8/8** — golden-core, the full `--include-ignored` run with both ROM roots,
  `BOARD-FEATURES.md`, the census note, the journal and the final pass.

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
