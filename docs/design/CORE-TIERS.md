# CORE-TIERS — several Zaxnes cores by mapper group, and Fidélité+ (design, 2026-11-13)

## What the owner asked for

Plan of 2026-11-09, completed 2026-11-12/13:

| Core | Boards | Chosen | Perf rule |
|---|---|---|---|
| 1. Base | 0, 1, 2, 3, 4 (+118, MC-ACC), 7 — frozen | automatically, at load | +0.02 % per ROM |
| 2. Officiel | + licensed carts and big homebrews | automatically | +0.02 % on its own set |
| 3. Extra | + common pirates and multicarts | automatically | owner's tolerance |
| 4. Super-Extra | every board (the parked ones too) | automatically | measured, not binding |
| 5. Super-Extra-FidelitePlus | every board + accuracy rules (exact NMI rule, costly AccuracyCoin fixes, parked accuracy options) | **a checkbox "Fidélité+"** | measured, not binding |

Cores 1-4 share one timing and must give identical frames; core 5 diverges on purpose and has its
own golden-core. The measurement of 2026-11-09 (`verify/rust-v2/src-tierbase`, a throwaway copy
reduced to the six base boards, against `6c253faa` and its 141 boards): **core −1.5 % to −4 %**
(SMB3 −3.99 %), total −1 % to −2.5 %.

## What the code is today

* One `Board` enum (`cartridges/board.rs`) of 37 variants, three of them nested enums
  (`SmallBoard`, `IrqBoard`, `DecodeBoard`) that exist precisely because every new top-level
  variant was measured to cost: jump tables grow, LLVM moves fall-throughs, `#[inline(always)]`
  chains go out of line (MF-6a3, MF-6c, MF-LEG-3 notes in the file).
* `NesCore` is a concrete type and the application uses it as one: about 60 mentions in
  `zax-app`, 50 calls of `EngineHandle::zaxnes()`/`zaxnes_mut()` (bus, PPU, cartridge, hack packs,
  wide picture, achievements, Lua), about 100 mentions in `zax-harness`.
* A timing type parameter existed (`NesCoreOf<P: TimingProfile>`, NESHAWK-PROFILE lots) and was
  removed on 2026-11-12 (`5767b937`); its design is in git history and is the template for core 5.

## Three ways to build it

### A. One core type per tier: `NesCore<B: BoardSet>`

