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

[PROGRESS] MF-6 step 0/4 done: the plan, the sources, and the ROM situation settled first
