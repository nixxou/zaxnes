# Lot PR-1 — the split `read_internal`, measured on more than two test ROMs

Baseline `2754a7e8` (MF-4c's end). Q-119 point 3 and R0's decision 3.

A performance lot. It changes no behaviour at all: only attributes and wrappers on the CPU's
bus read path.

## Steps

1. The instrument: the ROM set, a committed script, and the baseline on `2754a7e8`.
2. The split form of Q-119 point 3, its generated code, and what it measures on the set.
3. The variants that try to keep the gain without the cost.
4. The decision, and `PERFORMANCE-HISTORY.md` §7.
5. `cargo fmt`, clippy, `cargo test --release -p zax-nes -p zax-harness`.
6. The whole test-ROM campaign, ROM by ROM against MF-4c's.

## Step 1 — the instrument

### What the lot has to be able to see

Q-119 point 3 says the split form measures **−0.86 %** on `M1_P128K` and **−0.79 %** on
`M0_P32K_C8K_V`, that only 8.8 M of the 43.5 M saved is the `jmp` the split removes, and that
the rest is LLVM moving `Ppu2C02::read` and a `Board` read **off** the CPU read path — so every
`$2002` or `$2007` read then pays a `callq` and a prologue. The two ROMs that show the gain
cannot price that, because they barely read PPU registers.

So the set has to span one axis the earlier lots never varied: **how often a CPU read lands on a
PPU register**. The baseline table below confirms the premise before anything is built —
`Ppu2C02::read` has **no exclusive count on any of the thirteen ROMs**, i.e. it is inlined into
`NesBus::read_internal` everywhere, and so are `Cartridge::try_cpu_read` and
`Board::cpu_read_override`.

### How the ROMs were chosen, and not guessed

The register-read rate is a property of a ROM, not of this lot, so it was **measured** first.
A throwaway counter (a `static [AtomicU64; 8]` incremented at the top of `Ppu2C02::read`, printed
by `bench-core`) was built, read once over every `.nes` under `assets-free/test-roms` and over the
commercial folder, and then reverted — it is not in any commit. Over the same 300 frames the
callgrind runs use, reads of `$2000`-`$2007` by register:

| ROM | mapper | `$2002` | `$2004` | `$2007` | per frame, all registers |
| --- | ---: | ---: | ---: | ---: | ---: |
| `M0_P32K_C8K_V` | 0 | 6 254 | 0 | 464 | **22** |
| `M1_P128K` | 1 | 6 254 | 0 | 65 654 | **240** |
| `M4_P128K` | 4 | 6 254 | 0 | 65 878 | **240** |
| `mmc5exram` | 5 | 87 947 | 0 | 0 | **293** |
| `Duelito` | 0 | 1 207 258 | 0 | 0 | **4 024** |
| `ppu_vbl_nmi` | 1 | 304 710 | 0 | 0 | **1 016** |
| `test_ppu_read_buffer` | 3 | 1 019 629 | 65 | 24 401 | **3 482** |
| `oam_stress` | 0 | 14 650 | 432 430 | 0 | **1 490** |
| Mega Man 2 | 1 | 9 079 | 0 | 0 | **30** |
| Zelda II (USA) | 1 | 556 752 | 0 | 580 | **1 858** |
| Super Mario Bros. 3 | 4 | 17 327 | 0 | 288 | **59** |
| Super Mario Bros. | 0 | 69 528 | 0 | 315 | **233** |
| Metroid | 1 | 8 936 | 0 | 0 | **30** |

A frame is about 29 780 CPU cycles, of which roughly two thirds are reads, so **`Duelito` spends
about a fifth of its read cycles on `$2002`** — two orders of magnitude more than Mega Man 2, and
that is the span the lot needs. The sweep's other finding is that the §7 premise is only half
right: `M0_P32K_C8K_V` really does barely touch the registers (22 a frame), but `M1_P128K` reads
`$2007` 219 times a frame, and `M1_P128K`/`M4_P128K` are in fact the **heaviest `$2007` readers
available here** — nothing in the test-ROM corpus or the commercial folder reads `$2007` more
often per frame.

The thirteen, and why each is in `rust/tools/perf/roms.txt`:

| ROM | why |
| --- | --- |
| `M0_P32K_C8K_V`, `M1_P128K` | §7's two ROMs: the lot has to reproduce the −0.86 %/−0.79 % before it can argue about it |
| `M4_P128K` | an MMC3, and (with `M1_P128K`) the heaviest `$2007` reader of the corpus |
| `mmc5exram` | the MMC5, whose arm of `read_internal` is the largest (MF-4c: 786 instructions) and whose `Cartridge::default_cpu_read` is 283 M on its own |
| `Duelito` | the extreme `$2002` spinner: 4 024 a frame, ~20 % of all read cycles. Its loop runs steadily for about 310 frames and then stops, so 300 frames is in the steady phase |
| `ppu_vbl_nmi` | a `$2002` poller (1 016 a frame) on a **bank-switched** board, so the `Board` read and the register read are both live |
| `test_ppu_read_buffer` | both channels at once: 3 399 `$2002` **and** 81 `$2007` a frame |
| `oam_stress` | 1 441 `$2004` a frame — a third register arm, with no `$2007` and almost no `$2002` |
| Mega Man 2, Zelda II, SMB3, SMB1, Metroid | **R0's five**, the bar the lot is judged against. Zelda II turns out to be a commercial `$2002` spinner at 1 858 a frame, which is the "game that waits for V-blank by looping on `$2002`" Q-119 asked for and could not find |

Castlevania III is deliberately **not** in the set: MF-4c measured the MMC5's +3 % on it, and that
cost has nothing to do with this lot's read path. `mmc5exram` covers the MMC5 arm more cheaply.

### The script

`rust/tools/perf/`, four files, all committed:

| file | what it does |
| --- | --- |
| `make-tree.sh <ref\|--worktree> <out.tar>` | packs a clean tree with `git archive`, or the working tree with `git ls-files` for a variant not yet committed |
| `build-clean.sh <label> <tree.tar>` | extracts into a **brand-new** directory with its own `CARGO_TARGET_DIR`, builds with `CARGO_PROFILE_RELEASE_STRIP=false`, copies the binary out and **deletes the 4 GB build directory** |
| `callgrind-set.sh <label> [jobs]` | `valgrind --tool=callgrind --dump-instr=no --cache-sim=no --branch-sim=no` over `roms.txt`, 300 frames, no warm-up; writes `<key>.ann` and `<key>.ir` per ROM |
| `callgrind-diff.py <dir-a> <dir-b>` | the per-ROM table, the symbols that moved, and the verdict line this lot is judged on |
| `callgrind-symbols.py <dir>` | one run's exclusive counts for the symbols of the CPU read path |

Three things in them are not obvious and are the reason they exist as files rather than as a
paragraph of this document:

* **The tree and the build are separate scripts.** In a git *worktree* the `.git` entry is a file
  holding an absolute path to the main repository, so this worktree is not a git repository as the
  WSL2 Ubuntu sees it (`fatal: not a git repository: .../G:/...`). Git has to run on the host and
  only the tar crosses over.
* **CRLF normalisation.** The host checks files out with CRLF, so a tar of the *working tree*
  carries them and a `git archive` tar does not. Without the `sed` in `build-clean.sh`, a
  committed baseline and an uncommitted variant differ in every line of every file.
* **The runs go in parallel** (`jobs`, default 8). Callgrind is a simulator, so a count does not
  depend on machine load; thirteen ROMs take **43 s** of wall clock instead of nine minutes, which
  is what makes measuring every variant on the whole set affordable.

`callgrind-diff.py` takes the whole-process figure from the annotation's own `PROGRAM TOTALS` and
cross-checks it against valgrind's `Collected :` line, printing a warning if the two disagree —
the two files being mispaired is a real mistake, found while testing the parser on MF-4c's
leftovers. It also reports the part of the total the per-symbol list does **not** account for, so
a change hiding below `callgrind_annotate`'s fold cannot pass as "no symbol moved".

### The baseline, `2754a7e8`

`PR1_WORK=$HOME/pr1`, 300 frames, no warm-up, whole process:

| ROM | board | instructions |
| --- | --- | ---: |
| `vblnmi` | MMC1 | 3 476 935 328 |
| `oamstr` | NROM | 3 487 355 725 |
| `m1` | MMC1 | 5 063 537 884 |
| `metroid` | MMC1 | 5 304 394 605 |
| `m4` | MMC3 | 5 649 667 156 |
| `m0` | NROM | 5 694 534 646 |
| `smb1` | NROM | 6 181 802 642 |
| `mm2` | MMC1 | 6 188 849 351 |
| `z2` | MMC1 | 6 457 641 956 |
| `readbuf` | CNROM | 6 994 854 555 |
| `duelito` | NROM | 7 540 980 081 |
| `smb3` | MMC3 | 8 117 976 283 |
| `mmc5` | MMC5 | 11 486 356 993 |
| **total** | | **81 644 887 205** |

The procedure reproduces the earlier lots to the instruction where they overlap, which is the
check that the instrument is the same one:

| ROM | MF-4c step 4 | here, at `2754a7e8` | difference |
| --- | ---: | ---: | ---: |
| `m0` | 5 694 536 051 | 5 694 534 646 | −1 405 |
| `mm2` | 6 188 861 046 | 6 188 849 351 | −11 695 |
| `smb3` | 8 118 039 834 | 8 117 976 283 | −63 551 |
| `z2` | 6 457 595 787 | 6 457 641 956 | +46 169 |

all under 0.001 %, and they are not expected to be zero: MF-4c's table is its *step 4*, and
`2754a7e8` is two commits later. `NesBus::read_cycle` on `m1` is **26 344 580**, the same number
MF-4a-bis recorded for it.

And the baseline read path per symbol, exclusive (`callgrind-symbols.py`), on four of the thirteen:

| symbol | `m0` | `m1` | `duelito` | `mmc5` |
| --- | ---: | ---: | ---: | ---: |
| `NesBus::read_internal` | 246 329 972 | 253 890 369 | 340 769 903 | 607 291 787 |
| `NesBus::read_cycle` | 26 564 761 | 26 344 580 | 26 772 237 | 25 602 335 |
| `NesBus::halted_read_cycle` | 363 | 33 | 0 | 9 735 |
| `NesBus::tick` | 299 752 274 | 300 029 379 | 302 012 340 | 384 168 880 |
| `Ppu2C02::read` | *inlined* | *inlined* | *inlined* | *inlined* |
| `Cartridge::try_cpu_read` | *inlined* | *inlined* | *inlined* | *inlined* |
| `Board::cpu_read_override` | *inlined* | *inlined* | *inlined* | *inlined* |
| `Cartridge::default_cpu_read` | 4 719 024 | 4 981 306 | 0 | 283 356 728 |

`read_cycle` is the three-instruction gate — `cmpb`/`jne`/`jmp` — on about 8.8 M read cycles, and
it is the same ~26 M on every ROM of the set, which is what Q-119's "only 8.8 M of the 43.5 M is
the `jmp`" refers to: a third of that 26 M.

## Step 2 — the split form, its generated code, and what it measures

### The change

`nes_bus.rs`, exactly the shape Q-119 point 3 describes — an `#[inline(always)]` body and two
out-of-line callers:

```rust
#[allow(clippy::inline_always)]
#[inline(always)]
fn read_internal_body(&mut self, address: u16) -> u8 { /* the whole bus read */ }

#[inline(never)]                                // caller 1: every reader that is not a CPU cycle
fn read_internal(&mut self, address: u16) -> u8 { self.read_internal_body(address) }

#[inline(never)]                                // caller 2: the CPU read cycle, with the gate
fn read_cycle(&mut self, address: u16) -> u8 {
    if self.dma == 0 { self.read_internal_body(address) } else { self.halted_read_cycle(address) }
}
```

### Post-LTO, against the baseline

`cargo rustc --release -p zax-harness --bin zax-harness -- --emit=asm` at `2754a7e8`
(`bin-pr1-base.s`) and with the split (`bin-pr1-split.s`), compared with MF-4c's `cmp4c.py`.
**Every pinned symbol of the CPU path outside the read itself is IDENTICAL, raw** —
`Cpu6502::step` 897, `branch` 366, `resolve` 1 725, `NesBus::tick` 75, `write_internal` 2 564,
`catch_up_ppu` 219, `run_dma` 511, `Apu2A03::run_cpu_cycles` 873, `quiet_cycles` 90,
`Ppu2C02::run_dots` 1 936, `background_fetch` 199, `read_nametable_inner` 132,
`read_pattern_planes` 106, `Board::ppu_read` 312, `ppu_peek` 235, `on_ppu_address_bus` 156,
`on_cpu_cycle` 363. What moved:

| symbol | baseline | split | |
| --- | ---: | ---: | --- |
| `NesBus::read_cycle` | **3** | **288** | the gate (`cmpb`/`jne`/`jmp`) becomes the gate plus its own copy of the read |
| `NesBus::read_internal` | **786** | **312** | the other callers' copy, now a tail of what is left |
| `NesBus::dma_read` | 109 | 76 | its inlined read became the call `read_internal` already was |
| `Ppu2C02::read` | *inlined* | **257, 2 call sites** | **left the read path** |
| `Board::cpu_read_override` | *inlined* | **231, 2 call sites** | **left the read path** |
| `WideExtension::read` | *inlined* | **50, 2 call sites** | **left the read path** |

So the answer to the question Q-119 asked is **yes, confirmed in the generated code**: three hot
callees each gain their first call site, which is MF-3b rule 1's stop-the-lot signature. And they
leave by a **tail call**, not a `callq` — which is what makes the price much lower than Q-119
feared:

```
    movq    %rsi, %rcx            ; the two arguments
    movl    %edi, %edx
    addq    $32, %rsp             ; the caller's epilogue, brought forward
    popq    %rbx
    popq    %rdi
    popq    %rsi
    jmp     <Ppu2C02>::read       ; and the callee's own prologue:
                                  ;   pushq %r14 / %rsi / %rdi / %rbx ; subq $40, %rsp
```

Seven instructions at the site and five in the prologue, against the one shared epilogue
(`addq` + five `popq` + `retq`) the baseline paid once for the whole of `read_internal`:
**about +11 encoded** at the boundary, and no `retq` of its own because the callee's `retq`
returns straight to the CPU.

### What it measures on the whole set

300 frames, no warm-up, whole process:

| ROM | board | reg. reads | `2754a7e8` | split | delta | |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `oamstr` | NROM | 447 080 | 3 487 355 725 | 3 437 653 319 | −49 702 406 | −1.425 % |
| `vblnmi` | MMC1 | 304 710 | 3 476 935 328 | 3 426 125 404 | −50 809 924 | −1.461 % |
| `m1` | MMC1 | 71 908 | 5 063 537 884 | 5 014 060 377 | −49 477 507 | −0.977 % |
| `metroid` | MMC1 | 8 936 | 5 304 394 605 | 5 252 724 997 | −51 669 608 | −0.974 % |
| `m4` | MMC3 | 72 132 | 5 649 667 156 | 5 600 585 290 | −49 081 866 | −0.869 % |
| `m0` | NROM | 6 718 | 5 694 534 646 | 5 643 411 966 | −51 122 680 | −0.898 % |
| `smb1` | NROM | 69 843 | 6 181 802 642 | 6 130 401 472 | −51 401 170 | −0.831 % |
| `mm2` | MMC1 | 9 079 | 6 188 849 351 | 6 136 338 583 | −52 510 768 | −0.848 % |
| `z2` | MMC1 | 557 332 | 6 457 641 956 | 6 407 695 136 | −49 946 820 | −0.773 % |
| `readbuf` | CNROM | 1 044 160 | 6 994 854 555 | 6 947 866 869 | −46 987 686 | −0.672 % |
| `duelito` | NROM | 1 207 258 | 7 540 980 081 | 7 493 512 635 | −47 467 446 | −0.629 % |
| `smb3` | MMC3 | 17 615 | 8 117 976 283 | 8 066 046 557 | −51 929 726 | −0.640 % |
| `mmc5` | **MMC5** | 87 947 | 11 486 356 993 | 11 553 670 347 | **+67 313 354** | **+0.586 %** |
| **total** | | | 81 644 887 205 | 81 110 092 952 | −534 794 253 | −0.655 % |

§7's two figures are reproduced: `m1` −0.977 % against its −0.86 %, `m0` −0.898 % against its
−0.79 % (the lots differ by MF-4b and MF-4c, which both changed `read_internal`).

### The `$2002` spinner does not pay for it, and here is why

**The case Q-119 could not judge is now judged, and it comes out the other way.** `Duelito`
spends about a fifth of its read cycles on `$2002` — 4 024 register reads a frame against Mega
Man 2's 30, a factor of 130 — and it still **gains 0.63 %**. So does `readbuf` (3 482 a frame,
−0.672 %), so does Zelda II (1 858 a frame, −0.773 %), so does `oam_stress` on `$2004`
(−1.425 %).

