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
- [ ] **1/7** — `board-163`, the Nánjīng FC-001: the four write ports, the feedback latch, the PRG
  composition, and the 4 KiB CHR-RAM page latched off the rise of PPU A13.
- [ ] **2/7** — `board-162`, the Wàixīng FS304: the second kind of the same variant, its own PRG
  composition from the same four registers.
- [ ] **3/7** — `board-178`, the Wàixīng FS305 / Nánjīng NJ0430: $4800-$4803, the four PRG modes,
  the mirroring register and the banked PRG-RAM window.
- [ ] **4/7** — the question tickets, the four sweeps, the save-state round-trips and the feature
  configurations.
- [ ] **5/7** — the callgrind campaign per board against the base commit, and the feature decision
  for each.
- [ ] **6/7** — golden-core against the base commit, and the full `--include-ignored` run with both
  ROM roots.
- [ ] **7/7** — `BOARD-FEATURES.md`, the census note, the journal and the final pass.
