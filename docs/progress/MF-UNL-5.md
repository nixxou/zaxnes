# MF-UNL-5 — the Chinese-RPG boards with a PPU-driven CHR-RAM window

Lot **MF-UNL-5** of `docs/review/UNL-census.md` §7.2, the fifth of the twelve MF-UNL lots:
**three numbers, three boards, three Cargo features, ~13 original titles.**

Base commit: **`4a1cbffe`** (`MF-UNL-4 (9/9)`), the branch tip when the lot opened.
Branch: `mapper-review`. Worktree: the `wt-mappers` worktree.

| feature | number | board |
| --- | ---: | --- |
| `board-163` | **163** | 南晶 (Nánjīng) **FC-001**: four write ports and one read port at $5000-$5FFF, a 32 KiB PRG bank composed from three registers, a feedback/protection latch, and a 4 KiB CHR-RAM page latched off the PPU address bus |
| `board-162` | **162** | 外星 (Wàixīng) **FS304**: the same four write ports and the same CHR-RAM latch, with a different PRG composition and no feedback latch |
| `board-178` | **178** | 外星 **FS305** / 南晶 **NJ0430**: four registers at $4800-$4803, four PRG banking modes, register-driven mirroring, and an 8 KiB **banked** PRG-RAM window |

## The census's premise holds for two of the three numbers, and this is the lot's first finding

§7.2's heading is "the Chinese-RPG boards with a PPU-driven CHR-RAM window" and its rationale is
that all three boards "select a 4 KiB CHR-RAM page **from the PPU address lines** rather than from a
register". **Mapper 178's own page says the opposite**, and MF-UNL-2's rule — the number's own page
wins, each case in a Q ticket — applies:

| what | the census §7.2 | *INES Mapper 178* | ticket |
| --- | --- | --- | --- |
| the CHR window | "All three select a 4 KiB CHR-RAM page from the PPU address lines"; step 3 is listed under the same heading | *Banks*: "PPU $0000-$1FFF: **8 KiB unbanked CHR-RAM**", and the chipset "is designed for using 8 KiB of chip-internal unbanked CHR-RAM". **No sentence of the page mentions PPU A9, PPU A13, a 4 KiB CHR window or any CHR switching at all** | **Q-154** |
| the register addresses | step 3: "the four registers", under a heading whose other two boards decode $5000/$5100/$5200/$5300 | **$4800, $4801, $4802, $4803** | **Q-154** |
| the PRG-RAM | "All three also want battery-backed PRG-RAM" | "CPU $6000-$7FFF: 8 KiB **switchable** PRG-RAM bank", from a register with "PRG-RAM A20?..A13" — banked, and the page says nothing about a battery | **Q-154** |

So **178 is not in either PPU watch list** and adds no `on_ppu_address_bus` body; it is in this lot
because it is the third Chinese-RPG board of the census's roster and because its *other* two
mechanisms — the banked PRG-RAM window and the four PRG modes — are the ones its titles need.

## Where the three boards go

All three are a register decode and nothing else: no IRQ, no expansion audio, no CPU-cycle hook.
That is [`DecodeBoard`]'s entry condition, and MF-CART step 6 measured twelve more variants in a
shared enum at `mmc5` **+3 695** with a worst ROM of +0.002 %, so the discriminant is no longer the
scarce thing it was through MF-6c.

* **162 and 163 share one variant**, `DecodeBoard::ChrWindowBoard`, with a runtime
  `ChrWindowKind` saying which — the shape `SingleLatch` uses for seven boards and `Jv001` for six.
  The two boards decode the same four ports under the same masks and carry the *same* CHR-RAM latch
  (the rise of PPU A13 latching PPU A9); only the PRG composition and the feedback latch differ, and
  both of those are off every hot path.
* **178 is its own variant**, `DecodeBoard::WaixingFs305`: a different decode, a different window
  layout, a `prg_ram_window` override no other board of the enum needs, and no PPU hook.

`Mapper::expansion_audio_output` stays `None` and `clocks_on_cpu_cycle` stays false for all three,
so the deferral of 163's submapper-1 ADPCM and 178's submappers 1-3 is invisible to every other
board (§7.2 step 4).

**Scope: submapper 0 only**, as §7.2 states explicitly. 163's submapper 1 (the ADPCM chip of
*真 Samurai Spirits*), 178's submapper 1 (the infrared sensor), submapper 2 (the TMS5220 speech
chip) and submapper 3 (the solder-pad read-out) are deferred; every category-A title on all three
boards is a submapper-0 cartridge.

## Steps

- [x] **0/7** — the plan: this file.
- [x] **1/7** — `board-163`, the Nánjīng FC-001: the four write ports, the feedback latch, the PRG
  composition, and the 4 KiB CHR-RAM page latched off the rise of PPU A13.
