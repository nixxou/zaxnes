# Lot MF-6a — the licensed boards with no hook at all

The baseline is **`240a2718`**, the end of MF-5b-fix. The plan is `docs/review/MF-6-inventory.md`
§5 "MF-6a" (boards, steps 0-7, tests per board, commercial titles) and §4 (the hooks and the hot
paths), with the brief's five changes taking precedence over §5 where they differ.

What the lot adds: **20 mapper numbers** — 33, 37, 47, 70, 75, 76, 88, 89, 93, 94, 95, 97, 119,
140, 152, 154, 155, 184, 185, 206, 232 minus the six that are a parameter of an existing board —
in **eight new `Board` variants**, none of which touches `irq_asserted`, `on_cpu_cycle`,
`on_ppu_address_bus` or `read_nametable`.

## Steps

0. The plan, the baseline's test count, and the three instruments reproduced against `240a2718`.
1. **206**, and 76/88/95/154 as its parameters.
2. **33, 97, 140, 184, 89, 93** — five variants, all of them a register decode.
3. **70, 152, 75, 232.**
4. The parameters of existing boards: **94** on `UxRom`, **185** on `CnRom`, **155** on `Mmc1`.
5. **119, 47, 37** on `Mmc3`, last and alone (`m4` and `smb3` are MMC3s).
6. Save states for the new boards; the four sweeps re-run; `req_core_100_…_registered` extended.
7. The suites, the campaign ROM by ROM against MF-5b-fix's 331 rows, and the documents.

## The bar, and why it is two runs measured at every step

The brief's change 1. Both halves are measured at **every** step, not only at the end, each on a
clean `git archive` build in its own directory with its own target directory (`make-tree.sh` +
`build-clean.sh`, which deletes the tree and its target directory as soon as the binary is out):

* **level 2** — `callgrind-l2.sh` + `l2-diff.py --bar`, the five games of `roms-l2.txt` with their
  3D packs, 600 frames after the harness's 120 of warm-up, inputs pressed. Every game at
  `240a2718`'s **total and core** or lower. This is the run that caught MF-5b; PR-1's set alone
  hid it (PERFORMANCE-HISTORY §5.10).
* **PR-1's set** — `callgrind-set.sh` + `callgrind-diff.py`, thirteen ROMs, 300 frames, no pack.
  Every ROM at `240a2718`'s count or lower.

### The labels are all the same length, and that is deliberate

`rust/target-mf/mf6a.sh` is this lot's driver; every label it is given has the form
**`mf6a-<step>-<hash>`**, 15 characters, so that every build of this lot lives at a path of
exactly the same length. Step 0 found out why that matters: the same commit built twice, once as
`tree-base-240a2718` and once as `tree-mf6a-0-240a2718`, gives

| | PR-1 set, per ROM | level 2, `core` | level 2, `total` |
| --- | --- | --- | --- |
| spread between the two builds | −31 k to +52 k (≤ 0.0009 %) | **0, to the instruction** | −1.56 M to +0.28 M (≤ 0.012 %) |

The emulated core does not depend on where the tree was built; `load_profile` and `lua_hooks` do,
because they open files and run Lua whose paths are longer or shorter. MF-5b-fix saw the same
thing and said so (step 4, "the two stages that read files and run Lua"). So **the baseline this
lot is judged against is the one measured here, at an identically-shaped path**, and the ±50 k /
±1.5 M wobble above is the noise floor to read any single figure against.

## Step 0 — the baseline, and the three instruments

### The commands

Host (Git Bash), one tar per commit:

    mkdir -p rust/target-mf/tmp
    sh rust/tools/perf/make-tree.sh 240a2718 rust/target-mf/tmp/mf6a-0-240a2718.tar

WSL2 Ubuntu-24.04 (`rustc` 1.99.1 in `~/.cargo/bin`, `valgrind-3.22.0`), through the driver, which
sets `PR1_WORK=$HOME/mf6a`, the two ROM roots and the two pack roots:

    sh rust/target-mf/mf6a.sh build mf6a-0-240a2718     # 43 s, then the tree is deleted
    sh rust/target-mf/mf6a.sh set   mf6a-0-240a2718     # 13 ROMs, 8 jobs, about a minute
    sh rust/target-mf/mf6a.sh l2    mf6a-0-240a2718     # 5 games, 5 jobs, about six minutes
    sh rust/target-mf/mf6a.sh asm   mf6a-0-240a2718     # post-LTO asm, into target-mf/asm/

The level-2 run is launched from the host with `Start-Process` rather than `nohup … &` inside a
`wsl.exe` call: a process backgrounded inside such a call is killed when the call returns, which
cost two attempts before the symptom was understood. `pgrep valgrind` finds nothing while it runs
— valgrind execs `callgrind-amd64-linux`, so the pattern to poll for is `callgrind`.

### Instrument 1: PR-1's thirteen, reproduced

`callgrind-set.sh`, and beside it MF-5b-fix step 4's own column for the same code:

| ROM | board | MF-5b-fix's table | **`mf6a-0-240a2718`** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `oamstr` | NROM | 3 320 466 003 | **3 320 493 647** | +27 644 | +0.0008 % |
| `vblnmi` | MMC1 | 3 311 130 882 | **3 311 139 490** | +8 608 | +0.0003 % |
| `m1` | MMC1 | 4 887 150 614 | **4 887 169 086** | +18 472 | +0.0004 % |
| `metroid` | MMC1 | 5 116 394 014 | **5 116 421 901** | +27 887 | +0.0005 % |
| `m4` | MMC3 | 5 474 237 729 | **5 474 254 551** | +16 822 | +0.0003 % |
| `m0` | NROM | 5 521 486 644 | **5 521 503 625** | +16 981 | +0.0003 % |
| `smb1` | NROM | 5 997 201 028 | **5 997 252 807** | +51 779 | +0.0009 % |
| `mm2` | MMC1 | 6 020 978 149 | **6 020 951 979** | −26 170 | −0.0004 % |
| `z2` | MMC1 | 6 293 906 638 | **6 293 910 344** | +3 706 | +0.0001 % |
| `readbuf` | CNROM | 6 839 081 259 | **6 839 070 699** | −10 560 | −0.0002 % |
| `duelito` | NROM | 7 385 523 936 | **7 385 492 577** | −31 359 | −0.0004 % |
| `smb3` | MMC3 | 7 951 502 203 | **7 951 488 506** | −13 697 | −0.0002 % |
| `mmc5` | MMC5 | 11 298 802 676 | **11 298 803 771** | +1 095 | +0.0000 % |
| **total** | | **79 417 861 775** | **79 418 032 983** | +171 208 | +0.0002 % |

The right-hand column is **the bar for the whole lot**: every step at that number or lower, ROM by
ROM. The deltas against MF-5b-fix's own table are the build-path wobble described above; nine ROMs
move by less than 0.0005 % and none by more than 0.0009 %.

### Instrument 2: the five level-2 games, reproduced

`callgrind-l2.sh`, `stages.txt`, against MF-5b-fix step 4's shipped column:

| game/stage | MF-5b-fix's table | **`mf6a-0-240a2718`** | delta |
| --- | ---: | ---: | ---: |
| `metroid/total` | 13 736 666 706 | **13 736 688 715** | +22 009 |
| `metroid/core` | 11 148 433 485 | **11 148 433 485** | **0** |
| `smb1/total` | 13 590 153 669 | **13 588 597 037** | −1 556 632 |
| `smb1/core` | 11 516 670 821 | **11 516 670 821** | **0** |
| `zelda2/total` | 15 362 793 261 | **15 363 074 749** | +281 488 |
| `zelda2/core` | 12 778 994 409 | **12 778 994 409** | **0** |
| `mm2/total` | 18 983 454 568 | **18 983 316 177** | −138 391 |
| `mm2/core` | 12 043 869 909 | **12 043 869 909** | **0** |
| `smb3/total` | 24 195 620 108 | **24 194 492 263** | −1 127 845 |
| `smb3/core` | 15 320 386 010 | **15 320 386 010** | **0** |

**All five `core` figures are character for character MF-5b-fix's**, which is MF-5a step 1's drift
check at its strongest: the emulated core is the code R0 and the previous lot measured, so the
instrument has not drifted. The `total` column carries the `load_profile`/`lua_hooks` wobble.

The other six stages of the baseline, for the comparisons the later steps make:

| game | apu | ppu | draw_list | load_profile | convert | lua_hooks |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `metroid` | 1 262 309 912 | 6 926 425 344 | 181 680 722 | 1 837 132 206 | 394 143 059 | 21 385 469 |
| `mm2` | 2 127 696 029 | 6 966 635 242 | 181 525 920 | 6 009 162 148 | 545 789 610 | 91 853 738 |
| `smb1` | 1 933 200 654 | 6 593 034 016 | 127 208 122 | 612 525 467 | 1 193 280 990 | 901 520 919 |
| `smb3` | 2 370 188 700 | 9 523 151 746 | 418 218 675 | 5 252 021 180 | 2 977 191 111 | 1 717 987 586 |
| `zelda2` | 2 135 871 580 | 7 290 417 979 | 107 033 548 | 1 402 991 527 | 904 483 596 | 535 736 077 |

### Instrument 3: the post-LTO code

`asm/shape.py` on `lin-mf6a-0-240a2718.s`, beside MF-5b-fix step 3's own table (def/sites/instr):

| symbol | MF-5b-fix's "shipped" | **`mf6a-0-240a2718`** |
| --- | ---: | ---: |
| `<Board as Mapper>::irq_asserted` | 0 / 0 / 0 | **0 / 0 / 0** |
| `WideExtension::zone_offset` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu6502::execute_without_operand` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu6502::step` | 3 443 | **1 / 2 / 3 443** |
| `Cpu6502::resolve` | 1 854 | **2 / 6 / 1 854** |
| `Cpu6502::branch` | 418 | **1 / 1 / 418** |
| `NesBus::read_cycle` | 181, 65 sites | **1 / 65 / 181** |
| `NesBus::tick` | 82, 95 sites | **1 / 95 / 82** |
| `NesBus::read_internal` | 175, 17 sites | **1 / 17 / 175** |
| `NesBus::write_internal` | 3 020 | **1 / 10 / 3 020** |
| `Ppu2C02::run_dots` | 1 906 | **1 / 2 / 1 906** |

Every figure is MF-5b-fix's. The three claimants of `Cpu6502::step`'s inlining budget are all
inlined into it — **0 definitions and 0 call sites each** — which is the state the brief's change 2
requires this lot to preserve after **each** new `Board` variant, together with `branch` 418 and
`resolve` 1 854 moving by no more than §4.2's per-arm yardstick (+11 in `branch`, +47 in
`resolve`, per arm).

### The baseline's test count

    cargo test --release -p zax-nes -p zax-harness -- --test-threads=1
    816 passed, 0 failed, 8 ignored

which is MF-5b-fix's and MF-6's figure.

[PROGRESS] MF-6a step 0/7 done: the plan, 816 / 0 / 8, and the three instruments reproduced.

## Status

Step 0 done.
