# LEGACY-XCHECK — progress

Role R3, clean side. Branch `mapper-review`, worktree `wt-mappers`. Base `7dec9d7f`.

**This lot changed nothing.** The only two files it wrote are
`docs/review/LEGACY-XCHECK.md` (the report, 2 204 lines) and this note. No Rust code, no test,
no Cargo file, no spec file, no Q ticket and no existing document was touched. The questions
that would have become Q tickets are recorded in the report as **OPEN** entries, each with the
evidence that would settle it.

## The set under review

`git show v0.4.2:rust/crates/zax-nes/src/cartridges/registry.rs` advertises

```
0 1 2 3 4 5 7 9 10 11 13 15 16 18 21 22 23 25 27 28 30 32 34 41 64 66 71 87 101 156 225 228
```

32 iNES numbers over **26 board types** (9/10 share MMC2/MMC4, 21/22/23/25/27 share VRC2/VRC4,
87/101 share the Jaleco J87). All 26 have a section and all 32 numbers are named in one.

## Result

| | |
| --- | --- |
| points checked | 177 |
| OK | 157 |
| DIFFERS | 9 |
| OPEN | 11 |
| boards with no difference | 13 of 26 |
| MR-1b verdicts overturned | 0 |
| MR-1b verdicts independently confirmed | 14 |

Per board:

| # | board | verdicts |
| --- | --- | --- |
| 0 | NROM | 5 OK |
| 1 | MMC1 (SxROM) | 11 OK, 3 DIFFERS, 2 OPEN |
| — | shared: the bus conflict, re-counted | (eight boards, four new votes) |
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
| 28 | Action 53 | 7 OK |
| 30 | UNROM 512 | 5 OK, 1 OPEN |
| 32 | Irem G-101 | 4 OK, 1 DIFFERS |
| 34 | BNROM and NINA-001 | 4 OK |
| 41 | Caltron 6-in-1 | 3 OK, 1 OPEN |
| 64 | Tengen RAMBO-1 | 4 OK |
| 71 | Camerica / Codemasters BF909x | 3 OK |
| 87, 101 | Jaleco J87 | 2 OK, 1 OPEN |
| 156 | Daou Infosys DIS23C01 | 2 OK |
| 225 | 52/64-in-1 multicart | 4 OK |
| 228 | Action 52 | 2 OK |

## Where the depth is uneven, and why

The brief ordered the boards most-played first and the lot's time went the same way. Boards 0,
1, 2, 3, 4, 5, 7, 9, 10, 11, 13, 15, 16, 18, 21-27, 28, 30, 32, 34 and 66 were read against all
four new sources, plus Mesen where MR-1 had not settled the point. For **64 (RAMBO-1)** only
puNES was read, and the report says so in the board's own section: MR-1b's RAMBO-1 verdicts rest
on a hardware capture thread rather than on any emulator, so a fourth and fifth emulator vote
could not move them. For **41, 71, 87/101, 156, 225 and 228** puNES was read in full and the
other three only where a point was in doubt; each of those sections names the files that were
not read. Anyone resuming should start there.

## What the lot found

The nine DIFFERS and eleven OPEN entries are in the report's summary table and in its "For the
owner" section. The three worth naming here:

1. **mapper 1, the SNROM `E` bit** — this build implements a PRG-RAM disable that the page and
   the `holy-mapperel` README both describe and that all five other emulators decline, one of
   them after a named regression. The weak part is the size heuristic that decides a board is an
   SNROM, which fires on every iNES 1.0 mapper-1 dump with CHR-RAM — i.e. on Zelda, Zelda II,
   Metroid and Final Fantasy.
2. **mapper 5, the split screen and extended attribute mode** — absent here, present in all
   five others. REQ-CORE-100 marks both P2, so this is a recorded gap and not a regression, but
   it is the largest visible difference in the whole set.
3. **mapper 13, bus conflicts** — the *CPROM* board page carries the same unqualified
   "Subject to bus conflicts: Yes" row that put mappers 11, 66 and 78 on
   `registry::has_bus_conflicts`, and MR-BUS-1's enumeration missed it. Mapper 41 may be a
   second instance; 41-1 records it as OPEN rather than asserting it.

And one finding that is not about a mapper: **the mapper-conformance test ROMs are already in
`assets-free` and the harness runs none of them.** `holy-mapperel/testroms` covers sixteen of
the numbers under review and `nes-test-roms` has `mmc3_test`, `mmc3_test_2`, `mmc3_irq_tests`,
`mmc5test`, `mmc5test_v2`, `exram`, `MMC1_A12` and `m22chrbankingtest`. Only `mmc3_test_2` is
run (REQ-CORE-114). Wiring the `holy-mapperel` set in would have settled four of the twenty
non-OK entries without any reading.

## Method notes for whoever resumes this

- Every `file:line` citation in the report was verified against the file it names, after being
  written. Keep doing that: the four new sources are large and the numbers do not survive
  guessing — this lot corrected about sixty of its own citations that way.
- Check that a cited **file** exists before citing it. Two did not on the first pass
  (`fceux/src/boards/64.cpp` is really `tengen.cpp`; mapper 71 is `71.cpp`), and mapper 28,
  which I had recorded as absent from puNES and Nintendulator, turned out to be implemented by
  both — which changed a three-way agreement into a five-way one and made it the strongest
  single result in the lot.
- `docs/review/MR-1-mappers.md` is the prior art. Its "Fixed in MF-1" / "Fixed in MF-2" lines
  say which findings are closed; this lot re-opened none of them and only revisited a point when
  one of the four new sources contradicted the conclusion.
- Where this build and the page agree **against** a majority of the emulators, the report still
  marks it DIFFERS, because the brief defines the verdict that way — but every such entry says
  explicitly that the page is on this build's side. MMC1-1 and 15-1 are of that shape, and so is
  the mapper-66 bus conflict, which is recorded as OK because the page's row is unqualified.

## Final check

- [x] every mapper number of the v0.4.2 set has a section — 26 board sections covering all 32
      numbers (`grep -n "^## Mapper" docs/review/LEGACY-XCHECK.md`)
- [x] `git status` shows only `rust/target-*` untracked
- [x] `git diff --stat 7dec9d7f` lists only `docs/review/LEGACY-XCHECK.md` and
      `docs/progress/LEGACY-XCHECK.md`
- [x] no absolute path containing a user name appears in either file
- [x] licence note present: the five GPL projects were read, not copied; no code transcribed
