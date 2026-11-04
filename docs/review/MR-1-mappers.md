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

## Mapper 15 — 100-in-1 Contra Function 16

`multicart.rs:99-177` against `Mappers/Unlicensed/Mapper15.h`.

**Spec question Q-110, not a bug.** Our board follows amendment **AMD-021 R2-002** to the
letter — 6-bit bank, mode 1 fixing the last 16 KiB *of the ROM*, no sub-bank XOR in mode 0 —
and Mesen contradicts AMD-021 on three observable points (7-bit bank folding the mirroring
bit in, a `^ h` XOR across the four 8 KiB windows in mode 0, and mode 1 fixing the last bank
of the current 128 KiB *block* via `bank | 0x0E`). R2-002 carries an acceptance criterion
that pins our reading (`$8001 <- $05` -> 5/63 on a 1 MiB image), so the divergence is
deliberate rather than a drafting slip. Filed as `spec/changes/Q-110.md`; no test, since a
test here would assert Mesen against a normative amendment.

Mesen additionally write-protects CHR-RAM in modes 0 and 3 (`Mapper15.h:30`); AMD-021 says
nothing about it and we do not implement it. Classified **unknown** and folded into Q-110.

Everything else agrees: mode 2's `2B + h` in all four windows, mode 3's B in both halves,
the mirroring bit, and the power-on state (first 32 KiB visible).

## Mapper 28 — Action 53

`multicart.rs:394-527` against `Mappers/Homebrew/Action53.h`.

The bank arithmetic is **equivalent**, which is worth stating precisely because the two
codes look nothing alike. Our `m = (2 << gameSize) - 1` reproduces Mesen's
`innerAnd[] = {1,3,7,15}` and our `& !m` reproduces its
`outerAnd[] = {0x1FE,0x1FC,0x1F8,0x1F0}`; our four-way `match (mode >> 2) & 0x03` splits into
Mesen's `prgSize`/`slotSelect` pair in the same order; and in the 32 KiB modes our
`bank32 + 1` equals Mesen's `(prgSelect | 0x01) & innerAnd` because `outer` is always even
and `(inner * 2) & m` is always even with `m` odd, so the increment can never carry out of
the mask. The register select `((value & 0x80) >> 6) | (value & 0x01)` is identical.

**Spec question Q-110, not a bug**, on two points where Mesen contradicts **AMD-021 R2-001**:

- the mirroring bit. R2-001 says "*tant que* le bit 1 du mode vaut 0, toute écriture dans le
  registre CHR ou PRG interne remplace le bit 0 du mode par le bit 4 de la valeur écrite" —
  exactly our `take_mirroring_bit` (`multicart.rs:417-421`), gated on `mode & 0x02 == 0` and
  writing into mode bit 0. Mesen keeps `_mirroringBit` as a *separate* latch updated on every
  register 0/1 write regardless of the mode (`Action53.h:78-82`), so a bit captured while
  two-screen mirroring was selected re-emerges when one-screen is selected again. Observable;
  filed in Q-110.
- the power-on state. R2-001 fixes outer = 63, inner = 15, mode = 0 (our `reset`,
  `multicart.rs:484-490`); Mesen zeroes all four registers. Here **AMD-021 looks right and
  Mesen looks wrong**: the Action 53 menu lives at the end of the image and the documented
  power-on selects the last 32 KiB, which 63/15 gives and Mesen's zeros do not. Mesen gets
  away with it because `SelectPrgPage(1, -1)` still puts the last 16 KiB at $C000, where the
  reset code runs.

One real gap, severity cosmetic: Mesen leaves `BaseMapper`'s default 8 KiB of work RAM at
$6000-$7FFF while we return `prg_ram_enabled() = false` (`multicart.rs:480-482`). Action 53
boards do carry optional PRG-RAM. Not pursued: no Action 53 title is known to use it and the
spec does not mention it.

## Mapper 101 — Jaleco (variant of 87)

`multicart.rs:529-603` against `Mappers/Jaleco/JalecoJfxx.h`.

**Spec question Q-110, not a bug.** Our mapper-101 path implements **AMD-021 R2-003**
exactly — "une écriture à une adresse dont les bits 13 et 14 valent 1 (`$6000-$7FFF` et
`$E000-$FFFF`) choisit la banque CHR de 8 Kio `valeur & 3`" — which is our
`address & 0x6000 == 0x6000` and `register & 0x03` (`multicart.rs:591-595`, `:552-553`), and
the `// R2-003` reference in the code says so. Mesen decodes $6000-$7FFF only and uses the
full 8-bit value. R2-003 has an acceptance criterion pinning both points, so again Q-110
rather than a bug.

## Mapper 87 — Jaleco J87

Same file. **Equivalent**: the swapped two-bit CHR select
`((value & 0x01) << 1) | ((value >> 1) & 0x01)` matches Mesen's `JalecoJfxx(false)` arm
character for character, and both decode $6000-$7FFF only.

## Mapper 34 — BNROM and NINA-001

`multicart.rs:16-97` against `Mappers/Unlicensed/Nina01.h` and `Mappers/Irem/BnRom.h`.

Mesen resolves the two boards at construction from the CHR-ROM size
(`MapperFactory.cpp:320-326`); we keep one board and branch inside `update_banks`
(`multicart.rs:37-42`). That part is fine — but the *register decode* does not branch.

**MR-34-1 — our bug, severity visible.** On a BNROM board, $7FFD-$7FFF is ordinary PRG-RAM;
only NINA-001 puts registers there (`Nina01.h:11-12` vs `BnRom.h`, which keeps the default
$8000-$FFFF range). We decode $7FFD-$7FFF unconditionally (`multicart.rs:80-89`), so on a
CHR-RAM mapper-34 image an ordinary PRG-RAM write to one of the last three bytes of $7FFF
silently switches the 32 KiB PRG bank — a crash, not a glitch, and it would hit *Deadly
Towers*-class BNROM titles that use work RAM near the top of the window. Test
`mr_34_1_bnrom_has_no_register_at_7ffd`. Fix: branch the decode on
`memory.rom().chr_rom().is_empty()` the same way `update_banks` already does.

Two smaller differences, both **equivalent in practice**:

- NINA-001's PRG field is one bit in Mesen (`value & 0x01`, `Nina01.h:21`); we take all
  eight. A correctly sized NINA-001 image is 64 KiB, where our modulo over two banks gives
  the same answer. Only an oversized image diverges.
- Mesen's `Nina01` has no $8000-$FFFF register at all, while we latch the PRG bank there
  (`multicart.rs:91-96`). No NINA-001 game writes above $8000. Both halves of this and
  MR-34-1 disappear with the same fix.

Our write-through of the NINA-001 registers to the PRG-RAM underneath (`intercept_cpu_write`
returning `false`, `multicart.rs:88`) correctly reproduces Mesen's `WritePrgRam(addr, value)`
at `Nina01.h:25`. BNROM's bus conflicts (`BnRom.h:10`) fall under MR-BUS-1.

## Mapper 41 — Caltron 6-in-1

`multicart.rs:243-317` against `Mappers/Ntdec/Caltron41.h`. **The worst board of this step:
three independent bugs, one of them stopping half the cartridge from running.**

The outer register at $6000-$67FF carries all three fields in the *address*, in the nesdev
`..MC CPPP` layout that Mesen implements directly (`Caltron41.h:33-37`): bits 0-2 the 32 KiB
PRG bank, bits 3-4 the CHR outer bank, bit 5 the mirroring. We mask the address to five bits
and then read PRG from bits 0-1 and mirroring from bit 2 (`multicart.rs:261-264`) — the same
fields, shifted by one position. No amendment covers mapper 41, so REQ-CORE-100's nesdev
behaviour governs and Mesen is the reference.

- **MR-41-1 — our bug, severity crash/hang.** The PRG field is three bits wide
  (`addr & 0x07`); ours is two (`outer & 0x03`). A Caltron 6-in-1 holds six 32 KiB games, so
  banks 4-7 exist and ours folds them back to 0-3 through `wrap`. The menu cannot start games
  5 and 6 at all — it jumps into the wrong game's code. Test
  `mr_41_1_caltron_prg_field_is_three_bits`.
- **MR-41-2 — our bug, severity visible.** Mirroring is address bit 5; our `& 0x1F` mask
  discards bit 5 entirely and we read bit 2 instead, which is a PRG bit. So mirroring is
  wrong whenever the selected game is odd-numbered in its low bit, and never settable
  deliberately. Test `mr_41_2_caltron_mirroring_comes_from_address_bit_5`.
- **MR-41-3 — our bug, severity visible.** The inner CHR register at $8000-$FFFF only
  latches while the PRG bank is 4-7; Mesen gates it (`Caltron41.h:41-45`, quoting the nesdev
  note) and we latch unconditionally (`multicart.rs:311-316`). Test
  `mr_41_3_caltron_inner_chr_is_gated_on_the_prg_bank`.

The CHR outer field itself is **equivalent**: our `((outer >> 3) & 0x03) << 2` lands address
bits 3-4 on CHR bank bits 2-3, exactly as Mesen's `(addr >> 1) & 0x0C` does.

Proposed fix: one rewrite of `Caltron6In1::intercept_cpu_write` and `update_banks` to the
`..MC CPPP` layout, keeping the full address rather than masking it to five bits, plus the
`prg_bank >= 4` gate in `write_register`.

## Mapper 30 — UNROM 512

`discrete.rs:392-453` against `Mappers/Homebrew/UnRom512.h`. PRG (`value & 0x1F`), CHR
(`(value >> 5) & 0x03`) and the one-screen polarity (bit 7 set -> screen B) all agree.

- **MR-30-1 — our bug, severity visible.** UNROM 512 encodes four mirroring choices in
  byte 6 bits 3 and 0 *together*, using the combination iNES calls illegal: 0 horizontal,
  1 vertical, 8 one-screen mapper-controlled, 9 four-screen (`UnRom512.h:45-50`). Our header
  parser collapses 8 and 9 into `FourScreen` on bit 3 alone (`ines_header.rs:170-176`), and
  the board treats any `FourScreen` header as mapper-controlled one-screen
  (`discrete.rs:405-406`), so a genuinely four-screen image loses its four nametables at
  power-on. Test `mr_30_1_unrom512_byte6_09_is_four_screen`. The fix needs the raw byte 6,
  not just the parsed enum — the cheapest route is to keep `byte6` on `InesHeader`.
- **MR-30-2 — our bug, severity cosmetic.** The board has no work RAM at all
  (`UnRom512.h:21-22`), so $6000-$7FFF is open bus; we give it the header's 8 KiB default and
  answer reads there. Test `mr_30_2_unrom512_does_not_drive_6000_7fff`. The same
  `Mapper`-level override would serve several boards (see the shared-infrastructure section).
- **MR-30-3 — our bug, severity cosmetic.** Submapper 3 toggles vertical/horizontal, not
  screen A/B (`UnRom512.h:62-66`); we always do A/B. Same code site as MR-30-1, so one fix
  covers both. No separate test.
- **MR-30-4 — our bug, severity visible (missing feature).** Mesen emulates the board's
  SST39SF040 flash when the header declares a battery (`UnRom512.h:16`, `:36`, `:107-112`):
  writes to $8000-$BFFF program PRG-ROM instead of banking, and the diff is the save file.
  Self-flashing homebrew needs it. We have no flash and no `AllowRegisterRead` equivalent, so
  reads of $8000-$FFFF never return flash status. Out of REQ-CORE-100's wording; recorded as
  a gap for whichever lot wants mapper-30 homebrew.
- **MR-30-5 — our bug, severity cosmetic.** Submapper 4 moves an LED register to
  $8000-$BFFF and must *not* bank there (`UnRom512.h:76-79`). We bank on any $8000+ write.
- **MR-30-6 — our bug, severity visible.** With a four-screen header and 32 KiB of CHR-RAM,
  Mesen maps the last 8 KiB of CHR-RAM over $2000-$3FFF (`UnRom512.h:53-57`, the
  InfiniteNesLives variation), which is how four-screen works on this board at all. We have
  no cartridge-supplied nametable path for it (`nametable_page` returns `FOLLOW_MIRRORING`).
  Depends on MR-30-1 being fixed first to be reachable.

Also **MR-BUS-1**, which Mesen gates here on "submapper 0 without battery, or submapper 2".

## Mapper 225 — 52/64-in-1 multicart

`multicart.rs:179-241` against `Mappers/Unlicensed/Mapper225.h`. **Equivalent.** The high
bit extraction (our `(latch >> 14) & 0x01` shifted to bit 6 of both fields vs Mesen's
`(addr >> 8) & 0x40`), the $1000 16-KiB-mirror / 32-KiB split, the CHR field and the $2000
mirroring bit all agree; our `map_prg_32k(prg >> 1)` expands to Mesen's
`prgPage & 0xFE` / `+1` pair.

