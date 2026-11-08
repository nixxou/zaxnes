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
- [x] **2/8** — the other three banking modes — `$10` (GNROM), `$40` and `$50` (the two "weird"
  ones) — the mixed CHR-ROM/CHR-RAM window of `$xxx4`, and the PRG-RAM window `$xxx3` bit 5 opens
  at $5000-$5FFF.
- [x] **3/8** — the size ceiling: a 64 MiB image loads, a bigger declaration is refused, the
  SC0/SC1 half-select, and the `board_memory_sizes` row.
- [x] **4/8** — the question tickets, the four sweeps, the save-state round-trip and the feature
  configurations.
- [x] **5/8** — the callgrind campaign against the base commit, and the feature decision.
- [x] **6/8** — golden-core against the base commit, and the full `--include-ignored` run with
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

---

## Step 2 — the three other banking modes, the mixed CHR chip, and the window one clause opens

Still `board-268`, no new variant and no new forward but one: `try_read_prg`, which `IrqBoard` has
carried since MF-UNL-2 for mappers 121 and 187 and which the default build therefore already has.

### The banking mode is **two** bits of `$xxx3` and the page names the modes by their value

The legend of `$xxx3` draws one arrow across bits 6 and 4 and labels it "Banking mode", and the
page's four sections are headed by the value of those two bits — `$00`, `$10`, `$40`, `$50`. So
bit 4 is "GNROM mode" and bit 6 is "weird MMC3 mode", and the four combinations are the four
sections. Each gets one test, which is the census's "each documented banking mode, one case each".

| mode | what it is | the test |
| ---: | --- | --- |
| `$00` | the oversize MMC3 | step 1's two tests |
| `$10` | "normal" GNROM: PRG A16/A15 from `QQ`, A14 from `R` or CPU A14 by `L`, CHR A16-A13 from `MMMM` with `TTT` **ANDed** in and A12-A10 from the PPU | `req_core_102_mapper_268_banks_a_gnrom_page_in_banking_mode_10` |
| `$40` | "weird" MMC3: the two fixed PRG windows become **bank 0** instead of `$FE`/`$FF`, the odd window of each 2 KiB CHR pair becomes bank 0, and the LSB of R0 and R1 starts working | `req_core_102_mapper_268_fixes_the_top_windows_to_bank_zero_in_banking_mode_40` |
| `$50` | both | `req_core_102_mapper_268_combines_both_rules_in_banking_mode_50` |

Three readings this step had to take, each recorded in the code:

* **"Supposedly this effect is completely ignored when MMC3 PRG ROM bank mode is inverted"** is
  implemented as written, so the weird rule is applied only when `$8000` bit 6 is clear — and in
  mode `$50` that makes `$10` and `$50` the *same* layout on an inverted chip, which is the
  cleanest statement of "ignored" a test can make.
* **the weird CHR rule does not apply in GNROM mode**, which is how "(These descriptions should not
  be contradictory.)" is satisfied: in GNROM mode the MMC3's CHR registers reach nothing at all, so
  a rule about their LSB has nothing to act on.
* **bank 0 is a bank of the chip's own numbering**, so the weird rule is applied *before* the mask
  and offset and the outer register still carries those two windows.

`TTT` is taken literally as the page's "**ANDed** with CHR A16, A15, A14 in `MMMM`" — a register
that can only clear a line of its own offset, which is what a multicart menu needs to keep a game
inside its own CHR window — and `MMMM`'s own bit 0 (CHR A13) is never masked, the mask covering
three lines and the field being four wide. `$xxx2` bit 7's own lock is tested from the same place,
because GNROM mode is the only mode in which that register does anything at all.

### `$xxx4`, and the sentence that pins its comparison

> This register is only meaningful on cartridges that mount both CHR-ROM and CHR-RAM. By writing
> the appropriate value to D1-D7, the functionality of INES Mapper 074 and INES Mapper 194 is
> replicated.

Mapper 74's CHR-RAM is banks `$08-$09` and mapper 194's `$00-$01`: each a 2 KiB **pair** whose two
banks differ in D0 alone. So bits 7-1 name a pair and the comparison is `value >> 1`, where
`value` is the MMC3 bank **register**'s byte and not the window's bank — which is why
`chr_window_register` returns both. `$xxx4 = $09` *is* mapper 74 on this chipset and
`$xxx4 = $01` is mapper 194, and the test asserts exactly that.

