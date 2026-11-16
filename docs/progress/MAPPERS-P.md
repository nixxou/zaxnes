# MAPPERS-P — the mappers puNES runs and Zaxnes does not

Branch `feat/mappers-p` (from `feat/mappers-n` e4596884), worktree `G:/3DSen/wt-mappers`.
Owner's request (2026-11-15): "j'aimerai ne pas avoir à rougir devant puNES".

## Resume note (read first after a crash)

* Same rules as MAPPERS-N: one `board-xxx` feature per board, documented in
  `docs/review/BOARD-FEATURES.md`; sources nesdev then puNES (`cleanroom/third_party/punes`),
  Mesen2/FCEUX/BizHawk for triage; never 3dSen; approximations in `docs/review/FIDELITY-LEADS.md`;
  safe Rust; clippy `-D warnings`; commit often; never push or merge.
* Perf: every batch measured with clean builds (`cleanroom/verify/rust-v2/cg682b.sh` pattern);
  over +0.02 % per board → specialise (`BoardKind` constants) or park behind its feature.
* Next step: the first row of the table below whose state is not "done".

## Inventory (puNES `mappers.c` switch, commented cases excluded)

* puNES: 415 numbers; Zaxnes (feat/mappers-n): 286, all of Mesen2's; **135 missing**.
* `db`: entries in Mesen's game database (`ext-tools/MesenCE-2.2.1/MesenNesDB.txt`); `hk`: dumps in
  the owner's Hackset set.

