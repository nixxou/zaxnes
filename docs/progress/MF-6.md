# Lot MF-6 — the inventory of the boards still missing, and the plan of the sub-lots

Baseline `fcfa5cb3` (the end of MF-5b, the VRC7). **This lot writes no emulator code**: not one
`.rs` file changes. It produces `docs/review/MF-6-inventory.md`, the document MF-6a, MF-6b and
MF-6c will each be briefed from.

## Steps

0. The plan, the sources, and the ROM situation settled first.
1. The inventory: every iNES number Mesen 2 or NESdev documents and our registry does not handle.
2. The hooks each board needs, the hot paths they would touch, and the variants of boards we
   already have.
3. The plan of the sub-lots MF-6a / MF-6b / MF-6c, and what is deliberately left out.
4. The check: `cargo fmt --check` and `cargo test --release -p zax-nes -p zax-harness`, unchanged.

## The sources consulted

| source | what it settles |
| --- | --- |
| `rust/crates/zax-nes/src/cartridges/registry.rs` | `SUPPORTED_MAPPERS`, the 41 numbers this build runs |
| `third_party/mesen2/Core/NES/MapperFactory.cpp:279-648` | the numbers Mesen 2 knows, and which class each builds |
| `spec/SPEC-02-coeur-nes.md` §3.11 (REQ-CORE-100/101/102), §3.7 (REQ-CORE-075) | the required table, and that adding beyond it is permitted |
| `spec/changes/AMD-061-exactitude-des-cartes.md` §1 | public hardware documentation prevails over the reference program |
| `docs/review/MR-1-mappers.md` | the board inventory as reviewed against Mesen, and the variant questions (MR-4-3, MR-34-1) |
| `docs/progress/MF-5a.md`, `MF-5b.md`, `MF-3b.md` | how a board was added at zero cost to the others |
| `assets-free/test-roms/` | every `.nes`/`.unf` header decoded, by mapper number |
| NESdev wiki: "Mapper", "List of mappers", and each board's own page | board names, register layouts, licensed/unlicensed |

## The ROM situation, settled first because it decides every sub-lot's test plan

**No test ROM in `assets-free/test-roms` exercises any board this build does not already have.**
Re-established here independently of MF-5a step 0: every `.nes`, `.unf` and `.unif` under that
tree was opened and its header decoded (iNES signature, byte 6 bits 4-7, byte 7 bits 4-7, and
byte 8 bits 0-3 when byte 7 bits 2-3 say NES 2.0) —

    333 images with an iNES signature, mapper numbers
    0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 22, 28, 34, 66, 69, 78, 118, 180

and every one of those eighteen numbers is in `SUPPORTED_MAPPERS`. Four files carry no iNES
header: `nes-test-roms/other/LINUSMUS.NES` (signature `NES\x1b`) and the three UNIF images
`scanline/scanline.unif`, `scanline-a1/scanline.unif`, `stomper/smwstomp.unif`, which name a board
rather than a number and which this build does not load at all.

So **the "test ROM present?" column of the inventory is `no` for every row**, and no sub-lot can
be driven by a ROM. Each one has to rest on unit tests written against the NESdev page and
cross-checked against `third_party/mesen2`, with commercial titles named for R0 to run — which is
exactly the position MF-5a and MF-5b were in, and both shipped from it.

## Step 1 — the inventory

`docs/review/MF-6-inventory.md` §1 to §3. **245 of the numbers Mesen 2 builds a board for are
numbers this registry refuses**, derived mechanically from `MapperFactory.cpp:279-648` rather than
transcribed, and split four ways because they are not 245 equal items:

| § | what | count |
| --- | --- | ---: |
| 3.1 | boards with licensed titles and no counterpart here | 38 numbers |
| 3.2 | numbers that are a **parameter** of a board we already have | 27 + the 48 MMC3 clones |
| 3.3 | unlicensed boards of a single publisher (Sachen, Kaiser, Waixing, TXC, NTDEC, JY, Whirlwind, FFE) | 60 |
| 3.4 | pirate multicarts, BMC boards and MMC3 clones | 134 |
| 3.5 | numbers Mesen knows and refuses, the gaps, the UNIF-only boards, the FDS | 23 + the gaps |

