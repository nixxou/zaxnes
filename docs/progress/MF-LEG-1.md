# MF-LEG-1 — the small fixes from the legacy cross-check

Role R3, clean side. Base commit **`a775d0e7`** (`LEGACY-XCHECK (26/26, final)`), branch
`mapper-review`, worktree `wt-mappers`.

Sources consulted: `docs/review/LEGACY-XCHECK.md` (the summary table, the six DIFFERS entries this
lot takes and "For the owner"), `docs/review/BOARD-FEATURES.md`, `docs/progress/MF-FIX.md` (the
measurement method), and the nesdev pages of each board — *MMC1*, *INES Mapper 155*,
*NES 2.0 submappers*, *VRC2 and VRC4*, *CPROM*, *INES Mapper 018*, *INES Mapper 032*.

## What the lot does

Six entries of the legacy cross-check, one step each. Nothing else: entries 15-1 and MMC1-2 are the
owner's to keep as they are, and MMC5-1 is lot MF-LEG-2.

| # | step | entry |
| --: | --- | --- |
| 1 | MMC1: the SNROM `E` bit needs NES 2.0 evidence, plus the five-ROM extra check | MMC1-1 |
| 2 | VRC2: the one-bit latch at `$6000`-`$6FFF` | VRC2-1 |
| 3 | CPROM: bus conflicts on the bank register | 13-1 |
| 4 | Jaleco SS88006: `$9002`, the PRG-RAM enable and write protect | 18-2 |
| 5 | MMC1A: NES 2.0 submapper 3 | MMC1-4 |
| 6 | Irem G-101: *Major League* on the upper one-screen page | 32-1 |
| 7 | golden-core over the 98 originals against the base commit | — |
| 8 | the callgrind campaign and the feature decision per board | — |
| 9 | the final check, and the report annotations | — |

## Steps

- [x] 1/9 MMC1 SNROM (MMC1-1) — the `E` bit only on NES 2.0 evidence, and the extra check

### 1/9 — MMC1 SNROM: the `E` bit needs the header's word (entry MMC1-1)

**What the entry asked for.** Option (a) of its three: keep the `E` bit — the page and
`holy-mapperel`'s `4xxx` digit want it, and AMD-061 makes the public documentation prevail — but
narrow the **board test**, which fired on every iNES 1.0 mapper-1 dump with CHR-RAM and the default
8 KiB of PRG-RAM. That is the SGROM shape as much as the SNROM one, and an iNES 1.0 header cannot
tell them apart: no CHR-RAM field, and REQ-CORE-012 hands every dump 8 KiB of work RAM whether the
cartridge has a chip or not.

**What was built.** `Mmc1::is_snrom` (`mmc1.rs`) now requires an **NES 2.0** header that declares

* no CHR-ROM and exactly 8 KiB of CHR-RAM (byte 11 = 7). Byte 11 of zero means REQ-CORE-012's
  32 KiB default, which no MMC1 board carries, so it says nothing;
* a **non-zero** byte 10 totalling exactly 8 KiB — volatile or non-volatile, the chip is what
  matters. The byte has to be tested on its own because `nes20_prg_ram_size` floors at 8 KiB, so a
  declared *zero* reads back as the default;
* at most 256 KiB of PRG-ROM, as before, so the `P`/`E` readings of bit 4 can never both be live.

Nothing else moved: `refresh_prg_ram_disable` is the same one expression, `chr_disable_mask` is the
same mask, and the change is in `Mmc1::new` alone — **load time, nothing on any hot path**.

**The extra check the lot owes, and its result.** `tests/mf_leg_1_mmc1_e_bit_tests.rs`, `#[ignore]`d
and run with `ZAXNES_ROMS=G:/3dsen/demo/roms`. The question reduces to one bit: on a board the old
test fired on, the `E` bit *is* bit 4 of the committed CHR bank 0 register, so "the old rule would
have disabled the RAM" is "that bit was set". The register can only change on a CPU write, so the
run is driven **instruction by instruction** — `run_frame`'s own loop minus the margin decoder, the
overclock idle loop and the video callback, none of which these games have with no pack — and the
bit is sampled after every instruction. There is no sub-instruction window a commit could hide in.
One `pub const fn chr_bank0_has_the_e_bit` was added to `Mmc1` for it, over one field, on no path.

| ROM | frames | instructions | commits the `E` bit |
| --- | ---: | ---: | :-: |
| `Zelda II - The Adventure of Link (USA).nes` | 3600 | 35 675 778 | no |
| `Zelda II - The Adventure of Link (Europe).nes` | 3600 | 34 764 725 | no |
| `Metroid (USA).nes` | 3600 | 40 208 611 | no |
| `Mega Man 2 (USA).nes` | 3600 | 34 647 662 | no |
| `Final Fantasy (USA).nes` | 3600 | 35 637 469 | no |

**181 million instructions and not one commit of bit 4 to `$A000`.** So the fix changes nothing for
any of the five, the condition of the lot instruction is met, and **the fix is unconditional — no
Cargo feature.**

**Tests.** The two existing SNROM tests keep their behaviour; the positive one now loads an NES 2.0
image, which needed a `chr_ram(bytes)` builder on `MapperRom` (byte 11). Two new ones:
`mf_leg_1_mmc1_an_ines_1_0_dump_with_chr_ram_does_not_get_the_snrom_e_bit` — the Zelda II shape,
with the `$E000` half still disabling as the regression guard — and
`mf_leg_1_mmc1_the_snrom_e_bit_needs_all_three_nes_20_declarations`, which walks the three ways a
dump can fall short (byte 10 zero, byte 11 absent, 16 KiB of CHR-RAM) and then the shape that
passes. `cargo test --release -p zax-nes -p zax-harness`: **1157 passed, 0 failed, 10 ignored**.

Two comments that read `Mmc1::is_snrom`'s old signature — the NES-EVENT's `prg_ram_enabled`
override and its test — were corrected; the override itself is unchanged and still necessary.
- [ ] 2/9 VRC2 (VRC2-1) — the one-bit latch at `$6000`-`$6FFF`
- [ ] 3/9 CPROM (13-1) — bus conflicts on the bank register
- [ ] 4/9 Jaleco SS88006 (18-2) — `$9002`
- [ ] 5/9 MMC1A (MMC1-4) — NES 2.0 submapper 3
- [ ] 6/9 Irem G-101 (32-1) — *Major League* on the upper page
- [ ] 7/9 golden-core over the 98 originals
- [ ] 8/9 the callgrind campaign and the feature decisions
- [ ] 9/9 the final check and the report annotations
