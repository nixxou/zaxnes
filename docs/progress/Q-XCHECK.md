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
| 5/6 | Q-164, Q-165, Q-166, Q-167, Q-168 (12, 14, 83/264, 91) | **done** |
| 6/6 | Q-169, Q-170, Q-171, Q-172 + final check | **done** |

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

## Batch 5/6 — done

Tickets Q-164 to Q-168 (mappers 12, 14, 83/264 and 91). Twenty-four more verdict rows: fifteen
CONFIRMED, six OPEN, **one CHANGE**.

* **Q-164 §3 is the second CHANGE of the lot**: `Huang1::LANGUAGE_BIT` should be `$01`, not `$00`.
  FCEUX initialises its mapper-12 read register to 1 with the comment "chinese is default" and
  Nestopia answers a hard-coded `0x01`; the page says the GAL is wired to Chinese in every known copy,
  so this build's `$00` selects English in *Dragon Ball Z 5*. One constant, one test line (§CHANGE 6).
* **Q-164 §1 and §4 are clean**: no project gives mapper 12 a supervisor register or a VRC2/MMC1 mode
  (the census's `Somari` claim is wrong), and three of the four refresh the windows inside the `$4132`
  write while **FCEUX alone latches** — the page's attribution of that behaviour to FCEUX is verified.
* **Q-165 §1 is four for four on the bit positions** (mode at bit 1, A18 at bits 3/5/7), so the prose
  paraphrase MF-6b step 6 rejected is refuted everywhere. §5a is the interesting OPEN: **Nestopia
  implements "mirroring only in VRC2 mode" in this build's exact shape** — MMC3-mode `$A000`
  re-applies the VRC2's `$9000` value and ignores the datum — which verifies both the page's
  attribution *and* its "contradicts other emulators" caveat, since the other three contradict it.
  One free observation: this build's exact-address VRC2 decode is stricter than anyone's and is **not
  needed** for the `$A131` protection it was adopted for (A1/A0 alone suffice).
* **Q-166/Q-167**: the solder pad is a **user dip switch in all three** projects that support the
  number, which is exactly the host surface the ticket says the constant should wire to; nobody
  implements the `$8318` IRQ source, so this build is ahead there; and **puNES implements 83.2**, the
  refused variant, in the layout Q-167 documents — so that refusal is now only a mechanism cost, and
  it wants the same `Mapper::prg_ram_window` forward as mapper 195's `$5000` window.
* **Q-168**: puNES is the only other project with mapper 91's outer bank register or submapper 1, and
  it confirms **the outer bank's bit order** (A2 = PRG A18, the diagram's columns), the `$8000-$9FFF`
  decode, the 5/4 counter's borrow-out firing, "last bank of the inner 128 KiB" and `$6004` =
  horizontal. Only submapper 0's free-running-vs-stops question stays open, two-two.

## Batch 6/6 — done

Tickets Q-169 to Q-172 (mapper 111's flash save, mapper 164's 93C66, the Kǎshèng trio 215/208/219,
and the three singletons 262/292/246). Twenty-nine more verdict rows: twenty-two CONFIRMED, five OPEN,
**two CHANGE**.

* **Q-172 §2 is the most actionable finding of the whole lot.** **Mesen 2 and puNES both fill mapper
  292's two data registers by reading the game's zero page at `$6A` and `$FF` directly**, not from a
  bus latch, deriving it from the page's own "The game writes the desired values to zero page". So
  §7.2's "no new hook" is right after all — by a route §7.2 did not describe — and the +1.098 % that
  parked `board-292` is avoidable. §CHANGE 8 has the edit, the blocker (the board needs to read console
  RAM) and the accuracy trade.
* **Q-172 vindicates its own §0 method twice**: mapper 262's CHR-RAM select is bit **6** in all three
  projects that support the number (the paraphrase that put it at bit 7 is refuted everywhere), and
  **Mesen 2's four CHR A18 lines are `[3,3,2,2,0,0,1,1]` — this build's array to the entry, crossing
  included.** §1.3 is a CHANGE the other way: all three serve the CHR-RAM chip **unbanked**, the
  alternative the ticket names (§CHANGE 7).
* **Q-171 is confirmed on five of six points by puNES alone**, including the one the ticket says the
  page cannot settle: mapper 215 submapper 1's overlaid `$5001` rows are **PRG bits 3/1/0 and CHR bits
  3/2/1** in puNES, this build's reading to the bit. Also 208's `$A000` inert on submapper 0 and live
  on submapper 1, the index as the written value, 219's outer bank inside `$5000-$5FFF`, and the mode
  bit latched only by `$8002`. §3 (`$5001`'s `$xF` power-up) is the one OPEN: Mesen 2 and puNES both
  use `$03`.
* **Q-170**: puNES is the only project that implements the 93C66, and it confirms **all three** of
  §1's contradictions with §7.2 — **eight-bit words**, a separate device type, and the bytes in the
  NVRAM slice — plus §2.1 (bit 6 decoded and inert, in a comment) and §2.4 (the 1 bpp substitution on
  the write path).
* **Q-169**: Mesen 2 independently reaches this build's three shapes (a medium beside the battery RAM
  and not gated on the battery bit, a diff rather than a chip image, the diff in the savestate), while
  FCEUX gates on the battery bit and stores the whole 512 KiB. Mesen 2 also implements the
  `$90`/`$F0`/`$10` commands this build leaves out.
* **Q-172 §3**: puNES forces mapper 246's PRG A17 under **exactly this build's `$FFE4` constant**, and
  both FCEUX and puNES keep `$6003` across a soft reset as the page's "only" requires (Mesen 2 restores
  it). The register window is the one OPEN — all three decode `$6000-$67FF`, §7.2's figure rather than
  the page's mask.

## Final check

* Every ticket from **Q-140 to Q-172 except Q-153** has at least one verdict row in
  `docs/review/Q-XCHECK.md`'s summary table (147 rows in total) **and** an appended
  `## Cross-check (Q-XCHECK)` section in its own file under `spec/changes/`. Verified by enumeration.
* **Q-153 is untouched** — zero occurrences of `Q-XCHECK` in it.
* **No Rust code was changed**, no test was added or edited, and no file of any of the four projects
  was copied from. Every citation is a `third_party/<project>/<path>:<line>` pointer.
* **Eight CHANGEs** are recorded with their file, function, edit, affected titles and risk; the two
  with visible in-game effects are CHANGE 1 (mapper 123's `$5800` bank bit) and CHANGE 6 (mapper 12's
  language bit), and CHANGE 8 would unpark `board-292`.
* `git status`: clean except `rust/target-mf/` (untracked build output, never staged).