Every title count is the figure its own NESdev board page gives — its "N in NesCartDB" line or the
length of the game list it prints, the two distinguished in the table. NesCartDB's own host was
unreachable from this session, so the counts are the wiki's transcriptions of it and the document
says so. Where a page gives neither, the cell says that instead of guessing.

**Two findings worth more than the tables:**

* **REQ-CORE-100's required table is covered in full.** Nothing in this document is required by
  the spec; the build already adds twelve numbers beyond the table, which REQ-CORE-102 permits in
  as many words. So the sub-lots are ordered by what each buys, not by compliance.
* **The profile list has "Super Spike V'ball", and mapper 47 is the Nintendo multicart of exactly
  that title.** The US standalone is an ordinary mapper 4 and loads today; a user whose dump is
  `Super Spike V'Ball + Nintendo World Cup` gets REQ-CORE-011's refusal and never reaches the
  profile. `profiles.json` records no mapper number, so which board each of the 101 dumps sits on
  is R0's to check — but this one named a missing board by itself, and it is why 47 is in MF-6a.

## Step 2 — the hooks, the hot paths, and the variants

`MF-6-inventory.md` §4, one section per hook, each with the measurement that priced it rather than
a judgement. The summary table §4.10 is the one a sub-lot brief should carry verbatim.

**Eight of the ten hooks cost a board that does not use them exactly zero**, and each is zero
because a lot made it so: `clocks_on_cpu_cycle` (MF-5a step 2, which took 11 instructions per CPU
cycle to 0 for 26 of the 38 numbers), `watches_ppu_reads` and `watches_ppu_bus_without_read`
(MF-3b step 2, over 57 600 fetches a frame), `nametable_pages_vary`, `rom_windows`,
`bus_conflicts`, and the `mapper_number` field MF-5a step 1 turned from a per-sprite dispatch into
an `i32`.

**Two are not latched, and they are the whole performance story of MF-6:**

* **`NesBus::irq_asserted`.** MF-5b's thirty-second arm pushed the chain past the inliner's budget
  and LLVM outlined it: **+169 739 749 instructions, ~19 per CPU cycle, on twelve ROMs of thirteen
  whose board cannot assert an IRQ at all**, +0.542 % on every one. Four `#[inline(always)]` fixed
  it; the load-time `irq_capable` latch was **built and refuted** (+1.3 to +1.8 % on three of R0's
  ROMs, two of them MMC3s), so MF-6b must not re-litigate it. MF-6b adds up to five more arms and
  the yardstick per arm is MF-5a's and MF-5b's identical +11 in `branch` / +47 in `resolve`.
* **`Mapper::read_nametable`, capped at two distinct arm bodies by measurement.** Q-123: a third
  body takes `read_nametable_inner` from 132 to 109-83 *for every game*, and MF-5a's five shapes
  prove "the trigger is the third distinct body, not its size". **Mapper 68 (Sunsoft-4) is that
  third body** — its `$E000` bit 4 is the same CHR-ROM-nametable feature the VRC6 has and we
  decode and ignore. §4.5 says what MF-6c must do instead.

§3.2 is the variants: **27 numbers plus the 48 MMC3 clones are a parameter of a board we already
have, not a new board** — and `board.rs`'s own note is why that matters (32 variants, `Mmc5`
pinned at index 5 by the niche-filling layout, appended never inserted). The `McAcc` line
(MR-4-3, shipped) is the precedent for a `const` parameter that costs the parent board nothing;
mapper 26 is the precedent for a runtime `bool` when the test is off the per-cycle path.

Three decisions were taken here rather than deferred, each with the text that settles it:

* **mappers 72, 86, 92 get their boards and not their µPD7756C audio.** REQ-CORE-075 lists six
  optional chips and this is not one of them, and the ADPCM samples live in a mask ROM inside the
  ASIC — "sample files for the speech synthesis chip emulation", which no source this role may use
  contains. A test pins the silence. **No Q ticket.**
