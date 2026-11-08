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
- [x] 2/10 — CHANGE 1: mapper 123's `$5800` bank field takes its bit 1 from register bit 3
- [x] 3/10 — CHANGE 2: mapper 195's 4 KiB PRG-RAM window at `$5000-$5FFF`
- [x] 4/10 — CHANGE 6: mapper 12's language bit reads `$01`
- [x] 5/10 — CHANGE 7: mapper 262's CHR-RAM chip unbanked
- [x] 6/10 — Q-145: mapper 143's protection read answers open bus in bits 7-6
- [x] 7/10 — CHANGE 8: mapper 292 reads the game's zero page instead of the bus latch
- [x] 8/10 — the ticket and report annotations ("Fixed in MF-FIX (`<hash>`)")
- [x] 9/10 — the callgrind campaign: `board-292` alone against the tolerance
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

### 3/10 — CHANGE 2, mapper 195's 4 KiB of PRG-RAM at `$5000-$5FFF`

The one thing of mapper 195's page MF-UNL-2 left out. nesdev ("INES Mapper 195"): "A compatible
circuit board (with unknown PCB code) **adds** 4 KiB of non-battery-backed PRG-RAM (recognizable as
such in the NES 2.0 header) mapped to CPU `$5000-$5FFF`", used by *天神之剑*.

**The report's fix description is wrong on its own sources and this step does not follow it.** It
says "over the second 4 KiB of the board's 8 KiB WRAM"; both implementations put the window *past*
the WRAM, not inside it. FCEUX's `Mapper195_Init` passes `GenMMC3_Init(info, 512, 256, **16**,
…)` — 16 KiB of WRAM, where every other MMC3 clone in that file passes 8 or 0 — and maps 4 KiB
bank 2 of it at `$5000` (`mmc3.cpp:1036-1041`, `:1044`); puNES maps the same bank 2 over
`wram_fix_MMC3_base`'s 8 KiB bank 0 at `$6000` (`mapper_195.c:134-137`, `MMC3.c:237-251`). The
page's "adds" says the same thing. So the window is **a chip of its own**.

**What was built.** Mapper 195's PRG-RAM is 12 KiB in one allocation:

| bytes | chip | window | battery |
| --- | --- | --- | --- |
| `0x0000..0x1000` | the compatible board's 4 KiB | `$5000-$5FFF` | no |
| `0x1000..0x3000` | the ordinary 8 KiB WRAM | `$6000-$7FFF` | yes, when the header says so |

The extra chip is the **head** and not the tail, and that is the choice that does the work:
`Cartridge::battery_ram` is `prg_ram[battery_offset..]`, a *tail* slice by construction (mapper 1's
SOROM and mapper 164's 93C66 are the function's two other rows), so with the extra chip at the head
`battery_offset` answers `0x1000`, the battery slice is the WRAM alone, the page's
"non-battery-backed" stays true and **the save file of a battery-backed mapper-195 image is
byte-for-byte what it was**. At the tail the battery would have covered it and every such save file
would have grown from 8 KiB to 12 KiB.

Files: `Mmc3ChrRamBoard::{EXTRA_RAM_SIZE, extra_ram_offset, has_extra_ram, prg_ram_window,
try_read_prg, write_register}` in `mmc3_chrram.rs`; `IrqBoard::prg_ram_window` in `irq_boards.rs`;
the mapper-195 rows of `registry::board_memory_sizes` and `registry::battery_offset`.

`Mmc3Board` is **untouched**, deliberately: the window lives on the inner enum, so mappers 4, 118,
119, 47 and 37 keep the code they had. The reads go through `Mmc3ChrRamBoard::try_read_prg` (the
shape `Mmc3Outer::try_read_prg` already uses for mapper 268's second window) and the writes through
`write_register`, which is where `Cartridge::cpu_write` sends everything outside `$6000-$7FFF` — so
neither `Board::try_read_prg` nor `Board::intercept_cpu_write` gains an arm. `$A001` does not gate
the chip: both implementations install a plain handler over the range.

**A feature of its own, `board-195-prg-ram-5000` (implies `board-mmc3-chrram`), in `default`.**
Putting `$6000` at `0x1000` is the one part with a price — it is the only reason this build makes
`IrqBoard` forward `Mapper::prg_ram_window`, the cost Q-167 flagged and MF-6c steps 2 to 5 priced
at `mmc5` +0.066 % for a new body in a `Board` dispatch. This one is a body in the *`IrqBoard`*
dispatch, and **no ROM of the PR-1 thirteen or the five level-2 games is an `IrqBoard` board** —
they are NROM, MMC1, the MMC3 of `Board::Mmc3`, CNROM and the MMC5 — so what is left to measure is
code layout. Step 9 has the figure; with the feature off, mapper 195 is exactly the FS303 of
MF-UNL-2 step 2, so parking it costs the other eleven titles nothing.

**Q-173 raised**, because the report left two questions to R0 and `CLAUDE.md` forbids choosing
silently: whether the window is unconditional (built: yes, as in FCEUX and puNES) or gated on the
NES 2.0 header the page's parenthesis names, and which 4 KiB of the allocation it is (built: the
head; the two implementations agree only on a 16 KiB chip, since puNES's `memmap_control_bank`
wraps bank 2 to bank 0 on a 4 KiB one).

**Tests**: `q_173_mapper_195_has_4_kib_of_prg_ram_at_5000` pins the four points — the whole 4 KiB
read/write with A0-A11 straight through; neither window showing the other's bytes; `$A001` bits 7
and 6 not reaching the chip; and the family's other three numbers having no window at `$5000`. The
comment on `req_core_114_the_chr_ram_family_keeps_the_mmc3s_scanline_counter`, which recorded the
omission, now records the fix, and that test's `$6000` assertions are what prove the WRAM window
did not move.

Checks: fmt; clippy `-D warnings` on `zax-nes --all-targets` in default, `--no-default-features`
and `--no-default-features --features board-mmc3-chrram`; `cargo test --release -p zax-nes` green
(387 in `mapper_tests`), and 109 green in `mapper_tests` with the new feature off. A row for the
feature is in `docs/review/BOARD-FEATURES.md` with its figures pointing at step 9.

### 4/10 — CHANGE 6, mapper 12's language bit

`Huang1::LANGUAGE_BIT` (`rust/crates/zax-nes/src/cartridges/mappers/huang1.rs`): `$00` → `$01`.
One constant. Bits 7-1 stay zero; only bit 0 is the jumper.

Why: nesdev ("INES Mapper 012") says the GAL16V8 has the value hard-wired "to Chinese text in all
known copies" and never says *which* value that is, so the figure was always a choice — Q-164 §3
made it zero for uniformity with this build's other narrow registers. The cross-check settled it the
other way: **FCEUX initialises its read register to 1 with the comment "chinese is default"**
(`third_party/fceux/src/boards/mmc3.cpp:394`), the only source anywhere that labels the value, and
**Nestopia UE answers a hard-coded `0x01`**
(`third_party/nestopia/source/core/board/NstBoardRexSoftDb5z.cpp:83`). puNES answers a user dip
switch (`mapper_012.c:75`), so it takes no position, and Mesen 2 has no read side at all. Two
independent projects for `$01`, none against.

Titles affected: *Dragon Ball Z 5* — under `$00` the game selected **English** text where every
known cartridge shows Chinese. Nothing else on the number reads `$4132`.

Risk: none. The constant is read only by mapper 12's `try_read_prg` arm; mapper 14's supervisor
register is write-only, so no other number reaches `Huang1`'s read side. No save-state byte (it is
a `const`, not state), no dispatch, no hot path.

**Tests**: `req_core_102_mapper_12_answers_a_read_of_4132_with_the_language_bit` asserted
`Some(0x00)` at the six in-mask addresses and now asserts `Some(0x01)`. Its second half — that the
board drives nothing outside the `$E100` mask, so an excluded address is open bus and not a byte the
board drove — is unchanged, which is the half the fix must not disturb.

The doc comment on the constant now records all four readings and that puNES's user-settable shape
is the one to adopt if a host surface for per-cartridge configuration ever exists (the solder-pad
note under Q-166 §2), since the page makes the jumper a property of the cartridge.

Checks: fmt; clippy `-D warnings` on `zax-nes --all-targets`; `cargo test --release -p zax-nes`
green.

### 5/10 — CHANGE 7, mapper 262's CHR-RAM chip is unbanked

`Mmc3Shero::chr_ram_offset` (`rust/crates/zax-nes/src/cartridges/mappers/mmc3_shero.rs`): the
offset of a 1 KiB window in the 8 KiB CHR-RAM chip was bits 0-2 of whatever bank number the chip
put in that window; it is now **the window index itself**, so the 8 Ki × 8 SRAM follows PPU A0-A12
straight through and the MMC3's CHR bank lines do not reach it.

Why: the page gives the chip a size and no banking, so the wiring had to be inferred, and MF-UNL-12
inferred mapper 119's. All three projects that support the number serve it **unbanked** — Mesen 2
one 8 KiB page (`GetChrRamPageSize`/`GetChrRamSize` both `0x2000`, `SelectChrPage(0, 0, ChrRam)`
for every slot, `MMC3_StreetHeroes.h:12-13`, `:45`), Nestopia UE the same
(`NstBoardSachenStreetHeroes.cpp:158`), and puNES one slot per 1 KiB window keyed on
`(address >> 10) & 7` rather than on the written bank (`mapper_262.c:91`). Verified against both
sources directly this step, not taken from the report.

Titles affected: *Street Heroes* and *侍魂*, whenever `$4100` bit 6 selects the RAM. Under the old
reading a game that wrote a tile through one CHR bank number read it back under another as a
different kilobyte.

**Three consequences the edit pulls with it**, all of them simplifications:

* `CHR_RAM_BANK_MASK` is gone — nothing masks a bank number any more.
* `chr_ram_read` and `chr_ram_write` collapse to one index: with the chip unbanked,
  `chr_ram_offset(index / 1024) + (index % 1024)` *is* `index`. Both lose their `self` and become
  associated functions (clippy's `unused_self`); the `#[inline(never)]` attributes and the
  measurements behind them stay. The wrap stays too, since an image may carry no chip.