The reason is arithmetic, and it is worth writing down because it settles the question for the
hardware rather than for this set. Fitting the twelve non-MMC5 deltas to *(read cycles, register
reads)* by least squares — read cycles being `read_cycle`'s baseline count divided by its three
instructions:

| | instructions |
| --- | ---: |
| saved, per CPU **read cycle** | **−5.90** |
| paid, per **PPU register read** | **+4.09** |

with every residual under 2.5 M, i.e. under 0.05 % of a run. **A register read is itself a read
cycle.** It therefore saves 5.90 and pays 4.09, for a net **−1.81**, and no mix of addresses can
make the split lose on this axis: the break-even would need 1.44 register reads per read cycle,
which is more than one. Q-119's worry was that the gain and the cost sit on different
denominators; they sit on the same one.

### Where the MMC5 loses, and it is not the register read

The one ROM that is slower loses it all on a different callee. Per symbol on `mmc5`:

| symbol | baseline | split | delta |
| --- | ---: | ---: | ---: |
| `NesBus::read_internal` | 607 291 787 | 1 090 984 | −606 200 803 |
| `NesBus::read_cycle` | 25 602 335 | 383 374 809 | +357 772 474 |
| `Board::cpu_read_override` | *inlined* | **308 101 626** | **+308 101 626** |
| `Ppu2C02::read` | *inlined* | 5 364 857 | +5 364 857 |
| `WideExtension::read` | *inlined* | 2 357 220 | +2 357 220 |
| `Cartridge::default_cpu_read` | 283 356 728 | 283 356 728 | **0** |
| everything else (`tick`, `step`, `write_internal`, `catch_up_ppu`, `run_dots`, …) | | | **0** |