* **mappers 80, 82, 207 and 552's mapper-internal WRAM is the Namco 163's shape**, not a new
  mechanism: `registry.rs:90-97` already puts a chip's internal bytes "at the end of PRG-RAM so
  that REQ-CORE-095's single battery slice carries it".
* **the 24C01/24C02 EEPROM of 157 and 159 is genuinely new**, and REQ-CORE-100's mapper-16 row
  already prices it: "EEPROM : P3".

## Step 3 — the plan of the sub-lots, the exclusions, and Q-130

`MF-6-inventory.md` §5 and §6. **45 of the 245 numbers go into three sub-lots; 200 are left out,
and each of the four reasons is written down.**

| sub-lot | boards | numbers | new `Board` variants | licensed titles | performance risk |
| --- | --- | ---: | ---: | ---: | --- |
| **MF-6a** | the licensed boards with **no hook at all** | 20 | 8 | ~75 | low: eight more "no" arms on `irq_asserted` |
| **MF-6b** | the boards with a CPU-cycle or scanline IRQ | 6 | 4 | ~15 | **the highest in the document**, and it is one symbol |
| **MF-6c** | the rest worth doing | 14 | 7 | ~25 | two steps, each with a stop-the-lot condition |

Each sub-lot's section carries the boards, the steps, the tests per board (one per register claim,
in MF-5a's and MF-5b's naming), the test ROMs (**none exist**; the existing suites are run as
non-regression only), the commercial titles R0 should run, and the measurement to make. Four
things are common to all three and are stated once: the bar (MF-5b's), the three instruments, the
four `SUPPORTED_MAPPERS` sweeps that cover a new number without a line being written, and the
append-behind-versioning rule for save states.

**The ordering is not by title count alone.** MF-6a is first because its twenty numbers touch no
hot path and six of them are a flag on a board that already exists (94 on `UxRom`, 185 on `CnRom`,
155 on `Mmc1`, 119/47/37 on `Mmc3`). MF-6b is second — not last — because its boards are
*licensed* and its risk is known and bounded: five new arms on the one dispatch MF-5b watched
outline at the thirty-second, with `+11`/`+47` per arm as the yardstick and the refuted
`irq_capable` latch explicitly out of bounds. MF-6c is third because each of its boards has a hook
that is new (the EEPROM), awkward (mapper-internal WRAM) or capped by a measurement (mapper 68 and
the Q-123 ceiling).

Three steps carry an explicit **stop-the-lot condition**, because each touches a board on R0's
thirteen ROMs: MF-6a step 5 (`Mmc3`: `m4`, `smb3`), MF-6b step 3 (`Mmc1`: five of the thirteen),
MF-6c step 5 (mapper 77 is the only board in MF-6 that can reach `read_pattern_planes`, which has
been `IDENTICAL raw` since MF-2 built its shield).

**What is left out: 200 numbers, four reasons.** 182 pirate multicarts, BMC boards and MMC3 clones
— and the argument for the documented ones among them is the strongest in the document and is
quoted as such: *this is not an emulator, it is a 3D emulator*, its value is the 101 hand-built
profiles, every one a licensed title, and a game on an unlicensed board has no profile, so running
it adds a flat 2D game to a program whose point is the third dimension. Then the boards no title
uses (Mesen's own 23 refusals, the UNIF-only boards this build cannot reach, the Study Box);
**mapper 99 / VS System, set aside by the owner** — recorded with a price on it (four pieces,
three of them in SPEC-08 and SPEC-06 territory) so the decision can be revisited, and mapper 151
goes out with it; and **the FDS**, a disk drive and not a mapper.

**Q-130 filed** (`spec/changes/Q-130.md`), two questions, the next free number, each with its
provisional choice marked as provisional. The other 243 numbers are decided in the document and
need no arbitration.

1. **Mapper 116** — the only number the spec names for itself (REQ-CORE-101) and the only one where
   two normative texts give different jobs: "comme la référence" is a VRC2 of half a day,
   AMD-061 §1's "comme le matériel" is three ASICs with two IRQ counters and a supervisor register,
   for six unlicensed titles with no profile. A third reading is named so the answer can take it:
   AMD-061 §1 forbids copying the reference's *errors*, not leaving an optional board out.
   Provisional: **not implemented**, which REQ-CORE-101 allows and which
   `req_core_102_any_other_mapper_is_refused` already pins by name.
2. **REQ-CORE-102's acceptance criterion names mapper 200**, which is a real board Mesen builds —
   so the criterion tests "refuse an unsupported number" with a number that could stop being
   unsupported by the exercise of the permission the same requirement grants one sentence earlier.
   The test that embodies it sweeps 6, 116, 177 and 200 and **all four are real boards**, so it
   holds no safe number. Provisional: 200 stays out, the criterion holds — but it wants a number
   that is nobody's board.

## Step 4 — the check

Nothing in the code moved, and the check says so rather than assuming it.

| command | result |
| --- | --- |
| `cargo fmt --check` | **clean**, exit 0 |
| `cargo test --release -p zax-nes -p zax-harness` | **816 passed, 0 failed, 8 ignored** |

**816 / 0 / 8 is MF-5b's own closing figure, test for test** (`MF-5b.md` step 5: "816 passed,
0 failed, 8 ignored"), which is the check this lot actually needed: a documentation lot that moved
a test count would mean it had touched something it said it had not.

No clippy run: no crate's sources changed, so there is nothing to lint that MF-5b did not already
lint clean. `git status` at the close shows only the two documents of this lot and the untracked
`rust/target-mf/`, which the lot's rules forbid staging.

`spec/changes/Q-130.md` is written outside this repository (the spec tree is not a git working
copy), so it is not in any commit here; the ticket's content is summarised in step 3 above and in
`MF-6-inventory.md` §6.

## What the lot delivered, against the brief

- [x] **The inventory** — `docs/review/MF-6-inventory.md`, 910 lines. Every iNES number Mesen 2 or
      NESdev documents and this registry refuses: **245**, derived mechanically from
      `MapperFactory.cpp` rather than transcribed, with board name, chip family, licensed status,
      a title estimate cited to its own NESdev page, and the test-ROM answer (§2: **no**, for every
      row, established by decoding all 333 headers).
- [x] **The hooks per board**, and every one that would touch a hot path flagged with the
      measurement that priced it (§4). The two that are **not** latched —
      `NesBus::irq_asserted` and `Mapper::read_nametable` — are named as the whole performance
      story of MF-6, with MF-5a's and MF-5b's figures and the refuted `irq_capable` latch.
- [x] **The variants** (§3.2): 27 numbers plus the 48 MMC3 clones are a parameter of a board we
      already have, not a new board, with the `McAcc` `const`-parameter precedent and `board.rs`'s
      append-never-insert rule.
- [x] **The plan of the three sub-lots** (§5), each with its boards, steps, per-board tests, test
      ROMs (none), R0's commercial titles, and the measurement to make — plus three explicit
      stop-the-lot conditions on the steps that touch `Mmc3`, `Mmc1` and `read_pattern_planes`.
- [x] **What is left out and why** (§6): 200 numbers, four reasons, with mapper 99 priced rather
      than merely deferred.
- [x] **Q-130**, two questions, provisional choices marked.
- [x] **The check** (this step), and **no `.rs` file changed**: `git diff --stat fcfa5cb3 HEAD`
      touches `docs/progress/MF-6.md` and `docs/review/MF-6-inventory.md` and nothing else.

[PROGRESS] MF-6 step 0/4 done: the plan, the sources, and the ROM situation settled first
[PROGRESS] MF-6 step 1/4 done: the inventory, 245 numbers in four tables
[PROGRESS] MF-6 step 2/4 done: the hooks, the two unlatched hot paths, and the variants
[PROGRESS] MF-6 step 3/4 done: the three sub-lots, the 200 numbers left out, and Q-130
