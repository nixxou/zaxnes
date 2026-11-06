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

## Steps (n = 11)

**Step 5 was inserted after step 4**, which found a shape that looked as though it would un-park
`board-taito-x1` and said so; the insertion is that experiment, and it is a negative result. The
steps after it keep the order and the content step 0 planned.

0. The plan, the driver and the baseline at `8496c503` on all three instruments.
1. **153** — the `BandaiFcg` parameter, built and measured.
2. **80, 207, 82, 552** — the mapper-internal WRAM, the Namco 163's shape (§4.6), built and
   measured, both bank orders pinned in one test.
3. **157, 159** — the `Eeprom24C0X` state machine beside `vrc_irq.rs`, built and measured.
4. **68** — the CIRAM half through `nametable_pages_vary`, **and** the CHR-ROM nametable mode,
   which is the third `read_nametable` body, behind its own feature (owner's rule 1).
5. **The un-parking attempt step 4 recommended**: the two Taito X1 boards moved to `IrqBoard` and
   re-measured. *Inserted, and a negative result.*
6. **77** — the split `chr_view`, built and measured.
7. **72, 92, 86** — the three-way parameter, audio pinned silent.
8. **96** — the PPU address bus, `watches_ppu_bus_without_read`, MR-1's open question answered.
9. **111** — `nametable_page` if it reduces to one, else the same answer 68's mechanism got.
10. The features' `default` / `parked` placement from the measurements, `BOARD-FEATURES.md`, the
    four sweeps, every configuration green, the save states.
11. The check: `--ignored` with both ROM folders, the campaign field by field against MF-6a3's
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
| **total** | | **79 371 161 907** |

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

[PROGRESS] MF-6c step 0/11 done: the plan, the driver, and `8496c503` reproduced on all three
instruments — the five level-2 `core` deltas against MF-6a3 step 0 are mapper 116's own
+253 to +16 832, and all eight pinned symbols hold.

## Step 1 — mapper 153, a parameter of mapper 16's board, at **zero** on all five level-2 cores

### What was built

nesdev ("INES Mapper 153"): "a Bandai FCG board with an LZ93D50 ASIC and 8192 bytes of
battery-backed WRAM", whose IRQ and mirroring registers "function the same way as on INES Mapper
016, submapper 5". Three things are the number's own, and the code quotes the page on each:

| what | where |
| --- | --- |
| $8000-$8003 as an outer **PRG** bank, "Select 256 KiB outer PRG-ROM bank" on bit 0, "The same value must be written to all four registers" | `BandaiFcg::outer_prg_153` |
| both windows inside that block, so the fixed one is the block's last 16 KiB and not the image's | `BandaiFcg::update_banks` |
| "No CHR banking is available" — the linear 8 KiB `BankWindows::new` lays down is what stays | the same method |
| $800D bit 5 as the WRAM chip enable, where the LZ93D50 proper has the EEPROM port there | `BandaiFcg::write_register`, arm `0xD` |
| the ports mirrored across **$8000-$FFFF only**, because $6000-$7FFF *is* the WRAM | `BandaiFcg::register_floor` |
| the 8 KiB being the header's, so no `board_memory_sizes` arm: byte 10 = $70 is 64 << 7 = 8 192, and an iNES 1.0 dump gets `DEFAULT_PRG_RAM_SIZE` | `nes20_prg_ram_size`, unchanged |

**One reading was tightened, exactly as MF-6a3 step 1 tightened the MMC3's three.** `is_fcg12` was
`header.is_nes20 && header.submapper == 4`, which is mapper 16's condition written without naming
mapper 16. nesdev's *NES 2.0 submappers* defines submappers 4 and 5 for mapper 16 and **none** for
153, 157 or 159, so a NES 2.0 dump of one of those carrying a submapper byte of 4 would have been
read as an FCG-1/2. It is now `mapper == 16 && …`, which is identical for every mapper-16 image.

**Two divergences from Mesen 2, both settled for the page by AMD-061 §1.** Mesen ORs bit 0 of all
**eight** registers into the outer bank, and applies the outer bank to mapper **16** as well
whenever an image carries 512 KiB or more of PRG (`BandaiFcg.h:164-170`). The page documents the
outer bank on 153 alone and names $8000-$8003 alone, so this build does neither;
`mf_6c_an_ordinary_mapper_16_image_is_unaffected_by_153` pins both halves of that.

**One power-on value the page does not give**: $800D bit 5. Mesen only calls
`SetCpuMemoryMapping` when the register is written (`BandaiFcg.h:226`), so until then its
cartridge keeps `BaseMapper`'s read/write mapping — enabled. This build powers on **enabled**, for
the reason `board_memory_sizes` already gives for mapper 97: taking RAM away from a game that
never writes the register is the worse of the two errors. No Q ticket — nothing normative is
ambiguous, and the page's quirk note ("booting with WRAM filled with zeroes, *Famicom Jump II*
will freeze with a black screen … soft-resetting … will then always run the game properly") is
about the WRAM's **contents**, which REQ-CORE-095's import path owns, not about this bit.

### The shape: **no `Board` variant, no dispatch arm, no new `irq_asserted` body**

Mapper 153 is two `bool`s on `BandaiFcg` and three branches, because that board already answers
*yes* to `Mapper::irq_asserted` and already clocks on a CPU cycle. So this step adds **nothing**
to the two unlatched hooks of §4.10 — which is why the inventory calls 153 "the easy one of the
group", and why the measurement below is what it is.

**The two `bool`s are appended**, and that is deliberate. rustc groups a struct's fields by
alignment and keeps declaration order inside a group, so declaring them after `irq_pending` leaves
every existing field where it was. Measured with `offset_of!`: `irq_latch` **120**, `prg` **128**,
`irq_enabled` **130**, `irq_pending` **131**, and the two new fields at **132** and **133** —
`size_of::<BandaiFcg>()` 132 → **136**, not one existing byte moved. That matters because
`Board::irq_asserted` reads `irq_pending` once per CPU cycle and MF-6b step 1 measured what moving
such a field costs: one byte of displacement on `VrcIrq::pending` cost eleven of PR-1's thirteen
ROMs +0.009 % to +0.027 % with no code added at all. `Board`'s own size is the MMC5's and is
untouched.

### Instrument 1: level 2 — **all five `core` figures identical to the instruction**

`l2-diff.py --bar`, against `mf6c-0-8496c503`:

| game/stage | `8496c503` | **step 1** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 147 295 142 | **11 147 295 142** | **+0** | **equal** |
| `smb1/core` | 11 515 128 803 | **11 515 128 803** | **+0** | **equal** |
| `zelda2/core` | 12 775 651 048 | **12 775 651 048** | **+0** | **equal** |
| `mm2/core` | 12 036 822 861 | **12 036 822 861** | **+0** | **equal** |
| `smb3/core` | 15 315 558 322 | **15 315 558 322** | **+0** | **equal** |
| `metroid/total` | 13 735 627 354 | 13 735 783 448 | +156 094 | +0.001 % |
| `smb1/total` | 13 589 030 161 | 13 589 137 572 | +107 411 | +0.001 % |
| `zelda2/total` | 15 360 284 960 | **15 359 048 252** | −1 236 708 | −0.008 % |
| `mm2/total` | 18 976 349 017 | **18 976 194 247** | −154 770 | −0.001 % |
| `smb3/total` | 24 187 154 526 | 24 188 391 106 | +1 236 580 | +0.005 % |

**Zero on every core**, which is what a board that adds no arm to any dispatch should cost and the
first time this lot's instrument has had to say it. The three `total` rises are the owner's rule 2
case exactly: `sym6c.sh` at a 300 000 threshold names **every** symbol that moved on any of the
five, and there are five of them — `luaH_set` +686 816 (`smb1`) and +1 099 592 (`smb3`),
`luaH_getshortstr` +636 924, +745 423 and **−1 458 096** (`zelda2`), `getgeneric` −1 330 864, and
`__memcpy_avx_unaligned_erms` −403 262. Lua's `ltable.c` and `memcpy`, nothing else, and
`zelda2`'s −1.24 M is `smb3`'s +1.24 M in the other direction. Layout noise; the core decides.

### Instrument 2: PR-1's thirteen — worst ROM **+0.001 %**

`callgrind-diff.py`, against `mf6c-0-8496c503`:

| ROM | board | `8496c503` | **step 1** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 308 311 891 | **3 308 284 507** | −27 384 | −0.001 % |
| `oamstr` | NROM | 3 318 460 965 | 3 318 492 573 | +31 608 | +0.001 % |
| `m1` | MMC1 | 4 885 949 431 | **4 885 941 173** | −8 258 | −0.000 % |
| `m4` | MMC3 | 5 472 514 995 | **5 472 508 676** | −6 319 | −0.000 % |
| `m0` | NROM | 5 520 458 691 | 5 520 498 033 | +39 342 | +0.001 % |
| `metroid` | MMC1 | 5 115 944 288 | 5 115 972 128 | +27 840 | +0.001 % |
| `smb1` | NROM | 5 996 381 185 | **5 996 359 172** | −22 013 | −0.000 % |
| `mm2` | MMC1 | 6 017 851 607 | **6 017 828 412** | −23 195 | −0.000 % |
| `z2` | MMC1 | 6 291 458 642 | 6 291 511 207 | +52 565 | +0.001 % |
| `readbuf` | CNROM | 6 834 538 187 | **6 834 529 501** | −8 686 | −0.000 % |
| `duelito` | NROM | 7 380 437 910 | 7 380 495 579 | **+57 669** | **+0.001 %** |
| `smb3` | MMC3 | 7 949 470 334 | **7 949 441 466** | −28 868 | −0.000 % |
| `mmc5` | MMC5 | 11 279 383 781 | **11 279 337 111** | −46 670 | −0.000 % |
| **total** | | **79 371 161 907** | **79 371 199 538** | **+37 631** | **+0.000 %** |

**The worst ROM of the thirteen is `duelito` at +57 669, which is +0.001 %** — one twentieth of
the owner's +0.02 %, and inside the ±57 k-per-ROM `luaH_getshortstr` floor §5.11 measured. Eight
of the thirteen are below the baseline.

### Instrument 3: the post-LTO dump

`pins63.py`, step 1 against the baseline:

| symbol | `8496c503` | **step 1** |
| --- | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 3 626 | **3 626** |
| `Board::ppu_peek` | 272 | **272** |
| `NesBus::write_internal` | 3 815 | 3 819 |
| `Ppu::run_dots` | 1 907 | **1 907** |
| `NesBus::catch_up_ppu` | 188 | **188** |

and `shape.py` beside it: `Cpu::resolve` 2 / 6 / **1 948**, `Cpu::branch` 1 / 1 / **457**,
`NesBus::read_cycle` 1 / 65 / **181**, `tick` 1 / 95 / **82**, `read_internal` 1 / 17 / **175** —
every one identical. The **only** symbol that moved anywhere in either instrument's pinned list is
`NesBus::write_internal`, **+4 instructions**, which is mapper 153's three branches inside the
register decode `Cartridge::cpu_write` is one body with. `zone_offset` did not get dearer: it does
not exist as a symbol, which is the inlining holding.

And the two symbols the owner's rule 2 and §4.5 name, read with `fn.py`:

| symbol | `8496c503` | **step 1** |
| --- | ---: | ---: |
| `Ppu2C02::read_pattern_planes` | 107 | **107** |
| `Ppu2C02::read_nametable_inner` | 114 | **114** |

### The tests, 899 → **902**

| test | what it pins |
| --- | --- |
| `req_core_100_mapper_153_banks_a_256_kib_outer_block_and_no_chr` | the inner 16 KiB bank, the outer block on bit 0 of all four registers, the fixed window being the **block's** last bank, the inner field wrapping inside the block rather than leaving it, and "No CHR banking is available" against writes to all eight registers |
| `req_core_100_mapper_153_has_battery_wram_at_6000_gated_by_800d_bit_5` | the WRAM readable and writable at power-on, $800D bit 5 clear taking it off the bus *and* refusing writes while keeping its contents, bit 5 set bringing the same byte back, $6009 being RAM and not the mirroring register, and REQ-CORE-095's slice being the whole 8 KiB with the written byte in it |
| `mf_6c_an_ordinary_mapper_16_image_is_unaffected_by_153` | a 512 KiB mapper-16 image: $8000 moving a **CHR** window and no PRG window (the first Mesen divergence), $8008 being a full byte and the fixed window the image's last, $6009 still being the mirroring register, no readable RAM at $6123, and $800D enabling nothing |

The three sweeps that write themselves held with nothing added:
`mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched` passes because 153 went into
`Cartridge::create`'s `matches!`,
`amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` because `reset` puts
`prg_ram_on` back, and `req_core_100_every_required_mapper_is_registered` because the number was
added to its `additions` list (38 → 39 entries) behind `cfg!(feature = "board-153")`.

**Three configurations green**: default **902 passed / 0 failed / 8 ignored** (MF-6a3's 899 plus
three), `--no-default-features` **783** (MF-6a3's 782 plus the one test that is in no feature),
`--features parked-boards` the same as default. `cargo fmt --check` clean; clippy `-D warnings` on
`zax-nes` in all three configurations and on `zax-harness`, clean, **no new `allow` of any kind**.

`board-153` is **inside the tolerance by a factor of twenty on every figure**, so it goes in
`default`; step 9 writes its row in `BOARD-FEATURES.md` and does the all-features-off dump
comparison for every feature this lot adds at once, as MF-6a3 step 3 did for its nine.

[PROGRESS] MF-6c step 1/11 done: mapper 153 as two appended `bool`s on `BandaiFcg`, 902 / 0 / 8,
all five level-2 cores **+0**, worst of PR-1's thirteen `duelito` +0.001 %, and the only pinned
symbol that moved is `write_internal` +4.

## Step 2 — the Taito X1-005 and X1-017, and **the first board this project has parked**

### What was built: four numbers, two boards, two `SmallBoard` variants

| what | where | the page's words |
| --- | --- | --- |
| the $7EFx register file, **and its $7E7x image** | `TaitoX1005::register_of` | "All registers exist at $7EFx and mirrored at $7E7x because the CPU A7 line is ignored" |
| the two 2 KiB CHR registers taking a **2 KiB** bank number | `TaitoX1005::update_banks`, `TaitoX1017::update_banks` | "Bit 0 of written bank number is ignored when switching 2K banks ($7EF0, $7EF1)" |
| $7EF6's arrangement bit, and its **absence** at $7EF7 on the X1-005 | `TaitoX1005::write_x1_register` | "0: vertical (A11); 1: horizontal (A10)"; "$7EF6 … is not mirrored at $7EF7" |
| the 128 bytes, their mirror and their magic value | `TaitoX1005::try_read_prg`, `is_internal_ram`, `RAM_MAGIC` | "CPU $7F00-$7F7F: 128 B internal RAM"; "write $A3 to enable" |
| mapper 207's CIRAM A10 | `TaitoX1005::refresh_nametable_pages` | "modifies Taito's X1-005 (80) boards in the exact same way that TLSROM (118) differs from MMC3 (4)"; "$7EF6: was old mirroring control register, now ignored" |
| the X1-017's CHR A12 inversion | `TaitoX1017::update_banks` | "CHR A12 inversion (determines whether 2KB banks occupy $0000-$0FFF or $1000-$1FFF)" |
| its three RAM windows and three magic values | `TaitoX1017::ram_window`, `RAM_MAGIC` | $CA / $69 / $84 over 2 KiB + 2 KiB + 1 KiB |
| **both bank orders** | `TaitoX1017::prg_bank`, `reversed_bank` | "NES 2.0 Mapper 552 represents the actual way the mask ROM is connected and is thus the correct bank order, while iNES Mapper 082 represents the bank order as it was understood before January 2020, when the mapper was reverse-engineered" |
| the internal WRAM as §4.6's Namco 163 layout | `registry::board_memory_sizes`, arms `80 \| 207` and `82 \| 552` | the chip's size, with no header RAM in front, so REQ-CORE-095's single slice *is* the chip's RAM |

Both boards are **`SmallBoard` variants**, by that enum's own entry condition: both pages say
"IRQ: None", so both share the empty `Board::irq_asserted` arm and `Board` keeps its
thirty-five discriminants.

**Four divergences from Mesen 2, all four settled for the page by AMD-061 §1.** Three are the
X1-005's, listed in that struct's note: the 2 KiB LSB (Mesen maps `value` and `value + 1`,
`TaitoX1005.h:54-55`), $7EF7 (Mesen answers it) and the $7E7x image (Mesen decodes $7EF0-$7EFF
only). The fourth is the X1-017's register range: the X1-005's page gives the $7E7x mirror and
**the X1-017's page does not**, so this build gives it to one chip and not the other rather than
inferring from the family — `TaitoX1017::register_of` says so in as many words.

**One reading carried past the page, in the direction its own infobox points.** nesdev gives mapper
552's PRG register four documented lines (bit 5 → A13 down to bit 2 → A16) and the board a 512 KiB
PRG capacity, which four lines cannot reach. `TaitoX1017::reversed_bank` continues the reversal to
A17 and A18, which is what Mesen does and by the same pattern (`TaitoX1017.h:98-105`); the four
documented lines are identical in the two implementations.

**One power-on value neither page gives**: the three PRG bank registers. They power on at zero,
which names bank 0, and the test says so in a comment rather than asserting a linear layout.

**One sweep helper had to grow**, and it is the only non-board change the step makes to shared
code: `cartridges::tests::image` put only the low byte of the mapper number in the header, so
**552** — the first number in `SUPPORTED_MAPPERS` above 255 — came out as mapper 40 and the four
sweeps failed on an unsupported number instead of on what they test. Byte 8's low nibble is mapper
bits 8-11; it now carries them.

### The measurement: **seven shapes, and the figure never moved**

The accurate decode costs `mmc5` **+7 456 756 instructions over 300 frames, +0.066 %** — three
and a bit times the owner's +0.02 % — and **nothing else moves at all**: no other ROM of PR-1's
thirteen is outside ±31 000 in any shape, and **every one of the five level-2 `core` figures is
+0 in every shape**. The whole of it is one symbol, `Cartridge::default_cpu_read`,
**283 356 728 → 290 813 484**.

| # | shape | `mmc5` | the rest |
| ---: | --- | ---: | --- |
| a | the decode in `SmallBoard::cpu_peek_override` | **+0.066 %** | all within ±23 000 |
| b | a, with `#[inline(never)]` on that forward | +0.690 % | set **+0.111 %**, `try_cpu_read` +78.7 M |
| c | a, with `#[inline(always)]` on `Cartridge::default_cpu_read` | **−0.065 %** | `m0` +0.022 %, `m1` +0.024 % — `Board::prg_ram_enabled` out of line |
| d | c, with `#[inline(always)]` on `Board::prg_ram_enabled` too | **−0.065 %** | `m0` +0.021 %, `m1` +0.024 % — `Cartridge::prg_ram_enabled` out of line instead |
| e | a, with `#[inline(never)]` on `SmallBoard::try_read_prg` | +0.066 % | all within ±45 000 |
| f | the decode moved to `try_read_prg` + `prg_ram_enabled` | +0.066 % | all within ±31 000 — **the shipped shape** |
| g | f, with all three links of the chain pinned | +0.144 % | `m0`/`m1`/`m4` +0.014 % to +0.016 % |
| h | f, with `#[inline(never)]` on `Board::prg_ram_offset` | +0.066 % | all within ±41 000 |

**The post-LTO dump names the instruction.** `fn.py` on `Cartridge::default_cpu_read` gives
**377** instructions at `8496c503`, **382** after step 1 and **391** after step 2, and the diff of
the last pair is one hoist:

    leaq 240(%rdi), %rax

— `&self.board`, needed only by the *cold* arms of the inlined `Board::prg_ram_enabled` jump
table, among them mapper 116's boxed `IrqBoard::Somari(Box<Somari>)` — moved **above** the
dispatch, where it runs on every call, with the cold `movq 240(%rdi), %rax` becoming
`movq (%rax), %rax`. The MMC5 is the one board that maps PRG-**RAM** into $8000-$DFFF, so
`read_rom_window` cannot answer and every CPU read of an MMC5 game reaches this function: one
`leaq` × 7.46 M reads.

**This step read that as a size threshold, and step 3 refuted it** — see step 3's own
re-measurement. The honest statement is the one the two numbers support: `default_cpu_read` has
two layouts on this ROM, 283 356 728 and 290 813 484, and the two Taito `SmallBoard` variants
select the dearer one whatever else the tree looks like.

**It is the same number, to the instruction, that this project has now measured three times in
both directions.** MF-6b steps 2 to 5 were charged `default_cpu_read` **+7 456 756** on `mmc5` for
`IrqBoard`'s sixth body; MF-6a3 step 4 recorded **−7 456 756** on the same ROM for that enum's
eighth and ninth ("which is *to the instruction* the figure MF-6b steps 2 to 5 charged it for the
sixth"); and this step is charged it again. So **the figure is a two-state oscillation of one
function on one ROM, and `8496c503` happens to sit on the favourable side of it** — not work the
Taito boards do. Seven shapes were measured against it and none of the four that changed anything
changed it in the right direction without taking the cost somewhere else.

### What the lot did with that: **`board-taito-x1` is parked**

The owner's rule 2 is unconditional — "Above it, it stays out of `default` and joins
`parked-boards`" — so that is what happened, and `parked-boards` has its first entry since MF-6b
wired the switch. The boards, their four numbers, their mechanisms and their tests are all
**built**, which is rule 1: nothing is dropped, and one command reaches them.

    cargo build -p zax-nes --features parked-boards      # or --features board-taito-x1

**Two things are worth putting in front of whoever revisits this**, because the decision is the
owner's and not this lot's:

1. **At level 2 the boards cost nothing.** With the feature **on**, all five `core` figures are
   **+0** and the only `total` above the baseline is `smb3` at **+0.003 %** — on the instrument
   that loads a 3D pack, which is the one R0's own campaign runs. The +0.066 % exists on one ROM
   of the no-pack instrument alone.
2. **The figure is a coin-flip this project has already seen land both ways.** A future lot that
   adds any body to the `Board::prg_ram_enabled` chain may hand it straight back, exactly as
   MF-6a3 step 4 did.

### The default build is `8496c503` plus step 1, on all three instruments

With the feature parked, `mf6c-2-x1parked` — the **shipped** configuration — is:

| ROM | `8496c503` | **step 2 default** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 311 891 | **3 308 300 715** | −11 176 | −0.000 % |
| `oamstr` | 3 318 460 965 | **3 318 451 868** | −9 097 | −0.000 % |
| `m1` | 4 885 949 431 | **4 885 938 375** | −11 056 | −0.000 % |
| `m4` | 5 472 514 995 | **5 472 500 157** | −14 838 | −0.000 % |
| `m0` | 5 520 458 691 | **5 520 451 673** | −7 018 | −0.000 % |
| `metroid` | 5 115 944 288 | 5 115 957 346 | +13 058 | +0.000 % |
| `smb1` | 5 996 381 185 | **5 996 352 423** | −28 762 | −0.000 % |
| `mm2` | 6 017 851 607 | **6 017 848 108** | −3 499 | −0.000 % |
| `z2` | 6 291 458 642 | 6 291 472 168 | +13 526 | +0.000 % |
| `readbuf` | 6 834 538 187 | 6 834 607 122 | **+68 935** | **+0.001 %** |
| `duelito` | 7 380 437 910 | 7 380 463 342 | +25 432 | +0.000 % |
| `smb3` | 7 949 470 334 | **7 949 437 675** | −32 659 | −0.000 % |
| `mmc5` | 11 279 383 781 | **11 279 365 244** | −18 537 | −0.000 % |
| **total** | **79 371 161 907** | **79 371 146 216** | **−15 691** | **−0.000 %** |

Worst ROM **+0.001 %**, nine of thirteen below the baseline. Level 2: **all five `core` figures
+0**, and the three `total` rises (`smb3` +0.024 %, `zelda2` +0.008 %, `smb1` +0.005 %) are rule
2's layout-noise case checked and not assumed — `sym6c.sh` at a 400 000 threshold names **three**
symbols over all five games and they are `getgeneric` (+5 993 152 on `smb3`, +2 217 912 on `smb1`,
+1 152 136 on `zelda2`) and `luaH_getshortstr` (−1 994 874), which is Lua's `ltable.c` and nothing
else.

The post-LTO dump is **step 1's dump**, which is the owner's "with a feature off, the build is the
same as without it" shown rather than asserted: `pins63.py` gives `Cpu::step` **3 626**,
`ppu_peek` **272**, `write_internal` **3 819** (step 1's figure, mapper 153's +4), `run_dots`
**1 907**, `catch_up_ppu` **188** and the three claimants **0 / 0 / 0**; `shape.py`'s five others
are identical to the baseline; `read_pattern_planes` **107** and `read_nametable_inner` **114**
unchanged; and `default_cpu_read` is **382**, step 1's count, not step 2's 391.

### The tests, 902 → **907**

| test | what it pins |
| --- | --- |
| `req_core_100_taito_x1_005_banks_prg_and_chr_with_the_2_kib_lsb_ignored` | the three PRG windows and the wired last bank, the odd PRG mirrors, **$05 and $04 selecting the same 2 KiB pair**, the four 1 KiB registers, a write through the **$7E7x** image, $7EF6's two values, and $7EF7 doing nothing |
| `req_core_100_taito_x1_005_internal_ram_answers_only_after_a3_at_7ef8` | the RAM off the bus at power-on, $A3 turning it on, the $7F80 mirror in both directions, **$6000, $7000 and $7EFA reading open bus**, $7EF9 as the register's mirror, any other value disabling, and REQ-CORE-095's slice being **128 bytes** with the written byte in it |
| `req_core_100_mapper_207_puts_chr_a17_on_ciram_a10` | each 2 KiB register's bit 7 driving its own pair of tables, all four combinations, $7EF6 ignored, **and mapper 80 answering `FOLLOW_MIRRORING` for the same write** |
| `req_core_100_mapper_82_and_552_are_the_same_board_with_the_bank_order_reversed` | **both orders in one test**: $14 → bank 5 on 082 and bank 10 on 552, $04 → bank 1 and bank 8, the three windows being the same three, and each number reporting itself |
| `req_core_100_taito_x1_017_inverts_chr_a12_and_gates_three_ram_windows` | the eight CHR windows in both inversion modes with the arrangement bit independent, the three magic values each opening **only its own** window, a wrong value being as good as none, $7400 undecoded, and the 5 KiB laid out $6000 / $6800 / $7000 |

**Three configurations green**, all with `cargo fmt --check` clean and clippy `-D warnings` clean
on `zax-nes` (default, `--features parked-boards`, `--no-default-features`,
`--no-default-features --features board-taito-x1`) and on `zax-harness`, **no new `allow` of any
kind**:

| configuration | `zax-nes` tests |
| --- | ---: |
| default (the feature parked) | **839** |
| `--features parked-boards` | **844** |
| `--no-default-features` | **783** |
| `--no-default-features --features board-taito-x1` | 808 |

839 + 5 = 844, the five tests being the step's own. With `zax-harness`, the default figure is
**907 passed / 0 failed / 8 ignored** in the `parked-boards` configuration and 902 in the default
one. The four sweeps held once `image` carried mapper bits 8-11 — and
`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`,
`mf_4c_the_write_latch_is_the_read_latch_minus_the_boards_that_wait_on_rd` and
`mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched` are the three that found the omission,
which is the fourth lot running in which that family of sweeps has caught a real mistake before
any measurement did.

[PROGRESS] MF-6c step 2/11 done: the Taito X1-005 and X1-017 on four numbers, 907 / 0 / 8, all
five level-2 cores +0 with the feature on **and** off — and `board-taito-x1` **parked**, on one
hoisted `leaq` worth +0.066 % on `mmc5` that seven shapes could not move and that this project has
measured in both directions.

## Step 3 — the serial EEPROM, mappers 157 and 159, at **+0.001 %** on the worst ROM

### What was built: one state machine, two chips, two numbers

`cartridges/mappers/eeprom24c0x.rs` is new, and it is the counterpart MR-1's shared table records
as missing ("no counterpart" against Mesen 2's `BaseEeprom24C0X.h`, `Eeprom24C01.h` and
`Eeprom24C02.h`). **One** type with an [`EepromKind`] parameter and not three, for the reason
`VrcIrq` is one type — and because mapper 157 needs both chips at once:

| what | where | the page's words |
| --- | --- | --- |
| START and STOP | `Eeprom24C0X::write` | "START is identified by a high to low transition of the SDA line while the clock SCL is *stable* in the high state" |
| the X24C02's chip address and its R/W bit | `chip_address_done` | "Upon a correct compare the X24C02 outputs an acknowledge on the SDA line"; "When set to one a read operation is selected, when set to zero a write operations is selected" |
| a "Current Address Read" fetching the byte at the acknowledge | the same method | "Upon receipt of the slave address with the R/W bit set to one, the X24C02 issues an acknowledge and transmits the eight bit word during the next eight clock cycles" |
| the X24C01's seven address bits and R/W in one byte | `address_bit` | "the master sends a start condition followed by a seven bit word address and a write bit" |
| the sequential read and write, on **both** chips | `clock_fall`, `Mode::WaitAck` | "a multi-byte read or write from EEPROM will increment the address accordingly" |
| mapper 157's two chips and their wire-AND | `EepromPort` | "an internal 256-byte serial EEPROM (X24C02)"; "an additional external 128-byte serial EEPROM (X24C01)"; "their outputs are ANDed together via open-drain circuit" |
| the external chip's clock on $8000-$8003 | `write_external_clock` | "External EEPROM Clock ($8000-$8003 write)", the diagram `xxxx Cxxx` |
| $800D's three bits | `write_control` | the diagram `RDC. ....` — bit 7 direction, bit 6 SDA for both, bit 5 the internal SCL |
| the read register | `BandaiFcg::try_read_prg` | the diagram `xxxE Bxxx` — bit 4 the EEPROMs, bit 3 the barcode scanner |
| the ports at $8000-$FFFF only | `BandaiFcg::register_floor` | "Mappers 157 and 159 do not need to support the FCG-1 and -2 and so should only mirror the ports across $8000-$FFFF" |
| mapper 157's CHR | `BandaiFcg::no_chr_banking` | "No CHR banking is available"; "8 KiB of unbanked CHR-RAM" |
| the bytes on the battery | `registry::board_memory_sizes`, arm `157 \| 159` | §4.6's Namco 163 layout: the internal chip first, the external one behind it |

**The one behavioural claim of the step, quoted verbatim into the code and into the test**, from
nesdev ("INES Mapper 159"):

> Bandai's developers consistently thought the X24C01 was a little-endian device, and their code
> clocks in address and data bytes assuming this. However, it's actually a big-endian device, and
> a multi-byte read or write from EEPROM will increment the address accordingly.

So **both chips are clocked most-significant bit first** here, the X24C01 included. Mesen 2 clocks
its X24C01 LSB first (`Eeprom24C01.h:15`), which is the model the page says Bandai's developers
*had*; AMD-061 §1 makes the page prevail. The consequence is the one the page draws and the test
pins: a single byte still round-trips, because the game's bit order cancels on the way out as it
did on the way in, and a multi-byte transfer walks the **hardware's** address.

**Three more divergences from Mesen 2, all three settled for the page by AMD-061 §1.**

1. **Sequential transfers work on the X24C01 too.** Mesen sends that part to `Idle` after one byte
   in both directions (`Eeprom24C01.h:77-82` and `:116`), the first under a comment — "We expected
   an ack, but received something else" — that its own condition contradicts. The page's sentence
   above gives the X24C01 multi-byte reads *and* writes.
2. **The Datach's second chip is declared by byte 10 and by nothing else.** Mesen gives it to any
   dump that is not NES 2.0 (`BandaiFcg.h:55`), which hands a phantom chip to the six Datach games
   that have none — and a phantom chip is not harmless here, because its output is wire-ANDed with
   the real one and $8000-$8003 would clock it. `registry::datach_has_external_eeprom` tests byte
   10 alone.
3. **Mapper 16 is untouched.** Mesen connects a 24C02 to mapper 16 as well, on submapper 0 or on
   submapper 5 with 256 bytes of save RAM. REQ-CORE-100 makes that number's EEPROM P3 and the
   brief's scope is 157 and 159, so mapper 16's board comes out of this step byte for byte as step
   1 left it — `mf_6c_an_ordinary_mapper_16_image_is_unaffected_by_153` still passes unchanged.

**Two gaps recorded rather than hidden**, both on `EepromPort`'s own note:

* **the Datach's barcode scanner.** An input device, and this build has no host surface for one —
  the gap MR-30-4 records for the UNROM 512's flash, in the same words. Bit 3 of the read register
  is 0, so a Datach game sees no scan and plays on.
* **the six open-bus bits of that register read 0.** `Mapper::try_read_prg` answers a whole byte
  or nothing and `Mapper::cpu_peek_override` can only say "all open bus"; **no hook in this
  architecture drives two bits and releases six**. Routing the read through `cpu_peek_override`
  for the sake of `Some(None)` was not available either: step 2 measured one more body in that
  dispatch at +0.066 % on `mmc5`.

**One question filed: Q-134.** $800D bit 7 is "Both EEPROM's Direction bit (1=Enable Read)" on the
page, and on the hardware it releases the master's SDA driver so the chip can answer. This build
**decodes it and does not let it gate SDA**: the state machine already knows which way a byte is
going, because the protocol told it, and forcing SDA high would break a master that sets the bit
late. Mesen ignores the bit for the same reason.
`q_134_the_eeprom_direction_bit_is_decoded_and_does_not_gate_sda` drives the same transfer twice,
once with the bit clear throughout and once with it set throughout, and requires the same byte in
the EEPROM both times — the shape
`q_123_vrc6_keeps_its_nametables_on_ciram_when_n_asks_for_chr_rom` gave the VRC6's ignored bit.

**One header field was exposed**, and it is the pure addition `byte6` is the precedent for:
`InesHeader::byte10`. `prg_ram_size` collapses byte 10's two nibbles into one total **with a
floor** — `(volatile + non_volatile).max(DEFAULT_PRG_RAM_SIZE)`, so a Datach dump declaring 128
bytes of PRG-NVRAM comes out as 8 192 — where the page needs the raw field: "byte 10 value 0x10"
against "byte 10 value 0x00".

### The shape: the read goes through `try_read_prg`, which step 2 paid to learn

Mappers 157 and 159 have a **readable** register at $6000-$7FFF, which mapper 16's board did not.
Step 2's whole performance story was that `Board::cpu_read_override` is `#[inline(always)]` and
one more distinct body in it costs `mmc5` +0.066 %, where one more body in `Board::try_read_prg`
— also `#[inline(always)]` — costs nothing. So the EEPROM's data out is answered from
`BandaiFcg::try_read_prg`, and the measurement below is what that choice bought.

The EEPROM itself is a `None` on mappers 16 and 153 and adds **no `Board` variant, no `IrqBoard`
arm and no dispatch arm but that one**. Its bytes are in PRG-RAM with
`BandaiFcg::prg_ram_enabled` answering `false` on both numbers, so nothing the CPU can address
lives there: the bytes are reached only through the serial protocol.

### The measurement: free on both instruments

`callgrind-diff.py`, against `mf6c-0-8496c503`:

| ROM | board | `8496c503` | **step 3** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 308 311 891 | 3 308 327 534 | +15 643 | +0.000 % |
| `oamstr` | NROM | 3 318 460 965 | 3 318 461 730 | +765 | +0.000 % |
| `m1` | MMC1 | 4 885 949 431 | **4 885 917 630** | −31 801 | −0.001 % |
| `m4` | MMC3 | 5 472 514 995 | **5 472 488 535** | −26 460 | −0.000 % |
| `m0` | NROM | 5 520 458 691 | **5 520 453 899** | −4 792 | −0.000 % |
| `metroid` | MMC1 | 5 115 944 288 | 5 115 959 334 | +15 046 | +0.000 % |
| `smb1` | NROM | 5 996 381 185 | 5 996 385 963 | +4 778 | +0.000 % |
| `mm2` | MMC1 | 6 017 851 607 | **6 017 830 827** | −20 780 | −0.000 % |
| `z2` | MMC1 | 6 291 458 642 | 6 291 464 491 | +5 849 | +0.000 % |
| `readbuf` | CNROM | 6 834 538 187 | 6 834 568 809 | +30 622 | +0.000 % |
| `duelito` | NROM | 7 380 437 910 | 7 380 498 710 | **+60 800** | **+0.001 %** |
| `smb3` | MMC3 | 7 949 470 334 | **7 949 451 002** | −19 332 | −0.000 % |
| `mmc5` | MMC5 | 11 279 383 781 | **11 279 342 878** | −40 903 | −0.000 % |
| **total** | | **79 371 161 907** | **79 371 151 342** | **−10 565** | **−0.000 %** |

Worst ROM `duelito` at **+0.001 %**, one twentieth of the owner's tolerance and inside the ±57 k
`luaH_getshortstr` floor; the set total is **below** the baseline. **`mmc5` is −40 903**, which is
the step's receipt that the `try_read_prg` route was the right one.

Level 2, `l2-diff.py --bar`: **all five `core` figures +0** again. The three `total` rises
(`smb3` +0.014 %, `smb1` +0.004 %, `mm2` +0.000 %) are rule 2's layout-noise case checked and not
assumed — `sym6c.sh` at 400 000 names four symbols over all five games and they are `getgeneric`
(+3 745 480 on `smb3`, +2 217 944 on `smb1`), `luaH_getshortstr` (−2 013 943 and −1 033 758) and
`__memcpy_avx_unaligned_erms` (−403 395). Lua's `ltable.c` and `memcpy`, nothing else.

### Instrument 3, and the two pinned symbols that moved

| symbol | `8496c503` | **step 3** |
| --- | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 3 626 | **3 626** |
| `Board::ppu_peek` | 272 | 290 |
| `NesBus::write_internal` | 3 815 | 3 828 |
| `Ppu::run_dots` | 1 907 | 1 919 |
| `NesBus::catch_up_ppu` | 188 | **188** |
| `Ppu2C02::read_pattern_planes` | 107 | **107** |
| `Ppu2C02::read_nametable_inner` | 114 | **114** |

`shape.py`'s five others — `resolve` 1 948, `branch` 457, `read_cycle` 181, `tick` 82,
`read_internal` 175 — are identical to the instruction, and so are the three claimants and
`catch_up_ppu`. **`Board::ppu_peek` +18 and `Ppu::run_dots` +12 are reported and not waved away**:
they are the mapper-157 `no_chr_banking` test inside `BandaiFcg::update_banks`'s neighbourhood and
the register decode's extra arms, and what says they cost nothing is the dynamic column above —
all five level-2 `core` figures at **+0** and `readbuf`, the CNROM whose whole run is pattern
fetches, at +0.000 %.

### **Step 2's size-threshold explanation was wrong, and this step is what refuted it**

Step 3's `Cartridge::default_cpu_read` is **425** instructions — 377 at the baseline, 382 after
step 1, 391 after step 2, 425 now — and `mmc5` is **−40 903, below the baseline**. So "past about
390 LLVM hoists the `leaq`" is not the rule; that function's dearer layout is not a function of
its size.

So the step re-measured `board-taito-x1` on this tree, with the feature put back into `default`
(label `mf6c-3-x1again`), and the answer is **+7 456 756 on `mmc5`, +0.066 %** — the same number
to the instruction, for the eighth time, on a tree where the function it lands in has grown by 48
instructions and every other ROM of the thirteen is within ±60 000. **The park stands**, and the
claim is now the narrow one the measurements support: `default_cpu_read` has two layouts on that
ROM, 283 356 728 and 290 813 484, and the two Taito `SmallBoard` variants select the dearer one
whatever else the tree looks like. Step 2's write-up and `BOARD-FEATURES.md` are corrected to say
that instead.

### The tests, 902 → **906** in the default build

| test | what it pins |
| --- | --- |
| `req_core_100_the_x24c01_is_big_endian_and_its_address_increments_accordingly` | a byte round-tripping; **the bit order**, by requiring the little-endian reading of the same seven address bits ($05 → $50) to hold nothing; a three-byte sequential write landing at $10, $11, $12; a two-byte sequential read agreeing with it; the 128-byte wrap from $7F to $00; REQ-CORE-095's slice being the EEPROM's own 128 bytes; and $6000-$7FFF not being RAM |
| `req_core_100_the_datach_has_an_internal_24c02_and_an_external_24c01` | the 256 + 128 battery slice, the internal chip's chip-address byte and 8-bit word address, a sequential write and read-back, a **wrong chip address selecting nobody**, "No CHR banking is available" against writes to all eight registers, and the PRG banking and mirroring still being submapper 5's |
| `req_core_100_the_datach_external_eeprom_is_clocked_from_8000_and_wire_anded` | the external chip driven with SCL from $8000 bit 3 and SDA from $800D bit 6 — **lowering SCL before changing SDA**, because the two lines are in two registers and cannot move together — the byte landing behind the internal 256 and not in them, and the **wire-AND**: the register reads 0 while the external chip holds SDA low and 1 one clock fall later |
| `q_134_the_eeprom_direction_bit_is_decoded_and_does_not_gate_sda` | the same transfer with $800D bit 7 clear throughout and set throughout, into two identical cartridges, giving the same byte |

The test file gained an `i2c` module that drives the two-wire bus from the CPU side, so the three
EEPROM tests speak the protocol rather than poking state.

**Three configurations green**: default **906 passed / 0 failed / 8 ignored** with `zax-harness`
(843 for `zax-nes` alone), `--features parked-boards` **848** for `zax-nes`,
`--no-default-features` **783**. `cargo fmt --check` clean; clippy `-D warnings` on `zax-nes` in
all three configurations and on `zax-harness`, clean, **no new `allow` of any kind**.
`board-eeprom` is inside the tolerance by a factor of twenty and goes in `default`.

[PROGRESS] MF-6c step 3/11 done: `Eeprom24C0X` and mappers 157 and 159, 906 / 0 / 8, all five
level-2 cores +0 and `duelito` +0.001 % the worst of thirteen with `mmc5` −40 903 — and step 2's
park re-measured at the same +7 456 756 on a tree 48 instructions larger, which refutes that
step's size-threshold reading and leaves the park standing.

## Step 4 — the Sunsoft-4, **Q-123's ceiling lifted**, and what parks a board named at last

### What was built

nesdev ("INES Mapper 068"), six registers on the high nibble of $8000-$FFFF, and the code quotes
the page on each:

| what | where |
| --- | --- |
| four **2 KiB** CHR banks at $0000, $0800, $1000 and $1800 | `Sunsoft4::update_banks` |
| the 16 KiB window of `$F000`'s `...E BBBB`, with the image's last bank wired at $C000 | the same method |
| `$F000` bit 4, the PRG-RAM enable, **off at power-on** | `Sunsoft4::prg_ram_enabled` |
| `$E000` bits 1-0, the four mirroring modes, through `set_mirroring` and not through a page override | `Sunsoft4::write_x1_register`'s `0xE000` arm |
| `$C000` and `$D000`, the two nametable banks, "7 bits used; D7 ignored" with CHR A17 wired high | the same arm, `value \| 0x80` |
| **`$E000` bit 4**, "CIRAM / CHR-ROM Select for Nametables" | `Sunsoft4::read_nametable` |
| which register serves which logical table, by the mirroring mode | `Sunsoft4::nametable_register` |

**Two Cargo features, because the owner's rule 1 asks for exactly that.** §5 MF-6c step 4 of the
inventory offered this lot a choice between finding a shape MF-5a had not tried and decoding
`$E000` bit 4 and ignoring it, with "the second [as] the expected outcome". The owner's rule
replaces that — "A costly *mechanism* of a board (Sunsoft-4's CHR-ROM nametables through
`read_nametable` …) is implemented too, behind its own feature if it costs more than the board
itself" — so the mode is **built**, as `board-68-chr-nametables`, and the measurement decides only
where its feature lives. The bit is **decoded and kept in the savestate either way**, so the
feature switches the serving and not the decode, which
`mf_6c_sunsoft_4_decodes_e000_bit_4_in_both_configurations` pins by loading one configuration's
state into the other's cartridge.

**One gap recorded: *Nantettatte!! Baseball*.** Its second PRG-ROM sits on a subboard, selected by
a bit of `$F000` and gated by a licensing timer the board starts on a write to $6000-$7FFF. "A
second ROM image has no path through `Cartridge::create` and no spec text asks for one" — §5 MF-6c
step 4's own words — so `$F000` is a 4-bit bank here, which is what the page's `...E BBBB` diagram
gives it, and the external ROM joins MR-30-4's UNROM 512 flash and step 3's Datach barcode scanner
as a recorded gap. Mesen 2 reads bit 3 as a chip select and holds a 105 000-cycle timer
(`Sunsoft4.h:120-139`); on a single-chip dump the two readings agree, because bank 8 and above wrap
into the same eight banks.

### **Finding 1: what parks a board is the eleventh `SmallBoard` variant**

The board went into [`SmallBoard`] first, which is where its own page sends it — "IRQ: None", that
enum's entry condition — with **one** new variant and **one** forwarded method
(`prg_ram_enabled`, for `$F000` bit 4). Measured (`mf6c-4-sunsoft4`):

| | `8496c503` | **68 in `SmallBoard`** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `mmc5` | 11 279 383 781 | 11 286 797 392 | **+7 413 611** | **+0.066 %** |
| every other ROM | | | within **±89 000** | ≤ +0.001 % |
| set total | 79 371 161 907 | 79 378 790 369 | +7 628 462 | +0.010 % |

and the symbol diff names one function: `Cartridge::default_cpu_read`, **283 356 728 →
290 813 484, +7 456 756** — step 2's figure, to the instruction, for the **fourth** time in this
lot and the **sixth** in this project.

**That settles what step 2 could not.** Step 2 had two new variants and three forwards and blamed
the forwards, then the function's size; step 3 refuted the size. This step has **one** variant and
a *different* single forward, and gets the identical number. So the trigger is the variant:

> **The eleventh `SmallBoard` variant flips `Cartridge::default_cpu_read` into the dearer of its
> two layouts, at +0.066 % on `mmc5` and nothing anywhere else.** `SmallBoard` has been the
> recommended home for a register-decode board since MF-6a step 2, and that advice has run out.

### **Finding 2: `IrqBoard` is the cheap home, and it costs one PPU horizon**

[`IrqBoard`] was already nine variants deep with the same function in its *cheap* layout, so the
board was moved there — `irq_asserted` answering `false` through its own `Mapper` default, which
costs that enum's inner match one more arm of the body it already has — and three forwards widened
(`prg_ram_enabled`, `nametable_page`, and `read_nametable` behind the second feature). Measured
(`mf6c-4-s4irqbrd`), **with both features on**:

| ROM | `8496c503` | **68 in `IrqBoard`** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 311 891 | 3 308 335 323 | +23 432 | +0.001 % |
| `oamstr` | 3 318 460 965 | **3 318 459 686** | −1 279 | −0.000 % |
| `m1` | 4 885 949 431 | 4 885 952 434 | +3 003 | +0.000 % |
| `m4` | 5 472 514 995 | 5 472 518 088 | +3 093 | +0.000 % |
| `m0` | 5 520 458 691 | **5 520 455 246** | −3 445 | −0.000 % |
| `metroid` | 5 115 944 288 | **5 115 937 171** | −7 117 | −0.000 % |
| `smb1` | 5 996 381 185 | **5 996 355 737** | −25 448 | −0.000 % |
| `mm2` | 6 017 851 607 | 6 017 920 165 | **+68 558** | **+0.001 %** |
| `z2` | 6 291 458 642 | **6 291 437 125** | −21 517 | −0.000 % |
| `readbuf` | 6 834 538 187 | 6 834 561 249 | +23 062 | +0.000 % |
| `duelito` | 7 380 437 910 | 7 380 457 425 | +19 515 | +0.000 % |
| `smb3` | 7 949 470 334 | **7 949 447 453** | −22 881 | −0.000 % |
| `mmc5` | 11 279 383 781 | **11 279 348 656** | **−35 125** | −0.000 % |
| **total** | **79 371 161 907** | **79 371 185 758** | **+23 851** | **+0.000 %** |

**`mmc5` is −35 125 instead of +7 413 611**, the worst ROM of the thirteen is `mm2` at +0.001 %,
and level 2's verdict line is the one this lot has not seen before: **"at or below the baseline on
every game, total and core"** — all five `core` figures +0 and all five `total` figures below,
`smb3` −1 353 944 and `smb1` −820 240.

The price is this variant's `ppu_irq_horizon` `None`: a Sunsoft-4 could have let the PPU run
behind the CPU and gives that up, which is §4.10's rule and exactly the trade the TQROM and the
two MMC3 multicarts already make for the same reason.

### **Finding 3: Q-123's ceiling does not reproduce, and the third body is free**

The mechanism the inventory called "the hook a sub-lot should be most afraid of" turns out to cost
nothing on this tree, and the step measured it twice.

`Ppu2C02::read_nametable_inner`, with `fn.py`:

| | instructions | shape |
| --- | ---: | --- |
| MF-5a's baseline, seven lots ago | 132 | bitset, two arm bodies |
| MF-5a's best third-arm shape | 109 | jump table, the MMC5's arm out of line |
| **`8496c503`, this lot's baseline** | **114** | bitset, two arm bodies |
| **with the third body** | **75** | the body leaves by a tail call |

and the dynamic cost of that, measured as `board-68-chr-nametables` **against the same tree with
only `board-68`** (`mf6c-4-s4ntable` against `mf6c-4-sunsoft4`): the set total moves
**+17 007 instructions, +0.000 %**, the worst ROM is `metroid` at +0.001 %, and **`readbuf` — the
CNROM whose whole run is fetches — is −78 628**.

So the third arm is the 75-instruction shape, which is MF-5a's own second row ("the dispatch leaves
by a tail call") and not its third; the dispatch is reached only behind
`page == SERVED_BY_CARTRIDGE`, so a board that does not serve its own nametables never calls it.
**Q-123's "132 → 109 for every game in the library" is not the price on this tree**, and the step
re-measured it rather than quoting it, which is what §4.5 asked the next lot to do before adding a
fourth board. `Ppu2C02::read_pattern_planes` is **107**, unchanged, which is MF-2's shield holding.

### Instrument 3

| symbol | `8496c503` | **step 4** |
| --- | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 3 626 | **3 626** |
| `Board::ppu_peek` | 272 | 290 |
| `NesBus::write_internal` | 3 815 | **3 798** |
| `Ppu::run_dots` | 1 907 | 1 919 |
| `NesBus::catch_up_ppu` | 188 | **188** |
| `Ppu2C02::read_pattern_planes` | 107 | **107** |
| `Ppu2C02::read_nametable_inner` | 114 | 75 |

`shape.py`'s five others are identical to the instruction. `write_internal` is **below** the
baseline; `ppu_peek` +18 and `run_dots` +12 are step 3's figures carried forward, unchanged by this
step, and the dynamic column above is what prices them at nothing.

### One existing test had to move a number, and Q-130 question 2 predicted it

`req_core_102_an_unimplemented_mapper_is_refused_by_name` swept **6, 68, 177 and 200**, and 68 is
now implemented. **That is the second time a lot has had to edit that list** — MF-5a step 4 moved
19 out when it built the Namco 163 — and it is exactly what Q-130 question 2 says: "all four are
real boards Mesen builds, so the test holds no number guaranteed never to become supported … the
criterion wants a number that is nobody's board."

68's replacement is the first number in this project with that guarantee written down: **186**, the
Study Box, from §3.5's list of the numbers Mesen 2 has a `case` for and `break`s out of — "They are
**not** candidates for any sub-lot" — and which §6 item 2 names because "Mesen loads [it] as a BIOS
and not as a ROM". A number that is not a cartridge board cannot be claimed by a later mapper lot.
The test's comment says so, and references Q-130 question 2.

### The tests, 906 → **909**

| test | what it pins |
| --- | --- |
| `req_core_100_sunsoft_4_banks_four_2_kib_chr_windows_and_one_prg_window` | the power-on windows, the 4-bit bank, the four 2 KiB CHR registers on their own quarters of $8000-$BFFF, all four mirroring modes, `FOLLOW_MIRRORING` with bit 4 clear, and the PRG-RAM **off at power-on** then gated by $F000 bit 4 in both directions |
| `req_core_100_sunsoft_4_serves_its_nametables_from_chr_rom_on_e000_bit_4` | `SERVED_BY_CARTRIDGE` with bit 4 set; the two 1 KiB banks with bit 7 forced; **which register serves which table in all four mirroring modes**; the offset inside the bank; and the nametables going back to CIRAM without the registers moving |
| `mf_6c_sunsoft_4_decodes_e000_bit_4_in_both_configurations` | a savestate written with the mode on loading into a fresh cartridge, the derived pages coming back, and the **feature** and not the decode deciding what is served |

**Four configurations green**, `cargo fmt --check` clean and clippy `-D warnings` clean on
`zax-nes` in all four and on `zax-harness`, **no new `allow` of any kind**:

| configuration | `zax-nes` tests |
| --- | ---: |
| default (both mapper-68 features on) | **846** |
| `--features parked-boards` | **851** |
| `--no-default-features` | **783** |
| `--no-default-features --features board-68` | **785** |

With `zax-harness` the default figure is **909 passed / 0 failed / 8 ignored**. Both `board-68` and
`board-68-chr-nametables` are inside the tolerance by a factor of twenty and go in `default`.

### What step 5 was inserted to try, and it is worth 13 titles

**Finding 2 looks as though it un-parks `board-taito-x1`.** The two Taito boards are parked on
exactly the figure this step cured by moving one board out of `SmallBoard`, and both of their pages
say "IRQ: None" the same way the Sunsoft-4's does, so [`IrqBoard`] should take them on the same
terms — `try_read_prg`, `prg_ram_enabled` and `nametable_page` are all forwards that enum already
has or can widen, and the price is the same `ppu_irq_horizon`. **Step 5 is inserted to move them
and re-measure**; if it holds, thirteen licensed titles come back into `default` and
`parked-boards` is empty again. (It does not. See step 5.)

[`SmallBoard`]: ../../rust/crates/zax-nes/src/cartridges/mappers/small_boards.rs
[`IrqBoard`]: ../../rust/crates/zax-nes/src/cartridges/mappers/irq_boards.rs

[PROGRESS] MF-6c step 4/11 done: the Sunsoft-4 and its CHR-ROM nametables both in `default`,
909 / 0 / 8, level 2 **at or below the baseline on every game, total and core** — and two findings:
the eleventh `SmallBoard` variant is what parks a board, and Q-123's 132 → 109 ceiling does not
reproduce (114 → 75, set +17 007, `readbuf` −78 628).

## Step 5 — the un-parking attempt, and it **fails**: both enums have a limit

### What was tried

Exactly what step 4 recommended. The two Taito X1 boards came out of [`SmallBoard`] and went into
[`IrqBoard`] beside the Sunsoft-4, on the same terms:

* `try_read_prg` and `prg_ram_enabled` forwarded from `IrqBoard` instead, the first
  `#[inline(never)]` as it was in `SmallBoard`;
* `nametable_page`'s existing `IrqBoard` forward widened to cover mapper 207;
* `board-taito-x1` put back into `default` and `parked-boards` emptied;
* the four `SmallBoard` forwards step 2 had added taken back out, leaving that enum at the ten
  variants and the twelve methods MF-6a left it with.

The suites agree it is correct: **914 passed / 0 failed / 8 ignored** with `zax-harness`, every
test of the five Taito ones included, `cargo fmt --check` clean and clippy `-D warnings` clean in
all three configurations.

### What it measures: **two** units of the same figure

`callgrind-diff.py`, label `mf6c-5-x1irqbrd`, against `mf6c-0-8496c503`:

| ROM | `8496c503` | **step 5's attempt** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 311 891 | **3 308 295 191** | −16 700 | −0.001 % |
| `oamstr` | 3 318 460 965 | **3 318 450 163** | −10 802 | −0.000 % |
| `m1` | 4 885 949 431 | **4 885 930 601** | −18 830 | −0.000 % |
| `m4` | 5 472 514 995 | **5 472 491 025** | −23 970 | −0.000 % |
| `m0` | 5 520 458 691 | **5 520 447 707** | −10 984 | −0.000 % |
| `metroid` | 5 115 944 288 | **5 115 941 237** | −3 051 | −0.000 % |
| `smb1` | 5 996 381 185 | **5 996 353 716** | −27 469 | −0.000 % |
| `mm2` | 6 017 851 607 | **6 017 827 617** | −23 990 | −0.000 % |
| `z2` | 6 291 458 642 | **6 291 456 216** | −2 426 | −0.000 % |
| `readbuf` | 6 834 538 187 | 6 834 566 283 | +28 096 | +0.000 % |
| `duelito` | 7 380 437 910 | **7 380 410 865** | −27 045 | −0.000 % |
| `smb3` | 7 949 470 334 | **7 949 442 480** | −27 854 | −0.000 % |
| `mmc5` | 11 279 383 781 | 11 294 270 253 | **+14 886 472** | **+0.132 %** |
| **total** | **79 371 161 907** | **79 385 883 354** | +14 721 447 | +0.019 % |

**Eleven of thirteen below the baseline, and `mmc5` at +0.132 %** — twice the figure the park was
taken on. One symbol, as always: `Cartridge::default_cpu_read`, **283 356 728 → 298 270 240,
+14 913 512**, which is **2 × 7 456 756 to the instruction**.

### The finding this closes

> **`Cartridge::default_cpu_read`'s cost on `mmc5` is quantised in units of 7 456 756
> instructions, and a variant past each enum's own limit buys one unit.** `IrqBoard` had room for
> the tenth (the Sunsoft-4, free, step 4) and none for the eleventh and twelfth (the two Taito
> boards, two units). `SmallBoard` had none for the eleventh (one unit, steps 2 and 4).

So the recommendation step 4 wrote in good faith is **refuted**, and the attempt was reverted: the
tree at the end of this step is the tree step 4 committed, with the Taito boards back in
`SmallBoard` and `board-taito-x1` back in `parked-boards`. The only change this step leaves behind
is **the knowledge, written where the next lot will read it** — [`SmallBoard`]'s own doc comment
now carries all four measurements under the heading "This enum is full, and MF-6c measured where
its limit is", and [`IrqBoard::Sunsoft4`]'s says the same of its own enum.

**What that means for a later lot, and it is not a small thing**: a board that is a register decode
and nothing else has **no free home left**. Both shared variants are at their limit, a `Board`
variant of its own costs the +0.080 % MF-6a3 step 1 measured on `Board::expansion_audio_output`'s
jump table, and the figure this step quantified lives in the inlined `Board::prg_ram_enabled`
dispatch inside `Cartridge::default_cpu_read`. **The next lot that needs one has to redesign that
read path rather than pick an enum** — for instance by giving `Cartridge` a load-time cache of the
$6000-$7FFF decode the way `rom_windows` and `nametable_pages` are already cached, which is the one
shape none of this lot's eleven measurements tried.

### What this step changes, and what it does not

**No code behaviour changes and no measurement of step 4's tree is invalidated.** `git checkout`
restored the four files, so the committed tree is byte-identical to step 4's on
`small_boards.rs`, `irq_boards.rs` (but for the new note), `registry.rs` and `Cargo.toml`; the
suites are back to step 4's **909 / 0 / 8**, `--features parked-boards` to **851** and
`--no-default-features` to **783**. There is no new measurement to report for the shipped tree,
because the shipped tree is the one step 4 already measured.

[PROGRESS] MF-6c step 5/11 done: the un-parking step 4 recommended, tried and **refuted** —
`IrqBoard` at twelve variants puts `mmc5` at +0.132 %, two units of the same 7 456 756, so
`board-taito-x1` stays parked; both enums' limits are now measured and written into their own doc
comments.

---

## Steps 6 to 11 — the plan as the step-5 finding leaves it

**This is a plan revision and not a step**, written because step 5's measurement invalidates the
shape §5 MF-6c assumed for the four boards that are left, and a successor resuming this lot should
start from a correct plan rather than rediscover the wall. No code is involved.

### The wall

Every one of the four remaining boards is, structurally, "a register decode and nothing else":

| step | board | what it needs from a shared variant |
| ---: | --- | --- |
| 6 | **77**, Irem LROG017 | `ppu_peek` and `ppu_write` for the CHR-RAM at $0800-$1FFF (both already forwarded by either enum); its CHR-ROM half is ordinary banking |
| 7 | **72, 92, 86**, Jaleco JF-17 / JF-19 / JF-13 | `write_register` and `intercept_cpu_write` for 86's $6000-$6FFF; nothing else |
| 8 | **96**, Oeka Kids | `on_ppu_address_bus` — which `IrqBoard` forwards and `SmallBoard` deliberately does not |
| 9 | **111**, Cheapocabra | `nametable_page`, `ppu_peek`, `ppu_write` |

and step 5 measured that **neither shared enum has room**: one more variant in either is
+7 456 756 on `mmc5exram`, +0.066 %, against a tolerance of +0.02 %. So the plan §5 wrote — "add
it to `SmallBoard`, it is free" — is no longer available to any of them, and the four steps would
produce four parked features if run as written.

### The three shapes a successor should measure, in this order

1. **One new `Board` variant holding all four boards** — a third shared enum beside `SmallBoard`
   and `IrqBoard`, say `DecodeBoard`, with 77, 72/92/86, 96 and 111 inside it. This is **one**
   discriminant for all eight numbers, and the figure to beat is MF-6a3 step 1's: **two** new
   `Board` discriminants cost `mmc5` +8 994 278 (+0.080 %), one taken `jmp` per expansion-audio
   sample, because `Board::expansion_audio_output`'s 35-entry jump table grew by two and LLVM
   moved the fall-through off the MMC5's arm. **One** entry has never been measured on its own and
   may well be free — LLVM either moves that fall-through or it does not. This is the cheapest
   experiment available and it decides all four steps at once; it is also the only one that keeps
   `SmallBoard`'s and `IrqBoard`'s inner matches where they are.
2. **The `$6000-$7FFF` read path cached on `Cartridge`**, the way `rom_windows` and
   `nametable_pages` already are. Every one of this lot's eleven measurements landed in the
   inlined `Board::prg_ram_enabled` dispatch inside `Cartridge::default_cpu_read`; a load-time
   cache of "does this board decode $6000-$7FFF, and how" would take that dispatch out of the
   function entirely and is the one shape nothing has tried. It is the structural fix and it would
   lift the limit for every later lot — and it touches a function on the MMC5's every CPU read, so
   it is also the riskiest and wants its own sub-lot rather than a step inside this one.
3. **Four parked features**, one per step, if neither of the above holds. This is the owner's
   rule 2 applied and it is not a failure — the boards are all *built*, every test passes, and one
   `--features` flag reaches them — but it leaves eight numbers out of the default build and
   should be the answer only after 1 and 2 have been measured and written down.

### What has not changed about steps 6 to 11

The board designs §5 MF-6c gives are unaffected: 77's split `chr_view` (and its
**`read_pattern_planes` 107**, which this lot has held at every step), the three-way Jaleco
parameter with `expansion_audio_output` pinned at `None` on §4.8's grounds, 96's
`watches_ppu_bus_without_read` and MR-1's open MMC2 question, and 111's `nametable_page` — which
step 4's finding 3 now makes easier, since a **fourth** `read_nametable` body is measured and not
assumed. Steps 10 and 11 are unchanged.

## Step 6 — mapper 77, and **shape 1 holds: one new `Board` discriminant is free**

### What the step had to settle first

R0's decision after the plan revision is an experiment before a board: "**Measure shape 1 first**:
one new shared `Board` variant (`DecodeBoard`) holding 77, 72/92/86, 96 and 111. If it is within
the tolerance for all of them, ship them in `default`." So this step builds the enum with its
first member and prices **the discriminant**, which is the figure the three steps after it inherit.

The figure to beat was MF-6a3 step 1's, for **two** new `Board` discriminants: `mmc5`
**+8 994 278, +0.080 %**, one taken `jmp` per expansion-audio sample because the jump table of
`Board::expansion_audio_output` grew by two entries and LLVM moved the fall-through off the MMC5's
arm. **One** entry had never been measured on its own.

**It is free.** The whole of instrument 2 is below, and the headline is that `mmc5` — the ROM every
one of this lot's eleven earlier measurements landed on — is **−40 814**, the set total is
**−136 318**, and the worst of the thirteen is `metroid` at **+12 780, +0.000 %**: one
seventeen-hundredth of the owner's +0.02 %.

So `DecodeBoard` is the home of the four remaining boards, [`SmallBoard`] and [`IrqBoard`] stay at
the variant counts step 5 left them with, and their doc comments now say so.

### What was built: mapper 77, the Irem LROG017

nesdev ("INES Mapper 077"), one register and a memory map, and the code quotes the page on each:

| what | where | the page's words |
| --- | --- | --- |
| the one register, PRG in the low nibble and CHR in the high one | `IremLrog017::update_banks` | "`$8000-FFFF: [CCCC PPPP]`", `C` a "2k CHR reg @ $0000", `P` a "32k PRG reg @ $8000" |
| the write ANDed with the ROM's own byte | `registry::has_bus_conflicts`, arm `77` | "Bus conflicts: yes" |
| the CHR-ROM chip at $0000-$07FF and the CHR-RAM chip above it | `IremLrog017::ppu_peek`, `ppu_write` | "$0000-$07FF swappable CHR-ROM", "$0800-$1FFF fixed CHR-RAM" |
| the RAM chip's 6 KiB, which no header field can state | `registry::board_memory_sizes`, arm `77` | the implementation note, below |
| the four nametables | `IremLrog017::reset`, `NametableMirroring::FourScreen` | the map's "$2000-$27FF" and "$2800-$2FFF" as two distinct regions |
| the linear order inside the RAM chip | `IremLrog017::ram_offset` | "you do not need to care about the specific order of the CHR-RAM banks" |

**The split CHR is the whole board**, and it is the TQROM's problem again: a `ChrView` is one
slice plus eight window offsets and has no way to say "this window is the other chip". So the
answer is the TQROM's, in the same words — `chr_view` and `chr_peek_view` both `None`, and
`ppu_peek` and `ppu_write` serve the two chips with `#[inline(never)]` on each, so that the arms
they put in `Board::ppu_peek` and in `NesBus::write_internal` are a `callq` and not a body.

**`Ppu2C02::read_pattern_planes` is 107, unchanged**, which was §5 MF-6c step 5's
*stop-the-lot condition* for this board and the one figure the inventory feared it would move. It
could not have: a board whose `chr_view` is `None` is off the PPU's span fast path by construction.
What it gives up instead is that fast path — one fetch at a time through `ppu_peek`, for one game
— which is the price the nine `SmallBoard` boards have paid since MF-6a step 2 and §4.10's own
rule.

### The 10 KiB, in two pieces, and why Q-123's dispatch is untouched

The page's implementation note is what settles the layout: "When making an emulator, you do not
need to care about the specific order of the CHR-RAM banks: **just provide 10KiB from
$0800-$2FFF**." This build provides exactly that:

* **6 KiB of CHR-RAM** at $0800-$1FFF, from `CartridgeMemory::chr_ram`, sized by
  `board_memory_sizes` for the reason the TQROM's arm already gives — byte 5 of an iNES 1.0 header
  is the CHR-*ROM* size and REQ-CORE-012's CHR-RAM default only applies to an image with no
  CHR-ROM, so a *Napoleon Senki* dump would otherwise come up with none. Mesen 2 agrees,
  `GetChrRamSize` `0x1800`.
* **4 KiB of nametable** at $2000-$2FFF, which is `NametableMirroring::FourScreen`: this core's
  console VRAM is 4 KiB (`ppu::VRAM_SIZE`), so the four tables are four distinct kilobytes and
  **nothing is served by the cartridge at all**.

On the board those ten are 8 KiB of cartridge RAM (the chip the 74'21 enables) plus the console's
2 KiB of CIRAM, and which 2 KiB of $2000-$2FFF is which is invisible to a program — both are RAM
and all five 2 KiB regions are distinct either way. That is what the page's note licenses, and it
is why **this board needs no `Mapper::read_nametable` body**: §4.5's dispatch and Q-123's ceiling
are untouched, where the UNROM 512's four-screen mode does serve its nametables out of CHR-RAM
because that board has no CIRAM to spare. `read_nametable_inner` is **75**, step 4's figure to the
instruction.

The mirroring is **forced** at power-on and on reset rather than read from byte 6, which is Mesen
2's reading too (`SetMirroringType(MirroringType::FourScreens)` in `InitMapper`): the page's memory
map is the board's wiring, and an iNES 1.0 dump of a singleton mapper cannot be relied on to carry
the four-screen bit. `Cartridge::reset` puts `memory.mirroring` back to the header's before calling
the board's `reset`, so the force is in `reset` and
`amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` covers it.

### What the page does not give, and is therefore left alone

**PRG-RAM.** The page has no PRG-RAM line and its hardware note lists one RAM chip, on the PPU
bus; the register is at $8000-$FFFF, so nothing of the board decodes $6000-$7FFF. The header's size
therefore stands — no `board_memory_sizes` entry for it — which is Mesen 2's reading (that file
overrides no work-RAM size) and the choice step 1 argued for $800D bit 5: of the two possible
errors, taking RAM away from a game that never asks for it is the worse one. **No Q ticket**:
nothing normative is ambiguous, the page simply does not mention a memory the board does not have.

### The shape, and one Cargo feature that is not a board

`board-decode-boards` is the umbrella `board-77` implies, and it gates `DecodeBoard` itself — the
module, the `Board` variant, its `dispatch!` arm and the registry's `use`. Cargo cannot express the
other direction, so `--features board-decode-boards` alone would leave the enum with no variant and
`Board::DecodeBoard` uninhabited; a `compile_error!` behind `not(any(feature = "board-77"))` says
so in one line instead of nine. **A later member of this enum adds its feature to that `any` and to
nothing else.**

What `DecodeBoard` forwards is [`SmallBoard`]'s list minus what no member needs, and `chr_view`
answers `None` **directly** rather than forwarding — the one line MF-6a step 2's bar turned on,
because that dispatch's arms are inlined into `Ppu2C02::run_dots` and a forward is a distinct body
there (+3.3 M to +6.9 M on eight of the thirteen, measured). `irq_asserted`, `on_cpu_cycle`,
`on_ppu_address_bus`, `nametable_page` and `expansion_audio_output` are left to `Mapper`'s
defaults, which is that enum's entry condition read across: every board in here answers *no* to
`irq_asserted` and shares the empty arm.

### Instrument 2: PR-1's thirteen — worst ROM **+0.000 %**, ten of thirteen below

`callgrind-diff.py`, label `mf6c-6-decode77`, against `mf6c-0-8496c503`:

| ROM | board | `8496c503` | **step 6** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 308 311 891 | 3 308 321 829 | +9 938 | +0.000 % |
| `oamstr` | NROM | 3 318 460 965 | **3 318 457 076** | −3 889 | −0.000 % |
| `m1` | MMC1 | 4 885 949 431 | 4 885 950 837 | +1 406 | +0.000 % |
| `m4` | MMC3 | 5 472 514 995 | **5 472 508 620** | −6 375 | −0.000 % |
| `m0` | NROM | 5 520 458 691 | **5 520 452 801** | −5 890 | −0.000 % |
| `metroid` | MMC1 | 5 115 944 288 | 5 115 957 068 | **+12 780** | **+0.000 %** |
| `smb1` | NROM | 5 996 381 185 | **5 996 376 024** | −5 161 | −0.000 % |
| `mm2` | MMC1 | 6 017 851 607 | **6 017 825 235** | −26 372 | −0.000 % |
| `z2` | MMC1 | 6 291 458 642 | **6 291 456 227** | −2 415 | −0.000 % |
| `readbuf` | CNROM | 6 834 538 187 | **6 834 525 376** | −12 811 | −0.000 % |
| `duelito` | NROM | 7 380 437 910 | **7 380 414 301** | −23 609 | −0.000 % |
| `smb3` | MMC3 | 7 949 470 334 | **7 949 437 228** | −33 106 | −0.000 % |
| `mmc5` | MMC5 | 11 279 383 781 | **11 279 342 967** | **−40 814** | −0.000 % |
| **total** | | **79 371 161 907** | **79 371 025 589** | **−136 318** | **−0.000 %** |

The worst ROM of the thirteen is `metroid` at **+12 780**, and the three above the baseline are
+1 406, +9 938 and +12 780 — inside the ±57 k-per-ROM `luaH_getshortstr` floor §5.11 measured, on a
set whose total is **below** the baseline. **`readbuf`, the CNROM whose whole run is pattern
fetches, is −12 811**, which is the receipt that a `chr_view` of `None` on a new variant costs the
PPU nothing; and `mmc5` is −40 814 on the function four earlier steps were charged +7 456 756 for.

### Instrument 1: level 2 — four cores **+0** and the fifth **below**

`l2-diff.py --bar`, against `mf6c-0-8496c503`:

| game/stage | `8496c503` | **step 6** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 147 295 142 | **11 147 295 142** | **+0** | **equal** |
| `smb1/core` | 11 515 128 803 | **11 515 128 803** | **+0** | **equal** |
| `zelda2/core` | 12 775 651 048 | **12 775 651 048** | **+0** | **equal** |
| `mm2/core` | 12 036 822 861 | **12 036 822 861** | **+0** | **equal** |
| `smb3/core` | 15 315 558 322 | **15 315 537 499** | **−20 823** | −0.000 % |
| `metroid/total` | 13 735 627 354 | **13 735 574 314** | −53 040 | −0.000 % |
| `smb1/total` | 13 589 030 161 | **13 587 881 040** | −1 149 121 | −0.008 % |
| `zelda2/total` | 15 360 284 960 | **15 359 750 311** | −534 649 | −0.003 % |
| `mm2/total` | 18 976 349 017 | **18 976 296 014** | −53 003 | −0.000 % |
| `smb3/total` | 24 187 154 526 | 24 192 914 856 | +5 760 330 | **+0.024 %** |

**Nine of the ten figures are at or below the baseline**, and the one that is not is rule 2's
layout-noise case **checked and not assumed**. `sym6c.sh` at a 400 000 threshold names every symbol
that moved on any of the five games, and there are three: `luaH_getshortstr` (+4 039 948 on `smb3`,
−1 863 700 on `smb1`, −1 424 840 on `zelda2`), `getgeneric` (+2 248 672 on `smb3`) and `luaH_set`
(+719 517 and +1 018 389). Lua's `ltable.c` and `memcpy`, nothing else — and on `smb3` the
threshold was dropped to **120 000** to be sure: the same three lines, and **no symbol of this
project moved at all** on the game whose total rose. +4 039 948 + 2 248 672 − 427 268 is +5 861 352
against a total delta of +5 760 330, which is the whole of it. The `core` column decides, and it is
four zeroes and a −20 823.

### Instrument 3: the post-LTO dump

`pins63.py lin-mf6c-0-8496c503.s lin-mf6c-6-decode77.s`:

| symbol | `8496c503` | step 4 | **step 6** |
| --- | ---: | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 3 626 | 3 626 | **3 626** |
| `Board::ppu_peek` | 272 | 290 | 291 |
| `NesBus::write_internal` | 3 815 | 3 798 | 4 035 |
| `Ppu::run_dots` | 1 907 | 1 919 | **1 919** |
| `NesBus::catch_up_ppu` | 188 | 188 | **188** |
| `Ppu2C02::read_pattern_planes` | 107 | 107 | **107** |
| `Ppu2C02::read_nametable_inner` | 114 | 75 | **75** |

`shape.py`'s five others — `resolve` 1 948, `branch` 457, `read_cycle` 181, `tick` 82,
`read_internal` 175 — are identical to the instruction, and so are the three claimants,
`Cpu::step` and `catch_up_ppu`. **The owner's two named symbols hold**: `zone_offset` does not
exist as a symbol, which is the inlining holding, and `read_pattern_planes` is 107 for the
seventh lot running.

**The committed tree is the measured tree**, shown and not asserted: the write-up above added doc
comments to `SmallBoard`, `IrqBoard` and `BOARD-FEATURES.md` *after* the three instruments ran, so
the tree was built a second time (`mf6c-6-doc77chk`, the same fifteen characters) and the two
post-LTO dumps are **byte-identical** once the crate disambiguator is folded — 36 010 954 bytes
each, the same binary size, and `pins63.py` identical on all eight symbols.

**The two that moved are reported and not waved away.** `Board::ppu_peek` is **+1** over step 4
(the new variant's arm, which is the `callq` the `#[inline(never)]` bought) and `run_dots` is
step 3's figure unchanged. `NesBus::write_internal` 3 798 → **4 035** is the step's largest
structural move — the register decode and the `ppu_write` arm of the new variant, both reached
through that one body — and what prices it at nothing is the dynamic column: `smb3` −33 106, `m4`
−6 375 and all five level-2 cores at or below the baseline.

**And a third data point against step 2's size-threshold reading**: `Cartridge::default_cpu_read`
is **429** instructions here — 377 at the baseline, 391 at step 2, 425 at step 3 — with `mmc5`
**−40 814, below the baseline**. That function's dearer layout is selected by a variant in
`SmallBoard` or `IrqBoard` and by nothing else measured so far, its own size included.

### The tests, 909 → **911**

| test | what it pins |
| --- | --- |
| `req_core_100_mapper_77_banks_32_kib_of_prg_and_2_kib_of_chr_rom` | the power-on windows and `FourScreen` mirroring; one write of `[CCCC PPPP]` moving both fields; **the bus conflict**, the same write a second time latching `$35 & 20 = $14` through a numbered bank; four bits of PRG wrapping into the image's eight 32 KiB banks; four bits of CHR with the PRG field independent; and a write at **$FFFF** decoding the same register |
| `req_core_100_mapper_77_reads_chr_rom_at_0000_and_chr_ram_at_0800` | §5's own wording — "a pattern read from CHR-ROM at $0000 and from CHR-RAM at $0800 in the same test": the ROM window following the bank register and **swallowing** a write, the three 2 KiB RAM regions distinct and *not* moved by the bank register, both CHR views `None`, the 6 KiB and not REQ-CORE-012's 32 KiB, `nametable_page` answering `FOLLOW_MIRRORING` for all four tables, and **the four nametables of $2000-$2FFF holding four different bytes at once**, which is the other 4 KiB of the page's ten |

The sweeps that write themselves held with nothing added:
`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`,
`mf_4c_the_write_latch_is_the_read_latch_minus_the_boards_that_wait_on_rd` and
`mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched` all pass, because this board moves on no
cycle and watches no bus; `amd_061_...` passes because `reset` puts the register and the mirroring
back; and `req_core_100_every_required_mapper_is_registered` took the number in its `additions`
list behind `cfg!(feature = "board-77")` (46 → 47 entries). One test helper was widened:
`chr_write`, which was behind `board-119`, is now behind
`any(feature = "board-77", feature = "board-119")` and sits with `peek` at the top of the file,
because two boards now have a second CHR chip only `Cartridge::ppu_write` can reach.

**Four configurations green**, `cargo fmt --check` clean and clippy `-D warnings` clean on
`zax-nes` in all four and on `zax-harness`, **no new `allow` of any kind**:

| configuration | `zax-nes` tests |
| --- | ---: |
| default (`board-77` on) | **848** |
| `--features parked-boards` | **853** |
| `--no-default-features` | **783** |
| `--no-default-features --features board-77` | 785 |

848 + 5 = 853, the five being `board-taito-x1`'s. With `zax-harness` the default figure is
**911 passed / 0 failed / 8 ignored**, step 4's 909 plus this step's two.

`board-77` and `board-decode-boards` are **inside the tolerance on every figure** — the worst of
the eighteen numbers above is +0.024 % on one level-2 total that is entirely Lua's hash table, and
the worst `core` or per-ROM figure is +0.000 % — so both go in `default`, and
`docs/review/BOARD-FEATURES.md` has their two rows. Step 10 still owns the all-features-off dump
comparison for every feature this lot adds at once, as step 1 planned.

### What this means for steps 7, 8 and 9

**Shape 1 holds, so the three remaining steps are the ones §5 MF-6c wrote**, with one change of
address: 72/92/86, 96 and 111 go into `DecodeBoard` rather than into [`SmallBoard`], and each
still gets its own Cargo feature and its own measurement. The discriminant they share is **already
paid for and measured at zero**, so what each step has left to price is its own code — the Jaleco
three-way parameter, 96's `on_ppu_address_bus` (which this enum does not forward yet and which
`SmallBoard` deliberately never did), and 111's `nametable_page`.

Shape 2, the `$6000-$7FFF` read path cached on `Cartridge`, is **still worth its sub-lot** and R0
has already scheduled it: it is what would un-park `board-taito-x1` and lift the limit for the
twelve MF-UNL lots, and nothing in this step touches it. Shape 3, four parked features, is not
needed.

[PROGRESS] MF-6c step 6/11 done: mapper 77 and the new `DecodeBoard` variant, 911 / 0 / 8, and
**shape 1 holds** — one new `Board` discriminant costs `mmc5` **−40 814**, the set total
**−136 318** and the worst of thirteen `metroid` +0.000 %, with four level-2 cores +0, the fifth
−20 823 and `read_pattern_planes` **107 unchanged**, which was this board's own stop-the-lot
condition.

## Step 7 — the three Jaleco µPD7756C boards, **twelve of thirteen below the baseline**

### What was built: three numbers, two boards, two `DecodeBoard` variants

§5 MF-6c step 6 of the inventory asked for "the three-way parameter (PRG size, the command bit
'required by mapper 92 games only', and 86's registers at $6000-$6FFF)". **Reading the three pages
makes it a two-way parameter and a second board**, and that is the only change this step makes to
the plan: mappers 72 and 92 are one board — nesdev ("INES Mapper 092") says so in as many words,
"This mapper is identical to mapper 072 except for the different PRG Setup" — while mapper 86 is a
GNROM-like board with its ports below $8000 and a different register layout, sharing only the
speech chip. One `bool` for the first pair, a second variant for the third number. Mesen 2 splits
them the same way (`JalecoJf17_19.h` with a `_jf19Mode` flag, `JalecoJf13.h` on its own).

| what | where | the page's words |
| --- | --- | --- |
| the PRG latch, **on the rising edge of bit 7** | `JalecoJf17::write_register` | "When a 1 is written after a 0 was previously written, load the bank specified by D" |
| the CHR latch, the same on bit 6 | the same method | the second diagram, `xCxx DDDD` |
| three bits of PRG on 72 and four on 92 | the same method | "Required by mapper 92 games only" against bit 3 |
| which window the register drives | the same method, `is_jf19` | the two PRG Setup diagrams: `PRG Reg \| { -1}` and `{ 0 } \| PRG Reg` |
| the write ANDed with the ROM's byte, on both numbers | `registry::has_bus_conflicts`, arm `72 \| 92` | "Bus conflicts: Yes" |
| mirroring from the header and no register | nothing at all | "Cuttable trace selects vertical or horizontal" |
| 86's two fields, with the CHR field's third bit up at bit 6 | `JalecoJf13::update_banks` | `$6000-$6FFF: [.CPP ..CC]` |
| 86's ports being the $6000-$7FFF window itself | `registry::board_memory_sizes`, arm `86` | "No PRG RAM (mapper ports occupy $6000-$7FFF instead)" |
| 86's accidental copies | `JalecoJf13::write_register`, one mask | "accidental copies of the registers are present at $E000-$FFFF because /ROMSEL is high when M2 is low, not just when A15 is low" |

**Two things fell out rather than being written**, and both are worth naming because they are the
reason this step costs what it does:

1. **`address & 0x7000` *is* mapper 86's decode and its design flaw at once.** That mask is $6000
   for $6000-$6FFF and for $E000-$EFFF, $7000 for $7000-$7FFF and for $F000-$FFFF, and neither for
   any of $8000-$DFFF. So the two documented ports, their two accidental copies and the whole dead
   middle are one comparison — which is what a /ROMSEL driven by M2 instead of by A15 amounts to.
2. **Mapper 86 needs no `intercept_cpu_write`.** With `prg_ram: Some(0)`,
   `Cartridge::cpu_write` walks past its PRG-RAM branch and hands $6000-$7FFF to
   [`Mapper::write_register`] with the address intact — the route mappers 47 and 37 already use. So
   this step adds **no body to that dispatch**, where mapper 140's board (the same company, the
   same window) has one.