The MMC5's register reads are only 88 000, worth about 0.4 M at +4.09 each. The cost is
`Board::cpu_read_override`: the MMC5 maps PRG-**RAM** into `$8000`-`$DFFF`, so
`Cartridge::read_rom_window` cannot answer and **every** CPU read reaches the override — about one
call per read cycle, where an MMC1 reaches it on a few thousand reads out of 8.8 M — and the
boundary there costs about 13 instructions, which swamps the −5.90 the split saves.
`default_cpu_read` sitting at exactly its baseline count on both sides is the check that no work
appeared: this is a boundary, not a change of behaviour.

So the split as Q-119 wrote it **cannot be committed**: the bar is "no slower, to the instruction,
on every ROM of the set", and `mmc5` is +0.586 %. Step 3 tries to keep the −0.655 % without it.

## Step 3 — nine variants, and the one that is faster on every ROM of the set

### What was tried, and what each measured

Every row is a full `build-clean.sh` + `callgrind-set.sh` over all thirteen ROMs. "Slower on"
lists every ROM whose instruction count went **up** against `2754a7e8`, which is the only thing
that can disqualify a variant.

| | variant | total | slower on |
| --- | --- | ---: | --- |
| **A** | the split alone, Q-119 point 3 as written (step 2) | −0.655 % | `mmc5` **+0.586 %** |
| **B** | A + `#[inline(always)]` on `Board::cpu_read_override` | −0.767 % | **none** |
| **C** | B + `#[inline(always)]` on `Ppu2C02::read` **and** `WideExtension::read` | −0.084 % | `mmc5` +0.159 % |
| **D** | A + `#[inline(always)]` on `Ppu2C02::read` alone | **+0.024 %** | `mmc5` +0.890 % |
| **E** | `#[cold]` on the rare arms, **no split at all** | −0.068 % | eleven of thirteen (+0.000 % to +0.025 %) |
| **F** | A + the `#[cold]` rare arms | −0.730 % | `mmc5` +0.018 % |
| **G** | F + `#[inline(always)]` on `Board::cpu_read_override` | **−0.841 %** | **none** ← **kept** |
| **H** | G with plain `#[inline(never)]` instead of `#[cold]` | −0.842 % | none |
| **I** | G + `#[inline(always)]` on `Cartridge::try_cpu_read` | −0.802 % | none |

