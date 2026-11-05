# Lot MF-4c — a PPU dot loop chosen at load, so the MMC5 can hear its nametable fetches

Baseline `a5994fd1` (MF-4b's end). Q-116 issue 3, finding MR-5-4, AMD-061 §9.

## Steps

1. `Palette RAM Quirks`: a `$2007` read of palette RAM is ANDed with `$30` under greyscale.
2. The design of the variant, and the alternatives rejected — **before coding**.
3. The notification split by /RD, with no board using it yet; instruction-identical.
4. MR-5-4 on the MMC5 through it: the in-frame flag from the PPU bus.
5. The MMC5 games.
6. The suites, the campaign ROM by ROM, the documents and the guard.

## Instruments

Baseline post-LTO dump `rust/target-mf/asm/bin-4c-base.s`, built at `a5994fd1` with
`cargo rustc --release -p zax-harness --bin zax-harness -- --emit=asm`.

`rust/target-mf/asm/cmp4c.py` is MF-4b's `offsets.py` plus one normalisation: the `Cs<11>_`
**crate disambiguator** inside every mangled name. MF-4b's dumps and this lot's are built from
different `zax-harness` sources, which moves that hash in every symbol and in every `call`
target without moving an instruction. With it, `bin-4b-s4.s` against `bin-4c-base.s` is
**IDENTICAL on all 24 symbols that have one**, which is the expected result — MF-4b step 5
touched only `zax-harness` — and confirms the baseline is MF-4b's generated code.

## Step 1 — a palette read under greyscale returns six bits ANDed with `$30`

### The rule

`AccuracyCoin`'s `Palette RAM Quirks`, error code **6**: "With *Greyscale Mode* enabled, the
lower four bits of the value read should all be zero", and code **7**: "With *Greyscale Mode*
enabled, the lower four bits of the value **written** should be unaffected". So PPUMASK bit 0
masks the palette RAM's *output*, on the way to the CPU as well as to the pixel; the stored byte
is untouched. Mesen has the same single expression — `(ReadPaletteRam(addr) & _paletteRamMask) |
(_openBus & 0xC0)` with `_paletteRamMask = _mask.Grayscale ? 0x30 : 0x3F`
(`Core/NES/NesPpu.cpp:319`, `Core/NES/BaseNesPpu.cpp:158`) — and the five other codes of that
row were already passing, so this is the whole of it.

The mask is applied **before** the I/O latch is refreshed, which is the only ordering the
hardware has: the latch samples what is on the bus, and what is on the bus is the masked byte.

### The change

`ppu/mod.rs`, the palette arm of `Ppu2C02::read` register 7 — the arm MF-4b step 2 already
touched — `0x3F` becomes `self.palette_read_mask()`, one method returning `0x30` or `0x3F`.
`Ppu2C02::peek` is deliberately **not** changed: it is the side-effect-free host accessor
(REQ-CORE-081) and already answers the stored byte without the latch's two upper bits, i.e. the
value *written*, which code 7 says greyscale does not affect.

### Hot path

Post-LTO `cmp4c.py`, `bin-4c-base.s` against `bin-4c-s1.s`: all 23 other pinned symbols
**IDENTICAL** — `Cpu6502::step` 897, `NesBus::tick` 75, `write_internal` 2 565,
`catch_up_ppu` 219, `Ppu2C02::run_dots` 1 996, `background_fetch` 199,
`read_nametable_inner` 132, `read_pattern_planes` 106, `Board::ppu_read` 312,
`Board::on_ppu_address_bus` 155, `Board::on_cpu_cycle` 351, `Apu2A03::run_cpu_cycles` 873.

`NesBus::read_internal` is **757 → 760**, and the diff is a **single hunk**:

```
-movzbl  #(%rcx,%r9), %r9d        movzbl of the entry, then
-andb    $63, %r9b                  AND with the constant $3F
+testb   $1, #(%rcx)              PPUMASK bit 0
+movl    $63, %eax
+movl    $48, %r10d
+cmovel  %eax, %r10d              $30 or $3F
+andb    #(%rcx,%r9), %r10b       the entry, masked
```

**+3 instructions, inside the `is_palette_address` branch of the register-7 arm.** The
prologue is byte for byte the same — same pushes, same frame — so no callee-saved register was
added, and the RAM, PRG, PPU-register and `$4015` paths are untouched. Only a `$2007` read with
`v` in `$3F00-$3FFF` reaches it: nothing per cycle, nothing per fetch, nothing on a `$2007` read
outside the palette.

### Games whose output may move

A game must read `$2007` with `v` in `$3F00-$3FFF` **and** PPUMASK bit 0 set — i.e. read back a
palette entry while the picture is in greyscale. Greyscale is used for one-frame flashes, and a
palette read-back during one is not something a commercial title does; no golden-core ROM moves.
The *pixel* path already applied greyscale (`SpanPixels::of`, `color_mask`), so nothing visible
changes.

### Tests and ROMs

`ppu_rendering_tests::req_core_047_a_palette_read_under_greyscale_returns_the_value_anded_with_30`
pins both halves on three entries (`$21`, `$0F`, `$3F`): read masked to `$20`, `$00` and `$30`
with greyscale on, and all three read back whole once it is off, which is code 7 — nothing was
stored masked.

| | `a5994fd1` | step 1 |
| --- | --- | --- |
| `AccuracyCoin / PPU Behavior / Palette RAM Quirks` | fail 6 | **pass** |
| `AccuracyCoin` | 124 / 147 | **125 / 147**, the other 22 rows unchanged |

`cargo fmt --check` clean, `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean (**no new `allow`**), `cargo test --release -p zax-nes -p zax-harness`: **705 passed,
0 failed, 7 ignored**, MF-4b's 704 plus this one.

**Save states**: no field added, nothing moved. `palette_read_mask` reads PPUMASK, which the
block already carries.

## Step 2 — the design, and the premise of Q-116 issue 3 that does not hold

### What the brief asks for

> The PPU's fetch path must be able to tell the cartridge about **every nametable and attribute
> fetch** (address and dot), but only for boards that ask for it. Choose the shape once, at load
> time — for example a const-generic flag on the dot loop, giving two monomorphised copies.

The requirement is right. The example is a solution to a problem this core does not have:
**the notification already exists, it already carries nametable and attribute addresses, and it
is already gated by a load-time latch.**

### Where Q-116 went wrong

Q-116 §"Pourquoi le lot MF-2 ne peut pas l'appliquer" says:

> Pendant le rendu, le PPU ne notifie au cartouche que les adresses de **pattern**
> (`ppu/render.rs`, `notify_pattern_address` et le bloc des sprites). La lecture de nametable et
> celle d'attribut passent par `read_nametable_inner`, qui **ne notifie rien**.

Both halves of that sentence are true and the conclusion does not follow. `read_nametable_inner`
notifies nothing, but it is not the top of the fetch path: the background fetch calls
`Ppu2C02::read_vram_inner`, and the **first thing** that function does, for every address it is
given, is

```rust
if access != PpuBusAccess::Inert {
    self.notify_address_bus(address, cart);      // -> Cartridge::on_ppu_address_bus
}
```

which has been there since the first PPU commit (`5d0b4b69`, "Rust T02", `git log -L`). The
nametable fetch (`background_fetch` arm 0 and the two discarded fetches of dots 338 and 340), the
attribute fetch (`fetch_attribute`), both pattern fetches, every sprite pattern fetch and every
CPU `$2007` access all go through it. `notify_pattern_address` is not the *only* notification, it
is an *extra* one: the address of a pattern fetch is driven one dot before the byte is latched,
which is what `mmc3_test_2` test 4 measures.

The ticket's cost estimate — "une notification **nouvelle** sur le chemin de fetch du PPU, …
environ 33 000 instructions par trame **pour tous les jeux**" — is therefore the cost of
something already paid, and already reduced to **one load and one not-taken branch** for the
twenty-six boards that do not want it (`Cartridge::watches_ppu_address_bus`, MF-3b step 2).

### Measured, not argued

The claim was checked before any line of this lot was designed, by temporarily adding mapper 5 to
`watches_ppu_address_bus` and printing every `(address, ppu_clock)` the MMC5's arm received over
two frames of a synthetic mapper-5 ROM with rendering on (the patch is reverted; it is step 4's
real shape). **115 198 events for two frames**, and in them:

| what the probe shows | the event stream |
| --- | --- |
| nametable fetches arrive | `2001 6828`, `2002 6836`, `2003 6844`, `2004 6852`, … one per tile |
| attribute fetches arrive | `23C0 6822`, `23C0 6830`, … `23C1 6854`, one per four tiles |
| the **three identical nametable reads** are there | `2002 7156`, `2002 7157`, `2002 7159` — dots 337, 338 and 340 of a line |
| the "next PPU read" after them | `23C0 7163` — the attribute fetch at dot 3 of the next line |
| the vblank gap | one gap of **7 165 dots**, `2802 88999` → `2BC0 96164`: 21 lines with no PPU read |
| **the gap Q-116 warned about does not exist** | the histogram of gaps *inside* rendering is 1 dot (77 602), 2 (33 258), 3 (483), 4 (479), 5 (3 374). **The maximum is 5 dots**, under two CPU cycles |

That last row is the one that settles Q-116's objection to applying only the *clears*. The
ticket's nine-dot gap — "le dernier fetch de pattern d'une ligne (point 336) et le premier de la
suivante" — is nine dots only if the nametable fetches of dots 337, 338 and 340 and the attribute
fetch of dot 3 are invisible. They are not, so the longest silence during rendering is five dots,
and a counter that drops the flag after three CPU cycles (about nine dots) never fires inside a
frame. The flag therefore falls exactly where the hardware's falls: in vblank, on the 7 165-dot
gap.

### The design

**Nothing is added to the PPU's fetch path.** The MMC5 is added to the boards that ask for the
notification, and one real gap is closed:

1. **`Cartridge::on_ppu_address_bus` is split read / write.** `read_vram_inner` and
   `write_vram_inner` both call it today, and the MMC3's A12 counter is right to see both — A12 is
   driven on a write as much as on a read. The MMC5 must see **reads only**: nesdev's clear is
   "3 CPU cycles pass without a PPU read having occurred (PPU **/RD** has not been low during the
   last 3 M2 rises)", and /RD is not low on a write. Mesen hooks `MapperReadVram` and has no write
   hook on this board.

   So the single latch becomes two, both derived from the mapper number at load:
   `watches_ppu_reads` (4, 5, 64, 118) tested by the read path, `watches_ppu_writes`
   (4, 64, 118) tested by the write path. **The same `Board::on_ppu_address_bus` dispatch
   serves both** — no second dispatch function, no duplicated MMC3 body, and the read path's
   generated code does not move: one byte-compare against one field becomes one byte-compare
   against another.

2. **The MMC5 implements `Mapper::on_ppu_address_bus`** with Mesen's `DetectScanlineStart`, whose
   every part is on the nesdev page (MR-5-4 says so, and MR-1b verified it).

3. **The two clears** go where Q-116 already said they could: the idle counter on the MMC5's
   `on_cpu_cycle` (the board already steps every CPU cycle for its sound, so no other board pays),
   and the `$FFFA`/`$FFFB` hook in `Mmc5::cpu_read_override` (the board already decodes every
   `$8000`-`$FFFF` read, because `Mmc5::rom_windows` returns `NO_ROM_WINDOW` on all four windows,
   so no other board pays and the MMC5 pays one compare).

4. **`Mmc5::on_ppu_scanline` goes.** The bus mechanism replaces all three of its jobs — the set,
   the clear, and the scanline counter with its IRQ compare — so nothing is left for it to do. The
   MMC5 is the only board that implements the hook; the trait method and the PPU's call stay (the
   extension point is documented and two tests drive it), but every arm becomes the empty default,
   which LLVM drops.

### The alternatives, and why they were rejected

| alternative | why not |
| --- | --- |
| **The brief's const-generic dot loop** (`run_dots::<const NOTIFY: bool>`, two monomorphised copies chosen at load) | It solves a problem that does not exist, and it is not free: two copies of the hottest function in the emulator (`Ppu2C02::run_dots` is 1 996 instructions post-LTO, and it inlines `run_visible_span`, `run_visible_span_through_bus`, `background_fetch`, `step_dot`, `process_dot`, `run_render_dot` and `run_rendering_dot`) cost binary size, instruction cache and compile time on every build, for a notification the single existing call site already makes. It would also need the generic threaded through `run_cpu_cycles`, `step_cpu_cycle` and `catch_up_ppu`, i.e. through `NesBus`, and `Ppu2C02` would have to dispatch on the flag at every entry point — which is the per-call boolean test the brief forbids, moved up a level. |
| **A `const`-generic or generic parameter on the *fetch helpers* only** | Same objection, smaller blast radius, same answer: there is nothing to specialise. |
| **A fourth argument on `on_ppu_address_bus`** (`read: bool`) instead of the second latch | `Board::on_ppu_address_bus` is a real out-of-line call on the fetch path (155 instructions, called ~57 600 times a frame on an MMC3 game). A fourth argument is one `mov` per call for mappers 4, 64 and 118 — a regression on boards this lot does not touch, which MF-3's rule forbids outright. |
| **A second `Board` method for writes** (`on_ppu_write_address_bus`) | Works, costs the read path nothing, but duplicates the MMC3's A12 body into a second dispatch and gives the `Board` enum a new distinct arm shape — exactly the trap MF-3b rule 4 records (a new distinct arm turned the bitset into a jump table and cost every game 4.84 M instructions). The second latch gets the same result with no new dispatch. |
| **Counting writes as reads** (no split at all) | Cheaper still, and wrong in one reachable case: a game that uploads to `$2007` during vblank with the NMI disabled would hold `in_frame` set through the upload, where the hardware has dropped it — observable through `$5204` bit 6 and, since MR-5-2, through whether an ExRAM write is taken. The split costs one byte-compare on a path walked a few hundred times a frame. |
| **Keeping `on_ppu_scanline` as well**, as a belt-and-braces set | Two mechanisms racing on one flag is how an off-by-one-line bug hides. The PPU-side hook does not know about rendering switched on mid-frame, which is the whole point of MR-5-4. |

### What this moves, and the one thing R0 must look at

The scanline counter's **semantics** do not change: the first detected line arms the flag and
zeroes the counter with no compare, and the next increments and compares, which is MR-5-3 and
nesdev's sentence. What changes is **which PPU line is the first detected one**.

* Today `on_ppu_scanline` is called with `rendering && scanline < 240`, so the first detected line
  is **visible line 0**: it arms, line 1 gives counter 1, and a target of 1 fires on **line 1**.
* The hardware detects on the PPU bus, and the **pre-render line fetches exactly like a visible
  one**. The probe shows it: the triple at the end of line 239, then the pre-render line's first
  read (`2BC0`, dot 3) is the detection point — it arms — and the triple at the end of the
  pre-render line makes visible line 0 the first *counted* line, counter 1. A target of 1 fires on
  **line 0**.

So **every MMC5 split moves one scanline earlier**, and that is the whole of the ≤ 1-line
divergence AMD-061 §9 declared and this lot removes. It also means the absolute split line goes
back to what it was *before* MF-1: MF-1's MR-5-3 change was right about the counter and, with the
line-boundary approximation it had to keep, moved the observable split one line late; this lot
puts it back for the right reason. Step 5 measures Castlevania III frame by frame and says which
frames move. MR-5-3's test keeps both of its assertions and is re-driven through the bus.

### Save states

No new field goes into the block, so the format does not move and every older state keeps
loading. The four new board fields are transient and re-derived within one line, with one
deliberate choice: `nt_read_counter` is initialised to **2** on a state load, not 0. A state is
written at a frame boundary, i.e. in vblank, and the end of line 239 always leaves that counter at
2 (the triple of dots 337/338/340) — so 2 is the value a state load must restore for the
pre-render line's first read to detect, exactly as it does without a load. With 0 the detection
would slip to visible line 0 and the split would land one line late for that frame, which under
run-ahead (a state load per frame, AMD-041) would be a permanent one-line error.

## Step 3 — the notification split by /RD, with no board using it

### What the PPU drives, in three kinds

Writing the split out made a third case visible that the design section had folded into the
second. The PPU puts an address on the cartridge's bus in three ways, and the MMC5 cares about
exactly one of them:

| what happens | /RD | where | MMC3 / RAMBO-1 (A12) | MMC5 (in-frame) |
| --- | --- | --- | ---: | ---: |
| a render fetch, or a CPU `$2007` **read** | **low** | `read_vram_inner` | yes | **yes** |
| a CPU `$2007` **write** | high | `write_vram_inner` | yes | no |
| a `$2006` write loading `v` | high | `Ppu2C02::write` arm 6 | yes | no |

A12 is an *address* line: it is driven by all three, and the third alone is enough to clock an
MMC3 counter by hand — which is how `mmc3_test` drives it (REQ-CORE-114) and what
`req_core_114_writing_2006_clocks_the_mmc3_counter_by_hand` pins. The MMC5's rule is about /RD
in both directions, the set ("three consecutive PPU **reads** from the same nametable address")
and the clear ("PPU **/RD** has not been low during the last 3 M2 rises"), so it must take the
first row only. Mesen draws the same line with two hooks, `MapperReadVram` and
`NotifyVramAddressChange`, and the MMC5 overriding only the first.

So the name is not "read versus write" but **"with /RD low" versus "the address lines alone"**.

### The change

* `Cartridge::watches_ppu_address_bus` becomes **`watches_ppu_reads`**, and a second latch
  **`watches_ppu_bus_without_read`** joins it; both are `matches!(mapper, 4 | 64 | 118)` in this
  step, so the lot's behaviour is **unchanged** until step 4 adds mapper 5 to the first.
* `Cartridge::on_ppu_address_bus_without_read` is the second entry point. It dispatches to the
  **same** `Mapper::on_ppu_address_bus`, so there is no new `Board` method, no duplicated MMC3
  body and no new arm shape (MF-3b rule 4).
* `Ppu2C02::notify_address_bus_without_read` is its PPU-side caller, used at the two cold sites:
  `write_vram_inner` and the `$2006` arm of `Ppu2C02::write`.
* `Cartridge` gains a fourth `bool`, which trips `clippy::struct_excessive_bools`. The lint is
  allowed on the struct **with the reason on it**: the four facts are independent and each is
  read on a path of its own, and packing them into one byte would turn each test from the single
  `cmpb` the MF-3b and MF-4c measurements are written against into a `testb` of a bit — a
  needless re-measurement for no gain. `render.rs` already carries the same `allow` for
  PPUMASK's four layer switches.

### Hot path

Post-LTO `cmp4c.py`, `bin-4c-s1.s` against `bin-4c-s3.s`. **Every pinned symbol IDENTICAL**, raw,
except one — and that one is identical once struct displacements are normalised:

| symbol | instructions | verdict |
| --- | ---: | --- |
| `Cpu6502::step` | 897 → 897 | **IDENTICAL**, raw |
| `NesBus::read_cycle`, `tick`, `read_internal` | 3, 75, 760 | **IDENTICAL**, raw |
| `NesBus::catch_up_ppu`, `run_dma`, `dma_read` | 219, 511, 109 | **IDENTICAL**, raw |
| `Cpu6502::branch`, `resolve` | 366, 1 725 | **IDENTICAL**, raw |
| `Apu2A03::run_cpu_cycles`, `quiet_cycles` | 873, 90 | **IDENTICAL**, raw |
| `Ppu2C02::run_dots` | 1 996 → 1 996 | **IDENTICAL**, raw |
| `Ppu2C02::background_fetch`, `read_nametable_inner`, `read_pattern_planes` | 199, 132, 106 | **IDENTICAL**, raw |
| `Board::ppu_read`, `ppu_peek`, `on_ppu_address_bus`, `on_cpu_cycle` | 312, 235, 155, 351 | **IDENTICAL**, raw |
| `NesBus::write_internal` | 2 565 → 2 565 | **DIFFERS** raw, **IDENTICAL** normalised |

`write_internal`'s whole diff is two lines, the same field one byte further along because
`Cartridge` grew a `bool`:

```
-cmpb  $1, 11002(%rsi)      +cmpb  $1, 11003(%rsi)
-cmpb  $0, 11002(%rsi)      +cmpb  $0, 11003(%rsi)
```

Same instruction, same encoding length (a 32-bit displacement either way), so callgrind cannot
see it.

**Call sites** (MF-3b rule 1): `Cpu6502::step` 2, `NesBus::tick` 84, `NesBus::read` 116,
`read_internal` 13, `catch_up_ppu` 9, `run_dma` 1, `Apu2A03::run_cpu_cycles` 17,
`quiet_cycles` 2, `Ppu2C02::run_dots` 2, `Board::ppu_read` 4,
**`Board::on_ppu_address_bus` 12 → 12**. Not one site gained or lost: the two cold notifications
moved from one inlined helper to another, and the twelve sites are the same twelve.

### Games whose output may move

**None, and not by argument**: both latches hold the same three mapper numbers as the single
latch did, so every board takes the same path on the same events as at `a5994fd1`. The step is a
refactor whose only observable is the generated code above.

### Tests

* `cartridges::tests::mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` — MF-3b's
  invariant, with the sweep extended from twenty A12 rises to **twenty A12 rises plus three whole
  rendered lines** (`one_rendered_line`: per tile the nametable byte, the attribute byte and the
  two planes driven twice each, then the three identical nametable fetches of dots 337, 338 and
  340 and the attribute fetch that follows them). A board that reacts to nametable fetches rather
  than to A12 is now caught too, which is what step 4 needs. Still exactly `[4, 4, 64, 64, 118,
  118]`, the vacuity guard.
* The same test gains a second assertion, the invariant that makes the notification reliable:
  **a board that watches the PPU's reads must return `None` from `Mapper::chr_view`**. Otherwise
  `Ppu2C02::run_visible_span` takes dots 2 to 255 of every visible line through console VRAM and
  the `ChrView` directly and the board hears nothing — a silent hole, not a slow path. It holds
  for the three A12 boards today and it is what lets step 4 add the MMC5 without touching the dot
  loop.
* `cartridges::tests::mf_4c_the_write_latch_is_the_read_latch_minus_the_boards_that_wait_on_rd` —
  `watches_ppu_bus_without_read` implies `watches_ppu_reads`, over every supported mapper and
  both submappers.

`cargo fmt --check` clean, clippy clean (one `allow` added, on the struct, with its reason),
`cargo test --release -p zax-nes -p zax-harness`: **706 passed, 0 failed, 7 ignored**. The MMC3
suite's hand-driven `$2006` clocking passes, which is the end-to-end check that the
without-read notification still arrives.

**Save states**: both latches are derived from the mapper number at load, like their two
neighbours, and neither is written by `save_state`. The format does not move.

## Step 4 — MR-5-4: the MMC5's in-frame flag comes off its own bus

### The change

`mmc5.rs`, the whole of the documented mechanism and nothing more:

* **`Mapper::on_ppu_address_bus`** is implemented, and mapper 5 joins `watches_ppu_reads`. The
  arm is three instructions — it calls `Mmc5::see_ppu_read`, which is `#[inline(never)]` for a
  measured reason (below).
* **`Mmc5::see_ppu_read`** is Mesen's `DetectScanlineStart` plus the idle reload, each clause a
  sentence of the nesdev page: a nametable fetch (a `$2xxx` address with `address & $3FF < $3C0`)
  consumes an armed detection and raises the flag; the read **after** `NT_READS_TO_DETECT` (2)
  repeats of one `$2xxx` address is the detection point; a different address puts the repeat
  counter back to 0; and every read, of every kind, reloads `ppu_idle_counter` to 3.
* **`Mmc5::detect_scanline`** is MR-5-3 unchanged: the first detected line arms and zeroes with
  no compare, a later one increments and compares against `$5203`, with `$00` the documented
  special case.
* **The two clears**: `ppu_idle_counter` counted down in `Mapper::on_cpu_cycle`, next to the
  sound the board already clocks there; and an `$FFFA | $FFFB` arm in `Mmc5::cpu_read_override`
  that calls `leave_frame` and answers `None`, so the vector byte comes from the ordinary PRG
  path. Only the flag is cleared — Mesen also zeroes the scanline counter and the pending IRQ,
  which the source does not say and which the next detection makes redundant anyway.
* **`Mmc5::on_ppu_scanline` is gone.** The bus replaces all three of its jobs. No board
  implements the hook now; the trait method, `Cartridge::on_ppu_scanline` and the PPU's call stay
  (two tests drive it and it is a documented extension point), and every arm is the empty default.
* **Four fields**, placed after `in_frame`: `need_in_frame`, `ppu_idle_counter`,
  `nt_read_counter`, `last_ppu_read_addr`. The board grows 5 bytes.
* **`// Q-116`** now marks `Mmc5::on_ppu_address_bus`, the code that answers the ticket.

The address-only notifications moved too, and the reason is both correctness and cost:
`notify_pattern_address` and the sprite-fetch steps 3 and 5 put an address on the bus **one dot
before** the byte is latched, with /RD still high — the real read follows on the next dot through
`read_vram_inner`. They now go through `notify_address_bus_without_read`. An A12 counter is in
both latch lists and sees no difference; the MMC5 is spared **two of the six bus events a tile
produces**, which callgrind prices at 229 M instructions over 300 frames of *Castlevania III*
(+5.16 % became +3.01 %).

### Q-121: which PPU line is the first detected one

The counter's arithmetic is MR-5-3's and does not move. What moves is **which PPU line is the
first detected one**, and the source does not say: the pre-render line fetches exactly like a
visible one, three identical nametable reads at dots 337, 338 and 340 included, and nothing
documented resets the repeat counter across V-blank. So it is detected, visible line 0 is the
first *counted* line, and **`$5203` = N fires on visible line N-1**.

`$5203` = 60 on a synthetic mapper-5 ROM, dot of the IRQ read off the PPU:

| frame | line | why |
| --- | ---: | --- |
| 1 | **60** | power-on: the repeat counter is 0, so the pre-render line's first read detects nothing and visible line 0 arms |
| 2 and after | **59** | steady state: line 239's triple leaves the counter at 2, so the pre-render line's first read is the detection point |

`a5994fd1` fired on line 60 every frame. So **every MMC5 split moves one scanline earlier**,
which is the ≤ 1-line divergence AMD-061 §9 declared and this lot removes — and it puts the
observable split back where it was *before* MF-1, for a different reason and with the counter
arithmetic now right. **Q-121** records the ambiguity, takes this reading provisionally (AMD-061
§1: the documented hardware prevails; Mesen agrees; and the page's "the MMC5's scanline IRQ occurs
at PPU cycle 4" matches our detection at dot 3-4), and gives the owner the one-line alternative.
Marked `// Q-121` on `Mmc5::detect_scanline`.

### Hot path: two regressions found by callgrind and fixed, not argued

The generated-code check passed the first shape of this step and **callgrind did not**. Both
regressions were structural, both are the shapes MF-3b's rules name, and both are now 0.

**(a) `Ppu2C02::run_dots`, +30 to +40 M on every game, NROM included.** Removing the MMC5's
`on_ppu_scanline` left `process_dot` with two separate `if self.dot == 1` tests — the vblank/
pre-render block and the sprite capture of SPEC-02 §3.5 — reading the same field. LLVM merged
them and made the second branchless:

```
-cmpl  $238, %r8d          +cmpl  $239, %r8d
-ja    .LBB                +setae %al
                           +...
                           +setne %cl
                           +orb   %al, %cl
                            jne   .LBB
```

so the scanline compare ran on **every** dot reaching `process_dot`, about 21 000 a frame. The
fix is to nest the capture inside the one `dot == 1` block it belongs to, which is what it means;
the comment on it says why it must stay nested. `run_dots` went from +40 M to **−6 to −34 M**.

**(b) `Board::on_ppu_address_bus`, +16.6 M on *Super Mario Bros. 3*.** With the MMC5's detection
inlined into its arm, the 155-instruction dispatch became 214 and its **prologue gained one
instruction** — `movl %eax, %r9d`, the board tag kept alive across the jump-table index — paid by
the three A12 boards on all 17.3 M fetches of 300 frames. This is MF-3b rule 4 exactly: a new arm
with a body of its own flipping a whole dispatch. The fix is `#[inline(never)]` on
`Mmc5::see_ppu_read`: the arm is three instructions, the prologue is byte for byte `a5994fd1`'s
(the tag's own offset aside, `1355` → `1360`), the function is **156** instructions, and SMB3's
`Board::on_ppu_address_bus` is **316 413 291 → 316 413 291, 0 to the instruction**.

Post-LTO `cmp4c.py`, `bin-4c-s3.s` against `bin-4c-s4d.s`, after both fixes:

| symbol | instructions | verdict |
| --- | ---: | --- |
| `Cpu6502::step`, `branch`, `resolve` | 897, 366, 1 725 | **IDENTICAL**, offsets normalised |
| `NesBus::read_cycle`, `tick`, `catch_up_ppu` | 3, 75, 219 | **IDENTICAL** |
| `NesBus::run_dma`, `dma_read` | 511, 109 | **IDENTICAL** |
| `Apu2A03::run_cpu_cycles`, `quiet_cycles` | 873, 90 | **IDENTICAL**, raw |
| `Ppu2C02::read_pattern_planes` | 106 | **IDENTICAL**, raw |
| `Ppu2C02::run_dots` | 1 996 → **1 936** | −60, the dot-1 hook |
| `Ppu2C02::background_fetch`, `read_nametable_inner` | 199, 132 | same length, re-allocated |
| `Board::on_ppu_address_bus` | 155 → **156** | +1, the MMC5's three-instruction arm |
| `Board::on_cpu_cycle` | 351 → **363** | +12, the MMC5's idle counter; **prologue and the empty arm byte for byte identical** (MF-3b rule 3) |
| `NesBus::read_internal` | 760 → **786** | +26, inside the MMC5's inlined register decode: the `$FFFA` arm and the register churn around it. Prologue identical — same five pushes, same `subq $32` — and the RAM, PRG and PPU-register paths byte for byte the same |
| `NesBus::write_internal` | 2 565 → **2 564** | −1 |

**Call sites** unchanged, `Board::on_ppu_address_bus` **12 → 12**.

### No performance regression: measured

`PERFORMANCE-HISTORY.md` §3's procedure, under WSL2: `a5994fd1` from `git archive` and this step
from `git ls-files | tar` with line endings normalised, each in its own fresh directory with its
own `CARGO_TARGET_DIR` and `CARGO_PROFILE_RELEASE_STRIP=false`, then
`valgrind --tool=callgrind --dump-instr=no --cache-sim=no --branch-sim=no ./zax-harness bench-core
<rom> --frames 300 --warmup 0`. The two trees differ in exactly the seven files this lot touches.

**Super Mario Bros. 3 is in the set on purpose**: `Board::on_ppu_address_bus` is reached only by
mappers 4, 5, 64 and 118, so a board-dispatch regression is invisible on the MMC1 and NROM ROMs
R0's baseline uses.

**Whole process:**

| ROM | board | `a5994fd1` | MF-4c step 4 | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `M0_P32K_C8K_V` | NROM | 5 701 077 827 | 5 694 536 051 | **−6 541 776** | −0.115 % |
| Mega Man 2 | MMC1 | 6 197 444 397 | 6 188 861 046 | **−8 583 351** | −0.138 % |
| Super Mario Bros. 3 | **MMC3** | 8 129 267 163 | 8 118 039 834 | **−11 227 329** | −0.138 % |
| Zelda II | MMC1 | 6 492 072 420 | 6 457 595 787 | **−34 476 633** | −0.531 % |
| Castlevania III | **MMC5** | 10 689 703 722 | 11 011 814 947 | +322 111 225 | **+3.013 %** |

**Per symbol, exclusive.** On all four non-MMC5 ROMs, **0 to the instruction** on
`Cpu6502::step`, `resolve`, `NesBus::tick`, `read_cycle`, `catch_up_ppu`, `run_dma`,
`Apu2A03::run_cpu_cycles`, `quiet_cycles`, `Ppu2C02::background_fetch`, `run_sprite_fetches`,
`run_sprite_evaluation`, `read_nametable_inner`, `read_pattern_planes`, `rebuild_sprite_line`,
`Board::ppu_read`, `ppu_peek`, `on_cpu_cycle` — and `Board::on_ppu_address_bus` on SMB3. The only
symbols that move at all, on any of the four, are:

| symbol | NROM | Mega Man 2 | SMB3 | Zelda II |
| --- | ---: | ---: | ---: | ---: |
| `Ppu2C02::run_dots` | −6 413 910 | −8 510 138 | −11 302 020 | −34 467 371 |
| `NesBus::read_internal` | −131 084 | 0 | −9 056 | −828 |
| `NesBus::write_internal` | −16 | −587 | −637 | −1 219 |

All three negative, all four totals negative. Zelda II's −34.5 M is the largest because its world
map is the heaviest `process_dot` load of the set.

**Castlevania III, where the +322 M goes:**

| symbol | delta | per PPU read |
| --- | ---: | --- |
| `Mmc5::see_ppu_read` | +209 953 688 | ~15 instructions, the detection itself |
| `Board::on_ppu_address_bus` | +119 629 323 | ~8, the dispatch it now reaches |
| `Ppu2C02::background_fetch` | +71 533 620 | the call at the fetch sites |
| `Board::on_cpu_cycle` | +50 796 381 | ~3 per CPU cycle, the idle counter |
| `Ppu2C02::run_sprite_fetches` | +17 953 536 | the same call, sprite side |
| `NesBus::read_internal` | +18 741 735 | the `$FFFA` arm and its decode |
| `Ppu2C02::run_dots` | **−166 502 948** | the `on_ppu_scanline` hook, gone |
| everything else (`tick`, `catch_up_ppu`, `ppu_read`, `ppu_peek`, `run_cpu_cycles`, `push_expansion_audio`, …) | **0** | |

about 28 000 reads a frame after the address-only events moved off the read path, at roughly 25
instructions each. **The six MMC5 games pay 3 %, and no other game pays anything.** That is the
cost the brief asked to be measured; if the owner wants it lower, the lever is the same one that
took 5.16 % to 3.01 % — fewer events on the read channel — and the remaining ones are all genuine
/RD-low reads.

### Games whose output may move

**The six commercial MMC5 titles**, by one scanline on every split, as Q-121 sets out, and nothing
else. Every other board takes the same path on the same events as at `a5994fd1`: the two latches
hold the same numbers for them, and the four non-MMC5 ROMs above are 0 to the instruction on every
pinned symbol. Step 5 runs the MMC5 games frame by frame.

### Tests

| test | what it pins |
| --- | --- |
| `mr1_review_tests::mr_5_4_mmc5_in_frame_is_set_by_three_identical_nametable_reads_and_the_read_after_them` | each clause of the set, read by read: two repeats are not enough, a foreign read in between resets the run, the third repeat arms nothing by itself, and the read *after* it detects |
| `mr1_review_tests::mr_5_4_mmc5_in_frame_is_cleared_by_three_idle_cpu_cycles_and_by_the_nmi_vector` | both clears, and that the flag survives **two** idle cycles and not three; `$FFFA` and `$FFFB` each |
| `mr1_review_tests::mr_5_3_mmc5_scanline_target_one_fires_on_the_second_line` | MR-5-3's two assertions, re-driven through the bus instead of `on_ppu_scanline` |
| `mr1_review_tests::mr_5_2_mmc5_exram_write_outside_rendering_does_not_store_the_value` | unchanged, still passing: MR-5-2 reads the same `in_frame` |
| `mapper_irq_tests::req_core_100_the_mmc5_scanline_irq_fires_on_the_line_5203_names` | both lines of Q-121 — the power-on frame on 60 and the four frames after it on 59 — so the transient cannot be pinned by accident |
| `mapper_irq_tests::req_core_100_the_mmc5_scanline_irq_stays_quiet_without_rendering` | unchanged |
| `mapper_tests::req_core_100_mmc5_serves_background_fetches_from_the_second_set_with_tall_sprites` | MR-5-1's in-frame clause, with the flag armed through the bus (`arm_mmc5_in_frame`) |
| `cartridges::tests::mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` | the MMC5 now in the moved set: `[4, 4, 5, 5, 64, 64, 118, 118]` |

`common::mapper_fixture` gains `arm_mmc5_in_frame` and `detect_mmc5_scanline`, which drive the
board's bus the way the PPU's fetch path does.

`cargo fmt --check` clean, `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean, `cargo test --release -p zax-nes -p zax-harness`: **708 passed, 0 failed, 7 ignored**.

### Save states

**The format does not move and no older state is touched.** `in_frame` and `scanline_counter`
were already in the block; the four new fields are transient and out of it, with one deliberate
value on load: `nt_read_counter` is restored to **2**, not 0 (`Mmc5::NT_READS_AFTER_LOAD`). A
state is written at a frame boundary, i.e. in vblank, and line 239's dots 337/338/340 always leave
that counter at 2 — so 2 is what makes the pre-render line's first read the detection point,
exactly as it is without a load. With 0 the first detection would slip to visible line 0 and the
split would land one line late for that frame, which under run-ahead (a state load per frame,
AMD-041) would be permanent. `ppu_idle_counter` is restored to 0 so that a load cannot drop a flag
the block says is set; the first PPU read after the load arms it again.

## Step 5 — the MMC5 games, frame by frame

### The instrument

`zax-harness golden-core <dir> <out.jsonl> <frames>` digests the **picture**, the **tile records**
of SPEC-02 §3.5, the **CPU cycle count** and the **audio** at seven checkpoints — frames 1, 60,
300, 600, 1 200, 1 800 and 3 600 — so a divergence is dated rather than just detected. Both sides
are the `git archive` / `git ls-files` builds of step 4, each in its own directory.

### Castlevania III (`G:/3DSen/mmc5-test`), 3 600 frames

| digest | frame 1 | 60 | 300 | 600 | 1 200 | 1 800 | 3 600 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| picture | same | same | same | same | same | same | **differs** |
| tile records | same | same | same | same | same | same | **same** |
| CPU cycles | same | same | same | same | same | same | **same** |
| audio | same | same | same | same | **differs** | **differs** | **differs** |

So: **the cycle count is identical at every checkpoint, and so are the tile records the 3D engine
draws** — including at frame 3 600. The 2D reference picture is identical for the first 1 800
frames (the title and the opening screens) and differs at frame 3 600, which is inside the demo;
the audio differs from frame 1 200 on.

Both are the one scanline Q-121 explains. The IRQ fires on line 59 instead of 60 for a `$5203` of
60, so the handler runs about 341 dots earlier and the register writes it makes — a bank switch
for the split, a sound register — land a line earlier too. The cycle count not moving says the
instruction stream is the same length; the audio digest moving says one of those writes crossed a
sample boundary.

The harness gives no pixel dump at an arbitrary frame (`test-roms` stops at the first stable
screen, well before 3 600), so the *shape* of Castlevania III's frame-3 600 difference is not
measured here. It is measured exactly on the ROM built for it, below.

### `mmc5test.nes`, where the shift is visible and measurable

`nes-test-roms/mmc5test/mmc5test.nes` is the one MMC5 ROM here that draws from the scanline IRQ,
and it shows the change for what it is. Picture and tile records differ from frame 60 on; the
final-frame PNGs of the two builds, decoded and compared row by row:

```
lines that differ: 159 .. 239, count 76
  base line y equals new line y-1 on 75 of 76 differing lines
  base line y equals new line y+1 on  1 of 76 differing lines
```

**The picture below line 159 is the old picture moved up by exactly one scanline.** Not a
different picture, not corruption: the split boundary one line earlier, which is MR-5-4 applied
and AMD-061 §9's divergence removed. The campaign's text transcription of that screen is
unchanged, which is why step 6's ROM-by-ROM comparison does not flag it — the shift is a pixel
boundary, not a character.

### Every MMC5 ROM in the campaign

| ROM | picture | tiles | cycles | audio | |
| --- | --- | --- | --- | --- | --- |
| `nes-test-roms/mmc5test/mmc5test.nes` | **differs** | **differs** | same | same | the one-scanline shift above |
| `nes-test-roms/other/firefly.nes` | same | same | **differs** at 60, 600, 1 200 | same | the IRQ lands a line earlier, so a polling loop spins a different number of times; nothing observable |
| `nes-test-roms/exram/mmc5exram.nes` | same | same | same | same | drives its bars from V-blank code, not the scanline IRQ |
| `nes-test-roms/mmc5test_v2/mmc5test.nes` | same | same | same | same | a text/bank screen with no split |

### The golden-core set, 1 800 frames

`G:/3DSen/demo/roms`, the 18 commercial ROMs the harness runs there — NROM, MMC1, MMC3 (*Super
Mario Bros. 3*, *Ninja Gaiden III*), UxROM and CNROM among them:

```
18 ROMs, 18 identical on all four digests, 0 moved
```

**Not one of them moves, on the picture, the tile records, the cycle count or the audio.** That is
the statement the lot needs: no board but the MMC5 changed behaviour, and the four non-MMC5
callgrind ROMs of step 4 say no board but the MMC5 changed cost either.

### Golden core: what may move

Only the **six commercial MMC5 titles**, by one scanline on every split, and only in steady state
— the first frame after power-on keeps the old line because the chip's repeat counter starts at 0
(step 4's table). *Castlevania III*, *Just Breed*, *Uchuu Keibitai SDF*, *Gemfire*, *Bandit Kings
of Ancient China* and *L'Empereur* are the six. Measured here: Castlevania III's tile records and
cycle counts do not move at all over 3 600 frames, and its picture moves at one of the seven
checkpoints.

## Step 6 — the suites, the campaign, the documents and the guard

### The campaign, ROM by ROM

MF-4b's own campaign JSON is not on disk (`rust/target-mf` holds its step-4 run, with the old
drive), so the baseline was **re-measured** rather than read: `a5994fd1` rebuilt from a
`git archive` in its own directory, and both builds run against the same ROM collection on the
same host.

```
base 331 rows, new 331 rows, same set: true
rows whose verdict, code or message differs: 1
--- AccuracyCoin/AccuracyCoin.nes
  base: fail 23  124/147 tests passed; …
  new : fail 22  125/147 tests passed; …
```

**331 ROMs, one row.** 214 pass, 5 fail, 0 timeout, 1 unsupported, 111 manual — MF-4b's four
totals exactly. Every per-suite tally identical, and the 111 `manual` screens character for
character what they were, `double_2007_read`'s included.

The four MMC5 ROMs keep their `manual` verdicts and their transcriptions, which is why they are
not in that list; step 5's `golden-core` and PNG measurements are where their change shows.

### The suites

| suite | `a5994fd1` | MF-4c |
| --- | --- | --- |
| `AccuracyCoin` | 124 / 147 | **125 / 147** |
| `cpu_interrupts_v2` | 6 / 6 | **6 / 6** |
| `instr_timing`, `branch_timing_tests` | 3 / 3, 3 / 3 | **unchanged** |
| `cpu_dummy_reads`, `cpu_dummy_writes`, `cpu_exec_space`, `cpu_reset`, `cpu_timing_test6` | all pass | **unchanged** |
| `instr_misc`, `instr_test-v3`, `instr_test-v5`, `nes_instr_test` | 5, 17, 18, 11 | **unchanged** |
| `ppu_open_bus`, `ppu_read_buffer` | 1 / 1 each | **unchanged** |
| `ppu_vbl_nmi`, `vbl_nmi_timing` | 11 / 11, 7 / 7 | **unchanged** |
| `sprite_hit_tests_2005.10.05`, `sprite_overflow_tests` | 11 / 11, 5 / 5 | **unchanged** |
| `blargg_ppu_tests_2005.09.15b` | 4 / 5 | **4 / 5**, the same required row |
| `oam_read`, `oam_stress` | 1 / 1 each | **unchanged** |
| `apu_test`, `blargg_apu_2005.07.30`, `apu_reset`, `apu_mixer`, `pal_apu_tests` | 9, 11, 6, 4, 10 | **unchanged** |
| `sprdma_and_dmc_dma`, `dmc_dma_during_read4`, `dmc_tests` | 2 / 2, 3 pass 2 manual, 4 manual | **unchanged** |
| `holy-mapperel` | 40 / 40 | **40 / 40** |
| `mmc3_test`, `mmc3_test_2`, `mmc3_irq_tests` | 5 / 6 each | **5 / 6 each**, the same required failures |
| the four MMC5 ROMs | 4 manual | **4 manual** |
| **whole campaign** | 214 / 5 / 0 / 1 / 111 | **identical; 1 of 331 rows changed** |

The three MMC3 suites matter more than their unchanged tally suggests: the MMC3 is the board whose
`Board::on_ppu_address_bus` arm this lot had to leave untouched, and step 4's callgrind on *Super
Mario Bros. 3* is the measurement that caught it when it did not.

### The documents

* **`docs/review/MF-T-test-roms.md`** gains "What lot MF-4c changed": the one row of the campaign,
  the `Palette RAM Quirks` rule, the `golden-core` table for the four MMC5 ROMs with the
  one-scanline measurement on `mmc5test`, the suites that did not move, and the guard.
* **`docs/review/MR-1-mappers.md`**: MR-5-4 becomes **our bug, fixed in MF-4c (`08da8e0a`)**, with
  the correction that the entry's own cost note — and Q-116's — misread the call chain, and with
  the callgrind numbers. Its row in the summary table moves from `unknown` to fixed and names the
  two tests.
* **`rust/docs/PERFORMANCE-HISTORY.md`** §5.1 gains the lot's entry: why no notification was
  needed, the two regressions callgrind caught and the two invariants that must survive a port —
  the capture nested inside the one `dot == 1` block, and `Mmc5::see_ppu_read` staying
  `#[inline(never)]` — plus the note that **an MMC3 game belongs in the measurement set**, since
  `Board::on_ppu_address_bus` is invisible on the MMC1 and NROM ROMs the baseline uses.
* **`spec/changes/Q-121.md`** is the lot's one ticket. No spec document was edited; retiring
  AMD-061 §9 is R0's.

### The regression guard

`crates/zax-harness/tests/test_roms_tests.rs` grows from 13 to **14** groups:

| test | what it pins |
| --- | --- |
| `req_core_110_accuracy_coin_keeps_the_rows_mf_4b_and_mf_4c_turned_green` | the tally as a floor of **125**, no test left unrun or retired, and **thirteen** rows by name — MF-4b's twelve plus `PPU Behavior / Palette RAM Quirks` |
| `req_core_100_the_mmc5_rom_suites_keep_their_outcomes` | the three MMC5 suites' rows. All are `manual`, so what it guards is that they still **run**: a board that now reacts to every PPU read can hang or lose its IRQ, and either would show up as a `timeout` |

`ZAXNES_TEST_ROMS=… cargo test --release -p zax-harness --test test_roms_tests`: **14 passed**.

### Hot path, this step

Nothing under `crates/zax-nes/src/` changed, and `cmp4c.py` on `bin-4c-s4d.s` against
`bin-4c-s6.s` says so: **all 20 pinned symbols IDENTICAL**. The step touches the guard and four
documents.

## The lot, end to end

`cargo fmt --check` clean. `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean, with **one** `allow` added — `clippy::struct_excessive_bools` on `Cartridge`, with its
reason on it, matching the one `render.rs` already carries for PPUMASK's four switches.
`cargo test --release -p zax-nes -p zax-harness`: **709 passed, 0 failed, 7 ignored**, against
MF-4b's 704 — five added (the greyscale read, the write-latch invariant, the two MR-5-4 tests and
the MMC5 ROM group), none removed or `#[ignore]`d. **No `unsafe`.**

| brief item | state |
| --- | --- |
| 1. `Palette RAM Quirks`, checked free in the generated code | **done**, step 1: `AccuracyCoin` 124 → 125; +3 instructions inside the palette branch of the `$2007` arm and **0** everywhere else, the diff quoted |
| 2. design the variant, alternatives rejected, written before coding | **done**, step 2 — and the design is that **there is no variant to build**: the notification exists, carries nametable and attribute addresses, and is already gated by a load-time latch. Proved with a probe of 115 198 bus events before a line was written. Six alternatives rejected with reasons, the brief's const-generic dot loop among them |
| 3. implement it with no board using it, callgrind identical to the instruction | **done**, step 3: the split by /RD, both latches holding the same three mapper numbers, **every pinned symbol IDENTICAL** and `Board::on_ppu_address_bus` 12 → 12 call sites |
| 4. MR-5-4 through it, the `// Q-116` mark kept, Q-116's nine-dot warning minded | **done**, step 4. The warning is answered with a measurement: the longest silence on the PPU's bus during rendering is **five dots**, and the idle counter counts reads of every kind |
| MR-5-2 and MR-5-3 keep their tests passing | **done**: MR-5-2 untouched, MR-5-3 re-driven through the bus with both its assertions intact. Q-121 records that the *line* moves while the counter's arithmetic does not |
| the MR-5-4 tests active | **done**: they did not exist (MR-1 recorded "Test: none"), so step 4 wrote them — one per half of the mechanism |
| 5. the MMC5 games, and which frame moves and why | **done**, step 5: Castlevania III over 3 600 frames (tiles and cycles identical at every checkpoint, picture at frame 3 600, audio from 1 200), the four campaign MMC5 ROMs, and `mmc5test`'s picture measured as **the old picture moved up exactly one scanline** |
| 6. the suites, `AccuracyCoin`, the campaign ROM by ROM, the documents, the guard | **done**, step 6: 1 of 331 rows, 14 guard groups, four documents |
| no performance regression, none at all | **done, and twice re-done**: four non-MMC5 ROMs at −0.115 % to −0.531 % with every pinned symbol 0 to the instruction, after callgrind caught +0.5 % on every game and +16.6 M on the MMC3 that the generated-code check had passed |
| safe Rust, no `unsafe` | **done** |
| save states written before this lot keep loading | **done**, the format is unchanged; the one value a load has to get right is `nt_read_counter`, and `Mmc5::NT_READS_AFTER_LOAD` carries the argument |
| the variant chosen correctly after a state load | **done** in the sense the lot ends with: there is no variant, and the board's detection state is restored so that the frame after a load splits where the frame before it did |

**One Q ticket, Q-121**, on the one point the sources leave open. Q-122 is the next free number.

## Status

- [x] step 1 — `Palette RAM Quirks`
- [x] step 2 — the design of the variant
- [x] step 3 — the notification split by /RD, no board using it
- [x] step 4 — MR-5-4 on the MMC5
- [x] step 5 — the MMC5 games
- [x] step 6 — the suites, the campaign, the documents, the guard