**One power-on value no page gives**, and it is mapper 92's PRG register. On mapper 72 the
$C000-$FFFF window is the page's wired `{ -1}`; on mapper 92 the same window is the register's and
the page says nothing about what it holds at power-on. `JalecoJf17::reset_banks` puts the image's
**last** bank there on both numbers, which is Mesen 2's state (`SelectPrgPage(1, -1)` in its
`InitMapper`, whatever the mode) and the only choice that puts a cartridge's vectors under
$FFFA-$FFFF before the first write.

### The audio: **not implemented, and that is §4.8's decision, not this step's omission**

Bits 5 and 4 of mapper 72's port and the whole of mapper 86's $7000-$7FFF drive a µPD7756C ADPCM
speech chip — a track number, a start bit, and a reset that must be "low for at least 34 cycles".
§4.8 of the inventory settles it and this step quotes it rather than re-deciding: REQ-CORE-075
lists the expansion-audio chips that MAY be emulated — "VRC6, VRC7, MMC5, Namco 163, Sunsoft 5B,
FDS" — and this is not one of them, and more decisively the samples are an ADPCM mask ROM **inside
the ASIC**, bytes that are in `assets-free/`, in `assets-neutral/`, in the spec and in no
documentation this role may read. **No Q ticket**, for the reason §4.8 gives: nothing normative is
ambiguous.

