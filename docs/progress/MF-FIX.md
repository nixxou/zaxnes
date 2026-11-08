# MF-FIX — the cross-check's changes

Lot: apply the six fixes `docs/review/Q-XCHECK.md` §"CHANGE — ready-to-use fix descriptions for a
later lot" leaves ready, plus Q-145's majority reading. Branch `mapper-review`, worktree
`wt-mappers`.

**Base commit**: `12c669a9` (`Q-XCHECK: appendix on the fifth source, …`). Every callgrind figure
and the golden-core digest are compared against that commit, each built from `git archive` in a
fresh directory.

**Scope**: mappers 12, 123, 143, 195, 262 and 292 only — all six were added by MF-UNL and appear in
no ROM of the 98 originals, so golden-core over the originals must stay byte-identical.

**Not in this lot**: CHANGE 3 and CHANGE 4 (both conditional on a ROM observation) and CHANGE 5 (a
separate lot).

## Steps

- [ ] 1/9 — the progress note and the base commit
- [ ] 2/9 — CHANGE 1: mapper 123's `$5800` bank field takes its bit 1 from register bit 3
- [ ] 3/9 — CHANGE 2: mapper 195's 4 KiB PRG-RAM window at `$5000-$5FFF`
- [ ] 4/9 — CHANGE 6: mapper 12's language bit reads `$01`
- [ ] 5/9 — CHANGE 7: mapper 262's CHR-RAM chip unbanked
- [ ] 6/9 — Q-145: mapper 143's protection read answers open bus in bits 7-6
- [ ] 7/9 — CHANGE 8: mapper 292 reads the game's zero page instead of the bus latch
- [ ] 8/9 — the callgrind campaign: `board-292` alone against the tolerance
- [ ] 9/9 — the final check: fmt, clippy ×4, tests, golden-core, the figures

## Log

### 1/9 — the progress note

Started from the branch tip `12c669a9`, working tree clean apart from the untracked build directory
`rust/target-mf/`. No earlier MF-FIX note existed, so this lot starts at step 1.