Two cosmetic differences, neither pursued: we force vertical mirroring at power-on
(`multicart.rs:206` via the reset latch $8000) where Mesen leaves the header's layout until
the first write, and we disable the $6000-$7FFF RAM that Mesen leaves mapped by default.
The first write settles the mirroring either way.

## Mapper 228 — Action 52

`multicart.rs:319-392` against `Mappers/Unlicensed/ActionEnterprises.h`. **Equivalent** —
the chip-select fold (3 -> 2), the `((addr >> 6) & 0x1F) | (chipSelect << 5)` PRG field, the
`addr & 0x20` 16/32 KiB split, the `((addr & 0x0F) << 2) | (value & 0x03)` CHR field and the
`addr & 0x2000` mirroring bit are all identical. Same cosmetic $6000-$7FFF RAM difference as
mapper 225.

## Mapper 156 — Daou Infosys DIS23C01

`daou306.rs` against `Mappers/Unlicensed/DaouInfosys.h`. **Equivalent.** Our four-arm match
on $C000-$C00F reproduces Mesen's `bank = (addr & 0x03) + (addr >= 0xC008 ? 4 : 0)` with
`(addr & 0x04)` choosing the high or low byte; the 16-bit CHR bank `(high << 8) | low`, the
$C010 PRG bank, the $C014 mirroring bit, the fixed last 16 KiB at $C000 and the one-screen
power-on state all agree, and both ignore writes outside $C000-$C014.

## Mapper 1 — MMC1 (SxROM)

`mmc1.rs` against `Mappers/Nintendo/MMC1.h`.

Much of this board is right, and in ways that are not obvious from the code shape, so it is
worth recording what was checked and found **equivalent**:

- the serial register's bit order. Mesen shifts right and inserts at bit 4
  (`MMC1.h:45-46`), we shift left by a counter (`mmc1.rs:160`); both put the first bit
  written at bit 0.
- the reset bit's effect: Mesen sets `_prgMode` and `_slotSelect` and leaves everything else
  (`MMC1.h:38-42`), we `control |= 0x0C` (`mmc1.rs:155`) — the same two bits.
- all four PRG modes. Our `(control >> 2) & 0x03` four-way match decomposes into Mesen's
  `_prgMode`/`_slotSelect` pair in the same order, and in 32 KiB mode our
  `map_prg_32k(bank >> 1)` yields the same four 8 KiB banks as Mesen's
  `SelectPrgPage2x(0, (_prgReg & 0xFE) | prgBankSelect)`.
- both CHR modes: in 8 KiB mode our `map_chr_8k(chr0 >> 1)` equals Mesen's
  `SelectChrPage(0, _chrReg0 & 0x1E)` pair, because `8 * (chr0 >> 1) == 4 * (chr0 & 0x1E)`
  for a five-bit register.
- the two-cycle filter's unit. Mesen uses `_console->GetMasterClock()`, which for the NES is
  `_cpu->GetCycleCount()` (`NesConsole.cpp:385-387`) — CPU cycles, like our
  `memory.cpu_cycle()`. Not the master clock its name suggests.
- the power-on state (`control = 0x0C`, all other registers 0, screen A) and the PRG-RAM
  disable bit (bit 4 of $E000).

Four differences:

- **MR-1-1 — our bug, severity visible.** A write carrying the reset bit must be processed
  even within two cycles of the previous write. Mesen checks bit 7 *first*
  (`MMC1.h:208-213`) with the comment "If the reset bit is set, process the write even if
  another write just occurred (fixes bug in Shinsenden)". We apply the filter before looking
  at the value (`mmc1.rs:146-151`), so the reset is swallowed and the board keeps a PRG mode
  the game believes it has cleared. Affects *Shinsenden* by name. Test
  `mr_1_1_mmc1_reset_bit_bypasses_the_consecutive_write_filter`.
- **MR-1-2 — our bug, severity subtle.** On a 512 KiB board the PRG A18 line comes from bit
  4 of the CHR register written *last*: `extraReg = (_lastChrReg == 0xC000 && _chrMode) ?
  _chrReg1 : _chrReg0` (`MMC1.h:124`). We always read `chr0` (`mmc1.rs:37-41`). In 8 KiB
  CHR mode Mesen's expression reduces to `_chrReg0`, so the SUROM titles Mesen names
  (*Dragon Warrior III*, *IV*) agree with us; only 4 KiB CHR mode with $C000 written last
  diverges, and then we select the wrong 256 KiB half of the ROM. The same `extraReg` feeds
  the SXROM PRG-RAM bank (`MMC1.h:136`), where we also read `chr0` unconditionally
  (`mmc1.rs:129-131`), so one fix — tracking the last CHR register written — covers both.
  Test `mr_1_2_mmc1_suprom_outer_bank_follows_the_last_chr_register`.
- **MR-1-3 — our bug, severity subtle. No test (fixture gap).** The PRG-RAM bank field
  depends on the RAM size, and SOROM is the case we get wrong. Mesen
  (`MMC1.h:134-150`): above 16 KiB (SXROM, 32 KiB) the bank is `(extraReg >> 2) & 0x03`;
  at exactly 16 KiB with an 8 KiB battery half (SOROM) *bit 3* chooses which of the two
  chips answers; at 8 KiB there is no banking; at 0 the window is unmapped. We use
  `(chr0 >> 2) & 0x03` for anything above 8 KiB (`mmc1.rs:127-133`), which is right for
  SXROM but puts SOROM's selector on bit 2, so battery-backed saves land in the volatile
  half and back. No test because `MapperRom` has no PRG-RAM-size knob and a 16 KiB
  declaration needs an NES 2.0 byte 10 — adding that knob is the first step of any fix.
- **MR-1-4 — our bug, severity cosmetic.** NES 2.0 submapper 5 is the fixed-PRG MMC1
  (SEROM, SHROM, SH1ROM): no banking at all (`MMC1.h:152-156`). We have no submapper
  handling on this board, so the PRG register still banks a 32 KiB image into halves. Those
  boards' games never write the register meaningfully, hence cosmetic. Test
  `mr_1_4_mmc1_submapper_5_has_fixed_prg`.

Two further notes, neither a bug:

- the 512 KiB test. Mesen requires `_prgSize == 0x80000` exactly (`MMC1.h:126`); we use
  `> 256 * 1024` (`mmc1.rs:38`), so a 384 KiB image would get the outer bit from us and not
  from Mesen. No such MMC1 dump exists. **Equivalent** in practice.
- the MMC1A/MMC1B revision defaults. Mesen reads the board name from its game database to
  decide whether PRG-RAM starts disabled (MMC1B) or can never be disabled (MMC1A)
  (`MMC1.h:189-194`). We have no game database, which matches Mesen's own behaviour for any
  ROM the database does not list. **Equivalent** for synthetic and unlisted ROMs; a
  deliberate gap rather than an error.
- a soft reset. Mesen's MMC1 does not override `Reset`, so its registers survive a soft
  reset; our `Mapper::reset` restores the power-on values (`mmc1.rs:112-124`), as
  REQ-CORE-004 asks of the cartridge. **Unknown** which matches the chip; harmless in
  practice because a game's reset path must reprogram the serial register anyway, its state
  being unknowable.

## Mappers 9 and 10 — MMC2 (PxROM) and MMC4 (FxROM)

`mmc2.rs` against `Mappers/Nintendo/MMC2.h` and `MMC4.h`. **Equivalent**, including the
details most likely to be wrong:

- the latch trigger addresses, which are *asymmetric* on MMC2 and symmetric on MMC4. Mesen's
  MMC2 matches `$0FD8`/`$0FE8` exactly for the left latch but the ranges `$1FD8-$1FDF` /
  `$1FE8-$1FEF` for the right (`MMC2.h:124-132`); MMC4 widens both halves to ranges
  (`MMC4.h:28-38`). Our `move_latches` reproduces exactly that split on `is_mmc4`
  (`mmc2.rs:49-73`).
- the PRG layouts: MMC2's four 8 KiB windows with three fixed at -3/-2/-1, MMC4's 16 KiB
  switchable plus fixed last (`mmc2.rs:33-41`).
- register widths (`value & 0x0F` for PRG, `& 0x1F` for the four CHR registers), the $F000
  mirroring bit polarity, and both latches powering on selecting the $FE register.
- **the deferred CHR update is equivalent, not a difference.** Mesen does not apply the new
  bank when the latch flips: it sets `_needChrUpdate` and applies it on the next VRAM
  address change (`MMC2.h:118-123`). We apply it at once in `set_latch` (`mmc2.rs:85-86`).
  The observable state is the same, because the triggering fetch has already read its byte
  in both designs — ours reads through `ppu_peek` before moving the latches
  (`mmc2.rs:160-164`), and Mesen's hook runs before the next read uses the bank. This looks
  like a timing difference and is not one.

One difference, classified **unknown**, severity cosmetic: Mesen drives its latch hook from
every PPU address-bus change, so a bare `$2006` write of `$0FD8` flips the latch with no
pattern fetch at all; we hook `ppu_read`, so only a real CHR read moves it. Whether the
MMC2's PA0-PA13 comparator sees an address the PPU loaded but never used for a fetch is a
PPU-bus question the sources allowed here do not settle. REQ-CORE-033 and REQ-CORE-039
constrain our side anyway: the latches must move on real rendering only, never on a peek,
which is what `advance_chr_latch` and `chr_latch_state` exist for and which the current
design gets right.

## Mapper 4 — MMC3 (TxROM) and MMC6

`mmc3.rs` against `Mappers/Nintendo/MMC3.h`.

The core of the board is **equivalent**, and two of the things most often got wrong are
right here:

- **the IRQ revision.** Ours asserts on `counter == 0 && enabled` (`mmc3.rs:86-88`), which
  is rev B — what REQ-CORE-100 requires ("comportement « rev B »") and what Mesen's default
  path does (`MMC3.h:313-315`). Mesen's rev A path exists but is gated on its game database
  reporting an `MMC3A` chip (`MMC3.h:198-200`), which we have no equivalent of and which
  Mesen itself does not take for an unlisted ROM. REQ-CORE-114 explicitly permits
  `mmc3_test_2` test 6 ("MMC3 alt", the rev A test) to fail. Not a finding.
- **register 0/1 bit 0.** Mesen masks the written value (`MMC3.h:218-221`,
  "Writes to registers 0 and 1 always ignore bit 0") while we store it whole and drop the
  bit in the mapping (`mmc3.rs:71-73`). The resulting 1 KiB banks are identical —
  `map_chr_2k(slot, b >> 1)` gives `b & 0xFE` and `(b & 0xFE) + 1`, and Mesen's
  `_registers[0] & 0xFE` / `| 0x01` on an already-masked value gives the same pair. Only the
  stored register byte differs, which nothing observes. **Equivalent.**

Both CHR modes, both PRG modes, the four-screen guard on $A000, the non-MMC6 PRG-RAM enable
and write-protect bits ($A001 bits 7 and 6) and the MMC6 window mirroring every $400 were
checked and agree. The save state covers the same fields as Mesen's, plus a deliberate
`update_banks()` on load whose reason is documented in the code.

Findings:

- **MR-4-1 — our bug, severity visible.** The MMC6 $A001 read-enable and write-enable bits
  are swapped. Mesen (`MMC3.h:142-144`) takes the *lower* bit of each pair as the write
  enable and the upper as the read enable — bit 4 writes / bit 5 reads the low 512 bytes,
  bit 6 writes / bit 7 reads the high 512 — which is also how the nesdev MMC6 $A001 table
  reads. We have it the other way round (`mmc3.rs:51-57`). The MMC6 games are *StarTropics*
  and *Zoda's Revenge*, and both use the RAM for saves, so this is a save-corruption bug
  rather than a cosmetic one. Test
  `mr_4_1_mmc6_a001_read_and_write_bits_are_not_swapped`.
- **MR-4-2 — our bug, severity subtle.** A12 must be observed *low* before any rise can
  count. Mesen's MMC3 guards its detector with `_a12LowClock > 0` (`MMC3.h:288`); our shared
  `A12Watcher` starts at `low_since = 0` and compares against it (`a12_watcher.rs:36-39`,
  `:45`), so the first high address at a clock of 10 dots or more is reported as a rise.
  That is one spurious clock of the scanline counter at every power-on and at every
  `reset()` — which also re-zeroes `low_since` while the PPU clock keeps running, so a soft
  reset mid-frame can produce a second one. IRQs are disabled at power-on so no interrupt
  fires, but the counter is left one step off the hardware's, which shifts the first
  programmed scanline IRQ. Test `mr_4_2_mmc3_a12_needs_a_fall_before_the_first_rise`.
- **MR-4-3 — our bug, severity cosmetic.** Mapper 4 submapper 3 is not an MMC3 at all:
  Mesen builds `McAcc` (the Acclaim MC-ACC, whose A12 filter sits at a different point in
  the chain) at `MapperFactory.cpp:285-290`. We always build `Mmc3` (`registry.rs:48`). The
  affected dumps are a handful of Acclaim carts.
