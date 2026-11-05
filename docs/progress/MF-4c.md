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

## Status

- [x] step 1 — `Palette RAM Quirks`
- [ ] step 2 — the design of the variant
- [ ] step 3 — the notification split read/write, no board using it
- [ ] step 4 — MR-5-4 on the MMC5
- [ ] step 5 — the MMC5 games
- [ ] step 6 — the suites, the campaign, the documents, the guard