So the ports are decoded and dropped, [`Mapper::expansion_audio_output`] keeps its `None` default,
and `mf_6c_the_three_jaleco_upd7756c_boards_are_silent` pins the silence over eight writes on both
of the family's decodes so that it is a choice and not a bug — the shape Q-123's VRC6 test gave an
ignored bit. The two bits are **not** stored in the save state: there is no chip for them to be the
state of.

### One dispatch changed its answer, and the reason is accuracy and not speed

[`DecodeBoard::chr_peek_view`] was a direct `None` in step 6 and is a **forward** from this step
on. Three of this enum's four boards are ordinary CHR-ROM boards that can show a view, and what a
view buys is not speed: `Ppu2C02::read_row_planes` captures `view.offsets()` when a background row
is drawn and replays the row through them later (REQ-CORE-037), and with `None` it falls back on
`read_pattern_planes` **with the banks as they are now** — so a game that switches its 8 KiB CHR
bank mid-frame would have its 3D tiles captured from the wrong bank. That dispatch is not inside
`Ppu2C02::run_dots`, which is the whole reason [`DecodeBoard::chr_view`] may *not* be forwarded,
and [`SmallBoard`] and [`IrqBoard`] both forward it already. Mapper 77 goes on answering `None`
from its own impl, because its two CHR chips cannot be one `ChrView`.