The brief's two suggestions are **D** and **E**, and the measurement says both are wrong, which is
worth recording because each sounds right:

* **D, "keep the PPU-register arm inline and let only the rare arms go out of line."** It is the
  worst variant of the nine: +0.024 % overall and **+0.890 %** on the MMC5. The reason is the one
  MF-3b rule 3 states in another form — inlining `Ppu2C02::read` puts its prologue (four pushes
  and a 40-byte frame) inside the body, so it is paid on **every read cycle** instead of on the
  register reads alone. Per read cycle on `m1`: 25.2 instructions in **B**, 30.6 in **C**. The
  +5.4 exactly cancels the −5.9 the split saves, and the register arm's +4.09 is the smaller
  number by a factor of six even on `Duelito`.
* **E, "`#[cold]` on the rare arms rather than on the split."** On its own it is a wash: −0.518 %
  on the MMC5, where shrinking `read_internal` lets LLVM keep the board's override inline, and
  **+0.000 % to +0.025 % on the other twelve** — slower, so disqualified, though by amounts
  (5 688 instructions on `vblnmi`) that are near the instrument's floor.

What the table does say is that the two ideas are **complementary rather than alternative**: the
cold arms are what make the split's two copies cheap (A → F, mmc5 +0.586 % → +0.018 %), and the
override's attribute is what closes the last of it (F → G, +0.018 % → −0.732 %).

