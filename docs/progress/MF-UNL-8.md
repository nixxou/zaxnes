# MF-UNL-8 — Huang-1's last two numbers, Cony/Yoko, and J.Y.'s simple board

Lot **MF-UNL-8** of `docs/review/UNL-census.md` §7.2, the eighth of the twelve MF-UNL lots:
**five numbers, three boards, three Cargo features, ~10 original titles.**

Base commit: **`c895a93a`** (`MF-UNL-7 (9/9)`), the branch tip when the lot opened.
Branch: `mapper-review`. Worktree: the `wt-mappers` worktree.

| feature | numbers | board |
| --- | ---: | --- |
| `board-huang1` | **12, 14** | the **Huang-1** ASIC on its other two PCBs: the Gouder **SL-5020B** (12), where the chip "functions exclusively as an MMC3A clone" beside a GAL that adds an outer CHR register at $4132, and the Gouder **SL-1632** (14), where a PAL puts a supervisor register at **$A131** over the chip's own registers and gives it **two** modes and **three** independent CHR A18 bits |
| `board-cony` | **83, 264** | the Cony / Yoko chip-on-board ASIC: a PRG base, a mode register, four 8 KiB PRG registers, eight 1 KiB CHR registers, a 16-bit IRQ counter whose clock is **selectable between CPU M2 and PPU A12**, a solder-pad read and four bytes of scratch RAM |
| `board-91` | **91** | the **JY830623C** / **YY840238C** (submapper 0) and the **EJ-006-1** (submapper 1): four 2 KiB CHR registers and two 8 KiB PRG registers in $6000-$7FFF, and two IRQ counters that share nothing but their start and stop ports |

## The three things this lot has to carry from the census

### 1. MF-UNL-2's rule, and this lot is where it bites hardest

Where §7.2 and a number's own page differ, **the page wins**, and each case gets a Q ticket.
§7.2's step 1 says mapper 12 "is `Somari` with one more register" — three chips and a supervisor
register, as mapper 116 has. Mapper 12's own page says the opposite in one sentence: "The Huang-1
ASIC functions **exclusively as an MMC3A clone** on this PCB". There is no supervisor register on
that board, no VRC2 mode and no MMC1 mode; what the GAL16V8 adds is an outer CHR register and a
language-bit read, both at $4132. The page wins (Q-164).

### 2. Reuse through parameters, without moving what is already here

The instruction is exact: each board is "a near-relative of a board this build already has: reuse
it through parameters **without moving the existing boards' hot path or save-state format**". So
what this lot reuses is the **sub-chips** — `Mmc3` and `Vrc2Vrc4`, through the `pub(crate)`
accessors `Somari` already needed — and not `Somari` itself. `Somari` is not touched by a line:
with `board-huang1` off, every file of `board-116` is byte for byte what the base commit shipped,
and with it on `Somari`'s own struct, save-state block and hot path are unchanged too. The same
holds for the five Konami numbers and `Vrc2Vrc4`: the one field mapper 14 needs is behind
`board-huang1`, on the `board-298` precedent.

### 3. The performance rule as the instruction states it

The +0.02 % tolerance is on **each ROM's total** instruction count — the PR-1 set and the level-2
games' `total` — and not only on the `core` column. A total that moves further than that with the
cause outside the emulation (the Lua `ltable.c` column is the one this project has seen) is shown
with a same-behaviour pair and recorded; the tolerance is not redefined.

## Steps

- [x] **0/8** — the plan: this file.
- [x] **1/8** — `board-huang1`, mapper **12**: the Huang-1 as an MMC3A clone, the outer CHR
  register at $4132 and its read side.
- [ ] **2/8** — `board-huang1`, mapper **14**: the supervisor at $A131 over the sub-board's own
  registers, two modes, three CHR A18 bits and the page's two Nestopia caveats.
