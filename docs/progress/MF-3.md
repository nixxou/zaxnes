# Lot MF-3 — the mapper failures of the test-ROM campaign, Q-117, and the first new boards

Role R3 (clean-side implementer), worktree `wt-mappers`, branch `mapper-review`.

Sources of truth for this lot:

- `docs/review/MF-T-test-roms.md` (the campaign: which ROMs fail and what they say),
- `spec/changes/Q-117.md` **with its R0 answer** (the UNROM 512 bank latch),
- `REQ-CORE-100` (the mapper table and "le comportement documenté sur nesdev"),
  `REQ-CORE-102` ("Ajouter des mappers au-delà de la liste est permis"), `AMD-061` §1 (public
  documentation prevails over the original program), `AMD-054` / `REQ-CORE-075` (expansion-audio
  mixing),
- `docs/review/MR-1-mappers.md`, the **MR-1b** section, finding **MR-4-3** (the Acclaim MC-ACC),
- `docs/progress/MF-2.md` (what the previous lot did and the cost method it established),
- the NESdev wiki pages named per step, `holy-mapperel/README.md`, and `third_party/mesen2`
  where it agrees with them.

Build environment for every cargo command (the C: drive is nearly full):

    TMP=<worktree>/rust/target-mf/tmp  TEMP=<worktree>/rust/target-mf/tmp
    CARGO_TARGET_DIR=<worktree>/rust/target-mf

`zax-app` cannot build in this worktree (its demo ROM is git-ignored), so clippy is scoped to
`-p zax-nes -p zax-harness`.

## Steps

- [x] 1/9 — MMC1 SNROM: bit 4 of the CHR bank register disables the work RAM
- [x] 2/9 — Q-117: the UNROM 512 bank latch is decoded by configuration
- [x] 3/9 — mapper 69, Sunsoft FME-7 (banks, CPU-cycle IRQ, $6000 RAM/ROM switching)
- [ ] 4/9 — Sunsoft 5B audio: the three AY-3-8910 squares, envelope and noise
- [ ] 5/9 — mapper 118, TxSROM
- [ ] 6/9 — mapper 180, UNROM with the switchable bank at $C000
- [ ] 7/9 — mapper 78, submappers 1 and 3
- [ ] 8/9 — MR-4-3, the Acclaim MC-ACC (mapper 4 submapper 3)
- [ ] 9/9 — re-run the suites, the campaign document, and the close of the lot

## Out of scope (from the brief)

VRC6, VRC7, Namco 163, FDS, mapper 99 / VS. System, and anything in the CPU, PPU or APU core
(DMA, APU PAL timing, `AccuracyCoin`): lot MF-4.

**Registers in `$4020-$5FFF` (brief item 5).** None of this lot's boards has any, and each step
below says so for its own board. That range is shared with the hack packs' extension registers
and code zone (`zone_data`), which this branch does not carry; a board that needed it would stop
the lot and open a Q ticket.

## How the cost of each step is measured

The method MF-2 step 0 established, because there is no callgrind on this host:

    touch rust/crates/zax-nes/src/lib.rs
    cargo rustc --release -p zax-nes --lib -- --emit=asm

then the static instruction count of **every** symbol of the crate's `.s`, diffed against the
same dump taken before the step. The four functions the lot's rules name —
`Board::ppu_peek`, `Board::ppu_read`, `Ppu2C02::read_pattern_planes`,
`Ppu2C02::read_nametable_as` — are reported for every step; the full diff is what says whether
anything *else* moved.

Before the lot (`7cbd9c52`):

| function | instructions |
| --- | --- |
| `Board::ppu_peek` | 240 |
| `Board::ppu_read` | 241 |
| `Ppu2C02::read_pattern_planes` | 106 |
| `Ppu2C02::read_nametable_as` | 107 |

## Findings → fix → test → hot path (for R0)