The chip is reached through `ppu_peek` and `ppu_write`, both of which test a derived flag byte
first — `chr_ram_windows`, zero on every cartridge that never writes `$xxx4` — and the CHR-RAM
body is `#[inline(never)]`, which is `Mmc3Board::chr_ram_read`'s own measured shape (MF-6a2 step 3
priced that body inlined at `Ppu2C02::background_fetch` 271 → 583 instructions). `chr_peek_view`
answers `None` in mixed mode, the CHR-RAM family's own answer for the same reason: a `ChrView`
cannot say "this window is the other chip".

Three PCB rows of the page are **not** implemented and the code says so: the J-852C's PRG chip
select (submappers 6/7), the SMD72A's CHR-RAM write protect (8/9) and the SMD172C-L1's
single-screen mirroring (10/11) all belong to submappers this board refuses, and `$xxx5`'s
mega-UNROM mode ("uses CHR A16 and A17 bits as PRG A18 and A19") is latched and inert — the page
documents no released cartridge that sets it and its own *Notes* say the same of the weird modes.

### `$xxx3` bit 5, which is one clause and two readings

> `Z` — 1: Also enable PRG RAM in `$5000-$5FFF`

That is the whole of it: no mapping for a 4 KiB window over an 8 KiB chip, and no game named that
uses it. **Q-159** holds both readings:

* **a COOLBOY only.** On a MINDKIDS the register file is at `$5000-$5FFF` under mask `$F007`,
  which decodes the *whole* page — `$5400 & $F007` is `$5000` — so there is no room for RAM there
  and the bit cannot mean what it says. The test shows it from that side: on a MINDKIDS, with the
  bit set, a write at `$5400` still banks the cartridge.
* **CPU A0-A12 straight through**, so the window shows the chip's second 4 KiB and `$5400` is
  `$7400`. That is what an ASIC asserting a chip select over an unchanged address bus does, and it
  is the only reading that needs no wiring the page does not give.

"Also" is read as adding a window and not an enable, so the MMC3's `$A001` still gates both sides;
the test pins the write-protected case.

### What it measures to the suites

**1039 passed, 0 failed, 9 ignored** (step 1 left it at 1034/0/9): five tests, one per banking mode
beyond `$00` plus the mixed chip and the second RAM window. `cargo fmt --all -- --check` clean and
`cargo clippy -p zax-nes --all-targets -- -D warnings` clean.

---

## Step 3 — the 64 MiB image, and the ceiling there was never any of

§7.2 step 2 is "Lift the PRG/CHR size ceiling in the loader and in `board_memory_sizes` to what a
64 MiB image needs — a check this build has never had to make, **and the lot's real risk**".

### The finding: there is no ceiling in the loader to lift

**Q-158** records it. Every piece of the path was read and none of them imposes a maximum:

| where | what it does with 64 MiB |
| --- | --- |
| `InesHeader::try_parse` | reads NES 2.0's twelve-bit size field, and its **exponent-multiplier** form when the MSB nibble is `$F` — which is the only form that can say 64 MiB, the twelve-bit field topping out at 65 520 KiB |
| `nes20_rom_size` | `multiplier.wrapping_shl(exponent)` then `usize::try_from(...).unwrap_or(usize::MAX)`: an impossible exponent becomes `usize::MAX`, **not** a panic |
| `RomImage::try_load` | refuses an announcement the file cannot carry, with `TruncatedFile` — which is REQ-CORE-011's typed cause, so the "declaring more is refused" half of the pair is answered by a check that was already there |
| `BankWindows::new` | counts banks into an `i32`: **8 192** of them at 64 MiB, and `i32::try_from(len / size)` is the guard |
| `BankWindows::map_prg_8k` | `bank.rem_euclid(count)`, so every bank number this board composes is in range by construction |

So the step adds **no ceiling and lifts none**; what it adds is the regression test that says so,
`req_core_011_a_64_mib_image_loads_and_a_larger_declaration_is_refused`: a real 64 MiB image
loads and banks, a 128 MiB declaration behind a 16-byte file is refused, an exponent of 63 with
multiplier 7 — the arithmetic that would overflow — is refused the same way, and a 512 MiB CHR
declaration too. No `panic!`, `unwrap` or `expect` is reachable on any of the four.

### PRG A25, the one row of the page a bank-window core cannot express

