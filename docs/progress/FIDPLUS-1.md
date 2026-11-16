# FIDPLUS-1 — Fidélité+ leads that need no new stepper

Branch `feat/fidplus-1`, from `feat/core-3` (6d252c10).

| Lead | Outcome |
|---|---|
| D2b (`$4015` latch per batch) | tried twice, dropped: APU +3.2 to +4.4 % (const generic), +0.24 to +0.29 % (bool) |
| C5 (`$2002` open-bus decay) | settled, no change: both decays inside nesdev's range, no reference separates them |
| G2 (PAL-M, PAL-N) | done: `NesRegion::PalM = 4`, `PalN = 5`, timings and duty swap from BeesNES |
| G1 (console model) | done: Emulation > Console model, a preset of region + C6 + D3 |

Details, sources and what stays unknown: `docs/review/FIDELITY-LEADS.md`, C5, D2b, G1, G2.

## G2 in short

| | CPU clock | Lines | VBlank | Odd-frame skip | Frame counter, noise/DMC | Duties 1/2 |
|---|---|---|---|---|---|---|
| PAL-M | 1 787 806 Hz | 262 | 241 | no | NTSC | swapped |
| PAL-N | 1 791 028 Hz | 312 | 291 | yes | NTSC | swapped |

FCEUX and NesHawk run PAL-M as NTSC and PAL-N as Dendy (their geometries). The existing regions
are untouched (`swaps_pulse_duty` false, same table rows): no change for any game outside the two
new regions.

## Measurement (callgrind, clean builds, 8045427f against feat/core-3 6d252c10)

Every ROM of the PR-1 set within ±0.001 % (m1, m0, m4, mmc5, vblnmi, oamstr, readbuf, duelito,
mm2, smb1, z2, smb3, metroid): the two new regions add a test on the `$4000`/`$4004` write only.

## Validation

Test-ROM campaign 215 pass / 4 fail (as CORE-3), AccuracyCoin 147/147 in the four alignments,
golden-core identical to CORE-3 (the three SMB3 Japan pictures listed by `check.sh` against its old
base move since FIDELITY step 31, unchanged here).

Tests: 1666 (core, harness, settings, NesHawk, ignored included) and 867 (application), all green.