- [x] **2/7** — `board-162`, the Wàixīng FS304: the second kind of the same variant, its own PRG
  composition from the same four registers.
- [x] **3/7** — `board-178`, the Wàixīng FS305 / Nánjīng NJ0430: $4800-$4803, the four PRG modes,
  the mirroring register and the banked PRG-RAM window.
- [x] **4/7** — the question tickets, the four sweeps, the save-state round-trips and the feature
  configurations.
- [x] **5/7** — the callgrind campaign per board against the base commit, and the feature decision
  for each.
- [x] **6/7** — golden-core against the base commit, and the full `--include-ignored` run with both
  ROM roots.
- [x] **7/7** — `BOARD-FEATURES.md`, the census note, the journal and the final pass.

---

## Step 1 — the Nánjīng FC-001, and a CHR window that is the PPU's address bus

`board-163`, in `default`. One new module, `cartridges/mappers/chr_window.rs`, one new
`DecodeBoard` variant (`ChrWindowBoard`, with a `ChrWindowKind` field so that the lot's second
PPU-latched board joins it rather than the enum), two forwards widened, and the number in **both**
PPU watch lists.

### The register map, from nesdev's *INES Mapper 163*

| port | mask | what it is |
| --- | --- | --- |
| `$5000`, write | `$FF00` | `C... PPPP` — the CHR switch (bit 7) and PRG A18..A15 |
| `$5100-$5101`, write | `$FF01` | the feedback latch, `.... .F.E`, `A0` picking the action |
| `$5200`, write | `$FF00` | `.... ..PP` — PRG A20..A19 |
| `$5300`, write | `$FF00` | `.... .A?B` — `B` swaps D0/D1 on the bank ports, `A` sources A15/A16 |
| `$5500-$5501`, read | `$F300` | `.... .F..` — the **inverted** feedback bit |

**The PRG bank is six bits** — `A15 | A16<<1 | A17<<2 | A18<<3 | A19<<4 | A20<<5`, 2 MiB — and the
page's own arithmetic check is what pins the composition: "All registers are initialized to $00 on
reset", and "because reset clears the A bit, games will boot in 32 KiB PRG-ROM bank **#3**". With
`A` clear the two low bits are forced to `11b`, which is bank 3 and nothing else. The *Notes*
confirm it from the other side — "cartridges should be dumped with `$5300=$04`", `$04` being the
`A` bit alone.

**The read mask is `$F300` and not `$FF00`**, which is worth writing down because it is not a
typo: `$5500 & $F300` is `$5100`, so the port answers at every `$5xxx` address whose A9 is clear
and whose A8 is set — `$51xx`, `$55xx`, `$59xx`, `$5Dxx` — and the feedback *write* port is one of
those four windows. One hardware decode, read two ways, and the test walks all four.

### The CHR window, which is the mechanism the lot is named after

> 0: CHR A12=PPU A12 (disable auto-switch)
> 1: CHR A12=PPU A9 latched on last rise of PPU A13 (enable auto-switch)

With bit 7 of `$5000` clear the 8 KiB chip is flat. With it set, **CHR A12 stops being the fetch's
own A12** and becomes a latch, so `map_chr_4k(0, page)` and `map_chr_4k(4, page)` both show the
same page and `$0000` and `$1000` read the same byte — which is the only way to say "the left
pattern table is used for the top half of any nametable, and the right pattern table for the
bottom half, regardless of the scroll position" in a bank-window core.

PPU A13 rises when the address enters `$2000-$3FFF`; A9 of such an address is the half of the
nametable being read. So the hook is `Mapper::on_ppu_address_bus` and the board is in **both** of
§4.3's latches, mapper 96's reason in the same words: a bare `$2006` write that drives the address
across that boundary is the same rise, and neither this core nor the board can tell it from a
fetch's. `req_core_102_mapper_163_latches_its_chr_page_from_the_rise_of_ppu_a13` drives the
without-read entry point explicitly, which is the observation that proves membership of the second
list — the field itself is private to the crate.

**The latch is kept while the switch is off.** The page makes it a property of the address lines,
not of the enable, so a game that turns the switch on mid-frame does not read a stale edge; only
the window update is skipped. That is also what makes the number move in
`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`, whose expected list gains it.

### Three readings, one of them a ticket

| what | the choice | why |
| --- | --- | --- |
| **the swap bit's range** | the two **bank** ports (`$5000`, `$5200`) and **not** `$5100` | the page says "swap D0,D1 on writes to `$5000-$5200`", a range that textually contains `$5100` — but that port's two data bits are the feedback latch's `F` and `E`, not bank lines, and the only purpose the page gives the bit is "to obtain the correct PRG bank order". **Q-155**, pinned by `q_155_mapper_163_swaps_d0_and_d1_on_the_two_bank_ports_only` |
| **where the swap acts** | on the **write** path, so the latch holds the swapped value | that is where the hardware puts it: two data pins exchanged on the way into the latch. Clearing `B` afterwards therefore does not re-swap a bank already written, and the test pins that too |
| **the undriven bits of the read** | zero | the page draws bit 2 and says nothing about the other seven; this board drives bit 2 and leaves the rest clear |