### The variant kept, **G**

Three changes, all of them attributes or wrappers; nothing computes anything different.

1. `nes_bus.rs` — `read_internal` split into `read_internal_body` (`#[inline(always)]`) plus two
   out-of-line callers, `read_internal` (`#[inline(never)]`) and `read_cycle`, which is Q-119's
   form.
2. `nes_bus.rs` — the two rare arms of the body moved into `#[cold] #[inline(never)]` helpers:
   `read_io_register` (`$4000`-`$401F`) and `read_wide_window` (the widescreen window at
   `$5000`-`$5FFF` and its option over `$6000`-`$7FFF`). The caller's **single** `address < 0x8000`
   test stands for the two ranges the second one had, the address being already past `$401F`
   there — so each copy of the body loses a range test as well as the arm.
3. `cartridges/board.rs` — `#[inline(always)]` on `Mapper::cpu_read_override` for `Board`.

| ROM | board | `2754a7e8` | **G** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `oamstr` | NROM | 3 487 355 725 | 3 437 678 323 | −49 677 402 | −1.425 % |
| `vblnmi` | MMC1 | 3 476 935 328 | 3 426 078 147 | −50 857 181 | −1.463 % |
| `m1` | MMC1 | 5 063 537 884 | 5 013 867 622 | −49 670 262 | −0.981 % |
| `metroid` | MMC1 | 5 304 394 605 | 5 252 749 510 | −51 645 095 | −0.974 % |
| `m4` | MMC3 | 5 649 667 156 | 5 600 394 107 | −49 273 049 | −0.872 % |
| `m0` | NROM | 5 694 534 646 | 5 643 170 346 | −51 364 300 | −0.902 % |
| `smb1` | NROM | 6 181 802 642 | 6 130 416 678 | −51 385 964 | −0.831 % |
| `mm2` | MMC1 | 6 188 849 351 | 6 136 343 372 | −52 505 979 | −0.848 % |
| `z2` | MMC1 | 6 457 641 956 | 6 407 739 231 | −49 902 725 | −0.773 % |
| `readbuf` | CNROM | 6 994 854 555 | 6 947 841 972 | −47 012 583 | −0.672 % |
| `duelito` | NROM | 7 540 980 081 | 7 493 537 856 | −47 442 225 | −0.629 % |
| `smb3` | MMC3 | 8 117 976 283 | 8 066 048 574 | −51 927 709 | −0.640 % |
| `mmc5` | MMC5 | 11 486 356 993 | 11 402 193 161 | −84 163 832 | **−0.733 %** |
| **total** | | 81 644 887 205 | 80 958 058 899 | **−686 828 306** | **−0.841 %** |

