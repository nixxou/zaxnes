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

- [x] **0/8** — the plan: this file.
- [x] **1/8** — `Mmc3Outer`: the six outer registers, banking mode `$00` (the oversize MMC3),
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

---

## Step 1 — the six outer registers, and the board that powers on as a 512 KiB MMC3

`board-268`, in `default`. One new module, `cartridges/mappers/mmc3_outer.rs`, one new **boxed**
[`IrqBoard`] variant, three forwards widened (`prg_ram_enabled`, `prg_ram_writable`,
`intercept_cpu_write`), two `const` accessors on `Mmc3Board`, and the number in **both** PPU watch
lists — the board holds an `Mmc3`, counter included.

### The register file, from nesdev's *NES 2.0 Mapper 268*

Six registers under **mask `$F007`**, at `$6000-$6FFF` on a COOLBOY and `$5000-$5FFF` on a
MINDKIDS — bit 0 of the submapper is which. The mask reads A12-A15 and A0-A2 and nothing between,
so the six repeat through the whole 4 KiB page and `$xxx6`/`$xxx7` are inside the window without
being registers.

| register | diagram | what step 1 implements |
| --- | --- | --- |
| `$xxx0` | `ABCC DEEE` | `EEE` = PRG A19..A17, `D` = alternate CHR A17, `CC` = PRG A24/A23 **and** CHR A19/A18, `B` = the PRG A17 mask, `A` = the CHR A17 mask |
| `$xxx1` | `GHIJ KKLS` | `S` = SC0, `L` = the GNROM bank size (step 2), `JKK` = PRG A20, A22, A21, `GHI` = the PRG A18/A19/A20 masks |
| `$xxx2` | `STTT MMMM` | latched, with its own `S`-and-`T` lock; what it drives is GNROM mode (step 2) |
| `$xxx3` | `N.ZP QQRS` | `N` = the lockout, bits 6 and 4 = the banking mode, `Z` = the $5000-$5FFF RAM (step 2), `QQR` = the GNROM PRG offset (step 2), `S` = SC1 |
| `$xxx4` | `UUUU UUUV` | latched; the mixed CHR-ROM/CHR-RAM select is step 2 |
| `$xxx5` | `xxxW XXYY` | latched; `W`'s mega-UNROM mode is a recorded gap |

### The one thing in this board that reads backwards, and it is the page's own

> `B` — PRG mask (PRG A17 from 0: MMC3; 1: offset)
> `G` — PRG mask (PRG A18 from 0: MMC3; 1: offset)
> `H` — PRG mask (PRG A19 from **0: offset**; 1: MMC3)
> `I` — PRG mask (PRG A20 from **0: offset**; 1: MMC3)

Two of the four mask bits are the negation of the other two, so **with every register clear the
MMC3 drives A13-A18 and an offset of zero drives A19 and A20**: the board powers on as a *512 KiB*
MMC3 and a plain MMC3 write that selects a bank above it is truncated. That is not a quirk, it is
the mechanism — it is what makes a COOLBOY menu work, it is why the page says "a great number of
Chinese cartridges use the ASIC as a simple MMC3 clone and thus are usually found set to INES
Mapper 004", and it is why the twelve titles have to open the lines before they bank.
`req_core_102_mapper_268_powers_on_as_a_512_kib_mmc3` is that paragraph as a test, including the
**two routes to the same bank**: `$45` through the chip with the mask open, and `$45` through the
offset with it closed.

### Why the board recomputes all twelve windows instead of laying an overlay over the MMC3's

`Mmc3Scramble` and `Mmc3Txc189` call `Mmc3Board::refresh_banks` and then overwrite windows; this
board cannot, and the reason is that its outer register replaces **address lines** of both bank
numbers rather than adding to them. `BankWindows` holds a bank *wrapped modulo the chip's own
banks*, and `(bank mod N) & mask | bits` is only the hardware's answer when the recovery is exact.
So `Mmc3Outer::refresh` composes the four PRG and eight CHR bank numbers itself, from
`Mmc3Board::select` and `Mmc3Board::bank_register` — two new `const` accessors behind `board-268`
— and writes all twelve. **No change to `BankWindows`**, which is the shared type this lot leaves
alone, and no change to any existing board's composition.

One consequence is worth writing down: the MMC3's two fixed windows are `$FE` and `$FF` **as
eight-bit numbers**, not "the last two banks of the image". On this board they have to be — the
chip drives A13-A20 and the outer register A21 upwards, so the fixed banks are the last two of the
*selected* window — and it is the page's own vocabulary, banking mode `$40` saying the two become
"8 KiB bank 0, instead of `0xFE` and `0xFF`".

### The lockout, which §7.2 asks be a no-op and the page refutes

§7.2 step 3 is "The lockout: documented, and implemented as a **no-op** with the reason in a
comment, as the Camerica row of `BOARD-FEATURES.md` already does for 232". MF-UNL-2's rule applies
and the page wins (**Q-156**): the lockout is implemented, because the page's own *WRAM usage*
section is what it is *for*.

On a COOLBOY the register window **is** the lower half of the WRAM window, so a game that wants
both writes the page's sequence — registers up with the MMC3's WRAM disabled, `$80` to `$6003`,
then WRAM on — and the page states the consequence where the bit does not work: "it is impossible
to have [a] GNROM game use WRAM in the `$6000-$6FFF` (submapper 0) setting, as any WRAM write
would inadvertently change the outer bank registers". With a no-op lockout the first byte a save
routine wrote to `$6003` would re-bank the cartridge under itself.
`q_156_mapper_268_locks_its_registers_out_and_keeps_the_wram_behind_them` runs that sequence and
then checks both halves: no register moves, and every byte written is readable where it was
written.

The write is consumed by `intercept_cpu_write` and the WRAM written **there**, which is the MMC6's
shape and this board needs it for the same reason: `Cartridge::cpu_write`'s own PRG-RAM branch
returns without re-reading the ROM windows a register write has just moved.

### Submappers 2 to 11 are refused, and three sweeps had to learn to let them

`Mmc3Outer::new` answers `None` outside submappers 0 and 1, which `Cartridge::create` turns into
REQ-CORE-011's `UnsupportedMapper`. **Q-157** records the reading of §7.2's "leave the rest
refusing" that gets there.

The three in-crate sweeps (`mf_3b`, `mf_5a`, `mf_4c`) build each number at submappers 0 **and 3**,
for the MC-ACC's sake, so they would have panicked on this number. They now go through
`latched_cartridge`, which skips a pair the registry refuses **and asserts that the pair was
mapper 268's and the cause `UnsupportedMapper`** — so the sweeps still fail on any other refusal,
and `mf_3b`'s expected vector gains `268` **once** rather than twice.

### What it measures to the suites

**1034 passed, 0 failed, 9 ignored** for `-p zax-nes -p zax-harness` (the base commit left it at
1028/0/9): six tests. `cargo fmt --all -- --check` clean and
`cargo clippy -p zax-nes --all-targets -- -D warnings` clean.