### Submapper 1 is deferred, and the deferral is pinned

"Submapper 1 denotes a PCB that mounts the same ADPCM chip used in NES 2.0 Mapper 419 … used only
on the 真 Samurai Spirits - 武士道列传: 侍魂 (NJ-033) cartridge." §7.2 scopes the lot to submapper 0;
`req_core_075_mapper_163_has_no_expansion_audio_and_no_irq_horizon` loads a **submapper-1** image,
runs ten frames and requires `expansion_audio_output` `None`, `ppu_irq_horizon`
`Some(i64::MAX)` and no IRQ — so the gap is a pinned decision and not an omission. One cartridge
is concerned and it is not one of the six category-A titles.

### The memories

`registry::board_memory_sizes` gains a row: `prg_ram: Some(8 KiB)` and `chr_ram: Some(8 KiB)`, both
on the page's *Banks* section. The CHR row earns its place on its own account — the automatic
switch selects a 4 KiB page of **this** chip, so REQ-CORE-012's 32 KiB default would put pages the
hardware cannot address behind PPU A12. The page says "battery-backed" and the dump's own battery
bit still decides whether a host persists the bytes, as it does for every board in that function.

### What it measures to the suites

**1017 passed, 0 failed, 9 ignored** for `-p zax-nes -p zax-harness` (MF-UNL-4 left it at
1011/0/9): the board's six tests and nothing else. `cargo fmt --check` clean and
`cargo clippy -p zax-nes --all-targets -- -D warnings` clean.

---

## Step 2 — the Wàixīng FS304, which is the same four ports read differently

`board-162`, in `default`. **No new `Board` or `DecodeBoard` variant**: the number is a second
`ChrWindowKind` in the variant step 1 paid for, which is `SingleLatch`'s shape for seven boards and
`Jv001`'s for six. What the step adds to the tree is one enum variant, one arm in `prg_bank`, one
arm in `mapper_number`, one arm in the `$5100` decode, a kind test in `bank_port_value`, and the
number in six tables.

### The PRG composition, and the two mode bits where 163 has one

> `B` ($5300.0) — PRG A15 mode: 0: PRG A15=$5100.1; 1: PRG A15=1 if `A`=0, PRG A15=$5000.0 if `A`=1
>
> `A` ($5300.2) — PRG A16 mode: 0: PRG A16=1; 1: PRG A16=$5000.1

with `$5000` reading `C... PPpq` — `PP` is A18..A17 whatever the mode is, `p` is A16 "if $5300.2=1"
and `q` is A15 "if $5300.0=1 **and** $5300.2=1". `$5200` is A20..A19 as on the FC-001.

**The page's own arithmetic check is the boot bank again**: "Games boot in PRG bank #2 since reset
initializes all registers to $00". `B`=0 gives A15 = `$5100.1` = 0 and `A`=0 gives A16 = 1, so the
bank is 2 — and 2 is what the implementation composes, which is the evidence that the four
annotation lines have been read the right way round. The test walks all four states of the two mode
bits, and the `A`|`B` state is the one in which this board and the FC-001 read `$5000`'s low nibble
identically.

### Two places the kind matters, and one of them is a bug if it is missed

| where | why |
| --- | --- |
| `bank_port_value` | `$5300` bit 0 is 163's **swap** bit and 162's **A15 mode** bit. Without the kind test the FS304 would exchange D0 and D1 of every bank port write whenever a game selected "PRG A15=1 if A=0" — i.e. scramble its own banks |
| `try_read_prg` | 162 has no readable register. The method is behind `board-163` and tests the kind inside, so a 162-only build has no feedback decode at all and takes `Mapper`'s default |

### The test the two pages wrote for each other

Each page's *Notes* give a dumper the same one-register check, from its own side:

> (162) To ensure that a cartridge to be dumped is really INES Mapper 162, write $04 to $5300 and
> verify that $5100.1 **changes** the PRG bank.
>
> (163) To verify that the cartridge is really mapper 163, you can rule out INES Mapper 162 by
> writing $04 to $5300 and verifying that $5100.1 does **not** change the PRG bank.

`req_core_102_the_two_chr_window_boards_differ_by_the_dumpers_own_test` is that pair of sentences
as one test — two cores, the same two writes, opposite assertions — plus the other half of the
difference from the same state: only the FC-001 answers a read of the `$F300` window. It is behind
`all(board-162, board-163)`, the two numbers it compares.

### Everything else is shared, and the step says so by not changing it