### Instrument 2: PR-1's thirteen — **twelve below the baseline**, worst +0.000 %

`callgrind-diff.py`, label `mf6c-7-jaleco72`, against `mf6c-0-8496c503`:

| ROM | board | `8496c503` | **step 7** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 308 311 891 | **3 308 285 408** | −26 483 | −0.001 % |
| `oamstr` | NROM | 3 318 460 965 | **3 318 452 514** | −8 451 | −0.000 % |
| `m1` | MMC1 | 4 885 949 431 | **4 885 900 439** | −48 992 | −0.001 % |
| `m4` | MMC3 | 5 472 514 995 | **5 472 463 007** | −51 988 | −0.001 % |
| `m0` | NROM | 5 520 458 691 | **5 520 444 917** | −13 774 | −0.000 % |
| `metroid` | MMC1 | 5 115 944 288 | **5 115 943 738** | −550 | −0.000 % |
| `smb1` | NROM | 5 996 381 185 | **5 996 335 416** | −45 769 | −0.001 % |
| `mm2` | MMC1 | 6 017 851 607 | **6 017 820 847** | −30 760 | −0.001 % |
| `z2` | MMC1 | 6 291 458 642 | **6 291 437 400** | −21 242 | −0.000 % |
| `readbuf` | CNROM | 6 834 538 187 | **6 834 536 251** | −1 936 | −0.000 % |
| `duelito` | NROM | 7 380 437 910 | 7 380 453 626 | **+15 716** | **+0.000 %** |
| `smb3` | MMC3 | 7 949 470 334 | **7 949 427 277** | −43 057 | −0.001 % |
| `mmc5` | MMC5 | 11 279 383 781 | **11 279 348 896** | −34 885 | −0.000 % |
| **total** | | **79 371 161 907** | **79 370 849 736** | **−312 171** | **−0.000 %** |

