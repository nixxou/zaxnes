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
- [ ] **4/7** — the question tickets, the four sweeps, the save-state round-trips and the feature
  configurations.
- [ ] **5/7** — the callgrind campaign per board against the base commit, and the feature decision
  for each.
- [ ] **6/7** — golden-core against the base commit, and the full `--include-ignored` run with both
  ROM roots.
- [ ] **7/7** — `BOARD-FEATURES.md`, the census note, the journal and the final pass.

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