- [ ] **3/8** — `board-cony`: mappers **83** and **264**, four of the page's five PCB variants.
- [ ] **4/8** — `board-91`: both submappers, two IRQ counters, the address-driven outer register.
- [ ] **5/8** — the question tickets.
- [ ] **6/8** — the four sweeps, the save-state round-trips and the feature configurations.
- [ ] **7/8** — the callgrind campaign against the base commit, and the feature decision per board.
- [ ] **8/8** — golden-core, the full `--include-ignored` run with both ROM roots,
  `BOARD-FEATURES.md`, the census note, the journal and the final pass.

(Nine entries and not six: §7.2's step list puts the two Huang-1 numbers in one step and they
share only the ASIC — 12 has no supervisor register at all and 14's is a different decode with a
different field layout — so each takes its own, and §7.1's three common things are split from the
tickets because the tickets are what the sweeps are written against.)

---

## Step 0 — the plan, and the five pages read before any code

The five numbers' pages were read in full before the design was settled, because three of them say
something §7.2 does not.

### Mapper 12, the Gouder SL-5020B (submapper 0)

* The chip is "a chip-on-board variant of the same Huang-1 ASIC as mappers 14, 116 and 238 plus
  GAL16V8 programmable logic", and **"The Huang-1 ASIC functions exclusively as an MMC3A clone on
  this PCB"**. So: an `Mmc3` with the **revision A** counter — "MMC3A-specific IRQ behavior is
  necessary for running *Dragon Ball Z 5*" — and nothing else of mapper 116's board.
* The GAL's outer CHR register, `$4132`, mask `$E100`:

      D~7654 3210 (write)
        ---------
        ...B ...A
           |    +- CHR A18 when PPU A12=0
           +------ CHR A18 when PPU A12=1

  Bit 0 and **bit 4**, one CHR A18 per pattern table. "Because this register is implemented
  outside the ASIC, it is not affected by MMC3 $8000 bit 7, and writes take effect immediately."
* The same address **read** is `.... ...A`, a language bit, and the page says the GAL has it
  "hard-wired (to Chinese text in all known copies)" without saying which value Chinese is.
* FCEUX's latch behaviour — the register taking effect on the next MMC3 CHR write — is named on
  the page as *not* the hardware's, with the consequence spelled out: "The homebrew hack
  *Ultimate Mortal Kombat III* relies on FCEUX' behavior and would therefore not work on the
  original PCB." This build is the PCB.
* **Submapper 1, the Front Fareast Magic Card 4M, is not implemented**: it is copier hardware with
  a 512-byte trainer that must be "initialized by JSRing to $7003 before JMPing to the game's
  reset vector", which is §1.8 of the census's reason for leaving 6/8/17 out, and §7.2 says so by
  name.

### Mapper 14, the Gouder SL-1632

* "Like INES Mapper 116, it uses the Huang-1 ASIC together with a PAL that provides a supervisor
  register for selecting CHR A18 and for switching between MMC3 and VRC2 modes (A0/A1, VRC2b), but
  moves the supervisor register address to **$A131**, overlapping the MMC3 and VRC2 registers
  there."
* The supervisor register, mask `$FFFF?` — the question mark is the page's:

      D~7654 3210
        ---------
        A.B. C.M.
        | |  | +-- Mapper mode
        | |  |      0: VRC2
        | |  |      1: MMC3
        | |  +---- CHR A18 for PPU $0000-$0FFF
        | +------- CHR A18 for PPU $1000-$17FF
        +--------- CHR A18 for PPU $1800-$1FFF

  **Bit 1** the mode, **bit 3**, **bit 5** and **bit 7** the three CHR A18 bits — which is MF-6b
  step 6's table to the bit, and not the bit-0/2/4/6 reading a prose summary of the same page
  gives. Three unequal window groups: 1 KiB windows 0-3, 4-5 and 6-7.
* Two modes and **no MMC1 mode**, so the board holds an `Mmc3` and a `Vrc2Vrc4` and no `Mmc1`.
* "According to Nestopia's mapper source code, mirroring can only be controlled in VRC2 mode, and
  the VRC2 PRG and mirroring registers do not respond to address mirrors. These behaviors
  contradict other emulators and have not been verified on original hardware." Both are
  implemented, and the caveat is in the code comment: the second one is what keeps a supervisor
  write at $A131 off the VRC2's own $A000 PRG register, and the first is what the `Mmc3`'s
  `ignore_a000` already expresses.