One ROM of the thirteen is above the baseline and it is `duelito` at **+15 716**, a thirteenth of
the ±57 k-per-ROM `luaH_getshortstr` floor §5.11 measured and one thirteen-hundredth of the owner's
+0.02 %. The set total is **−312 171**, the lowest this lot has recorded, and `mmc5` is −34 885 —
so the two new `DecodeBoard` variants confirm step 6's finding from the other side: a board joining
*this* enum is free, where the eleventh variant of `SmallBoard` or the twelfth of `IrqBoard` is
+0.066 % or +0.132 % on that ROM.

### Instrument 1: level 2 — four cores **+0**, the fifth below, and **both totals inside**

`l2-diff.py --bar`, against `mf6c-0-8496c503`:

| game/stage | `8496c503` | **step 7** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 147 295 142 | **11 147 295 142** | **+0** | **equal** |
| `smb1/core` | 11 515 128 803 | **11 515 128 803** | **+0** | **equal** |
| `zelda2/core` | 12 775 651 048 | **12 775 651 048** | **+0** | **equal** |
| `mm2/core` | 12 036 822 861 | **12 036 822 861** | **+0** | **equal** |
| `smb3/core` | 15 315 558 322 | **15 315 537 499** | **−20 823** | −0.000 % |
| `metroid/total` | 13 735 627 354 | 13 735 675 814 | +48 460 | +0.000 % |
| `smb1/total` | 13 589 030 161 | **13 586 678 949** | −2 351 212 | −0.017 % |
| `zelda2/total` | 15 360 284 960 | **15 359 136 808** | −1 148 152 | −0.007 % |
| `mm2/total` | 18 976 349 017 | **18 976 303 003** | −46 014 | −0.000 % |
| `smb3/total` | 24 187 154 526 | 24 189 760 284 | +2 605 758 | +0.011 % |

