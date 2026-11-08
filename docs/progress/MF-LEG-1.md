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
| 8 | **step 1 behind a feature, off by default**: the hardware test ROM contradicts the rule | MMC1-1 |
| 9 | the callgrind campaign and the feature decision per board | — |
| 10 | the final check, and the report annotations | — |

**The plan grew one step at step 8.** Running the ROM-gated suite with both roots set — which the
final check asks for and which nothing before step 8 had done — showed that step 1's narrowing
makes two `holy-mapperel` rows fail that lot MF-3 step 1 exists to have fixed. That is a decision
for R0, so the narrowing is now behind a feature off by default, and the step that does it is its
own. Steps 2 to 6 are untouched.

## Steps

- [x] 1/10 MMC1 SNROM (MMC1-1) — the `E` bit only on NES 2.0 evidence, and the extra check

### 1/10 — MMC1 SNROM: the `E` bit needs the header's word (entry MMC1-1)

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
any of the five and the condition of the lot instruction is met. (It was therefore unconditional as
step 1 shipped. **Step 8 put it behind `mmc1-snrom-header`, off in `default`**, for a reason this
step had not looked for: two `holy-mapperel` rows. See step 8 and Q-176.)

**Tests.** The two existing SNROM tests keep their behaviour; the positive one now loads an NES 2.0
image, which needed a `chr_ram(bytes)` builder on `MapperRom` (byte 11). Two new ones:
`mf_leg_1_mmc1_an_ines_1_0_dump_with_chr_ram_does_not_get_the_snrom_e_bit` — the Zelda II shape,
with the `$E000` half still disabling as the regression guard — and
`mf_leg_1_mmc1_the_snrom_e_bit_needs_all_three_nes_20_declarations`, which walks the three ways a
dump can fall short (byte 10 zero, byte 11 absent, 16 KiB of CHR-RAM) and then the shape that
passes, both behind `mmc1-snrom-header` since step 8. `cargo test --release -p zax-nes -p
zax-harness`: **1157 passed, 0 failed, 10 ignored**.

Two comments that read `Mmc1::is_snrom`'s old signature — the NES-EVENT's `prg_ram_enabled`
override and its test — were corrected; the override itself is unchanged and still necessary.
- [x] 2/10 VRC2 (VRC2-1) — the one-bit latch at `$6000`-`$6FFF`

### 2/10 — the VRC2's Microwire latch (entry VRC2-1)

**The page.** nesdev ("VRC2 and VRC4"): the chip "was supposed to have shipped with a Microwire
interface for save games. However, Konami never used it", and the games "rely on the two Data pins
being connected to each other, and so expect to be able to read the written value back" —
`.... ...L`, "1-bit latch value (r/w)", with "open bus for the top 7 bits" and "$7000-$7FFF only
ever return open bus". *Contra (J)* and *Ganbare Goemon 2 (J)* are the titles it names.

**The gate, which is the whole safety of the change.** `Vrc2Vrc4::has_microwire_latch`: an
**NES 2.0** header that declares no PRG-RAM (`byte10 == 0` — tested on the byte, because
`nes20_prg_ram_size` floors at 8 KiB so a declared zero reads back as the default), on one of the
three numbers whose VRC2 variant that header names — mapper 22, always a VRC2a, or 23/25 with
submapper 3. That is Nestopia's gate (`if (!board.GetWram())`) and puNES's (NES 2.0 and no WRAM).
Submapper 0 of 23 and 25 is the OR'd heuristic and keeps the VRC4's features, so it is not a VRC2
here; mapper 27 is a VRC4 clone; mappers 14 and 116 come through `new_vrc2b`, which clears the
field because `new` is asked there for mapper 23.

**So no dump in circulation moves.** Every common dump of the two named games is iNES 1.0, keeps
its REQ-CORE-012 work RAM and passes its check through the RAM exactly as before — which is the
accident the entry describes and which this step deliberately leaves in place.

**What was built.**

* `registry::board_memory_sizes` gives a latched board **`prg_ram: Some(0)`**. The latch *replaces*
  the chip, it does not sit beside it: without this the 8 KiB floor would keep RAM on top of the
  latch and `$7000`-`$7FFF` would answer RAM where the page says open bus.
* `Vrc2Vrc4::write_register`'s `address < 0x8000` arm keeps bit 0 of a write to `$6000`-`$6FFF`.
  **That arm is reached only by a write the board already dropped**, so the cost is one `is_some`
  on a path no VRC game takes.
