# Q-XCHECK — progress

Lot **Q-XCHECK** (clean side, R3), started 2026-11-08 on branch `mapper-review`, base `5aa91b02`.

**The task**: cross-check the thirty-two open tickets **Q-140 to Q-172** (Q-153 skipped — audio, not
a mapper) against **Mesen 2**, **FCEUX**, **Nestopia UE** and **puNES** in `third_party/`. One verdict
line per ticket (CONFIRMED / CHANGE / OPEN), a report at `docs/review/Q-XCHECK.md`, and a short
appended `## Cross-check (Q-XCHECK)` section on each ticket file in `spec/changes/`.
**No Rust code is changed by this lot.**

## Batches

| k/n | tickets | state |
| --- | --- | --- |
| 1/6 | Q-140, Q-141, Q-142, Q-143, Q-144, Q-145, Q-146, Q-147 | **done** |
| 2/6 | Q-148, Q-149, Q-150, Q-151, Q-152 (J.Y. Company ASIC) | **done** |
| 3/6 | Q-154, Q-155, Q-156, Q-157, Q-158, Q-159 (178, 163, 268) | pending |
| 4/6 | Q-160, Q-161, Q-162, Q-163 (8259, 112/193, 298, 218) | pending |
| 5/6 | Q-164, Q-165, Q-166, Q-167, Q-168 (12, 14, 83/264, 91) | pending |
| 6/6 | Q-169, Q-170, Q-171, Q-172 + final check | pending |

## Batch 1/6 — done

Tickets Q-140 to Q-147 (mappers 173, 150/243, 195, 74/192/194, 123/114/182/115/121/187, 143, 144,
38, 107). Twenty-two verdict rows in the report's summary table.

Headline results:

* **Q-144 §1 is a CHANGE**, and it is the one point with a visible in-game effect. nesdev's mapper-123
  page **contradicts itself** — its ASCII diagram's columns put the bank's bit 1 at register bit 4,
  its prose says "bits 0-3/5" twice, which means bit 3 — and all four emulators implement the prose
  with one shared expression while running *Mortal Kombat 3*. The exact one-term edit is §CHANGE 1.
* **Q-142 is largely CONFIRMED by a source nobody had looked at**: puNES implements mapper 195's
  run-time GAL, with the same three-bit row field (register bits 6, 3, 1), the same PPU-write latch
  and the same 8 KiB chip as this build. Its power-on row is `$82`, not the page's `$80`, so that one
  point goes OPEN; and FCEUX and puNES both map the `$5000-$5FFF` PRG-RAM window this build omits
  (§CHANGE 2).
* **Q-144 §2 CONFIRMED**: puNES holds both of mapper 114's one-to-one permutation tables, identical
  to this build's; Mesen 2 and FCEUX are the lineage the page refutes.
* **Q-140, Q-143, Q-145, Q-146 and Q-147 §2 are clean CONFIRMEDs**, several of them four for four.
* Three OPEN points carried forward (Q-141 mode 3, Q-142 power-on, Q-144 §3 and §4, Q-147 §1), each
  with the observation that would close it.

One methodological note recorded in the report and worth repeating: **the four are not four
independent witnesses** for the unlicensed MMC3 clones. Mesen 2's, Nestopia's and puNES's expressions
are frequently FCEUX's, character for character modulo language, so a 4-0 tally is often one
lineage. Where it is, the report says so and weighs it as one.

## Batch 2/6 — done

Tickets Q-148 to Q-152, the J.Y. Company ASIC (mappers 35, 90, 209, 211). Fourteen more verdict rows.

The governing finding: **puNES builds all four numbers as one board** with one per-number flag
(`id != 90`), which is this build's shape exactly, and it is an **independent** reading — where it
agrees with this build it agrees on details Mesen's and FCEUX's shared ancestor gets differently.

* **Nine CONFIRMEDs**, several of them reversing the ticket's expectation that this build stood alone:
  `$D001` bit 3 (puNES decodes it), mapper 211 = 209 (puNES has no special case), PRG mode 3's middle
  bit (puNES's reversal has seven terms), `$8003` as a 32 KiB bank (three of four; Mesen 2 is
  inconsistent with itself in two places), `$D003` bit 7 not folding CHR registers (Mesen 2 alone),
  the latch gated on `$D003` bit 7 and not on the mapper number (puNES identical), a disabled counter
  not advancing (3-2), the prescaler's down-wrap at the mask (3-1), and a dropped ROM-nametable write
  (3-1, Mesen 2 the only one that keeps the byte and the only one with a forum citation for it).
* **Four OPENs**: mapper 35's board (Mesen 2 and FCEUX run the number's one title on the UNL-SC-127,
  whose only non-subset register is the counter — the falsifier is now precisely specified); the
  `L=1` CHR mask (puNES agrees with this build, the other three share one FCEUX expression);
  "disabling resets the prescaler" (only puNES implements it); and IRQ source 3 (FCEUX and Nestopia
  have the same gap, Mesen 2 needs a CPU-level hook this core lacks, and **puNES's cheap partial —
  clock on cartridge writes only — is reachable here for no new hook**).
* **Two divergences the ticket did not know about**, recorded: FCEUX compares `$B00x` bit 7 against
  `$D001` bit 7 where the page and the other three use `$D002`; and Mesen 2's 16 KiB PRG mode treats
  `$8003` as an 8 KiB bank while shifting `$8001`.

`git status`: clean except `rust/target-mf/` (untracked build output, never staged).
