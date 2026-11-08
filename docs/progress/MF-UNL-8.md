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
- [x] **2/8** — `board-huang1`, mapper **14**: the supervisor at $A131 over the sub-board's own
  registers, two modes, three CHR A18 bits and the page's two Nestopia caveats.
- [x] **3/8** — `board-cony`: mappers **83** and **264**, four of the page's five PCB variants.
- [x] **4/8** — `board-91`: both submappers, two IRQ counters, the address-driven outer register.
- [x] **5/8** — the question tickets.
- [x] **6/8** — the four sweeps, the save-state round-trips and the feature configurations.
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

---

## Step 2 — mapper 14, the same ASIC with a PAL that sits on top of it

The same feature, the same module and the same `IrqBoard` variant; one `Vrc2Vrc4` field beside the
`Mmc3`, two `cfg` widenings and one new `cfg`'d field on `Vrc2Vrc4` with three guards that read it.

### MF-6b step 6's design, implemented to the bit

That step decided mapper 14 out of mapper 116's board and recorded its design as a four-row table.
Every row held when the page was read again, and the one that matters most is the **bit
positions**:

```text
D~7654 3210
  ---------
  A.B. C.M.
  | |  | +-- Mapper mode (0: VRC2, 1: MMC3)
  | |  +---- CHR A18 for PPU $0000-$0FFF
  | +------- CHR A18 for PPU $1000-$17FF
  +--------- CHR A18 for PPU $1800-$1FFF
```

**Bit 1** is the mode and **bits 3, 5 and 7** the three lines, which is what MF-6b step 6 wrote.
A prose summary of the same page gives bit 0 and bits 2/4/6 — one place lower throughout — and the
diagram's own columns settle it. The `D~7654 3210` header was read column by column against the
`A.B. C.M.` row and the pointer lines under it before a line of this step was written; the step-0
section of this file has the transcript.

So: `MODE_BIT_14 = 0x02`, `CHR_A18_BITS_14 = [0x08, 0x20, 0x80]`, and `chr_a18_windows` returns
`[low, low, low, low, middle, middle, high, high]` — the **unequal** groups that are the reason
this is a board and not mapper 116's `C` bit. `or_chr_banks_each` is what carries them, which is
step 1's method doing the job MF-6b step 6 named it for.

### Two modes and no MMC1

The page's own sentence is "a supervisor register for selecting CHR A18 and for switching between
MMC3 and VRC2 modes (A0/A1, VRC2b)". One mode bit, two chips, and no value of the register can
reach a third — which a test pins, because mapper 116's three-mode board is the thing a reader
would expect here. The VRC2 half is `VrcVariant::Vrc2b`, the same configuration mapper 116 asks
for, so it is `Vrc2Vrc4::new_vrc2b` with one rule added rather than a new variant.

### The page's two Nestopia-sourced rules, and its own caveat

> According to Nestopia's mapper source code, **mirroring can only be controlled in VRC2 mode**,
> and **the VRC2 PRG and mirroring registers do not respond to address mirrors**. These behaviors
> contradict other emulators and have not been verified on original hardware.

Both are implemented, and the page's last sentence is in the code comment and in Q-165 — this is
the one place in the lot where the page itself says its rule is unverified, and the choice is to
follow it rather than the emulators it says it contradicts.

* **Mirroring from the VRC2 alone.** The MMC3 half's one mirroring register is `$A000` and it has
  no other function, so on this board it drives nothing. `Mmc3Board::ignore_a000` is already
  exactly that condition — mapper 4's NES 2.0 submapper 2, a four-screen header and the TxSROM
  reach it at load — so the step adds `force_ignore_a000`, the shape `force_irq_revision_a`
  already had, and no field and no branch anywhere.
* **No address mirrors on the VRC2's PRG and mirroring registers.** One `cfg`'d `bool` on
  `Vrc2Vrc4` and three guards, `exact_prg_and_mirroring_decode` / `ignores_mirror`, on the
  `$8000`, `$9000` and `$A000` arms of `write_register`. All three registers have their low twelve
  address bits clear, so "is a mirror" is `address & 0x0FFF != 0` — one test for the three. This is
  the `board-298` precedent to the letter: **with `board-huang1` off the field and the three guards
  do not exist**, so `Vrc2Vrc4` and every function of it is byte for byte what `c895a93a` shipped
  and the five Konami numbers pay nothing.

