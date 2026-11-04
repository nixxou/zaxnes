# MR-1 — adversarial review of the mapper boards against Mesen 2

Role R3, clean side. Lot MR-1. **This lot changes no emulation behaviour**: the only code it
adds is `#[ignore]`d tests that each reproduce one finding, so `cargo test` stays green.

Reference: `third_party/mesen2/` — Mesen 2, GPL v3, commit `b9fa69d`, used as the accuracy
reference (`third_party/README.md`). Paths below are relative to
`third_party/mesen2/Core/NES/`. Our paths are relative to
`rust/crates/zax-nes/src/cartridges/`.

Method: for each board, read our implementation and Mesen's side by side and assume ours is
wrong until the code says otherwise. Each difference is classified:

- **our bug** — we are wrong; a `#[ignore]`d test is added that fails on current code;
- **Mesen differs / Mesen in error** — Mesen is deliberately different (a game-database
  heuristic we do not have) or contradicts the hardware;
- **equivalent** — different code, same observable behaviour;
- **unknown** — the difference is real but neither side can be shown right from the sources
  allowed here.

The spec prevails over Mesen. Where our code follows a REQ that Mesen contradicts, the entry
is a spec question (a `Q-NNN` ticket in `spec/changes/`), not a bug.

---

## Board inventory

Our registry (`registry.rs:43-71`) advertises 32 mapper numbers over 26 board types. The
mapping to Mesen's classes, via `MapperFactory.cpp:280-500`:

| iNES | our board (`mappers/`) | Mesen class | Mesen file |
| --- | --- | --- | --- |
| 0 | `Nrom` (`nrom.rs`) | `NROM` | `Mappers/Nintendo/NROM.h` |
| 1 | `Mmc1` (`mmc1.rs`) | `MMC1` | `Mappers/Nintendo/MMC1.h` |
| 2 | `UxRom` (`discrete.rs`) | `UNROM` | `Mappers/Nintendo/UNROM.h` |
| 3 | `CnRom` (`discrete.rs`) | `CNROM` | `Mappers/Nintendo/CNROM.h` |
| 4 | `Mmc3` (`mmc3.rs`) | `MMC3` (+ `MMC6`) | `Mappers/Nintendo/MMC3.h` |
| 5 | `Mmc5` (`mmc5.rs`) | `MMC5` | `Mappers/Nintendo/MMC5.h` |
| 7 | `AxRom` (`discrete.rs`) | `AXROM` | `Mappers/Nintendo/AXROM.h` |
| 9 | `Mmc2` (`mmc2.rs`) | `MMC2` | `Mappers/Nintendo/MMC2.h` |
| 10 | `Mmc2` (`mmc2.rs`) | `MMC4` | `Mappers/Nintendo/MMC4.h` |
| 11 | `ColorDreams` (`discrete.rs`) | `ColorDreams` | `Mappers/Unlicensed/ColorDreams.h` |
| 13 | `CpRom` (`discrete.rs`) | `CpRom` | `Mappers/Nintendo/CpRom.h` |
| 15 | `Mapper015` (`multicart.rs`) | `Mapper15` | `Mappers/Unlicensed/Mapper15.h` |
| 16 | `BandaiFcg` (`bandai_fcg.rs`) | `BandaiFcg` | `Mappers/Bandai/BandaiFcg.h` |
| 18 | `JalecoSs88006` (`jaleco_ss88006.rs`) | `JalecoSs88006` | `Mappers/Jaleco/JalecoSs88006.h` |
| 21/22/23/25/27 | `Vrc2Vrc4` (`vrc2_vrc4.rs`) | `VRC2_4` | `Mappers/Konami/VRC2_4.h` |
| 28 | `Action53` (`multicart.rs`) | `Action53` | `Mappers/Homebrew/Action53.h` |
| 30 | `UnRom512` (`discrete.rs`) | `UnRom512` | `Mappers/Homebrew/UnRom512.h` |
| 32 | `IremG101` (`iremg101.rs`) | `IremG101` | `Mappers/Irem/IremG101.h` |
| 34 | `Mapper034` (`multicart.rs`) | `Nina01` / `BnRom` | `Mappers/Unlicensed/Nina01.h`, `Mappers/Irem/BnRom.h` |
| 41 | `Caltron6In1` (`multicart.rs`) | `Caltron41` | `Mappers/Ntdec/Caltron41.h` |
| 64 | `Rambo1` (`rambo1.rs`) | `Rambo1` | `Mappers/Tengen/Rambo1.h` |
| 66 | `GxRom` (`discrete.rs`) | `GxRom` | `Mappers/Nintendo/GxRom.h` |
| 71 | `Codemasters` (`discrete.rs`) | `BF909x` | `Mappers/Codemasters/BF909x.h` |
| 87/101 | `JalecoJ87` (`multicart.rs`) | `JalecoJfxx` | `Mappers/Jaleco/JalecoJfxx.h` |
| 156 | `Daou306` (`daou306.rs`) | `DaouInfosys` | `Mappers/Unlicensed/DaouInfosys.h` |
| 225 | `Multicart225` (`multicart.rs`) | `Mapper225` | `Mappers/Unlicensed/Mapper225.h` |
| 228 | `Action52` (`multicart.rs`) | `ActionEnterprises` | `Mappers/Unlicensed/ActionEnterprises.h` |