- **MR-4-4 — unknown, severity cosmetic.** The power-on bank registers differ. Mesen seeds
  them to `0,2,4,5,6,7,0,1` (`MMC3.h:63-70`, via `GetPowerOnByte(default)`), giving a linear
  CHR layout; we zero all eight but register 7 (`mmc3.rs:180-181`), giving
  `0,1,0,0,0,0,0,0`. The PRG pair agrees, which is what matters for booting, and every MMC3
  game programs all eight registers during init with rendering off. Neither value is
  hardware-documented.
- **MR-4-5 — unknown, severity cosmetic.** Our power-on `$A001 = 0x80` (PRG-RAM enabled and
  writable) against Mesen's `GetPowerOnByte()` = 0 (disabled). Our code already argues the
  case in a comment (`mmc3.rs:176-178`): it is the only value that lets a game use the RAM
  from a cold start without writing $A001. Recorded as a difference with our rationale
  attached, not as a bug — it is a deliberate choice about an undocumented power-on value,
  and REQ-CORE-114's test ROMs constrain it.
- **MR-4-6 — our bug, severity cosmetic.** The MMC6 carries 1 KiB of RAM
  (`MMC3.h:184-185`: `GetSaveRamSize()` = `0x400`, page size `0x200`); we expose the 8 KiB
  default. The window mapping is unaffected because `mmc6_offset` masks with `0x03FF` and
  the `% prg_ram.len()` is then the identity (`mmc3.rs:45-47`), so only the exposed and
  saved RAM size is wrong. Same family as MR-13-1 and MR-30-2; one shared override fixes all
  three.

## Shared: the A12 watcher

`a12_watcher.rs` against `Mappers/A12Watcher.h` — and against Mesen's MMC3, which pointedly
does **not** use that class.

Our filter is one constant: A12 must have been low for `FILTER_DOTS = 10` PPU dots
(`a12_watcher.rs:27`, `:45`). Mesen has three different answers:

| user | threshold | unit |
| --- | --- | --- |
| `A12Watcher.h` default (`minDelay = 10`, `_cyclesDown > minDelay`) | >= 11 | PPU dots (frame cycles) |
| `MMC3::IsA12RisingEdge` (`MMC3.h:286-296`) | >= 3 | **CPU cycles** (`GetMasterClock()`) |
| `Rambo1::NotifyVramAddressChange` (`Rambo1.h:166`, `UpdateVramAddress<30>`) | >= 31 | PPU dots |

So Mesen's own MMC3 and its shared watcher disagree with each other, and RAMBO-1 wants a
filter three times longer than either. Our single 10-dot constant sits between the first two
and is far short of the third (see MR-64-3).

**Classification: unknown** for the MMC3/shared-watcher spread — the real filter is an RC
delay on the cartridge and all three figures are approximations of it; REQ-CORE-114's
acceptance is `mmc3_test_2` tests 1-5, which the current constant satisfies. The RAMBO-1
figure is a different matter and is recorded as MR-64-3, because 10 against 31 is not an
approximation of the same number.

Two structural notes for whoever changes this:

- `A12Watcher::MIN_RISE_INTERVAL` (`a12_watcher.rs:31`) is derived from `FILTER_DOTS` and
  feeds `Mmc3::ppu_irq_horizon` (`mmc3.rs:167`), which is how far the PPU may run behind the
  CPU. The two must move together or the horizon becomes a wrong bound and an IRQ lands
  late. This is the one place in the lot where an accuracy change has a knock-on effect on
  a scheduling decision.
- Mesen's class takes the threshold as a template parameter precisely because its users
  disagree. Ours needs the same: a `FILTER_DOTS` passed in at construction, not a constant.
- Mesen's frame-relative clock forces it to carry an explicit frame-wrap correction
  (`A12Watcher.h:31-38`, the `89342 - _lastCycle` term). Our monotone `ppu_clock` has no
  such case, which is **simpler and equivalent** — worth keeping.

## Mapper 64 — Tengen RAMBO-1

`rambo1.rs` against `Mappers/Tengen/Rambo1.h`. The PRG layout (three switchable 8 KiB
windows with the fixed one at $E000, and the $40 bit rotating which window takes register
15) and the CHR 1 KiB mode with its A12 inversion are **equivalent** — our
`high`/`low` pair reproduces Mesen's `slot ^ a12Inversion` for all eight slots, which is
worth stating because the two expressions look unrelated.

The IRQ, though, is a different counter from the MMC3's, and we implemented it as if it
were the same one.

- **MR-64-1 — our bug, severity visible.** RAMBO-1 *always* decrements, and the reload
  loads the latch plus two — plus *one* when the latch is 0 or 1 (`Rambo1.h:76-91`, the
  asymmetry carrying the comment "Fixes Hard Drivin'"). The period is therefore one clock
  longer than an MMC3's. We reuse the MMC3 shape, assigning the latch with no decrement
  (`rambo1.rs:72-83`), so every RAMBO-1 IRQ arrives one scanline early: split-screen status
  bars sit one line off in *Klax*, *Skull & Crossbones*, *Rolling Thunder* and
  *Hard Drivin'*. Test `mr_64_1_rambo1_counter_always_decrements`.
- **MR-64-2 — our bug, severity visible.** The 2 KiB CHR windows do not ignore bit 0 of the
  register: the pair is `reg` and `reg + 1` (`Rambo1.h:112-113`). Mesen is deliberately
  different from its own MMC3 on this point — `MMC3.h:89-90` masks with `0xFE` and
  `Rambo1.h` does not — which is strong evidence the asymmetry is real and not an oversight.
  Our `map_chr_2k(slot, b(0) >> 1)` (`rambo1.rs:63-64`) drops the bit, so an odd register
  shows the wrong kilobyte pair. Test `mr_64_2_rambo1_2kib_chr_window_keeps_bit_0`.
- **MR-64-3 — our bug, severity subtle.** The A12 filter is `minDelay = 30`
  (`Rambo1.h:166`), not the 10 we share with the MMC3, so we count rises this board ignores
  and the scanline counter runs fast whenever A12 dips for less than 31 dots. Test
  `mr_64_3_rambo1_a12_filter_is_30_dots`. Fix: make the threshold a construction parameter
  of `A12Watcher` (see the shared section).
- **MR-64-4 — our bug, severity subtle.** The IRQ assertion is delayed: 2 CPU cycles when
  the counter was clocked by A12, 1 when clocked by the CPU divider (`Rambo1.h:13-14`,
  `:55-60`, the `_needIrqDelay` countdown in `ProcessCpuClock`). We set `irq_pending`
  immediately (`rambo1.rs:81`). No test: observing a one-to-two-cycle offset needs a
  cycle-level IRQ probe the fixture does not have, and the fix is mechanical once MR-64-1 is
  in.
- **MR-64-5 — our bug, severity visible.** Leaving cycle mode must grant one more divider
  clock. Mesen sets `_forceClock` when $C001 is written with bit 0 clear *while already in
  cycle mode* (`Rambo1.h:137-142`), quoting FHorse: "after the write in the reg $C001, are
  needed more than four CPU clock cycles before the switch takes place, allowing another
  clock of irq running the reload" — and says it fixes *Skull & Crossbones*. We have no such
  latch and reset the divider on every $C001 write (`rambo1.rs:170-174`) where Mesen zeroes
  it only when *entering* cycle mode. One fix covers both halves.

One difference where **Mesen looks like the one in error**: its `$8000` case sets
`_currentRegister` and does *not* call `UpdateState` (`Rambo1.h:122-124`), so changing the
CHR-inversion or PRG-rotation bits only takes effect on the next `$8001` write. Its own MMC3
does call `UpdateState` from `$8000` (`MMC3.h:214-216`), and the bank selection is
combinational on hardware, so a mode change should be visible at once — which is what we do
(`rambo1.rs:156-159`). Recorded as **Mesen in error**; no change on our side.

## Mapper 5 — MMC5 (ExROM)

`mmc5.rs` and `apu/mmc5_audio.rs` against `Mappers/Nintendo/MMC5.h` and
`Mappers/Audio/Mmc5Audio.h`.

### What the spec already settles

Two large pieces of this board are governed by the spec rather than by Mesen, and checking
that first removed three findings I had drafted:

- **ExRAM extended attributes and the vertical split are P2**, not missing features.
  REQ-CORE-100's mapper-5 row reads "au minimum : modes de banques PRG et CHR, PRG-RAM, IRQ
  de ligne. ExRAM, attributs étendus, split vertical : P2". Mesen implements the split in
  full (`MMC5.h:481-520`, `_verticalSplitEnabled`, `_splitTileNumber`, `$5200-$5202`) and we
  implement none of it. That is a prioritised gap the spec itself records, so it is neither a
  bug nor a question. The board's own doc comment says so (`mmc5.rs:8-9`).
- **the expansion audio is pinned to the integer by AMD-054**, including constants the
  amendment says outright are the reference program's and unlike public documentation.

### PRG banking and PRG-RAM — equivalent

Checked in detail and agreeing with Mesen, which is worth recording because the two
expressions are structurally different:

- all four PRG modes and their masks. Mesen masks per mode at the call site
  (`value & 0x7C` for the 32 KiB window, `& 0xFE` / `& 0x7E` for the 16 KiB ones,
  `MMC5.h:160-170`); we derive the same masks from the window count in `map_prg_window`
  (`mmc5.rs:150`, `first = bank & !(windows - 1)`).
- **which window can show RAM.** Mesen's rule is register-based: RAM when
  `((bankNumber & 0x80) == 0 && reg != 0x5117) || reg == 0x5113` (`MMC5.h:80`), so $5117 is
  always ROM. Ours is window-based: `register & 0x80 != 0 || slot + windows == PRG_WINDOWS`
  (`mmc5.rs:144`). These are the same predicate, because the window that reaches $FFFF is
  exactly the one fed by $5117 in every one of the four modes. Equivalent.
- the $5113 RAM bank masked to three bits, the $5102/$5103 write-protect pair
  (`protect1 & 3 == 2 && protect2 & 3 == 1`, `mmc5.rs:338`, matching `MMC5.h:82`), and
  $6000-$7FFF always readable.
- **the 64 KiB PRG-RAM.** We force 64 KiB regardless of the header
  (`registry.rs:33-35`, `Mmc5::MAX_PRG_RAM_SIZE`); for a plain iNES MMC5 image Mesen does
  the same thing by a different route — "Emulate as if a single 64k block of work/save ram
  existed" (`MMC5.h:305-330`). Equivalent for iNES. For an NES 2.0 image Mesen honours the
  declared sizes and adds an "empty socket" rule returning open bus for banks 4-7 when the
  board is not exactly 2x8 KiB (`MMC5.h:96-110`); we ignore both. Recorded as a difference,
  severity cosmetic, not pursued — no licensed MMC5 board does this, as Mesen's own comment
  notes.

### Findings

- **MR-5-1 — our bug, severity visible.** With 8x8 sprites the PPU must use the "A" CHR
  register set for everything; the last-written-set rule is the *not-rendering* rule, not the
  8x8 rule. Mesen's predicate (`MMC5.h:192`) is
  `!largeSprites || (sprite fetch window) || (!_ppuInFrame && _lastChrReg <= 0x5127)` — three
  separate clauses. We collapsed it to two (`mmc5.rs:163-167`), using `last_set_was_b` where
  the first clause belongs, so an 8x8-sprite game that last wrote $5128-$512B renders its
  whole screen from set B. Test `mr_5_1_mmc5_8x8_sprites_always_use_chr_set_a`. The fix is to
  restore the third clause and gate it on `in_frame`, which we already track.
- **MR-5-2 — our bug, severity subtle.** In ExRAM modes 0 and 1 a CPU write to
  $5C00-$5FFF outside rendering stores **zero**, not the value (`MMC5.h:420-426`). We store
  the value in every mode but 3 (`mmc5.rs:412-418`). Observable through the nametable path,
  since those are the modes that show ExRAM to the PPU. Test
  `mr_5_2_mmc5_exram_write_outside_rendering_stores_zero`.
- **MR-5-3 — unknown, severity subtle.** The scanline counter is off by one against Mesen.
  Mesen's first in-frame detection sets `_scanlineCounter = 0` and each later line
  increments it (`MMC5.h:430-445`), so target 1 fires on the second detected line; we set
  `scanline_counter = scanline + 1` from the PPU's own signal (`mmc5.rs:487`), so target 1
  fires on visible line 0. One of the two is a line early. I cannot settle it here:
  REQ-CORE-100 asks only for "IRQ de ligne" on this board and pins no line, unlike
  REQ-CORE-114 for the MMC3. A later lot should settle it against a split-screen title
  (*Castlevania III*, *Just Breed*) or an MMC5 test ROM. No test, because a test would have
  to assert one of the two conventions.
