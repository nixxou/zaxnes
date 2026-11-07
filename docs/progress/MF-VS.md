# Lot MF-VS: the Nintendo VS. System (single-screen)

The owner reopened the VS. System on 2026-11-06; `docs/review/MF-6-inventory.md` §6 item 3 had set
it aside on 2026-10-29 and priced what it would take — "a coin and DIP-switch input surface, a
different palette, a two-bank CHR select, and a per-ROM configuration table, four pieces, none of
them in the cartridge layer". This lot builds those four and the two mapper numbers that go with
them, 99 and 151.

**Baseline and performance reference: `f1278f46`**, the end of MF-CART. What that lot changed about
the price of a board is in `rust/docs/PERFORMANCE-HISTORY.md` §5.15, and the one sentence this plan
is built on is its own:

> A board that shares a variant is free: a shared enum takes at least twelve more at +0.002 % on
> the worst ROM, a tenth of the tolerance. A board that wants a `Board` discriminant of its own is
> still a lottery, and the only quantum left in this build is `NesBus::push_expansion_audio`
> ±8 934 160.

So **mapper 99 joins `DecodeBoard`** (a sixth variant of the shared enum MF-6c step 6 paid the
discriminant for) and **mapper 151 is a parameter of the existing VRC1** in `SmallBoard`, which
costs that enum no variant at all. Neither asks for a new `Board` discriminant, and nothing in this
lot touches `NesBus::push_expansion_audio`.

## Sources

Allowed sources only. Every register, palette and bit below is cited in the code at the line that
implements it.

* nesdev wiki, **"Vs. System"** — the `$4016`/`$4017` read layouts (coins, service button, DIP
  switches), the `$4016` write layout (strobe, DualSystem bit, the mapper-99 CHR bit), the
  `$4020`-`$5FFF` coin counter, the left-stick/right-stick wiring, and which PPU each family of
  games has.
* nesdev wiki, **"INES Mapper 099"** — the banks, the bank-select register and the 40 KiB PRG
  variant of *VS. Gumshoe*.
* nesdev wiki, **"PPU palettes"** — the four RP2C04 palettes as the page's own
  `PaletteLUT_2C04_000x` arrays ("Emulator authors may implement the 2C04 variants as a LUT
  indexing the ordered palette"), and the 2C03/2C05 relationship.
* nesdev wiki, **"NES 2.0"** — byte 7 bits 1-0 (console type) and byte 13 (Vs. PPU type in the low
  nibble, Vs. hardware type in the high nibble), including the extended console type 1.
* nesdev wiki, **"Standard controller"** — the eight button bits of a port.
* `third_party/mesen2`, `Core/NES/Mappers/VsSystem/` and `Core/NES/NesPpu.cpp` — read as a
  **cross-check** of the readings above, never as the source of a claim. Two independent
  agreements are recorded in the steps: the four 2C04 LUTs match the page's arrays value for
  value, and the DIP-switch bit order resolves one ambiguity of the page's own wording (step 5).

## The design, decided before step 1 and not to be re-litigated

1. **One state, chosen once at load.** `VsSystem` holds the PPU model, the hardware type, the eight
   DIP switches, the coin and service lines and the swapped-port flag. Nothing downstream asks
   "which PPU is this?" per dot, per pixel or per register: each consumer is handed the *answer* at
   load — a `&'static [u8; 64]` palette LUT, a `u8` of status bits, a `bool`.
2. **The palette is an index remap, applied where palette RAM is written.** The core's pixels are
   6-bit NES colour indices and the RGB table is the host's (`zax_engine3d::palette`, SPEC-04 §4.1,
   a player setting). A VS PPU differs from the 2C02 by the *order* of its master palette, which
   the nesdev page itself says may be implemented as a LUT into the ordered palette. The plan put
   the remap at `Ppu2C02::snapshot_palette`, the one place the 32 bytes leave the core; **step 2
   found a strictly better place and took it** — the one palette-RAM *write*, beside the
   `palette_marked` shadow that already exists there for the same reason. A NES image pays
   nothing at all, not even a per-frame test, and the per-pixel path is untouched:
   `Ppu2C02::read_pattern_planes` must come out of the dump unchanged.
3. **The RC2C05's two quirks are arithmetic, not a branch.** `$2002`'s low five bits become
   `(decay & status_latch_mask) | status_id`, two fields set at load (`0x1F` and `0x00` for every
   other PPU, so the value is identical and no model is tested); the `$2000`/`$2001` write swap
   lives **inside those two arms only**, which are the rarest arms of `Ppu2C02::write`, so the
   `$2005`/`$2006`/`$2007` paths are byte-identical.
