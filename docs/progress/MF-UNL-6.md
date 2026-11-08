# MF-UNL-6 — COOLBOY / MINDKIDS

Lot **MF-UNL-6** of `docs/review/UNL-census.md` §7.2, the sixth of the twelve MF-UNL lots:
**one number, one board, one Cargo feature, ~12 original titles.**

Base commit: **`a4315007`** (`MF-UNL-5 (7/7)`), the branch tip when the lot opened.
Branch: `mapper-review`. Worktree: the `wt-mappers` worktree.

| feature | number | board |
| --- | ---: | --- |
| `board-268` | **268** | the **AA6023** ASIC (SMD132 / SMD133): an MMC3 clone with **six outer bank registers**, which take it from the MMC3's 512 KiB to **32 MiB** of PRG-ROM — 64 MiB on one PCB — with a GNROM-style banking mode, two "weird" modes, a mixed CHR-ROM/CHR-RAM window and a register lockout. UNIF `COOLBOY` (registers at $6000-$6FFF) and `MINDKIDS` (at $5000-$5FFF) |

## The one thing the census flags, and what this lot does with it

§7.2's own warning is that "the page lists thirteen PCB codes … and its documentation is
**partial**. A single implementation covering all thirteen is a guess; the lot should implement
what the page documents, name in the code which PCB codes it is known to serve, and leave the
rest refusing."

The page's own table indexes those PCB codes by **NES 2.0 submapper**, in six groups, and the
groups do not share a register layout: `$xxx0` has four distinct bit diagrams and `$xxx1` three.
So "implement what the page documents and leave the rest refusing" is read as **submappers 0 and
1**, which is one register layout, the `COOLBOY`/`MINDKIDS` port pair and **six** of the thirteen
PCB codes — and the six that carry all twelve original titles. Submappers 2-11 are refused at
load with REQ-CORE-011's typed cause rather than run on a layout that is not theirs.

**Mapper 224 is not added**, as §7.2 says: the same chipset, no title.

## Steps

- [ ] **0/8** — the plan: this file.
- [ ] **1/8** — `Mmc3Outer`: the six outer registers, banking mode `$00` (the oversize MMC3),
  the register lockout, the registry arms and the two PPU watch lists.
- [ ] **2/8** — the other three banking modes — `$10` (GNROM), `$40` and `$50` (the two "weird"
  ones) — the mixed CHR-ROM/CHR-RAM window of `$xxx4`, and the PRG-RAM window `$xxx3` bit 5 opens
  at $5000-$5FFF.
- [ ] **3/8** — the size ceiling: a 64 MiB image loads, a bigger declaration is refused, the
  SC0/SC1 half-select, and the `board_memory_sizes` row.
- [ ] **4/8** — the question tickets, the four sweeps, the save-state round-trip and the feature
  configurations.
- [ ] **5/8** — the callgrind campaign against the base commit, and the feature decision.
- [ ] **6/8** — golden-core against the base commit, and the full `--include-ignored` run with
  both ROM roots.
- [ ] **7/8** — `BOARD-FEATURES.md`, the census note, the journal and the final pass.

(Eight steps and not seven: the size ceiling §7.2 asks for in its step 2 is a loader question
rather than a board one, and it carries its own tests.)
