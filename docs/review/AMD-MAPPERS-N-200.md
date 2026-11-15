# AMD proposal — REQ-CORE-102's acceptance criterion names a mapper Mesen2 supports

**Status:** proposed by lot MAPPERS-N (2026-11-15), awaiting the owner.

SPEC-02 §3.11, REQ-CORE-102: "Tout autre numéro de mapper DOIT être refusé au chargement
(REQ-CORE-011). Ajouter des mappers au-delà de la liste est permis." Its acceptance criterion:
"en-tête annonçant le mapper 200 → échec."

Mapper 200 is a real multicart board (Mesen2 `Unlicensed/Mapper200.h`, FCEUX `mappers/200.cpp`),
and the owner's request of 2026-11-15 is to implement the mappers the other emulators run. The
requirement allows it; only the criterion's example number forbids it.

**Proposed amendment:** the criterion becomes "en-tête annonçant le mapper 186 → échec". 186 is the
Fukutake Study Box, which Mesen2 handles as a BIOS file and not as a cartridge board
(`MapperFactory.cpp`, "The study box is handled as a bios file, not a iNES rom"), so no later
mapper lot can implement it — `docs/review/MF-6-inventory.md` §6 item 2 already chose it for that
reason, and `cartridge_tests.rs` already sweeps it.

**Meanwhile:** `board-200` exists and is tested, but stays out of `default`, so the default build
still refuses 200 and the criterion holds. Accepting the amendment means adding `board-200` to
`default` and replacing 200 by 186 in `req_core_102_any_other_mapper_is_refused`,
`amd002_tests.rs` and `cartridge_tests.rs`.
