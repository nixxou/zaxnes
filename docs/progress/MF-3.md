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
- [ ] 2/9 — Q-117: the UNROM 512 bank latch is decoded by configuration
- [ ] 3/9 — mapper 69, Sunsoft FME-7 (banks, CPU-cycle IRQ, $6000 RAM/ROM switching)
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