- **MR-5-4 — unknown, severity subtle.** The in-frame detection mechanism differs. Mesen
  infers it from the PPU address bus — three identical nametable reads then the attribute
  fetch (`DetectScanlineStart`, `MMC5.h:429-457`) — plus a `_ppuIdleCounter` that drops
  `_ppuInFrame` after three idle CPU cycles and an `$FFFA-$FFFB` read hook
  (`MMC5.h:333`) to catch the frame end. We are told directly by the PPU through
  `on_ppu_scanline(scanline, rendering)` (`mmc5.rs:480-494`). For a game that renders
  normally the two agree; they diverge when rendering is toggled mid-frame or driven by
  `$2006` writes, which is precisely what the MMC5's bus-sniffing design exists to survive.
  Ours is much simpler and is the design the rest of our PPU interface assumes; recorded so
  that a later lot knows the mechanism is a deliberate simplification, not an oversight.
- **MR-5-5 — our bug, severity cosmetic.** ExRAM reads from the CPU in modes 0 and 1 return
  **open bus** on Mesen (its mapping has no read access), while we drive a 0
  (`mmc5.rs:395-399` returns `Some(Some(0))` where `Some(None)` is our open-bus answer).
  One-line fix; same family as MR-30-2.

### Expansion audio

Our implementation follows **AMD-054** (REQ-CORE-075 amended) to the integer, and the
existing `tests/mmc5_audio_tests.rs` covers its numeric criteria. Compared with Mesen:

| point | AMD-054 (and our code) | Mesen `Mmc5Audio.h` | status |
| --- | --- | --- | --- |
| envelope cadence | 240 Hz, every 7458 CPU cycles | 240 Hz (`masterClockRate / 240`) | equivalent |
| length-counter cadence | 120 Hz (every other envelope tick) | 240 Hz, same tick | **spec deviation, already marked (D)** |
| no sweep unit | yes | yes (`InitializeSweep` empty) | equivalent |
| $5011 write of 0 | keeps the level, raises the PCM IRQ flag | keeps the level (IRQ a TODO) | equivalent on the level |
| $5010 read mode | not emulated | not emulated (TODO) | equivalent |
| PCM IRQ on the CPU line | never asserted | not implemented | **spec deviation, already marked (D)** |
| mix | `M = r(Km·P(q1+q2)) + 31·pcm`, `Km = 8000/P(30)` | delta mixer, squares at APU pulse volume, PCM added 1:1 | **spec deviation, covered by AMD-054's note** |
| period below 8 | silences the channel | does **not** silence | **Q-111** |
| polarity | positive, added to `A` | negated | **Q-111** |

The last two are the only audio points where AMD-054 contradicts nesdev and Mesen **without**
carrying a `(D)` mark or a note, while the amendment is otherwise scrupulous about flagging
exactly that. Mesen quotes the nesdev sentences verbatim in both places
(`Mmc5Audio.h:35` and `:78-79`) and even carries an `_isMmc5Square` flag through its shared
pulse class to disable the period-8 cutoff for this board specifically; we get the cutoff
because our MMC5 squares reuse `PulseChannel`, whose `muted()` tests `timer_period < 8`
(`apu/pulse_channel.rs:66-68`). Filed as `spec/changes/Q-111.md`. The spec prevails, so no
code changes and no tests: if the answer keeps AMD-054, only the `(D)` marks are missing.

Polarity is the one of the two with audible consequences: an inverted expansion channel
subtracts from the APU's mix instead of adding to it, so a passage where both sound partly
cancels rather than reinforcing. AMD-054's numeric criteria are positive, so it cannot be
flipped without reopening the amendment and REQ-CORE-073's `S = A + M`.

### Save state

Ours covers every register, the 1 KiB ExRAM, both CHR register sets, the IRQ state and the
audio, with the audio appended **last** specifically so that a pre-AMD-054 block still loads
(`mmc5.rs:264-266`, `:300-307`) — a versioning care Mesen does not need and we do.
Equivalent coverage to Mesen's `Serialize`, which additionally keeps the split-screen fields
we have no equivalent of.

## Mappers 21, 22, 23, 25, 27 — Konami VRC2 and VRC4

`vrc2_vrc4.rs` against `Mappers/Konami/VRC2_4.h` and `Mappers/Konami/VrcIrq.h`.

Mesen models nine variants of the same silicon (`VRCVariant`) and resolves them from the
mapper number *and* the NES 2.0 submapper (`DetectVariant`, `VRC2_4.h:37-77`). When the
submapper is 0 it falls back on a heuristic that **ORs together** the two candidate wirings
for that mapper number, with the comment "ORing both values should make most games work"
(`VRC2_4.h:208-247`). When the submapper is known — and unconditionally for mappers 22 and
27 — it uses one exact wiring (`VRC2_4.h:249-305`).

Our `register_index` (`vrc2_vrc4.rs:52-59`) is **exactly Mesen's three heuristics**, applied
always. Line for line: our mapper-21 arm is its mapper-21 heuristic, our `22 | 25` arm its
mapper-25 heuristic, our fallback arm its mapper-23 heuristic. For mappers 21, 23 and 25
without a submapper we therefore agree with Mesen completely — which is the common case.

Two of the five numbers are wrong, and both are the ones Mesen never heuristicises:

- **MR-21-1 — our bug, severity visible.** Mapper 22 is VRC2a, whose wiring is known
  exactly (`A0 = (addr >> 1) & 1`, `A1 = addr & 1`, no OR term). We route it through the
  mapper-25 heuristic, so address bits 2 and 3 leak into the register index and we decode a
  different register for the same address. Test
  `mr_21_1_vrc2a_uses_the_exact_register_wiring`.
- **MR-21-2 — our bug, severity visible.** The VRC2 has no PRG swap mode: all of
  $9000-$9003 is the mirroring register (`VRC2_4.h:165-166`), and only VRC4 splits
  $9002-$9003 off as the mode (`:177-178`). We split on every variant
  (`vrc2_vrc4.rs:191-200`), so on mapper 22 a write to $9003 swaps the PRG layout instead of
  setting mirroring. Test `mr_21_2_vrc2a_has_no_prg_swap_mode`.
- **MR-21-3 — our bug, severity subtle.** Mapper 27 gets the mapper-23 heuristic from us
  where Mesen uses the exact VRC4_27 wiring; and no submapper is honoured on any of the five
  numbers, so a correctly headered mapper-21 submapper-2 (VRC4c) or mapper-25 submapper-2
  (VRC4d) image decodes through the OR'd pair rather than its own lines. Same root cause as
  MR-21-1; one `DetectVariant` equivalent fixes all of it. No separate test.

