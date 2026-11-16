# AMD proposal — REQ-CORE-102's acceptance criterion names a mapper Mesen2 supports

**Status:** proposed by lot MAPPERS-N and **accepted by the owner (2026-11-15)**: SPEC-02 lines 90
and 451 now name mapper 84, `board-200` is in `default`, the three tests sweep 84. nesdev ("INES
Mapper 084") confirms 84 has only conflicting allocations (Pasofami, an SMB2J hack) and no known
board.

SPEC-02 §3.11, REQ-CORE-102: "Tout autre numéro de mapper DOIT être refusé au chargement
(REQ-CORE-011). Ajouter des mappers au-delà de la liste est permis." Its acceptance criterion:
"en-tête annonçant le mapper 200 → échec."

Mapper 200 is a real multicart board (Mesen2 `Unlicensed/Mapper200.h`, FCEUX `mappers/200.cpp`),
and the owner's request of 2026-11-15 is to implement the mappers the other emulators run. The
requirement allows it; only the criterion's example number forbids it.

**Proposed amendment (revised the same day):** the criterion becomes "en-tête annonçant le mapper
84 → échec". The first draft proposed 186, the Fukutake Study Box, on the grounds that no lot could
ever implement it; that was wrong: FCEUX (`boards/186.cpp`) and puNES (`mapper_186.c`) run it as
mapper 186, and Mesen2 runs it from its own `.studybox` tape format (`StudyBoxLoader.cpp`) while
refusing the iNES number (`MapperFactory.cpp:459`). 186 would therefore walk into 200's trap again.
Mapper 84 is implemented by none of Mesen2 (`MapperFactory.cpp`), FCEUX (`ines.cpp`) or puNES, and
the refusal tests already use it (`mapper_tests.rs`, `cartridge_tests.rs`). Whether a cartridge of
it was ever dumped is to be checked against nesdev ("INES Mapper 084") before the spec is changed.

**Meanwhile:** `board-200` exists and is tested, but stays out of `default`, so the default build
still refuses 200 and the criterion holds. Accepting the amendment means adding `board-200` to
`default` and replacing 200 by 84 in `req_core_102_any_other_mapper_is_refused`,
`amd002_tests.rs` and `cartridge_tests.rs`.