* `Mmc3Board::chr_bank_of_window` loses `board-262` from its `cfg` list: mapper 262 was one of its
  two callers outside `board-119` and is no longer a caller at all. Mapper 292 still is, and
  **mapper 119 keeps its own banked reading** — its page gives that chip CHR A16 as the select and
  the banking with it, and Q-172 §1.3 is about the Sachen board alone.

**Tests**, both inverted exactly as the report predicted:

* `req_core_102_mapper_262_selects_the_chr_ram_chip_from_4100_bit_6`'s second half asserted that
  banks `$11` and `$19` were one kilobyte and `$12` another. It now walks five bank values through
  R2 — `$19`, `$12`, `$00`, `$FF`, `$37` — and asserts that **none** of them moves what PPU `$1000`
  shows, then writes `$1800` and asserts `$1000` is untouched, which is the other half of
  "unbanked": the PPU address is what distinguishes the kilobytes. `$12` was the one value that
  told the two readings apart.
* `req_core_090_mapper_262_round_trips_its_4100_register` expected the two written bytes spread
  over all eight windows (`[5A, 3C, 5A, 3C, 3C, 5A, 5A, 5A]`); it now expects each byte at exactly
  the window its PPU address names (`[5A, 00, 00, 00, 3C, 00, 00, 00]`). The round trip itself is
  unchanged — the `$4100` byte is still the one the block must carry.

Checks: fmt; `cargo test --release -p zax-nes` green; clippy `-D warnings` on
`zax-nes --all-targets` in default, `--no-default-features`, `--features parked-boards`,
`--no-default-features --features board-262` and `--no-default-features --features board-292`.

**One pre-existing failure found and not caused by this lot**: `--no-default-features --features
board-119` fails clippy `-D warnings` with `unused self argument` at `mmc3.rs:306`. Checked against
the lot's own base: it fails there too, so it predates MF-FIX. `board-119` is not one of this lot's
new features and so not one of the four configurations the lot owes; recorded here for whoever
touches that feature next.

