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
| 3/6 | Q-154, Q-155, Q-156, Q-157, Q-158, Q-159 (178, 163, 268) | **done** |
| 4/6 | Q-160, Q-161, Q-162, Q-163 (8259, 112/193, 298, 218) | **done** |
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

## Batch 3/6 — done

Tickets Q-154 to Q-159 (mappers 178, 163 and 268). Ten more verdict rows. Nestopia UE does not
support mapper 268, so those four tickets are a three-way comparison.

* **Q-156 is the clean win of the batch**: the mapper-268 lockout is implemented by all three
  projects that support the number, and **FCEUX uses this build's exact condition and annotates it in
  the same words** ("7th bit is 1 AND 4th is 0"), with the WRAM write happening first and
  independently. §7.2's "implement it as a no-op" has no support anywhere.
* **Q-154 §1 is a four-for-four CONFIRMED**: no project gives mapper 178 any CHR banking, so the
  lot's section heading was what was wrong. §3 confirms the battery rule (FCEUX uses the dump's bit
  exactly as this build does) and supplies the figure the ticket asked for: Mesen 2 and FCEUX both fix
  the PRG-RAM at **32 KiB**.
* **Q-155's lean has reversed.** Three projects do not implement mapper 163's D0/D1 swap at all;
  **puNES does, and applies it to `$5100` too — reading (a)** — with a hardware note about 1 MiB
  boards wiring both ASIC PRG A19/A20 outputs to ROM A19. The verdict stays OPEN (one vote, three
  abstentions) but §CHANGE 4 now holds the edit, including puNES's size-dependent exemption.
* **Q-157** is CONFIRMED in form — FCEUX refuses unimplemented submappers with an error exactly as
  this build does — and adds a scope note (§CHANGE 5): the four groups this build refuses are
  implemented by two of the three projects, and their two non-trivial features are named in both.
* **Q-158 and Q-159 are OPEN because everyone is silent**: nobody implements SC0/SC1 or PRG A25, and
  nobody decodes `$xxx3` bit 5 (FCEUX's `$5000` window is a crash workaround by its own comment). One
  incidental finding: FCEUX *does* cap that board's PRG at 16 MiB, which is the ceiling §7.2 feared —
  in a project, not here.

## Batch 4/6 — done

Tickets Q-160 to Q-163 (the Sachen 8259 family, mappers 112 and 193, mapper 298, mapper 218).
Fourteen more verdict rows, and **every one of them CONFIRMED** — the first batch with no OPEN and no
CHANGE.

* **Q-160 §1**: Mesen 2 implements the per-page mirroring swap *exactly* as this build does — the
  8259D on the mapper-137 page, the A/B/C on the family page — which is the question the ticket put to
  R0. §3 and §4 are four for four (simple mode uses CHR register 0 on every variant; register 6
  exists and drives the 8259D's fourth window).
* **Q-161**: all four decisions confirmed. (b) is 3-1 with **Mesen 2 the only project that keeps bit 0
  of the 2 KiB CHR selects**; (c) is confirmed by puNES, which implements `$6004` with this build's
  polarity and is the only other project to use the page's `$E007` eight-register decode (the three
  that mask to two bits could not reach `$6004` at all).
* **Q-162 is the find of the batch.** puNES's mapper 298 is one line —
  `init_VRC2and4(VRC24_VRC4, 0x02, 0x01, FALSE, …)` — and that fourth argument, `irq_repeated`, is
  read in exactly one place: the `$F003` A→E copy. Every Konami number passes TRUE, 298 passes FALSE.
  So the page's clause **and this build's load-time-bool architecture** are both independently
  confirmed, and §7.2's "does not reload the counter" is confirmed as a paraphrase (no project reloads
  on an acknowledge).
* **Q-163 is bit-for-bit.** Mesen 2, FCEUX and puNES all serve mapper 218's **pattern tables** from
  the 2 KiB nametable memory, none through a nametable-page hook, and FCEUX writes this build's
  four-row table out as a literal array with the comment "this could be done by bit logic, but this is
  self-documenting". Reading (b) is right.

`git status`: clean except `rust/target-mf/` (untracked build output, never staged).