### Mappers 83 and 264, Cony / Yoko

One page, one ASIC, **five** PCB variants: 83.0 (256 KiB CHR), 83.1 (512 KiB CHR), 264 (512 KiB
CHR, 2-in-1), 83.2 (4-in-1 with 32 KiB PRG-NVRAM) and 83.3 (4-in-1, 128 KiB inner PRG). Mapper 264
"connects CPU A11.A10 to the ASIC's A9..A8 inputs, so all addresses change accordingly", which is
one expression and not a second decode. The register map is complete on the page: PRG base
($8000), the mode register ($8100: mirroring, PRG banking mode, the $6000 window, the IRQ
direction and the IRQ arm), the 16-bit IRQ counter ($8200-$8201), four 8 KiB PRG registers
($8300-$8303), eight 1 KiB CHR registers ($8310-$8317), the IRQ clock source ($8318), the solder
pad ($5000, read) and four bytes of scratch RAM ($5100-$5103).

**Submapper 2 is refused at load** and 83.0, 83.1, 83.3 and 264 are implemented — see step 3 and
Q-167: that PCB alone puts 32 KiB of **PRG-NVRAM** behind $6000-$7FFF with its bank in the PRG
base register, which needs `Mapper::prg_ram_window` forwarded through `IrqBoard` — a body in a
`Board` dispatch this lot has no measurement for — for a 4-in-1 multicart that carries no original
title. MF-UNL-6's Q-157 is the precedent, in the same words.

### Mapper 91, the J.Y. clone board

* Submapper 0 (JY830623C, YY840238C) "has a PA12-based IRQ counter and hard-wired mirroring";
  submapper 1 (EJ-006-1) "an M2-based IRQ counter and selectable mirroring". The register masks
  differ with them: `$F003` on submapper 0 and `$F007` on submapper 1.
* Four 2 KiB CHR registers at $6000-$6003, two 8 KiB PRG registers at $7000-$7001, $C000-$FFFF
  "hard-wired to last bank", IRQ stop/acknowledge at $7006 and start/reset at $7007 with the value
  ignored on both, and on submapper 1 two mirroring ports ($6004 horizontal, $6005 vertical) and a
  16-bit counter register ($6006 LSB, $6007 MSB).
* The outer bank register of submapper 0 is an **address** and not a datum:

      A~FEDC BA98 7654 3210
        -------------------
        .... .... .... .PPC
                        ||+- Select outer 512 KiB CHR-ROM bank (CHR A19)
                        ++-- Select outer 128 KiB PRG-ROM bank (PRG A17-A18)

  over $8000-$9FFF with "Mask: unknown".
* The two counters: submapper 0's "is a binary counter clocked by PPU A12, and simply counts a
  fixed number of 64 unfiltered rises of PPU A12"; submapper 1's "is a binary counter clocked by
  the M2 signal with a factor of 5/4, meaning counting down by five every fourth M2 cycle."
* "The commonly available ROM image of *Super Fighter III* is a mapper hack for INES Mapper 197" —
  a comment on the available dumps, so that title is not evidence for this board and is not in the
  titles below.

### Titles R0 should source

*Dragon Ball Z 5* (12); *Samurai Spirits*, the 8-character version (14); *Street Fighter II Pro*,
*Fatal Fury 2*, *World Heroes 2'*, *Super Blaster VII Turbo 28* (Cony, plus *Mortal Kombat II/V
Pro* and *Master Fighter VI* on 264); *Street Fighter 3*, *Mortal Kombat II*, *Dragon Ball Z 2*,
*Mario & Sonic 2* (91). **No ROM of any of the five numbers exists under
`assets-free/test-roms`** (§5 of the census), so nothing in the campaign exercises them and every
test below is written from the register tables above.

### Where the three boards go