Shared, reviewed with the boards that use it:

| ours | Mesen |
| --- | --- |
| `banked.rs` (`BankWindows`, 4x8K PRG + 8x1K CHR) | `BaseMapper::SelectPrgPage*` / `SelectChrPage*`, `BaseMapper.h` |
| `a12_watcher.rs` (`A12Watcher`) | `Mappers/A12Watcher.h` |
| `board.rs` (`Board` enum dispatch) | `BaseMapper` virtual dispatch |
| `mappers/mod.rs` (`Mapper` trait defaults) | `BaseMapper` defaults |
| `rom/ines_header.rs` (header, submapper) | `NesHeader.h`, `Loaders/` |
| no counterpart | `Mappers/Konami/VrcIrq.h` (shared VRC IRQ) |
| no counterpart | `Mappers/Bandai/BaseEeprom24C0X.h`, `Eeprom24C01.h`, `Eeprom24C02.h` |
| no counterpart | `GameDatabase.cpp` (per-ROM overrides, bus-conflict and CHR-RAM sizing) |

Two differences in the inventory itself, before any register is compared:

- **Mapper 4 submapper 3** selects Mesen's `McAcc` (Acclaim MC-ACC, a different A12 filter
  position); we always build `Mmc3`. See MR-4-5.
- **Mapper 34 submapper 0** is resolved by Mesen from the *CHR-ROM size*
  (`MapperFactory.cpp:322`: CHR present -> NINA-001, absent -> BNROM); we decide the same way
  but in the board. See MR-34-1.

---

## Shared: bus conflicts (MR-BUS-1)

Reviewed here because it is the single most widely-shared difference among the discrete
boards.

A board whose bank latch hangs directly off the data bus, with no `/ROMSEL`-gated write
enable, sees the CPU's write *and* the PRG-ROM's own output drive the bus at once. The
value latched is the AND of the two. Mesen implements it centrally, in
`BaseMapper::WriteRam` (`BaseMapper.cpp:884-896`):

```cpp
if(_hasBusConflicts) {
    uint8_t prgValue = _prgPages[addr >> 8][(uint8_t)addr];
    if(value != prgValue) { _emu->BreakIfDebugging(...); }
    value &= prgValue;
}
WriteRegister(addr, value);
```

`_hasBusConflicts` comes from the per-board `HasBusConflicts()` override, overridable per
ROM by the game database (`BaseMapper.cpp:658-667`). Among our boards it is set by:

| board | Mesen condition | file:line |
| --- | --- | --- |
| 2 UNROM | submapper 2 | `Mappers/Nintendo/UNROM.h:21` |
| 3 CNROM | submapper 2 | `Mappers/Nintendo/CNROM.h:19` |
| 7 AXROM | submapper 2 | `Mappers/Nintendo/AXROM.h:20` |
| 11 Color Dreams | always | `Mappers/Unlicensed/ColorDreams.h:10` |
| 30 UNROM 512 | submapper 0 without battery, or submapper 2 | `Mappers/Homebrew/UnRom512.h:25` |

We have no such path: `Cartridge::cpu_write` (`mod.rs:390-410`) passes the value to
`Board::write_register` unchanged, and nothing in `rust/crates` mentions a bus conflict.

**Classification: our bug** (a missing feature, shared by five boards). NES 2.0 submapper 2
of mappers 2/3/7 exists precisely to say "this board has bus conflicts", so a correctly
headered ROM that relies on it mis-banks. The practical blast radius is small: the games
that actually depend on it write a value equal to the ROM byte anyway (which is why they
work on hardware), and Color Dreams titles write to $8000-$FFFF addresses holding $FF.
The failure mode when it bites is a wrong PRG bank, i.e. a crash.

Severity **subtle**. Tests `mr_bus_1_color_dreams_write_is_anded_with_the_rom_byte` and
`mr_bus_1_uxrom_submapper_2_write_is_anded_with_the_rom_byte`; the companion
`mr_bus_1_uxrom_without_a_submapper_has_no_conflict` is *not* ignored and pins the
behaviour a fix must not break.

