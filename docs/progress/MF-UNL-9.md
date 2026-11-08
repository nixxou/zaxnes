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
- [x] **2/6** — the four sweeps, the feature configurations, `fmt` and clippy in the four
  configurations the instruction names.
- [x] **3/6** — the callgrind campaign against the base commit, and the feature decision.
- [x] **4/6** — golden-core against the base commit, and the full `--include-ignored` run with both
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

## Step 2 — the sweeps, the configurations, `fmt` and clippy

**This lot adds no Cargo feature.** The flash is a mechanism of `board-111`, and the owner's rule
puts a mechanism behind its own feature only "if it costs more than the board itself", which
step 3's measurement is what settles. So the instruction's "each new feature alone" is
`--no-default-features --features board-111`, and the four configurations are the four below.

### Where the flash had to be gated, and where it deliberately was not

`--no-default-features` turned up four dead-code errors under `-D warnings`, and the fix draws the
line the module's note already argued:

* **gated on `board-111`** — `FlashCommand`, `FlashCommandState` and its `impl`,
  `FlashRom::SST39SF040_SECTOR`, `FlashRom::sector_of`, `FlashRom::program`,
  `FlashRom::erase_sector`, `CartridgeMemory::flash_program`, `CartridgeMemory::flash_erase_sector`.
  All of them are about *issuing* a command, and mapper 111 is the one board that issues one.
* **not gated** — `FlashRom` itself, the `CartridgeMemory::flash` field, `flash()`,
  `export_flash_save`, `import_flash_save`, `save_flash_state`, `load_flash_state`, and
  `NesCore::export_flash_save` / `import_flash_save`. These are on paths every configuration
  compiles, and the two public ones answer "empty" and `false` rather than disappearing: the host
  API must not change shape with a board feature, because `zax-app` is built once.

A second flash board adds its feature to those eight `cfg`s and to the `pub(crate) use` in
`cartridges/mod.rs`, which that line's comment says in place.

### clippy `-D warnings`, `--all-targets`

| configuration | result |
| --- | --- |
| default | clean |
| `--no-default-features` | clean |
| `--features parked-boards` | clean |
| `--no-default-features --features board-111` | clean |

`cargo fmt --all --check`: clean. **No new `allow` of any lint**, in the source or in the tests.

### The suites, per configuration

| configuration | passed | failed | ignored |
| --- | ---: | ---: | ---: |
| default | **1 046** | 0 | 8 |
| `--no-default-features` | **784** | 0 | 8 |
| `--features parked-boards` | **1 099** | 0 | 8 |
| `--no-default-features --features board-111` | **791** | 0 | 8 |

791 − 784 = **7**, which is `board-111`'s own tests: MF-6c step 9's three and this lot's four.
(MF-6c step 10 recorded three behind the feature; the count is now seven.) The eighth test this lot
wrote, `req_core_095_a_board_without_a_flash_chip_exposes_nothing`, is *not* gated — it asserts
what a board **without** a flash chip does, so it belongs in every configuration, and it is one of
the 784.

### The four sweeps over `SUPPORTED_MAPPERS`, run explicitly in each of the four