**No figure of this instrument needs rule 2's layout-noise argument at all**: the two above the
baseline are +0.000 % and +0.011 %, and both are *inside* the owner's +0.02 % on their own. The
argument is made anyway, because the lot has made it at every step and a reader should be able to
check it: `sym6c.sh` at 400 000 names four symbols over all five games and they are `getgeneric`
(+2 247 212 on `smb3`, −1 330 936 on `smb1`), `luaH_set` (+879 700 on `smb3`) and
`luaH_getshortstr` (−1 337 100 and −1 373 608). Lua's `ltable.c`, nothing else, and nothing at all
on `metroid`. `smb3/core` is the same −20 823 step 6 measured, to the instruction.

### Instrument 3: the post-LTO dump

`pins63.py`, the baseline and step 6 beside it:

| symbol | `8496c503` | step 6 | **step 7** |
| --- | ---: | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 3 626 | 3 626 | **3 626** |
| `Board::ppu_peek` | 272 | 291 | 296 |
| `NesBus::write_internal` | 3 815 | 4 035 | **3 843** |
| `Ppu::run_dots` | 1 907 | 1 919 | **1 919** |
| `NesBus::catch_up_ppu` | 188 | 188 | **188** |
| `Ppu2C02::read_pattern_planes` | 107 | 107 | **107** |
| `Ppu2C02::read_nametable_inner` | 114 | 75 | **75** |