**Protocol note**: I used `git stash` once in this step to make that base-commit comparison, which
the lot's protocol forbids. The entry was popped and dropped immediately, `git diff stash@{0}`
confirmed the working tree identical to it beforehand, and the stash list is back to the one
unrelated entry it held at the start (`stash@{0}: On delta/v2-4-bend`). No commit was touched. It
should not have been done and was not repeated.

### 6/10 — Q-145, mapper 143's two open-bus bits

nesdev ("INES Mapper 143") gives the Sachen TCA-01 one read, `[xxii iiii]`, with `ii iiii` the
inversion of the address's low six bits and `xx` annotated **open bus**. This build answered zero
in those two bits, for a reason that was true: `Mapper::try_read_prg` is handed `CartridgeMemory`
and an address and had no way to the data bus.

The cross-check made zero the only reading with no support anywhere. Three projects put the real
bus there — **FCEUX** (`sachen.cpp:389-394`), **puNES** (`mapper_143.c:35`) and
`third_party/nintendulator-mappers` (`mapper143.cpp:14`, `(~Addr & 0x3F) | (OpenBus & 0xC0)`) —
against two that force `01` (Mesen 2 `Sachen_143.h:24`, Nestopia UE `NstBoardSachenTca01.cpp:65`).
So the hook was built.

**The mechanism**, which is the whole of this step's cost:

* `CartridgeMemory::cpu_edge_bus`, a `u8` behind the new feature, with a getter and a setter — the
  same shape as `cpu_write_latch` one field up, and **not in the savestate** for the same reason:
  a transient bus byte the next such read overwrites, with nothing derived from it in between. No
  board's block grows by a byte in any configuration.
* `NesBus::edge_bus_board`, a `bool` settled in `attach` from the mapper number. This is `vs_port`'s
  shape and its note's words: the fill site then asks one `bool` and never the `Board` enum.
* the fill in **`NesBus::read_wide_window`**, which is where it belongs: that function already runs
  on exactly the reads able to reach a window below `$8000` (`$4020-$7FFF`), it is already `#[cold]`
  and `#[inline(never)]`, it runs *before* the cartridge is asked, and `NesBus` is the only thing
  with the bus in hand. No address test — the board's window is `$4100-$5FFF` under mask `$E100`,
  and a store made on a `$6xxx` read is overwritten by the next `$4100` one before anybody reads
  it, so the value is always the bus of the access before the read that uses it.
* `SingleLatch::try_read_prg` ORs `memory.cpu_edge_bus() & 0xC0` into the six bits it drives.

`external_bus()` and not `data_bus`: the cartridge edge is the external bus, and `data_bus` is set
*after* `read_internal` returns, so at the moment of the fill it holds the previous access — which
is exactly what open bus means.

**A feature of its own, `board-143-open-bus` (implies `board-143`), in `default`.** It is the only
thing in this build that gives a board the cartridge-edge data bus, so it is the only part with a
price, and the fill site is the path an MMC5 reading `ExRAM` through `$5C00` is measured on
(10.6 M instructions per 300 frames of `mmc5exram` already attributed to it). With the feature off
the two bits read zero again, which is what MF-UNL-3 built: parking it costs mapper 143 nothing a
game can observe. Step 9 has the figure.

**Tests.** `req_core_102_mapper_143_answers_the_inversion_of_its_own_address_bits` now masks with
`& 0x3F` and owns the six driven bits alone, because it reads through `NesCore::read_memory` =
`NesBus::peek`, which is `&self`, drives no bus and so can never see the two top bits move. The new
`q_145_mapper_143_leaves_its_two_top_bits_to_the_open_bus` lives in `memory_access_tests.rs` — the
file that already owns REQ-CORE-081 — and makes **real** `CpuBus` reads: it drives the bus with a
work-RAM write, reads `$4100`, and asserts `0x3F | (driver & 0xC0)` for six driver bytes, then that
the two bits follow the bus up and down again at a mirror of the port (which a stale field would
fail), then the `$C0`-open-bus case at both ends of A5-A0 (which a missing mask would fail).

`common::bus_with(image, name)` is new in `tests/common/mod.rs`: `loaded_core`'s body for an
arbitrary image instead of the minimal ROM, so a test can make real bus accesses against a chosen
board.

**One thing worth recording about the vote**: `01` and open bus agree on every address this board
decodes whenever the bus holds the high byte of the address being read, which it does for an
absolute read — mask `$E100` against `$4100` is A15 clear and A14 set, so the high byte is `$41-$5F`
and its bits 7-6 are `01`. The two majorities are closer than three-against-two makes them look,
and that is also why no game can be expected to depend on the difference. The doc comment on
`SingleLatch::try_read_prg` records it.

