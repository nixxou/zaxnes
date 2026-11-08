# MF-UNL-7 — the Sachen 8259, the 206 clones and two homebrew boards

Lot **MF-UNL-7** of `docs/review/UNL-census.md` §7.2, the seventh of the twelve MF-UNL lots:
**nine numbers, six boards, six Cargo features, ~10 original titles.**

Base commit: **`fc64566a`** (`MF-UNL-6 (8/8)`), the branch tip when the lot opened.
Branch: `mapper-review`. Worktree: the `wt-mappers` worktree.

| feature | numbers | board |
| --- | ---: | --- |
| `board-sachen8259` | **137, 138, 139, 141** | the Sachen **8259**, one chip with four wirings (8259D, B, C, A), an index/data port pair at $4100/$4101 under mask $C101 |
| `board-112` | **112** | the 206's register file with the ports split across $8000/$A000/$E000 and the register order permuted |
| `board-193` | **193** | NTDEC's **TC-112**: three CHR registers over **unequal** windows at $6000-$6002, one PRG window, mirroring at $6004 |
| `board-298` | **298** | a VRC4 clone in the **VRC4b** configuration whose $F003 "only acknowledges a pending IRQ" |
| `board-218` | **218** | the single-chip homebrew board of *Magic Floor*: **no CHR memory at all**, the 2 KiB of CIRAM serving the pattern tables |
| `board-29` | **29** | the Sealie **RET-CUFROM**: one $8000 register, two bits of CHR-RAM bank and three of PRG bank |

## The three things this lot has to carry from the census

### 1. The 8259's documentation caveat, and the attribution gap

§7.2 quotes the page on itself: its information "was gathered primarily from FCEU-mm and while it
appears to be a good enough behavioral description to run many ROMs, it is likely incomplete",
and it governs four numbers at once. So the lot implements the four variants from the page's own
A/B/C/D rewiring table and **records the attribution gap in the code comment**: neither NESdev nor
Wikipedia attributes *The Great Wall* and *Q-Boy* to 137/138/139/141 beyond what the pages do say
— 137's own page gives *The Great Wall* as "the only known 8259D game", and the 8259 page gives
*Q-Boy* as the **2M-RAM-COB** board, "usually set to mapper 141". Nothing is guessed past that.

### 2. MF-UNL-2's rule: where §7.2 and a number's own page differ, the page wins

Checked step by step as the lot goes, and each case gets a Q ticket.

### 3. The performance rule as the instruction states it

The +0.02 % tolerance is on **each ROM's total** instruction count — the PR-1 set and the level-2
games' `total` — and not only on the `core` column. A total that moves further than that with the
cause outside the emulation (the Lua `ltable.c` column is the one this project has seen) is shown
with a same-behaviour pair and recorded; the tolerance is not redefined.

## Steps

- [x] **0/9** — the plan: this file.
- [ ] **1/9** — `board-sachen8259`: the index/data pair, registers 0-5 and 7, the per-variant CHR
  shift, the four mirroring modes and the CHR-RAM board.
- [ ] **2/9** — `board-112`: the 206 with the page's three ports and its permuted register order.
- [ ] **3/9** — `board-193`: the TC-112's three unequal CHR windows and its $6004 mirroring.
- [ ] **4/9** — `board-298`: the VRC4b `const` selection and the one documented difference.
- [ ] **5/9** — `board-218`: a board with no CHR memory, four CIRAM A10 wirings.
- [ ] **6/9** — `board-29`: the RET-CUFROM's one register, and the flash path that is not emulated.
- [ ] **7/9** — the question tickets, the four sweeps, the save-state round-trips and the feature
  configurations.
- [ ] **8/9** — the callgrind campaign against the base commit, and the feature decision per board.
- [ ] **9/9** — golden-core, the full `--include-ignored` run with both ROM roots,
  `BOARD-FEATURES.md`, the census note, the journal and the final pass.

(Ten entries and not eight: §7.2's step list puts 218 and 29 in one step, and they share nothing —
one is the only board in this build with no CHR memory and the other is a plain latch — so each
takes its own.)