| mapper | puNES source | lines | db | hk | state |
| ---: | --- | ---: | ---: | ---: | --- |
| 55 | `mapper_055.c` | 38 | 3 | 0 | done (batch 1, `MultiP`) |
| 81 | `mapper_081.c` | 60 | 3 | 0 | done (batch 1, `MultiP`) |
| 100 | `mapper_100.c` | 175 | 1 | 0 | done (batch 29, `Mmc3P`) |
| 122 | `mapper_184.c` | 64 | 0 | 0 | done (batch 1, `MultiP`) |
| 186 | `mapper_186.c` | 82 | 1 | 0 | done (batch 60, from `.studybox` tapes as Mesen 2; a bare iNES 186 stays refused) |
| 237 | `mapper_237.c` | 98 | 1 | 0 | done (batch 1, `MultiP`) |
| 248 | `mapper_115.c` | 115 | 0 | 0 | done (batch 20, alias, `registry::board_number`) |
| 256 | `mapper_256.c` | 123 | 241 | 0 | done (batch 53, `IrqP` + `one_bus.rs`) |
| 267 | `mapper_267.c` | 87 | 1 | 0 | done (batch 12, `Mmc3P`) |
| 269 | `mapper_269.c` | 130 | 3 | 0 | done (batch 26, `Mmc3P`) |
| 271 | `mapper_271.c` | 69 | 1 | 0 | done (batch 2, `MultiP`) |
| 272 | `mapper_272.c` | 160 | 1 | 0 | done (batch 37, `IrqP`) |
| 281 | `mapper_281.c` | 69 | 11 | 0 | done (batch 2, `JyCompany` + `JyMulticart`) |
| 282 | `mapper_282.c` | 76 | 13 | 0 | done (batch 2, `JyCompany` + `JyMulticart`) |
| 291 | `mapper_291.c` | 90 | 1 | 0 | done (batch 12, `Mmc3P`) |
| 295 | `mapper_295.c` | 69 | 12 | 0 | done (batch 2, `JyCompany` + `JyMulticart`) |
| 297 | `mapper_297.c` | 119 | 1 | 0 | done (batch 11, `Mmc1Outer`) |
| 308 | `mapper_308.c` | 90 | 1 | 0 | done (batch 16, `VrcP`) |
| 311 | `mapper_311.c` | 97 | 1 | 0 | done (batch 18, `IrqP`) |
| 315 | `mapper_315.c` | 92 | 1 | 0 | done (batch 12, `Mmc3P`) |
| 322 | `mapper_322.c` | 101 | 1 | 0 | done (batch 12, `Mmc3P`) |
| 327 | `mapper_327.c` | 89 | 0 | 0 | done (batch 12, `Mmc3P`) |
| 334 | `mapper_334.c` | 95 | 1 | 0 | done (batch 12, `Mmc3P`) |
| 335 | `mapper_335.c` | 85 | 1 | 0 | done (batch 2, `MultiP`) |
| 337 | `mapper_337.c` | 92 | 0 | 0 | done (batch 3, `MultiP`) |
| 338 | `mapper_338.c` | 86 | 1 | 0 | done (batch 2, `MultiP`) |
| 339 | `mapper_339.c` | 97 | 1 | 0 | done (batch 12, `Mmc3P`) |
| 340 | `mapper_340.c` | 79 | 1 | 0 | done (batch 17, `MultiP`) |
| 341 | `mapper_341.c` | 67 | 1 | 0 | done (batch 2, `MultiP`) |
| 342 | `mapper_342.c` | 2381 | 9 | 0 | done (batch 56, `IrqP` + `cool_girl.rs`) |
| 343 | `mapper_343.c` | 71 | 0 | 0 | done (batch 2, `MultiP`) |
| 344 | `mapper_344.c` | 125 | 3 | 0 | done (batch 24, `Mmc3P`) |
| 345 | `mapper_345.c` | 102 | 0 | 0 | done (batch 12, `Mmc3P`) |
| 347 | `mapper_347.c` | 122 | 2 | 0 | done (batch 38, `MultiP`) |
| 350 | `mapper_350.c` | 99 | 0 | 0 | done (batch 3, `MultiP`) |
| 351 | `mapper_351.c` | 305 | 0 | 0 | done (batch 46, `ChipSwitch`) |
| 352 | `mapper_352.c` | 66 | 0 | 0 | done (batch 2, `MultiP`) |
| 353 | `mapper_353.c` | 149 | 1 | 0 | done (batch 28, `Mmc3P`) |
| 354 | `mapper_354.c` | 111 | 0 | 0 | done (batch 4, `MultiP`) |
| 355 | `mapper_355.c` | 258 | 4 | 1 | done (batch 58, `IrqP` + `pic16c54.rs`, EPROMs embedded by owner decision) |
| 356 | `mapper_356.c` | 106 | 1 | 0 | done (batch 25, `Mmc3P`) |
| 357 | `mapper_357.c` | 118 | 1 | 0 | done (batch 18, `IrqP`) |
| 358 | `mapper_358.c` | 76 | 4 | 0 | done (batch 3, `JyCompany` + `JyMulticart`) |
| 359 | `mapper_359.c` | 293 | 3 | 0 | done (batch 42, `IrqP`) |
| 360 | `mapper_360.c` | 54 | 1 | 0 | done (batch 2, `MultiP`) |
| 361 | `mapper_361.c` | 85 | 5 | 0 | done (batch 12, `Mmc3P`) |
| 362 | `mapper_362.c` | 99 | 2 | 0 | done (batch 41, `VrcP`) |
| 368 | `mapper_368.c` | 106 | 2 | 0 | done (batch 18, `IrqP`) |
| 369 | `mapper_369.c` | 221 | 1 | 0 | done (batch 44, `Mmc3P`) |
| 370 | `mapper_370.c` | 117 | 1 | 0 | done (batch 23, `Mmc3P`) |
| 372 | `mapper_372.c` | 96 | 1 | 0 | done (batch 12, `Mmc3P`) |
| 374 | `mapper_374.c` | 56 | 4 | 0 | done (batch 11, `Mmc1Outer`) |
| 375 | `mapper_375.c` | 77 | 1 | 0 | done (batch 3, `MultiP`) |
| 377 | `mapper_377.c` | 86 | 3 | 0 | done (batch 12, `Mmc3P`) |
| 380 | `mapper_380.c` | 89 | 9 | 0 | done (batch 4, `MultiP`) |
| 381 | `mapper_381.c` | 60 | 0 | 0 | done (batch 2, `MultiP`) |
| 382 | `mapper_382.c` | 70 | 1 | 0 | done (batch 5, `MultiP`) |
| 384 | `mapper_384.c` | 65 | 0 | 0 | done (batch 16, `VrcP`) |
| 386 | `mapper_386.c` | 76 | 1 | 0 | done (batch 5, `JyCompany` + `JyMulticart`) |
| 387 | `mapper_387.c` | 76 | 4 | 0 | done (batch 5, `JyCompany` + `JyMulticart`) |
| 388 | `mapper_388.c` | 76 | 2 | 0 | done (batch 5, `JyCompany` + `JyMulticart`) |
| 389 | `mapper_389.c` | 90 | 1 | 1 | done (batch 1, `MultiP`) |
| 390 | `mapper_390.c` | 93 | 0 | 0 | done (batch 5, `MultiP`) |
| 393 | `mapper_393.c` | 123 | 1 | 0 | done (batch 15, `Mmc3P`) |
| 394 | `mapper_394.c` | 193 | 0 | 0 | done (batch 54, `JyCompany` + `Jy394`) |
| 395 | `mapper_395.c` | 88 | 4 | 0 | done (batch 13, `Mmc3P`) |
| 396 | `mapper_396.c` | 68 | 5 | 0 | done (batch 6, `MultiP`) |
| 397 | `mapper_397.c` | 69 | 1 | 0 | done (batch 6, `JyCompany` + `JyMulticart`) |
| 398 | `mapper_398.c` | 89 | 1 | 0 | done (batch 41, `VrcP`) |
| 399 | `mapper_399.c` | 79 | 1 | 0 | done (batch 13, `Mmc3P`) |
| 400 | `mapper_400.c` | 105 | 1 | 0 | done (batch 6, `MultiP`) |
| 401 | `mapper_401.c` | 97 | 1 | 0 | done (batch 13, `Mmc3P`) |
| 403 | `mapper_403.c` | 96 | 4 | 0 | done (batch 6, `MultiP`) |
| 404 | `mapper_404.c` | 71 | 1 | 0 | done (batch 11, `Mmc1Outer`) |
| 406 | `mapper_406.c` | 120 | 2 | 0 | done (batch 50, `Mmc3P`) |
| 409 | `mapper_409.c` | 56 | 1 | 0 | done (batch 6, `MultiP`) |
| 410 | `mapper_410.c` | 96 | 1 | 0 | done (batch 13, `Mmc3P`) |
| 411 | `mapper_411.c` | 103 | 7 | 0 | done (batch 13, `Mmc3P`) |
| 412 | `mapper_412.c` | 106 | 1 | 0 | done (batch 15, `Mmc3P`) |
| 413 | `mapper_413.c` | 135 | 1 | 0 | done (batch 49, `IrqP`) |
| 414 | `mapper_414.c` | 80 | 1 | 0 | done (batch 7, `MultiP`) |
| 415 | `mapper_415.c` | 69 | 1 | 0 | done (batch 7, `MultiP`) |
| 416 | `mapper_416.c` | 133 | 1 | 0 | done (batch 18, `IrqP`) |
| 417 | `mapper_417.c` | 131 | 1 | 0 | done (batch 18, `IrqP`) |
| 420 | `mapper_420.c` | 96 | 1 | 0 | done (batch 13, `Mmc3P`) |
| 421 | `mapper_421.c` | 72 | 1 | 0 | done (batch 7, `JyCompany` + `JyMulticart`) |
| 422 | `mapper_126.c` | 153 | 7 | 0 | done (batch 21, `Mmc3P`) |
| 428 | `mapper_428.c` | 95 | 6 | 0 | done (batch 7, `MultiP`) |
| 429 | `mapper_429.c` | 56 | 1 | 0 | done (batch 7, `MultiP`) |
| 431 | `mapper_431.c` | 81 | 1 | 0 | done (batch 7, `MultiP`) |
| 432 | `mapper_432.c` | 112 | 1 | 0 | done (batch 24, `Mmc3P`) |
| 433 | `mapper_433.c` | 75 | 1 | 0 | done (batch 8, `MultiP`) |
| 434 | `mapper_434.c` | 74 | 4 | 0 | done (batch 8, `MultiP`) |
| 436 | `mapper_436.c` | 59 | 1 | 0 | done (batch 53, `IrqP` + `one_bus.rs`) |
| 437 | `mapper_437.c` | 74 | 1 | 0 | done (batch 8, `MultiP`) |
| 438 | `mapper_438.c` | 71 | 2 | 0 | done (batch 8, `MultiP`) |
| 442 | `mapper_442.c` | 127 | 1 | 0 | done (batch 39, `IrqP`) |
| 446 | `mapper_446.c` | 746 | 4 | 0 | done (batch 55, `ChipSwitch`) |
| 447 | `mapper_447.c` | 98 | 1 | 0 | done (batch 16, `VrcP`) |
| 451 | `mapper_451.c` | 147 | 2 | 0 | done (batch 50, `Mmc3P`) |
| 452 | `mapper_452.c` | 119 | 1 | 0 | done (batch 40, `MultiP`) |
| 455 | `mapper_455.c` | 97 | 1 | 0 | done (batch 14, `Mmc3P`) |
| 456 | `mapper_456.c` | 83 | 1 | 0 | done (batch 14, `Mmc3P`) |
| 457 | `mapper_457.c` | 86 | 1 | 0 | done (batch 14, `Mmc3P`) |
| 471 | `mapper_471.c` | 109 | 0 | 0 | done (batch 33, `IrqP`) |
| 481 | `mapper_481.c` | 55 | 0 | 0 | done (batch 8, `MultiP`) |
| 512 | `mapper_512.c` | 94 | 2 | 1 | done (batch 47, `Mmc3P`) |
| 516 | `mapper_516.c` | 79 | 2 | 0 | done (batch 14, `Mmc3P`) |
| 517 | `mapper_517.c` | 107 | 1 | 1 | done (batch 1, `MultiP`) |
| 524 | `mapper_524.c` | 81 | 0 | 0 | done (batch 16, `VrcP`) |
| 525 | `mapper_525.c` | 91 | 1 | 0 | done (batch 10, `MultiP`) |
| 526 | `mapper_526.c` | 123 | 1 | 0 | done (batch 34, `IrqP`) |
| 527 | `mapper_527.c` | 61 | 0 | 0 | done (batch 27, `VrcP`) |
| 528 | `mapper_528.c` | 118 | 0 | 0 | done (batch 43, `IrqP`) |
| 532 | `mapper_532.c` | 411 | 0 | 0 | done (batch 59, UNIF loader + alias of 19 + `ersan2_overlay.rs`) |
| 534 | `mapper_126.c` | 153 | 10 | 0 | done (batch 21, `Mmc3P`) |
| 536 | `mapper_195.c` | 148 | 0 | 0 | done (batch 20, alias, `registry::board_number`) |
| 538 | `mapper_538.c` | 64 | 2 | 0 | done (batch 9, `MultiP`) |
| 539 | `mapper_539.c` | 113 | 1 | 0 | done (batch 35, `MultiP`) |
| 540 | `mapper_359.c` | 293 | 1 | 0 | done (batch 42, `IrqP`) |
| 541 | `mapper_541.c` | 70 | 1 | 0 | done (batch 9, `MultiP`) |
| 543 | `mapper_543.c` | 87 | 0 | 0 | done (batch 22, `Mmc1Outer`) |
| 547 | `mapper_547.c` | 241 | 8 | 4 | done (batch 51, `IrqP`) |
| 550 | `mapper_550.c` | 86 | 1 | 0 | done (batch 11, `Mmc1Outer`) |
| 551 | `mapper_178.c` | 109 | 4 | 0 | done (batch 19, `WaixingFs305`) |
| 554 | `mapper_554.c` | 94 | 1 | 0 | done (batch 10, `MultiP`) |
| 555 | `mapper_555.c` | 142 | 3 | 0 | done (batch 32, `Mmc3P`) |
| 556 | `mapper_556.c` | 178 | 1 | 0 | done (batch 45, `ChipSwitch`) |
| 557 | `mapper_557.c` | 58 | 1 | 0 | done (batch 31, `MultiP`) |
| 558 | `mapper_558.c` | 135 | 14 | 0 | done (batch 36, `IrqP`) |
| 559 | `mapper_559.c` | 108 | 1 | 0 | done (batch 30, `VrcP`) |
| 560 | `mapper_560.c` | 83 | 1 | 0 | done (batch 48, `IrqP`) |
| 561 | `mapper_561.c` | 276 | 203 | 0 | done (batch 52, `IrqP`) |
| 562 | `mapper_562.c` | 538 | 290 | 0 | done (batch 57, `IrqP` + `game_doctor.rs`, BIOS embedded by owner decision) |
| 564 | `mapper_564.c` | 77 | 0 | 0 | done (batch 9, `MultiP`) |