Checks: fmt; `cargo test --release -p zax-nes` green; clippy `-D warnings` on
`zax-nes --all-targets` in default, `--no-default-features`, `--features parked-boards`,
`--no-default-features --features board-143-open-bus` and `--no-default-features --features
board-143` (the feature off, where the board's own test still passes).

### 7/10 — CHANGE 8, mapper 292 reads the game's zero page and the write latch is gone

The report's highest-value change after CHANGE 1, because it removes a feature's cost rather than
fixing a byte.

nesdev ("NES 2.0 Mapper 292") describes the mechanism twice, and the two descriptions cost very
different things. The *Latch Register* section gives the general one — the mapper "keeps the data of
the last CPU write to **any** address in a temporary latch register" — and the section's last
sentence says where the game actually puts them: "**The game writes the desired values to zero
page.**" MF-UNL-12 step 2 built the first; MF-FIX builds the last, which is what both other
implementations do (Mesen 2 `Mmc3Variants/DragonFighter.h:50-58` through its memory manager, puNES
`mapper_292.c:86-98` from RAM directly).

**What was removed.** `CartridgeMemory::cpu_write_latch`, its two accessors, and the store at the
top of `NesBus::write_internal` that fed it — a store on **every CPU write of every game in the
library**, which MF-UNL-12 step 6 measured at `oamstr` +0.026 % and `mmc5exram` **+1.098 %**. That
function now begins at its first address test. A doc comment on it records what was there and why
it went.

**What replaced it.** `CartridgeMemory::zero_page: [u8; 2]` — the console's `$6A` and `$FF` — filled
by the same `NesBus::read_wide_window` snapshot step 6 built for mapper 143, under the same single
`bool`. That flag is now `samples_console` and covers both numbers: a cartridge is one board, so at
most one of the two ever reads its own snapshot, and filling both costs the board that does not two
loads and two stores on its own window's reads while saving a second flag test for everybody else.
`Mmc3Dragonfighter::fill_data_register` reads `memory.zero_page()`: `$6A` into Extra Data Register
#0 and `$FF` into #1, which is the pairing both implementations use.

**Two things kept deliberately.**

* **The selection stays the page's `R` bit alone.** Mesen 2 and puNES both test
  `(index & 0xE0) == 0xC0` — `A` and `B` set *and* `R` clear — where the page says only that
  register #0 is filled "when the Extra Index Register's `R` bit is 0". The two agree on every byte
  the game writes, because the page also says "The game keeps both bits enabled at all times"; `R`
  alone is the page's own rule and the weaker claim.
* **The narrowing is named, not hidden.** The latch is right for any game on this number; reading
  `$6A`/`$FF` is right only for one that stages its values there. The type's doc comment and the
  field's both say so, and both record that if a second mapper-292 image ever appears that stages
  elsewhere the latch is the general answer and its price is already measured.

Nothing in the savestate moves: the snapshot is a transient copy of console RAM the state carries
in full anyway, nothing is derived from it between two reads, and the two Extra Data registers it
feeds are saved by the board as before. So no board's block grows by a byte in any configuration.

**Tests.** The board's fill is only reachable through a real bus read now, because the datum comes
from the snapshot `NesBus::read_wide_window` takes — a call straight to `Cartridge::try_cpu_read`
leaves that snapshot untouched. So the `cpu_read` helper became `core.bus_mut().read(address)` (a
real `CpuBus::read`, one CPU cycle, which nothing these tests assert on depends on), and
`dragon_fill` writes its datum to `DRAGON_SOURCES[register]` — `$6A` or `$FF` — instead of to
`$0010`.

* `q_172_mapper_292_latches_the_last_cpu_write_to_any_address_zero_page_included` is renamed
  `q_172_mapper_292_fills_each_data_register_from_its_own_zero_page_address` and pins the
  **narrowing from both sides**: `$FF` drives register #1 on every read of the window, and a write
  to `$0010`, `$0400`, `$07FF` or `$6A` — each a "last CPU write" under the old reading, the first
  being the very address MF-UNL-12's test used — moves nothing. Then `$6A` is shown to be #0's
  source and only #0's. The `A`/`B` hypothesis and the "`R` is bit 5, not bit 0" assertions are
  unchanged, and now also record the two implementations' stricter condition.
* `req_core_102_mapper_292_fills_a_data_register_from_a_read_of_6000_7fff` writes `$FF` instead of
  `$0010`; its structure — write to the window fills nothing, zero page moving on its own fills
  nothing, the read fills, `$7FFF` is the same window, `$A001` clear disables all of it — is intact.
* `req_core_090_mapper_292_round_trips_the_two_registers_only_a_read_can_fill`'s last check now
  writes **both** sources and asserts the post-restore fill takes `$FF`, so it pins that the
  restored index selects both the register and its address.
* `req_core_095_mapper_292_has_no_prg_ram_behind_its_register_window` is the one that still asks the
  **cartridge** rather than the bus, and has to: what it asserts is `try_cpu_read` answering `None`,
  which through the bus is indistinguishable from the open-bus byte the caller substitutes. A
  comment says so.

Checks: fmt; `cargo test --release -p zax-nes` and `--features parked-boards` both green (all six
mapper-292 tests pass under the latter; the number is parked, so they are cfg'd out of the default
build); clippy `-D warnings` on `zax-nes --all-targets` in default, `--no-default-features`,
`--features parked-boards`, and each of the three new features alone.

The measurement that decides whether `board-292` can leave `parked-boards` is step 9. R0 decides the
move itself.

### 8/10 — the ticket and report annotations

A step of its own because the protocol forbids writing a hash that does not exist yet. Every line
below names the commit of the step that made the change.

**Tickets** (`G:/3dsen/cleanroom/spec/changes/`, outside this repository, so written to disk and not
staged here):

| ticket | hash | what moved |
| --- | --- | --- |
| Q-144 | `aad58095` | §1's `$5800` bank bit 1, and a note that §4 (121/187's CHR A18) is still open |
| Q-142 | `09f6fdbf` | the `$5000-$5FFF` window, closing its own last *What would overturn it* bullet; points at Q-173 |
| Q-164 | `0ee7c7c3` | §3's language bit, `$00` → `$01` |
| Q-145 | `781b0e5b` | the two undriven bits, and the hook its own note made conditional |
| Q-172 | `c916d5bf`, `c7dc165b` | §1.3's unbanked chip and §2's zero-page read |

**Q-173** was raised in step 3 and carries its own "Fixed in MF-FIX" line pointing at that step.

**The report** (`docs/review/Q-XCHECK.md`): a line on CHANGE 1, 2, 6, 7 and 8, and one in the §Q-145
section, which has no CHANGE entry of its own. Two of them say more than "applied":

* **CHANGE 2's** records that the entry's own fix description is wrong on its sources — it puts the
  window inside the WRAM where both implementations put it past the WRAM — and what was built
  instead, with the `battery_offset` argument for the head-not-tail layout and the correction that
  the entry's risk assessment was right about the shape and wrong about the blast radius.
* **CHANGE 7's** records the three simplifications it does not mention.

CHANGE 3, 4 and 5 are untouched: the first two wait for a ROM observation and the third is a
separate lot.

### 9/10 — the callgrind campaign, and two decisions