The CHR switch, the masks, the save-state block, the memories row (`162 | 163` on one arm, both
pages giving the same two lines), the PPU watch lists, the `on_ppu_address_bus` forward and the
`mf_3b` expected list — all of them take the number without a second body.
`req_core_102_mapper_162_latches_the_same_chr_page_from_the_ppu_address_bus` re-runs the census's
own criterion on the second board rather than trusting the sharing, and adds one assertion step 1
had no reason to make: turning the CHR switch on must not disturb the PRG bank, bit 7 not being a
bank line.

### What it measures to the suites

**1022 passed, 0 failed, 9 ignored** (step 1 left it at 1017/0/9): five new tests, one of them the
cross-board pair. `cargo fmt --check` clean, `cargo clippy -p zax-nes --all-targets -- -D warnings`
clean.

---

## Step 3 — the Wàixīng FS305, the board this lot's own heading does not describe

`board-178`, in `default`. One new module, `cartridges/mappers/waixing_fs305.rs`, one new
`DecodeBoard` variant, **one** new forward (`prg_ram_window`), and the number in **neither** PPU
watch list.

### What the page says, against what §7.2 says

The census's heading is "the Chinese-RPG boards with a PPU-driven CHR-RAM window" and its rationale
is that "all three select a 4 KiB CHR-RAM page from the PPU address lines rather than from a
register". Three sentences of *INES Mapper 178* contradict that, and MF-UNL-2's rule applies:

| what | §7.2 | the page | where it lands |
| --- | --- | --- | --- |
| the CHR window | a 4 KiB page from the PPU address lines | "PPU $0000-$1FFF: **8 KiB unbanked CHR-RAM**", and the chipset "is designed for using 8 KiB of chip-internal unbanked CHR-RAM". **PPU A9, PPU A13, a 4 KiB CHR window and CHR switching appear nowhere on the page** | no PPU hook, neither watch list, and `q_154_mapper_178_has_no_ppu_driven_chr_window` is the anti-test |
| the ports | "the four registers", in a section whose other two boards use $5000-$5300 | **$4800, $4801, $4802, $4803** | the decode, and the same test's last block: a write at $5000-$5300 moves nothing |
| the PRG-RAM | "battery-backed", like the lot's other two | "8 KiB **switchable** PRG-RAM bank" from "PRG-RAM A20?..A13", and no battery mentioned | `Mapper::prg_ram_window`, and `min_prg_ram` rather than an exact size |

**Q-154** records all three. The board is still the right third member of the lot: it is the census
roster's third Chinese-RPG board, its titles are the ones §7.2 lists, and the two mechanisms it
*does* have are new to this enum.

### The four PRG modes, as this core's two 16 KiB windows

| mode | $8000-$BFFF | $C000-$FFFF | the page |
| ---: | --- | --- | --- |
| 0 | `(L & 6) \| H<<3` | `(L & 6) \| 1 \| H<<3` | "NROM-256/BNROM (PRG A14=CPU A14)" |
| 1 | `(L & 7) \| H<<3` | `7 \| H<<3` | "UNROM (PRG A14..16=111b if CPU A14=1)" |
| 2 | `(L & 7) \| H<<3` | the same | "NROM-128" |
| 3 | `(L & 7) \| H<<3` | `(L & 1) \| 6 \| H<<3` | "UNROM but with bit 0 of \"fixed\" bank selectable (PRG A15..16=11b if CPU A14=1)" |

Mode 0 masks `L`'s bit 0 away because the page says A14 is the **CPU's** line there, and the test
pins exactly that: `$4801=$00` and `$4801=$01` give the same windows in mode 0 and different ones
in modes 1 to 3. Mode 3 is mode 1 with A14 of the fixed half handed back to the register, which is
what "bit 0 of the fixed bank selectable" says and what the test walks over all eight inner values.

### The banked PRG-RAM window, which is the enum's first

`DecodeBoard` gains its first `prg_ram_window` forward: "PRG-RAM is mapped to CPU $6000-$7FFF and
is banked in 8 KiB amounts", all eight bits of `$4803`. It is MF-CART's hook, asked **once per CPU
write** and cached on `Cartridge`, so no board's read path grows; the other eleven boards of the
enum take `Mapper`'s own default through the forward, which is the whole chip at offset zero —
byte-identical to what the un-forwarded dispatch gave them.

`req_core_102_mapper_178_banks_its_prg_ram_from_4803` writes a distinct byte into each of four
banks of a 32 KiB chip, reads all four back, checks that bank 4 and bank 255 wrap onto banks 0 and
3 the way the missing address lines of a smaller chip would, finds every bank's byte in
`export_battery_ram`, and then does the whole thing again on an image that declares no RAM at all —
where the 8 KiB floor makes every bank the same chip.

### Three sentences of the page this build deliberately does not act on