4. **Mapper 99's register is a CPU register, not a cartridge one.** `$4016` writes are handled by
   `NesBus::write_internal` and never reach the cartridge; the lot adds the forward there, gated so
   that a non-VS image pays one test on a path every game takes twice a frame.
5. **The per-ROM table is keyed by the CRC-32 of the PRG+CHR data**, the shape
   `builtin_patches::BuiltinPatch::source_crc` already uses, because an iNES 1.0 VS header carries
   no PPU field. An image no row matches gets the RP2C03 — the plain NES palette — and says so in
   the load message.
6. **One Cargo feature, `board-vs-system`**, gating all of it: the board, the PPU fields, the input
   surface, the table, the save-state block and the tests. In `default` if it measures inside
   +0.02 % per ROM on both runs, in `parked-boards` otherwise (`docs/review/BOARD-FEATURES.md`).

## Out of scope, recorded as gaps (owner's list)

* the **dual-screen** VS. System (*VS. Tennis*, *VS. Baseball*…): two CPUs, two PPUs, the
  `$4016` write's bit 1 and the shared 2 KiB at `$6000`-`$7FFF`. Byte 13's hardware types $5 and $6
  are *detected* and named in the load message, and the image then runs as a single screen.
* the **VS. Zapper** games' light gun.
* the four **protection boards** byte 13's hardware types $1-$4 name (RBI Baseball, TKO Boxing,
  Super Xevious, VS. Ice Climber Japan): detected and named, not emulated.
* the app's **UI binding** of the coin, service and DIP inputs: the core API and a harness command
  are this lot's deliverable, the UI is the next step.
* any **3D profile or widescreen pack** for a VS image.

## Steps

* [x] **0/10** — the plan: the baseline read, the design above settled, this file. No code.
* [x] **1/10** — detection: byte 7 bit 0 and NES 2.0 console type 1 (and extended type 1), byte 13's
  two nibbles, the per-ROM CRC table, the load message, the `VsPpuModel` and `VsHardware` types.
* [x] **2/10** — the palettes: the four RP2C04 LUTs and the 2C03/2C05 identity, chosen at load and
  applied in `snapshot_palette` and the margin palette.
* [ ] **3/10** — the RC2C05: the `$2000`/`$2001` write swap and the `$2002` ID.
* [ ] **4/10** — mapper 99 as a `DecodeBoard` member, the `$4016` write forward, the 40 KiB PRG
  bank; mapper 151 as a parameter of the VRC1.
* [ ] **5/10** — the inputs: two coin slots, the service button, the eight DIP switches at
  `$4016`/`$4017`, the swapped ports, the coin counter at `$4020`-`$5FFF`; the core API; the save
  state behind the existing versioning.
* [ ] **6/10** — the harness: a `vs` command that loads an image, sets DIP switches, inserts a coin
  and prints what the core reports.
* [ ] **7/10** — the measurement: level 2 with `callgrind-l2.sh` + `l2-diff.py --bar`, PR-1's
  thirteen with `callgrind-set.sh` + `callgrind-diff.py`, clean `git archive` builds, the three
  `Cpu6502::step` inlinings, `zone_offset` and `read_pattern_planes` on the dump; the feature's
  place in `default` decided by the figures.
* [ ] **8/10** — the documents: the `BOARD-FEATURES.md` row, `PERFORMANCE-HISTORY.md` §5.16, the
  gaps, the Q tickets, `MF-6-inventory.md` §6 item 3 answered, the journal.
* [ ] **9/10** — the check: fmt; clippy `-D warnings` on the crates touched in both configurations;
  `cargo test --release -p zax-nes -p zax-harness` in both; the ignored suite with the two ROM
  variables; the campaign's 331 rows field by field against MF-CART's.

## What each step found

### Step 0 — the plan

Read: `docs/progress/MF-CART.md`, `rust/docs/PERFORMANCE-HISTORY.md` §5.15 and §5.14,
`docs/review/BOARD-FEATURES.md`, `docs/review/MF-6-inventory.md` §6 item 3 and rows 99/151/75, and
the code the lot touches — `rom/ines_header.rs`, `rom/rom_image.rs`, `input/mod.rs`, `nes_bus.rs`'s
`$4016` paths, `ppu/mod.rs`'s register read and write, `cartridges/mappers/decode_boards.rs`,
`cartridges/registry.rs`, `builtin_patches.rs`'s table shape and `zax-engine3d`'s `palette.rs`.

Three facts that shaped the design, each checked in the tree rather than assumed:

1. **The core never produces RGB.** `zax_engine3d::palette` holds eleven 64-colour tables and the
   player picks one (`video.nes_palette`); the core's frames carry 6-bit indices and
   `FrameBatch::palette`, the 32 bytes of `Ppu2C02::snapshot_palette`, is the whole of what the
   renderer colours them with. A VS palette is therefore an index remap and not a table of colours,
   which is also the form the nesdev page offers it in.
2. **`snapshot_palette` is called once per frame**, from `nes_core.rs` where the frame batch is
   built, and `zax-app`'s `game_session.rs` reads entry 0 of it for the backdrop. Two call sites,
   neither per pixel.
3. **`$4016` writes do not reach the cartridge.** `NesBus::write_internal`'s `address < 0x4020`
   arm sends `$4014` to the OAM DMA, `$4016` to `InputPorts::write` and everything else to the APU.
   Mapper 99 needs that write, so the forward is new code on a path that is not cold — and is taken
   twice a frame by every game, which is why step 7 measures it rather than assuming it.

Free space on the drive at the start of the lot: 13 GB. `rust/target-mf` holds 26 GB of earlier
lots' build trees and logs; the clean-build directories of step 7 are deleted as soon as their
figures are recorded, and the check before each build is the rule of the brief (stop under 5 GB).

### Step 1 — detection, the per-ROM table and the load note

**Three rules make an image a VS. System** (`vs::VsSystem::is_vs_image`), and the first is the one
the brief names: an **iNES 1.0** header with **byte 7 bit 0** set, a **NES 2.0** header with
**console type 1** — or console type 3 with byte 13's *extended* console type equal to 1, which the
page lists and which says "Vs. System" and nothing more — and, whatever the console type says,
**mapper 99 or 151**, two numbers that exist only on this board. A **dirty** iNES header is
excluded, because REQ-CORE-012's existing rule is that bytes 7 to 15 of one are not read at all: an
old tool's `DiskDude!` comment must not be able to turn a game into an arcade cabinet, and a test
pins that.

`InesHeader` gains three fields and no behaviour: `console_type` (byte 7's two low bits, zero on a
dirty header), `byte13` and `byte15`, in the shape `byte6` and `byte10` already had. Byte 13's low
nibble is the Vs. PPU type and its high nibble the Vs. hardware type; byte 15 is the default
expansion device, whose `$04`, `$05` and `$07` are "1P via `$4016`", "1P via `$4017`" and the
Vs. Zapper — so the **stick swap is a header field**, not a guess.

**The table is generated, not hand-written, and the source is the one the brief names.** The public
NES 2.0 ROM header database (`nes20db.xml`, `NewRisingSun`, the build dated 2026-09-26) has 21 339
games, of which exactly **74** carry `<console type="1">`, each with a `<vs hardware= ppu= />` and
an `<expansion type= />` and, in the same entry, the `<rom crc32= />` of the data **without the
header** — which is byte for byte the key `builtin_patches::data_crc` already computes. The 74 rows
are sorted by CRC and bisected; a test walks every one of them through the detection path and
another pins the sort.

Two things the data settled that the plan had only assumed:

* **Mapper 151 is in no row.** *Vs. Gradius* and *Vs. The Goonies* are recorded as **mapper 75**
  with console type 1, which is exactly what `MF-6-inventory.md` row 151 says the number is — "the
  **75** board of MF-6a … nothing but the VS System wrapper". So step 4's job for 151 is to make the
  number reach the VRC1 and turn the VS layer on, and not to build anything.
* **The eight mapper numbers a VS image can have are 0, 1, 2, 4, 67, 75, 99 and 206**, and every one
  but 99 is already supported. 55 of the 74 rows are mapper 99, and the two 40 KiB PRG images the
  page mentions are *Vs. Gumshoe* and *Raid on Bungeling Bay*.

**No DIP column**: the header database records no DIP settings and the nesdev wiki has no per-game
DIP list, so the brief's fallback applies and every image starts with the eight switches off.

**One bug worth recording, because it was in the generator and not in the code.** The database's
comment separates several file names with `"; "`, and `"Stroke &amp; Match Golf"` carries a
semicolon of its own inside its XML escape: splitting on `";"` before unescaping truncated five
titles to `"Stroke &amp"`. Caught by clippy's `doc_markdown` pass over the generated file rather
than by a test, which is why `scratchpad/check_table.py` now re-derives the 74 rows from the
database and compares them to the committed file field by field — it reports **identical**.

`RomLoadResult` gains a `note: Option<String>`, a message about the image that is not a failure,
and `NesCore::load_rom` fills it from `VsSystem::load_note`. The note names the hardware type, the
PPU model, where the model came from, and **every piece this build leaves out** — the `DualSystem`
running as a single screen, the protection board, the Zapper — so that nothing is silently dropped.
An image nothing identifies says so in the brief's own words: "unknown VS image … so the plain NES
palette".