**Thirteen ROMs, thirteen negative.** The MMC5 goes from the one loser of the split (+0.586 %) to
the **largest** gain of the set (−0.733 %).

### Post-LTO, as MF-3b's rule requires

`cmp4c.py`, `bin-pr1-base.s` against `bin-pr1-v7.s`. **Every pinned symbol outside the read is
IDENTICAL, raw** — the same seventeen listed in step 2, `Cpu6502::step` 897, `tick` 75,
`write_internal` 2 564, `Board::ppu_read` 312, `on_cpu_cycle` 363 and the rest. What moved:

| symbol | baseline | **G** | |
| --- | ---: | ---: | --- |
| `NesBus::read_cycle` | 3 | **182** | the gate plus its copy of the read; 288 in the split, the cold arms taking 106 out |
| `NesBus::read_internal` | 786 | **176** | the other callers' copy |
| `NesBus::read_io_register` | *inlined* | **71, out of line** | deliberately |
| `NesBus::read_wide_window` | *inlined* | **165, out of line** | deliberately |
| `Board::cpu_read_override` | *inlined* | *inlined* | **the attribute's whole job** |
| `WideExtension::read` | *inlined* | *inlined* | it is reached only from the cold arm now |
| `Ppu2C02::read` | *inlined* | **257, 2 call sites** | priced: +4.09 a register read |
| `Cartridge::try_cpu_read` | *inlined* | **244, 2 call sites** | priced below |
| `NesBus::dma_read` | 109 | 76 | its inlined read became the call `read_internal` already was |