1. **The two swapped-register dumps** (*宠物: 小精灵 IV*, CRC32 `0xB0B13DBD`, and
   *Education Computer 32-in-1*, `0xF834F634`). §7.2 step 3 asks for a comment and not a
   workaround, "the page attribut[ing] it to bad dumps, not to a hardware variant", and the
   board's doc comment is that note. A CRC32 table would make two bad dumps run and every good
   dump of the same titles stop.
2. **"Supposedly, the new PRG bank is only committed when $4801 is written."** The page's own next
   clause rejects it — "Emulating this supposed behavior however will break Ping Pong" — so the
   board recomputes on every register write.
3. **"The Waixing FS305 circuit board connects PRG A14 directly to CPU A14, making all PRG banking
   modes except BNROM unusable."** A fact about one of the PCBs the number covers, and no header
   field distinguishes an FS305 from an NJ0430. The chipset is emulated; a game on an FS305 selects
   mode 0 anyway, which is the mode in which the two PCBs agree.

Submappers 1 (the infrared sensor's IRQ), 2 (the TMS5220 speech chip) and 3 (the solder-pad
read-out) are deferred by §7.2, and the Dendy timing the educational variants want is a
console-timing matter.
`req_core_075_mapper_178_has_no_expansion_audio_and_no_irq_on_any_submapper` loads all three
submappers, runs ten frames of each, and requires silence and no IRQ — so each deferral is a pinned
decision.

### What it measures to the suites

**1028 passed, 0 failed, 9 ignored** (step 2 left it at 1022/0/9): six new tests.
`cargo fmt --check` clean, `cargo clippy -p zax-nes --all-targets -- -D warnings` clean.

---

## Step 4 — two question tickets, the four sweeps, and the six configurations

### The tickets

| ticket | what it holds |
| --- | --- |
| **Q-154** | mapper 178's page against §7.2's section heading, in three parts: **no PPU-driven CHR window at all** ("8 KiB unbanked CHR-RAM", and PPU A9, PPU A13, CHR A12 and CHR switching appear nowhere on the page), the registers at **$4800-$4803** and not $5000-$5300, and a **switchable** PRG-RAM window the page never calls battery-backed. Plus the two readings this build had to take without the page: the register decode (the four addresses literally, no mask being given) and the PRG-RAM size (`min_prg_ram`, the header deciding) |
| **Q-155** | mapper 163's "swap D0,D1 on writes to **$5000-$5200**", a range that textually contains the feedback port at `$5100`. This build swaps the two *bank* ports only; the ticket gives the four-write sequence that distinguishes the two readings observably, and notes the smaller choice of answering zero on the feedback read's seven undrawn bits |

Both carry a code reference (`// Q-154` on `WaixingFs305::PORT_FIRST`, `// Q-155` on
`ChrWindowBoard::bank_port_value`) and a named test. **Q-155 is the one a reviewer can close in a
session**: the ticket's own four writes on a real mapper-163 dump with the swap bit set answer it.

### The four sweeps of §7.1 item 1

All four are existing tests that reach every number of `SUPPORTED_MAPPERS`, so the three new
numbers joined them by being registered:

| sweep | what it checks for 162, 163 and 178 |
| --- | --- |
| `req_core_100_every_required_mapper_is_registered` | the expected-additions table, **built from the features** so that a number is expected exactly when its feature is on; the table is now 91 rows and the ascending-order assertion covers the three insertions |
| `req_core_102_any_other_mapper_is_refused` | unchanged — none of the three is on the refused list |
| `req_core_090_the_mapper_state_survives_a_save_and_restore` | loads each number with a battery, writes `$8000-$E001` and `$6000`, runs frames, saves, moves, restores, and requires the re-saved bytes identical. It reaches none of this lot's own ports, which is why each board also has its own round-trip (§7.1 item 2, step 1 / 2 / 3) |
| `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` | the reset path of all three, including `$5000`, which **is** a port of two of them |

and `mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` — the invariant, not a list —
gains 162 and 163 in its expected vector and is the test that would have caught a missing watch-list
entry. **178 is deliberately absent from it**, which is Q-154 in test form: the board does not move
on that bus.

### The six configurations

clippy `-D warnings` on `zax-nes` `--all-targets` **and** its tests, in every configuration the lot
adds plus the two §7.1 requires:

| configuration | clippy | tests |
| --- | :-: | --- |
| `default` | clean | **965 passed, 0 failed, 8 ignored** (MF-UNL-4 left it at 948) |
| `--no-default-features` | clean | **783 / 0 / 8** — unchanged by this lot, as by the three before it, every test of it being behind one of its features |
| `--features parked-boards` | clean | **1018 / 0 / 8** (the VS. System's 45 beside the default's 965, less the 8 of its own that overlap) |
| `--no-default-features --features board-163` | clean | 789 / 0 / 8 |
| `--no-default-features --features board-162` | clean | 787 / 0 / 8 |
| `--no-default-features --features board-178` | clean | 789 / 0 / 8 |