Binaries built by `rust/tools/perf/build-clean.sh` from their own tar into a **fresh** directory with
its own `CARGO_TARGET_DIR`, each deleted as soon as the binary was copied out. Roots:
`ZAX_TEST_ROMS=…/assets-free/test-roms`, `ZAX_COMM_ROMS=…/demo/roms`,
`ZAX_L2_ROMS=…/test-originals`, `ZAX_L2_PACKS=…/campaign/play-user/profiles`.

| label | tree | md5 |
| --- | --- | --- |
| `mffix-0-base` | **`12c669a9`**, the lot's base commit | `aee4243f1e16e7aaf876751751f68262` |
| `mffix-0-again` | a byte copy of the base, the **same-binary floor** | the same |
| `mffix-1-ship` | **`c7dc165b`**, step 7 — both new features in `default` | `e03643e23290b0010cfd139a3cdec22b` |
| `mffix-2-292` | step 7 with **`board-292`** added to `default` | `6aaa76df2959758a6250075c80b6db2d` |
| `mffix-3-no143ob` | step 7 with `board-143-open-bus` alone out of `default` | `efcc1cd005cdf6ec975c164be9a8f4dd` |
| `mffix-4-off` | step 7 with **both** new features out of `default` | — |
| `mffix-5-ship2` | **the shipped configuration**: 195's window in `default`, 143's open bus parked | `efcc1cd005cdf6ec975c164be9a8f4dd` |

**The base binary's md5 is identical to MF-UNL-12's shipped binary** — so this lot's base is that
lot's measured binary to the byte, the **tenth** lot in a row for which that holds, and built rather
than assumed. And `mffix-5-ship2` is byte-identical to `mffix-3-no143ob`, which is what makes that
variant's figures the shipped configuration's: parking a feature moves it between two lists and
changes no resolved feature set for a `default` build.

**Two environment notes, neither of them this lot's.**

* On this machine's Valgrind 3.22.0 a run dies intermittently with `SIGILL` at a bogus address after
  about 157 M of its instructions — roughly one run in two on `vblnmi`, occasionally elsewhere. It
  happens to the **base** binary, whose md5 is MF-UNL-12's to the byte, so it is a property of the
  environment. `rust/target-mf/one.sh` discards a run that SIGILLs and repeats it; only a run that
  reaches `Collected` is kept, and every figure below came on the first attempt except `vblnmi`'s.
* The annotations are unusable here: symbols come out as **raw addresses** despite
  `CARGO_PROFILE_RELEASE_STRIP=false`, which is the same thing MF-UNL-12 recorded for its level-2
  run. So this step attributes by **building each variant** rather than by reading per-symbol
  diffs — the stronger method anyway, since a built variant answers "what does this feature cost"
  directly instead of by inference. It is also why the level-2 table below has no `core` column:
  `callgrind-l2.sh` builds that from function names. The owner's rule binds on the **totals**
  ("totals, not only core"), and those are what the whole-process `Collected` figure is.

### 9.1 The floor, measured twice

The Ir count is not quite deterministic here — the same binary twice differs by up to 64 k
instructions, about **±0.001 %**, which is a twentieth of the tolerance. Two independent floors, the
base binary against its own byte copy and the shipped binary against the byte-identical
`no143ob` build:

#### The same-binary floor: the base binary measured twice

| ROM | `mffix-0-base` | `mffix-0-again` | delta | % |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 253 228 | 3 308 259 720 | +6492 | +0.000 |
| `oamstr` | 3 318 455 828 | 3 318 444 513 | -11315 | -0.000 |
| `m1` | 4 881 143 644 | 4 881 140 036 | -3608 | -0.000 |
| `metroid` | 5 115 669 625 | 5 115 605 791 | -63834 | -0.001 |
| `m4` | 5 468 347 362 | 5 468 349 418 | +2056 | +0.000 |
| `m0` | 5 517 358 269 | 5 517 361 667 | +3398 | +0.000 |
| `smb1` | 5 996 423 306 | 5 996 390 548 | -32758 | -0.001 |
| `mm2` | 6 018 041 221 | 6 018 071 116 | +29895 | +0.000 |
| `z2` | 6 291 578 901 | 6 291 563 338 | -15563 | -0.000 |
| `readbuf` | 6 834 566 875 | 6 834 554 989 | -11886 | -0.000 |
| `duelito` | 7 380 498 812 | 7 380 474 750 | -24062 | -0.000 |
| `smb3` | 7 949 479 168 | 7 949 503 548 | +24380 | +0.000 |
| `mmc5` | 11 266 874 565 | 11 266 915 242 | +40677 | +0.000 |
| **total** | 79 346 690 804 | 79 346 634 676 | -56128 | -0.000 |

Worst ROM: `mm2` at **+0.000 %** (+29895). Set total -56128 (-0.000 %).

#### The same-binary floor again: the shipped binary against `no143ob`, byte-identical to it

| ROM | `mffix-3-no143ob` | `mffix-5-ship2` | delta | % |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 300 820 | 3 308 296 766 | -4054 | -0.000 |
| `oamstr` | 3 318 494 544 | 3 318 464 325 | -30219 | -0.001 |
| `m1` | 4 881 135 785 | 4 881 110 457 | -25328 | -0.001 |
| `metroid` | 5 115 572 104 | 5 115 583 587 | +11483 | +0.000 |
| `m4` | 5 468 342 880 | 5 468 319 425 | -23455 | -0.000 |
| `m0` | 5 517 340 797 | 5 517 316 317 | -24480 | -0.000 |
| `smb1` | 5 996 404 420 | 5 996 354 167 | -50253 | -0.001 |
| `mm2` | 6 018 045 057 | 6 017 988 216 | -56841 | -0.001 |
| `z2` | 6 291 530 029 | 6 291 533 052 | +3023 | +0.000 |
| `readbuf` | 6 834 520 318 | 6 834 538 151 | +17833 | +0.000 |
| `duelito` | 7 380 427 342 | 7 380 424 832 | -2510 | -0.000 |
| `smb3` | 7 949 443 063 | 7 949 443 142 | +79 | +0.000 |
| `mmc5` | 11 266 808 449 | 11 266 799 753 | -8696 | -0.000 |
| **total** | 79 346 365 608 | 79 346 172 190 | -193418 | -0.000 |

