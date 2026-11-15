# CORE-3 — parked boards and mechanisms back in `default`

Lot CORE-2 gave the popular boards (NROM, MMC1, UxROM, CNROM, AxROM, MMC3, MMC5) specialised paths
that never see the `Board` dispatch. What was left out of `default` because it cost every game
through that dispatch is put back:

| Feature | What it is | Why it was out |
|---|---|---|
| `board-292` | mapper 292 (BMW8544) | over the tolerance on PR-1 (MF-UNL-12) |
| `board-143-open-bus` | the two open-bus bits of mapper 143 | the test on its read path |
| `mmc1-snrom-e-bit` | the SNROM PRG-RAM `E` bit of the MMC1 | a test on the MMC1's paths |
| `board-jy-asic-tile-latch` | the JY ASIC's tile latch in the tile emitter | the emitter's hooks |

`board-vs-system` stays out: the VS. System is set aside (owner's decision).

## Measurement (callgrind, clean builds, 36adf690 against dev c6b4378c)

metroid +0.005 %, mm2 −0.000 %, smb1 −0.008 %, smb3 +0.014 %, zelda2 +0.002 %. PR-1 set: m0 +0.012 %,
m4 +0.013 %, mmc5 +0.026 %, every other ROM within ±0.002 %.

## Validation

AccuracyCoin 147/147 in the four alignments; test-ROM campaign **215 pass / 4 fail** (was 213 / 6):
`holy-mapperel` `M1_P128K` and `M1_P128K_CR8K` (SNROM) now pass with `mmc1-snrom-e-bit`; golden-core
identical to dev; tests 1664 (core, harness, settings, NesHawk) and 863 (application), all green.