## Plan

1. Boards with a Hackset dump: 355, 389, 512, 517, 547.
2. The small boards by family (latch multicarts, MMC3 clones, VRC clones, discrete).
3. The large ones: 256 (OneBus/VT02-03, 241 db entries), 342 (CoolGirl), 446, 532, then 561/562
   (Bung/Venus Game Doctor copiers, 203/290 db entries: their disk images need a loader).

## Log

* Batch 1 (`multicart_p.rs`, `MultiP`, umbrella `board-multi-p`): 55, 81, 122, 237, 389, 517.
  Unit tests in `tests/mappers_p_tests.rs`; leads in FIDELITY-LEADS J37. The `additions` table of
  `mapper_tests.rs` is now kept in ascending order (the scratchpad helper sorts it after each
  registration).
* Batch 2: the J.Y. ASIC multicarts 281, 282 and 295 reuse `JyCompany` (`jy_company.rs`) with a
  `JyMulticart` decode of `$D003` (puNES replaces the same two bank functions); they join 209's
  latch lists in `cartridges/mod.rs`. `MultiP` gains 271, 335, 338, 341, 343, 352, 360 and 381
  (`MultiP::map_b2`); 352 and 381 count the console's resets through `Mapper::soft_reset`.
* Batch 3: `MultiP` 337, 350 (PRG-ROM bank 1 at `$6000`) and 375 (CHR-RAM write protection,
  `MultiP::ppu_write`); `JyMulticart::M358`. `MultiP::map` is split per batch (`map_b2`, `map_b3`)
  to stay under clippy's line limit.
* Batch 4: `MultiP` 354 and 380, the first two kinds that read the NES 2.0 submapper
  (`MultiP::submapper`): 354's latch window (`$F000`/`$E000`), 380's DIP read, last bank and
  mirroring bit.
* Perf, batches 1-2 (adf16776) against the MAPPERS-N tip e4596884, clean builds
  (`cleanroom/verify/rust-v2/cgp.sh`): mm2 core +0.000 %, total -0.001 %; zelda2 core +0.000 %,
  total -0.004 %.