nesdev's *64 MiB mapping* is the YH2018A's wiring: SC0 (`$xxx1` bit 0) and SC1 (`$xxx3` bit 0)
drive PRG A25 through the ASIC's OA0 pin, and the table has three rows —

| `$xxx1.0` | `$xxx3.0` | the page |
| :-: | :-: | --- |
| 0 | 0 | "PRG A25=PRG A0, i.e. bytes at even addresses come from the first 32 MiB and bytes at odd addresses come from the second 32 MiB half" |
| 0 | 1 | "PRG A25=1, i.e. second 32 MiB half" |
| 1 | x | "PRG A25=0, i.e. first 32 MiB half" |

— of which **the first is not a bank**. A `BankWindows` window is a byte offset plus a length, and
that row interleaves two 32 MiB chips byte by byte. It is also the *reset* state, and the page
says what the hardware's users do about that: "either the code at the reset vector must be
carefully placed with bytes alternating between the two 32 MiB halves, or (as the existing
multicarts do) **must be duplicated in both 32 MiB halves**."

So the board answers the **first** half for that row — the half every known dump duplicates its
boot code into — and **Q-158** records it with the observation that would close it.
`q_158_mapper_268_reads_the_second_32_mib_half_only_when_sc1_is_set_alone` walks all four states
of the two bits and then shows the line wrapping away on a 512 KiB image, which is what the real
chip's missing address line does: the line is driven on every image because **no header field
distinguishes a YH2018A**.

### The `board_memory_sizes` row

`268 => min_prg_ram: 8 KiB`, on the page's "up to 32 MiB of PRG-ROM, 256 KiB of CHR-ROM or -RAM,
and **8 KiB of WRAM**" and on its whole *WRAM usage* paragraph, which is about a game keeping that
WRAM behind the register window. A **floor** and not an exact size, which is mapper 190's and
35's shape in that function: the figure is what REQ-CORE-012's default gives an iNES 1.0 image
anyway, and an NES 2.0 dump that declares more keeps what it declares.

**The CHR is deliberately the header's**, which is mapper 97's reason in this function rather than
the TQROM's: the page gives no single figure to state. Its PCB table gives a *maximum* per board —
"max. 256 KiB CHR-RAM" on the SMD72A, "max. 128 KiB" on the J-852C, "max. 1 MiB CHR-ROM" on the
JTH-813 — and the rows this build serves are CHR-RAM-only or mixed, which only an NES 2.0 byte 11
can describe. `req_core_012_mapper_268_keeps_its_8_kib_of_wram_and_takes_its_chr_from_the_header`
pins both halves, the mixed cartridge with two chips side by side included.

### What it measures to the suites

**1042 passed, 0 failed, 9 ignored** (step 2 left it at 1039/0/9): three tests. `cargo fmt --all
-- --check` clean and `cargo clippy -p zax-nes --all-targets -- -D warnings` clean.

---

## Step 4 — four question tickets, the four sweeps, the round-trip and the configurations

### The tickets

| ticket | what it holds |
| --- | --- |
| **Q-156** | §7.2 step 3's "the lockout … implemented as a **no-op**" against the page's own *WRAM usage* section, which is three instructions of 6502 and a paragraph about why the bit exists. MF-UNL-2's rule applies: the page wins, the lockout is implemented, and the ticket asks that §7.2's sentence be corrected so a later lot does not stub it again. Plus the one reading the page cannot settle: whether the bit can be **cleared** once set (this build re-reads `$xxx3` rather than latching a flag, so it is sticky until a reset) |
| **Q-157** | which of the thirteen PCB codes the board serves, i.e. what "leave the rest refusing" means. The page indexes the codes by **submapper** and the six groups do not share a register layout — four `$xxx0` diagrams and three `$xxx1` diagrams — so submappers 0 and 1 are served (six codes, all twelve titles) and 2 to 11 are refused with REQ-CORE-011's typed cause. The ticket has the table of what each refused group would have got wrong, and records mapper 224's absence |
| **Q-158** | the 64 MiB mapping's first row, "PRG A25=PRG A0", which interleaves two 32 MiB chips byte by byte and is the **reset** state: no `BankWindows` window can say it, so this build answers the first half — the half the page says the existing multicarts duplicate their boot code into. **Part 2 of the same ticket** records that §7.2 step 2's "lift the PRG/CHR size ceiling in the loader" describes a ceiling this loader does not have, with the five places on the path read one by one |
| **Q-159** | `$xxx3` bit 5's "Also enable PRG RAM in `$5000-$5FFF`", one clause with no mapping behind it: three readings (a COOLBOY only, CPU A0-A12 straight through, and "also" adding a window rather than an enable), each with what would close it. The ticket also offers the cheaper alternative — not serving the window at all until a dump is found — because no title the census counts is said to use it |