`Cartridge`, `NesBus` and `NesCore` take a type parameter naming the board enum of the tier (a
`BoardSet` trait with `type Board: Mapper` and the registry's `create`). Each tier is its own
enum, generated from one table by a macro. Core 5 adds a second parameter, the timing.

* **For**: exactly the measured situation — each core is compiled with only its own boards, the
  dispatch of core 1 has six arms, adding a board to core 4 can never move core 1's figures.
* **Against**: the application's 50 `zaxnes()` calls need either a 5-way enum with forwarding for
  every method they use, or a trait object over those methods (one indirect call each — most are
  per frame, not per cycle, so affordable but to be measured). The harness's tools take a type
  parameter. Binary size: five copies of the core (the profile lot measured one copy at 266 KiB
  for the old core; with 141 boards, core 4 is the big one — estimate 1-2 MiB more in total).
* **Effort**: 3 to 5 days, most of it in the application's forwarding and the tests.

### B. One core, two-level dispatch

`Board` keeps the six base boards as top-level variants and puts every other board behind one
boxed `Board::Extended(Box<ExtendedBoard>)`, `ExtendedBoard` being today's other 31 variants.

* **For**: a few files in `zax-nes`, no change to the application or the harness; no binary growth.
* **Against**: it is a **hypothesis** that the gain comes from the width of the dispatch — the
  file's own notes show the layout of the *whole* enum matters (niches, tag position, the MMC5's
  inline chains). The boards behind the box pay a pointer load per call; `IrqBoard`'s note says
  some of them are on per-cycle paths. And it does not give core 5 (that needs a timing parameter
  anyway, as A's second parameter).
* **Effort**: 1 to 2 days, of which the measurement is half.

### C. Separate binaries

One `zax-nes` per tier through Cargo features, one executable per tier, the launcher picking.
Rejected: the application, the settings and the savestates would have to cross process
boundaries; it is A with worse plumbing.

## Recommendation

1. **Measure B first (half a day)**: a prototype of the two-level dispatch, callgrind on the five
   level-2 games and PR-1's set against the current `dev`, and the same against a fresh base-only
   copy (the 2026-11-09 measurement was against `6c253faa`; the gap may have moved). If B recovers
   **at least about 80 %** of the base-only gain on the base games without costing the others more
   than the +0.02 % rule, take B for cores 1-4: no binary growth, no application refactor.
2. **Otherwise A**, in the order of the plan: the generic `NesCore<B>` with every board in one
   tier first (identical frames, identical figures), then the six base boards in core 1, then
   Officiel, then Extra.
3. **Core 5 in both cases** is the timing parameter brought back from `5767b937`'s parent
   (`NesCoreOf<P>`), with `Standard` and `FidelitePlus`; the checkbox selects
   `NesCoreOf<FidelitePlus>` for the game. Its first content: the exact NMI rule of Mesen, then the
   eight AccuracyCoin rows NesHawk passes and Zaxnes fails, then the parked accuracy options.

## Open questions for the owner

* B is less than what was planned (one core, not four) if it gives the same speed: acceptable?
* Binary growth for A (estimate 1-2 MiB, more on the APK): acceptable?
* Core 5 and savestates: a Fidélité+ state refused when the box is unticked, as between engines
  (proposed), or converted?

## Re-study (2026-11-14): performance first, without duplicated code

The owner's constraints: performance is the absolute priority, but no duplicated code, and the
code must stay understandable and maintainable.

### Where the measured gain comes from

The 2026-11-09 base-only copy (`l2/tierbase` against `l2/6c253faa`, exclusive cost per function,
`scratchpad tierdiff.py`) shows that the gain is not spread across the core. It sits in a few
places:

| Function | SMB3 (MMC3) | SMB1 (NROM) | MM2 (UxROM) |
|---|---|---|---|
| `Board::on_ppu_address_bus` (A12 watching) | −0.83 % | — | — |
| `Board::ppu_read` (CHR fetch) | −0.46 % | −0.17 % | −0.13 % |
| `Board::ppu_peek` (tile emitter) | −0.27 % | −0.38 % | −0.23 % |
| `Cpu6502::step`, `NesBus::tick`, `Ppu2C02::run_dots` (inlining around the calls) | −0.5 % | −1.4 % | −0.5 % |

So the gain is the cost of the **dispatch on the `Board` enum** at the calls made per fetch and per
cycle, and of what that dispatch prevents (inlining of the board into the PPU fetch and the bus).
The number of boards matters only through that dispatch.

### What the options do to the code

* **A (one core per tier)** removes part of the dispatch (a smaller enum) but keeps it, and moves the
  cost into structure: four or five core types, the application's 50 `zaxnes()` calls forwarded
  through an enum, the harness made generic, a board's tier to decide and to move. The tier
  boundaries are themselves a new thing to maintain.
* **B (two-level enum)** keeps the dispatch on every call and adds a pointer load for the boards
  behind the box; the layout rules of `board.rs` (append only, niche, `Mmc5Ext`) already show how
  fragile that is.

### D. One generic hot path, specialised per popular board (recommended)

The CPU is already generic over its bus (`Cpu6502::step::<B: CpuBus>`, with `wide_decode::Guest` as
a second implementation). The proposal extends the same idea to the cartridge:

* a trait `CartAccess` with the calls the hot path makes (`ppu_read`, `ppu_peek`,
  `on_ppu_address_bus`, `cpu_write`, `irq_asserted`, the CPU read windows…);
* two implementations: `Cartridge` itself (today's enum dispatch, every board), and
  `Specialized<'a, M: Mapper>`, a borrow of the cartridge's memory and of **one** concrete board;
* the PPU's `Cart` handle and the bus's `CpuBus` view take the type parameter (about 35 PPU
  functions, inside `zax-nes` only);
* `NesCore::run_frame` matches the board **once per frame** and runs the frame through the path
  compiled for it: `Board::Nrom(b) => self.run_frame_with(Specialized::new(mem, b))`, …, and every
  other board through the generic path. The list of specialised boards is one table in one file.

What it gives:

* **No duplicated source.** Each board is written once; the CPU, PPU and bus are written once. The
  compiler makes the copies (monomorphisation), as it already does for the CPU.
* **More than the tiers.** In a specialised path there is no dispatch at all: the board's
  `ppu_read` can be inlined into the PPU fetch. The base-only copy still dispatched over six boards.
* **The public types do not change.** `NesCore`, `NesBus`, `Cartridge` stay concrete; the
  application, the harness, the savestates and the 3D/Lua/widescreen layers see nothing. No core
  to choose at load, no tier to maintain, no option "force Super-Extra".
* **Adding a board never costs the popular ones**: a new or parked board joins the generic path,
  which the specialised paths do not contain. The parked boards can come back.
* **Equivalence is cheap to check**: one test runs each specialised board's ROMs through the generic
  path too and compares frames, cycles and audio.

What it costs:

* Binary size: one copy of the hot path (CPU step, bus tick, PPU dots) per specialised board.
  Estimate 150-300 KiB each; five specialisations (NROM/CNROM/AxROM grouped by their CHR view,
  UxROM, MMC1, MMC3, MMC5) about 1 MiB.
* Compile time of `zax-nes` in release: roughly proportional to the copies.
* A rule to keep: a board specialised must not be reached through `&mut Board` in the hot path
  (the trait is the only door).

**The MMC5's light board must be kept** (owner, 2026-11-14). Today (`board-mmc5-exattr-split`, in
`default` since 2026-11-09, lot MF-LEG-3) a cartridge starts on `Board::Mmc5`, the light board with
neither extended attribute mode nor vertical split, and a CPU write that arms one of them
(`Mmc5Ext::write_arms_substitution`: `$5104` mode %01, `$5200` bit 7, `$5201`, `$5202`) replaces it
with `Board::Mmc5Ext` for good (a reset or a state restores the light board). Under D the two are two
specialisations, `Specialized<Mmc5>` and `Specialized<Mmc5Ext>`; the light one does not even contain
the split code. The switch happens mid-frame, so the specialised loop needs one exit: the arming
write raises a flag, the loop stops at the end of that instruction, and `run_frame` resumes the
frame on the `Mmc5Ext` path. The same exit serves any future board that changes shape at run time.
The prototype checks Castlevania III (light) and a split game (Uchuu Keibitai SDF) against `dev`.

### Fidélité+ in the same model (revised 2026-11-14, owner's question)

Named hooks in the fast loop are not enough: half-cycle CPU/bus interleaving (A1) changes how the
loop itself is stepped, and hooks sprinkled through the CPU and the bus would make the standard path
harder to read and risk its speed. Fidélité+ is split by nature instead:

* **Cold behaviours** (parked accuracy options, glitches, the switches of the FIDELITY lot): they
  stay in the standard core as options, on cold paths, as every step of that lot did: zero cost
  off, no second copy.
* **Structural timing** (PHI1/PHI2 halves, the exact NMI sample, the calibrated shortcuts removed):
  a **second stepper**, its own module (its own CPU micro-step and bus loop, written for that
  model and nothing else), chosen **once per frame** inside `NesCore`, exactly as the board path is.
  What it shares, unchanged: the boards (through `CartAccess`), the PPU and APU units, the tile
  records, the savestate primitives, and therefore 3D, Lua, widescreen and the hack packs.
  The duplication is the stepping code, which is precisely what differs; nothing else is copied,
  and the fast stepper has no Fidélité+ code in it.
* **If the measurement shows the PPU must change too** (sub-dot accesses), the PPU's stepping goes
  into that module as well, still behind the per-frame choice. The last resort, a separate engine
  behind the app's `Engine` trait as `zax-neshawk` is (it already reuses `zax-nes`'s
  `Cartridge`), is kept only if the shared parts cannot follow: it would lose 3D and the hacks
  until ported, which is why it is not the plan.

### Plan and gates

1. **Prototype (1 day, throwaway branch)**: `CartAccess` + `Specialized` for NROM and MMC3 only,
   the rest generic. Callgrind level 2 + PR-1 against `dev`. **Gate**: SMB3 and SMB1 at least as
   good as the base-only copy (−2.5 % and −2.2 %), every other ROM within +0.02 %.
2. **If the gate holds (2-3 days)**: the trait done properly, the specialisation table (UxROM, MMC1,
   MMC5 added one by one, each measured, kept only if it gains), the equivalence test, the
   documentation of the rule.
3. **Fidélité+**: first the cold options and Mesen's NMI rule in the standard core where they cost
   nothing (1-2 days); then the half-cycle stepper as its own module, its scope set by a netlist
   study (estimate 5-8 days).
4. **If the gate fails**: fall back to the tiers (A) — the measurement tells which.
