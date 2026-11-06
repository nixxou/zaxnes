# Lot MF-CART — lift the limit on new boards, then un-park

The baseline **and** the performance reference is `44f77f72`, the end of MF-6c. Read first:
`docs/progress/MF-6c.md` (all of it — step 2's seven shapes, step 5's measurement of both shared
enums' limit, the plan revision `9367929e`, step 9's `DecodeBoard` limit),
`docs/progress/MF-6a3.md` step 1, `docs/review/BOARD-FEATURES.md`,
`rust/docs/PERFORMANCE-HISTORY.md` §5.10 onwards.

## The problem this lot exists for

Every new board now costs `mmc5` about **+7.46 M instructions (+0.066 %)** over 300 frames, above
the owner's **+0.02 %** tolerance, through the inlined `Board::prg_ram_enabled` dispatch inside
`Cartridge::default_cpu_read` — the MMC5 is the one board that maps PRG-RAM into $8000-$DFFF, so
`Cartridge::read_rom_window` cannot answer and **every** CPU read of an MMC5 game reaches that
function. MF-6c measured the figure **nine times**, in both directions, and found each shared
enum's own limit: `SmallBoard` ten variants, `IrqBoard` ten, `DecodeBoard` four. Two boards are
parked on it today — `board-taito-x1` (80, 207, 82, 552) and `board-111` — and twelve MF-UNL lots
(37 boards) plus MF-VS are planned after this one.

## Steps (n = 8)

0. The plan, the driver and the baseline at `44f77f72` on all three instruments.
1. **Reproduce** the limit: one more variant in each shared enum, priced on `mmc5`, and the symbol
   where it lands. The figure to beat.
2. **The structural fix**, built and green: the $6000-$7FFF read answer settled on `Cartridge`
   the way `rom_windows` and `nametable_pages` already are, so that `Cartridge::default_cpu_read`
   dispatches on `Board` for it no longer. No behaviour change.
3. **The fix measured**: the default build against the baseline, and step 1's experiment re-run.
   The target is that one more `Board` variant costs **no ROM more than +0.02 %**.
4. Other shapes, if step 3 does not reach the target (the jump tables of
   `Board::expansion_audio_output`, `ppu_peek`, `irq_asserted`).
5. **Un-park**: `board-taito-x1` and `board-111` re-measured on the new tree; each one inside the
   tolerance goes into `default`. `BOARD-FEATURES.md` updated.
6. **The new limit written down** in `PERFORMANCE-HISTORY.md`: how many more variants the shared
   enums, and `Board` itself, take before a ROM crosses +0.02 %, so MF-VS and the MF-UNL lots can
   plan for it.
7. The check: fmt, clippy, the suites in both configurations, the `--ignored` pair, golden-core and
   the campaign field by field.

## The bar, at every step

* Both runs at `44f77f72`'s figures or lower in the **default** build: PR-1's thirteen ROMs
  (`callgrind-set.sh` + `callgrind-diff.py`) and level 2's five games on `core` *and* `total`
  (`callgrind-l2.sh` + `l2-diff.py --bar`), against `mfct-0-44f77f72`, on clean `git archive`
  builds in their own directories.
* The three claimants of `Cpu6502::step` at **0 / 0 / 0**, `WideExtension::zone_offset` not dearer
  (the widescreen hack), and `Ppu2C02::read_pattern_planes` **unchanged**.
* `cargo fmt --check` clean, clippy `-D warnings` on the crates touched in `default` *and*
  `--features parked-boards` with no new `allow`, and the suites green in both.
* No behaviour change: the save-state format unchanged, golden-core and the campaign identical.

### The labels

`rust/target-mf/mfct.sh` is this lot's driver — scratch, never staged, as `mf6a.sh`, `mf62.sh`,
`mf6b.sh`, `mf63.sh` and `mf6c.sh` were, because `rust/target-*` is never committed. Every label
has the form **`mfct-<step>-<8 chars>`**, 15 characters, the length of every MF lot's since MF-6a
step 0 measured why: the same commit built at two paths of different lengths differs by up to
1.56 M instructions on a level-2 `total`, all of it in `load_profile` and `lua_hooks`, which open
files and run Lua whose paths are longer or shorter. `asmlinct.sh` and `symct.sh` are the same
two helpers under this lot's scratch root.

## Step 0 — the baseline at `44f77f72`, on three instruments

### The commands

Host (Git Bash):

    mkdir -p rust/target-mf/tmp
    sh rust/tools/perf/make-tree.sh 44f77f72 rust/target-mf/tmp/mfct-0-44f77f72.tar

WSL2 Ubuntu-24.04 (valgrind-3.22.0, 24 cores), through the driver, every one in the
**foreground** with a long timeout: a headless session ends as soon as it stops answering, and a
process backgrounded inside a `wsl.exe` call is killed when the call returns (MF-6a step 0). The
`wsl.exe` invocations go through PowerShell and not Git Bash, which rewrites a `/mnt/g/...`
argument into `C:/Program Files/Git/mnt/g/...`.

    sh rust/target-mf/mfct.sh build mfct-0-44f77f72      # 45 s, then the tree is deleted
    sh rust/target-mf/mfct.sh set   mfct-0-44f77f72      # 13 ROMs, 8 jobs
    sh rust/target-mf/mfct.sh l2    mfct-0-44f77f72      # 5 games, 5 jobs
    sh rust/target-mf/mfct.sh asm   mfct-0-44f77f72      # post-LTO asm, into target-mf/asm/