All four carry a `// Q-NNN` reference in `mmc3_outer.rs` and a named test. **Q-156 is the one a
reviewer can close in a session**: it is a reading of two documents that are both in front of them,
not a measurement.

### The four sweeps of §7.1 item 1

| sweep | what it checks for 268 |
| --- | --- |
| `req_core_100_every_required_mapper_is_registered` | the expected-additions table, **built from the features**, is now **92** rows and the ascending-order assertion covers the insertion |
| `req_core_102_any_other_mapper_is_refused` | unchanged — 268 is not on the refused list, and `Cartridge::is_supported(268)` is true even for the submappers the board declines |
| `req_core_090_the_mapper_state_survives_a_save_and_restore` | reaches the number by registration; it writes `$8000-$E001` and `$6000`, and of this board's six registers only `$xxx0` is at any of those — which is why the board also has its own round-trip below |
| `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` | the reset path, including a `$6000` write, which **is** a register of this board |

and `mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` — the invariant, not a list —
gains `268` in its expected vector **once** rather than twice, because the sweep builds every
number at submappers 0 *and* 3 and this is the first number in `SUPPORTED_MAPPERS` that refuses
one. All three in-crate sweeps now go through `latched_cartridge`, which skips a refused pair and
asserts the pair was mapper 268's and the cause `UnsupportedMapper`, so they still fail on any
other refusal.

### The save-state round-trip (§7.1 item 2)

`req_core_090_mapper_268_round_trips_its_six_outer_registers` writes all six registers, the eight
MMC3 ones, a byte into the CHR-RAM chip `$xxx4` selected and a byte into the WRAM behind the
register window; saves; moves every one of the six; requires the PRG and CHR layouts, the CHR-RAM
byte and the WRAM byte back, and the **re-saved block identical byte for byte**. Then it resets and
requires the power-on layout, which is AMD-061 §5 for this board.

The block is `Mmc3`'s own followed by six bytes, appended behind it — so no byte of mapper 4's,
118's or any other board's state moved (REQ-CORE-090). `chr_ram_windows` is derived and carries no
byte: `load_registers` brings it back through `refresh`.

### The configurations

clippy `-D warnings` on `zax-nes --all-targets` **and** its tests, in every configuration the lot
adds plus the two §7.1 requires, and on `zax-harness --all-targets` in `default`:

| configuration | clippy | tests |
| --- | :-: | --- |
| `default` | clean | **980 passed, 0 failed, 8 ignored** (MF-UNL-5 left it at 965) |
| `--no-default-features` | clean | **783 / 0 / 8** — unchanged by this lot, as by the four before it |
| `--features parked-boards` | clean | **1033 / 0 / 8** (the VS. System's 53 beside the default's 980, less the 8 that overlap) |
| `--no-default-features --features board-268` | clean | **798 / 0 / 8** |
| `zax-harness --all-targets`, `default` | clean | — |

**All fifteen of the lot's tests are reachable with `board-268` alone** — 798 against 783 — which
is the first MF-UNL lot in five where the *alone* configuration found nothing: there is one
feature and one board, so there is no cross-board test to gate and no constant that only a second
board's code reads.

`cargo test --release -p zax-nes -p zax-harness` is **1043 / 0 / 9** (step 3 left it at 1042/0/9).

---

## Step 5 — the callgrind campaign, and one feature in `default`

Three binaries, each built by `build-clean.sh` from its own tar into a **fresh** directory with
its own `CARGO_TARGET_DIR`, which is then deleted. Roots:
`ZAX_TEST_ROMS=…/assets-free/test-roms`, `ZAX_COMM_ROMS=…/demo/roms`,
`ZAX_L2_ROMS=…/test-originals`, `ZAX_L2_PACKS=…/campaign/play-user/profiles` (the 101 `.3dn`
profiles).