All three hold an IRQ counter that can pull /IRQ, which is [`IrqBoard`]'s entry condition, so each
is one variant of that enum and **`Board` does not grow a discriminant** — the measurement
`IrqBoard`'s own note records (+0.080 % on `mmc5` for two new `Board` discriminants) is the reason.
Three new inner variants take that enum from sixteen to nineteen; step 7 measures them.

[`IrqBoard`]: ../../rust/crates/zax-nes/src/cartridges/mappers/irq_boards.rs

[PROGRESS] MF-UNL-8 step 0/8 done: the plan, the five pages read in full, and the three places
§7.2 and a page disagree named for step 5's tickets.

---

## Step 1 — mapper 12, the Huang-1 with one mode and a GAL beside it

`board-huang1`, in `default`, implying nothing. One new module,
`cartridges/mappers/huang1.rs`; one new `IrqBoard` variant (boxed); one new `BankWindows` method;
two widened `cfg` lists on `Mmc3`'s existing `pub(crate)` accessors; and **not one line of
`Somari`**.

### The sentence that decides the whole shape

> The Huang-1 ASIC functions **exclusively as an MMC3A clone** on this PCB; MMC3A-specific IRQ
> behavior is necessary for running *Dragon Ball Z 5*. The GAL16V8 provides an outer CHR bank
> register at $4132 to support 512 KiB of CHR-ROM.

So mapper 12 is **not** three chips and a supervisor register. There is no supervisor register, no
VRC2 mode and no MMC1 mode; what there is beside the chip is a GAL with one write register and one
read bit. §7.2's "it is `Somari` with one more register" is refuted by the number's own page, which
is MF-UNL-2's rule, and **Q-164** records it with the three other things this page leaves to a
choice.

What the board therefore reuses is the **chip** and not mapper 116's board: `Huang1` holds an
`Mmc3`, calls `Mmc3::refresh_banks` (the accessor `Somari` already needed, its `cfg` widened by one
feature) and `Mmc3::force_irq_revision_a` (the accessor mappers 114 and 268 already needed, the
same). `Somari` is untouched, so with `board-huang1` off every file of `board-116` is byte for byte
what `c895a93a` shipped, and with it on mapper 116's struct, its save-state block and its hot path
are unchanged.

### The outer CHR register, and the one new `BankWindows` method