* `Vrc2Vrc4::try_read_prg` answers the bit anywhere in `$6000`-`$6FFF`. The `#[cfg]` is on the
  **method**, so with the feature off there is no override at all and this board's arm of the
  `#[inline(always)]` `Board::try_read_prg` dispatch — the pinned read path of PR-1, MF-5b and
  MF-CART — is byte for byte the base commit's.
* `reset` puts the latch back to 0 on a board that has one and leaves `None` alone: the field is
  the wiring as much as the value.

**Feature `board-vrc2-latch`, in `default`** pending step 8's figure, on `board-298`'s stated
reason: the five Konami numbers may not pay for a register only three of them can have.

**Two choices the entry did not foresee, both in Q-174.**

1. **The top seven bits read 0, where the page says open bus.** A board below `$8000` has no way to
   the bus byte in this build except `CartridgeMemory::cpu_edge_bus`, and that hook is behind
   `board-143-open-bus`, which MF-FIX **parked** for `mmc5exram` +0.036 %. Reaching it would mean
   putting three more mappers on the `samples_console` flag, i.e. resurrecting a cost the owner has
   refused, for seven bits no source says a game reads. D0 is exact.
2. **The latch is not in the savestate.** Three constraints meet: no change to the other boards'
   blocks (REQ-CORE-090 and the lot instruction), older states must keep loading, and no format
   version bump. A byte written only when the latch exists satisfies the first and breaks the
   second — the PRG-RAM blob survives the board's RAM going to zero, because it is read as a
   prefix, but an extra register byte makes every older state one byte short. So the bit stays out,
   `load_registers` leaves the live latch alone, and the protection check it exists for is a write
   immediately followed by a read in the boot code, which no state taken after boot can observe.
   The precedent is `cpu_edge_bus` and `zero_page`.

**Tests.** Four, in `mapper_tests.rs`: the latch itself (D0 both ways, `$6100` and `$6FFF` the same
register, `$7000`-`$7FFF` open bus, `prg_ram` empty); the iNES 1.0 dump keeping its work RAM and
reading the whole byte back; the eight (mapper, submapper) pairs that decide who gets it; and the
savestate statement, which also checks the latched board's state is the other's **minus its 8 KiB
of RAM** — the format difference and nothing else.

`cargo test --release -p zax-nes -p zax-harness`: **1161 passed, 0 failed, 10 ignored**.
`--no-default-features`: 788 passed, 0 failed, 9 ignored. Clippy `-D warnings --all-targets` on
`zax-nes`: clean.
- [x] 3/10 CPROM (13-1) — bus conflicts on the bank register

### 3/10 — the CPROM's bank register has bus conflicts (entry 13-1)

**One number.** `registry::has_bus_conflicts`'s unconditional list is now
`11 | 13 | 66 | 70 | 78 | 89 | 93 | 152 | 180 | 185`. nesdev ("CPROM") carries the same
unqualified "Subject to bus conflicts: **Yes**" row that put 11, 66 and 78 there, against a bank
register the page places at "$8000-$FFFF" — where the PRG-ROM drives. MR-BUS-1 enumerated the
boards whose pages carry the row and missed this one; the cross-check's own "Counts" section
records that as a gap in the enumeration, not a reversal of a verdict. Nestopia and puNES model the
conflict, FCEUX, Nintendulator and Mesen 2 do not, and *Videomation* is the one cartridge.

**No feature.** `has_bus_conflicts` is evaluated once at load into the `Cartridge::bus_conflicts`
bool that every game already tests on every cartridge write, and the AND happens on a path reached
by a single `STA` per bank change. Nothing is added anywhere.

**Tests.** `mf_leg_1_the_cprom_bank_register_is_subject_to_bus_conflicts` walks the four windows:
`$8100` over the zero-filled PRG bank 0 pulls every bit low and the register takes 0 whatever the
CPU drove, `$A100` over bank 1 gives `3 & 1`, `$C100` gives `3 & 2`, `$E100` gives `3 & 3`.

**One existing test had to move**, and it is the evidence the fix bites:
`req_core_100_cprom_fixes_the_first_4_kib_of_chr_ram` selected its banks by writing `$8000`, which
over the fixture's zero-filled bank 0 now lands as 0 — so it writes `$E100` instead. It is the only
test in the suite the change touched (the whole suite was run to find out). Note for whoever writes
the next one: `$E000` itself is the fixture's `JMP $E000` and carries `$4C`, not the bank number;
the `$x100` addresses are the numbered fill, which is why `prg_windows()` reads there.