| item | fix (file:line) | test | hot path touched, and why other games do not pay | commercial games whose output changes |
| --- | --- | --- | --- | --- |
| MF-T, `M1_P128K` / `M1_P128K_CR8K` detailed `4000` (SNROM work-RAM disable) | `mmc1.rs:29-38` (two fields), `:72-104` (`is_snrom`), `:105-116` (`refresh_prg_ram_disable`), `:118-119` (`update_banks`), `:200-201` (`load_registers`), `:211-215` (`prg_ram_enabled`) | `req_core_100_mmc1_snrom_disables_prg_ram_with_bit_4_of_the_chr_register`, `req_core_100_mmc1_only_an_snrom_reads_bit_4_of_the_chr_register_as_a_ram_enable`; the two Holy Mapperel rows in `test_roms_tests.rs` | **none.** The whole-crate asm diff is three symbols, all off every hot path: `Mmc1::update_banks` 137 → 148, `Mmc1::load_registers` 89 → 118, `Cartridge::create` 3435 → 3471 (the load-time decision). `Board::prg_ram_enabled` is **byte-identical** — the answer is a precomputed `bool`, so the $6000-$7FFF decode is the same load and test as before | mapper-1 images on an SNROM-shaped board (CHR-ROM ≤ 8 KiB, PRG-RAM 8 KiB, PRG-ROM ≤ 256 KiB) **that set bit 4 of `$A000`**: on real SNROM hardware that disables the RAM, so no shipped game does it while using the RAM. The golden-core set has mapper 1; *Mega Man 2* and *Zelda II* are both on this shape and neither writes that bit with the RAM live |
| Q-117 (R0's answer): the UNROM 512 latch is at `$C000-$FFFF` on a self-flashable configuration, `$8000-$BFFF` being the flash write window, and the chip recognises its sequences on **chip** addresses | `discrete.rs:422-446` (the type doc and the `latch_base` field), `:496-503` (the load-time decision), `:514-526` (`chip_address`, the two unlock constants), `:528-585` (`step_flash`), `:688-698` (`write_register`) | `q_117_unrom512_programs_a_byte_of_its_own_flash` (was `mr_30_4_…`, rewritten with the page's sequences verbatim), `q_117_unrom512_flashes_the_bank_the_driver_selects`, `q_117_unrom512_unlocks_only_on_the_chips_own_command_addresses`, `q_117_unrom512_without_self_flashing_keeps_the_latch_over_8000_ffff`, `mr_30_4_unrom512_an_incomplete_flash_sequence_changes_nothing`, `mr_30_4_unrom512_with_bus_conflicts_cannot_flash`, `mr_30_5_unrom512_submapper_4_does_not_bank_through_the_led_register` (unchanged, and now implied by the new rule) | **none, and it is a win.** `UnRom512::write_register` 116 → **106** (one compare replaces a `bool` test plus a second compare) and `Cartridge::create` 3471 → **3462** (one `u16` replaces two `bool`s). Nothing else in the crate moved. The read path is untouched: `rom_windows` is still a plain window, so no board pays anything on a read | self-flashing UNROM 512 homebrew, and **every** mapper-30 image on a no-bus-conflict configuration: its bank latch has moved from `$8000-$FFFF` to `$C000-$FFFF`. No golden-core entry is on mapper 30 |
| mapper 69, Sunsoft FME-7 / 5A / 5B — a **new board** (REQ-CORE-102) | `fme7.rs` (new, 240 lines), `board.rs:12-30` (the append rule) + `:70-72` + `:105` (the variant), `registry.rs:20-23` (`SUPPORTED_MAPPERS`) + `:181` (`create_board`), `mappers/mod.rs:26` + `:40` | `req_core_100_fme7_*` x4 and `req_core_090_the_fme7_state_survives_a_save_and_restore` in `mapper_tests.rs`, `req_core_100_the_fme7_*` x3 and `req_core_100_a_write_to_fme7_command_d_acknowledges_the_irq` in `mapper_irq_tests.rs`; `M69_P128K_C64K_S8K` and `_W8K` now **pass** with `detailed 0000` | `Board::ppu_peek` 240 → **251**, `ppu_read` 241 → **252**; `read_pattern_planes` and `read_nametable_as` **byte-identical**. The +11 is LLVM switching the function's shared bounds-check tail from tail-*merging* to tail-*duplication*: the arms' own code is unchanged and the executed path of the arms concerned is **one instruction and two taken branches shorter**. Every other hot dispatch (`on_cpu_cycle`, `irq_asserted`) has its existing arms **byte-identical** with the FME-7's appended. See "Step 3" for the wall-clock measurement and what it is worth | **none.** No commercial game moves: this is a board that did not load before. The golden-core set (mappers 0-4, 156) has no mapper-69 entry |