Proposed fix: one check in `Cartridge::cpu_write` before `write_register`, gated on a
`Mapper::has_bus_conflicts()` default-`false` trait method, ANDing with
`read_rom_window(address)`.

---

## Shared: bank-window wrapping

Our `BankWindows::wrap` (`banked.rs:236-238`) is `bank.rem_euclid(count)` over 8 KiB PRG
units; Mesen wraps per-mapper page in `SetCpuMemoryMapping`'s `wrapPageNumber`
(`BaseMapper.cpp:98-105`): `page % pageCount` for a non-negative page, `pageCount + page`
for a negative one.

**Equivalent** for every real image. Because `(v % (n/k)) * k == (v * k) % n` when `k`
divides `n`, our "convert to 8 KiB units then wrap" agrees with Mesen's "wrap in the
mapper's own page unit" whenever the PRG size is a multiple of the page size — which holds
for every power-of-two ROM. Two edge cases differ and neither occurs in a real dump:

- a PRG size that is *not* a multiple of the board's page size (e.g. 40 KiB on mapper 2):
  Mesen truncates `pageCount` and leaves the tail unreachable, we wrap in 8 KiB units and
  reach it. Ours is the more defensible reading of "the last 16 KiB"; nothing depends on it.
- a page number below `-pageCount`: Mesen's `pageCount + page` stays negative (its comment
  explains the deliberate choice of addition over modulo, for mapper 191), ours wraps. No
  board we implement passes anything below -4.

Mesen's `SelectPrgPage` undersize special case (`BaseMapper.cpp:381-393`, "map the entire
PRG to all slots" when the page size exceeds a sub-32-KiB PRG) is **equivalent** to what our
modulo already does: a 16 KiB PRG on a 32 KiB-page board gives 8 KiB banks 0,1,0,1 both ways.

---

## Mapper 0 — NROM

`nrom.rs` against `Mappers/Nintendo/NROM.h`. **Equivalent.** No registers; `try_read_prg`
wraps the address modulo the PRG size (`nrom.rs:40`), which mirrors the 16 KiB NROM-128
image twice exactly as Mesen's two `SelectPrgPage` calls over a 1-page ROM do.

## Mapper 2 — UxROM

`discrete.rs:8-58` against `Mappers/Nintendo/UNROM.h`. Register effect, fixed last bank at
$C000 and the full 8-bit bank value all agree. One difference: **MR-BUS-1** (submapper 2).

Mesen's register range is the `BaseMapper` default $8000-$FFFF, ours is `address >= 0x8000`
(`discrete.rs:53`) — the same. Mapper 2's sibling boards `UnRom_94.h` and `UnRom_180.h`
(mappers 94, 180) are different mapper numbers and out of scope.

## Mapper 3 — CNROM

`discrete.rs:60-108` against `Mappers/Nintendo/CNROM.h`. **Equivalent** but for
**MR-BUS-1** (submapper 2).

Mesen's power-on CHR bank is `GetPowerOnByte()` rather than a hard 0 (`CNROM.h:14`); that
returns 0 unless the user turns on `RandomizeMapperPowerOnState`, so the default build
agrees with our 0. Mesen's CNROM-with-protection-diodes variant is a *different mapper
number* (185, `Mappers/Nintendo/CnromProtect.h`) and is out of this lot's scope — we do not
claim 185.

## Mapper 7 — AxROM

`discrete.rs:110-163` against `Mappers/Nintendo/AXROM.h`. **Equivalent** but for
**MR-BUS-1** (submapper 2). Bank field `value & 0x0F`, single-screen select from bit 4 with
the same polarity (bit set -> screen B), and Mesen's power-on `WriteRegister(0, GetPowerOnByte())`
reduces to our reset to register 0 in the default build.

## Mapper 66 — GxROM

`discrete.rs:165-214` against `Mappers/Nintendo/GxRom.h`. **Equivalent**: PRG from bits
4-5, CHR from bits 0-1, both masked to two bits on both sides, no bus conflicts on either.

## Mapper 11 — Color Dreams

`discrete.rs:267-315` against `Mappers/Unlicensed/ColorDreams.h`.

**MR-11-1 — our bug, severity subtle.** The latch's PRG field is four bits wide. Mesen:
`SelectPrgPage(0, value & 0x0F)` (`ColorDreams.h:22`), matching the nesdev "Color Dreams"
register layout `CCCC PPPP` that REQ-CORE-100 makes normative. We mask with `0x03`
(`discrete.rs:283`), so bits 2-3 never reach PRG A17-A18 and no Color Dreams image above
128 KiB can reach its upper half — the modulo in `wrap` silently folds the bank back to
0-3 instead. CHR is `value >> 4` on both sides and agrees.

