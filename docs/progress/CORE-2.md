# CORE-2 — one generic hot path, specialised per popular board

Design: `docs/design/CORE-TIERS.md`, option D ("One generic hot path, specialised per popular board"),
chosen over tiered cores after the re-study of 2026-11-14. Branch `feat/core-spec`, from dev `5b2dee99`.

## What changed

* `cartridges/board.rs`: the trait `BoardKind` (how a frame reaches its board), `AnyBoard` (every
  board through the `Board` dispatch, the path every game took before), one kind per specialised
  board, and **the table** `with_board_kind!`, the only place that lists them.
* `cartridges/mod.rs`: every hot `Cartridge` method has a `_as::<K>` version; its old name forwards
  with `AnyBoard`, so nothing outside the hot path changed.
* `ppu/`: the cartridge handle (`Cart<'_, '_, K>`, a `CartSlot`) carries the kind, so the compiler
  infers it through the PPU's internal calls; the public entry points keep their names.
* `nes_bus.rs`: the bus functions that reach the cartridge are generic; the logic of `CpuBus` is
  written once (`cpu_read_as` etc.) and both `NesBus`'s implementation and `KindBus` forward to it.
* `nes_core.rs`: `run_frame` looks at the board once per frame and runs `run_frame_steps::<K>`.
* MMC5: the light board (`Mmc5Kind`) and the full one (`Mmc5ExtKind`) are two kinds. The write that
  arms extended attributes or the split replaces the board (as before); the light loop sees it
  after that instruction and the frame goes on under `Mmc5ExtKind`. For the cycles of the arming
  instruction left after the write, `Mmc5Kind` reaches `Mmc5Ext::base`, the light board inside.

**The rule**: nothing in the hot path reaches the board other than through `BoardKind`; a frame
runs through a specialised kind only while the board is that variant.

## Measurements (callgrind, clean builds, against dev `5b2dee99`)

| Commit | SMB3 | SMB1 | MM2 | Metroid | Zelda II | Note |
|---|---|---|---|---|---|---|
| 5ae3b56a | −2.4 % | +1.6 % | +2.3 % | +3.1 % | +3.1 % | first build: `span_pixel` and four others outlined |
| a995738f | −4.38 % | −1.59 % | +0.17 % | +0.21 % | +0.53 % | those five pinned `inline(always)` |
| febf73ce | −4.20 % | −1.22 % | +1.32 % | +1.82 % | +1.50 % | dispatch and catch-up pinned too: worse, reverted |
| 25489ea2 | −4.40 % | −1.59 % | +1.00 % | +1.38 % | +1.35 % | the two dispatches alone: they were the cause, reverted |
| 12d6b8ba | −4.39 % | −1.59 % | −0.48 % | −2.13 % | −2.10 % | MMC1, UxROM, CNROM, AxROM specialised |
| ab65eb6d | −4.19 % | −1.23 % | −0.19 % | −1.75 % | −1.94 % | PPU catch-up pinned: helps test ROMs, costs games, reverted |
| ec1dfc86 | −4.40 % | −1.59 % | −0.48 % | −2.12 % | −2.10 % | MMC5 specialised (PR-1 `mmc5` +2.98 % → +0.28 %) |

Lesson: with several copies of the hot path LLVM stops inlining functions that every copy calls;
pinning helps only for the per-pixel and per-fetch ones, and pinning more hurt twice.

Known cost: three blargg test ROMs of the PR-1 set (`oam_stress`, `ppu_vbl_nmi`, `ppu_read_buffer`)
stay +1.3 to +2.4 %: they hit the PPU registers constantly, and the PPU catch-up is out of line in
the specialised copies. Every game measured gains.

Code size (`.text`): 4.90 MB on dev, 5.02 MB with two specialised boards; final figure below.

## Equivalence

`tests/core_specialised_path_tests.rs` (ignored, needs ROMs): every ROM of `ZAXNES_TEST_ROMS` and
`ZAXNES_ROMS` on a specialised board runs 600 frames with a button pattern through its specialised
path and through `AnyBoard` (`NesCore::set_generic_path`); cycles and picture of every frame and the
savestate at the end must match. 403 ROMs, all identical, `mmc5exram` among them (it arms the
extended attributes). No commercial MMC5 game was available locally: the vertical split is covered
by the logic, not by a ROM.