## Step 1 — the MMC1 SNROM work-RAM disable

### What the ROM says

`M1_P128K.nes` and `M1_P128K_CR8K.nes` (identical headers — the suite's ROMs "differ only by
header", and these two do not even differ there) print

    SNROM (MMC1): detailed 4000 (WRAM 4: $A000 bit 4 does not disable WRAM (SNROM), or
    disables it (all but SNROM)); 128K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK

The other ten MMC1 rows — SJROM, SKROM, SUROM, SXROM — were already `0000`, which is what
localised the defect to the CHR-register line of one board.

### The hardware

NESdev, *MMC1*, SNROM: the board, "which only supports 8 KiB of CHR-ROM/-RAM, uses the upper CHR
bank select line coming out of the mapper (CHR A16; bit 4 of bank number) as an additional chip
enable for the PRG-RAM". Both CHR bank registers carry it as `E`, "PRG-RAM disable (0: enable,
1: open bus)", CHR bank 1's being "ignored in 8 KiB mode"; and the page then requires that
"SNROM's `E` bit and SO/U/XROM's `P` and `S` bits in both CHR bank registers must be set to the
same values". So CHR bank 0 is the whole answer for any program the page admits, and it is also
the register the test ROM names.

**Mesen 2 does not implement this at all** — `MMC1.h:87` latches `_wramDisable` from `$E000`
only, and `UpdateState` never looks at bit 4 of a CHR register for the RAM. AMD-061 §1 makes the
public documentation prevail, so the page is followed and the divergence from Mesen is
deliberate.

### Identifying the board

The same line means something else on every other MMC1 board that has it: `P`, the 256 KiB outer
PRG bank, on SUROM and SXROM; CHR A16, a real bank line, on SJROM / SKROM / SLROM. The board page's
table separates them by size alone:

| board | PRG-ROM | PRG-RAM | CHR |
| --- | --- | --- | --- |
| SNROM | 128/256 KiB | 8 KiB | 8 KiB RAM/ROM |
| SOROM | 128/256 KiB | **16 KiB** | 8 KiB RAM/ROM |
| SUROM | **512 KiB** | 8 KiB | 8 KiB RAM/ROM |
| SXROM | 128/256/512 KiB | **32 KiB** | 8 KiB RAM/ROM |

and those are exactly the three quantities Holy Mapperel's driver measures (with its bank tags)
before it decides which way the `4xxx` digit goes. So `Mmc1::is_snrom` is those three
comparisons, taken at load.

**One trap, and it decides the iNES-1.0 half of the question.** The CHR test is on the header's
CHR-**ROM** size, not on the CHR-RAM buffer: an NES 2.0 image gives its CHR-RAM exactly
(`M1_P128K.nes` declares 8 KiB in byte 11), but REQ-CORE-012 hands an iNES 1.0 image with no
CHR-ROM **32 KiB** of CHR-RAM — a size no MMC1 board carries. Every CHR-RAM MMC1 board (SNROM,
SOROM, SUROM, SXROM, SGROM) has 8 KiB and no more, so a declared CHR-ROM of 0 *is* the 8 KiB
board. Reading the padded buffer instead would have passed the two test ROMs and left every
iNES 1.0 SNROM dump — The Legend of Zelda, Zelda II, Final Fantasy — on the wrong wiring. The
PRG-RAM size is the effective one, which for iNES 1.0 is the 8 KiB default: a dump that cannot
be told from an SNROM is an SNROM here, as it is for the test ROM's own driver. A 512 KiB
PRG-ROM is excluded by the same comparison `update_banks` already uses for `outer`, so the two
readings of bit 4 can never both be live.

### The shape, and why it costs nothing

`prg_ram_enabled` needs *both* registers on an SNROM. Written as an expression
(`(self.prg | (self.chr0 & mask)) & 0x10 == 0`) it would put a second load and a mask on the CPU's
$6000-$7FFF decode for every MMC1 game, so the answer is a `bool` field recomputed where the two
registers commit — `update_banks`, which every register write already ends with. It is derived
state, so nothing new is written to the savestate and **every mapper-1 state from before this lot
loads unchanged**; `load_registers` recomputes the bool after reading the registers back
(REQ-CORE-091).

Registers in `$4020-$5FFF`: none — the MMC1 decodes `$8000-$FFFF` only.

### Result

* `cargo fmt --check`, `cargo clippy -p zax-nes -p zax-harness --all-targets -D warnings`: clean.
* `cargo test --release -p zax-nes -p zax-harness`: 0 failed.
* `ZAXNES_TEST_ROMS=…/assets-free/test-roms cargo test --release -p zax-harness --test
  test_roms_tests`: 3 passed — the 40 Holy Mapperel rows now include `M1_P128K` and
  `M1_P128K_CR8K` as `pass`, the other ten MMC1 boards still `pass`.
* Generated code: `Board::ppu_peek` 240, `Board::ppu_read` 241,
  `Ppu2C02::read_pattern_planes` 106, `Ppu2C02::read_nametable_as` 107 — all four unchanged, and
  the whole-crate diff is the three load-time / register-commit symbols in the table above.

## Step 2 — Q-117, the UNROM 512 bank latch

### What R0 answered

Q-117 asked which bank the flash chip sees, given that — as MF-2 read the page — every byte of
a command sequence is also a write to the bank latch. R0 reread the NESdev *UNROM 512* page and
none of the ticket's three readings was right:

> **sur une configuration reprogrammable (sous-mappeur 0 avec le bit de pile, sous-mappeurs 1, 3
> et 4), le verrou de banque est en `$C000-$FFFF` seulement**, et `$8000-$BFFF` est la fenêtre
> d'écriture de la flash.

The page says it in the register diagram itself: `D~[NCCP PPPP] A~[1... .... .... ....]`, and
then "For self-flashable configurations, the address range shifts to `$C000-$FFFF`" with
`A~[11.. .... .... ....]`. And the sequences it gives put a bank write in front of **every**
command byte:

* erase — `$C000:$01, $9555:$AA`; `$C000:$00, $AAAA:$55`; `$C000:$01, $9555:$80`;
  `$C000:$01, $9555:$AA`; `$C000:$00, $AAAA:$55`; `$C000:BANK, ADDR:$30`;
* program — `$C000:$01, $9555:$AA`; `$C000:$00, $AAAA:$55`; `$C000:$01, $9555:$A0`;
  `$C000:BANK, ADDR:DATA`.

The chip sees `bank × $4000 + (ADDR & $3FFF)`, so bank 1 at `$9555` is chip `$5555` and bank 0
at `$AAAA` is chip `$2AAA` — the SST39SF040's own command addresses — and the **last** write of
each sequence reaches the bank the driver named. The interaction MF-2 had to reason about does
not exist: the bank writes are at `$C000`, outside the window, so they never touch the state
machine.

### What changed

Three things, and the first pays for the other two:

1. **`can_flash` and `has_led_register` collapse into one `latch_base: u16`**, $C000 on a
   self-flashable configuration and $8000 otherwise. "Self-flashable" is "no bus conflicts",
   since they "interfere with the self-flashing procedure" — the set
   `registry::has_bus_conflicts` already decides, so there is no new load-time predicate. The
   whole of `write_register` becomes

       if address < self.latch_base { self.step_flash(…); return; }
       self.register = value;
       self.update_banks(memory);

   One compare answers both "is this the flash window?" and "is this the latch?", and on a
   non-flashable board `address < 0x8000` is unreachable after the guard above it, so
   `step_flash` is simply never entered.
2. **`step_flash` matches on `chip_address(address)`** — `(register & $1F) × $4000 +
   (address & $3FFF)` — against `$5555` and `$2AAA` instead of on the CPU addresses `$9555` and
   `$AAAA`.
3. **MR-30-5 falls out of the new rule.** Submapper 4's LED register is at `A~[10..]`, which is
   where its flash window is; its bank register is at `$C000-$FFFF` like every other
   self-flashable board's. The special case is gone and
   `mr_30_5_unrom512_submapper_4_does_not_bank_through_the_led_register` passes unchanged.

A side effect of the page's own sequences, worth writing down: `$C000:$01` and `$C000:$00` also
set the CHR-RAM bank bits to 0 and the arrangement bit to 0, because they are writes to the
whole register. A real driver lives with that; so does ours.

### The tests

`mr_30_4_unrom512_programs_a_byte_of_its_own_flash` is replaced by
`q_117_unrom512_programs_a_byte_of_its_own_flash`, which drives the page's sequences verbatim
through two helpers (`erase_sector`, `program_byte`) that write the `$C000` bank in front of
every byte. Two new tests pin what the answer buys: the sequences reach **the bank the driver
selects** (erase the same offset in banks 1 and 3, program into 1 and not 3), and an unlock on
the wrong chip address — `$9555` latched to bank 2, chip `$1555` — programs nothing. A fourth,
in `mapper_tests.rs`, pins the other side: a configuration that is not self-flashable keeps the
latch over the whole of `$8000-$FFFF`.

Four existing tests moved their bank writes from `$8000` to `$C000`, because their images are
self-flashable configurations: `req_core_100_unrom512_switches_prg_chr_ram_and_mirroring`,
`mr_30_1_unrom512_byte6_08_is_switchable_one_screen`,
`mr_30_3_unrom512_submapper_3_switches_horizontal_and_vertical`, and the battery arm of
`mr_bus_1_unrom512_conflicts_follow_the_submapper_and_the_battery_bit`. The non-flashable arms
of that last one still write `$8100`, which is the point.

### Save states and the $4020-$5FFF rule

`latch_base` is derived at load, as `can_flash` and `has_led_register` were, so **no savestate
byte changed**: a mapper-30 state written before this lot loads unchanged (`register`,
`flash_step`, `flash_erasing`, in that order).

Registers in `$4020-$5FFF`: none. The UNROM 512 decodes `$8000-$FFFF` only, and REQ-CORE-100
gives the board no PRG-RAM at all (MR-30-2).

### Result

* `cargo fmt --check`, `cargo clippy -p zax-nes -p zax-harness --all-targets -D warnings`: clean.
* `cargo test --release -p zax-nes -p zax-harness`: 0 failed.
* Generated code: `Board::ppu_peek` 240, `Board::ppu_read` 241,
  `Ppu2C02::read_pattern_planes` 106, `Ppu2C02::read_nametable_as` 107 — unchanged. The only two
  symbols in the whole-crate diff both **shrank**: `UnRom512::write_register` 116 → 106,
  `Cartridge::create` 3471 → 3462.

## Step 3 — mapper 69, the Sunsoft FME-7

### The board

NESdev, *Sunsoft FME-7*. Everything goes through two addresses: a **command register** at
`$8000-$9FFF` (`.... CCCC`, four bits naming one of sixteen commands) and a **parameter
register** at `$A000-$BFFF`, where "Writing here invokes the command with the specified parameter
byte". The sixteen commands:

| command | effect |
| --- | --- |
| `$0`-`$7` | 1 KiB CHR bank at PPU `$0000-$03FF` … `$1C00-$1FFF`, `BBBB BBBB` |
| `$8` | the `$6000-$7FFF` window, `ERbB BBBB` |
| `$9`-`$B` | 8 KiB PRG bank at `$8000`, `$A000`, `$C000`, `..bB BBBB` |
| `$C` | nametable arrangement, `.... ..MM` |
| `$D` | IRQ control, `C... ...T` |
| `$E`, `$F` | low and high byte of the IRQ counter |

`$E000-$FFFF` is absent from the PRG table because the last bank is wired there. Mapper 69 is
outside REQ-CORE-100's table; REQ-CORE-102 says "Ajouter des mappers au-delà de la liste est
permis", and Holy Mapperel's `README.md` names the boards: "069: JLROM, JSROM (Batman: Return of
the Joker, Gimmick!, Hebereke)".

**Command `$8`, the one window that is not plain.** `E` is the RAM chip's `+CE` and `R` picks
"0=ROM, 1=RAM", so the window has three states: RAM (both set), **open bus** (`R` set, `E`
clear — the chip deselected), and a plain 8 KiB PRG-**ROM** bank (`R` clear, whatever `E` says).
`Cartridge::default_cpu_read` reads `$6000-$7FFF` from PRG-RAM when `prg_ram_enabled` says so and
otherwise asks the board, so the ROM and open-bus states are served by a `try_read_prg` override
and nothing else changes: the `$8000-$FFFF` fast path stays `Cartridge::read_rom_window` over the
default `rom_windows`.

`ram_readable` and `rom_offset_6000` are precomputed from the register, so `prg_ram_enabled` — on
the CPU's `$6000-$7FFF` decode — is one load and one test like every other board's. The bank count
for the wrap comes from `BankWindows::prg_bank_count_8k()` rather than from the ROM, which lets
`load_registers` (handed a reader and no memories) bring both derived halves back with the
register; the first version read the ROM length, and the savestate test caught it at once.

**Command `$C` and the arrangement vocabulary.** The page names the four values "0: Horizontal
(A10), 1: Vertical (A11), 2: 1scA (GND), 3: 1scB (VCC)" — the *arrangement* wording of the iNES
header, where "(A10)" means CIRAM A10 is driven by PPU A10, i.e. the horizontal arrangement, which
every board page in this crate calls **vertical mirroring**. Mesen 2 reads it the same way
(`SunsoftFme7.h:103-110`: 0 → `Vertical`), the two sources agree, and the Holy Mapperel ROM is the
witness: it "detects what mapper it's running on through mirroring", so the wrong reading would
have beeped `MIR` instead of printing `J*ROM (FME-7)`.

**The IRQ.** "When the IRQ counter is decremented from `$0000` to `$FFFF` an IRQ is generated",
the 16-bit counter running off M2 — one step per CPU cycle — while `$D`'s `C` bit allows it, and
"Writes to the IRQ counter registers directly set the lower or upper eight bits of the counter":
`$E` and `$F` are halves of the counter, not a latch and a reload. The line is only pulled if `T`
was set *at the moment of the wrap* — the wrap is not latched into a flag a later `T` could
reveal, which is how Mesen reads the page too (`SunsoftFme7.h:59-66`) and what
`req_core_100_the_fme7_needs_both_command_d_bits` pins. "All writes to this register acknowledge
an active IRQ" is command `$D` and nothing else. No `ppu_irq_horizon` override: nothing the PPU
does can move this line, which is the trait's default.

Registers in `$4020-$5FFF`: **none** — the board decodes `$6000-$FFFF` only, so the brief's
item 5 does not bite.

The 5B's three sound channels are at `$C000-$DFFF` and `$E000-$FFFF`; a write there is ignored for
now, which is exactly what a 5A — the same chip without the sound — does. Step 4 adds them.

### The cost of a new `Board` variant, measured properly

This is the first step of the lot that adds a variant, so it is where the method is set down.

**`board.rs` now says that a board is appended, never inserted**, and the note there gives the two
measured reasons: `Mmc5` must stay at index 5 (the niche-filling tag lives in its last `bool`, and
every dispatch prologue is `movzbl 1355(%rcx); subb $2; movl $5; cmovael` — a literal 5), and
appending leaves the existing discriminants, and so the existing jump-table slots, where they are.
Checked both ways on `Board::ppu_peek`: appended, the arms before the new one are
instruction-identical; inserted after `GxRom`, four arms move and their operand offsets move with
them.

**The four functions the lot's rules name:**

| function | step 2 | step 3 |
| --- | ---: | ---: |
| `Board::ppu_peek` | 240 | **251** |
| `Board::ppu_read` | 241 | **252** |
| `Ppu2C02::read_pattern_planes` | 106 | 106 (byte-identical) |
| `Ppu2C02::read_nametable_as` | 107 | 107 (instruction-identical; one immediate widens by a bit, being a bitset over the variants) |

The +11 on the two `Board` fetch functions is **not added work**. The prologue, the jump table's
shape and every existing arm's own instructions are unchanged; what changed is that LLVM stopped
tail-*merging* the shared "offset in range → load → return" tail and tail-*duplicated* it into
four arms instead. For those arms the executed path goes from
`jmp; cmpq; jb(taken); movzbl; retq` to `cmpq; jae(not taken); movzbl; retq` — **one instruction
and two taken branches fewer**. The same check on the two dispatches that run every CPU cycle,
`Board::on_cpu_cycle` and `Board::irq_asserted`, shows the existing arms **byte-identical** with
the FME-7's appended at the end.

**Wall clock, because the static count is only a proxy.** `zax-harness bench-core`, 3600 frames
after 300 of warm-up, interleaved runs of two binaries that differ only by this step, on
`mmc3_test/5-MMC3.nes` (mapper 4) and `holy-mapperel/M0_P32K_C8K_V.nes` (mapper 0) — neither can
execute a line of FME-7 code:

| build | 5-MMC3, min of 6 | NROM, min of 4 |
| --- | ---: | ---: |
| before the step | 0.9932 ms/frame | 0.8226 |
| the variant with an **empty** board (control) | 1.0079 | 0.8225 |
| the step as committed | 1.0243 | 0.8345 |

So the 27th variant with no code behind it costs **0.0 %** on NROM, and the step as committed
about 1.4 % — on a ROM that never reaches the board. That is code placement and register
allocation, not work: a second run of the same three binaries in a later session put *before* at
0.8172-0.8387 (a ±1.5 % spread of its own), and a behaviour-neutral perturbation of the FME-7
(`#[inline(never)]` on two of its own private functions, which no other board can reach) landed
between the two at 0.8323. Two things did change in the PPU and the CPU without being asked to —
`Ppu2C02::step_dot` is the same instructions with `%r15` renamed to `%r13`, and
`Cpu6502::execute` stopped being a symbol of its own because `execute_without_operand` left it,
which let it inline into `step` — and neither followed from a source change on those paths.
`#[inline]` on `execute_without_operand` does not move it back, and the CPU is out of this lot's
scope, so no attribute was added for it.

**For R0:** the instrument for this question is callgrind, whose instruction counts are
deterministic, not this host's wall clock. The claim this step makes is the narrow one that can be
checked statically and was: no existing board's dispatch arm gained an instruction, and the two
fragile PPU functions are unchanged. Three of this lot's remaining boards go **into existing
boards** rather than into new variants — mapper 118 and the MC-ACC because the brief asks for them
"as a variant of the MMC3 board" and "confined to the MMC3 board", mapper 180 because it is a
UNROM wiring — so mapper 69 and mapper 78 are the only two variants the lot adds.

### Result

* `cargo fmt --check`, `cargo clippy -p zax-nes -p zax-harness --all-targets -D warnings`: clean.
* `cargo test --release -p zax-nes -p zax-harness`: 0 failed. Three existing tests had to move
  with the registry: `req_core_100_every_required_mapper_is_registered` now asserts REQ-CORE-100's
  table is a **subset** of `SUPPORTED_MAPPERS` and names the two numbers REQ-CORE-102 lets this
  build add (69 and 156), and the two "an unimplemented mapper is refused" tests use 68 where they
  used 69.
* `zax-harness test-roms --filter M69`: **2 pass**, `J*ROM (FME-7): 128K PRG ROM, 8K PRG RAM OK,
  64K CHR ROM OK, detailed 0000` — the IRQ digit included, which is the FME-7 interval timer Holy
  Mapperel's README says it measures.
* `ZAXNES_TEST_ROMS=… cargo test --release -p zax-harness --test test_roms_tests`: 3 passed, with
  the two mapper-69 rows flipped from `unsupported` to `pass`.
