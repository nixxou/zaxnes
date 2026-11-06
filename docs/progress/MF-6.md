# Lot MF-6 — the inventory of the boards still missing, and the plan of the sub-lots

Baseline `fcfa5cb3` (the end of MF-5b, the VRC7). **This lot writes no emulator code**: not one
`.rs` file changes. It produces `docs/review/MF-6-inventory.md`, the document MF-6a, MF-6b and
MF-6c will each be briefed from.

## Steps

0. The plan, the sources, and the ROM situation settled first.
1. The inventory: every iNES number Mesen 2 or NESdev documents and our registry does not handle.
2. The hooks each board needs, the hot paths they would touch, and the variants of boards we
   already have.
3. The plan of the sub-lots MF-6a / MF-6b / MF-6c, and what is deliberately left out.
4. The check: `cargo fmt --check` and `cargo test --release -p zax-nes -p zax-harness`, unchanged.

## The sources consulted

| source | what it settles |
| --- | --- |
| `rust/crates/zax-nes/src/cartridges/registry.rs` | `SUPPORTED_MAPPERS`, the 41 numbers this build runs |
| `third_party/mesen2/Core/NES/MapperFactory.cpp:279-648` | the numbers Mesen 2 knows, and which class each builds |
| `spec/SPEC-02-coeur-nes.md` §3.11 (REQ-CORE-100/101/102), §3.7 (REQ-CORE-075) | the required table, and that adding beyond it is permitted |
| `spec/changes/AMD-061-exactitude-des-cartes.md` §1 | public hardware documentation prevails over the reference program |
| `docs/review/MR-1-mappers.md` | the board inventory as reviewed against Mesen, and the variant questions (MR-4-3, MR-34-1) |
| `docs/progress/MF-5a.md`, `MF-5b.md`, `MF-3b.md` | how a board was added at zero cost to the others |
| `assets-free/test-roms/` | every `.nes`/`.unf` header decoded, by mapper number |
| NESdev wiki: "Mapper", "List of mappers", and each board's own page | board names, register layouts, licensed/unlicensed |

## The ROM situation, settled first because it decides every sub-lot's test plan

**No test ROM in `assets-free/test-roms` exercises any board this build does not already have.**
Re-established here independently of MF-5a step 0: every `.nes`, `.unf` and `.unif` under that
tree was opened and its header decoded (iNES signature, byte 6 bits 4-7, byte 7 bits 4-7, and
byte 8 bits 0-3 when byte 7 bits 2-3 say NES 2.0) —

    333 images with an iNES signature, mapper numbers
    0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 22, 28, 34, 66, 69, 78, 118, 180

and every one of those eighteen numbers is in `SUPPORTED_MAPPERS`. Four files carry no iNES
header: `nes-test-roms/other/LINUSMUS.NES` (signature `NES\x1b`) and the three UNIF images
`scanline/scanline.unif`, `scanline-a1/scanline.unif`, `stomper/smwstomp.unif`, which name a board
rather than a number and which this build does not load at all.

So **the "test ROM present?" column of the inventory is `no` for every row**, and no sub-lot can
be driven by a ROM. Each one has to rest on unit tests written against the NESdev page and
cross-checked against `third_party/mesen2`, with commercial titles named for R0 to run — which is
exactly the position MF-5a and MF-5b were in, and both shipped from it.

## Step 1 — the inventory

`docs/review/MF-6-inventory.md` §1 to §3. **245 of the numbers Mesen 2 builds a board for are
numbers this registry refuses**, derived mechanically from `MapperFactory.cpp:279-648` rather than
transcribed, and split four ways because they are not 245 equal items:

| § | what | count |
| --- | --- | ---: |
| 3.1 | boards with licensed titles and no counterpart here | 38 numbers |
| 3.2 | numbers that are a **parameter** of a board we already have | 27 + the 48 MMC3 clones |
| 3.3 | unlicensed boards of a single publisher (Sachen, Kaiser, Waixing, TXC, NTDEC, JY, Whirlwind, FFE) | 60 |
| 3.4 | pirate multicarts, BMC boards and MMC3 clones | 134 |
| 3.5 | numbers Mesen knows and refuses, the gaps, the UNIF-only boards, the FDS | 23 + the gaps |

Every title count is the figure its own NESdev board page gives — its "N in NesCartDB" line or the
length of the game list it prints, the two distinguished in the table. NesCartDB's own host was
unreachable from this session, so the counts are the wiki's transcriptions of it and the document
says so. Where a page gives neither, the cell says that instead of guessing.

