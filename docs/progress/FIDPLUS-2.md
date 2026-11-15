# FIDPLUS-2 — per-dot differential against the transistor netlist (step 1)

Branch `feat/fidplus-2`, from `feat/fidplus-1` (6289064d). Details: `docs/review/FIDELITY-LEADS.md` A5.

| Item | State |
|---|---|
| Per-dot trace (`ppu-dot-trace`, harness `dot-trace`) | done, diagnostics only, never in `default` |
| `INC $2007` double write (first address gets the final value) | done, all regions |
| Dot-257 copy after a `$2001` switch on dot 256 (alignments 2-3) | done, NTSC/Multi |
| `$2007` render-read merge rules | done, on: confirmed on Zelda II's title split replayed on the netlist; changes Zelda II Europe golden frame 600 |
| Second `$2006` write: increment of D+4 applies to the new `v` | done, NTSC |
| Tile latch of the first fetch after a switch on | open lead |

## Validation (7ee71a40)

Campaign 215 pass / 4 fail (unchanged), AccuracyCoin 147/147 in the four alignments, golden-core
identical to FIDPLUS-1, tests 1668 (core, harness, settings, NesHawk) and 867 (application), all green.

## Measurement

Callgrind, clean builds, against FIDPLUS-1 6289064d: +0.02 % to +0.10 % on the PR-1 set.

## Step 2 (2252d645)

Zelda II's title split replayed on the netlist: the `$2007` merge rules are right, on by default
(Zelda II Europe golden frame 600 changes). Second `$2006` write: the D+4 increment applies to the
loaded `v`. `g2005`, `g2006x` compared. Tile-latch lead: first attempt dropped (see A5).
Validation: campaign 215 / 4, AccuracyCoin 147 x4, golden = step 1 plus Zelda II Europe, tests
1668 + 867 green. Callgrind against FIDPLUS-1: +0.00 % to +0.10 % (metroid +0.10, mm2 +0.09).
