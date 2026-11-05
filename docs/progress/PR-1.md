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
| **total** | | **84 944 886 205** |

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