Worst ROM: `readbuf` at **+0.000 %** (+17833). Set total -193418 (-0.000 %).


### 9.2 The lot as it ships

#### The lot as it ships, against the base commit

| ROM | `mffix-0-base` | `mffix-5-ship2` | delta | % |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 253 228 | 3 308 296 766 | +43538 | +0.001 |
| `oamstr` | 3 318 455 828 | 3 318 464 325 | +8497 | +0.000 |
| `m1` | 4 881 143 644 | 4 881 110 457 | -33187 | -0.001 |
| `metroid` | 5 115 669 625 | 5 115 583 587 | -86038 | -0.002 |
| `m4` | 5 468 347 362 | 5 468 319 425 | -27937 | -0.001 |
| `m0` | 5 517 358 269 | 5 517 316 317 | -41952 | -0.001 |
| `smb1` | 5 996 423 306 | 5 996 354 167 | -69139 | -0.001 |
| `mm2` | 6 018 041 221 | 6 017 988 216 | -53005 | -0.001 |
| `z2` | 6 291 578 901 | 6 291 533 052 | -45849 | -0.001 |
| `readbuf` | 6 834 566 875 | 6 834 538 151 | -28724 | -0.000 |
| `duelito` | 7 380 498 812 | 7 380 424 832 | -73980 | -0.001 |
| `smb3` | 7 949 479 168 | 7 949 443 142 | -36026 | -0.000 |
| `mmc5` | 11 266 874 565 | 11 266 799 753 | -74812 | -0.001 |
| **total** | 79 346 690 804 | 79 346 172 190 | -518614 | -0.001 |

Worst ROM: `vblnmi` at **+0.001 %** (+43538). Set total -518614 (-0.001 %).


Every ROM at or below **+0.001 %**, which is the floor, and the set total **down** by half a million
instructions. **Inside the tolerance on all thirteen.**

### 9.3 The attribution

With both new features out of `default` — so the build is the six fixes' emulation changes and
nothing else, the deleted write latch included:

#### The six fixes with both new features out of `default`, against the base commit

| ROM | `mffix-0-base` | `mffix-4-off` | delta | % |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 253 228 | 3 308 301 202 | +47974 | +0.001 |
| `oamstr` | 3 318 455 828 | 3 318 478 391 | +22563 | +0.001 |
| `m1` | 4 881 143 644 | 4 881 089 351 | -54293 | -0.001 |
| `metroid` | 5 115 669 625 | 5 115 564 874 | -104751 | -0.002 |
| `m4` | 5 468 347 362 | 5 468 296 761 | -50601 | -0.001 |
| `m0` | 5 517 358 269 | 5 517 331 219 | -27050 | -0.000 |
| `smb1` | 5 996 423 306 | 5 996 340 171 | -83135 | -0.001 |
| `mm2` | 6 018 041 221 | 6 017 992 922 | -48299 | -0.001 |
| `z2` | 6 291 578 901 | 6 291 538 386 | -40515 | -0.001 |
| `readbuf` | 6 834 566 875 | 6 834 513 299 | -53576 | -0.001 |
| `duelito` | 7 380 498 812 | 7 380 411 536 | -87276 | -0.001 |
| `smb3` | 7 949 479 168 | 7 949 440 723 | -38445 | -0.000 |
| `mmc5` | 11 266 874 565 | 11 266 851 195 | -23370 | -0.000 |
| **total** | 79 346 690 804 | 79 346 150 030 | -540774 | -0.001 |

Worst ROM: `vblnmi` at **+0.001 %** (+47974). Set total -540774 (-0.001 %).


So the six fixes together cost **nothing**: worst ROM +0.001 %, inside the floor, and a set total
down 540 774. The latch's removal is the reason the figure is negative rather than zero.

Then one feature at a time, each measured as the only difference between two builds:

#### `board-195-prg-ram-5000` alone

| ROM | `mffix-4-off` | `mffix-3-no143ob` | delta | % |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 301 202 | 3 308 300 820 | -382 | -0.000 |
| `oamstr` | 3 318 478 391 | 3 318 494 544 | +16153 | +0.000 |
| `m1` | 4 881 089 351 | 4 881 135 785 | +46434 | +0.001 |
| `metroid` | 5 115 564 874 | 5 115 572 104 | +7230 | +0.000 |
| `m4` | 5 468 296 761 | 5 468 342 880 | +46119 | +0.001 |
| `m0` | 5 517 331 219 | 5 517 340 797 | +9578 | +0.000 |
| `smb1` | 5 996 340 171 | 5 996 404 420 | +64249 | +0.001 |
| `mm2` | 6 017 992 922 | 6 018 045 057 | +52135 | +0.001 |
| `z2` | 6 291 538 386 | 6 291 530 029 | -8357 | -0.000 |
| `readbuf` | 6 834 513 299 | 6 834 520 318 | +7019 | +0.000 |
| `duelito` | 7 380 411 536 | 7 380 427 342 | +15806 | +0.000 |
| `smb3` | 7 949 440 723 | 7 949 443 063 | +2340 | +0.000 |
| `mmc5` | 11 266 851 195 | 11 266 808 449 | -42746 | -0.000 |
| **total** | 79 346 150 030 | 79 346 365 608 | +215578 | +0.000 |

Worst ROM: `smb1` at **+0.001 %** (+64249). Set total +215578 (+0.000 %).


**`board-195-prg-ram-5000`: worst ROM +0.001 %, set total +215 578 (+0.000 %).** Inside the
tolerance by a factor of twenty, and inside the floor. The `IrqBoard::prg_ram_window` forward that
step 3 priced as the one risky part of CHANGE 2 is **free**, for the reason step 3 predicted: no ROM
of the PR-1 thirteen and no level-2 game is an `IrqBoard` board, so the new body in that dispatch is
never entered and all that could have moved is code layout. It did not. **Stays in `default`.**

#### `board-143-open-bus` alone

