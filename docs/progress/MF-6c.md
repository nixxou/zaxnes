# Lot MF-6c — the rest of the licensed boards, under the owner's two rules

The baseline is **`45aa86ae`** (the branch tip, after the UNL census, which changed no code) and
the performance reference is **`8496c503`** (the end of MF-6a3). The plan is
`docs/review/MF-6-inventory.md` §5 "MF-6c" and §4; read also `docs/review/BOARD-FEATURES.md`,
`docs/progress/MF-6a3.md` and `rust/docs/PERFORMANCE-HISTORY.md` §5.10 to §5.14.

**The owner's rules of 2026-11-06 replace every "stop-the-lot condition" and every "decode and
ignore it" of §5 MF-6c:**

1. **No board, and no feature of a board, is ever dropped for performance.** Each board that
   raises any figure gets its own Cargo feature in `zax-nes`, gating its code, variant, dispatch
   arms, registry entries, save-state block and tests, as `BOARD-FEATURES.md` describes. A costly
   *mechanism* of a board — Sunsoft-4's CHR-ROM nametables through `read_nametable`, 77's split
   `chr_view`, 96's PPU-bus watching — is implemented too, behind its own feature if it costs more
   than the board itself.
2. **The tolerance**: at most **+0.02 % per ROM** on both runs against `8496c503`, with the three
   `Cpu6502::step` inlinings at 0 / 0 / 0, `zone_offset` not dearer and
   `Ppu2C02::read_pattern_planes` unchanged in the default build. Inside it the feature goes in
   `default`; above it, it stays out and joins `parked-boards`. With a feature off the build is the
   same as without it. A level-2 total-only rise the per-function diff puts entirely in Lua's
   `ltable.c` or in `malloc` is layout noise; the `core` figure decides.
3. **Every new feature gets its row in `BOARD-FEATURES.md`**, and every configuration (default, all
   off, `--features parked-boards`) passes fmt, clippy `-D warnings` and the tests.
4. The only reasons left to leave something out are **no reliable public documentation** or a
   **development cost out of proportion** (argued). The µPD7756C sample audio of 72/92/86 stays out
   on that ground unless its page documents it fully (§4.8); Oeka Kids' tablet and the external
   option ROM of *Nantettatte!! Baseball* stay out as input/loader surfaces this build has none of
   — recorded as gaps.

## The fourteen boards

| board | numbers | the mechanism that costs |
| --- | ---: | --- |
| Bandai FCG + WRAM | **153** | a `BandaiFcg` parameter; PRG-RAM where the page says it is |
| Taito X1-005 | **80**, **207** | 128 B of mapper-internal battery WRAM, two mirroring wirings |
| Taito X1-017 | **82**, **552** | 5 KiB of the same; 552 is the corrected bank order |
| Bandai LZ93D50 + EEPROM | **157**, **159** | a serial `Eeprom24C0X`, new to this build |
| Sunsoft-4 | **68** | the Q-123 ceiling: CHR-ROM nametables, a third `read_nametable` body |
| Irem LROG017 | **77** | a split `chr_view`: CHR-ROM at $0000, CHR-RAM at $0800-$1FFF |
| Jaleco JF-17 / JF-19 / JF-13 | **72**, **92**, **86** | boards yes, µPD7756C audio no (§4.8) |
| Oeka Kids | **96** | CHR banking from every PPU address-bus change |
| Cheapocabra (GTROM) | **111** | mapper-selected CHR-RAM nametable pages, self-flashing PRG |

## Steps (n = 10)

0. The plan, the driver and the baseline at `8496c503` on all three instruments.
1. **153** — the `BandaiFcg` parameter, built and measured.
2. **80, 207, 82, 552** — the mapper-internal WRAM, the Namco 163's shape (§4.6), built and
   measured, both bank orders pinned in one test.