Not findings, on the spec's authority: mapper 27's eight-bit CHR high field
(`vrc2_vrc4.rs:36-38`, against Mesen's uniform `value & 0x1F`) and the VRC2 mirroring mask
are both recorded by **AMD-021 R2-004** in its table of *accepted* divergences — "Comportement
nesdev pour le mapper 27 ; différences de largeur CHR (>= 256 Kio) et de mirroring VRC2 sans
effet sur les cartouches commerciales".

The IRQ is **equivalent** to Mesen's shared `VrcIrq`: the same `-= 3` / `+= 341` prescaler
with the same `<= 0` threshold, the same `0xFF`-triggered reload, the same control-register
bit assignment, and the same acknowledge semantics (`$F003` restoring `irq_enabled` from the
after-ack bit). Two small differences, neither pursued:

- in cycle mode Mesen still decrements and re-adds the prescaler (`VrcIrq.h:46-56`) while we
  leave it untouched (`vrc2_vrc4.rs:231-234`). Mesen's bookkeeping grows the counter by 338
  per cycle, bounded only by `SetControlValue` resetting it to 341; ours stays put. No
  observable difference, and ours is the more defensible of the two.
- the VRC2 microwire latch at $6000-$6FFF — one writable bit readable back in bit 0
  (`VRC2_4.h:107-110`, `:147-151`) — exists on Mesen when the board declares no work RAM. We
  give every board 8 KiB of PRG-RAM there instead. Cosmetic; same family as MR-30-2.

## Mapper 16 — Bandai FCG

`bandai_fcg.rs` against `Mappers/Bandai/BandaiFcg.h`.

The register file agrees: eight CHR registers on $x000-$x007, PRG on $x008, the four-way
mirroring on $x009 in the same order, and the IRQ control/low/high trio on $x00A-$x00C. The
register window is $6000-$FFFF on both sides.

**The IRQ counter is equivalent**, which is worth showing because the two codes look
opposite. Mesen tests before decrementing (`BandaiFcg.h:131-139`, with a comment that this
is the only way to satisfy both *Famicom Jump II* and *Magical Taruruuto-kun 2*); we
decrement and test for underflow (`bandai_fcg.rs:129-135`). Tracing a counter of 2: Mesen
fires on the third cycle (2 -> 1 -> 0, then 0 == 0), and so do we (1, 0, then -1 < 0). Both
wrap to 0xFFFF afterwards. Same cycle, same period.

- **MR-16-1 — our bug, severity subtle.** On the FCG-1/2 (NES 2.0 submapper 4) $x00B and
  $x00C write the **counter** directly and there is no latch; only the LZ93D50 (submapper 5)
  has the latch that $x00A copies across. Mesen branches on it in all three register cases
  (the `_romInfo.SubMapperID != 4` guards), quoting the wiki: "all such games therefore
  disable counting before changing the counter value". We always write the latch
  (`bandai_fcg.rs:119-121`) and always reload on $x00A (`:114-118`). The two models agree
  while a game writes the value *before* enabling — which is why this has not shown up — and
  diverge as soon as one is written mid-count. Test
  `mr_16_1_bandai_submapper_4_writes_the_counter_directly`.
- **MR-16-2 — our bug, severity cosmetic.** On an image of 512 KiB or more Mesen builds a
  PRG outer bank out of bit 0 of all eight CHR registers (`BandaiFcg.h:165-173`,
  `GetPrgPageCount() >= 0x20`); we have no such path, so the upper half is unreachable. No
  commercial mapper-16 image is that large — the feature belongs to mapper 153, out of scope.
- **MR-16-3 — our bug, severity subtle.** On a CHR-RAM board the CHR registers must not
  bank: Mesen gates the `SelectChrPage` on `!HasChrRam()` (`BandaiFcg.h:174-176`) and we
  always bank (`bandai_fcg.rs:106-109`).

The **EEPROM at $x00D is not a finding**: REQ-CORE-100's mapper-16 row reads "IRQ ; EEPROM :
P3", so its absence is a prioritised gap the spec records, and the code already says so
(`bandai_fcg.rs:124`). Mesen implements the full 24C01/24C02 pair plus the Datach barcode
reader (`BaseEeprom24C0X.h`, `Eeprom24C01.h`, `Eeprom24C02.h`) — see the shared-infrastructure
list.

## Mapper 18 — Jaleco SS88006

`jaleco_ss88006.rs` against `Mappers/Jaleco/JalecoSs88006.h`.

The nibble-pair register file is **equivalent** throughout: three PRG banks on
$8000-$9001 with $9002-$9003 dead, eight CHR banks on $A000-$D003, the four IRQ latch
nibbles on $E000-$E003, and $F000-$F002 for reload, control and mirroring. Our
`set_nibble` plus `group`/`index` decomposition reproduces Mesen's `addr & 0xF003` switch
and its `updateUpperBits` flag exactly, the counter-width selection has the same priority
order (bit 3 before bit 2 before bit 1), and the mirroring order is the same unusual
horizontal-first one.

- **MR-18-1 — our bug, severity subtle.** The IRQ fires on the cycle whose decrement
  *produces* zero; ours fires on the cycle that observes a zero already there, so every
  period is one CPU cycle long. Mesen is `if(--counter == 0)` (`JalecoSs88006.h:91-100`); we
  test `counted == 0` before subtracting (`jaleco_ss88006.rs:171-179`). We also reload the
  masked field to all-ones on firing where Mesen lets it wrap naturally — the same value,
  reached a cycle apart. Test `mr_18_1_jaleco_ss88006_irq_fires_one_cycle_earlier`.

$F003 is the board's expansion audio, which neither side emulates — and REQ-CORE-075 leaves
it optional, so that is not a finding on either.

## Shared: the power-on bank layout

Three boards reviewed in this step, plus the Irem G-101, power on with a **linear** bank
layout on our side and an **all-zero** one on Mesen's:

| board | ours | Mesen |
| --- | --- | --- |
| VRC2/VRC4 | `prg0 = 0`, `prg1 = 1` (`vrc2_vrc4.rs:40-41`) | both `GetPowerOnByte() & 0x1F` = 0 |
| Irem G-101 | `prg0 = 0`, `prg1 = 1` (`iremg101.rs:21-22`) | `_prgRegs[0] = _prgRegs[1] = 0` |
| Jaleco SS88006 | `prg = [0, 1, 2]` (`jaleco_ss88006.rs:31`) | `memset(_prgBanks, 0, 3)` |
| MMC3 | `bank[7] = 1`, rest 0 (`mmc3.rs:180-181`) | `0,2,4,5,6,7,0,1` |

**Unknown, severity cosmetic** in every case. None of these values is hardware-documented;
all four boards have the last PRG bank fixed at $E000, so the reset vector is fetched
correctly either way, and every game programs its bank registers before enabling rendering.
Recorded once here rather than four times, and flagged because a future lot comparing
power-on screenshots against Mesen will see differences that are not bugs.

## Mapper 32 — Irem G-101

`iremg101.rs` against `Mappers/Irem/IremG101.h`. The two PRG registers, the $9000 mode and
mirroring pair, the eight 1 KiB CHR registers on $B000-$B007 and the fixed banks at $C000
and $E000 all agree.

- **MR-32-1 — our bug, severity visible.** NES 2.0 submapper 1 is *Major League*, whose
  board ties CIRAM A10 high: the mirroring is fixed one-screen and the PRG mode is forced to
  0. Mesen handles both halves (`IremG101.h:23-27` and `:60-62`); we have no submapper
  handling on this board, so *Major League* takes its mirroring from the header and then
  from every $9000 write, and can swap its PRG layout. Test
  `mr_32_1_irem_g101_submapper_1_is_fixed_one_screen`.

One difference left as **equivalent in practice**: Mesen masks both PRG registers to five
bits (`& 0x1F`) and we keep all eight (`iremg101.rs:79`, `:83`). The G-101 tops out at
256 KiB, where our modulo gives the same bank; only an oversized image would diverge.

---

## Shared infrastructure Mesen has and we lack

REQ-CORE-102 permits adding mappers beyond the required list, and REQ-CORE-101 leaves
mapper 116 optional. Everything below is machinery Mesen factors out and we would have to
build; the ones marked **blocking** are needed by more than one of this lot's own fixes, so
they are worth building first.

1. **An MMC3 variant base** — *blocking for breadth, the single biggest gap*.
   `Core/NES/Mappers/Mmc3Variants/` holds roughly thirty boards that subclass `MMC3` and
   override `UpdatePrgMapping`, `UpdateChrMapping`, `UpdateMirroring`, `TriggerIrq` or
   `ForceMmc3RevAIrqs` (mappers 12, 14, 37, 44, 45, 47, 49, 52, 114, 115, 118, 119, 165,
   182, 189, 245, 249, 250 and more). Our `Mmc3` is a concrete struct with no extension
   points, and `Board` is a closed enum. Any future MMC3-derived mapper means either copying
   `mmc3.rs` or giving it hooks. Mapper 116 (REQ-CORE-101) and TxSROM (118) both land here.
2. **Per-board memory-size overrides** — *blocking*: four of this lot's findings need it
   (MR-13-1, MR-30-2, MR-4-6, and the MMC5 NES 2.0 note). Mesen has `GetChrRamSize()`,
   `GetWorkRamSize()`, `GetSaveRamSize()` and `GetSaveRamPageSize()` as virtuals consulted
   before the memories are allocated. We take everything from the header plus one
   special case, `registry::minimum_prg_ram_size`, that exists only for the MMC5.
   A `Mapper`-level override consulted by `CartridgeMemory::new` would serve all four.
3. **Submapper dispatch** — *blocking*: eight findings are submapper handling
   (MR-1-4, MR-4-3, MR-21-3, MR-32-1, MR-16-1, MR-30-3, MR-30-5 and the gating of
   MR-BUS-1). We parse `submapper` in `ines_header.rs:132` and only `Mmc3` ever reads it
   (`mmc3.rs:34`). Mesen threads `_romInfo.SubMapperID` into every board and sometimes into
   `MapperFactory` itself (mapper 4 submapper 3 selects a different class entirely).
4. **A bus-conflict hook** — *blocking* for MR-BUS-1 across five boards. One
   `Mapper::has_bus_conflicts()` plus three lines in `Cartridge::cpu_write`.
5. **An `A12Watcher` with a per-board threshold** — *blocking* for MR-64-3. Mesen's class
   takes the delay as a template parameter because its users disagree (10 dots shared,
   3 CPU cycles in MMC3's own detector, 30 dots in RAMBO-1). Mappers 35, 116, 117, 222 and
   the Waixing FK23C all use it. Note that `MIN_RISE_INTERVAL` feeds
   `Mmc3::ppu_irq_horizon`, so the two must change together.
6. **The shared VRC IRQ** — `Mappers/Konami/VrcIrq.h`, used by VRC2/4, VRC6, VRC7 and by
   mapper 116's VRC2 emulation. Ours is inlined in `Vrc2Vrc4` (`vrc2_vrc4.rs:61-78`,
   `:228-245`) and is otherwise a faithful copy, so lifting it out is mechanical — and
   mapper 116 is the one optional mapper the spec names.
7. **A generic expansion-audio path** — Mesen has `BaseExpansionAudio` (a `ClockAudio`
   virtual) and `NesApu::AddExpansionAudioDelta(channel, delta)`, a per-channel *delta*
   mixer, with five boards built on it (`Mappers/Audio/`: MMC5, VRC6, VRC7, Namco 163,
   Sunsoft 5B). We have a single `Mapper::expansion_audio_output() -> Option<i32>` polled
   each cycle, which REQ-CORE-073's `S = A + M` model wants and which is right for one
   board. REQ-CORE-075 leaves VRC6/VRC7/Namco/Sunsoft/FDS optional, so this is only needed
   if a later lot takes one on; the shape to add is a per-board `M` contribution, not
   Mesen's delta queue, because AMD-054 defines `M` as an absolute level.
8. **Serial EEPROM** — `BaseEeprom24C0X.h`, `Eeprom24C01.h`, `Eeprom24C02.h`, needed by
   mapper 16 submapper 5 (P3 in REQ-CORE-100, so optional) and by mapper 157 (Datach, with
   its barcode reader). Nothing on our side.
9. **Open bus as a distinct answer from a driven zero.** We already have the shape —
   `cpu_peek_override` returning `Some(None)` means "handled, drives nothing" — but several
   boards drive a 0 where the cartridge is absent (MR-5-5, MR-30-2, and the MMC6 window
   below $7000 gets this right already). Worth a pass once per-board RAM sizes exist.
10. **A nametable that can point at CHR-RAM.** `SetPpuMemoryMapping(0x2000, 0x3FFF,
    ChrMemoryType::ChrRam, ...)` is how Mesen does mapper 30's four-screen variation
    (MR-30-6) and TxSROM's (mapper 118) mirroring. Our `nametable_page` returns a console
    VRAM page, `FOLLOW_MIRRORING`, or `SERVED_BY_CARTRIDGE`; the third could carry it, but
    `read_nametable` would need the CHR memory passed in, which it currently is not.
11. **Flash emulation** — `Homebrew/FlashSST39SF040.h`, for MR-30-4 and for the
    self-flashing homebrew boards generally. Needs a ROM-diff save path
    (`SerializeRomDiff`) we have no equivalent of.
12. **A PPU register tap wider than PPUCTRL.** The MMC5 needs to see writes to
    $2000-$2007 (`Mmc5MemoryHandler.h`); we expose only `on_ppu_control` (PPUCTRL). The
    MMC5 vertical split, P2 in REQ-CORE-100, would need $2001/$2005/$2006 as well.
13. **A game database.** `GameDatabase.cpp` carries per-ROM overrides for bus conflicts,
    CHR-RAM sizing, the MMC3 revision (`MMC3A`), the MMC1 revision (`MMC1A`/`MMC1B`) and
    MMC5 RAM sizes. We have none, and that is a defensible choice — but it shapes several
    comparisons in this report, because Mesen falls back to the same behaviour we have
    whenever a ROM is not listed. Any finding that cites a database-gated Mesen path is
    marked **equivalent for unlisted ROMs** above, not a bug.

One piece of our own infrastructure has **no Mesen counterpart** and should be kept in mind
by anyone touching these boards: `Mapper::chr_view` / `chr_peek_view` /
`chr_latch_state` / `advance_chr_latch` exist for REQ-CORE-033 and REQ-CORE-039 — the tile
emitter must read pattern data without disturbing the machine. Mesen has no such
requirement, so when porting a board from Mesen the side-effect split has to be re-derived:
anything Mesen does in `NotifyVramAddressChange` or `MapperReadVram` belongs in our
`ppu_read` / `on_ppu_address_bus`, never in `ppu_peek`.

---

## Summary

27 boards reviewed (32 mapper numbers), plus the shared bank windows, A12 watcher, IRQ
paths, PRG-RAM and battery handling, header and submapper handling, MMC5 audio and mix, and
each board's save state.

**31 differences classified as our bug**, of which **25 carry a failing `#[ignore]`d test**
in `rust/crates/zax-nes/tests/mr1_review_tests.rs`. Every one of those 25 was run and
confirmed to fail for the reason claimed. One additional test is *not* ignored
(`mr_bus_1_uxrom_without_a_submapper_has_no_conflict`): it passes today and pins behaviour a
fix must not break. **No board was changed.**

Also: 7 differences classified **unknown**, 2 where **Mesen looks like the one in error**,
9 recorded as **spec questions** (`Q-110`, `Q-111`), and 7 boards found **equivalent**
outright (0, 66, 87, 156, 225, 228, and MMC2/MMC4 as a pair).

### Findings

| id | board | severity | class | games likely affected | evidence (ours / Mesen) | test | proposed fix |
| --- | --- | --- | --- | --- | --- | --- | --- |
| MR-41-1 | 41 | **crash/hang** | our bug | Caltron 6-in-1, games 5-6 | `multicart.rs:261` / `Caltron41.h:33` | `mr_41_1_caltron_prg_field_is_three_bits` | PRG field is address bits 0-2, not 0-1 |
| MR-4-1 | 4 | **visible** | our bug | StarTropics, Zoda's Revenge (saves) | `mmc3.rs:51-57` / `MMC3.h:142-144` | `mr_4_1_mmc6_a001_read_and_write_bits_are_not_swapped` | swap the read/write bit in each $A001 pair |
| MR-64-1 | 64 | **visible** | our bug | Klax, Skull & Crossbones, Rolling Thunder, Hard Drivin' | `rambo1.rs:72-83` / `Rambo1.h:76-91` | `mr_64_1_rambo1_counter_always_decrements` | always decrement; reload to latch+2 (+1 when latch <= 1) |
| MR-64-2 | 64 | **visible** | our bug | all RAMBO-1 titles using odd 2 KiB banks | `rambo1.rs:63-64` / `Rambo1.h:112` | `mr_64_2_rambo1_2kib_chr_window_keeps_bit_0` | map `reg` and `reg + 1`, do not mask bit 0 |
| MR-64-5 | 64 | **visible** | our bug | Skull & Crossbones | `rambo1.rs:170-174` / `Rambo1.h:137-142` | — | add `force_clock` when leaving cycle mode |
| MR-5-1 | 5 | **visible** | our bug | any 8x8-sprite MMC5 title | `mmc5.rs:163-167` / `MMC5.h:192` | `mr_5_1_mmc5_8x8_sprites_always_use_chr_set_a` | restore the three-clause predicate, gate the last on `in_frame` |
| MR-1-1 | 1 | **visible** | our bug | Shinsenden | `mmc1.rs:146-151` / `MMC1.h:208-213` | `mr_1_1_mmc1_reset_bit_bypasses_the_consecutive_write_filter` | test bit 7 before the two-cycle filter |
| MR-34-1 | 34 | **visible** | our bug | BNROM titles using work RAM near $7FFF | `multicart.rs:80-89` / `BnRom.h` vs `Nina01.h:11` | `mr_34_1_bnrom_has_no_register_at_7ffd` | branch the decode on CHR-ROM presence |
| MR-41-2 | 41 | **visible** | our bug | Caltron 6-in-1 | `multicart.rs:264` / `Caltron41.h:37` | `mr_41_2_caltron_mirroring_comes_from_address_bit_5` | mirroring is address bit 5 |
| MR-41-3 | 41 | **visible** | our bug | Caltron 6-in-1 | `multicart.rs:311-316` / `Caltron41.h:41-45` | `mr_41_3_caltron_inner_chr_is_gated_on_the_prg_bank` | gate the inner CHR write on `prg_bank >= 4` |
| MR-30-1 | 30 | **visible** | our bug | four-screen UNROM 512 homebrew | `ines_header.rs:170` / `UnRom512.h:45-50` | `mr_30_1_unrom512_byte6_09_is_four_screen` | decode byte 6 bits 3+0 together; keep `byte6` on the header |
| MR-30-4 | 30 | **visible** | our bug | self-flashing UNROM 512 homebrew | — / `UnRom512.h:16`, `:107` | — | emulate the SST39SF040 (see infra 11) |
| MR-30-6 | 30 | **visible** | our bug | four-screen UNROM 512 homebrew | `mmc5.rs`-style nametable path absent / `UnRom512.h:53-57` | — | last 8 KiB of CHR-RAM over $2000-$3FFF (see infra 10) |
| MR-21-1 | 22 | **visible** | our bug | VRC2a titles (Twinbee, Gradius) | `vrc2_vrc4.rs:52-59` / `VRC2_4.h:252-256` | `mr_21_1_vrc2a_uses_the_exact_register_wiring` | use the exact wiring when the variant is known |
| MR-21-2 | 22 | **visible** | our bug | VRC2a titles | `vrc2_vrc4.rs:191-200` / `VRC2_4.h:165-166` | `mr_21_2_vrc2a_has_no_prg_swap_mode` | no PRG mode on VRC2; all $9000-$9003 is mirroring |
| MR-32-1 | 32 | **visible** | our bug | Major League | `iremg101.rs` / `IremG101.h:23-27`, `:60-62` | `mr_32_1_irem_g101_submapper_1_is_fixed_one_screen` | submapper 1: one-screen, PRG mode forced 0 |
| MR-BUS-1 | 2,3,7,11,30 | subtle | our bug | Color Dreams titles; submapper-2 dumps | `mod.rs:390-410` / `BaseMapper.cpp:884-896` | `mr_bus_1_color_dreams_write_is_anded_with_the_rom_byte`, `mr_bus_1_uxrom_submapper_2_write_is_anded_with_the_rom_byte` | AND the value with the ROM byte (see infra 4) |
| MR-11-1 | 11 | subtle | our bug | oversized Color Dreams images | `discrete.rs:283` / `ColorDreams.h:22` | `mr_11_1_color_dreams_prg_field_is_four_bits` | mask PRG with `0x0F` |
| MR-1-2 | 1 | subtle | our bug | SUROM/SXROM in 4 KiB CHR mode | `mmc1.rs:37-41`, `:129-131` / `MMC1.h:124`, `:136` | `mr_1_2_mmc1_suprom_outer_bank_follows_the_last_chr_register` | track the last CHR register written |
| MR-1-3 | 1 | subtle | our bug | SOROM battery saves | `mmc1.rs:127-133` / `MMC1.h:134-150` | — (fixture lacks a PRG-RAM-size knob) | 16 KiB uses CHR bit 3; 32 KiB bits 2-3 |
| MR-4-2 | 4 | subtle | our bug | any MMC3 title's first programmed IRQ | `a12_watcher.rs:36-39`, `:45` / `MMC3.h:288` | `mr_4_2_mmc3_a12_needs_a_fall_before_the_first_rise` | require a fall before the first rise counts |
| MR-64-3 | 64 | subtle | our bug | RAMBO-1 titles | `a12_watcher.rs:27` / `Rambo1.h:166` | `mr_64_3_rambo1_a12_filter_is_30_dots` | per-board filter threshold (see infra 5) |
| MR-64-4 | 64 | subtle | our bug | RAMBO-1 titles | `rambo1.rs:81` / `Rambo1.h:13-14`, `:55-60` | — | delay the assertion 2 cycles (A12) / 1 (CPU) |
| MR-5-2 | 5 | subtle | our bug | MMC5 titles using ExRAM as a nametable | `mmc5.rs:412-418` / `MMC5.h:420-426` | `mr_5_2_mmc5_exram_write_outside_rendering_stores_zero` | modes 0/1 store 0 unless `in_frame` |
| MR-16-1 | 16 | subtle | our bug | Famicom Jump II (FCG-1/2 dumps) | `bandai_fcg.rs:114-121` / `BandaiFcg.h` submapper-4 guards | `mr_16_1_bandai_submapper_4_writes_the_counter_directly` | submapper 4: $x00B/$x00C write the counter |
| MR-16-3 | 16 | subtle | our bug | CHR-RAM mapper-16 boards | `bandai_fcg.rs:106-109` / `BandaiFcg.h:174-176` | — | do not bank CHR on a CHR-RAM board |
| MR-18-1 | 18 | subtle | our bug | SS88006 titles (Moero Pro Yakyuu) | `jaleco_ss88006.rs:171-179` / `JalecoSs88006.h:91-100` | `mr_18_1_jaleco_ss88006_irq_fires_one_cycle_earlier` | fire when the decrement yields 0 |
| MR-21-3 | 27 + submappers | subtle | our bug | mapper 27 dumps; submapper-tagged 21/23/25 | `vrc2_vrc4.rs:52-59` / `VRC2_4.h:249-305` | — | a `DetectVariant` equivalent (see infra 3) |
| MR-13-1 | 13 | cosmetic | our bug | Videomation (save-state size only) | `ines_header.rs:154` / `CpRom.h:9` | `mr_13_1_cprom_has_16_kib_of_chr_ram` | CHR-RAM override (see infra 2); REQ-CORE-100 pins 16 KiB |
| MR-1-4 | 1 | cosmetic | our bug | SEROM/SHROM/SH1ROM dumps | `mmc1.rs` / `MMC1.h:152-156` | `mr_1_4_mmc1_submapper_5_has_fixed_prg` | submapper 5: fixed 32 KiB |
| MR-30-2 | 30 | cosmetic | our bug | UNROM 512 homebrew | `mod.rs:52-57`, `:378-387` / `UnRom512.h:21-22` | `mr_30_2_unrom512_does_not_drive_6000_7fff` | no work RAM on this board (see infra 2) |
| MR-30-3 | 30 | cosmetic | our bug | submapper-3 UNROM 512 | `discrete.rs:416-422` / `UnRom512.h:62-66` | — | submapper 3 toggles vertical/horizontal |
| MR-30-5 | 30 | cosmetic | our bug | submapper-4 UNROM 512 | `discrete.rs:447-452` / `UnRom512.h:76-79` | — | submapper 4: no banking in $8000-$BFFF |
| MR-4-3 | 4 | cosmetic | our bug | Acclaim MC-ACC dumps | `registry.rs:48` / `MapperFactory.cpp:285-290` | — | submapper 3 is a different board |
| MR-4-6 | 4 | cosmetic | our bug | MMC6 (exposed RAM size only) | `mmc3.rs:45-47` / `MMC3.h:184-185` | — | 1 KiB save RAM (see infra 2) |
| MR-16-2 | 16 | cosmetic | our bug | none commercial (mapper 153 territory) | `bandai_fcg.rs` / `BandaiFcg.h:165-173` | — | PRG outer bank from the CHR registers' bit 0 |
| MR-5-5 | 5 | cosmetic | our bug | none known | `mmc5.rs:395-399` / `MMC5.h` mapping | — | ExRAM modes 0/1 read as open bus |
| MR-13-2 | 13 | visible | unknown | Videomation | `discrete.rs:254-257` / `CpRom.h:15` | — | decide whether CPROM hardwires vertical |
| MR-71-1 | 71 | visible | unknown | Fire Hawk | `discrete.rs:381` / `BF909x.h:37` | — | **polarity is inverted between us and Mesen; one side is wrong** |
| MR-5-3 | 5 | subtle | unknown | Castlevania III, Just Breed | `mmc5.rs:487` / `MMC5.h:430-445` | — | settle the scanline counter's off-by-one |
| MR-5-4 | 5 | subtle | unknown | MMC5 titles toggling rendering mid-frame | `mmc5.rs:480-494` / `MMC5.h:429-457` | — | in-frame from the PPU bus vs our PPU signal |
| MR-A12-1 | shared | subtle | unknown | MMC3 titles | `a12_watcher.rs:27` / three Mesen thresholds | — | the 10-dot constant is an approximation; `mmc3_test_2` 1-5 pass |
| MR-4-4 | 4 | cosmetic | unknown | none (pre-init frames) | `mmc3.rs:180-181` / `MMC3.h:63-70` | — | power-on bank registers |
| MR-4-5 | 4 | cosmetic | unknown | blargg MMC3 ROMs | `mmc3.rs:176-178` / `MMC3.h:57` | — | power-on $A001; our rationale is in the code |
| MR-PWR-1 | 1,18,22,32,64 | cosmetic | unknown | none | see the power-on section | — | linear vs all-zero power-on bank layout |
| MR-64-6 | 64 | subtle | **Mesen in error** | — | `rambo1.rs:156-159` / `Rambo1.h:122-124` | — | Mesen's $8000 does not re-map; banking is combinational. No change |
| MR-28-2 | 28 | cosmetic | **Mesen in error** | — | `multicart.rs:484-490` / `Action53.h:18-21` | — | AMD-021 R2-001 fixes outer=63/inner=15; Mesen zeroes. No change |
| MR-71-2 | 71 | cosmetic | equivalent | — | `discrete.rs:379-386` / `BF909x.h:34-38` | — | decode range differs, unreachable in practice |
| Q-110 | 15, 28, 101 | subtle | spec question | none known | `spec/changes/Q-110.md` | — | AMD-021 R2-001/002/003 contradict Mesen on 7 points |
| Q-111 | 5 (audio) | visible | spec question | none (no MMC5-audio profile) | `spec/changes/Q-111.md` | — | AMD-054 contradicts nesdev on the period-8 cutoff and the polarity, unmarked |

### The three to fix first

If a later lot takes only part of this:

1. **MR-41-1/2/3** — the Caltron 6-in-1 cannot run half its cartridge. Smallest fix, worst
   symptom.
2. **MR-4-1** — the MMC6 writes saves through a bit that should only enable reads, so
   *StarTropics* corrupts its own save file.
3. **MR-64-1** — every RAMBO-1 IRQ is a scanline early, visible in four Tengen titles, and
   the fix is eight lines.

**MR-71-1 is the one open question worth closing cheaply**: our mirroring polarity and
Mesen's are exact opposites, so one of us is certainly wrong and *Fire Hawk* shows it. One
look at the nesdev register table settles it.

---

# MR-1b — sourced re-check

Role R3, clean side. Lot MR-1b. **This lot changes no emulation behaviour either**: the only
code it touches is the `#[ignore]`d tests of MR-1 (some withdrawn, some re-aimed, some added).

Lot MR-1 compared our boards with Mesen 2 *without web access*. It could therefore record a
contradiction but not settle it, and in several places it took Mesen for the hardware. This
lot re-examines all 50 of its findings against the public sources — the NESdev wiki page of
the board or chip with its register table, the forum threads that carry the hardware tests,
the test-ROM documentation, and NesCartDB for which cartridges use a board — and says for
each which side the evidence supports.

Rules applied, in this order:

1. **The spec still prevails.** Where a source contradicts a normative REQ or amendment, the
   entry is a spec question, not a bug.
2. A finding becomes `our bug` **only** with a cited source or a test-ROM fact, and leaves
   `our bug` the same way.
3. `unknown` is kept only where the public sources genuinely do not settle the point, and the
   entry then says what would — a specific hardware test or cartridge.
4. Every `our bug` carries a **performance cost class**, derived by reading the code path:
   **none** (load or rare write only), **negligible** (a predictable branch on a rare path),
   **measurable** (a per-access or per-cycle path, for affected games only), **risk** (a path
   every game runs). For `measurable` and `risk`, the cheapest confinement is given.

The summary table at the end of this section **replaces** MR-1's, which is marked superseded.

## Severity and cost vocabulary

Severity is unchanged from MR-1 (`crash/hang` > `visible` > `subtle` > `cosmetic`). The cost
classes are judged against `PERFORMANCE-HISTORY.md` section 5.1 and section 10, whose
relevant facts are:

- **a cartridge write is already a PPU sync point** (5.1, "PPU caught up in batches"), and
  `Cartridge::cpu_write` (`mod.rs:390-410`) already calls `refresh_rom_windows()` after every
  one of them. Work added on a register write is therefore cheap by construction;
- `Cartridge::read_rom_window` (`mod.rs:343-353`) is `#[inline]` and indexes a cached
  `rom_windows[address >> 13]`, so the ROM byte a bus conflict needs is one load away;
- `Mapper::chr_view` returning `Some` is what buys a board the fast span
  (`run_visible_span`); a board that gains a side effect on a CHR fetch drops to
  `run_visible_span_through_bus`. MMC2/MMC4, MMC3 and compatibles, RAMBO-1 and MMC5 are
  **already** on the through-bus path, so changes inside their `ppu_read` cost nothing new;
- `A12Watcher::MIN_RISE_INTERVAL` feeds `Mmc3::ppu_irq_horizon` (`mmc3.rs:167`): a per-board
  threshold has to move both together or the PPU is observed late;
- expansion audio forces `apu_quiet = 0`, and only the MMC5 has any, so MMC5 audio already
  runs per CPU cycle;
- `NesBus::tick`'s prologue is pinned in the post-LTO assembly (section 10): nothing
  board-specific may be inlined into it.

---

## Shared: bus conflicts (MR-BUS-1)

**MR-1 verdict: our bug. New verdict: our bug — confirmed, and the AND is normative rather
than "undefined behaviour".**

Sources:

- *NES 2.0 submappers*, mappers 2/3/7 submapper 2 — "Bus conflicts occur, producing the
  bitwise AND of the written value and the value in ROM"; submapper 1 — "Bus conflicts do not
  occur". <https://www.nesdev.org/wiki/NES_2.0_submappers>
- *Color Dreams* (board page) — commercial releases take the AND of the written value and the
  ROM content at that address; the *Free Fall* prototype has conflict-prevention logic.
  <https://www.nesdev.org/wiki/Color_Dreams>
- *INES Mapper 011* — "All commercially-released games are subject to bus conflicts
  (submapper 2)". <https://www.nesdev.org/wiki/INES_Mapper_011>
- *UNROM 512*, submapper list — submapper 0: "The Battery bit must be specified if the game
  relies on bus conflicts not being present"; submapper 2: "Bus conflicts present".
  <https://www.nesdev.org/wiki/UNROM_512>
- *Bus conflict* — "Most ASIC based mappers include logic to disable the ROM's output enable
  during writes". <https://www.nesdev.org/wiki/Bus_conflict>

The submapper definitions settle the two things MR-1 could only infer from Mesen: that the
result is the bitwise AND, and that the gate is the submapper. They also confirm Mesen's
per-board conditions, including the unusual mapper-30 one — submapper 0 *without* battery,
because on that board the battery bit is what a submapper-0 dump uses to say "no conflicts".
The *Bus conflict* page's "Programmers must not rely on this undefined behavior" is advice to
ROM authors about relying on a clean AND, not licence for an emulator to skip it.

**Test change:** none. `mr_bus_1_color_dreams_write_is_anded_with_the_rom_byte` and
`mr_bus_1_uxrom_submapper_2_write_is_anded_with_the_rom_byte` stay exactly as MR-1 wrote
them: a confirmed finding keeps the test and the `MR-1` label of the lot that wrote it, and
only the tests this lot withdraws, re-aims or adds are touched. The non-ignored guard
`mr_bus_1_uxrom_without_a_submapper_has_no_conflict` keeps pinning plain mapper 2.

**Cost: negligible.** The AND runs on a CPU write to $8000-$FFFF on a board that declares
conflicts — the rarest write path there is, and already a PPU sync point. The byte comes from
`read_rom_window`, one load from a cached window base. Confinement: a `bus_conflicts: bool`
resolved **at load** into `Cartridge` (three fields today), so `cpu_write` gains one test of a
hot-in-cache bool that is `false` for 27 of our 32 mapper numbers and for every submapper-0
dump of the other five. No `Mapper` method is consulted per write and the `Board` enum
dispatch is untouched; nothing reaches the PPU or CPU stepping.

## Shared: bank-window wrapping

**MR-1 verdict: equivalent. New verdict: equivalent — unchanged.**

No public source specifies what a board does with a PRG size that is not a multiple of its
page size, because no such dump exists: the NESdev *iNES* page expresses PRG size in 16 KiB
units and *NES 2.0* as a size exponent with a multiplier, and every commercial dump is a power
of two. MR-1's two edge cases are therefore unreachable rather than undecided, and its
algebraic argument stands on its own. No test.

## Shared: the A12 filter (MR-A12-1)

**MR-1 verdict: unknown. New verdict: equivalent in practice for the MMC3 — and the sources
settle the *unit*: the filter counts M2 cycles, not PPU dots.**

Sources:

- *MMC3*, IRQ section — the counter is clocked by "PPU A12, triggered on a rising edge after
  the line has remained low for three falling edges of M2".
  <https://www.nesdev.org/wiki/MMC3>
- *RAMBO-1*, scanline-mode pseudo-code, on each M2 falling edge — while PA12 = 0, "If 4-bit
  counter < 16: 4-bit counter++"; on PA12 = 1, "If 4-bit counter = 16: Clock the 8-bit
  counter". <https://www.nesdev.org/wiki/RAMBO-1>

The real filter on both boards counts **M2 falling edges while A12 is low** — three on the
MMC3, sixteen on the RAMBO-1 — which is why Mesen's own MMC3 detector works in CPU cycles
while its shared `A12Watcher` works in dots: the dot figure is a conversion, not the
specification. Our `FILTER_DOTS = 10` is 3.33 CPU cycles, so for the MMC3 it brackets the
documented three falls: it rejects every dip shorter than three falls and accepts every low
longer than four, and the only lows it could misjudge are 7 to 9 dots, a length no rendering
pattern produces (the dips of the fetch pipeline are 4 dots; the next-shortest low is the
20-dot-plus prefetch gap). `mmc3_test_2` tests 1-5 pass, as REQ-CORE-114 requires.

What would settle the residue: a ROM that walks $2006 to make an A12 low of exactly 7, 8 and
9 dots and reports whether the counter clocked. No public test ROM does that today.

**Test change:** none (MR-1 wrote none).

**Cost of the only change worth making** — passing the threshold in at construction, which
MR-64-3 needs anyway: **negligible**. `A12Watcher` gains one `i64` field in place of an
associated constant, read on the same cache line as `low_since` inside `rose()`, which is
called only from boards already off the fast span. The catch is structural, not arithmetic:
`MIN_RISE_INTERVAL` is an associated *constant* feeding `Mmc3::ppu_irq_horizon`, so it has to
become a method and the horizon has to be recomputed from the instance.

## Shared: the power-on bank layout (MR-PWR-1), and MR-4-4

**MR-1 verdict: unknown (both). New verdict: neither side is wrong — the sources say the
power-on values are unspecified. Both become `equivalent`.**

Source: *MMC3* — "Because the values in R6, R7, and $8000 are unspecified at power on, the
reset vector must point into $E000-$FFFF, and code must initialize these before jumping out
of $E000-$FFFF". <https://www.nesdev.org/wiki/MMC3>

That sentence is the authority for MR-1's whole family of power-on differences, not just the
MMC3's: the hardware does not define the values, the documentation says so, and it tells the
ROM author to program them. Our linear layout, Mesen's zeros and Mesen's randomisable
`GetPowerOnByte()` are all conforming, and the "last bank fixed at $E000" argument MR-1 made
is exactly the invariant the wiki leans on. There is nothing left to discover, so these stop
being `unknown`.

**Test change:** none. No fix, so no cost.

## Mapper 1 — MMC1 (SxROM)

The register layouts quoted below come from *MMC1* (<https://www.nesdev.org/wiki/MMC1>) and
the board table from *SxROM* (<https://www.nesdev.org/wiki/SxROM>). The CHR bank 0 register of
a SOROM/SUROM/SXROM board is printed there as

```
4bit0
-----
PSSxC
||| |
||| +- Select 4 KiB CHR-RAM bank at PPU $0000 (ignored in 8 KiB mode)
|++--- Select 8 KiB PRG-RAM bank
+----- Select 256 KiB PRG-ROM bank
```

which settles three of MR-1's four findings at once, in two different directions.

### MR-1-1 — MMC1 reset bit and the consecutive-write filter

**MR-1 verdict: our bug (visible). New verdict: our bug — confirmed by name.**

Source: *MMC1* — the consecutive-write restriction "only applies to the data being written on
bit 0; the bit 7 reset is **never ignored**", and the page's own discussion of the restriction
is the one that names *Shinsenden*. The reset write itself is "A write with bit set will reset
shift register and write Control with (Control OR $0C)".

The wiki and Mesen agree, and the wiki is the normative source under REQ-CORE-100. Our
`write_register` (`mmc1.rs:142-146`) returns before it ever looks at the value, so a reset
written as the second half of a read-modify-write is lost and the board keeps a PRG mode the
game believes it has cleared.

**Test:** unchanged, MR-1's test stands —
`mr_1_1_mmc1_reset_bit_bypasses_the_consecutive_write_filter`.

**Cost: none.** The fix moves the `value & 0x80` test above the two-cycle filter inside
`write_register`, which already runs on exactly these writes; no field is added, no path is
widened, and the serial register is not read anywhere else.

### MR-1-2 — the 512 KiB outer bank and the "last written" CHR register

**MR-1 verdict: our bug (subtle). New verdict: neither side is right — and both are
equivalent on every commercial dump. Leaves `our bug`.**

Source: *MMC1* — "In 4 KiB CHR bank mode, SNROM's `E` bit and SO/U/XROM's `P` and `S` bits in
both CHR bank registers must be set to the same values, or the PRG-ROM and/or RAM will be
bankswitched/enabled **as the PPU renders**", followed by the advice that a program should
"just set 8 KiB bankswitching mode".

That is the hardware answer, and it is neither ours nor Mesen's: in 4 KiB CHR mode the outer
PRG bank and the PRG-RAM bank follow *whichever CHR register PPU A12 is selecting at that
instant*, so they move during rendering. Mesen's "the register written last" and our "always
`chr0`" are both conventions for a case the documentation tells programs not to create. It is
also unreachable in practice: *SxROM* gives SUROM and SXROM 8 KiB of CHR **RAM**, and the
titles concerned (*Dragon Warrior III* and *IV* on SUROM, *Final Fantasy I·II* and *Best Play
Pro Yakyuu Special* on SXROM) run in 8 KiB CHR mode, where Mesen's expression reduces to
`chr0` and the two sides coincide.

**Test change: withdrawn.** `mr_1_2_mmc1_suprom_outer_bank_follows_the_last_chr_register`
asserted Mesen's convention as if it were the hardware's, which the source does not support.
Removed rather than re-labelled: there is no behaviour here for a test to pin short of
emulating the A12-driven switching, which no game needs.

**Cost if someone implements the real thing anyway: risk, and not worth paying.** The PRG
window and the PRG-RAM offset would have to be recomputed on every PPU A12 transition — a
value consulted by `read_rom_window` on every CPU fetch, driven from a PPU-bus hook. That
would also cost the MMC1 its `chr_view`, moving every MMC1 game (the largest board family in
the library) from `run_visible_span` to `run_visible_span_through_bus`. No game needs it.

### MR-1-3 — the SOROM PRG-RAM selector

**MR-1 verdict: our bug (subtle, no test). New verdict: our bug — confirmed, and the bit is
the one Mesen uses.**

Sources: *MMC1* — "For SXROM, the upper `S` (bit 3) selects the SRAM's A14, and the lower `S`
(bit 2) selects A13", and "The SOROM board only implements the upper `S` bit". *SxROM* gives
SOROM 128/256 KiB PRG with 16 KiB PRG-RAM, SXROM 32 KiB.

So a 32 KiB board uses bits 3-2, which is exactly our `(chr0 >> 2) & 0x03`
(`mmc1.rs:129-133`) — our SXROM path is right — while a 16 KiB SOROM board uses **bit 3
alone**. Our modulo over a 16 KiB RAM turns `(chr0 >> 2) & 0x03` into bit 2, so the selector
is one bit off and *Genghis Khan* and *Nobunaga's Ambition* (the SOROM titles) put their saves
in the half that is not battery-backed and read them from the half that is.

**Test change: still none, and now for a recorded reason.** `MapperRom` has no PRG-RAM-size
knob and a 16 KiB declaration needs NES 2.0 byte 10; adding that knob is the first step of any
fix. Noted in the progress file so the fixing lot does not re-discover it.

**Cost: none, if the selector is precomputed.** `prg_ram_offset` runs on every CPU read and
write in $6000-$7FFF, so a second runtime branch there would be `measurable` on
PRG-RAM-heavy games. It is avoidable: the shift and mask depend only on
`memory.prg_ram().len()`, which is fixed at load, so `Mmc1::new` can store a
`ram_bank_shift: u32` and `ram_bank_mask: u8` and the hot path keeps exactly the one shift and
one mask it has today.

### MR-1-4 — submapper 5, fixed PRG

**MR-1 verdict: our bug (cosmetic). New verdict: our bug — confirmed, with a named game.**

Sources: *NES 2.0 submappers*, mapper 1 submapper 5 — "use a fixed 32k PRG ROM with no
banking support"; *MMC1* describes submapper 5 as the SEROM/SHROM/SH1ROM boards with 32 KiB of
unbanked PRG-ROM; *SxROM* gives SEROM 32 KiB PRG and names **Dr. Mario**.

Both sources agree with Mesen, and the board is a real one with a well-known title, so this is
slightly less cosmetic than MR-1 made it: a correctly headered *Dr. Mario* dump that writes
the PRG register at all would bank a 32 KiB image into halves on our side.

**Test:** unchanged, MR-1's test stands — `mr_1_4_mmc1_submapper_5_has_fixed_prg`.

**Cost: none.** The submapper is read at construction into a `fixed_prg: bool`; `update_banks`
gains one branch, and `update_banks` runs only on a committed serial write.

### MR-1-5 — only one of SOROM's two RAM chips is battery-backed (new)

**New finding. Verdict: our bug, severity cosmetic.**

Source: *MMC1* — "Even if the SOROM and SZROM boards utilizes a battery, it is connected to
only one PRG-RAM chip. The first RAM chip will not retain its data, but the second one will."

`Cartridge::battery_ram` (`mod.rs:330-337`) returns the *whole* `prg_ram` whenever the header
declares a battery, so on a SOROM or SXROM board we persist 16 or 32 KiB where the hardware
persists 8. The save file is larger than the cartridge's, and a cold boot shows retained data
in a half that should come up uninitialised — which is exactly the half a game may use as
scratch, so a game could in principle read stale data it expects to be garbage.

**Test:** none; it needs the same PRG-RAM-size knob as MR-1-3.

**Cost: none.** The battery half is a sub-slice chosen at load — Mesen's `GetSaveRamSize()` /
`GetWorkRamSize()` split, which the shared-infrastructure item 2 of MR-1 already calls for.

### The three MMC1 notes MR-1 left open

- **the 512 KiB test** (`> 256 KiB` vs Mesen's `== 512 KiB`). *MMC1* prints the CHR register
  bit as "PRG-ROM A18 if PRG-ROM size is 512 KiB", and *SxROM* lists 512 KiB as the only
  size above 256 for SUROM/SXROM. **Equivalent** — no 384 KiB MMC1 dump exists.
- **the MMC1A / MMC1B revision**. The source makes the difference bigger than MR-1 implied:
  on an MMC1A, bit 4 of the PRG register is not a PRG-RAM disable at all but a PRG-ROM A17
  control ("bit 3 directly controls A17 across the entire $8000-$FFFF address range"). We
  implement MMC1B (`mmc1.rs:107-110`), which is both the common revision and what Mesen does
  for any ROM its database does not list. **Equivalent for unlisted ROMs**; distinguishing
  the two needs a game database, which REQ-CORE-100 does not ask for.
- **the soft reset**. Still **unknown**: no public source describes what the MMC1 does on
  the CPU reset line, and REQ-CORE-004 asks the cartridge to power on cleanly, which is what
  `Mmc1::reset` does. What would settle it: a hardware test holding the reset line and
  reading back the PRG window.

## Mapper 4 — MMC3 and MMC6

Sources for this board: *MMC3* (<https://www.nesdev.org/wiki/MMC3>), *MMC6*
(<https://www.nesdev.org/wiki/MMC6>) and *NES 2.0 submappers*
(<https://www.nesdev.org/wiki/NES_2.0_submappers>).

### MR-4-1 — the MMC6 $A001 read and write bits

**MR-1 verdict: our bug (visible). New verdict: our bug — confirmed, bit for bit.**

Source: *MMC6* prints $A001 as

```
7  bit  0
---- ----
HhLl xxxx
||||
|||+------ Enable writing to RAM at $7000-$71FF
||+------- Enable reading RAM at $7000-$71FF
|+-------- Enable writing to RAM at $7200-$73FF
+--------- Enable reading RAM at $7200-$73FF
```

so bit 4 writes and bit 5 reads the low half, bit 6 writes and bit 7 reads the high half —
Mesen's assignment exactly, and the opposite of ours (`mmc3.rs:50-56`, where
`mmc6_half_readable` takes bits 4 and 6 and `mmc6_half_writable` bits 5 and 7). The same page
names the two titles, *StarTropics* and *Zoda's Revenge*, and both save through this RAM, so
MR-1's "save corruption rather than a glitch" reading stands.

**Test:** unchanged — `mr_4_1_mmc6_a001_read_and_write_bits_are_not_swapped`.

**Cost: none.** Two constants swap inside `mmc6_half_readable` / `mmc6_half_writable`. Those
run on CPU accesses to $6000-$7FFF of an MMC6 — two cartridges in the library — and the
instruction count does not change at all.

### MR-4-2 — "A12 must fall before the first rise counts"

**MR-1 verdict: our bug (subtle). New verdict: leaves `our bug` — ours matches the hardware
filter and Mesen's extra guard does not.**

Source: *MMC3* — the counter is clocked by "PPU A12, triggered on a rising edge after the line
has remained low for three falling edges of M2".

The condition is on how long A12 has *been* low, not on having observed it go low. At
power-on the PPU address bus holds `v = 0`, so A12 has been low since the machine started and
the filter is satisfied: the first rise the MMC3 sees does clock the counter. That is what our
`A12Watcher` does with `low_since = 0` (`a12_watcher.rs:36-39`, `:45`), and Mesen's
`_a12LowClock > 0` guard (`MMC3.h:288`) makes it *miss* that first clock. MR-1 read Mesen's
bookkeeping as the specification and called our behaviour a spurious clock; the source says it
is the hardware's.

One residue in our favour and against us, both unobservable: we reject a rise occurring in the
first ten dots after power-on or after `reset()`, where the hardware would accept it. IRQs are
disabled at both moments, so nothing can see it.

**Test change: withdrawn.** `mr_4_2_mmc3_a12_needs_a_fall_before_the_first_rise` asserted
Mesen's guard. Removed, with the reason recorded in the file.

**Cost: none** — no fix.

### MR-4-3 — mapper 4 submapper 3 is the Acclaim MC-ACC

**MR-1 verdict: our bug (cosmetic). New verdict: our bug — confirmed, and the difference is
bigger than "a different A12 filter position".**

Sources: *NES 2.0 submappers*, mapper 4 — "Submapper 3: MC-ACC chip variant"; *MMC3* —
"Acclaim's MC-ACC chip is their own variant of the MMC3 ... The only known difference is that
the scanline counter uses a ÷8 prescaler instead of filtering A12, clocked by falling edges."

So the MC-ACC does not filter A12 at all: it divides the A12 **falling** edges by eight. Over
a normal rendered line that still gives one clock per scanline, which is why these games
mostly work on an MMC3 emulation, but the phase differs (the clock lands on a fall, a few
pixels from where the MMC3's rise lands) and any game or test that drives A12 by hand diverges
outright. The second-source boards are Acclaim's; *The Incredible Crash Dummies* and *Mickey's
Safari in Letterland* are the titles named in public discussion of the chip, and a dump can
only be recognised by its NES 2.0 submapper or its hash.

**Test:** none added. Reproducing the ÷8 prescaler needs a second counter in the fixture's
reach but a *board* we do not have; the fix is a new board, not a branch, so the test belongs
to the lot that adds it.

**Cost: none for every other game, measurable only for MC-ACC dumps.** The right shape is a
separate board (Mesen does the same: `MapperFactory.cpp:287` returns `McAcc`) chosen at load
from the submapper, so no MMC3 game pays a branch. Inside that board the prescaler replaces
the `A12Watcher` call in `on_ppu_address_bus` one for one — a counter increment instead of a
clock comparison, on a path these cartridges already take through the bus. Note that the new
board needs its own `ppu_irq_horizon`: eight falls per clock is not
`A12Watcher::MIN_RISE_INTERVAL`, and a horizon copied from `Mmc3` would make the PPU late.

### MR-4-4 — the power-on bank registers

**MR-1 verdict: unknown. New verdict: equivalent** — see the shared power-on entry above.
*MMC3* says the values "are unspecified at power on"; both our seeding and Mesen's conform.
No test, no fix, no cost.

### MR-4-5 — the power-on $A001

**MR-1 verdict: unknown. New verdict: equivalent for the MMC3; for the MMC6 the question
dissolves.**

Source: *MMC6* — "When PRG RAM is disabled via $8000, the mapper continuously sets $A001 to
$00, and so all writes to $A001 are ignored."

For the MMC3 proper nothing in the sources gives a power-on value for $A001, so our 0x80 and
Mesen's 0 are both conforming and the rationale already in our code (`mmc3.rs:189-192`) — it
is the only value that lets blargg's MMC3 ROMs, which never write $A001, use the RAM from a
cold start — is a good reason to keep it. For the MMC6 the value is documented: the chip
powers on with $8000 = 0, RAM therefore disabled, and $A001 therefore held at 0. Our
`protect = 0x80` is dormant there because `prg_ram_enabled` consults $8000 bit 5 on an MMC6
(`mmc3.rs:173-179`) — but see MR-4-7, which is where it becomes visible.

### MR-4-6 — the MMC6 RAM is 1 KiB

**MR-1 verdict: our bug (cosmetic). New verdict: our bug — confirmed.**

Source: *MMC6* — the chip "contains 1 KB of internal PRG RAM", at "CPU $7000-$7FFF: 1 KB PRG
RAM, mirrored".

Mesen's `GetSaveRamSize() = 0x400` is the same number. Our 8 KiB default changes no byte a
game can read, because `mmc6_offset` masks with `0x03FF` (`mmc3.rs:46-48`), but the exposed
and saved size is eight times the cartridge's.

**Test:** none; it needs the same `chr_ram_size_override`-style hook as MR-13-1 (MR-1's
shared-infrastructure item 2).

**Cost: none.** A per-board save-RAM size is resolved in `CartridgeMemory::new` at load. It
also *removes* work from the hot path: `mmc6_offset`'s `% memory.prg_ram().len()` becomes the
identity on a 1 KiB allocation and can go away.

### MR-4-7 — the MMC6 holds $A001 at zero while RAM is disabled (new)

**New finding. Verdict: our bug, severity subtle.**

Source: *MMC6* — "When PRG RAM is disabled via $8000, the mapper continuously sets $A001 to
$00, and so all writes to $A001 are ignored."

We store $A001 unconditionally (`mmc3.rs:259`), so on an MMC6 a value written while $8000
bit 5 is clear survives and takes effect the moment the game sets bit 5. On the hardware that
write is lost and the four enable bits are 0 until the game writes $A001 *after* enabling the
RAM. *StarTropics* and *Zoda's Revenge* are the cartridges that could see it, and the
difference is in which 512-byte half answers a read immediately after the RAM is switched on.

**Test:** added — `mr_4_7_mmc6_a001_is_held_at_zero_while_ram_is_disabled`.

**Cost: none.** One condition on the $A001 arm of `write_register`, which runs only on a write
to $A001, and only the MMC6 branch of it.

### MR-4-8 — the MMC6 window is open bus when no half is readable (new)

**New finding. Verdict: our bug, severity cosmetic.**

Source: *MMC6* — "If neither bank is enabled for reading, the $7000-$7FFF area is open bus."

Our `cpu_peek_override` answers a driven `0` in exactly that case (`mmc3.rs:208`, `:211-215`,
`Some(Some(0))`), where `Some(None)` is our open-bus answer. The $6000-$6FFF half of the
window already returns open bus correctly, which shows the shape is there and only these two
sites are wrong. Same family as MR-5-5 and MR-30-2, and no known game reads a disabled MMC6
window.

**Test:** none — observing open bus from the fixture needs the bus's last-value behaviour,
which `MapperMachine::prg_windows` does not expose; the three open-bus findings are better
tested together by the lot that fixes them.

**Cost: none.** Two returns change value in a function that only MMC6 dumps enter.

### MR-4-9 — submappers 2 and 4 of mapper 4 are not handled (new)

**New finding. Verdict: our bug, severity cosmetic.**

Source: *NES 2.0 submappers*, mapper 4 — "Submapper 2: MMC3C with hard-wired mirroring";
"Submapper 4: NEC MMC3 (Loading latch with 0 disables IRQ)"; "Submapper 5: Indicates the
presence of a T9552 scrambling chip".

We read only submapper 1 (`mmc3.rs:34`, the MMC6) and Mesen only submapper 3, so neither of us
handles 2 or 4 — this is a gap the Mesen comparison could not reveal. Submapper 2 wants the
$A000 mirroring writes ignored, which is the same code path as our existing `four_screen`
guard (`mmc3.rs:250-258`). Submapper 4 asks for the rev A IRQ, which **REQ-CORE-100 pins to
rev B** for this board and REQ-CORE-114 then lets `mmc3_test_2` test 6 fail; that is a
spec-versus-submapper question rather than a plain bug, and it is small enough that it belongs
in the same ticket as any other rev A decision rather than in a new one. Submapper 5 is a
scrambling chip and out of REQ-CORE-100's scope.

**Test:** none. Submapper 2's fix is one extra condition on a flag we already have; it is not
worth a test ahead of the submapper dispatch MR-1's infrastructure item 3 calls for.

**Cost: none.** Both are load-time flags; submapper 2 widens an existing branch, submapper 4
selects which of two comparisons `clock_irq_counter` makes.

### The two MMC3 non-findings, re-checked

- **the IRQ revision.** *MMC3* — "Sharp MMC3B and MMC3C generate an IRQ on each scanline
  while $C000 is $00", while "Sharp MMC3A and all NEC MMC3 variants generate only a single
  IRQ when $C000 is $00". Our rev B (`mmc3.rs:86-90`) is both what REQ-CORE-100 requires and
  the majority chip. **Still not a finding.**
- **register 0/1 bit 0.** Nothing in the sources observes the stored register byte, only the
  resulting 2 KiB bank, which our `map_chr_2k(slot, b >> 1)` gets right. **Still
  equivalent.** The RAMBO-1 entry below shows why this detail is worth stating twice: the
  same masking question has the *opposite* answer on that board, and nesdev says so for both.
