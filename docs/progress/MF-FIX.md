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

- [x] 1/10 — the progress note and the base commit
- [ ] 2/10 — CHANGE 1: mapper 123's `$5800` bank field takes its bit 1 from register bit 3
- [ ] 3/10 — CHANGE 2: mapper 195's 4 KiB PRG-RAM window at `$5000-$5FFF`
- [ ] 4/10 — CHANGE 6: mapper 12's language bit reads `$01`
- [ ] 5/10 — CHANGE 7: mapper 262's CHR-RAM chip unbanked
- [ ] 6/10 — Q-145: mapper 143's protection read answers open bus in bits 7-6
- [ ] 7/10 — CHANGE 8: mapper 292 reads the game's zero page instead of the bus latch
- [ ] 8/10 — the ticket and report annotations ("Fixed in MF-FIX (`<hash>`)")
- [ ] 9/10 — the callgrind campaign: `board-292` alone against the tolerance
- [ ] 10/10 — the final check: fmt, clippy ×4, tests, golden-core, the figures

The annotations are a step of their own because the protocol forbids writing a hash that does not
exist yet: every "Fixed in MF-FIX (`<hash>`)" line names the commit of the step that made the
change, so they can only be written once steps 2 to 7 are committed.

## Log

### 1/10 — the progress note

Started from the branch tip `12c669a9`, working tree clean apart from the untracked build directory
`rust/target-mf/`. No earlier MF-FIX note existed, so this lot starts at step 1.

### 2/10 — CHANGE 1, mapper 123's `$5800` bank field

`Mmc3Scramble::apply_override`, the `OverridePort::Port5800` arm
(`rust/crates/zax-nes/src/cartridges/mappers/mmc3_scramble.rs`): the bank's bit 1 now comes from
register bit **3**, so `((register >> 3) & 0x02)` became `((register >> 2) & 0x02)`. Register bit 4
is the don't-care. Bank bits 3, 2 and 0 (register bits 5, 2 and 0), `S` at bit 1 and `M` at bit 6
are untouched.

Why: nesdev's mapper-123 page contradicts itself — its diagram `[.M31 .2S0]` labels register bit 4
as the bank's bit 1, while the page's prose says twice to "ignore `$5800` bits 0-3/5", which is only
true of a field whose bits are 5, 3, 2 and 0. All four cross-checked projects (Mesen 2, FCEUX,
Nestopia UE, puNES) read bit 3, and `third_party/nintendulator-mappers` makes it four against one.
So the diagram is the error. Q-XCHECK §CHANGE 1.

Also updated: the type's doc-comment table row (`bits 5, 4, 2, 0` → `bits 5, 3, 2, 0`) and the
paragraph that used to claim the page's reading against Mesen 2's, which now records the page's
self-contradiction and the four-way agreement.

**Tests**: no existing test walked `$5800` with bits 3 and 4 unequal — the two cases that write the
port (`req_core_100_the_nrom_override_takes_prg_away_from_the_mmc3` uses bank 5, bits 2 and 0; the
save-state and reset sweeps write `$65`) are blind to the difference, and `$65`'s bank is 13 under
both readings. So the lot **adds**
`q_144_mapper_123_takes_the_banks_bit_1_from_register_bit_3`, which walks the two bits one at a
time: `$48` must move the bank to 2, `$50` must leave it at 0, `$58` must still be 2 (which rules
out an implementation that ORs both bits in), and `$65` must still be bank 13. Under the old
reading the first two assertions fail the other way round, so the test is a real discriminator and
keeping the bit-4 case is what stops the old reading coming back silently.

Titles affected: *Mortal Kombat 3* (UNL-H2288) and *Earthworm Jim 2* (Kǎshèng / Shin-Shin) — half
of the `$5800` values previously loaded the wrong 16 KiB PRG bank.

Risk: none to other boards. `OverridePort::Port5800` is mapper 123's only port; 114, 115, 121, 182
and 187 take other arms of the same `match`. No save-state byte moves, no `Board` dispatch changes,
no hot path.

`cargo test --release -p zax-nes`: all green (386 in `mapper_tests`, the new test included).