`board-162` alone is two tests short of `board-163` alone because
`req_core_102_the_two_chr_window_boards_differ_by_the_dumpers_own_test` is behind
`all(board-162, board-163)` and the lot's 162 set has five tests against 163's six.

**One thing the single-feature configurations found**, and nothing else did: with `board-162` alone
the two feedback-read constants are unreachable, and `-D warnings` is `-D dead_code`. They are now
behind `board-163` beside the method that uses them. That is the fourth lot in a row in which the
*alone* configurations caught something the default build cannot see.

---

## Step 5 — the callgrind campaign per board, and three features in `default`

Five binaries, each built by `build-clean.sh` from its own tar into a **fresh** directory with its
own `CARGO_TARGET_DIR`, which is then deleted. Roots:
`ZAX_TEST_ROMS=…/assets-free/test-roms`, `ZAX_COMM_ROMS=…/demo/roms`,
`ZAX_L2_ROMS=…/test-originals`, `ZAX_L2_PACKS=…` (the 101 profiles, `.3dn`).

| label | tree |
| --- | --- |
| `unl5base` | **`4a1cbffe`**, the lot's base commit |
| `unl5b163` | `b6dec63d`, step 1 — **`board-163` alone**, the new `DecodeBoard` variant and the PPU-bus forward |
| `unl5b162` | `adf5266b`, step 2 — **+ `board-162`**, which adds no variant and no forward |
| `unl5` | `249d4863`, step 4 — **+ `board-178`**, the lot as it ships |
| `unl5off` | the shipped source with the three features **out of `default`** — the control, whose emulation is the base commit's byte for byte |

**One check before any figure was read**: `zax-harness-unl5base` is **byte-identical** to MF-UNL-4's
own measured `zax-harness-unl4ship`. The lot's base commit adds only documentation over the commit
that lot measured, so the two instruments are the same instrument and this lot's deltas are
comparable with MF-UNL-4's.

### PR-1's thirteen ROMs, 300 frames, no pack, each against `4a1cbffe`

| stage | set total | % | worst ROM | ROMs below |
| --- | ---: | ---: | --- | ---: |
| `unl5b163` — `board-163` alone | **+225 155** | +0.000 | `m1` **+0.001 %** (+55 928) | 2 |
| `unl5b162` — **+ `board-162`** | **+323 610** | +0.000 | `m1` **+0.002 %** (+83 939) | 4 |
| **`unl5` — the lot, all three** | **+451 305** | **+0.001** | `duelito` / `m1` / `m4` / `metroid` / `mm2` / `oamstr` **+0.001 %** | 0 |
| `unl5off` — the control | **+129 626** | +0.000 | `vblnmi` **+0.002 %** (+51 433) | 4 |

**The worst ROM of the whole campaign is +0.002 %, a tenth of the tolerance — and it is the
control's.** `unl5off` changes no behaviour at all and is `vblnmi` +0.002 %; `unl5b162` is `m1`
+0.002 %. So on this instrument the lot's cost is inside the noise of gating three features off,
which is the cleanest reading the PR-1 set can give.

The three boards are also **flat in each other's presence**: 163 alone is +225 155 on the set,
163+162 is +323 610, and all three are +451 305 — about +100 000 a board on 79.3 G, i.e. nothing
beyond what one `DecodeBoard` variant and a handful of `match` arms cost. MF-CART step 6's
twelve-variant measurement is what predicted that, and this is the fifth lot to find it holds.

### Level 2: five games, 600 frames with their 3D packs, inputs pressed

| build | `core` | `total` |
| --- | --- | --- |
| `unl5b163` | three **+0, identical to the instruction** (`smb1`, `smb3`, `zelda2`); `metroid` +9 952 and `mm2` +18 944, both +0.000 % | −0.001 % to **+0.006 %** (`smb3`) |
| `unl5b162` | **the same five figures, to the instruction** — 162 adds nothing to `core` at all | −0.008 % to **+0.016 %** (`smb3`) |
| **`unl5`** | **the same five figures again** — 178 adds nothing either | −0.000 % to **+0.016 %** (`smb1`) |
| `unl5off` | **all five +0** | −0.000 % to **+0.016 %** (`smb3`) |

**Every figure of both columns is inside the owner's +0.02 %**, which is what the brief asks be
checked per ROM on `total` as well as on `core`. This is the second MF-UNL lot with nothing outside
the tolerance on either instrument, after MF-UNL-3.

`metroid/core` **+9 952** and `mm2/core` **+18 944** are not new numbers: MF-UNL-1 step 6 and
MF-UNL-3 step 7 both measured those two games at exactly **+9 952** and **+18 944** for their own
lots, and MF-UNL-2 at +0.000 % on the same two. They are the quantum of adding any board to this
tree at all, and this lot pays it once for three boards — `unl5b163`, `unl5b162` and `unl5` are
identical to the instruction on all five `core` figures.