**Two findings worth more than the tables:**

* **REQ-CORE-100's required table is covered in full.** Nothing in this document is required by
  the spec; the build already adds twelve numbers beyond the table, which REQ-CORE-102 permits in
  as many words. So the sub-lots are ordered by what each buys, not by compliance.
* **The profile list has "Super Spike V'ball", and mapper 47 is the Nintendo multicart of exactly
  that title.** The US standalone is an ordinary mapper 4 and loads today; a user whose dump is
  `Super Spike V'Ball + Nintendo World Cup` gets REQ-CORE-011's refusal and never reaches the
  profile. `profiles.json` records no mapper number, so which board each of the 101 dumps sits on
  is R0's to check — but this one named a missing board by itself, and it is why 47 is in MF-6a.

## Step 2 — the hooks, the hot paths, and the variants

`MF-6-inventory.md` §4, one section per hook, each with the measurement that priced it rather than
a judgement. The summary table §4.10 is the one a sub-lot brief should carry verbatim.

**Eight of the ten hooks cost a board that does not use them exactly zero**, and each is zero
because a lot made it so: `clocks_on_cpu_cycle` (MF-5a step 2, which took 11 instructions per CPU
cycle to 0 for 26 of the 38 numbers), `watches_ppu_reads` and `watches_ppu_bus_without_read`
(MF-3b step 2, over 57 600 fetches a frame), `nametable_pages_vary`, `rom_windows`,
`bus_conflicts`, and the `mapper_number` field MF-5a step 1 turned from a per-sprite dispatch into
an `i32`.

**Two are not latched, and they are the whole performance story of MF-6:**

* **`NesBus::irq_asserted`.** MF-5b's thirty-second arm pushed the chain past the inliner's budget
  and LLVM outlined it: **+169 739 749 instructions, ~19 per CPU cycle, on twelve ROMs of thirteen
  whose board cannot assert an IRQ at all**, +0.542 % on every one. Four `#[inline(always)]` fixed
  it; the load-time `irq_capable` latch was **built and refuted** (+1.3 to +1.8 % on three of R0's
  ROMs, two of them MMC3s), so MF-6b must not re-litigate it. MF-6b adds up to five more arms and
  the yardstick per arm is MF-5a's and MF-5b's identical +11 in `branch` / +47 in `resolve`.
* **`Mapper::read_nametable`, capped at two distinct arm bodies by measurement.** Q-123: a third
  body takes `read_nametable_inner` from 132 to 109-83 *for every game*, and MF-5a's five shapes
  prove "the trigger is the third distinct body, not its size". **Mapper 68 (Sunsoft-4) is that
  third body** — its `$E000` bit 4 is the same CHR-ROM-nametable feature the VRC6 has and we
  decode and ignore. §4.5 says what MF-6c must do instead.

§3.2 is the variants: **27 numbers plus the 48 MMC3 clones are a parameter of a board we already
have, not a new board** — and `board.rs`'s own note is why that matters (32 variants, `Mmc5`
pinned at index 5 by the niche-filling layout, appended never inserted). The `McAcc` line
(MR-4-3, shipped) is the precedent for a `const` parameter that costs the parent board nothing;
mapper 26 is the precedent for a runtime `bool` when the test is off the per-cycle path.

Three decisions were taken here rather than deferred, each with the text that settles it:

* **mappers 72, 86, 92 get their boards and not their µPD7756C audio.** REQ-CORE-075 lists six
  optional chips and this is not one of them, and the ADPCM samples live in a mask ROM inside the
  ASIC — "sample files for the speech synthesis chip emulation", which no source this role may use
  contains. A test pins the silence. **No Q ticket.**
* **mappers 80, 82, 207 and 552's mapper-internal WRAM is the Namco 163's shape**, not a new
  mechanism: `registry.rs:90-97` already puts a chip's internal bytes "at the end of PRG-RAM so
  that REQ-CORE-095's single battery slice carries it".
* **the 24C01/24C02 EEPROM of 157 and 159 is genuinely new**, and REQ-CORE-100's mapper-16 row
  already prices it: "EEPROM : P3".

[PROGRESS] MF-6 step 0/4 done: the plan, the sources, and the ROM situation settled first
[PROGRESS] MF-6 step 1/4 done: the inventory, 245 numbers in four tables
[PROGRESS] MF-6 step 2/4 done: the hooks, the two unlatched hot paths, and the variants