3. **157, 159** — the `Eeprom24C0X` state machine beside `vrc_irq.rs`, built and measured.
4. **68** — the CIRAM half through `nametable_pages_vary`, **and** the CHR-ROM nametable mode,
   which is the third `read_nametable` body, behind its own feature (owner's rule 1).
5. **77** — the split `chr_view`, built and measured.
6. **72, 92, 86** — the three-way parameter, audio pinned silent.
7. **96** — the PPU address bus, `watches_ppu_bus_without_read`, MR-1's open question answered.
8. **111** — `nametable_page` if it reduces to one, else the same answer 68's mechanism got.
9. The features' `default` / `parked` placement from the measurements, `BOARD-FEATURES.md`, the
   four sweeps, every configuration green, the save states.
10. The check: `--ignored` with both ROM folders, the campaign field by field against MF-6a3's
    331, the inventory, the Q tickets and the journal.

## The bar, at every step

* **+0.02 % per ROM** on PR-1's thirteen (`callgrind-set.sh` + `callgrind-diff.py`) and on level
  2's five games, `core` *and* `total` (`callgrind-l2.sh` + `l2-diff.py --bar`), against
  `mf6c-0-8496c503`, on clean `git archive` builds in their own directories.
* The three claimants of `Cpu6502::step` at **0 / 0 / 0**, `zone_offset` not dearer, and
  `Ppu2C02::read_pattern_planes` **unchanged** in the default build.
* `cargo fmt --check` clean, `cargo clippy -p <crates touched> --all-targets -- -D warnings` clean
  with **no new `allow`**, and the suites green in default, `--no-default-features` and
  `--features parked-boards`.

### The labels

`rust/target-mf/mf6c.sh` is this lot's driver — scratch, not committed, as `mf6a.sh`, `mf62.sh`,
`mf6b.sh` and `mf63.sh` were, because `rust/target-*` is never staged. Every label has the form
**`mf6c-<step>-<8 chars>`**, 15 characters, the length of MF-6a's, MF-6a2's, MF-6b's and MF-6a3's.
MF-6a step 0 measured why: the same commit built at two paths of different lengths differs by up
to 1.56 M instructions on a level-2 `total`, all of it in `load_profile` and `lua_hooks`, which
open files and run Lua whose paths are longer or shorter.

## Step 0 — the baseline at `8496c503`, on three instruments

### The commands

Host (Git Bash):

    mkdir -p rust/target-mf/tmp
    sh rust/tools/perf/make-tree.sh 8496c503 rust/target-mf/tmp/mf6c-0-8496c503.tar

WSL2 Ubuntu-24.04 (valgrind-3.22.0, 24 cores), through the driver, every one in the **foreground**
with a long timeout: a headless session ends as soon as it stops answering, and a process
backgrounded inside a `wsl.exe` call is killed when the call returns (MF-6a step 0). The
`wsl.exe` invocations go through PowerShell and not Git Bash, which rewrites a `/mnt/g/...`
argument into `C:/Program Files/Git/mnt/g/...`.

    sh rust/target-mf/mf6c.sh build mf6c-0-8496c503      # 37 s, then the tree is deleted
    sh rust/target-mf/mf6c.sh set   mf6c-0-8496c503      # 13 ROMs, 8 jobs
    sh rust/target-mf/mf6c.sh l2    mf6c-0-8496c503      # 5 games, 5 jobs
    sh rust/target-mf/mf6c.sh asm   mf6c-0-8496c503      # post-LTO asm, into target-mf/asm/

### Instrument 1: the five level-2 games — **the bar for the whole lot**

`stages.txt` of `l2-mf6c-0-8496c503`, beside MF-6a3 step 0's column, which is `07325f6e`'s code —
the difference between the two is MF-6a3 itself (119, 47, 37 and 116):

| game/stage | MF-6a3 step 0 (`07325f6e`) | **`mf6c-0-8496c503`** | delta |
| --- | ---: | ---: | ---: |
| `metroid/core` | 11 147 278 310 | **11 147 295 142** | +16 832 |
| `smb1/core` | 11 515 125 238 | **11 515 128 803** | +3 565 |
| `zelda2/core` | 12 775 641 366 | **12 775 651 048** | +9 682 |
| `mm2/core` | 12 036 819 361 | **12 036 822 861** | +3 500 |
| `smb3/core` | 15 315 558 069 | **15 315 558 322** | +253 |
| `metroid/total` | 13 735 514 951 | 13 735 627 354 | +112 403 |
| `smb1/total` | 13 587 898 562 | 13 589 030 161 | +1 131 599 |
| `zelda2/total` | 15 358 834 193 | 15 360 284 960 | +1 450 767 |
| `mm2/total` | 18 976 300 081 | 18 976 349 017 | +48 936 |
| `smb3/total` | 24 187 868 326 | 24 187 154 526 | −713 800 |

**Every `core` delta is MF-6a3 step 4's own figure to the instruction** — that step measured
mapper 116 at "+253 to +16 832" on the five cores and named `smb3/core` 253 and `metroid/core`
16 832 explicitly. So the reference is reproduced: this column is `8496c503`'s code and nothing
else, and it is **the bar for the whole lot**.

### Instrument 2: PR-1's thirteen — **the bar, ROM by ROM**

| ROM | board | **`mf6c-0-8496c503`** |
| --- | --- | ---: |
| `vblnmi` | MMC1 | 3 308 311 891 |
| `oamstr` | NROM | 3 318 460 965 |
| `m1` | MMC1 | 4 885 949 431 |
| `m4` | MMC3 | 5 472 514 995 |
| `m0` | NROM | 5 520 458 691 |
| `metroid` | MMC1 | 5 115 944 288 |
| `smb1` | NROM | 5 996 381 185 |
| `mm2` | MMC1 | 6 017 851 607 |
| `z2` | MMC1 | 6 291 458 642 |
| `readbuf` | CNROM | 6 834 538 187 |
| `duelito` | NROM | 7 380 437 910 |
| `smb3` | MMC3 | 7 949 470 334 |
| `mmc5` | MMC5 | 11 279 383 781 |
| **total** | | **79 377 161 907** |

### Instrument 3: the post-LTO dump

`pins63.py lin-mf6c-0-8496c503.s` — the three claimants plus the five symbols the brief names:

| symbol | def / sites / instructions |
| --- | --- |
| `Board::irq_asserted` | **0 / 0 / 0** |
| `Wide::zone_offset` | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | **0 / 0 / 0** |
| `Cpu::step` | 1 / 2 / **3 626** |
| `Board::ppu_peek` | 1 / 4 / **272** |
| `NesBus::write_internal` | 1 / 10 / **3 815** |
| `Ppu::run_dots` | 1 / 5 / **1 907** |
| `NesBus::catch_up_ppu` | 1 / 12 / **188** |

`shape.py` beside it: `Cpu::resolve` 2 / 6 / **1 948**, `Cpu::branch` 1 / 1 / **457**,
`NesBus::read_cycle` 1 / 65 / **181**, `tick` 1 / 95 / **82**, `read_internal` 1 / 17 / **175**.
`Cpu::step`, `ppu_peek`, `run_dots`, `catch_up_ppu`, `resolve`, `branch`, `read_cycle`, `tick` and
`read_internal` are **MF-6a3 step 0's figures to the instruction**; `write_internal` is 3 580 →
**3 815**, the register decodes of 119, 47, 37 and 116 that MF-6a3 added.

**Two more symbols this lot has to watch, read on the same dump with `fn.py`**, because the
owner's rule 2 names one of them and §4.5 is the other:

| symbol | `mf6c-0-8496c503` |
| --- | ---: |
| `Ppu2C02::read_pattern_planes` | **107** instructions |
| `Ppu2C02::read_nametable_inner` | **114** instructions, two distinct arm bodies |

`read_pattern_planes` at 107 is the figure §5 MF-6c step 5 names ("106-107 instructions,
`IDENTICAL raw` through MF-5a and MF-5b") and the one step 5 must not move. `read_nametable_inner`
at **114** is Q-123's ceiling as this tree stands — MF-5a measured 132 with the two bodies and
109 with a third, on a tree seven lots older; **this lot re-measures it rather than quoting the
132**, because step 4 is the step that adds the third body and the owner's rule says it ships
behind its own feature whatever it costs.

[PROGRESS] MF-6c step 0/10 done: the plan, the driver, and `8496c503` reproduced on all three
instruments — the five level-2 `core` deltas against MF-6a3 step 0 are mapper 116's own
+253 to +16 832, and all eight pinned symbols hold.
