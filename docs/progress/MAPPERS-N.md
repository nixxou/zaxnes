# MAPPERS-N — the mappers other emulators run and Zaxnes does not

Branch `feat/mappers-n`, worktree `G:/3DSen/wt-mappers`, base `e2b7fced` (feat/fidplus-1).
Owner's request (2026-11-15): implement the mappers missing compared with the other emulators.

## Resume note (read first after a crash)

* Every finished board is committed; the table below says which. Anything not in a commit is lost.
* Build: one target dir, `rust/target` in this worktree. `df -h /g` before a big build.
* Next step is always the first row of the table whose state is not "done"/"non vérifiée".

## Inventory (step 1)

* `SUPPORTED_MAPPERS` at e2b7fced: 144 numbers.
* Mesen2 `MapperFactory.cpp` (`cleanroom/third_party/mesen2/Core/NES/MapperFactory.cpp`): 309
  `case` labels; **165 numbers Mesen2 lists and Zaxnes does not**, of which ~25 are empty `case`
  labels in Mesen2 itself (186, 256, 257, 271, 295, 315, 322, 327, 335, 337-342, 344, 345, 350,
  524-528). Those are out: no emulator of the allowed list implements them in Mesen2's table.
* Local ROMs: a header scan of every `.nes` under `G:/3DSen` (test-originals, test-originals-extra,
  demo/roms, assets-free/test-roms, mm1corpus, roms-owner, …) finds **two** files on a missing
  number: mapper **250** (`roms-owner/Insector_X/… queen_bee_v_unl_.nes`) and mapper **253**
  (`roms-owner/Excitebike_1_/… vs._excitebike_fds_conversion_b1.nes`). Every other board of this
  lot is therefore "non vérifiée" (unit tests from the page only).
* Game counts: entries per mapper in Mesen2's game database (`ext-tools/MesenCE-2.2.1/MesenNesDB.txt`).

## Design

* No new `Board` variant per board. Two shared wrappers, as `DecodeBoard`/`IrqBoard` already are:
  * `Mmc3Clone` (in `IrqBoard`): an `Mmc3` plus an outer register file and a `kind`; the kind
    decides the register decode and how the MMC3's bank numbers are masked/offset — the shape
    Mesen2 gives these boards (subclasses overriding `SelectPrgPage`/`SelectChrPage`).
  * `LatchMulti` (in `DecodeBoard`): the discrete multicarts — a few latched registers, the kind
    deciding the decode and the banking.
* Features: `board-mmc3-clones` and `board-latch-multi` umbrellas, one `board-NNN` per number.

## Plan, ordered (games = Mesen2 DB entries)

| # | mapper(s) | games | ref | family | difficulty | state |
|---|---|---:|---|---|---|---|
| 1 | 250 | 2 | Mesen2 `MMC3_250.h` | MMC3 clone (address lines) | S | **done**, Queen Bee V plays (4e9082a6) |
| 2 | 253 | 5 | Mesen2 `Mapper253.h` | VRC4 + CHR-RAM | M | non vérifiée: local dump is a bad dump [b1] (ba013045) |
| 3 | 45, 52, 134, 44, 49, 205, 197, 198, 199, 196, 245, 249, 254, 126, 191, 217, 238, 287, 325, 259, 313 | ~450 | Mesen2 `MMC3_*.h` | MMC3 clones | S-M each | todo |
| 4 | 58, 241, 59, 57, 227, 212, 242, 236, 235, 226, 221, 63, 200-204, 213, 214, 229-231, 233, 240, 244, 255, 60-62, 46, 51, 104, 108, 113, 120, 125, 170, 174, 177, 179, 216, 261, 274, 286, 289 | ~550 | Mesen2 `Mapper*.h`, `Bmc*.h` | discrete multicarts | S each | todo |
| 5 | 40, 42, 43, 50, 106, 117, 222, 183, 56/142, 171 | ~40 | Mesen2 | discrete + IRQ | M | todo |
| 6 | 6, 8, 17 | ~430 | Mesen2 `FrontFareast.h` | FFE copier + IRQ | M | todo |
| 7 | 176 | ~420 | Mesen2 `Fk23C.h` | MMC3 multicart, large | L | todo |
| 8 | the rest (Kaiser FDS conversions, Subor, …) | few | Mesen2 | various | M | todo |

## Log

* 2026-11-15 4e9082a6 — `Mmc3Clone` (IrqBoard): 44 45 49 52 134 196 197 205 238 245 249 250. 250 verified
  on Queen Bee V (menu + gameplay with IRQ split). Others: unit tests (`tests/mappers_n_tests.rs`).
* ba013045 — 253 (`Waixing253`, IrqBoard, clocks on CPU cycles). The only local dump has its vectors
  in zero padding: unusable in any emulator; unit tests only.
* a415bbcd — `LatchMulti` (DecodeBoard): 57 58 59 63 200 201 202 203 204 212 213 214 221 226 227 229
  230 231 233 235 236 240 241 242 244 255. **board-200 off by default**: SPEC-02 REQ-CORE-102's
  acceptance criterion is "mapper 200 → échec"; amendment proposed in
  `docs/review/AMD-MAPPERS-N-200.md` (move it to 186).
* Tools: `scratchpad/register.py` (features, SUPPORTED_MAPPERS, create_board, A12 lists) and
  `addtest.py` (req_core_100 table) — not committed; redo by hand if lost.
* Baseline on e2b7fced: campaign 215/4/1/111, golden 98 ROMs in scratchpad `base/golden-base.jsonl`.