`cargo test --release -p zax-nes -p zax-harness`: **1162 passed, 0 failed, 10 ignored**.
`cargo fmt --all -- --check` and clippy `-D warnings --all-targets` on `zax-nes`: clean.
- [x] 4/10 Jaleco SS88006 (18-2) — `$9002`

### 4/10 — the Jaleco SS88006's `$9002` (entry 18-2)

**The page.** nesdev ("INES Mapper 018") gives `$9002` as `.... ..WR`, bit 0 "PRG RAM chip enable
(0: disable; 1: enable)" and bit 1 "Write protection (0: deny writes; 1: allow writes)", and names
the equivalence outright — "same as MMC3", whose `$A001` carries the same pair with the write bit
inverted. This build handled indices 0 and 1 of the `$9000` group and fell through for `$9002`, so
the board took the trait defaults: always enabled, always writable.

**What was built**, which is the three-line shape the entry asked for and `Mmc3Board` already has:
one arm in the `$9000` group storing `value & 0x03`, and `prg_ram_enabled` / `prg_ram_writable`
reading its two bits. A disabled chip accepts no writes either, so `prg_ram_writable` tests both.
`$9003` stays undecoded — the page lists no register there.

**Power-on `$03`, both permitted**, which the lot instruction requires and the entry asks for by
name. Q-175 section 1 records the ambiguity: the page says nothing about power-on, puNES and this
build's own MMC3 come up permitted, and **Nestopia comes up with both denied** — which only works
if every mapper-18 game writes the register before touching its save. Permitted is the reading
under which a game that never writes `$9002` keeps its work RAM at all.

**No feature.** `JalecoSs88006` is its own `Board` variant and both methods are already in the
dispatch; this fills that board's two arms and touches nothing else. Mapper 18 is in no measured
set.

**Not in the savestate** (Q-175 section 2), for Q-174 section 2's reason in the same words: the
board's block has no length prefix, so a byte at its end makes every mapper-18 state written before
this step one short — and because more blocks follow the cartridge's, the mis-parse would not even
be a clean error. The third way out, bumping `NesCore::STATE_VERSION` as MF-UNL-9 did for the
flash, changes the version number in every state file for one board's two bits, which is outside
what this lot may touch. A state loaded into a running machine keeps the live permissions; a machine
powered on has `$03`, the state that cannot lose data.

**Tests.** Two. `mf_leg_1_the_ss88006_decodes_9002_as_its_prg_ram_enable_and_write_protect` walks
power-on, `R`-without-`W` (readable, writes discarded), `R` clear (open bus, nothing latched), both
back with the pre-protection byte intact, `$9003` undecoded, and the register answering across its
`$1000` block. `q_175_the_ss88006_prg_ram_protect_is_not_in_the_savestate` pins the omission and
that the round trip is still byte-identical.

`cargo test --release -p zax-nes -p zax-harness`: **1164 passed, 0 failed, 10 ignored**. fmt and
clippy `-D warnings --all-targets` on `zax-nes`: clean.
- [x] 5/10 MMC1A (MMC1-4) — NES 2.0 submapper 3

### 5/10 — the MMC1A, NES 2.0 mapper 1 submapper 3 (entry MMC1-4)

**What the page actually says**, which is not quite what the entry says. nesdev ("MMC1") prints the
`R` bit of `$E000` as the one place the two ASIC revisions differ:

```text
RPPPP
|++++- Select 16 KiB PRG-ROM bank (low bit ignored in 32 KiB mode)
+----- MMC1A:
           0: fixed bank affects A17..A14
           1: fixed bank only affects A16..A14, bit 3 directly controls A17 across the
              entire $8000-$FFFF address range
       MMC1B:
           0: PRG-RAM enabled
           1: PRG-RAM disabled
```

and, on the MMC1A, "PRG-RAM is always enabled at `$6000-$7FFF`". The entry reads `R` as "bit 4
drives PRG A17"; the page makes it **bit 3** that drives A17, and only for the *fixed* window — the
one window whose bank number does not already come from the register. The switchable window and
32 KiB mode have bit 3 as A17 already, which is the rest of "across the entire `$8000-$FFFF`
address range". AMD-061 makes the page prevail, so the page is what was built. The page also names
the stake: "Two games are known to rely on MMC1A behavior, writing values between `$10` and `$1F`
to the PRG bank register, write to WRAM at `$6000-$7FFF`, and expect the write to succeed."

