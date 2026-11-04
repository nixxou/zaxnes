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

<!-- sections for steps 6..8 are appended as the review progresses -->