### Instrument 1: the five level-2 games — **the bar for the whole lot**

`stages.txt` of `l2-mfct-0-44f77f72`, with MF-6c's own baseline column (`8496c503`) beside it so
that a reader can see the whole of MF-6c in the difference:

| game/stage | MF-6c step 0 (`8496c503`) | **`mfct-0-44f77f72`** | delta |
| --- | ---: | ---: | ---: |
| `metroid/core` | 11 147 295 142 | **11 147 295 142** | **+0** |
| `smb1/core` | 11 515 128 803 | **11 515 128 803** | **+0** |
| `zelda2/core` | 12 775 651 048 | **12 775 651 048** | **+0** |
| `mm2/core` | 12 036 822 861 | **12 036 822 861** | **+0** |
| `smb3/core` | 15 315 558 322 | **15 315 537 499** | **−20 823** |
| `metroid/total` | 13 735 627 354 | 13 735 578 935 | −48 419 |
| `smb1/total` | 13 589 030 161 | 13 586 757 575 | −2 272 586 |
| `zelda2/total` | 15 360 284 960 | 15 360 318 280 | +33 320 |
| `mm2/total` | 18 976 349 017 | 18 976 430 445 | +81 428 |
| `smb3/total` | 24 187 154 526 | 24 188 986 419 | +1 831 893 |

**Four `core` figures identical to the instruction and the fifth is MF-6c's own −20 823**, the
figure its steps 6 to 9 all measured on `smb3/core`. So the reference is reproduced: this column
is `44f77f72`'s code and nothing else, and it is **the bar for the whole lot**.

### Instrument 2: PR-1's thirteen — **the bar, ROM by ROM**

| ROM | board | **`mfct-0-44f77f72`** | against `mf6c-9-gt111prk` |
| --- | --- | ---: | ---: |
| `vblnmi` | MMC1 | 3 308 287 870 | −1 991 |
| `oamstr` | NROM | 3 318 485 575 | −33 716 |
| `m1` | MMC1 | 4 885 953 734 | +50 807 |
| `m4` | MMC3 | 5 472 522 428 | +54 688 |
| `m0` | NROM | 5 520 499 653 | −8 226 |
| `metroid` | MMC1 | 5 115 985 915 | +36 613 |
| `smb1` | NROM | 5 996 362 814 | −22 359 |
| `mm2` | MMC1 | 6 017 861 569 | +826 |
| `z2` | MMC1 | 6 291 442 776 | −45 179 |
| `readbuf` | CNROM | 6 834 534 036 | −62 362 |
| `duelito` | NROM | 7 380 421 165 | −14 602 |
| `smb3` | MMC3 | 7 949 489 726 | +34 859 |
| `mmc5` | MMC5 | **11 279 358 122** | **−6 196** |
| **total** | | **79 371 205 383** | **−16 838** |

The right-hand column is the **same code built under another label**, and every figure in it is
inside the ±57 k-per-ROM `luaH_getshortstr` floor §5.11 measured — which is what says the
instrument is the instrument. The one that matters is **`mmc5` −6 196**: `44f77f72` sits on the
*favourable* side of `Cartridge::default_cpu_read`'s two layouts, which is the side a successor
lot has to be able to stay on.

### Instrument 3: the post-LTO dump

`pins63.py lin-mfct-0-44f77f72.s`, with MF-6c step 9's shipped figures beside it:

| symbol | MF-6c step 9 | **`mfct-0-44f77f72`** |
| --- | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 3 626 | **3 626** |
| `Board::ppu_peek` | 291 | **291** |
| `NesBus::write_internal` | 3 861 | **3 861** |
| `Ppu::run_dots` | 1 919 | **1 919** |
| `NesBus::catch_up_ppu` | 188 | **188** |

`shape.py` beside it: `Cpu::resolve` 2 / 6 / **1 948**, `Cpu::branch` 1 / 1 / **457**,
`NesBus::read_cycle` 1 / 65 / **181**, `tick` 1 / 95 / **82**, `read_internal` 1 / 17 / **175** —
every one MF-6c's figure to the instruction.

And the three functions this lot has to watch by name, with `fn.py`:

| symbol | MF-6c step 9 | **`mfct-0-44f77f72`** |
| --- | ---: | ---: |
| `Ppu2C02::read_pattern_planes` | 107 | **107** |
| `Ppu2C02::read_nametable_inner` | 75 | **75** |
| **`Cartridge::default_cpu_read`** | **429** | **429** |

The last row is the function this lot is about. `read_pattern_planes` at **107** is the figure
MF-2's shield has held for eight lots and the one no step here may move.

[PROGRESS] MF-CART step 0/8 done: the plan, the driver, and `44f77f72` reproduced on all three
instruments — four level-2 `core` figures identical to the instruction and `smb3/core` at MF-6c's
own −20 823, every ROM of PR-1's thirteen inside the ±57 k whole-process floor against MF-6c step
9's shipped build, `mmc5` on the **favourable** layout at −6 196, and all eleven pinned symbols
holding with `default_cpu_read` at **429**.