| label | tree |
| --- | --- |
| `unl6base` | **`a4315007`**, the lot's base commit |
| `unl6` | **`499e1aa2`**, step 4 — the lot as it ships |
| `unl6off` | the shipped source with `board-268` **out of `default`** — the control, whose emulation is the base commit's: with the feature off the number is not in `SUPPORTED_MAPPERS`, so no cartridge of it can exist |

### The base measurement was already on disk, and that is a check and not a shortcut

`zax-harness-unl6base` is **byte-identical** (`md5 2f56dfb40b4093a7569f8f6f45b13506`) to
MF-UNL-5's own `zax-harness-unl5` *and* to its `zax-harness-unl5ship` — the lot's base commit
adds only documentation over the commit MF-UNL-5 measured. So `cg-unl5` and `l2-unl5` are this
lot's base figures, this lot's deltas are directly comparable with MF-UNL-5's, and the whole
campaign is on one instrument. The base was built anyway, to prove the identity rather than assume
it.

`unl6off` is **not** byte-identical to the base, and the reason is worth writing down: the two PPU
watch lists in `cartridges::mod` gain `| 268` **unconditionally**, which is the "one `matches!` is
one `matches!`" precedent mappers 68, 111 and the J.Y. ASIC's four numbers already set in that
file. The *emulation* is identical either way — a number whose board is off is never created — so
`unl6off` is the same-behaviour pair the brief asks for, and it is also the measurement of what
gating the feature off costs.

### PR-1's thirteen ROMs, 300 frames, no pack, each against `a4315007`

| stage | set total | % | worst ROM | ROMs below |
| --- | ---: | ---: | --- | ---: |
| **`unl6` — the lot** | **−150 669** | **−0.000** | `vblnmi` **+0.001 %** (+20 578) | 7 |
| `unl6off` — the control | **−36 771** | −0.000 | `smb1` / `vblnmi` **+0.001 %** | 8 |

**The worst ROM of the lot is +0.001 %, a twentieth of the tolerance, and the set total is
below the base commit.** The control's worst ROM is the same +0.001 % on two of the same ROMs, so
on this instrument the board costs less than the spread of gating it off — the cleanest reading
the PR-1 set can give, and the same reading MF-UNL-5 got.

### Level 2: five games, 600 frames with their 3D packs, inputs pressed

| build | `core` | `total` |
| --- | --- | --- |
| **`unl6`** | **all five +0, identical to the instruction** | −0.008 % (`smb1`) to **+0.0195 %** (`smb3`) |
| `unl6off` | **all five +0** | −0.015 % (`smb1`) to +0.001 % (`metroid`) |

`ppu`, `apu` and `draw_list` are **+0 to the instruction** on all five games in both builds too,
`zelda2/draw_list` +1 337 excepted. So nothing this project runs per dot, per cycle or per tile
moved at all.

**Every figure of both columns is inside the owner's +0.02 %**, which is what the brief asks be
checked per ROM on `total` as well as on `core`. `smb3/total` **+0.0195 %** is the one figure near
the edge, and the next section is the same-behaviour pair the brief requires for it.

### `smb3/total`, shown with its pair as the brief requires

| `smb3`, `unl6` vs `unl6base` | |
| --- | --- |
| `core`, `ppu`, `apu`, `draw_list` | **+0, identical to the instruction** |
| `convert` | +4 293 796 (**+0.144 %**) |
| `lua_hooks` | +4 335 824 (**+0.253 %**) |
| the only symbols above 300 000 | `luaH_getshortstr` **+4 308 974**, `__memcpy_avx_unaligned_erms` +432 596 |

So 4.31 M of a 4.73 M whole-run move is **one Lua function** — `ltable.c`'s short-string hash
lookup — and **no symbol of this project moves by 300 000 on any of the five games**.

**The pair is `unl6off`**, whose emulation is the base commit's. On the same instrument it moves
the same symbol by comparable amounts in the *other* direction: `luaH_getshortstr` **−1 288 850**
on `smb1` and **−1 000 081** on `zelda2`, with `getgeneric` −1 330 708 and −718 136 and `luaH_set`
+344 074 — a build that changes no emulation at all moving that one column by over a million
instructions a game, both ways. That is the finding MF-UNL-2 step 8, MF-UNL-3 step 7 and MF-UNL-5
step 5 each recorded, and this lot confirms it a fourth time. The tolerance is **not** redefined:
the figure is inside it, and what is recorded is where it comes from.

### The decision

