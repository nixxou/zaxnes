# LEGACY-XCHECK — progress

Role R3, clean side. Branch `mapper-review`, worktree `wt-mappers`. Base `7dec9d7f`.

**This lot changes nothing.** The only two files it writes are
`docs/review/LEGACY-XCHECK.md` (the report) and this note. No Rust code, no test, no Cargo
file, no spec file, no Q ticket, no existing document is touched. Questions that would have
become Q tickets are recorded in the report instead.

## The set under review

`git show v0.4.2:rust/crates/zax-nes/src/cartridges/registry.rs` advertises

```
0 1 2 3 4 5 7 9 10 11 13 15 16 18 21 22 23 25 27 28 30 32 34 41 64 66 71 87 101 156 225 228
```

32 iNES numbers over **26 board types** (9/10 share MMC2/MMC4, 21/22/23/25/27 share VRC2/VRC4,
87/101 share the Jaleco J87). `n = 26` for the commit counter.

Order of work: most-played first — NROM, MMC1, UxROM, CNROM, MMC3, MMC5, AxROM, MMC2/MMC4, then
the discrete boards, VRC2/VRC4, RAMBO-1, Bandai FCG, SS88006, Irem G-101, the multicarts, Daou
306.

## Boards done

| # | board | verdicts |
| --- | --- | --- |
| 0 | NROM | 5 OK |
| 1 | MMC1 (SxROM) | 11 OK, 3 DIFFERS, 2 OPEN |
| 2 | UxROM | 8 OK |
| 3 | CNROM | 7 OK |
| 7 | AxROM | 7 OK |
| 66 | GxROM | 4 OK |
| 4 | MMC3, MMC6, MC-ACC | 12 OK, 3 OPEN |
| 5 | MMC5 (ExROM) + expansion audio | 16 OK, 1 DIFFERS, 1 OPEN |
| 9, 10 | MMC2 (PxROM), MMC4 (FxROM) | 8 OK, 1 OPEN |
| 11 | Color Dreams | 4 OK |
| 13 | CPROM | 4 OK, 1 DIFFERS |
| 15 | 100-in-1 Contra Function 16 | 7 OK, 1 DIFFERS |
| 16 | Bandai FCG-1/2 and LZ93D50 | 7 OK, 1 OPEN |
| 18 | Jaleco SS88006 | 7 OK, 1 DIFFERS |
| 21, 22, 23, 25, 27 | Konami VRC2 and VRC4 | 10 OK, 1 DIFFERS |

Plus one shared section, **the bus conflict re-counted**, which carries the four new votes for
the eight boards that share `registry::has_bus_conflicts`.

## Boards remaining

28, 30, 32, 34, 41, 64 (RAMBO-1), 71, 87/101, 156, 225, 228.

## Notes for whoever resumes this

- The report is appended at the `<!-- NEXT-BOARD -->` marker; the summary table goes at
  `<!-- SUMMARY-TABLE -->` and is composed last, from the per-board sections.
- Every `file:line` citation in the report has been verified against the file it names. Keep
  doing that: the four new sources are large and the line numbers do not survive guessing.
- `holy-mapperel/testroms` (in `assets-free/test-roms`) has configurations for mappers 0, 1, 2,
  3, 4, 7, 9, 10, 11, 28, 34, 66, 69, 78.3, 118 and 180 — i.e. for sixteen of the numbers under
  review. **None of them is run by the project's test harness today**, which is itself worth
  saying once in the report's final section.
- `docs/review/MR-1-mappers.md` is the prior art; its "Fixed in MF-1" lines say which of its
  findings are already closed, and those points are only re-opened here when one of the four new
  sources contradicts the conclusion.

## Final check (to be completed at the end of the lot)

- [ ] every mapper number of the v0.4.2 set has a section
- [ ] `git status` shows only `rust/target-*` untracked
- [ ] `git diff --stat 7dec9d7f` lists only the two files above