The state lives on `InputPorts`, as `Option<Box<VsSystem>>`, because what it carries is the
cabinet; like the plugged device types it is configuration and survives `reset`. `NesBus` has one
setter, `set_vs_system`, which steps 2 to 4 extend to push the PPU's and the board's answers out
from the same call.

Green: **880 passed, 0 failed, 7 ignored** by default (862 at the baseline plus this step's 18),
**783 / 0 / 7** with `--no-default-features`, fmt clean and clippy `-D warnings` clean on `zax-nes`
in all three configurations and on `zax-harness`.

### Step 2 — the palettes, and a better place for the remap than the plan had

**The four tables are the page's own.** nesdev's "PPU palettes" offers the implementation
directly — "Emulator authors may implement the 2C04 variants as a LUT indexing the ordered
palette" — and gives four 64-byte `PaletteLUT_2C04_000x` arrays. Those are in
`src/vs/palettes.rs` value for value. Cross-checked against `third_party/mesen2`'s
`NesDefaultVideoFilter.cpp`, whose `_unscrambledPalettes` holds the same four tables in decimal:
**every one of the 256 entries agrees**, which is two independent sources for each.

The RP2C03, the RC2C03 and the RC2C05 have **no table at all** — `palette_lut()` is `None` — because
their master palette *is* the ordered one (the page's note on the 2C05-99 variant: "believed to be
identical to that of the 2C03"). That `None` is also the brief's rule for an unidentified image,
"gets the RP2C03 (plain NES palette)", so the default costs nothing and means exactly what it says.

**The plan said the remap goes in `snapshot_palette`; the tree offered somewhere strictly better,
and the step took it.** `Ppu2C02` already keeps a *derived* copy of palette RAM, `palette_marked`,
rebuilt on the one register write "here and not per pixel" (AMD-050 REQ-CORE-049's own comment).
The VS remap is the same shape of problem, so palette RAM became **three** arrays:

* `palette_raw` — what the game wrote. Six bits, and what a `$2007` read of `$3F00`-`$3F1F` gives
  back, because palette RAM sits *before* the chip's colour decoder and a read never sees it. It is
  also what the state blob carries, so the savestate format is **unchanged** and a NES state is
  byte for byte the same block it was.
* `palette` — what the PPU *shows*: `palette_raw` through the remap, and the identity on every NES
  image. Every output of the core already resolves this array — the pixel image of REQ-CORE-048,
  the tile records, the backdrop, the frame batch's 32 bytes — so **nothing downstream changed at
  all**.
* `palette_marked` — as before, derived from `palette`.

So the cost of a VS palette is **one array lookup per palette-RAM write**, a handful per frame, and
**`snapshot_palette` is untouched**: it is still the same `const fn` returning `palette` with the
four mirrors filled in. There is no per-frame test either, which is better than the plan's budget
of one. Four read paths moved from `palette` to `palette_raw` (the `$2007` read, the register peek
and the two VRAM reads) and the single write path now fills all three; `refresh_marked_palette`
became `refresh_palette_views` and rebuilds the two derived arrays from the raw one at the three
places that replace the whole palette — the power-up fill, the state restore, and the new
`set_vs_ppu`.

The margin palette is **deliberately not remapped**: it belongs to the widescreen hack packs, which
exist only for the fifteen commercial NES games they were built for, and "any 3D profile or
widescreen pack variant for VS images" is one of the lot's recorded gaps. A VS image cannot have
one, so a branch there would be dead code.

One consequence named rather than hidden: the real 2C03 renders some of the 2C02's grey-ramp
entries as black, which an **index** remap cannot express, because the table the indices are
coloured in is the player's choice (`video.nes_palette`, eleven of them) and not the chip's DAC.
The brief settles it in one phrase — "plain NES palette" — and it is in the gap list.

Six tests: the four tables are four different non-identity remaps holding the same multiset of
indices (the page's "same master palette … different permutations"); the 2C03 and the five 2C05s
have none; a synthetic image **per PPU model** writes all 32 palette bytes and checks each one
against the chosen table while `$2007` still reads back what was written; a plain NES image whose
byte 13 *claims* an RP2C04-0004 gets no remap, because byte 7 does not say Vs. System; the remap
reaches the pixel image (a backdrop written `$01` comes out `$27` on an RP2C04-0002, the page's
entry 1); and a savestate round trip puts the shown palette back through the remap.

Green: **886 / 0 / 7** by default, **783 / 0 / 7** with `--no-default-features`, fmt and clippy
`-D warnings` clean in all three configurations.