| feature | decision | on what |
| --- | :-: | --- |
| `board-268` | **`default`** | PR-1 worst ROM **+0.001 %**, set total **−150 669**; level 2 **all five `core` +0 to the instruction**, worst `total` +0.0195 %, every figure inside the tolerance |

**No board is parked**, and `parked-boards` keeps the one feature it has had since MF-VS. The
gating is free either way — `unl6off` is set total −36 771 with all five level-2 `core` figures
+0 — so the owner's rule gives no reason to move the one board out of `default`.

### Disk and machine

G: stayed above **18 GiB** free throughout. Each of the three build directories was deleted by
`build-clean.sh` as soon as its binary was copied out; the three source tars were deleted after
the last build. No GPU campaign; every long run in the foreground with its output read, as the
headless rule requires.

---

## Step 6 — golden-core against the base commit, and the full check

### The binary that was measured is the binary the commit builds

Step 5's figures come from `unl6`, built from step 4's commit `499e1aa2`. Step 5 added only its own
progress note, so this step built `unl6ship` from **`c1376dc3`** by `git archive` into a fresh
directory and compared:

    4b457c6c14d74216f57327b95b2a4fda  zax-harness-unl6
    4b457c6c14d74216f57327b95b2a4fda  zax-harness-unl6ship

**Byte-identical**, so every figure of step 5 is the shipped commit's. (And `unl6base` is
byte-identical to MF-UNL-5's own `unl5` and `unl5ship`, as step 5 recorded, so the whole campaign
is on one instrument.)

### golden-core over the 98 commercial originals, 1 200 frames each

    zax-harness-<label> golden-core /mnt/g/3dsen/test-originals gc98-<label>.jsonl 1200

| | |
| --- | --- |
| `a4315007` (`unl6base`) against the lot (`unl6ship`) | **byte-identical** |
| `a4315007` against the lot with `board-268` out of `default` (`unl6off`) | **byte-identical** |

`md5 f79e7df842c3ee43e7d9b67861a927b5` for all three 98-line dumps — **the digest MF-UNL-2 step 9,
MF-UNL-3 step 8, MF-UNL-4 step 8 and MF-UNL-5 step 6 all recorded**, so five lots and twenty-nine
mapper numbers later no commercial original's pixels, tile records, cycles, audio digest or blargg
status has moved. golden-core runs with no hack code, so there is nothing between the ROM and the
core.

### The suite, with the ROM roots set

| | |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **1043 passed, 0 failed, 9 ignored** |
| the same `--no-fail-fast -- --include-ignored`, with `ZAXNES_ROMS=…/demo/roms` and `ZAXNES_TEST_ROMS=…/assets-free/test-roms` | **1052 passed, 0 failed, 0 ignored** |

The nine ignored are the ROM-driven ones, and the second row is the proof they ran rather than
passing vacuously: nine moved out of the ignored column and into the passing one.

### fmt and clippy

`cargo fmt --all -- --check` clean. `cargo clippy -D warnings` clean on **both** crates the lot's
tests touch — `zax-nes --all-targets` in all four configurations (step 4's table) and
`zax-harness --all-targets` in `default`. `zax-app` does not build in this worktree and the lot
touches none of it.

### The owner's rules, one line each

| rule | this lot |
| --- | --- |
| one feature per board that costs | one feature, one board |
| +0.02 % per ROM on PR-1's thirteen and level 2's five, `total` **and** `core` | worst PR-1 ROM **+0.001 %** (set total **−150 669**); level 2 all five `core` **+0 to the instruction**, worst `total` **+0.0195 %** (`smb3`), whose 4.31 M is one Lua symbol |
| a board over the tolerance is parked, never dropped | none is over; `parked-boards` keeps the one feature it has had since MF-VS |
| no exclusion for performance | the number is in `default` and reachable behind its feature with `--no-default-features` too |
| no change to the emulation of an existing mapper | golden-core **byte-identical** over the 98 originals, and 268 is a new number |
| no change to the save-state format of existing boards | `NesCore::STATE_VERSION` unchanged; the six bytes added are appended behind `Mmc3`'s own block and behind the `cfg` that is the only reason the board exists |
| safe Rust | no `unsafe` anywhere in the lot |
| nothing edited under `spec/` but new Q tickets | `Q-156.md`, `Q-157.md`, `Q-158.md` and `Q-159.md`, all new |