Affects oversized Color Dreams images; every licensed-by-nobody Color Dreams cartridge
(*Crystal Mines*, *Metal Fighter*, *Operation Secret Storm*) is 64-128 KiB and unaffected,
which is why this has not shown up. Mesen's own comment at `ColorDreams.h:20` ("Re-add size
restriction when adding an option to prevent oversized roms") confirms the 4-bit field is
deliberate. Test `mr_11_1_color_dreams_prg_field_is_four_bits`.

Also **MR-BUS-1**, which Mesen applies to this board unconditionally.

## Mapper 13 — CPROM

`discrete.rs:216-265` against `Mappers/Nintendo/CpRom.h`.

Banking agrees: the low 4 KiB of CHR-RAM fixed, the high 4 KiB selected by bits 0-1
(`discrete.rs:261` masks `0x03`, Mesen `value & 0x03` at `CpRom.h:19`), PRG fixed.

**MR-13-1 — our bug, severity cosmetic/none.** CPROM carries 16 KiB of CHR-RAM.
REQ-CORE-100 pins it in the table ("13 | CPROM | CHR-RAM 16 Kio") and Mesen overrides
`GetChrRamSize()` to `0x4000` (`CpRom.h:9`). We take the size from the header, and
`ines_header.rs:154` gives the 32 KiB `DEFAULT_CHR_RAM_SIZE` because no board may override
it. No pattern byte changes — the windows only ever address the low 16 KiB, so the upper
half is dead — but the cartridge exposes twice the RAM it should and the save state carries
16 KiB of zeroes per frame of rollback. Test `mr_13_1_cprom_has_16_kib_of_chr_ram`.

Proposed fix: a `Mapper::chr_ram_size_override()` consulted by `CartridgeMemory::new`,
which MR-30 would reuse.

**MR-13-2 — unknown, severity visible if it bites.** Mesen forces
`SetMirroringType(MirroringType::Vertical)` in `InitMapper` (`CpRom.h:15`); we leave the
header's layout alone. *Videomation* is the only mapper-13 title and its header declares
vertical, so the two agree on every real dump; a dump with a wrong byte-6 bit 0 would
render differently. Whether the CPROM board hardwires vertical or honours the solder pad is
not something the sources allowed here settle, so this stays unknown rather than becoming a
bug on either side. No test.

## Mapper 71 — Camerica / Codemasters

`discrete.rs:317-390` against `Mappers/Codemasters/BF909x.h`.

**MR-71-1 — unknown, severity visible.** The one-screen mirroring bit is read with
**opposite polarity** on the two sides, so exactly one of us is wrong:

- Mesen (`BF909x.h:37`): `SetMirroringType((value & 0x10) ? ScreenAOnly : ScreenBOnly)`.
- ours (`discrete.rs:381`, `:342-346`): bit 4 set -> `SingleScreenB`, clear -> `SingleScreenA`.

This is a genuine contradiction rather than a difference of expression, and it is visible:
*Fire Hawk* is the BF9097 title that writes $9000, and it would show the wrong nametable on
whichever side is wrong. I am not able to settle it from the sources this lot may use — the
nesdev mapper-71 page is the authority REQ-CORE-100 defers to and I cannot consult it here,
and `third_party/3DNes_Engine` implements no mapper 71 to cross-check. Recorded as unknown
deliberately: calling it "our bug" and writing a test asserting Mesen's polarity would be
asserting a conclusion I have not established.

Recommended resolution for the next lot: run *Fire Hawk* and compare the status bar against
Mesen, or read the nesdev register table; the fix is a one-character change either way.

**MR-71-2 — equivalent in practice.** The decode differs in two ways, neither reachable by
a real mapper-71 game:

- before any $9000 write, Mesen also selects PRG on a write to $8000-$BFFF
  (`BF909x.h:34`, the `|| !_bf9097Mode` arm); we ignore those writes (`discrete.rs:382-386`).
  BF9093 games (*Micro Machines*, the *Dizzy* titles) bank through $C000-$FFFF only.
- after entering BF9097 mode, Mesen treats all of $8000-$BFFF as the mirroring register
  while we decode only $9000-$9FFF (`discrete.rs:379`); and Mesen enters the mode on
  `addr == 0x9000` exactly where we enter on any $9xxx. *Fire Hawk* writes $9000.

Our `single_screen` latch is saved and restored (`discrete.rs:356`, `:362`) exactly as
Mesen serializes `_bf9097Mode` (`BF909x.h:46`), and submapper 1 forcing the mode at
power-on (`BF909x.h:19`) is the one piece we lack — harmless, since the first $9000 write
sets it, but worth folding into any fix.

<!-- sections for steps 3..8 are appended as the review progresses -->