`shape.py`'s five others — `resolve` 1 948, `branch` 457, `read_cycle` 181, `tick` 82,
`read_internal` 175 — are identical to the instruction, and `Cartridge::default_cpu_read` is
**429**, step 6's count unchanged: two more variants in this enum do not touch the function four
earlier steps were charged 7 456 756 instructions on.

**`NesBus::write_internal` went *down*, 4 035 → 3 843**, which is worth a line because it is the
only pinned symbol this lot has seen fall while code was added: both new register decodes carry
`#[inline(never)]`, so what that body gained is two `callq`s and what it lost is whatever LLVM had
spilled around step 6's arms. `Board::ppu_peek` 291 → 296 is the two new inner-match arms, and the
dynamic column prices them at nothing — `readbuf`, the CNROM whose whole run is fetches, is
−1 936, and all five level-2 cores are at or below the baseline.

### The tests, 911 → **915**

| test | what it pins |
| --- | --- |
| `req_core_100_the_jf17_latches_load_on_a_rising_edge_of_their_own_bit` | the power-on windows; bit 7 clear loading nothing; **0→1 loading, 1→1 and 1→0 not**; the three-bit field on mapper 72 with `$8F` reaching bank 7; the CHR latch on bit 6 being independent of both; the 8 KiB CHR bank covering $0000-$1FFF whole; and the mirroring a write cannot change |
| `req_core_100_mapper_92_drives_the_other_window_with_a_fourth_bank_bit` | **both numbers on one write of `$8B`**: mapper 72's three bits making it 16 KiB bank 3 at $8000-$BFFF with the image's last bank wired above, and mapper 92's four making it bank 11 at $C000-$FFFF with bank 0 wired below |
| `req_core_100_the_jf13_registers_have_accidental_copies_at_e000` | the `.CPP ..CC` split with **bit 6 as the CHR field's third bit** (`$61` being CHR bank 5 and not 1); $E123 being the $6000 port; $F000 being the **audio** port's copy and moving no bank; all six of $8000-$DFFF decoding nothing; and the board carrying no PRG-RAM |
| `mf_6c_the_three_jaleco_upd7756c_boards_are_silent` | `has_expansion_audio` false and `expansion_audio_output` `None` on all three numbers, across eight writes of the track number, the start bit and the reset bit on both of the family's decodes |

One test helper fixed a trap worth recording for the next board with bus conflicts:
**`prg_bank_all_ones(15)` does not make `$E000` writable through the conflict**, because the
fixture's stub program and the vectors live in the last bank — `$E000` reads `$4C` (a `JMP`) there,
and `$03 & $4C` is 0. The JF-17 test writes through `$C000` and `prg_bank_all_ones(14)` instead,
the other half of the same wired window, and says so in a comment.

The sweeps held with nothing added: these three boards move on no CPU cycle and watch no PPU bus,
and `req_core_100_every_required_mapper_is_registered` took the three numbers in its `additions`
list behind `cfg!(feature = "board-jaleco-72-92-86")` (47 → 50 entries).

**Four configurations green**, `cargo fmt --check` clean and clippy `-D warnings` clean on
`zax-nes` in all four and on `zax-harness`, **no new `allow` of any kind**:

| configuration | `zax-nes` tests |
| --- | ---: |
| default | **852** |
| `--features parked-boards` | **857** |
| `--no-default-features` | **783** |
| `--no-default-features --features board-jaleco-72-92-86` | 787 |

With `zax-harness` the default figure is **915 passed / 0 failed / 8 ignored**, step 6's 911 plus
this step's four. `board-jaleco-72-92-86` is **inside the tolerance by a factor of fifty on the
worst figure of either run**, so it goes in `default`, and `docs/review/BOARD-FEATURES.md` has its
row.

[PROGRESS] MF-6c step 7/11 done: Jaleco's JF-17, JF-19 and JF-13 on three numbers, 915 / 0 / 8,
**twelve of thirteen ROMs below the baseline** with the set total −312 171 and `mmc5` −34 885, four
level-2 cores +0 and the fifth −20 823 — and the µPD7756C audio left out on §4.8's grounds with a
test pinning the silence.

