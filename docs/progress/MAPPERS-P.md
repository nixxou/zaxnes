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
| 100 | `mapper_100.c` | 175 | 1 | 0 | todo |
| 122 | `mapper_184.c` | 64 | 0 | 0 | done (batch 1, `MultiP`) |
| 186 | `mapper_186.c` | 82 | 1 | 0 | todo |
| 237 | `mapper_237.c` | 98 | 1 | 0 | done (batch 1, `MultiP`) |
| 248 | `mapper_115.c` | 115 | 0 | 0 | todo |
| 256 | `mapper_256.c` | 123 | 241 | 0 | todo |
| 267 | `mapper_267.c` | 87 | 1 | 0 | todo |
| 269 | `mapper_269.c` | 130 | 3 | 0 | todo |
| 271 | `mapper_271.c` | 69 | 1 | 0 | done (batch 2, `MultiP`) |
| 272 | `mapper_272.c` | 160 | 1 | 0 | todo |
| 281 | `mapper_281.c` | 69 | 11 | 0 | done (batch 2, `JyCompany` + `JyMulticart`) |
| 282 | `mapper_282.c` | 76 | 13 | 0 | done (batch 2, `JyCompany` + `JyMulticart`) |
| 291 | `mapper_291.c` | 90 | 1 | 0 | todo |
| 295 | `mapper_295.c` | 69 | 12 | 0 | done (batch 2, `JyCompany` + `JyMulticart`) |
| 297 | `mapper_297.c` | 119 | 1 | 0 | todo |
| 308 | `mapper_308.c` | 90 | 1 | 0 | todo |
| 311 | `mapper_311.c` | 97 | 1 | 0 | todo |
| 315 | `mapper_315.c` | 92 | 1 | 0 | todo |
| 322 | `mapper_322.c` | 101 | 1 | 0 | todo |
| 327 | `mapper_327.c` | 89 | 0 | 0 | todo |
| 334 | `mapper_334.c` | 95 | 1 | 0 | todo |
| 335 | `mapper_335.c` | 85 | 1 | 0 | done (batch 2, `MultiP`) |
| 337 | `mapper_337.c` | 92 | 0 | 0 | done (batch 3, `MultiP`) |
| 338 | `mapper_338.c` | 86 | 1 | 0 | done (batch 2, `MultiP`) |
| 339 | `mapper_339.c` | 97 | 1 | 0 | todo |
| 340 | `mapper_340.c` | 79 | 1 | 0 | todo |
| 341 | `mapper_341.c` | 67 | 1 | 0 | done (batch 2, `MultiP`) |
| 342 | `mapper_342.c` | 2381 | 9 | 0 | todo |
| 343 | `mapper_343.c` | 71 | 0 | 0 | done (batch 2, `MultiP`) |
| 344 | `mapper_344.c` | 125 | 3 | 0 | todo |
| 345 | `mapper_345.c` | 102 | 0 | 0 | todo |
| 347 | `mapper_347.c` | 122 | 2 | 0 | todo |
| 350 | `mapper_350.c` | 99 | 0 | 0 | done (batch 3, `MultiP`) |
| 351 | `mapper_351.c` | 305 | 0 | 0 | todo |
| 352 | `mapper_352.c` | 66 | 0 | 0 | done (batch 2, `MultiP`) |
| 353 | `mapper_353.c` | 149 | 1 | 0 | todo |
| 354 | `mapper_354.c` | 111 | 0 | 0 | done (batch 4, `MultiP`) |
| 355 | `mapper_355.c` | 258 | 4 | 1 | todo |
| 356 | `mapper_356.c` | 106 | 1 | 0 | todo |
| 357 | `mapper_357.c` | 118 | 1 | 0 | todo |
| 358 | `mapper_358.c` | 76 | 4 | 0 | done (batch 3, `JyCompany` + `JyMulticart`) |
| 359 | `mapper_359.c` | 293 | 3 | 0 | todo |
| 360 | `mapper_360.c` | 54 | 1 | 0 | done (batch 2, `MultiP`) |
| 361 | `mapper_361.c` | 85 | 5 | 0 | todo |
| 362 | `mapper_362.c` | 99 | 2 | 0 | todo |
| 368 | `mapper_368.c` | 106 | 2 | 0 | todo |
| 369 | `mapper_369.c` | 221 | 1 | 0 | todo |
| 370 | `mapper_370.c` | 117 | 1 | 0 | todo |
| 372 | `mapper_372.c` | 96 | 1 | 0 | todo |
| 374 | `mapper_374.c` | 56 | 4 | 0 | todo |
| 375 | `mapper_375.c` | 77 | 1 | 0 | done (batch 3, `MultiP`) |
| 377 | `mapper_377.c` | 86 | 3 | 0 | todo |
| 380 | `mapper_380.c` | 89 | 9 | 0 | done (batch 4, `MultiP`) |
| 381 | `mapper_381.c` | 60 | 0 | 0 | done (batch 2, `MultiP`) |
| 382 | `mapper_382.c` | 70 | 1 | 0 | done (batch 5, `MultiP`) |
| 384 | `mapper_384.c` | 65 | 0 | 0 | todo |
| 386 | `mapper_386.c` | 76 | 1 | 0 | done (batch 5, `JyCompany` + `JyMulticart`) |
| 387 | `mapper_387.c` | 76 | 4 | 0 | done (batch 5, `JyCompany` + `JyMulticart`) |
| 388 | `mapper_388.c` | 76 | 2 | 0 | done (batch 5, `JyCompany` + `JyMulticart`) |
| 389 | `mapper_389.c` | 90 | 1 | 1 | done (batch 1, `MultiP`) |
| 390 | `mapper_390.c` | 93 | 0 | 0 | done (batch 5, `MultiP`) |
| 393 | `mapper_393.c` | 123 | 1 | 0 | todo |
| 394 | `mapper_394.c` | 193 | 0 | 0 | todo |
| 395 | `mapper_395.c` | 88 | 4 | 0 | todo |
| 396 | `mapper_396.c` | 68 | 5 | 0 | todo |
| 397 | `mapper_397.c` | 69 | 1 | 0 | todo |
| 398 | `mapper_398.c` | 89 | 1 | 0 | todo |
| 399 | `mapper_399.c` | 79 | 1 | 0 | todo |
| 400 | `mapper_400.c` | 105 | 1 | 0 | todo |
| 401 | `mapper_401.c` | 97 | 1 | 0 | todo |
| 403 | `mapper_403.c` | 96 | 4 | 0 | todo |
| 404 | `mapper_404.c` | 71 | 1 | 0 | todo |
| 406 | `mapper_406.c` | 120 | 2 | 0 | todo |
| 409 | `mapper_409.c` | 56 | 1 | 0 | todo |
| 410 | `mapper_410.c` | 96 | 1 | 0 | todo |
| 411 | `mapper_411.c` | 103 | 7 | 0 | todo |
| 412 | `mapper_412.c` | 106 | 1 | 0 | todo |
| 413 | `mapper_413.c` | 135 | 1 | 0 | todo |
| 414 | `mapper_414.c` | 80 | 1 | 0 | todo |
| 415 | `mapper_415.c` | 69 | 1 | 0 | todo |
| 416 | `mapper_416.c` | 133 | 1 | 0 | todo |
| 417 | `mapper_417.c` | 131 | 1 | 0 | todo |
| 420 | `mapper_420.c` | 96 | 1 | 0 | todo |
| 421 | `mapper_421.c` | 72 | 1 | 0 | todo |
| 422 | `mapper_126.c` | 153 | 7 | 0 | todo |
| 428 | `mapper_428.c` | 95 | 6 | 0 | todo |
| 429 | `mapper_429.c` | 56 | 1 | 0 | todo |
| 431 | `mapper_431.c` | 81 | 1 | 0 | todo |
| 432 | `mapper_432.c` | 112 | 1 | 0 | todo |
| 433 | `mapper_433.c` | 75 | 1 | 0 | todo |
| 434 | `mapper_434.c` | 74 | 4 | 0 | todo |
| 436 | `mapper_436.c` | 59 | 1 | 0 | todo |
| 437 | `mapper_437.c` | 74 | 1 | 0 | todo |
| 438 | `mapper_438.c` | 71 | 2 | 0 | todo |
| 442 | `mapper_442.c` | 127 | 1 | 0 | todo |
| 446 | `mapper_446.c` | 746 | 4 | 0 | todo |
| 447 | `mapper_447.c` | 98 | 1 | 0 | todo |
| 451 | `mapper_451.c` | 147 | 2 | 0 | todo |
| 452 | `mapper_452.c` | 119 | 1 | 0 | todo |
| 455 | `mapper_455.c` | 97 | 1 | 0 | todo |
| 456 | `mapper_456.c` | 83 | 1 | 0 | todo |
| 457 | `mapper_457.c` | 86 | 1 | 0 | todo |
| 471 | `mapper_471.c` | 109 | 0 | 0 | todo |
| 481 | `mapper_481.c` | 55 | 0 | 0 | todo |
| 512 | `mapper_512.c` | 94 | 2 | 1 | todo |
| 516 | `mapper_516.c` | 79 | 2 | 0 | todo |
| 517 | `mapper_517.c` | 107 | 1 | 1 | done (batch 1, `MultiP`) |
| 524 | `mapper_524.c` | 81 | 0 | 0 | todo |
| 525 | `mapper_525.c` | 91 | 1 | 0 | todo |
| 526 | `mapper_526.c` | 123 | 1 | 0 | todo |
| 527 | `mapper_527.c` | 61 | 0 | 0 | todo |
| 528 | `mapper_528.c` | 118 | 0 | 0 | todo |
| 532 | `mapper_532.c` | 411 | 0 | 0 | todo |
| 534 | `mapper_126.c` | 153 | 10 | 0 | todo |
| 536 | `mapper_195.c` | 148 | 0 | 0 | todo |
| 538 | `mapper_538.c` | 64 | 2 | 0 | todo |
| 539 | `mapper_539.c` | 113 | 1 | 0 | todo |
| 540 | `mapper_359.c` | 293 | 1 | 0 | todo |
| 541 | `mapper_541.c` | 70 | 1 | 0 | todo |
| 543 | `mapper_543.c` | 87 | 0 | 0 | todo |
| 547 | `mapper_547.c` | 241 | 8 | 4 | todo |
| 550 | `mapper_550.c` | 86 | 1 | 0 | todo |
| 551 | `mapper_178.c` | 109 | 4 | 0 | todo |
| 554 | `mapper_554.c` | 94 | 1 | 0 | todo |
| 555 | `mapper_555.c` | 142 | 3 | 0 | todo |
| 556 | `mapper_556.c` | 178 | 1 | 0 | todo |
| 557 | `mapper_557.c` | 58 | 1 | 0 | todo |
| 558 | `mapper_558.c` | 135 | 14 | 0 | todo |
| 559 | `mapper_559.c` | 108 | 1 | 0 | todo |
| 560 | `mapper_560.c` | 83 | 1 | 0 | todo |
| 561 | `mapper_561.c` | 276 | 203 | 0 | todo |
| 562 | `mapper_562.c` | 538 | 290 | 0 | todo |
| 564 | `mapper_564.c` | 77 | 0 | 0 | todo |

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