### The `total` column, shown with a same-behaviour pair as the brief requires

The lot's worst `total` is **`smb1` +0.016 %**, inside the tolerance. The stage table says where it
is, and the per-function diff says what it is:

| `smb1`, `unl5` vs `unl5base` | |
| --- | --- |
| `core`, `ppu`, `apu`, `draw_list` | **+0, identical to the instruction** |
| `convert` | +2 495 703 (**+0.209 %**) |
| `lua_hooks` | +2 490 770 (**+0.276 %**) |
| the only symbols above 300 000 | `getgeneric` **+1 380 172**, `luaH_getshortstr` **+1 255 652**, `__memcpy_avx_unaligned_erms` −365 614 |

So 2.64 M of a 2.19 M whole-run move is Lua's string-key hash table, and no symbol of this project
moves by 300 000 on that game.

**The pair the brief asks for is `unl5off`**, whose emulation is the base commit's byte for byte —
no board code of this lot is reachable in it at all. On the same game it is `smb1/total`
**+0.013 %** and `smb3/total` **+0.016 %**, with `luaH_getshortstr` **+1 891 303** and `luaH_set`
+337 056 as its only symbols above 300 000 and **all five `core` figures +0**. A build that changes
nothing moves `total` by as much as the lot does, in the same function, which is the finding
MF-UNL-2 step 8 and MF-UNL-3 step 7 recorded and this lot confirms a third time.

### The decision

| feature | decision | on what |
| --- | :-: | --- |
| `board-163` | **`default`** | PR-1 worst ROM **+0.001 %**, set total +225 155; level 2 three `core` **+0** and two at +0.000 %, worst `total` +0.006 % |
| `board-162` | **`default`** | PR-1 worst ROM **+0.002 %**, set total +323 610, four ROMs below; level 2 **the same five `core` figures as 163 alone**, worst `total` +0.016 % |
| `board-178` | **`default`** | PR-1, with all three on, worst ROM **+0.001 %**, set total +451 305; level 2 the same five `core` figures again, worst `total` +0.016 % |

**No board is parked**, and `parked-boards` keeps the one feature it has had since MF-VS. Every
figure of the lot is inside the tolerance on both instruments and on both columns, so the owner's
rule gives no reason to move any of the three out of `default`; and the gating is free either way —
`unl5off` is set total +129 626 with all five level-2 `core` figures +0.

### Disk and machine

G: stayed above **18 GiB** free on the WSL side throughout. Each of the five build directories was
deleted by `build-clean.sh` as soon as its binary was copied out; no GPU campaign; every long run
in the foreground with its output read, as the headless rule requires.

---

## Step 6 — golden-core against the base commit, and the full check

### The binary that was measured is the binary the commit builds

Step 5's figures come from `unl5`, built from step 4's commit `249d4863`. Step 5 added only its own
progress note, so this step built `unl5ship` from **`6199a58e`** by `git archive` into a fresh
directory and compared:

    2f56dfb40b4093a7569f8f6f45b13506  zax-harness-unl5
    2f56dfb40b4093a7569f8f6f45b13506  zax-harness-unl5ship

**Byte-identical**, so every figure of step 5 is the shipped commit's. (And `unl5base` is
byte-identical to MF-UNL-4's own `unl4ship`, as step 5 recorded, so the whole campaign is on one
instrument.)

### golden-core over the 98 commercial originals, 1 200 frames each

    zax-harness-<label> golden-core /mnt/g/3dsen/test-originals gc98-<label>.jsonl 1200

| | |
| --- | --- |
| `4a1cbffe` (`unl5base`) against the lot (`unl5ship`) | **byte-identical** |
| `4a1cbffe` against the lot with the three features out of `default` (`unl5off`) | **byte-identical** |

`md5 f79e7df842c3ee43e7d9b67861a927b5` for all three 98-line dumps — **the digest MF-UNL-2 step 9,
MF-UNL-3 step 8 and MF-UNL-4 step 8 all recorded**, so four lots and twenty-eight mapper numbers
later no commercial original's pixels, tile records, cycles, audio digest or blargg status has
moved. golden-core runs with no hack code, so there is nothing between the ROM and the core.

### The suite, with the ROM roots set

| | |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **1028 passed, 0 failed, 9 ignored** |
| the same `--no-fail-fast -- --include-ignored`, with `ZAXNES_ROMS=…/demo/roms` and `ZAXNES_TEST_ROMS=…/assets-free/test-roms` | **1037 passed, 0 failed, 0 ignored** |

The nine ignored are the ROM-driven ones, and the second row is the proof they ran rather than
passing vacuously: nine moved out of the ignored column and into the passing one.

### fmt and clippy