**What was built.**

* `prg_disable_mask` is zero on a submapper-3 dump as it is on mapper 155, so `R` is not a disable.
* `chr_disable_mask` is **not** zeroed by the submapper, which is a deliberate split from the
  mapper-155 line above it: the SNROM `E` bit is the *board* wiring CHR A16 to the RAM's second
  chip enable, whichever revision of the ASIC sits on it, and only mapper 155's page removes a
  wire. No dump is known that is both a declared SNROM and a declared MMC1A, so the two readings
  have nothing to disagree about today.
* `mmc1a_fixed_mask: u8` is `$08` on an MMC1A, and `update_banks` computes the two fixed-bank
  numbers as `prg & r` and `((0x08 & !r) | (prg & r)) | 0x07`, where `r = (prg >> 1) & mask` is
  `$08` exactly when the board is an MMC1A and `R` is set. A **mask and not a `bool`**, for the
  reason `prg_disable_mask`'s own note gives and MF-6a2 step 2 measured: a test beside the register
  on the commit path cost three plain mapper-1 games up to +8 001 instructions over 600 frames for
  a board that was not theirs.
* `mapper_number` had to learn the difference: zeroing `prg_disable_mask` would otherwise have made
  every submapper-3 dump report **155**. It now tests `mmc1a_fixed_mask == 0` beside it, which
  mapper 155 has and a submapper-3 dump has not.

**Feature `board-mmc1a`, in `default`** pending step 8's figure. **This is the one fix of the lot
with a layout risk**: the mask is a byte on `Mmc1`, the board of `m1`, `mm2`, `metroid` and `z2`,
and MF-6b step 1 priced one byte of displacement on a comparable struct at +0.009 % to +0.027 % on
eleven of PR-1's thirteen ROMs **with no code added at all**. With the feature off the field does
not exist, the struct's layout is the base commit's to the byte, and the two fixed-bank numbers
fold to the constants `outer` and `outer | $0F` — the arms the base commit compiled.