* Batch 5: `MultiP` 382 (bus conflicts through `has_bus_conflicts`) and 390; `JyMulticart` 386,
  387 and 388 (388 carries mapper 90's nametable jumper, puNES `init_JYASIC(FALSE, ...)`).
* Batch 6: `MultiP` 396, 400 (bus conflicts; `$80` keeps the header's mirroring), 403 (registers
  at `$4100-$4103`), 409; `JyMulticart::M397`. 400's LED is not modelled (FIDELITY-LEADS J37).
* Batch 7: `MultiP` 414 and 431 (bus conflicts), 415 (8 KiB PRG-ROM at `$6000`), 428 (registers
  at `$6000`), 429; `JyMulticart::M421`. `MultiP::latch` split (`latch_late`) for clippy.
* Batch 8: `MultiP` 433 (CHR-RAM write enable), 434 and 437 (outer bank below `$8000`, bus
  conflicts above), 438, 481 (`latch_b8`, `map_b8`).
* Batch 9: `MultiP` 538 (FDS conversion, PRG-ROM at `$6000`), 541, 564. Deferred to their
  families: 557 (a Namcot 118 variant, with the MMC3-like boards) and 560 (CHR fetched through
  the nametable address, needs a PPU read hook).
* Batch 10: `MultiP` 525 (eight 1 KiB CHR registers, `MultiP::chr`, in the save state) and 554
  (latch moved by CPU **reads** of trigger addresses: its four ROM windows are `NO_ROM_WINDOW`,
  so every `$8000-$FFFF` read reaches `cpu_read_override` on this board alone).
* Perf, batches 1-10 (cacb0bbb) against e4596884: mm2 core +0.000 %, total -0.000 %; zelda2 core
  +0.000 %, total -0.002 %.
* Batch 11: `mmc1_outer.rs` (`Mmc1Outer`, boxed `DecodeBoard` variant, umbrella
  `board-mmc1-outer`), the `FaridSlrom` pattern: the MMC1 untouched, its windows rebuilt
  (`Mmc1::refresh_banks`) and narrowed after every change. 297 and 550 also have a latch mode
  that bypasses the chip; 374 counts resets. 543 (MMC1 with a serial outer register and banked
  WRAM) is still to do.
* Batch 12: `mmc3_p.rs` (`Mmc3P`, a boxed `IrqBoard` variant, umbrella `board-mmc3-p`), the
  `Mmc3Clone` shape with puNES's `prg_swap`/`chr_swap` hooks as `prg_page`/`chr_page`. puNES
  writes the RAM before the board (`cpu_inline.h`), so `$6000-$7FFF` outer registers leave the RAM
  its byte. The cfg lists naming `board-mmc3-clones` in `irq_boards.rs` and `mmc3.rs` now name
  `board-mmc3-p` too; the numbers join the two PPU-bus latch lists and the mf_3b sweep. 11 boards:
  267, 291, 315, 322, 327, 334, 339, 345, 361, 372, 377.
* Batch 13: `Mmc3P` 395, 399 (own PRG/CHR banks over the chip's IRQ), 401 (DIP lines at 0), 410,
  411 (registers at `$5000`), 420. The MMC3-P numbers moved out of the two PPU-bus lists into
  `mmc3_p_board` (`cartridges/mod.rs`), which both call, to keep them under clippy's line limit.
* Batch 14: `Mmc3P` 455 and 456 (registers at `$4100`), 457, 516 (A4-latched outer bank). 432
  and 344 (DIP reads over `$8000-$FFFF`) and 436 (OneBus) wait for their own step.
* Batch 15: `Mmc3P` 393 (UNROM latch on every chip write, CHR-RAM mode) and 412. 370 (CIRAM
  page per table from CHR bits, like `TxSROM`) and 512 (banked WRAM, CHR-RAM nametables) wait.
* Batch 16: `vrc_p.rs` (`VrcP`, boxed `IrqBoard` variant, umbrella `board-vrc-p`), the `Ax5705`
  shape: a `Vrc2Vrc4` untouched (`VRC2b` or `VRC4e` wiring), narrowed by `remap_*_banks` (384, 447
  with its NROM mode through the new `Vrc2Vrc4::prg_registers`) or with its own CPU-cycle IRQ
  (308, 524). The four numbers join `clocks_on_cpu_cycle`. 362 and 398 (PRG switched by CHR
  fetches), 527 (CIRAM pages from CHR bit 7), 528 and 559 remain.
* Batch 17: `MultiP` 340 (CHR-RAM write protection in its NROM modes); `map_b9` split off.
* Batch 18: `irq_p.rs` (`IrqP`, boxed `IrqBoard` variant, umbrella `board-irq-p`): the *SMB2J*
  conversions 311, 357, 368, 416 and the *Batman* pirate 417 (CIRAM page per table, in
  `nametable_pages_vary`; its free-running counter is in the mf_5a sweep). All five join
  `clocks_on_cpu_cycle`.
* Batch 19: 551 on `WaixingFs305` (mapper 178's board), whose PRG decode is puNES's `prg_fix_178`
  exactly; a load-time `chr_banked` flag moves `$4803` to the CHR bank and the ports to the
  `$FF00` decode puNES gives 551. Mapper 178's own path is unchanged apart from the flag's test.
* Batch 20: `registry::board_number` — puNES dispatches 248 to `map_init_115` and 536 to
  `map_init_195`; `Cartridge::create` builds the board, its memory sizes, bus conflicts and latch
  lists from that number and keeps the header's as `mapper_number`. The mf_3b sweep lists both.
* Batch 21: 422 and 534 in `Mmc3P`, puNES's own `mapper_126.c` model (Zaxnes's 126 follows
  Mesen2's `MMC3_126.h` and is left as it is). 534 inverts `$C000`/`$C001` data.
* Batch 22: `Mmc1Outer` 543 (outer register shifted in through `$5000` D3, WRAM bank from it and the MMC1's `$A000` D3, power-on `$E000` = `$0E` via `Mmc1::force_prg_register`).
* Batch 23: `Mmc3P` 370 (CIRAM page per table from R0-R5 D7 in mode 1). The `cfg(any(...))` lists of `IrqBoard`'s forwards (`nametable_page`, `try_read_prg`, `prg_ram_enabled`) now name `board-irq-p`/`board-mmc3-p` too: the default build already compiled those forwards through other features, but a build with only these boards on would have dropped them. Both 370 and 417 have a page test.
* Batch 24: `Mmc3P` 344 and 432, whose DIP / solder-pad read replaces `$8000-$FFFF`:
  `Mmc3P::rom_windows` answers `NO_ROM_WINDOW` while it is on, so the read reaches
  `try_read_prg`. `write_low`'s tail moved to `after_low_write`.
* Batch 25: `Mmc3P` 356 (410's registers, CHR-RAM while `reg[2]` D5 is clear, four-screen from D6; `Mmc3P::a000` keeps the chip's last `$A000` to return to, in the save state).
* Batch 26: `Mmc3P` 269. Without CHR-ROM its CHR is the PRG-ROM with the data lines permuted; the copy lives in the board (`Mmc3P::chr_269`, derived at load) because the core clears the CHR-RAM after a board's power-on, and `ppu_peek`/`ppu_write` serve it as ROM. The CHR-RAM is sized to the PRG-ROM (`board_memory_sizes`) so that the windows wrap on it. `020-FFF` registers moved to `Mmc3P::write_5000`.
* Batch 27: `VrcP` 527 (VRC2b, CIRAM page from D7 of CHR registers 0/1 through the new `Vrc2Vrc4::chr_register`; `IrqBoard::nametable_page` names `board-vrc-p`).
* Batch 28: `Mmc3P` 353 (four games selected by a chip write to `$x080`).
* Perf, batches 1-28 (f2989b02) against e4596884: mm2 core +0.000 %, total -0.001 %; zelda2 core
  +0.000 %, total -0.006 %.
* Batch 29: `Mmc3P` 100 (Nesticle's raw bank registers over the chip's IRQ and mirroring;
  `Mmc3P::raw_100`, saved for that kind only). 186 was written and taken back out: the Fukutake
  Study Box is refused on purpose (`cartridge_tests::req_core_102`, MF-6 inventory §3.5 and
  AMD-MAPPERS-N-200 say it stays refused although FCEUX and puNES run it). Left for the owner to
  decide; puNES's model (registers `$4200-$4203`, `$4202` reads `$40`, RAM at `$4400-$4FFF` and a
  banked 8 KiB at `$6000`) is in the session notes if the decision changes.
* Batch 30: `VrcP` 559 (a VRC4 indexed by A10/A11 through the new `Vrc2Vrc4::new_vrc4_lines`;
  `$9C00` the `$C000` bank, `$9C04-$9C07` four nametable registers in `VrcP::pages`, saved for
  that kind only, and in `nametable_pages_vary`; high-nibble registers take D4-D7).
* Batch 31: `MultiP` 557 (an FDS conversion on a Namcot 118 clone: the 118's eight registers kept
  in `MultiP::chr`, already in the save state; PRG `& $0F` with the fixed pair at `$0E`/`$0F`,
  CHR fixed, R5 D5 the mirroring). Kept off `Namcot108`, whose layout is a measured hot path.
* Batch 32: `Mmc3P` 555 (*Nintendo Campus Challenge 1991*): outer registers at `$5800`/`$5C00`,
  RAM at `$5000-$57FF` (the work RAM's 4 KiB bank 2), the competition timer on `on_cpu_cycle`
  (in `clocks_on_cpu_cycle`; `timer_555`/`expired_555` saved for that kind only) and the mode
  whose CHR registers with D6 select 1 KiB windows of an 8 KiB CHR-RAM beside the CHR-ROM
  (`ram_windows`, derived; `ppu_peek`/`ppu_write` serve them, `chr_peek_view` is `None`).
* Batch 33: `IrqP` 471 (the written address is the bank; an IRQ on every A12 rise through the
  MMC3's `A12Watcher` filter, acknowledged by any write). In both PPU-bus lists and the mf_3b
  sweep, not in `clocks_on_cpu_cycle`; the watcher's state is saved for that kind only.
* Batch 34: `IrqP` 526 (four PRG and eight CHR registers, a free-running 16-bit CPU-cycle
  counter asserting while bit 12 is set; in `clocks_on_cpu_cycle` and in the mf_5a sweep, since
  it counts from power-on).
* Batch 35: `MultiP` 539 (*Kid Icarus* FDS conversion): fixed banks with RAM pieces laid over
  `$6000-$65FF`, `$8200-$82FF`, `$C000-$D1FF` and `$DF00-$DFFF` (`ram_offset_539`); windows 0
  and 2 leave the ROM cache. Its CHR-RAM power-on pattern is set in
  `Cartridge::clear_volatile_ram` behind `board-539`, since the core clears that RAM after a
  board's power-on. `read_6000`'s 8 KiB ROM reads now share `MultiP::rom_8k`.
* Batch 36: `IrqP` 558 (Yancheng YC-03-09): 32 KiB banks with the D0/D1 swap, and the
  existing `Eeprom93C66` (module now also behind `board-irq-p`) when the NES 2.0 header declares
  512 bytes of PRG-NVRAM, as puNES decides; its bytes follow 8 KiB of work RAM
  (`board_memory_sizes`, `battery_offset`). Not in `clocks_on_cpu_cycle` (no IRQ).
  532 (*China Ersan 2*) is blocked: puNES runs mapper 19 plus a text overlay drawn from a font
  file the user supplies separately (`chinaersan2.font`), which no image carries.
* Perf, batches 1-36 (54ebf500) against e4596884: mm2 core +0.000 %, total +0.000 %; zelda2 core
  +0.000 %, total -0.004 %.
* Batch 37: `IrqP` 272 (J.Y. *Akumajō Special* bootleg): two decodes per write, nibble CHR
  registers, and an IRQ stepped by each low-plane pattern fetch (in `watches_ppu_reads` only;
  absent from the mf_3b expectations, since its counter moves only once enabled).
* Batch 38: `MultiP` 347 (Kaiser KS-7030, *Doki Doki Panic* conversion): 1 KiB-grained pieces
  of PRG-ROM and an 8 KiB RAM over `$6000-$7FFF`, `$B800-$BFFF` and `$C000-$D7FF`
  (`source_347`); the dump with PRG CRC `$FA4DAC91` keeps the pre-2020 bank order, as puNES
  checks; both registers return to `$0F` on a soft reset. 355 (*3D Block*) is blocked: its
  board runs a PIC16C5x microcontroller program (puNES embeds the EPROM and a PIC core), which
  is a CPU emulator of its own.
* Batch 39: `IrqP` 442 (a PEC-9588 relative): `$5000` the 32 KiB bank and the 1 bpp mode, PPU
  A0/A9 latched on the rise of A13 in `on_ppu_address_bus` (both PPU-bus lists and the mf_3b
  sweep, as mapper 164) and substituted for CHR A3/A12 in `ppu_peek`; `chr_peek_view` is `None`
  for it.
* Batch 40: `MultiP` 452 (DS-9-27): a write to ROM latches address and data (438's arm); the
  8 KiB work RAM covers one or two `$8000-$FFFF` windows (`ram_windows_452`), which leave the ROM
  cache and take writes as RAM; submapper 1's top-nibble decode as puNES has it.
* Fix (after batch 40): `IrqBoard` did not forward `Mapper::rom_windows`, so `Mmc3P`'s override
  (batch 24) was never reached and the CPU's cached path (`Cartridge::read_rom_window`) read ROM
  where 344 and 432 answer their DIP / solder-pad read; the tests read through `peek`, which
  goes to `try_read_prg`, and missed it. The forward is now there behind
  `board-mmc3-p`/`board-vrc-p`, out of line, and both tests check `read_rom_window` too.
* Batch 41: `VrcP` 362 and 398 (VRC4f, `new_vrc4f` now also behind `board-vrc-p`): the outer
  banks follow D7-D8 (362) or D0-D2 (398) of the CHR register of the window the PPU last
  fetched from, through `on_ppu_address_bus` (`watches_ppu_reads` and the mf_3b sweep; the
  notification comes before the read, as puNES switches before `chr_rd`). While that holds,
  `rom_windows` answers `NO_ROM_WINDOW`, now reached through the `IrqBoard` forward. 362 toggles
  to its second game on a reset of an image above 512 KiB (`soft_reset`); 398 latches the low
  address byte of every write.
* Batch 42: `IrqP` 359 and 540 (BS-400R; 540 with 2 KiB CHR banks): outer PRG/CHR banks and
  masks, PRG register 3 at `$6000`, and the IRQ in either mode: CPU cycles on a 16-bit counter,
  or A12 rises past puNES's own five-CPU-cycle filter (`ext_359[6]`, counted down in
  `on_cpu_cycle`), raw rises seen in `on_ppu_address_bus` (both PPU-bus lists, the mf_3b sweep,
  `clocks_on_cpu_cycle`). puNES's `map_init_359` sets `prg[2]` twice and leaves `prg[3]` 0;
  kept. The mf_3b list's comments were regrouped (rustfmt had moved them onto the previous
  numbers).
* Batch 43: `IrqP` 528 (1201, *Gimmick!* bootleg): FME-7 commands and data written together at
  `$A000`/`$C000`, A14 the outer bank `$10`, command 8 = 1 the work RAM at `$6000`; commands
  `$D`-`$F` drive Zaxnes's own `VrcIrq` (control, acknowledge, latch), stepped in `on_cpu_cycle`.
* Batch 44: `Mmc3P` 369 (N-49): `$4100` picks NROM banks, the *SMB2J* layout (ROM `$0E` at
  `$6000`, a 12-bit CPU-cycle IRQ in `outer[2]`/`outer[3]`, the chip's A12 counter idle) or the
  MMC3 at `$10`/`$20`; in `clocks_on_cpu_cycle` and still in both PPU-bus lists.
* Batch 45: `chip_switch.rs` (`ChipSwitch`, boxed `IrqBoard` variant, umbrella
  `board-chip-switch`): boards that hold two whole chips and route writes, hooks and the IRQ
  line to the one in use, narrowing its windows to the outer bank afterwards. 556: `Mmc3` or a
  VRC4 indexed by A0|A2 and A1|A3 (the new `Vrc2Vrc4::new_vrc4_or_lines`), A2/A3 swapped above
  A11. In `clocks_on_cpu_cycle` (VRC4), both PPU-bus lists and the mf_3b sweep (MMC3). The
  `cfg` of `Vrc2Vrc4::new_vrc4_wired` had been left on `new_vrc4_lines` by batch 30's insertion;
  restored.
* Batch 46: `ChipSwitch` 351 (Techline XB): MMC3, MMC1 (`Mmc1::refresh_banks` now also behind
  `board-chip-switch`) or VRC4e by `$5000` D0-D1, the outer bank through each chip's own units,
  the NROM and 8 KiB CHR-ROM modes of `$5002` over them, and the DIP read at `$5000-$5FFF`.
* Batch 47: `Mmc3P` 512 (SL-12): `$4100` D1 puts every CHR window on an 8 KiB CHR-RAM beside the
  CHR-ROM (555's `ram_windows`, `board_memory_sizes`), and `$4100` = 1 four-screen tables in that
  RAM's second 4 KiB, served by the board (`nametable_page` = `SERVED_BY_CARTRIDGE`,
  `read_nametable`/`write_nametable`, now forwarded by `IrqBoard` behind `board-512`; in
  `nametable_pages_vary`). The decision is taken where the chip's mirroring is settled.
* Perf, batches 1-46 (52c21a15) against e4596884, with smb3 added (an MMC3 in `IrqBoard`, for the
  `rom_windows` forward): mm2 core +0.000 %, total +0.000 %; zelda2 core +0.000 %, total
  -0.006 %; smb3 core +0.000 %, total +0.018 % (all of it in the `convert` and `lua_hooks`
  stages, +2.75 M each, outside the core). Script: `cleanroom/verify/rust-v2/cgs.sh`.
* Batch 48: `IrqP` 560: the board serves the console's two nametables itself (by the header's
  mirroring, `SERVED_BY_CARTRIDGE`) so that it sees the writes to tables 1 and 3 it copies to its
  1 KiB extension RAM; the nametable address latched on A13's rise (`watches_ppu_reads`, the
  mf_3b sweep) picks that RAM's byte as the CHR bank of the next pattern fetch in bank 1.
* Batch 49: `IrqP` 413: four bank registers loaded by D6-D7 of a `$E000` write, an MMC3-style
  A12 counter (both PPU-bus lists, mf_3b), and the serial read-out of the NES 2.0 misc ROM (the
  image past its CHR-ROM, `RomImage::raw`) at `$4800-$4FFF`/`$C000-$CFFF` through
  `cpu_read_override` (advancing) and `cpu_peek_override`, both now forwarded by `IrqBoard`
  behind `board-413`, as is `rom_windows` (`$C000` window off the cache).
* Batch 50: `Mmc3P` 406 and 451, MMC3 boards whose PRG-ROM is a flash chip: puNES's
  `SST39SF040.c` command machine in the board (unlock addresses, IDs, software-ID mode, program,
  sector and chip erase with the busy toggle), programs and erases going through
  `CartridgeMemory::flash_program`/`flash_erase_sector` (their `cfg` and `FlashRom`'s widened to
  `board-406`/`board-451`) so the changed sectors are the save file (`flash_sector: 64 KiB` in
  `board_memory_sizes`); the windows leave the cache while the chip answers. 406 crosses A0/A1
  (submapper 0) or A13-A14 (submapper 1) on the way to the MMC3 and has no WRAM; 451 feeds the
  MMC3 from the address lines and banks from `$E000` A0-A1. Both in `clocks_on_cpu_cycle`.
* Batch 51: `IrqP` 547 (Konami QTa, *Q-Tai*): registers at `$D000-$DFFF` (window 2 off the
  cache for the `$DC00`/`$DD00` JIS-to-tile lookup), 4 KiB work-RAM windows (`try_read_prg` and
  `intercept_cpu_write`), a 16-bit CPU-cycle IRQ, and the QT RAM: the board serves the console's
  tables and a 2 KiB QT RAM beside them (`$DA00` D0 routes nametable writes there), a tile fetch
  latches the QT byte (`watches_ppu_reads`), and the next background pattern fetch reads the
  kanji CHR-ROM (D6, plane 1 from D7) or a CHR-RAM bank (D0); sprites see the 8 KiB CHR-RAM
  (`board_memory_sizes`, `IrqP::ppu_write`).
* Batch 52: `IrqP` 561 (Bung Super Game Doctor / Venus Turbo Game Doctor format): PRG and CHR in
  the board's own copier RAM (loaded from the image, 256 KiB PRG on submapper 3, saved in the
  state), writable while the copier mode says so; the eight bank modes of `prg_fix_561` as 8 KiB
  banks; both timers on `on_cpu_cycle` (in the mf_5a sweep, the FDS-style one runs from
  power-on). New core hook `Mapper::boot_program` (forwarded by `Board`, and by `IrqBoard` behind
  `board-561`), asked by `NesCore::power_on` after the CPU reset: the misc ROM's load address,
  data and init, run through a `JSR init` / `JMP ($FFFC)` stub at `$0700`. 562 stays blocked: its
  puNES file carries the copier's BIOS as a firmware dump.
* Batch 53: 256 and 436, the V.R. Technology OneBus famiclones (VT02/VT03), as `IrqP` kinds over a
  new `one_bus.rs` holding the chip (`OneBus.c`): PRG from `$4107-$410B` with `$4100`/`$410A`
  blocks, CHR 1 KiB windows from `$2012-$2017`/`$2018`/`$201A` over the CHR-ROM (or the PRG-ROM
  without one), the MMC3 register file at `$8000-$FFFF` as an alias, the A12 counter, and the PCM
  channel. 256's submapper wirings (`regs[]` of `mapper_256.c`, CRC detection on iNES 1.0) and
  436's outer banks. Core hooks, all behind `board-256`/`board-436`: `NesBus` routes every CPU
  write to `$2008-$3FFF` to the cartridge instead of the PPU (`vt_bus`), passes the APU writes
  through `Mapper::snoop_apu_write` (which may clear `$4015` D4) and the `$4015` read through
  `Mapper::apu_status`, and treats the board as an expansion-audio one so that
  `push_expansion_audio` hands the APU the PCM's `$4011` byte (`Mapper::take_dac_write`). New
  `irq_p_a12_board` list (471, 413, 256, 436, 359, 540) shared by both PPU-bus lists.
* Batch 54: 394 in `JyCompany` with a boxed `Jy394` board (behind `board-394`): `$5000-$5003`
  (A0-A1) hold the outer PRG/CHR bank and the chip select; `$5001` D4 clear hands every `Mapper`
  method `JyCompany` answers (banks, IRQ, PRG reads and RAM, PPU bus, nametable page, CHR writes,
  register writes) to an embedded `Mmc3`, whose windows go through the outer bank and the NROM
  mode (`Mmc3::refresh_banks` widened to `board-394`); with D4 set the ASIC's own `prg_bank`,
  `chr_bank` and ROM-nametable decodes take the outer bank. In the four latch lists and the mf_3b
  sweep with 209.
* Perf after batch 53 (ab6a49fb, the OneBus bus hooks) against e4596884: mm2 core +0.003 %,
  zelda2 core +0.002 %, smb3 core +0.002 % and total +0.002 %.
* Batch 55: 446 (Mindkids SMD172B) as a `ChipSwitch` kind with a boxed `Mindkids` board: the
  eight `$5000-$5007` registers until `$5000` D7 locks a personality (submapper 0's mode-1
  quirk), the discrete modes (UNROM, CNROM, ANROM, GNROM, NROM, the Bandai latch) in the board's
  own windows, the MMC1/MMC3 (TLSROM, 189 through `$6000`)/VRC2/VRC4/VRC6 chips through the outer
  bank, `$5005` D2's CHR write protect, and the flash chip of the unlocked menu. The JEDEC command
  machine of 406/451 moved to `cartridges/jedec_flash.rs` (`JedecFlash`, `JedecChip`) and is shared;
  `FlashRom`'s and `CartridgeMemory`'s flash `cfg`s take `board-446`. `IrqBoard` forwards
  `rom_windows` and `expansion_audio_output` for it; 446 is in both PPU-bus lists (MMC3) and in
  `nametable_pages_vary` (VRC6, MMC3).
* Batch 56: 342 (COOLGIRL) as an `IrqP` kind over a new `cool_girl.rs` holding the FPGA
  (`mapper_342.c`, after FCEUX `m342.cpp`): the `$5000-$5007` configuration until `$5007` D7
  locks it, the 1 GiB PRG bank space through `prg.base`/`prg.mask` and modes 0-7, the CHR-RAM
  modes 0-7 through `chr.mask` (mapper 24's halved banks), `$6000` as work RAM (four 8 KiB pages)
  or a ROM bank, the personalities 1-37 (UxROM, CNROM, 78, 97, 93, 18, AxROM/BNROM, 228, Color
  Dreams, GxROM, 87, 90, 65, MMC5 subset, MMC1, MMC2/MMC4, 152, VRC3, MMC3/TxSROM/189, 112,
  33/48, 42, VRC2/VRC4, FME-7, G-101, 79/146, 133, 36, 70, 184, 38, VRC1, 83, 67, 89, 163) with
  their CPU-cycle IRQs, the MMC2 latches, the mapper-189 CIRAM pages, and the NOR flash command
  set (sector erase, byte program, CFI table) over the save area, the top 8 MiB of the bank
  space, kept by 8 KiB page as written. Reads go through `cpu_read_override` (the `$5204`
  acknowledge); `IrqBoard` forwards that override for `board-342`. 342 is in `irq_p_a12_board`,
  `nametable_pages_vary` and the mf_3b sweep. The MAPPERS-P list is done: 133 of the 135
  numbers, 186 refused, 355, 532 and 562 blocked.
* Perf after batch 56 (1cac40fd) against e4596884: mm2 core +0.003 %, total +0.002 %; zelda2
  core +0.002 %, total +0.003 %; smb3 core +0.002 %, total +0.014 %. smb3's `convert` (+0.150 %)
  and `lua_hooks` (+0.238 %) both grew by the same 2 020 920 instructions, outside any board
  code: a shared function's layout, to recheck if it persists.
* Owner decisions (2026-11-15): 355 option A (the PIC's EPROM embedded as puNES does, with a
  port of the PIC16C5x core, MAME code under BSD-3); 532 yes (a UNIF loader, the font read from
  the image's `FONT` chunk); 562 option A (the copier's BIOS embedded as puNES does). 186: the
  owner asked whether Mesen reads the tapes; it does (`.studybox`/`STBX` files, a user-supplied
  `StudyBox.bin`), answer pending.
* Batch 57: 562 (Venus Turbo Game Doctor) as an `IrqP` kind over a new `game_doctor.rs`: the
  copier DRAM loaded from the image's PRG and CHR, the registers at `$4024/$4025` (FDS-style
  timer), `$42FC-$42FF` (mode 1, mirroring, latch on/off, CHR lock), `$43FC-$43FF`, `$4400-$4407`
  (1 KiB CHR), `$440C/$440D` (16-bit counter), `$4411` (TGD mode) and their read-backs (`$4420`
  the last CHR window fetched), the eight latch PRG modes, DRAM writes at `$8000+` while the
  latch is off, and the misc-ROM boot shared with 561. The BIOS is embedded as puNES carries it
  (owner decision, option A): `data/vtgd_bios.bin`, the 2 KiB `bios[]` holds (puNES declares
  4 KiB and reads it through `& $7FF`). In both PPU-bus lists (the CHR window latch), the mf_3b
  and mf_5a sweeps.
* Batch 58: 355 (Hwang Shinwei *3D Block*, *Block Force*) as an `IrqP` kind: NROM-256 with a
  PIC16C54 in a new `pic16c54.rs`, a port of MAME's PIC16C5x core (BSD-3, ver 1.16, as puNES
  carries it; notice in `THIRD-PARTY-NOTICES.md`) limited to that model. The PIC runs one
  instruction cycle per four CPU cycles and reads the last CPU address (`$4020+`, reads and
  writes) on its ports: A6-A4 on port A D1-D3, A12/A7/A10/A11/A9/A8/A13/A14 on port B. Port A
  writes drive the IRQ (`pic16c5x_wr`: D0 or a `tris` write releases it). The program is the
  image's single 1 KiB NES 2.0 misc ROM, else the EPROM puNES carries for the PRG CRC
  (owner decision, option A: `data/pic355_3d_block.bin`, `data/pic355_block_force.bin`). 355's
  ROM windows are off the read cache, or the PIC would miss the CPU's fetches: the game runs its
  wait loops at `$E180-$E1FF`, which the PIC recognises (port B `$E2`) before raising the IRQ
  that splits the screen. Checked on the Hackset `[!]` dump (its misc ROM equals puNES's
  EPROM): the 3D well and the falling pieces draw, where the same PRG as mapper 0 repeats the
  top half of the screen.
* Batch 59: 532 (*China Ersan 2*). Correction of batch 36: puNES reads the font from the image's
  own UNIF `FONT` chunk, not from a separate file; what was missing was a UNIF loader. New
  `rom/unif.rs` rewrites a UNIF image as the NES 2.0 image it describes before the header is
  parsed (puNES `unif.c`: `MAPR` with its prefix stripping and puNES's 167-name board table,
  `PRG0-F`/`CHR0-F`, `MIRR`, `BATR`, `TVCI`; `FONT` becomes the misc ROM; ROMs padded by
  repetition to whole 16/8 KiB banks). 532 is an alias of 19 (`registry::board_number`, the
  Namco 163) and `NesCore` paints puNES's `chinaersan2_apply_font` over the 2D picture
  (`ersan2_overlay.rs`), with CPU RAM `$00-$FF` taken at the pre-render line: only for a 532
  image with a font, `run_frame` steps the CPU with the PPU in sync up to that line, so other
  games pay one test a frame. New `Cartridge::board_number`; the power-on fill of battery RAM
  now asks it, so 532 keeps the Namco 163's internal RAM at zero as 19 does (a savestate round
  trip found it). The app's ROM dialog and handheld list accept `.unf`/`.unif`. Mesen 2 notes
  that no such board is known (a VirtuaNES convention); there is no dump in the Hackset to check
  against, so only the synthetic tests cover it.