| ROM | `mffix-3-no143ob` | `mffix-1-ship` | delta | % |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 300 820 | 3 308 294 650 | -6170 | -0.000 |
| `oamstr` | 3 318 494 544 | 3 318 435 237 | -59307 | -0.002 |
| `m1` | 4 881 135 785 | 4 882 006 925 | +871140 | +0.018 |
| `metroid` | 5 115 572 104 | 5 115 643 699 | +71595 | +0.001 |
| `m4` | 5 468 342 880 | 5 469 213 694 | +870814 | +0.016 |
| `m0` | 5 517 340 797 | 5 518 206 016 | +865219 | +0.016 |
| `smb1` | 5 996 404 420 | 5 996 377 227 | -27193 | -0.000 |
| `mm2` | 6 018 045 057 | 6 017 971 882 | -73175 | -0.001 |
| `z2` | 6 291 530 029 | 6 291 507 614 | -22415 | -0.000 |
| `readbuf` | 6 834 520 318 | 6 834 538 422 | +18104 | +0.000 |
| `duelito` | 7 380 427 342 | 7 380 423 702 | -3640 | -0.000 |
| `smb3` | 7 949 443 063 | 7 949 506 585 | +63522 | +0.001 |
| `mmc5` | 11 266 808 449 | 11 270 903 088 | +4094639 | +0.036 |
| **total** | 79 346 365 608 | 79 353 028 741 | +6663133 | +0.008 |

Worst ROM: `mmc5` at **+0.036 %** (+4094639). Set total +6663133 (+0.008 %).


**`board-143-open-bus`: `mmc5` +0.036 %, with `m1` +0.018 %, `m4` +0.016 % and `m0` +0.016 % behind
it.** Over the +0.02 % tolerance on `mmc5exram`, and within a rounding of it on three more.

The cause is the one step 6 named as the risk: the `samples_console` test in
`NesBus::read_wide_window`, which runs on every CPU read of `$4020-$7FFF`. `mmc5exram` reads its
`ExRAM` through `$5C00` a million times a run, and `m0`/`m1`/`m4` read PRG-RAM at `$6000-$7FFF`.
It cannot be moved anywhere cheaper — that function is the only one that both runs on the reads able
to reach the window and has the bus in hand — and it cannot be narrowed, because the cost is the
**test**, not the store.

**Decision: `board-143-open-bus` is parked.** Moved out of `default` into `parked-boards`, never
dropped, which is the owner's rule; `--features parked-boards` is the one flag that turns it on. The
cost in accuracy is two bits of one read on one board: with the feature off they read zero, which is
what MF-UNL-3 built, the six bits the board does drive are identical either way, and **no mapper-143
game can tell the two builds apart under the one use the page describes** — which is what Q-145's
own note said before this lot and still says. It is the third parked feature of this build, after
`board-vs-system` and `board-292`.

### 9.4 `board-292`: still over, and the report's expectation is not met

#### `board-292` on, against the base commit

| ROM | `mffix-0-base` | `mffix-2-292` | delta | % |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 253 228 | 3 308 280 245 | +27017 | +0.001 |
| `oamstr` | 3 318 455 828 | 3 318 490 280 | +34452 | +0.001 |
| `m1` | 4 881 143 644 | 4 882 429 204 | +1285560 | +0.026 |
| `metroid` | 5 115 669 625 | 5 115 676 046 | +6421 | +0.000 |
| `m4` | 5 468 347 362 | 5 469 633 132 | +1285770 | +0.024 |
| `m0` | 5 517 358 269 | 5 518 657 773 | +1299504 | +0.024 |
| `smb1` | 5 996 423 306 | 5 996 375 984 | -47322 | -0.001 |
| `mm2` | 6 018 041 221 | 6 017 983 764 | -57457 | -0.001 |
| `z2` | 6 291 578 901 | 6 291 513 163 | -65738 | -0.001 |
| `readbuf` | 6 834 566 875 | 6 834 510 626 | -56249 | -0.001 |
| `duelito` | 7 380 498 812 | 7 380 417 294 | -81518 | -0.001 |
| `smb3` | 7 949 479 168 | 7 949 543 615 | +64447 | +0.001 |
| `mmc5` | 11 266 874 565 | 11 393 782 001 | +126907436 | +1.126 |
| **total** | 79 346 690 804 | 79 477 293 127 | +130602323 | +0.165 |

Worst ROM: `mmc5` at **+1.126 %** (+126907436). Set total +130602323 (+0.165 %).

#### `board-292` on, against the lot as it ships

| ROM | `mffix-5-ship2` | `mffix-2-292` | delta | % |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 296 766 | 3 308 280 245 | -16521 | -0.000 |
| `oamstr` | 3 318 464 325 | 3 318 490 280 | +25955 | +0.001 |
| `m1` | 4 881 110 457 | 4 882 429 204 | +1318747 | +0.027 |
| `metroid` | 5 115 583 587 | 5 115 676 046 | +92459 | +0.002 |
| `m4` | 5 468 319 425 | 5 469 633 132 | +1313707 | +0.024 |
| `m0` | 5 517 316 317 | 5 518 657 773 | +1341456 | +0.024 |
| `smb1` | 5 996 354 167 | 5 996 375 984 | +21817 | +0.000 |
| `mm2` | 6 017 988 216 | 6 017 983 764 | -4452 | -0.000 |
| `z2` | 6 291 533 052 | 6 291 513 163 | -19889 | -0.000 |
| `readbuf` | 6 834 538 151 | 6 834 510 626 | -27525 | -0.000 |
| `duelito` | 7 380 424 832 | 7 380 417 294 | -7538 | -0.000 |
| `smb3` | 7 949 443 142 | 7 949 543 615 | +100473 | +0.001 |
| `mmc5` | 11 266 799 753 | 11 393 782 001 | +126982248 | +1.127 |
| **total** | 79 346 172 190 | 79 477 293 127 | +131120937 | +0.165 |

Worst ROM: `mmc5` at **+1.127 %** (+126982248). Set total +131120937 (+0.165 %).