**Mapper 155 is deliberately left alone**, although the page makes it the same revision ("iNES
Mapper 1 and 155 denote the SxROM and 2ME circuit boards mounting the MMC1B (iNES Mapper 1) or
MMC1A (iNES Mapper 155) revision of the ASIC"). It is outside this lot's scope — "no change to the
emulation of any mapper outside the ones listed above" — so a mapper-155 dump still gets the
suppressed PRG-RAM disable it had and no `R`-driven A17. **Follow-up for the owner**: mapper 155
should get `mmc1a_fixed_mask` too, which is one condition on the same line and no new code. It is a
new finding of this step; the report's entry MMC1-4 does not mention it.

**Tests.** Three. `req_core_100_an_mmc1a_never_disables_its_prg_ram` runs the page's own sentence —
`$1A` into the PRG register, then a work-RAM write that must succeed — and checks the same dump
without the submapper loses its RAM, and that a submapper-3 dump still reports mapper **1**.
`req_core_100_an_mmc1a_r_bit_drives_a17_of_the_fixed_bank` walks a 256 KiB image through both fixed
modes with the low nibble held constant: mode 3's fixed window goes `$F`, `$7`, `$F` as `R` and
bit 3 move, mode 2's goes `$0`, `$8`, `$0`, and 32 KiB mode does not move at all.
`mf_leg_1_an_mmc1b_fixed_window_ignores_bit_4_of_the_prg_register` is the regression guard and runs
in **every** configuration.

`cargo test --release -p zax-nes -p zax-harness`: **1167 passed, 0 failed, 10 ignored**.
`--no-default-features`: 792 passed, 0 failed, 9 ignored. fmt clean; clippy `-D warnings
--all-targets` clean in all four configurations (default, `--no-default-features`, and each of
`board-mmc1a` and `board-vrc2-latch` alone).
- [x] 6/10 Irem G-101 (32-1) — *Major League* on the upper page

### 6/10 — *Major League* on the upper one-screen page (entry 32-1)

**One enum variant.** nesdev ("INES Mapper 032"): "Major League wants hardwired 1-screen
mirroring. (CIRAM A10 is tied to **+5V** on this game)." A10 held high is the **second** physical
nametable, which is `NametableMirroring::SingleScreenB`; `IremG101::reset` set `SingleScreenA`, the
lower page. puNES uses `mirroring_SCR1`, the same page as the wiki; the other three emulators do
not implement the case, and **Mesen's `ScreenAOnly` is the lower page** — so this is the one point
of the finding where this build now follows the page against Mesen, which AMD-061 §1 requires.

**Unobservable to the game**, which is why the entry records it rather than raising it: all four
logical nametables show the one physical page whichever of the two it is, and the other kilobyte of
CIRAM is never addressed. It is recorded for the two things that *can* see it — a savestate
compared across emulators, and this project's 3D nametable path if it ever reads the unselected
page.

**No feature, nothing on any path**: the variant is chosen in `reset`.

**Tests.** `mr_32_1_irem_g101_submapper_1_is_fixed_one_screen` now expects the upper page, and
gained the other half of its own finding as a second assertion: a `$9000` write must not move the
mirroring of a submapper-1 dump. It was the only test in the suite the change touched.

`cargo test --release -p zax-nes -p zax-harness`: **1167 passed, 0 failed, 10 ignored**. fmt and
clippy clean.
- [x] 7/10 golden-core over the 98 originals

### 7/10 — golden-core over the 98 originals, against the base commit

```
zax-harness golden-core G:/3dsen/test-originals gc98-<label>.jsonl 1200
```

The base commit was extracted with `git archive a775d0e7` into a **fresh** directory with its own
`CARGO_TARGET_DIR`, built there, and the directory deleted once the digest was taken.

| build | binary md5 | md5 of the 98-ROM digest |
| --- | --- | --- |
| `leg1-base` (`a775d0e7`, fresh tree) | `fad139512082b27b724803020e683b78` | `f79e7df842c3ee43e7d9b67861a927b5` |
| `leg1-fix` (this tree, `default`) | `8a6159927e589ab7e6c032446c6f776a` | `f79e7df842c3ee43e7d9b67861a927b5` |
| `leg1-parked` (this tree, `--features parked-boards`) | — | `f79e7df842c3ee43e7d9b67861a927b5` |

**Byte-identical, and identical to the digest twelve lots have now recorded.** 98 ROMs run, 98
files, both builds. It holds with `parked-boards` on as well, which is what makes the two new
features' park decision in step 8 a figure and not a behaviour change.

**Why it could not have moved, board by board.** The 98 use mappers 0 (10 ROMs), 1 (**27**),
2 (16), 3 (4), 4 (40) and 156 (1). Four of the six fixes are on boards that are not there at all —
13, 18, 22/23/25 and 32 — so only steps 1 and 5 could have shown, and step 5 only on a declared
submapper 3, of which there is none (the three NES 2.0 headers among the 98 are *Castlevania
[World][Konami Collector's Series]* submapper 2 and the two *Zelda II* dumps, submapper 0).

**So the 27 mapper-1 ROMs are an independent second run of step 1's extra check**, at 1200 frames
each instead of 3600 and over 27 dumps instead of five, and it agrees: the SNROM `E` bit fires on
none of them.

**And the headers say the fix is right in the right direction.** Reading bytes 5, 10 and 11 of the
five dumps step 1 tested, plus *The Legend of Zelda*:

| dump | NES 2.0 | CHR-ROM | byte 10 | byte 11 | SNROM before | SNROM now |
| --- | :-: | :-: | --- | --- | :-: | :-: |
| *Final Fantasy (USA)* | yes | none | `$70` (8 KiB NV) | `$07` (8 KiB) | yes | **yes** |
| *The Legend of Zelda (USA)* | yes | none | `$70` | `$07` | yes | **yes** |
| *Metroid (USA)* | yes | none | `$07` (8 KiB volatile) | `$07` | yes | **yes** |
| *Mega Man 2 (USA)* | yes | none | `$00` — **no RAM chip** | `$07` | yes | **no** |
| *Zelda II (USA)* and *(Europe)* | yes | 128 KiB | `$70` | — | no | no |

The three cartridges that really are SNROMs keep the bit; *Mega Man 2*, whose header declares **no
work RAM** — it is an SGROM — loses it, which is the whole point of the narrowing. And *Zelda II*,
which the report's "For the owner" entry 2 names first, **was never affected either way**: its
dumps carry 128 KiB of CHR-ROM, which the old size test already excluded. Worth saying plainly to
the owner, because the entry reads as though it were at risk.
- [x] 8/10 step 1 behind `mmc1-snrom-header`, off by default (Q-176)

### 8/10 — step 1 behind a feature, off by default: the hardware test ROM says otherwise (Q-176)

**What happened.** The final check asks for the ROM-gated suite with both roots set. Run for the
first time in this lot, it fails one case:

```
holy-mapperel/testroms/M1_P128K.nes: expected pass, got fail —
  SNROM (MMC1): detailed 4000 (WRAM 4: $A000 bit 4 does not disable WRAM (SNROM), or
  disables it (all but SNROM)); 128K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK
```

**The report's entry MMC1-1 is wrong on a point of fact, and this is it.** The entry says the
suite's twelve `M1_*` images contain no SNROM-shaped one, that therefore "`Mmc1::is_snrom` returns
false for all twelve", and that the suite "does not actually exercise the SNROM side". Reading the
headers:

| image | NES 2.0 | CHR-ROM | byte 10 | byte 11 | old rule | new rule |
| --- | :-: | :-: | --- | --- | :-: | :-: |
| `M1_P128K.nes` | yes | none | `$00` | `$07` (8 KiB) | **SNROM** | not |
| `M1_P128K_CR8K.nes` | yes | none | `$00` | `$07` | **SNROM** | not |
| the other ten | yes | present, or 512 KiB PRG | — | — | not | not |

The entry is right that **no image declares the SNROM shape** — none has ≤ 256 KiB of PRG, 8 KiB of
CHR-RAM *and* a work-RAM declaration. What it missed is that the old rule did not read the
declaration: it read the **effective** PRG-RAM size, which REQ-CORE-012 floors at 8 KiB, so those
two images *were* SNROMs here. They announce themselves as "SNROM (MMC1)", they report
"8K PRG RAM OK" — the ROM found the work RAM the core gave it — and they probe digit `4000`. They
passed before this lot, and **lot MF-3 step 1 exists to have made them pass**: its own note quotes
this exact failure text.

**So the two sources cannot both be satisfied.** The ROM is a correct hardware test of a board that
has a work-RAM chip; its own NES 2.0 header does not declare one. A rule that asks the header to
name the board cannot see the chip the ROM is testing. The owner's rule is right about real dumps —
step 7's header table shows it keeping the bit on the three genuine SNROMs and taking it off the
SGROM — and the test ROM is right about the hardware.

**The decision is R0's, not this lot's**, so the narrowing is behind **`mmc1-snrom-header`**, out of
`default`:

* with the feature **off** — the shipped build — `Mmc1::is_snrom` is the three-way size test every
  lot from MF-3 to this one's base commit shipped, character for character, and the two rows pass;
* with it **on**, the owner's rule applies and the two rows fail.

It is **not** in `parked-boards`: that umbrella is for boards left out for a figure, and this is
neither a board nor a figure — the precedent is `ppu-exact-sprite-eval` and `ppu-2007-render-read`,
both out of `default` for what they change rather than what they cost.
`cargo build -p zax-nes --features mmc1-snrom-header` is the one flag.

**How the tests stay green both ways.** `Mmc1::SNROM_NEEDS_A_DECLARED_HEADER`, a `pub const` over
`cfg!`, lets a test ask which way a build went. The harness's expectation for the two rows is
`SNROM_ROW`, that constant's answer, rather than `Outcome::Pass` pinned either way; the stale
`Diagnosis` entry for `m1_p128k` in `report.rs` — left behind by MF-3 and never updated — now
explains both configurations and takes its `expected` flag from the same constant. The two
narrowing tests of step 1 are `#[cfg(feature = "mmc1-snrom-header")]`; everything else in step 1 is
unconditional, including the `#[ignore]`d five-ROM extra check, whose statement (no game commits
the bit) is true in both configurations.

**Tests, with `ZAXNES_ROMS` and `ZAXNES_TEST_ROMS` set and `--include-ignored --no-fail-fast`:**

| configuration | result |
| --- | --- |
| `default` | **1175 passed, 0 failed, 0 ignored** |
| `--features zax-nes/mmc1-snrom-header` | **1177 passed, 0 failed, 0 ignored** |

The two extra are step 1's narrowing tests, which only exist in the gated build.

**Q-176** carries the evidence and the question for R0: which of the hardware test ROM and the
header-declared rule wins for a dump whose header under-declares the cartridge it tests.
- [x] 9/10 the callgrind campaign and the feature decisions

### 9/10 — the callgrind campaign, and the three feature decisions

Binaries built by `rust/tools/perf/build-clean.sh` from their own tar into a **fresh** directory
with its own `CARGO_TARGET_DIR`, each deleted as soon as the binary was copied out. Roots:
`ZAX_TEST_ROMS=…/assets-free/test-roms`, `ZAX_COMM_ROMS=…/demo/roms`,
`ZAX_L2_ROMS=…/test-originals`, `ZAX_L2_PACKS=…/campaign/play-user/profiles`. The variant tars were
made from the shipped tar by deleting or adding one line of zax-nes's `default` list
(`rust/target-mf/variant-tar.sh`, `variant-tar-add.sh`, both scratch and neither committed).

| label | tree | md5 |
| --- | --- | --- |
| `leg1-base` | **`a775d0e7`**, the lot's base commit | `efcc1cd005cdf6ec975c164be9a8f4dd` |
| `leg1-again` | a byte copy of the base, the **same-binary floor** | the same |
| `leg1b-ship` | **the shipped configuration** (`530979f7`): the six fixes, `board-vrc2-latch` and `board-mmc1a` in `default`, `mmc1-snrom-header` out | `6ec0ea0ce0c33062b93d03ba74cc5b3e` |
| `leg1b-off` | the shipped tree with **both board features out** of `default` — the four free fixes alone | `5476d3de7d7b76a294910977bffb8046` |
| `leg1b-vrc2` | `leg1b-off` plus `board-vrc2-latch` | `da4a30e837cd70346a318f3ed4228131` |
| `leg1b-mmc1a` | `leg1b-off` plus `board-mmc1a` | `c3cb7eb03120e743adb136cc219dfb4b` |
| `leg1b-snrom` | the shipped configuration plus **`mmc1-snrom-header`** | `5b153d8bedb433c35aa5c47d10f08778` |
| `leg1b-ship2` | a byte copy of `leg1b-ship`, the **second** same-binary floor | the same |

**The base binary's md5 is `efcc1cd005cdf6ec975c164be9a8f4dd`, which is MF-FIX's shipped binary to
the byte** — so this lot's base is that lot's measured binary, the **eleventh** lot in a row for
which that holds, and built rather than assumed.

A first set of six builds was measured before step 8 and is discarded: it carried the SNROM
narrowing unconditionally, which is not what ships. Its one finding survives as a cross-check —
`m1` +0.008 % against the base, the same figure `leg1b-snrom` gives below for the same cause.

#### 9.1 The floor, measured twice on each instrument

PR-1's thirteen, the base binary against its own byte copy: worst **+0.001 %** (`m0` +30 986,
`oamstr` +32 548, `vblnmi` +17 300), set total −21 671. So **±0.001 % is noise on PR-1**, a
twentieth of the tolerance, which is what MF-FIX recorded too.

Level 2 is a different matter, and this is the step's most important figure:

| pair (identical binaries) | `metroid` | `mm2` | `smb1` | `smb3` | `zelda2` |
| --- | ---: | ---: | ---: | ---: | ---: |
| `leg1-base` vs `leg1-again` | −0.000 | +0.000 | +0.004 | −0.009 | **+0.014** |
| `leg1b-ship` vs `leg1b-ship2` | +0.000 | −0.000 | −0.007 | **+0.018** | −0.006 |

**The same binary measured twice differs by up to +0.018 % on a level-2 game.** The owner's
tolerance is +0.02 % per ROM, so on this machine the level-2 instrument sits *at* its own noise
floor and **cannot resolve anything this lot does**. Every level-2 figure below is inside that
spread, and none of them should be read as a cost. PR-1 is the instrument with signal here.

#### 9.2 The shipped configuration against the base commit

PR-1's thirteen:

| ROM | `leg1-base` | `leg1b-ship` | delta | % |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 276 144 | 3 308 310 018 | +33 874 | +0.001 |
| `oamstr` | 3 318 463 579 | 3 318 481 961 | +18 382 | +0.001 |
| `m1` | 4 881 121 571 | 4 881 147 212 | +25 641 | +0.001 |
| `metroid` | 5 115 565 028 | 5 115 559 737 | −5 291 | −0.000 |
| `m4` | 5 468 329 412 | 5 468 349 834 | +20 422 | +0.000 |
| `m0` | 5 517 316 685 | 5 517 339 588 | +22 903 | +0.000 |
| `smb1` | 5 996 382 100 | 5 996 347 410 | −34 690 | −0.001 |
| `mm2` | 6 018 023 926 | 6 018 006 275 | −17 651 | −0.000 |
| `z2` | 6 291 549 527 | 6 291 498 199 | −51 328 | −0.001 |
| `readbuf` | 6 834 494 756 | 6 834 503 592 | +8 836 | +0.000 |
| `duelito` | 7 380 435 837 | 7 380 441 958 | +6 121 | +0.000 |
| `smb3` | 7 949 444 516 | 7 949 447 140 | +2 624 | +0.000 |
| `mmc5` | 11 266 834 150 | 11 266 804 885 | −29 265 | −0.000 |
| **total** | 79 346 237 231 | 79 346 237 809 | **+578** | +0.000 |

**Worst ROM +0.001 %, which is the floor, and the set total is +578 instructions out of
79 346 237 231.** The lot costs nothing measurable.

Level 2: `smb1` +0.017 %, `zelda2` +0.014 %, `smb3` −0.003 %, `metroid` −0.001 %, `mm2` −0.000 %,
total +0.004 %. All inside the ±0.018 % floor of §9.1 — `zelda2`'s +0.014 % is the number the
identical binary produced, and `smb3`, which the identical binary moved +0.018 %, is *negative*
here.

#### 9.3 The attribution

**The four fixes with no feature** (steps 3, 4, 6 and the MMC1A-less part of 5),
`leg1-base` → `leg1b-off`: PR-1 worst `vblnmi` **+0.003 %** (+83 626), set total +90 530
(+0.000 %), nine of thirteen inside ±0.001 %. Level 2 worst `smb3` +0.016 %, inside the floor.
`vblnmi` is `ppu_vbl_nmi` and is on none of the four boards, so its +83 626 is code layout; it is
a seventh of the tolerance.

**`board-vrc2-latch` alone**, `leg1b-off` → `leg1b-vrc2`: PR-1 worst `oamstr` **+0.002 %**
(+58 413), set total +103 283 (+0.000 %). Level 2: **no game slower, total −0.005 %** — the
verdict line's own "keepable". No ROM of either set is a VRC board, so what moved is layout, and
the one real cost — an `is_some` on a write below `$8000` and one in `try_read_prg` — is on paths
these ROMs do not take. **Stays in `default`.**

**`board-mmc1a` alone**, `leg1b-off` → `leg1b-mmc1a`: PR-1 worst `mm2` **+0.001 %** (+44 997), set
total **−46 243**. Level 2 `smb1` +0.010 % against `smb3` −0.023 %, total −0.004 % — mixed signs,
i.e. noise. **The layout risk this feature was created for did not materialise**: the byte on
`Mmc1` cost `m1`, `mm2`, `metroid` and `z2` nothing beyond the floor, where MF-6b step 1 had
priced one byte of displacement on a comparable struct at up to +0.027 %. **Stays in `default`.**

**`mmc1-snrom-header` alone**, `leg1b-ship` → `leg1b-snrom`: PR-1 worst `m1` **+0.008 %**
(+371 127), every other ROM inside ±0.001 %, set total +403 238 (+0.001 %). Level 2 worst `smb3`
+0.002 %, total −0.001 %.

`m1` is `holy-mapperel/testroms/M1_P128K.nes` — **the very ROM of Q-176**. With the feature on it
stops being an SNROM, its `4000` digit fails, and the ROM runs a different path; the +371 127 is
that, not a code cost. The pre-step-8 build set measured the same +0.008 % on the same ROM from the
same cause, which is the cross-check. Inside the tolerance by a factor of two and a half.

#### 9.4 The decisions

| feature | worst PR-1 ROM | worst level-2 game | in `default`? |
| --- | --- | --- | :-: |
| the six fixes as shipped | `vblnmi`/`oamstr`/`m1` **+0.001 %** (the floor); total **+578** | `smb1` +0.017 %, inside the ±0.018 % floor | yes |
| `board-vrc2-latch` | `oamstr` **+0.002 %**; total +103 283 | none slower, total −0.005 % | **yes** |
| `board-mmc1a` | `mm2` **+0.001 %**; total **−46 243** | mixed signs, total −0.004 % | **yes** |
| `mmc1-snrom-header` | `m1` **+0.008 %** — the test ROM's own path, not a code cost | `smb3` +0.002 %, total −0.001 % | **no — Q-176, and not for its figure** |

**Nothing is parked for a figure.** All three features are inside the owner's +0.02 % per ROM on
both instruments, so the "no exclusion for performance" rule is not reached at all;
`mmc1-snrom-header` is out of `default` for what it changes (two `holy-mapperel` rows) and not for
what it costs, which is why it is **not** in `parked-boards` either — the same place
`ppu-exact-sprite-eval` and `ppu-2007-render-read` sit, and for the same reason.
- [ ] 10/10 the final check and the report annotations