`$4132`, mask `$E100`, `...B ...A` — **bit 0** is CHR A18 for PPU $0000-$0FFF and **bit 4** for
$1000-$1FFF, one line per pattern table. `$E100` is A15, A14, A13 and A8 and nothing else, so the
register repeats through the whole of $4100-$5FFF wherever A8 is set; `$6100 & $E100` is `$6100`,
so **no address in $6000-$7FFF reaches it** and the board keeps the PRG-RAM window an MMC3 has
(see the `board_memory_sizes` comment, which is mapper 133's reason in the same words).

MF-6b step 6 predicted the gap this leaves in `BankWindows`: "no way to express a 1 KiB window
table whose A18 changes at `$1000` and again at `$1800` other than by computing it per window".
Mapper 116's `offset_chr_banks` adds one bank number to all eight windows, which cannot say that,
so this step adds **`or_chr_banks_each([i32; 8])`** behind the feature — an **OR**, because both
pages call the thing an address line, which is also what makes it idempotent: the board re-applies
the lines after every register write and never remembers what it last laid down.

That idempotence is what lets the page's own two timing sentences fall out rather than be coded:

* "Because this register is implemented outside the ASIC, it is **not affected by MMC3 $8000 bit
  7**" — bit 7 swaps which of the chip's registers drive which windows *inside* the chip, and the
  lines go over the windows afterwards, so the swap moves banks and not which windows carry A18.
* "**writes take effect immediately**" — `Huang1::refresh` runs on the $4132 write itself.
  FCEUX's reading, "a temporary latch register that takes effect on the next write to an MMC3 CHR
  bank register", is deliberately **not** implemented, and the page's own next sentence is why:
  the homebrew *Ultimate Mortal Kombat III* "relies on FCEUX' behavior and would therefore not work
  on the original PCB". This build is the PCB (Q-164).

### The read side, and submapper 1

A read of the same address is `.... ...A`, the bit *Dragon Ball Z 5* uses "to select between
Chinese and English text". The page does not say which value is Chinese — the GAL "has the value
hard-wired (to Chinese text in all known copies)" — so the figure is Q-164's choice (zero) and what
the test pins is the shape the page does give: the board answers inside the `$E100` mask and
**drives nothing outside it**, which is `try_cpu_peek` answering `Some(0)` against `None`. The
forward it needs, `Mapper::try_read_prg`, is one `IrqBoard` already had for the scrambled MMC3
clones, so no `Board` dispatch grows a body.

**Submapper 1 is refused at load** with REQ-CORE-011's message: it is the Front Fareast Magic
Card 4M, "ROM images that have been extracted from disk images" for copier hardware whose images
need the Super Magic Card's register file and a 512-byte trainer. §1.8 of the census leaves 6, 8
and 17 out for exactly that, and §7.2 leaves this submapper out by name.

### Where it sits, and what it costs the library

`IrqBoard::Huang1(Box<Huang1>)` — the board holds the chip's own counter, which is that enum's
entry condition, so `Board` does not grow a discriminant and the +0.080 % on `mmc5` that two new
ones cost is not paid. Boxed on `IrqBoard::Somari`'s own measurement: mapper 14 will add a
`Vrc2Vrc4` beside the `Mmc3`, which inline would set the enum's size. Every method the board needs
is one this variant already forwards for an earlier board — `try_read_prg`, `prg_ram_enabled`,
`prg_ram_writable`, `on_ppu_address_bus`, `irq_asserted` — and the three `cfg` lists that grew by
one feature are **behaviour-neutral for the other boards**, as each forward's own note already
records.

Mapper 12 joins both PPU watch lists (`watches_ppu_reads`, `watches_ppu_bus_without_read`) and the
expected list of `mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`, which is the test
§4.3 of the inventory exists to catch: it failed with its own list until the pair was added.

### Tests

| test | what it pins |
| --- | --- |
| `req_core_102_mapper_12_drives_chr_a18_per_pattern_table_from_4132` | the four values of the two lines over 512 KiB of CHR, read through `chr_peek_view().offsets()` because the fixture's fill byte cannot tell bank 0 from bank 256; and the census's own "with $4132 clear it is not" |
| `req_core_102_mapper_12_decodes_4132_under_e100_and_acts_at_once` | seven mirrors the mask admits and four it excludes ($6132 and $7132 among them), the write taking effect with no CHR register touched after it, and $8000 bit 7 moving the chip's banks without moving which windows carry A18 |
| `req_core_102_mapper_12_answers_a_read_of_4132_with_the_language_bit` | `Some(0)` inside the mask, **`None`** outside it |
| `req_core_011_mapper_12_submapper_1_is_the_magic_card_4m_and_is_refused` | the refusal, and that submapper 0 and an iNES 1.0 image are both the SL-5020B |
| `req_core_102_mapper_12_leaves_the_mmc3_numbers_alone` | $4132 is nothing on mapper 4, and a mapper-12 submapper byte is not an MMC6's |
| `req_core_114_mapper_12_has_the_mmc3a_counter_whose_zero_latch_disables_the_irq` | the revision A/B A/B against mapper 4: both let the first clock through (the `$C001` reload is pending), and after an acknowledge revision B fires again where mapper 12 goes quiet — `Mmc3Board::clock_irq_counter`'s own reading of AMD-061 §8, which the test had to be written *to* rather than against |

`cargo test --release -p zax-nes`: **all suites green**, 313 in `mapper_tests` and 58 in
`mapper_irq_tests`. `cargo fmt --check` clean; clippy `-D warnings --all-targets` clean in
`default`, `--no-default-features` and `--no-default-features --features board-huang1`.

[PROGRESS] MF-UNL-8 step 1/8 done: mapper 12 is the Huang-1 as an MMC3A clone and not mapper 116's
board — the page refutes §7.2 — with one GAL register at $4132, its language-bit read, submapper 1
refused and `or_chr_banks_each` the per-window CHR A18 table MF-6b step 6 named.