So **two** callees still gain their first call site, and MF-3b rule 1 says to name them rather
than let them pass:

* `Ppu2C02::read`, measured at **+4.09 instructions a register read** against **−5.90 a read
  cycle** — it cannot lose (step 2).
* `Cartridge::try_cpu_read`, which replaced `Board::cpu_read_override` as the boundary. It is the
  *outer* function, so the MMC5 now pays **one** boundary where the split made it pay the
  override's as well, and variant **I** measured that forcing this one inline too is **worse**
  (−0.802 % against −0.841 %, and −0.463 % against −0.733 % on the MMC5): with both inline the
  two copies of the body grow past what they save. The boundary where G leaves it is the measured
  optimum of the nine.

Per symbol, the MMC5, which is where the whole argument of this lot lives:

| symbol | baseline | split (A) | **G** |
| --- | ---: | ---: | ---: |
| `NesBus::read_internal` | 607 291 787 | 1 090 984 | 1 076 430 |
| `NesBus::read_cycle` | 25 602 335 | 383 374 809 | **231 692 814** |
| `Board::cpu_read_override` | *inlined* | **308 101 626** | ***inlined*** |
| `Cartridge::try_cpu_read` | *inlined* | *inlined* | **300 054 385** |
| `Cartridge::default_cpu_read` | 283 356 728 | 283 356 728 | 283 356 728 |
| `NesBus::read_wide_window` | — | — | 10 607 490 |
| `Ppu2C02::read` | *inlined* | 5 364 857 | 5 364 857 |
| `WideExtension::read` | *inlined* | 2 357 220 | *inlined* |
| **whole process** | 11 486 356 993 | 11 553 670 347 | **11 402 193 161** |

`default_cpu_read` at exactly the same count in all three columns is the check that none of this
is work appearing or disappearing: it is where the boundary sits, nothing else.

The two cold arms cost, per 300 frames: `read_wide_window` 1.57 M on the Holy Mapperel ROMs,
10.6 M on the MMC5 reading ExRAM through `$5C00`, under 0.15 M on any commercial game;
`read_io_register` under 90 000 anywhere. `read_wide_window` is `#[cold]` even though
`$6000`-`$7FFF` is the PRG-RAM of every battery game, because that range reaches
`wide_window_read`, which answers `None` for any cartridge whose ROM never wrote the unlock
sequence — the numbers above are what that detour costs.

### `#[cold]` or plain `#[inline(never)]`: the instrument cannot tell, so the code says why

Variant **H** is **G** with the `#[cold]` dropped: −0.842 % against −0.841 %, a difference of
157 000 instructions in 81 **billion**. `#[cold]` is kept anyway, and the reason is stated rather
than implied: it moves those two bodies into a section away from the hot path, which is an
instruction-cache property callgrind's `Ir` count is blind to and which matters on the A53's small
caches (§4, §8 of `PERFORMANCE-HISTORY.md`). Where the instrument is silent, the attribute that
documents the intent wins.

### What this instrument's floor actually is

Worth recording, because it was measured by accident. The variant was built twice from trees that
differ **only in comments** (the final doc comments were written after G was measured), and the
whole-process figures differ by **100 469 instructions in 81 billion, 0.0001 %**. Every single
`zax_nes` symbol is identical between the two, to the instruction; the entire difference is
`getgeneric` in glibc's time handling (+29 k on `m1`) and Lua's string table
(`luaH_getshortstr` −13 k, `luaS_newlstr` +5 k), i.e. the harness's start-up, not the emulator.

So the whole-process figure carries about 10⁵ instructions of host noise, and differences at that
scale between variants — **E**'s +5 688 on `vblnmi`, **G** against **H** — are not signal. The
numbers this lot rests on are 47 to 85 **million**, three orders of magnitude above the floor, and
the per-symbol `zax_nes` counts have no floor at all: they are exact.
