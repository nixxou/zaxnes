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
| 186 | `mapper_186.c` | 82 | 1 | 0 | refused (MF-6 inventory §3.5, AMD-MAPPERS-N-200: the Study Box stays refused) |
| 237 | `mapper_237.c` | 98 | 1 | 0 | done (batch 1, `MultiP`) |
| 248 | `mapper_115.c` | 115 | 0 | 0 | done (batch 20, alias, `registry::board_number`) |
| 256 | `mapper_256.c` | 123 | 241 | 0 | todo |
| 267 | `mapper_267.c` | 87 | 1 | 0 | done (batch 12, `Mmc3P`) |
| 269 | `mapper_269.c` | 130 | 3 | 0 | done (batch 26, `Mmc3P`) |
| 271 | `mapper_271.c` | 69 | 1 | 0 | done (batch 2, `MultiP`) |
| 272 | `mapper_272.c` | 160 | 1 | 0 | todo |
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
| 342 | `mapper_342.c` | 2381 | 9 | 0 | todo |
| 343 | `mapper_343.c` | 71 | 0 | 0 | done (batch 2, `MultiP`) |
| 344 | `mapper_344.c` | 125 | 3 | 0 | done (batch 24, `Mmc3P`) |
| 345 | `mapper_345.c` | 102 | 0 | 0 | done (batch 12, `Mmc3P`) |
| 347 | `mapper_347.c` | 122 | 2 | 0 | todo |
| 350 | `mapper_350.c` | 99 | 0 | 0 | done (batch 3, `MultiP`) |
| 351 | `mapper_351.c` | 305 | 0 | 0 | todo |
| 352 | `mapper_352.c` | 66 | 0 | 0 | done (batch 2, `MultiP`) |
| 353 | `mapper_353.c` | 149 | 1 | 0 | done (batch 28, `Mmc3P`) |
| 354 | `mapper_354.c` | 111 | 0 | 0 | done (batch 4, `MultiP`) |
| 355 | `mapper_355.c` | 258 | 4 | 1 | todo |
| 356 | `mapper_356.c` | 106 | 1 | 0 | done (batch 25, `Mmc3P`) |
| 357 | `mapper_357.c` | 118 | 1 | 0 | done (batch 18, `IrqP`) |
| 358 | `mapper_358.c` | 76 | 4 | 0 | done (batch 3, `JyCompany` + `JyMulticart`) |
| 359 | `mapper_359.c` | 293 | 3 | 0 | todo |
| 360 | `mapper_360.c` | 54 | 1 | 0 | done (batch 2, `MultiP`) |
| 361 | `mapper_361.c` | 85 | 5 | 0 | done (batch 12, `Mmc3P`) |
| 362 | `mapper_362.c` | 99 | 2 | 0 | todo |
| 368 | `mapper_368.c` | 106 | 2 | 0 | done (batch 18, `IrqP`) |
| 369 | `mapper_369.c` | 221 | 1 | 0 | todo |
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
| 394 | `mapper_394.c` | 193 | 0 | 0 | todo |
| 395 | `mapper_395.c` | 88 | 4 | 0 | done (batch 13, `Mmc3P`) |
| 396 | `mapper_396.c` | 68 | 5 | 0 | done (batch 6, `MultiP`) |
| 397 | `mapper_397.c` | 69 | 1 | 0 | done (batch 6, `JyCompany` + `JyMulticart`) |
| 398 | `mapper_398.c` | 89 | 1 | 0 | todo |
| 399 | `mapper_399.c` | 79 | 1 | 0 | done (batch 13, `Mmc3P`) |
| 400 | `mapper_400.c` | 105 | 1 | 0 | done (batch 6, `MultiP`) |
| 401 | `mapper_401.c` | 97 | 1 | 0 | done (batch 13, `Mmc3P`) |
| 403 | `mapper_403.c` | 96 | 4 | 0 | done (batch 6, `MultiP`) |
| 404 | `mapper_404.c` | 71 | 1 | 0 | done (batch 11, `Mmc1Outer`) |
| 406 | `mapper_406.c` | 120 | 2 | 0 | todo |
| 409 | `mapper_409.c` | 56 | 1 | 0 | done (batch 6, `MultiP`) |
| 410 | `mapper_410.c` | 96 | 1 | 0 | done (batch 13, `Mmc3P`) |
| 411 | `mapper_411.c` | 103 | 7 | 0 | done (batch 13, `Mmc3P`) |
| 412 | `mapper_412.c` | 106 | 1 | 0 | done (batch 15, `Mmc3P`) |
| 413 | `mapper_413.c` | 135 | 1 | 0 | todo |
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
| 436 | `mapper_436.c` | 59 | 1 | 0 | todo |
| 437 | `mapper_437.c` | 74 | 1 | 0 | done (batch 8, `MultiP`) |
| 438 | `mapper_438.c` | 71 | 2 | 0 | done (batch 8, `MultiP`) |
| 442 | `mapper_442.c` | 127 | 1 | 0 | todo |
| 446 | `mapper_446.c` | 746 | 4 | 0 | todo |
| 447 | `mapper_447.c` | 98 | 1 | 0 | done (batch 16, `VrcP`) |
| 451 | `mapper_451.c` | 147 | 2 | 0 | todo |
| 452 | `mapper_452.c` | 119 | 1 | 0 | todo |
| 455 | `mapper_455.c` | 97 | 1 | 0 | done (batch 14, `Mmc3P`) |
| 456 | `mapper_456.c` | 83 | 1 | 0 | done (batch 14, `Mmc3P`) |
| 457 | `mapper_457.c` | 86 | 1 | 0 | done (batch 14, `Mmc3P`) |
| 471 | `mapper_471.c` | 109 | 0 | 0 | done (batch 33, `IrqP`) |
| 481 | `mapper_481.c` | 55 | 0 | 0 | done (batch 8, `MultiP`) |
| 512 | `mapper_512.c` | 94 | 2 | 1 | todo |
| 516 | `mapper_516.c` | 79 | 2 | 0 | done (batch 14, `Mmc3P`) |
| 517 | `mapper_517.c` | 107 | 1 | 1 | done (batch 1, `MultiP`) |
| 524 | `mapper_524.c` | 81 | 0 | 0 | done (batch 16, `VrcP`) |
| 525 | `mapper_525.c` | 91 | 1 | 0 | done (batch 10, `MultiP`) |
| 526 | `mapper_526.c` | 123 | 1 | 0 | done (batch 34, `IrqP`) |
| 527 | `mapper_527.c` | 61 | 0 | 0 | done (batch 27, `VrcP`) |
| 528 | `mapper_528.c` | 118 | 0 | 0 | todo |
| 532 | `mapper_532.c` | 411 | 0 | 0 | todo |
| 534 | `mapper_126.c` | 153 | 10 | 0 | done (batch 21, `Mmc3P`) |
| 536 | `mapper_195.c` | 148 | 0 | 0 | done (batch 20, alias, `registry::board_number`) |
| 538 | `mapper_538.c` | 64 | 2 | 0 | done (batch 9, `MultiP`) |
| 539 | `mapper_539.c` | 113 | 1 | 0 | done (batch 35, `MultiP`) |
| 540 | `mapper_359.c` | 293 | 1 | 0 | todo |
| 541 | `mapper_541.c` | 70 | 1 | 0 | done (batch 9, `MultiP`) |
| 543 | `mapper_543.c` | 87 | 0 | 0 | done (batch 22, `Mmc1Outer`) |
| 547 | `mapper_547.c` | 241 | 8 | 4 | todo |
| 550 | `mapper_550.c` | 86 | 1 | 0 | done (batch 11, `Mmc1Outer`) |
| 551 | `mapper_178.c` | 109 | 4 | 0 | done (batch 19, `WaixingFs305`) |
| 554 | `mapper_554.c` | 94 | 1 | 0 | done (batch 10, `MultiP`) |
| 555 | `mapper_555.c` | 142 | 3 | 0 | done (batch 32, `Mmc3P`) |
| 556 | `mapper_556.c` | 178 | 1 | 0 | todo |
| 557 | `mapper_557.c` | 58 | 1 | 0 | done (batch 31, `MultiP`) |
| 558 | `mapper_558.c` | 135 | 14 | 0 | todo |
| 559 | `mapper_559.c` | 108 | 1 | 0 | done (batch 30, `VrcP`) |
| 560 | `mapper_560.c` | 83 | 1 | 0 | todo |
| 561 | `mapper_561.c` | 276 | 203 | 0 | todo |
| 562 | `mapper_562.c` | 538 | 290 | 0 | todo |
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
