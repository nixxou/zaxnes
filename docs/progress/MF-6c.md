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

[PROGRESS] MF-6c step 0/10 done: the plan, the driver, and `8496c503` reproduced on all three
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

[PROGRESS] MF-6c step 1/10 done: mapper 153 as two appended `bool`s on `BandaiFcg`, 902 / 0 / 8,
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

[PROGRESS] MF-6c step 2/10 done: the Taito X1-005 and X1-017 on four numbers, 907 / 0 / 8, all
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

[PROGRESS] MF-6c step 3/10 done: `Eeprom24C0X` and mappers 157 and 159, 906 / 0 / 8, all five
level-2 cores +0 and `duelito` +0.001 % the worst of thirteen with `mmc5` −40 903 — and step 2's
park re-measured at the same +7 456 756 on a tree 48 instructions larger, which refutes that
step's size-threshold reading and leaves the park standing.