* Batch 60: 186, the Fukutake Study Box, on the owner's go (2026-11-16) after Mesen 2's model:
  the game is a `.studybox` tape (`STBX`: decoded `PAGE`s with their lead-in and data positions
  in the audio, and an `AUDI` WAV) run with the user's `StudyBox.bin` (256 KiB). The new
  `study_box.rs` packages them as a NES 2.0 image (`zax_nes::rom::package_study_box`: mapper
  186, the BIOS as PRG-ROM, 64 KiB of work RAM, 8 KiB of CHR-RAM, four-screen, the tape as the
  misc ROM) and runs `StudyBox.h` as an `IrqP` kind: `$4200` (work-RAM pages), `$4201` (the
  `$8000` BIOS page), `$4202` (the drive: commands clocked on D4's falling edge, reset on D5's,
  decoder and IRQ enables), the seek (3 000 000 cycles a page), the lead-in, the bytes every
  3 355 cycles and their IRQ, and the tape's audio played from the data's start as the
  expansion audio. The tape is read in place in the image. `Cartridge::create` accepts 186 only
  when its misc ROM is a tape (`registry::is_study_box`), so `req_core_102`'s bare 186 is still
  refused and 186 stays out of `SUPPORTED_MAPPERS`. The app finds `StudyBox.bin` beside the tape
  or beside the executable. All 135 numbers are covered.