And the second rule is what makes the first one's register reachable at all, which is the part of
this board that only falls out when both are implemented: `$A131` is a mirror of the VRC2's
`$A000`, so in VRC2 mode a supervisor write does **not** clobber the chip's second PRG bank.

### The overlap at $A131, and the order

`$A131 & $E001` is `$A001` — the MMC3's PRG-RAM protect register — and `$A131` sits in the VRC2's
`$A000` block, so one write is two writes. Census §7.2 states the order ("the board's decode runs
*before* the sub-board's") and `Huang1::write_register` does that: the supervisor is latched first
and the write then goes to the chip the **new** mode names.

The test the census asks for is written on the side-effect this makes observable without inventing
a probe: in MMC3 mode, writing `$02` to `$A131` is MMC3 mode *and* `$A001` with bit 7 clear, so the
PRG-RAM goes away — `try_cpu_peek(0x6000)` turns from `Some` to `None` — and `$82` brings it back.
In VRC2 mode nothing of the pair reaches the chip, for the page's own reason above.

The decode is `$A131` **exactly**, which is the page's "Mask: $FFFF?" with the page's own question
mark; Q-165 records that the page is unsure. It matters because every near miss is a real register
of the sub-board — `$A130`/`$A132` are the MMC3's `$A000`/`$A001` and the whole `$A000` block is
the VRC2's PRG register — so a board that decoded loosely would switch mode on an ordinary bank
write. Seven near misses are tested.

### The save-state block

Mapper 14 writes the supervisor byte, the MMC3's block and the VRC2's; **mapper 12 writes only the
first two**, because its board has no VRC2 at all. That is `Mmc3Board`'s own shape for mapper 195's
GAL byte and the two multicarts' outer register — a block carries what the board has. Within mapper
14's half, mapper 116's reason governs: the page requires each chip to keep its own registers across
a mode switch, so both are written whatever the mode is, and the supervisor goes first so the mode
is known before any block is read. On load both chips are put back to their register-derived windows
before the lines go on, which is mapper 116's `load_state` line for line.

### What stayed out

* **`force_chr_registers`** (the VRC2 CHR registers powering up to `$FF`) is mapper 116's alone:
  that page says "one version of *Somari the Adventurer* relies on this" and mapper 14's page says
  nothing of the kind, so the `cfg` on that method was **not** widened.
* **The MMC3A counter** is mapper 12's alone, for the same reason read the other way: only that
  page states it (Q-165).
* **Mapper 238** stays out, as MF-6b step 6 left it: its page is one sentence about *Contra
  Fighter* with no register, no mask, no mode and no submapper, and a board cannot be written from
  it. This lot does not change that, and `board-huang1` is the code a later lot would parameterise.

### Tests

| test | what it pins |
| --- | --- |
| `req_core_102_mapper_14_switches_between_two_modes_from_a131` | bit 1, the power-on mode, each chip's registers surviving a round trip through the other, and that **no value reaches an MMC1** |
| `req_core_102_mapper_14_drives_three_chr_a18_bits_over_unequal_window_groups` | one case per group — windows 0-3, 4-5 and 6-7 — all three at once, clearing them, and the lines holding in VRC2 mode as well |
| `q_165_mapper_14_supervisor_writes_are_seen_by_the_sub_board_at_the_same_address` | the overlap in MMC3 mode through `$A001` bit 7, and its absence in VRC2 mode through the mirror rule |
| `q_165_mapper_14_takes_its_mirroring_from_the_vrc2_alone_and_from_no_mirror` | `$9000` working, five of its mirrors not, `$A000` inert in MMC3 mode, and the arrangement surviving the switch |
| `q_165_mapper_14_decodes_the_supervisor_register_at_a131_exactly` | seven near misses, each of which is a sub-board register |
| `req_core_102_mapper_14_leaves_mapper_116_and_the_vrc_numbers_alone` | `$A131` is nothing on mapper 116, and a real VRC2b (mapper 23 submapper 3) still answers `$A003` and `$9003` |

`cargo test --release -p zax-nes`: all suites green, **319** in `mapper_tests`. `cargo fmt` clean;
clippy `-D warnings --all-targets` clean in `default` and in
`--no-default-features --features board-huang1`.

[PROGRESS] MF-UNL-8 step 2/8 done: mapper 14 is MF-6b step 6's table to the bit — bit 1 the mode and
bits 3, 5 and 7 three CHR A18 lines over unequal groups — with the supervisor at $A131 seen by the
sub-board in MMC3 mode and ignored in VRC2 mode, which is the page's own mirror rule doing the work.

---

## Step 3 — the Cony / Yoko ASIC: one chip, five PCBs, four of them built

`board-cony`, in `default`, implying nothing. One new module, `cartridges/mappers/cony.rs`; one new
boxed `IrqBoard` variant; three `cfg` lists extended by one feature each; and two numbers in each of
the three latched lists of `cartridges::mod`.

### One board and not five, because the page says so

> The distinction between mappers 83 and 264 as well as NES 2.0 Submappers denote different printed
> circuit boards that all incorporate the **same chip-on-board ASIC**.

So `ConyVariant` is four rows and four `const fn`, on the `Jv001` and `Namcot108` precedent, and the
difference between the PCBs is **four parameters**:

| parameter | what differs | which PCBs |
| --- | --- | ---: |
| `remaps_addresses` | "connects CPU A11.A10 to the ASIC's A9..A8 inputs, so all addresses change accordingly" | 264 |
| `banks_chr_in_2k` | CHR-ROM A11..A18 on the ASIC's A10..A17, so 2 KiB banks through registers 0, 1, 6 and 7 | 83.1, 264 |
| `inner_prg_mask` | "the former have a 256 KiB inner PRG bank, and the latter a 128 KiB" | `$1F` on 83.0/83.1, `$0F` on 83.3/264 |
| the PRG base layout | `PPPP PPPP` against `CCPP PPPP` | 83.3 is the odd one |

**The address remap is one expression**, and the page's own five-row table is what checks it:
`(address & 0xF0FF) | ((address & 0x0C00) >> 2)` reproduces `$5100`→`$5400`, `$8100`→`$8400`,
`$8200`→`$8800`, `$8300`→`$8C00` and `$8318`→`$8C18` exactly. The ASIC's own A11-A10 come out zero,
which every mask on the page leaves undecoded anyway. Its test is written the only way a rewiring
can be shown rather than asserted: on a mapper-264 image the **mapper-83** address is written and
found to be a *different* register — `$8100` is the PRG base there, and `$8318` is no register at
all, so the counter stays on M2 when `$FF` is written to it.

**`inner_prg_mask` is also what makes "the `p` bit (PRG A17) has no effect on mappers 83.3 and 264"
true without a second test**, which is the kind of thing that only falls out if the mask is the
thing being modelled rather than the sentence.

### The register map, and the three masks that differ in two address bits

The page gives the four registers above `$8300` three masks — `(Address AND $8310)=$8300`,
`(Address AND $8318)=$8310` and `(Address AND $8318)=$8318` — which differ in A4 and A3 and nothing
else. So the decode is `asic & 0x8300` into four arms and then two bit tests, and `$8308`-`$830F` is
the PRG register file again because A3 is not in **its** mask; a test pins that rather than leaving
it to a reader.

The PRG composition is the page's three modes, in 8 KiB bank units with A13 at bit 0:

* mode 0, UxROM: `base | low` for the two windows of `$8000`-`$BFFF` and
  `base | (mask & !1) | low` for the two of `$C000`-`$FFFF` — "PRG A17/A16..A14=1 if CPU A14=1";
* mode 1, NROM-128: `base | low` throughout, so CPU A14 reaches no PRG line and the 16 KiB bank
  appears twice;
* modes 2 and 3: `(base & !mask) | (register & mask)`, with `$E000`-`$FFFF` at
  `(base & !mask) | mask` — "hard-wired to the last (inner) bank".

### The IRQ, clause by clause

> When counting is enabled and **the counter is not zero**, the counter is incremented or
> decremented according to the Mode register's **D** bit. If the counter is then zero, **counting
> is disabled**, and an IRQ is raised.

All three clauses are in `Cony::tick_irq` and each one is a case in
`req_core_114_cony_counts_up_or_down_and_disables_itself_at_zero`: a counter at zero does not move
(so an armed board with a zero counter is silent for ever), the direction is the mode register's,
and the firing turns counting off — so after an acknowledge through the LSB the counter stays
silent until the MSB is written again. The arm is the page's "writing to the MSB enables counting
if bit `C` in the Mode register was **previously** set", which is one test on the mode byte at the
moment of the MSB write.

The clock source is `$8318`, "unfiltered PPU A12 rising edges" against "CPU M2 cycles", and both
hooks were already forwarded by `IrqBoard` for the J.Y. ASIC — the other board in that enum with
two sources. `last_ppu_address` is kept whatever the source is, the J.Y. ASIC's own reason, which is
why both numbers are in the two PPU watch lists whatever `$8318` holds.

### Submapper 2 is refused, and that is the lot's one scope decision

The N-C88-1603 4-in-1 of *Dragon Ball Party* is the one PCB of the five whose `$6000`-`$7FFF` is
32 KiB of **PRG-NVRAM** rather than PRG-ROM, with its bank in the PRG base register's top two bits
— "the circuit board hardware around the ASIC forces CPU $6000-$7FFF to PRG-RAM instead". That
window is `Mapper::prg_ram_window`, which `IrqBoard` **does not forward**, so the PCB would put a
new body in a `Board` dispatch for a multicart that carries no original title in the census's
roster. It is refused at load with REQ-CORE-011's message, on MF-UNL-6's Q-157 precedent, and
**Q-167** has the argument. 83.0, 83.1, 83.3 and 264 are all built; 83.3 costs two `const` rows.

### The window at $6000, and `prg_ram: Some(0)`

The page's *Banks* section gives `$6000`-`$7FFF` as "8 KiB switchable window into 32 KiB PRG-NVRAM
(Mapper 83.2)/**into PRG-ROM (all others)**", so on the four PCBs this build serves there is no RAM
behind that window at all: it is a PRG-ROM bank when `$8100` bit 5 enables one and **open bus** when
it does not. `board_memory_sizes` gives both numbers `prg_ram: Some(0)`, `Cony::prg_ram_enabled`
answers `false`, and `Cony::try_read_prg` answers the window — the `JyCompany` shape. The test shows
the difference between "nothing there" and "a zero the board drove" with `try_cpu_peek`.

### The solder pad, and a printed mask that cannot be right

`$5000` answers **0** and that is a recorded gap (Q-166): the page says "the 'good' value is not
always 0 or 3" and **no field of any header records how a pad was cut**, so §7.2's "returns the
configured value" has no configuration to read. A `const` and not a field, so no savestate byte
carries a figure nothing can set.

The scratch RAM's printed mask is "(Address AND $8310)=$8300", which is a copy of the PRG bank
register's line and cannot be right — `$5100 AND $8310` is `$0100`. The address pattern
`A~[0101 ..01 .... 00AA]` is used instead, i.e. `(asic & 0xF30C) == 0x5100`, and Q-166 records it.
The test walks that decode: `$5110` is `$5100` again (A4 undecoded) and `$5104` is not (A2 decoded).

### Tests

Eight in `mapper_tests` and three in `mapper_irq_tests`: the four PRG modes and the five-bit
register, 1 KiB against 2 KiB CHR with **registers 2-5 disconnected** on the two large-CHR PCBs,
the 264 rewiring shown four ways, the `$6000` window on and off, the four mirroring values, 83.3's
128 KiB inner bank and its outer CHR bank, the solder pad and the scratch RAM, submapper 2 refused
with the other four loading — and the three IRQ tests: the counter's three clauses, the source
selection in both directions with the unfiltered eight-rises-a-line case, and
`ppu_irq_horizon` `None` for every source on both numbers, which is the property census §7.2 asks
for.

`cargo test --release -p zax-nes`: all suites green, **327** in `mapper_tests` and **61** in
`mapper_irq_tests`. `cargo fmt --check` clean; clippy `-D warnings --all-targets` clean in
`default`, `--no-default-features`, `--features parked-boards` and
`--no-default-features --features board-cony`.

[PROGRESS] MF-UNL-8 step 3/8 done: the Cony / Yoko ASIC is one board with four parameters — the
264 rewiring as one expression, the 2 KiB CHR of 83.1 and 264 through registers 0, 1, 6 and 7, two
inner-bank widths and two base layouts — with submapper 2 refused for the PRG-NVRAM window it alone
needs.

---

## Step 4 — mapper 91: two boards whose difference is one mask bit

`board-91`, in `default`, implying nothing. One new module,
`cartridges/mappers/mapper091.rs`; one **unboxed** `IrqBoard` variant; the number in all three
latched lists of `cartridges::mod`.

### One `bool`, and it carries five differences

> Submapper 0 has a PA12-based IRQ counter and hard-wired mirroring; submapper 1 an M2-based IRQ
> counter and selectable mirroring.

Behind that sentence the page gives every register "Mask: `$F003`/`$F007` (submapper 0/1)", and
**that one bit is the whole submapper**: with the mask at `$03`, `$6004` and `$6005` fold back onto
CHR registers 0 and 1, `$6006`/`$6007` onto registers 2 and 3, and `$7006`/`$7007` land on indices
2 and 3 — which is exactly a board with hard-wired mirroring, no counter register and the IRQ ports
where submapper 0's page puts them. So `Mapper091::index_mask` is the board, and the registers
submapper 1 "adds" are the four the mask makes reachable. Its test writes each of those four
addresses on each board and shows the two decodes apart rather than asserting them.

### The outer bank register is an address and not a datum

```text
A~FEDC BA98 7654 3210
  -------------------
  .... .... .... .PPC
                  ||+- Select outer 512 KiB CHR-ROM bank (CHR A19)
                  ++-- Select outer 128 KiB PRG-ROM bank (PRG A17-A18)
```

The diagram header is **`A~`**: *which* address of `$8000`-`$9FFF` a game writes is the register and
the byte it writes is not read at all. The test shows that by writing the **same value** to four
addresses and getting four layouts, and then the same address with three values and getting one.

Two things it leaves open, both in **Q-168**:

* **Which of A2 and A1 is PRG A18.** Every other field diagram on nesdev is MSB-first and writes a
  multi-bit range *descending* ("PRG A21..A14", "CHR-ROM A19..A18"); this one writes "PRG A17-A18"
  **ascending** with a hyphen. This build takes the diagram's column order — A2 is A18, A1 is A17 —
  because the columns are the convention and the label is prose. The alternative swaps the two
  middle 128 KiB banks and nothing else.
* **"Mask: unknown"**, so the page's own range `$8000`-`$9FFF` is the decode and A15-A13 are the
  only bits tested above A2.

A19 is the 512 KiB boundary, which is why the test needs a **megabyte** of CHR to show the line at
all — the first draft used 512 KiB and the window wrapped back onto the first bank, which is the
kind of thing only a real assertion catches.

### Two counters that share nothing but their ports

* **Submapper 0**: "a binary counter clocked by PPU A12, and simply counts a fixed number of **64
  unfiltered rises** of PPU A12." Six stages whose carry out is the IRQ, so
  `(counter + 1) % 64 == 0`. **Unfiltered** is the word that keeps it away from `A12Watcher`, and
  the test drives sixty-three rises with two-dot dips between them — the pattern an MMC3's filter
  counts as one — before the sixty-fourth fires.
* **Submapper 1**: "a binary counter clocked by the M2 signal with a **factor of 5/4**, meaning
  counting down by five every fourth M2 cycle." A two-bit prescaler and a sixteen-bit counter
  `$6006`/`$6007` write **directly** — the page gives the board no latch beside it, "Set IRQ
  Counter LSB/MSB" — so `$7007` reloads nothing and only clears the prescaler.

  **The IRQ is the borrow out, and that is forced rather than chosen**: a step of five lands on
  exactly zero only when the counter is a multiple of five, so a board that fired at zero would
  mostly never fire. A counter of 20 therefore fires after twenty M2 cycles (15, 10, 5, 0, borrow)
  and `$0100` after 208. The first draft of the test assumed the fire-at-zero reading and said
  sixteen; the arithmetic is in Q-168 with the alternative named.

Each test also shows the *other* clock doing nothing — 20 000 M2 cycles on submapper 0 and 200 A12
rises on submapper 1 — which is the pair census §7.2 asks for on this board as it does on the Cony
one.

### What the page says and this follows, against the lot's other page

`$6004`/`$6005` are "Select **horizontal mirroring**" and "Select **vertical mirroring**", value
ignored. The page uses the *mirroring* vocabulary where mapper 83's page gives both namings
("0: Horizontal (Vertical mirroring)"), so this takes the word each page uses — which means the two
boards of one lot read the same-looking sentence two different ways, and Q-168 records that rather
than harmonising them behind the pages' backs.

### Where it sits, and the row

`IrqBoard::Mapper091(Mapper091)`, **unboxed** on `Mmc3Kof97`'s reason: the board is a
`BankWindows` and sixteen bytes where that enum already carries a whole `Mmc3Board` inline for the
TQROM. Both hooks the two submappers need are already forwarded there. The number is in
`clocks_on_cpu_cycle` (submapper 1's M2), in both PPU watch lists (submapper 0's A12) and in
`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`'s expected list **twice** — the
board keeps `last_ppu_address` on both submappers so that neither reads a stale edge.

`board_memory_sizes` gives the number `prg_ram: Some(0)`: the register file *is* the board's decode
of `$6000`-`$7FFF` and the mask leaves no address in those 8 KiB that is not a register, which is
mappers 114, 115, 182 and 193's arm in the same words.

### Tests

Four in `mapper_tests` — the two PRG windows and the four 2 KiB CHR windows on both boards, the
`$F003`/`$F007` decode shown both ways, the two mirroring ports with the value ignored and a
hard-wired board that both of them cannot move, and the address-driven outer register in six cases
— and three in `mapper_irq_tests`: each counter with its own clock and against the other's, and
`ppu_irq_horizon` `None` on both submappers.

`cargo test --release -p zax-nes`: all suites green, **331** in `mapper_tests` and **64** in
`mapper_irq_tests`. `cargo fmt --check` clean; clippy `-D warnings --all-targets` clean in six
configurations — `default`, `--no-default-features`, `--features parked-boards` and each of the
lot's three features alone.

[PROGRESS] MF-UNL-8 step 4/8 done: mapper 91's two boards are one board and one `bool`, because the
$F003/$F007 mask *is* the submapper; its outer register is an address and not a datum, and its two
counters are 64 unfiltered A12 rises against a borrow out of five-every-four M2 cycles.

---

## Step 5 — five question tickets

`G:/3dsen/cleanroom/spec/changes/Q-164.md` to `Q-168.md`, one per board plus one for the scope
decision. Nothing else under `spec/` is touched.

| ticket | what it records |
| --- | --- |
| **Q-164** | **§7.2 says mapper 12 is "`Somari` with one more register" and the number's own page refutes it** — "The Huang-1 ASIC functions exclusively as an MMC3A clone on this PCB" — plus `$4132`'s undocumented power-on, which value of the language bit is Chinese, the FCEUX latch the page names as *not* the hardware's with a homebrew hack depending on it, and submapper 1 refused as copier hardware |
| **Q-165** | mapper 14: the bit positions where the page's **diagram** and a prose rendering of the same page disagree by one place throughout; "Mask: `$FFFF?`" with the page's own question mark; no power-on value; which chip latches a supervisor write that changes the mode; and **two rules the page says "have not been verified on original hardware"** — with the finding that the second of them is what keeps `$A131` off the VRC2's own PRG register, a consequence that is on neither page |
| **Q-166** | the Cony page: a **printed register mask that cannot be right** (`$5100 AND $8310` is `$0100`, and the mask is character for character the preceding register's); a solder pad with **no configuration source in any header**, against §7.2's "returns the configured value"; "it's not known which of the eight data bits is decisive"; what `$6000`-`$7FFF` shows in PRG modes 0 and 1; and why `ppu_irq_horizon` answers the census's conditional with a stronger statement |
| **Q-167** | **mapper 83 submapper 2 refused**, and that §7.2's account of the board never mentions there are five PCBs. The boundary is a *mechanism* boundary: that PCB alone needs `Mapper::prg_ram_window` forwarded through `IrqBoard`, an unmeasured `Board` dispatch body, for a 4-in-1 multicart with no original title — where 83.3 is also a multicart and is **implemented**, because it costs two `const` rows. The ticket ends with what reopening it would take |
| **Q-168** | mapper 91: the outer register's bit order, where the page's **label writes the pair ascending** while every other nesdev field diagram is MSB-first; "Mask: unknown"; **neither counter has a stated firing rule**, with the arithmetic that makes submapper 1's borrow-out reading nearly forced; "hard-wired to last bank" — of what; **the two pages of this lot do not share a vocabulary for mirroring**, so the two boards read the same-looking sentence two different ways and each follows its own page; and that §7.2 under-counts the board at one counter and four register groups where the page has two counters, five groups and an outer register it does not mention |

Three of the five are the MF-UNL-2 case the lot instruction names — "§7.2's steps were drafted from
Mesen's `MapperFactory` and three of them contradicted the numbers' own pages: **the number's own
page wins**" — and the one that bites hardest is Q-164, because it is the difference between a
three-chip board and a one-chip board. Q-167 is a scope decision rather than a contradiction, and it
is in a ticket because the lot's rule is that nothing is left out silently. Q-166's item 1 and
Q-168's item 5 are the two places a page is wrong or inconsistent rather than merely silent.

[PROGRESS] MF-UNL-8 step 5/8 done: five tickets — mapper 12 is not mapper 116's board, mapper 14's
two rules are unverified by its own page's admission, the Cony page prints a mask that cannot be
right and a pad nothing can configure, submapper 2 is refused on a mechanism and not on content, and
mapper 91's two counters have no stated firing rule at all.

---

## Step 6 — §7.1's first two common things, and one thing the round trip found

### 1. The four sweeps over `SUPPORTED_MAPPERS`

`req_core_100_every_required_mapper_is_registered` (whose **built** expected list gains all five
numbers, so an accidental addition is caught as surely as an accidental removal),
`req_core_102_any_other_mapper_is_refused`,
`amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` and
`req_core_090_the_mapper_state_survives_a_save_and_restore` all walk the table, so every number
this lot adds is built, reset and round-tripped by them. The three in-crate latch sweeps
(`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`,
`mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched`, and MF-4c's) build each number at
submappers 0 and 3 besides — and `mf_3b`'s expected list had to be extended three times over this
lot, once per board, which is exactly what §4.3 of the inventory keeps it for.

### 2. A save-state round trip per board, with the registers dirtied first

Needed because the generic sweep writes `$8000`-`$E001` and `$6000`, and **not one of this lot's
four boards keeps its own registers only there**: mapper 12's GAL register is at `$4132`, mapper
14's supervisor at `$A131` with a second chip's whole block behind it, the Cony board's file is at
`$8300`-`$831F` and `$5100`-`$5103`, and mapper 91's is at `$6000`-`$7001` with an outer register
that is an *address* in `$8000`-`$9FFF`.

| test | what it moves before the capture |
| --- | --- |
| `req_core_090_mapper_12_round_trips_its_outer_chr_register` | `$4132` with both CHR A18 lines, two MMC3 bank registers and the counter, two frames run |
| `req_core_090_mapper_14_round_trips_both_chips_and_the_supervisor` | the MMC3's pair, then the VRC2's two PRG registers and its mirroring in the other mode, then back with two of the three lines set — and it walks away **through** the other mode and changes its registers too, so nothing the capture held is still live. It also checks that mapper 12's block is **shorter** than mapper 14's |
| `req_core_090_cony_round_trips_its_register_file_and_its_scratch_ram` | on both numbers: the mode register, the base, four PRG registers, four CHR registers, the clock source, the counter and all four scratch bytes — and the `$6000` window's content, read back through `try_cpu_peek` |
| `req_core_090_mapper_91_round_trips_both_of_its_boards` | on both submappers: four CHR registers, two PRG registers, the outer register or the mirroring port, the counter register, and the counter itself clocked ten times on its own clock |
| `amd_061_a_reset_puts_every_board_of_mf_unl_8_back_to_its_power_on_state` | all five numbers, each through a register the generic reset sweep does not write, with mapper 14's non-zero power-on (`$02`, MMC3 mode) among them |

Each one asserts that the dirtying **moved the layout** before the capture is trusted — without
that line a round-trip test passes on a board whose register never reaches the state at all,
because the live field would still hold what the capture held.

### The thing that found: `IrqBoard` does not forward `save_state`, so mapper 116's override is unreachable

`req_core_090_mapper_14_round_trips_both_chips_and_the_supervisor` failed on its first run with the
VRC2's registers back at their **power-on** values, and the reason is not mapper 14's:

* `Cartridge::save_state` calls `Board::save_state`.
* **`Board` does not override `save_state` or `load_state`** — it forwards `save_registers` and
  `load_registers` — so `Mapper::save_state`'s default runs: `banks()`, then `save_registers()`.
* **`IrqBoard` does not override them either**, for the same reason and in the same two methods.

So a board inside either shared variant is reached **only** through
`save_registers`/`load_registers`, and an override of `save_state` on such a board is dead code.
`Huang1` was written with that override, on `Somari`'s model, and silently wrote nothing but the
active chip's window table.

**`Somari` has the same override and the same problem**, and it is mapper 116's: its own note
explains at length why `save_state` is overridden "rather than `save_registers`/`load_registers`",
and that override has never been called. What a mapper-116 savestate carries today is
`Board::banks()` — *whichever chip the current mode names* — and nothing else: no supervisor
register, no second or third chip, no IRQ state. `req_core_090_the_mapper_state_survives_a_save_and_restore`
does not catch it because the sweep compares the re-saved **bytes**, and a block that holds only a
window table is self-consistent after a restore.

**This lot does not fix it**, and that is the owner's rule rather than a preference: "no change to
the emulation of an existing mapper, and no change to the save-state format of existing boards".
Moving mapper 116's block is exactly what a fix is. It is recorded here, in `Huang1`'s own
`save_registers` note, and in the census note of step 8, with what the fix would be — move
`Somari::save_state`'s body into `save_registers` and its `load_state`'s into `load_registers`,
which is one mechanical change and a new savestate version for that one board.

`Huang1` now writes its state through `save_registers`/`load_registers`. The default's leading
`banks()` block stays in front of it and is **redundant but harmless**: `load_registers` ends by
recomputing both chips' windows from their own registers and laying the CHR A18 lines back over the
active one, so whatever the default wrote into whichever chip the pre-load mode named is overwritten
before the state is used. `Cony` and `Mapper091` were written through `save_registers` from the
start and were never affected.

### 3. The configurations

`cargo fmt --check` clean. `cargo clippy -p zax-nes --all-targets -- -D warnings` clean **and the
whole `zax-nes` suite green** in six configurations — the three the lot adds, each alone with no
defaults, plus `default`, `--no-default-features` and `--features parked-boards`:

| configuration | tests passed |
| --- | ---: |
| `default` | **1 041** |
| `--no-default-features` | 783 |
| `--features parked-boards` | **1 094** |
| `--no-default-features --features board-huang1` | 796 |
| `--no-default-features --features board-cony` | 795 |
| `--no-default-features --features board-91` | 791 |

**0 failed in all six.** `cargo test --release -p zax-nes -p zax-harness` is **1 104 passed, 0
failed**, and `cargo clippy -p zax-harness --all-targets -- -D warnings` is clean — the two crates
this lot touches, which is the scope `zax-app`'s own note allows in this worktree.

The whole suite and not `mapper_tests` alone, which is MF-UNL-7 step 7's lesson: that lot's
`req_arch_002_the_crate_has_no_platform_dependency` reads `zax-nes/Cargo.toml` as **text** and
refuses the substring `windows`, and three feature comments tripped it. This lot's three new
comments were checked against it in every configuration; they say "regions", "ranges" and "window
groups", and the test passes.

[PROGRESS] MF-UNL-8 step 6/8 done: the four sweeps and the three latch sweeps reach all five
numbers, five dedicated round trips move what the generic sweep cannot — and the first of them found
that `Board` and `IrqBoard` forward `save_registers` and not `save_state`, so mapper 116's own
override has never been called; `Huang1` is fixed and mapper 116's block is left exactly as it is.
