# Lot MF-4c — a PPU dot loop chosen at load, so the MMC5 can hear its nametable fetches

Baseline `a5994fd1` (MF-4b's end). Q-116 issue 3, finding MR-5-4, AMD-061 §9.

## Steps

1. `Palette RAM Quirks`: a `$2007` read of palette RAM is ANDed with `$30` under greyscale.
2. The design of the variant, and the alternatives rejected — **before coding**.
3. The notification split read/write, with no board using it yet; instruction-identical.
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

## Status

- [x] step 1 — `Palette RAM Quirks`
- [x] step 2 — the design of the variant
- [ ] step 3 — the notification split read/write, no board using it
- [ ] step 4 — MR-5-4 on the MMC5
- [ ] step 5 — the MMC5 games
- [ ] step 6 — the suites, the campaign, the documents, the guard