* Perf, the whole lot against e4596884. After batch 59 (5c5b8de2): mm2 core +0.059 %, zelda2
  core +0.062 %, smb3 core +0.048 %. A bisect put all of it on batch 59 (562 and 355 alone:
  +0.003 %), on the 532 frame hooks inlined into `run_frame` and the 256-byte snapshot on its
  frame. With them cold, out of line and the snapshot boxed (a8a846bd): mm2 core +0.003 % and
  total +0.001 %, zelda2 core +0.002 % and total +0.002 %, smb3 core +0.002 % and total +0.007 %.
* Flash saves kept on disk (owner, 2026-11-16: "il faut que ce point marche"). The core already
  wrote a flash chip's changed sectors as a save file (`export/import_flash_save`, MF-UNL-9) but
  no host called it, so a game that saves in its flash (111, 406, 451, 446, 682) lost it when it
  closed. The app now keeps it in `<rom>.flash` beside the `.srm` (`BatteryRamStore::for_flash`):
  read before the first frame (not over a savestate, which carries its own), written every 30 s
  when it changed and when the game stops; the export is assembled only when a write is due.
  The COOLGIRL's save area goes through the same calls (`Mapper::export_save_area`, file `ZXCG`
  with the written pages). Tests: `a_flash_save_survives_a_restart` (a 342 program writes its
  save area, the file appears and comes back) and `mappers_p_342_save_area_survives_a_new_session`.
* The app's full suite (with `bundled`) had five failures left by MAPPERS-N (its VS coins,
  service, DIP switches and Rainbow network option had not reached the command, action, binding
  and option-tree tests and goldens); updated, the suite passes (863 tests).