**`board-292` does not clear the tolerance: `mmc5exram` +1.126 % against the base commit.** On level
2 it is inside — worst game `metroid` +0.001 %, set total below the base — and on twelve of the
thirteen PR-1 ROMs it is inside too. The one ROM that fails is the one that failed before, by the
same margin.

**`docs/review/Q-XCHECK.md` §CHANGE 8 expected this change to unpark the board and it does not.** The
entry reasons from the latch: "the ticket records that the latch store alone was +862 997 of
+874 540 instructions on `oamstr`, so removing it should recover almost all of that feature's cost."
That is true of `oamstr`, and the measurement confirms it — **`oamstr` goes from +0.026 % to
+0.002 %** against the shipped build, and `m0`, `m1` and `m4` roughly halve. But `oamstr` was never
the binding ROM. MF-UNL-12 step 6.2 named **two** causes and `mmc5`'s +1.098 % was the *other* one:
"`Board::cpu_read_override` gains a body. `IrqBoard` had no forward for it, so it shared that
dispatch's **empty** arm — and that dispatch carries `#[inline(always)]`, pinned by lot PR-1 step 3,
and folds into `Cartridge::try_cpu_read`, which the MMC5 reaches on **every CPU read**."

CHANGE 8 does not touch that cause and could not: mapper 292's registers are filled *by a read*, so
the board needs a `&mut self` read hook whatever the datum's source is. The entry's own sentence
"with the latch gone the board is a read hook and two registers" is exactly right — and the read hook
is the expensive half.

**Decision: `board-292` stays in `parked-boards`.** No change to the shipped configuration; the lot
does not move it and, per the owner's instruction, would not have moved it in either direction
without R0. What the lot does buy is that the latch's cost is gone from **every game in the
library**, parked board or not, which is why the shipped build is faster than its base.

### 9.5 Level 2, the five games

#### Level 2, the lot as it ships against the base commit

| game | `mffix-0-base` | `mffix-5-ship2` | delta | % |
| --- | ---: | ---: | ---: | ---: |
| `smb1` | 13 590 240 999 | 13 589 961 088 | -279911 | -0.002 |
| `metroid` | 13 736 148 307 | 13 736 060 771 | -87536 | -0.001 |
| `zelda2` | 15 362 757 027 | 15 359 693 399 | -3063628 | -0.020 |
| `mm2` | 18 977 422 970 | 18 977 335 566 | -87404 | -0.000 |
| `smb3` | 24 194 127 204 | 24 192 305 624 | -1821580 | -0.008 |
| **set** | 85 860 696 507 | 85 855 356 448 | -5340059 | -0.006 |

Worst game: `mm2` at **-0.000 %** (-87404).

#### Level 2, board-292 on against the base commit

| game | `mffix-0-base` | `mffix-2-292` | delta | % |
| --- | ---: | ---: | ---: | ---: |
| `smb1` | 13 590 240 999 | 13 590 216 385 | -24614 | -0.000 |
| `metroid` | 13 736 148 307 | 13 736 231 829 | +83522 | +0.001 |
| `zelda2` | 15 362 757 027 | 15 361 021 404 | -1735623 | -0.011 |
| `mm2` | 18 977 422 970 | 18 977 279 114 | -143856 | -0.001 |
| `smb3` | 24 194 127 204 | 24 190 545 454 | -3581750 | -0.015 |
| **set** | 85 860 696 507 | 85 855 294 186 | -5402321 | -0.006 |

Worst game: `metroid` at **+0.001 %** (+83522).

#### Level 2, board-292 on against the lot as it ships

| game | `mffix-5-ship2` | `mffix-2-292` | delta | % |
| --- | ---: | ---: | ---: | ---: |
| `smb1` | 13 589 961 088 | 13 590 216 385 | +255297 | +0.002 |
| `metroid` | 13 736 060 771 | 13 736 231 829 | +171058 | +0.001 |
| `zelda2` | 15 359 693 399 | 15 361 021 404 | +1328005 | +0.009 |
| `mm2` | 18 977 335 566 | 18 977 279 114 | -56452 | -0.000 |
| `smb3` | 24 192 305 624 | 24 190 545 454 | -1760170 | -0.007 |
| **set** | 85 855 356 448 | 85 855 294 186 | -62262 | -0.000 |

Worst game: `zelda2` at **+0.009 %** (+1328005).


**Every game at or below the base in the shipped build** — worst `mm2` −0.000 %, set total
−5 340 059 — and `board-292` inside the tolerance here too, worst `metroid` +0.001 %. `zelda2`'s
−0.020 % and `smb3`'s −0.008 % in the shipped column are the write latch coming out of
`NesBus::write_internal`, which is the one figure of this lot that is a gain rather than a cost.

### 9.6 golden-core over the 98 originals

```
zax-harness-<label> golden-core /mnt/g/3dsen/test-originals gc98-<label>.jsonl 1200
```

| build | md5 of the 98-ROM digest |
| --- | --- |
| `mffix-0-base` | `f79e7df842c3ee43e7d9b67861a927b5` |
| `mffix-5-ship2` (shipped) | `f79e7df842c3ee43e7d9b67861a927b5` |
| `mffix-2-292` (`board-292` on) | `f79e7df842c3ee43e7d9b67861a927b5` |

**Byte-identical**, and identical to the digest eleven lots have recorded. The six fixes change the
emulation of mappers 12, 123, 143, 195, 262 and 292 and none of the 98 originals is one of those
numbers, which is what the lot's scope said and this is the check of it. That it holds **with
`board-292` on as well** is what makes the park a figure and not a behaviour change.

### 9.7 The decisions, in one place

| feature | worst PR-1 ROM | worst level-2 game | decision |
| --- | --- | --- | --- |
| the six fixes' emulation changes | `vblnmi` +0.001 % (the floor) | — | in, unconditionally |
| `board-195-prg-ram-5000` | `smb1` +0.001 % | — | **`default`** |
| `board-143-open-bus` | `mmc5` **+0.036 %** | — | **parked** |
| `board-292` | `mmc5` **+1.126 %** | `metroid` +0.001 % | **stays parked** — R0 decides the move |

`docs/review/BOARD-FEATURES.md` carries the two new rows with these figures.