| sweep | default | `--no-default-features` | `parked-boards` | `board-111` alone |
| --- | :-: | :-: | :-: | :-: |
| `req_core_100_every_required_mapper_is_registered` | ok | ok | ok | ok |
| `req_core_102_any_other_mapper_is_refused` | ok | ok | ok | ok |
| `req_core_090_the_mapper_state_survives_a_save_and_restore` (the registry round trip, with every board's registers dirtied) | ok | ok | ok | ok |
| `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` | ok | ok | ok | ok |

The `board_memory_sizes` row for 111 gained a **third** column this lot — `flash_sector` beside
`prg_ram: Some(0)` and `chr_ram: Some(32 KiB)` — and both sides of it are asserted:
`req_core_095_the_gtrom_flash_save_is_the_changed_sectors` requires `memory().flash()` to be
`Some` for 111, and `req_core_095_a_board_without_a_flash_chip_exposes_nothing` requires it to be
`None` for a board whose PRG-ROM is mask ROM.

`cargo test --release -p zax-nes -p zax-harness` also passes whole; it is re-run in step 6 against
the tree that ships.

## Step 3 — the campaign, and a result that is below the instrument's own floor

Binaries built by `build-clean.sh` from their own `git archive` tar into a **fresh** directory with
its own `CARGO_TARGET_DIR`, each deleted as soon as its binary was copied out. Roots:
`ZAX_TEST_ROMS=…/assets-free/test-roms`, `ZAX_COMM_ROMS=…/demo/roms`,
`ZAX_L2_ROMS=…/test-originals`, `ZAX_L2_PACKS=…/campaign/play-user/profiles`.

| label | tree | md5 |
| --- | --- | --- |
| `mfunl9-0-base` | **`7972e5f3`**, the lot's base commit | `cfdf90abb19687f52778442c9865e976` |
| `mfunl9-1-flash` | **`f18eb44e`**, step 2 — the lot's last code commit | `c390d7a63faee1dc39798825b36ff084` |
| `mfunl9-0-again` | a byte copy of `mfunl9-0-base`, the **same-behaviour pair** the performance rule asks for | same md5 as the base |

`mfunl9-0-base`'s md5 is **identical to MF-UNL-8 step 7's `unl8` binary**, which that lot built from
`12efb950`. So the three commits between `12efb950` and `7972e5f3` added nothing a compiler can see
— they are the progress note, `BOARD-FEATURES.md` and the journal — and this lot's base is that
lot's measured binary to the byte. The identity has now held for six lots in a row, and this lot
built the binary rather than assuming it.

### The PR-1 set, thirteen ROMs, 300 frames

| ROM | base | lot | delta | % |
| --- | ---: | ---: | ---: | ---: |
| duelito | 7 380 384 834 | 7 380 423 708 | +38 874 | +0.001 |
| m0 | 5 517 237 696 | 5 517 235 183 | −2 513 | −0.000 |
| m1 | 4 880 947 363 | 4 880 987 754 | +40 391 | **+0.001** |
| m4 | 5 467 701 228 | 5 467 739 753 | +38 525 | +0.001 |
| metroid | 5 115 555 579 | 5 115 539 593 | −15 986 | −0.000 |
| mm2 | 6 017 959 790 | 6 017 964 745 | +4 955 | +0.000 |
| mmc5 | 11 266 800 864 | 11 266 787 743 | −13 121 | −0.000 |
| oamstr | 3 318 443 134 | 3 318 448 225 | +5 091 | +0.000 |
| readbuf | 6 834 503 479 | 6 834 507 138 | +3 659 | +0.000 |
| smb1 | 5 996 321 454 | 5 996 347 642 | +26 188 | +0.000 |
| smb3 | 7 949 404 848 | 7 949 364 350 | −40 498 | −0.001 |
| vblnmi | 3 308 270 639 | 3 308 258 838 | −11 801 | −0.000 |
| z2 | 6 291 555 371 | 6 291 503 343 | −52 028 | −0.001 |
| **total** | **79 345 086 279** | **79 345 108 015** | **+21 736** | **+0.000** |

Worst ROM **+0.001 %** against a tolerance of **+0.02 %**, six of thirteen below, set total
+0.000 %.

**The floor, measured as MF-UNL-8 step 7 said it must be — the same binary twice.** That lot found
that `callgrind-set.sh`'s claim of determinism holds for `core` and not for the whole process on
this machine, and this lot measured it again rather than quoting it:

| | worst ROM | set total | "SLOWER on" |
| --- | ---: | ---: | ---: |
| base vs the **same binary** (`mfunl9-0-again`) | m1 **+0.001 %** (+35 782) | **+55 761** | 7 ROMs |
| base vs the **lot** | m1 **+0.001 %** (+40 391) | +21 736 | 7 ROMs |

The two runs have the same worst ROM, the same percentage, the same count of ROMs over, and the
floor's set total is **larger** than the lot's. The lot's whole PR-1 result is inside the noise the
instrument has on an unchanged binary.

**The symbols.** With the threshold at 20 000, exactly one symbol moved anywhere in the thirteen
ROMs: `getgeneric` — Lua's `ltable.c` — at −21 940 (`mmc5`), +21 892 (`readbuf`) and −37 220
(`smb3`). **No emulation symbol moved at all**, in either direction.

### Level 2, the five games with their packs, 600 frames in play

| game | `core` base | `core` lot | delta | `total` % |
| --- | ---: | ---: | ---: | ---: |
| metroid | 11 146 849 562 | 11 146 849 562 | **+0** | +0.001 |
| mm2 | 12 037 230 685 | 12 037 230 685 | **+0** | −0.000 |
| smb1 | 11 515 129 187 | 11 515 129 187 | **+0** | **+0.019** |
| smb3 | 15 315 040 380 | 15 315 040 380 | **+0** | +0.001 |
| zelda2 | 12 774 981 932 | 12 774 981 932 | **+0** | −0.009 |

**`core` is identical to the instruction on all five.** Not "inside the tolerance" — equal, zero,
on every game. That is the strongest statement this instrument can make about a change to the
emulation, and it is what decision 1 of step 1 was designed to produce: the flash is a side table
written when a save is written and read by nothing on a bus path.

**`smb1/total` at +0.019 % is inside the +0.02 % tolerance, and the lot's rule asks for the
same-behaviour pair anyway.** Here it is, the identical binary against itself at level 2:

| | `smb1/total` | `smb3/total` | every `core` |
| --- | ---: | ---: | :-: |
| base vs the **same binary** | **+0.009 %** (+1 266 778) | **+0.018 %** (+4 263 745) | **+0** |
| base vs the **lot** | +0.019 % (+2 649 441) | +0.001 % (+171 842) | **+0** |

An unchanged binary moves `smb3/total` by +0.018 % and `smb1/total` by +0.009 % on this machine,
with `core` exactly equal both times. The lot's worst `total` figure is that same quantity on a
different game. The tolerance is **not** redefined — nothing in this lot exceeds it — and the pair
is recorded because the rule says to show it.

The level-2 symbols say the same thing as PR-1's. Above 200 000, the only movers are
`luaH_getshortstr` (+1 991 543 on `smb1`), `luaH_set` (+712 899 on `smb1`, −226 138 on `smb3`,
−696 977 on `zelda2`), `getgeneric` (−691 448 on `zelda2`) — all three are Lua's `ltable.c` — and
`__memcmp_avx2_movbe` (+231 095 on `smb3`), which is libc. **Not one emulation symbol.**

### The feature decision

| feature | numbers | measured | decision |
| --- | ---: | --- | --- |
| `board-111` | 111 | PR-1 worst ROM **+0.001 %** against a **+0.001 %** floor on the same binary; level-2 `core` **identical to the instruction on all five games**; no emulation symbol moved on any of the eighteen ROMs | **stays in `default`**, unchanged |

Three things follow, and each is a decision rather than an omission:

1. **`board-111` stays in `default`.** It was there when the lot opened (MF-CART step 5 put it
   back) and nothing this lot measured moves it. The brief's "parked boards stay parked" clause has
   no subject here, as step 0 established.
2. **The flash does not get a feature of its own.** The owner's rule puts a mechanism of a board
   behind its own feature "if it costs more than the board itself", the `board-68` /
   `board-68-chr-nametables` precedent. This one costs nothing a 300-frame or a 600-frame callgrind
   run can distinguish from zero, so a second feature would buy a switch with no figure behind it
   — and `BOARD-FEATURES.md`'s note 1 is explicit that "the granularity is the granularity the
   measurements exist at".
3. **Nothing moves into `parked-boards`**, which keeps `board-vs-system` as its only entry.

## Step 4 — golden-core, and the full run with both ROM roots

### golden-core over the 98 commercial originals, 1 200 frames each

    zax-harness-<label> golden-core /mnt/g/3dsen/test-originals gc98-<label>.jsonl 1200

| | |
| --- | --- |
| `7972e5f3` (`mfunl9-0-base`) against the lot (`mfunl9-1-flash`) | **byte-identical** (`cmp`, not just the digest) |
| the digest | `md5 f79e7df842c3ee43e7d9b67861a927b5` |

**That is the digest MF-UNL-2, MF-UNL-3, MF-UNL-4, MF-UNL-5, MF-UNL-6, MF-UNL-7 and MF-UNL-8 all
recorded.** Eight lots, and not one pixel of a commercial original has moved.

It matters more here than in the lots before it, because this is the first lot to touch code every
board compiles: `CartridgeMemory` gained a field, `Cartridge::save_state` and
`Cartridge::load_state` each gained a call, and `NesCore::STATE_VERSION` moved. A digest taken over
98 games at 1 200 frames each is what turns "those three changes are inert for every board but 111"
from an argument into a measurement.

### The full run with both ROM roots

    ZAXNES_ROMS=G:/3dsen/demo/roms
    ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms
    cargo test --release --no-fail-fast -p zax-nes -p zax-harness -- --include-ignored

**1 118 passed, 0 failed, 0 ignored.** The same command without `--include-ignored` and without the
two variables is **1 109 / 0 / 9**, so the nine move out of the ignored column and into the passing
one — which is the proof that they ran rather than passed vacuously. (1 109 + 9 = 1 118.)

1 118 against MF-UNL-8's 1 113 is **+5**, this lot's five tests.

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