## Step 8 — the Oeka Kids board, §4.4's hook, at **+4 instructions on the dispatch it joins**

### What was built

nesdev ("INES Mapper 096"): one CPU register, and one thing that is not a register at all.

| what | where | the page's words |
| --- | --- | --- |
| two bits of 32 KiB PRG bank | `OekaKids::write_register` | `CPU $8000-$FFFF: [xxxx xCPP]`, "Select 32KiB PRG bank" |
| bit 2, the 16 KiB outer CHR bank | the same method | "Select 16KiB outer CHR bank" |
| the 4 KiB inner CHR-RAM page, **off the PPU address bus** | `OekaKids::on_ppu_address_bus` | "Select 4 KiB inner CHR RAM bank for PPU $0000-$0FFF" on bits 9-8 |
| the **edge**, not the decode | the same method | "Latch CC bits when 'DD' changes from anything else to '10'" |
| the outer bank's last page wired at $1000-$1FFF | `OekaKids::update_chr` | Mesen 2's `SelectChrPage(1, _outerChrBank \| 0x03)` |
| no PRG-RAM, 32 KiB of CHR-RAM | `registry::board_memory_sizes`, arm `96` | "PRG RAM capacity: No", "CHR capacity: 32 KiB RAM" |
| vertical mirroring, forced | `OekaKids::reset` | "Hardwired Vertical mirroring" |
| the write ANDed with the ROM's byte | `registry::has_bus_conflicts`, arm `96` | "Bus conflicts: partly" — see **Q-135** |

### §4.4's question, answered **per board and from the page**

§4.4 of `docs/review/MF-6-inventory.md` set this board aside as the one whose hook it could not
classify in advance: "Mapper 96 (Oeka Kids) is the one PPU-read-shaped hook in the inventory, and
it is not this hook" — not `Mapper::ppu_read`, because the board reacts to nametable *addresses*
and not to pattern fetches — and then: "**Mapper 96 would be the first board for which that
distinction is not academic**, because its whole mechanism is the address bus. A sub-lot doing 96
inherits MR-1's open question and must say which way it read it."

**The page answers it for this board in one sentence**, and the step quotes it rather than
reasoning from the MMC2:

> This latching occurs regardless of whether the PPU or game code initiated the address change,
> enabling software to manually select CHR pages by writing specific values to $2006.

So mapper 96 goes into **both** of §4.3's latches — `watches_ppu_reads` for the rendering's own
fetches and `watches_ppu_bus_without_read` for a bare `$2006` write — and it is the **first number
in either list with no IRQ at all**, which is the cost class §4.4 predicted.

**It does not settle MR-1's question, and the write-up says so rather than implying otherwise.**
What MR-1 leaves open is whether the MMC2's *latch*, whose rule is about /RD, should move on a bare
address change; REQ-CORE-033 and REQ-CORE-039 constrain that one and this step does not touch it.
The two are different questions about different boards, and each board's own documentation answers
its own: mapper 96's page says "regardless", so it gets both latches; the MMC2's page is about a
fetch, so its latch stays where MF-4c left it. `OekaKids`'s doc comment carries that distinction in
as many words.

**The latch is an edge and the test is what insists on it.** `DD` must change "from anything else
to '10'", so walking inside $2000-$2FFF changes nothing and only the entry into it does; and
because only the *second* write of a `$2006` pair loads `v` and reaches the cartridge, a game
selects a page the way the hardware makes it — one pair that leaves the range, then one that enters
it at the page it wants. `OekaKids::last_address` is the other side of that edge and is **in the
save state**, because nothing derives a value that is in the past.

### One reading taken, and it is **Q-135**

The page's infobox says "Bus conflicts: **partly**", which is the one line in it that decides
nothing: this build has a single `bus_conflicts` bool per cartridge (MR-BUS-1), so "partly" has to
become yes or no. It is **yes**, which is Mesen 2's reading (`HasBusConflicts` true, with no
qualification) and the reading that cannot turn a deliberate AND into a different bank. Q-135 holds
both readings, the argument and what would change the answer; the arm carries a `// Q-135`
reference, and **the two tests write the register through an all-$FF window so that they pass under
either reading** — a test asserting the AND would freeze a reading the page does not support.

### The tablet, recorded as a gap

Both titles — *Oeka Kids: Anpanman no Hiragana Daisuki* and *Anpanman to Oekaki Shiyou!!* — are
drawing programs for the **Oeka Kids tablet**, a peripheral on the expansion port, and this build
has no host surface for an input device of that kind. The brief names it as a recorded gap and not
a deliverable, and it joins MR-30-4's UNROM 512 flash, step 3's Datach barcode scanner and step 4's
*Nantettatte!! Baseball* external ROM in the same words. The board is implemented and the games
boot; what cannot be done with them is draw.

### Instrument 2: PR-1's thirteen — worst ROM **+0.001 %**

`callgrind-diff.py`, label `mf6c-8-oekakids`, against `mf6c-0-8496c503`:

| ROM | board | `8496c503` | **step 8** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 308 311 891 | 3 308 345 619 | +33 728 | +0.001 % |
| `oamstr` | NROM | 3 318 460 965 | **3 318 452 581** | −8 384 | −0.000 % |
| `m1` | MMC1 | 4 885 949 431 | 4 885 983 379 | +33 948 | +0.001 % |
| `m4` | MMC3 | 5 472 514 995 | 5 472 546 162 | +31 167 | +0.001 % |
| `m0` | NROM | 5 520 458 691 | **5 520 444 940** | −13 751 | −0.000 % |
| `metroid` | MMC1 | 5 115 944 288 | **5 115 929 129** | −15 159 | −0.000 % |
| `smb1` | NROM | 5 996 381 185 | **5 996 380 365** | −820 | −0.000 % |
| `mm2` | MMC1 | 6 017 851 607 | 6 017 864 002 | +12 395 | +0.000 % |
| `z2` | MMC1 | 6 291 458 642 | 6 291 523 763 | +65 121 | +0.001 % |
| `readbuf` | CNROM | 6 834 538 187 | 6 834 582 998 | +44 811 | +0.001 % |
| `duelito` | NROM | 7 380 437 910 | 7 380 508 097 | **+70 187** | **+0.001 %** |
| `smb3` | MMC3 | 7 949 470 334 | **7 949 434 607** | −35 727 | −0.000 % |
| `mmc5` | MMC5 | 11 279 383 781 | 11 279 389 301 | +5 520 | +0.000 % |
| **total** | | **79 371 161 907** | **79 371 384 943** | **+223 036** | **+0.000 %** |

The worst ROM is `duelito` at **+70 187, +0.001 %** — a twentieth of the owner's +0.02 %, and
inside the ±57 k-per-ROM `luaH_getshortstr` floor to within 13 k. **`m4` +31 167 is the figure
worth naming**, because the MMC3 is the board that shares the dispatch this step joins: it is
+0.001 % on a ROM whose 300 frames make about 17 M calls to it, which is two thousandths of an
instruction per call — layout, not work, and the dump below says why.

### Instrument 1: level 2 — four cores **+0**, the fifth below, three totals inside

`l2-diff.py --bar`, against `mf6c-0-8496c503`:

| game/stage | `8496c503` | **step 8** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 147 295 142 | **11 147 295 142** | **+0** | **equal** |
| `smb1/core` | 11 515 128 803 | **11 515 128 803** | **+0** | **equal** |
| `zelda2/core` | 12 775 651 048 | **12 775 651 048** | **+0** | **equal** |
| `mm2/core` | 12 036 822 861 | **12 036 822 861** | **+0** | **equal** |
| `smb3/core` | 15 315 558 322 | **15 315 537 499** | **−20 823** | −0.000 % |
| `metroid/total` | 13 735 627 354 | **13 735 615 358** | −11 996 | −0.000 % |
| `smb1/total` | 13 589 030 161 | 13 589 042 754 | +12 593 | +0.000 % |
| `zelda2/total` | 15 360 284 960 | 15 360 543 443 | +258 483 | +0.002 % |
| `mm2/total` | 18 976 349 017 | **18 976 337 477** | −11 540 | −0.000 % |
| `smb3/total` | 24 187 154 526 | 24 188 150 752 | +996 226 | +0.004 % |

**Every `core` figure is at or below the baseline** — four at +0 and `smb3/core` at the same
−20 823 steps 6 and 7 measured — and the three totals above it are +0.000 %, +0.002 % and
+0.004 %, all three *inside* the tolerance on their own. `sym6c.sh` at a 300 000 threshold names
four symbols over all five games and they are `luaH_getshortstr`, `luaH_set`, `getgeneric` and
`memcpy`: Lua's `ltable.c` and nothing else, with **nothing at all on `metroid`, `zelda2` or
`mm2`**. The board that watches 57 600 PPU addresses a frame costs the five games that do not use
it nothing measurable, which is §4.10's rule holding on the hook §4.4 was most careful about.

### Instrument 3: the post-LTO dump, and **the +4 that is the whole step**

| symbol | `8496c503` | step 7 | **step 8** |
| --- | ---: | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 3 626 | 3 626 | **3 626** |
| `Board::ppu_peek` | 272 | 296 | 291 |
| `NesBus::write_internal` | 3 815 | 3 843 | 3 861 |
| `Ppu::run_dots` | 1 907 | 1 919 | **1 919** |
| `NesBus::catch_up_ppu` | 188 | 188 | **188** |
| `Ppu2C02::read_pattern_planes` | 107 | 107 | **107** |
| `Ppu2C02::read_nametable_inner` | 114 | 75 | **75** |
| **`Board::on_ppu_address_bus`** | **157** | **157** | **161** |

`shape.py`'s five others — `resolve` 1 948, `branch` 457, `read_cycle` 181, `tick` 82,
`read_internal` 175 — are identical to the instruction, and `Cartridge::default_cpu_read` is
**429**, steps 6 and 7's count.

**The last row is the step.** `Board::on_ppu_address_bus` is the dispatch the three A12 counters
and the MMC5's in-frame detection live in, and a fifth arm cost it **four instructions** — because
`OekaKids::on_ppu_address_bus` carries `#[inline(never)]`, so what the dispatch gained is a `callq`
and not a body. That is the shape MF-6a2 step 3 established for the TQROM's fetch bodies and
MF-6b step 5 for `IrqBoard::nametable_page`, applied to the one dispatch §4.4 called an A12 board's
cost class; `Board::ppu_peek` going **down** five instructions from step 7 and `read_pattern_planes`
holding at 107 are what say the PPU-side paths were not disturbed.

### The tests, 915 → **917**

| test | what it pins |
| --- | --- |
| `req_core_100_mapper_96_banks_32_kib_of_prg_and_has_no_prg_ram` | the two-bit PRG field over four 32 KiB banks; bit 2 moving **no** PRG window ($05 and $01 selecting the same bank); the hardwired vertical mirroring; "PRG RAM capacity: No"; and the 32 KiB of CHR-RAM |
| `req_core_100_mapper_96_latches_its_chr_page_from_the_ppu_address_bus` | all **eight** 4 KiB pages written and read back through the board's own two selects; the wired $1000-$1FFF window being the outer bank's last page; **the edge** — four addresses inside $2000-$2FFF latching nothing, and the next entry latching; **a bare `$2006` write selecting a page with no fetch at all**, in the leave-then-enter pair the hardware requires; two entries in a row being one edge; and the save state carrying the previous address, so that the restored board knows which side of the edge it is on |

The three sweeps that write themselves are what made this step's wiring provable:
`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` **saw the board move** and
required `96, 96` in its expected list and `Mapper::chr_view` to be `None` for it (which
`DecodeBoard` already answers for the whole variant, and which that sweep demands of any board in
the read latch — `Ppu2C02::run_visible_span` would otherwise take a whole line of dots without
telling the cartridge anything);
`mf_4c_the_write_latch_is_the_read_latch_minus_the_boards_that_wait_on_rd` held because the number
is in both lists; and `mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched` because this board
moves on no CPU cycle. `req_core_100_every_required_mapper_is_registered` took the number behind
`cfg!(feature = "board-96")` (50 → 51 entries).

**Four configurations green**, `cargo fmt --check` clean and clippy `-D warnings` clean on
`zax-nes` in all four and on `zax-harness`, **no new `allow` of any kind**:

| configuration | `zax-nes` tests |
| --- | ---: |
| default | **854** |
| `--features parked-boards` | **859** |
| `--no-default-features` | **783** |
| `--no-default-features --features board-96` | 785 |

With `zax-harness` the default figure is **917 passed / 0 failed / 8 ignored**, step 7's 915 plus
this step's two. `board-96` is **inside the tolerance by a factor of twenty on the worst figure of
either run**, so it goes in `default`, and `docs/review/BOARD-FEATURES.md` has its row.

**One housekeeping note, because the next session will meet it**: the host disk filled during this
step and the suites failed with `os error 112` rather than on anything in the tree. What was
deleted to make room is scratch only — the `.tar` trees under `rust/target-mf/tmp`, the post-LTO
dumps of lots older than MF-6c under `rust/target-mf/asm`, and `target-mf/debug/incremental` — all
of it regenerable, and `rust/target-*` is never staged. The dumps this lot's write-ups cite
(`mf6c-0`, `mf6c-1`, `mf6c-2-x1parked`, `mf6c-3`, `mf6c-4-sunsoft4`, `mf6c-4-s4irqbrd`, `mf6c-6`,
`mf6c-7`, `mf6c-8`) are kept.

[PROGRESS] MF-6c step 8/11 done: the Oeka Kids board and §4.4's hook, 917 / 0 / 8, worst of
thirteen `duelito` +0.001 % with all five level-2 cores at or below the baseline — and the dispatch
the three A12 counters share grew by **four instructions**, because the new arm is a `callq`.
MR-1's open question is answered for this board from its own page and left open for the MMC2;
"Bus conflicts: partly" is **Q-135**.
