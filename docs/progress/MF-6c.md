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