`cargo fmt --all -- --check` clean. `cargo clippy -D warnings` clean on **both** crates the lot's
tests touch — `zax-nes --all-targets` in all six configurations (step 4's table) and
`zax-harness --all-targets` in `default`. `zax-app` does not build in this worktree and the lot
touches none of it.

### The owner's rules, one line each

| rule | this lot |
| --- | --- |
| one feature per board that costs | three features for three boards; 162 and 163 share a `DecodeBoard` variant but not a feature |
| +0.02 % per ROM on PR-1's thirteen and level 2's five, `total` **and** `core` | worst figure of either instrument **+0.016 %** (`smb1/total`), worst PR-1 ROM **+0.001 %**; the same-behaviour control is +0.016 % on the same column |
| a board over the tolerance is parked, never dropped | none is over; `parked-boards` keeps the one feature it has had since MF-VS |
| no exclusion for performance | all three numbers are in `default` and all three are reachable behind their feature with `--no-default-features` too |
| no change to the emulation of an existing mapper | golden-core **byte-identical** over the 98 originals, and the three numbers are new |
| no change to the save-state format of existing boards | `NesCore::STATE_VERSION` unchanged; the only blocks added belong to two boards that did not exist at the base commit |
| safe Rust | no `unsafe` anywhere in the lot |
| nothing edited under `spec/` but new Q tickets | `Q-154.md` and `Q-155.md`, both new |

---

## Step 7 — `BOARD-FEATURES.md`, the census note, the journal and the final pass

### The documentation this step carries

* **Three rows in `docs/review/BOARD-FEATURES.md`** — `board-163`, `board-162` and `board-178` —
  each with step 5's figures in both cost columns, the mechanism it brings, the tickets it carries
  and the one command that flips it. The `--no-default-features` and `parked-boards` counts are
  brought to this lot's (**783** unchanged and **1018**, against **965** by default), and the "how
  to turn a board off" command gains the three names.
* **A new section in that file**, *MF-UNL-5 is the first lot with no figure outside the tolerance
  on either column*: the floor measurement (`unl5off`, whose emulation does not change and which
  is `smb3/total` +0.016 % all the same), the three same-behaviour pairs three lots have now built,
  and the per-board shape — 163 alone +225 155, +162 +323 610, all three +451 305, with the *same
  five* level-2 `core` figures to the instruction in all three builds.
* **A note at the head of §7.2's MF-UNL-5 section in `docs/review/UNL-census.md`**, which says
  plainly that **the section's own heading is wrong about one of its three boards** and why, points
  at Q-154 and Q-155, and records the two things the next reader of that section needs: that 162
  and 163 are one mechanism and one variant, and that the lot's measurement is the cleanest of the
  five.
* **A journal entry** in `G:/3dsen/cleanroom/journal/JOURNAL.md`, in the log's own language, with
  the role, the date, the folders consulted, the deliverables, the figures and the two open
  tickets.
* **Two question tickets** under `spec/changes/`, `Q-154.md` and `Q-155.md`, each with its
  provisional choice, its code reference and the observation that would close it.

### The final pass

| | |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo test --release -p zax-nes -p zax-harness` | **1028 / 0 / 9** |
| the same with `--include-ignored` and both ROM roots | **1037 / 0 / 0** |
| golden-core, 98 originals, 1 200 frames | **byte-identical** to `4a1cbffe`, with and without the three features; `md5 f79e7df842c3ee43e7d9b67861a927b5` |
| clippy `-D warnings`, `zax-nes --all-targets` | clean in all **six** configurations: `default` (965 / 0 / 8), `--no-default-features` (783 / 0 / 8), `--features parked-boards` (1018 / 0 / 8), and each of `board-163` (789), `board-162` (787) and `board-178` (789) alone |
| clippy `-D warnings`, `zax-harness --all-targets` | clean |
| PR-1's thirteen ROMs | worst ROM **+0.001 %**, set total +451 305 (+0.001 %) |
| level 2's five games | all five `core` inside (+0 on three), all five `total` inside, worst **+0.016 %** |
| the feature decision | **all three in `default`**; nothing parked |

### What this lot leaves for whoever comes next

**Q-154's first question is a decision and not a measurement**: either §7.2's heading is to be
corrected, or the census knows a source for mapper 178's CHR window that the number's page does not
carry. Everything else in the ticket follows from that answer. **Q-155 closes with four register
writes** on a real mapper-163 image, and the ticket spells them out.

And the thing no lot of this plan can do, in §7.2's own words: **there is no ROM here that runs any
of the three numbers.** Every test of this lot is written from the register table of its board's own
page, which is why §6 of the census graded documentation before §7 ordered the work.

### Lot totals

**3 numbers, 3 boards, 3 features, 17 tests, 2 question tickets, ~13 original titles**, measured at
worst ROM +0.001 % on the instrument that can resolve a board and worst +0.016 % on the one that
cannot resolve below about 0.02 % at all.
