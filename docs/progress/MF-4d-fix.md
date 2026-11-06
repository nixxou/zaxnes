# Lot MF-4d-fix — back out the `$2007`-while-rendering half of MF-4d step 6

Baseline: `fff53974` (ZR-2's end). Reference picture: a clean build of `c4577878` (MF-4a 5/5).

MF-4d step 6 (`a758f923`) made a `$2007` access during rendering move `v` by a coarse X increment
and a Y increment at once. The rule is right on hardware, but applied **blind to the dot** the
access lands on it costs more than it buys:

* **AMD-021 R2-005 criterion 2 fails** — `amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line`
  emits a background row on Super Mario Bros. 3's split line on 4 986 of 5 000 frames, with no hack
  code at all. It passed at `d96d73b7`. R2-005 is **P1**; one `AccuracyCoin` row is not.
* The golden-core pictures of Super Mario Bros. 3 and Zelda II move.
* The widescreen hack of Super Mario Bros. 3 is visibly broken at frame 1 500.

R0's decision: back out **only** that half, which MF-4d had isolated to
`Ppu2C02::increment_vram_address`. The `$2004` half of step 6 moves no golden-core ROM and stays.
Lot **MF-4e** owns the per-dot work and takes the rule back with R2-005 as a required test
(**Q-125**).

## Steps

1. Back out the `$2007` half; Q-125; the unit test inverted.
2. The ignored commercial-ROM tests, with the ROMs present.
3. The `AccuracyCoin` guard's floor, 129 → 128.
4. The pictures: the owner's fortress state, then golden-core against the `d96d73b7` side.
5. The suites and the campaign, ROM by ROM against ZR-2's.

## Step 1 — the `$2007` half is out, and the code is `d96d73b7`'s again

### What changed

`Ppu2C02::increment_vram_address` loses its `fetching()` branch and its `#[inline]`, and is the
body of `d96d73b7` **line for line**:

    let step = if (self.control & 0x04) != 0 { 32 } else { 1 };
    self.v = self.v.wrapping_add(step) & 0x7FFF;

The comment block above it is new and is the deferral: the hardware rule, its three sources
(nesdev "PPU registers" on PPUDATA, `third_party/mesen2`, `AccuracyCoin`'s `$2007 Read w/
Rendering` 2), what it broke, and that MF-4e must satisfy both it and R2-005. Marked `// Q-125`.

`Ppu2C02::increment_coarse_x` and `Ppu2C02::increment_y` go back from `pub(super)` to private, so
`rust/crates/zax-nes/src/ppu/render.rs` is **byte-identical to `d96d73b7`** (`git diff d96d73b7 --`
is empty).

**What is kept.** The `$2004` half of step 6 is untouched: a register write to `$2004` while the
PPU is fetching does not reach OAM and bumps OAMADDR by `(oam_address + 4) & $FC`. Its test
`req_core_112_a_2004_write_while_fetching_bumps_the_address_by_four_and_writes_nothing` is
unchanged and passes. `Ppu2C02::fetching()` survives for it alone, and its doc comment now says
so and points at Q-125.

### The unit test: **inverted**, not `#[ignore]`d

`req_core_112_a_2007_read_while_fetching_increments_coarse_x_and_y_at_once` becomes
`req_core_112_a_2007_read_while_fetching_still_steps_by_the_ppuctrl_amount_q_125`, asserting
`(6, 8, 2)` on the visible line where it asserted `(6, 8, 3)`. The vblank leg is unchanged.

**Why inverted.** An `#[ignore]`d test pins nothing: it would not run, so it could not catch the
rule coming back by accident, and nothing left in the suite would state what `$2007` actually does
during rendering. The inverted test runs on every suite, states the behaviour the lot deliberately
ships, and **MF-4e has to come here and change it** — which puts the deferral at the exact spot
where the rule returns. Its doc comment carries the hardware rule verbatim and the `(6, 8, 3)` it
will assert again, so nothing is lost by inverting it.

### MF-3b's rule, on the five functions the lot names

Post-LTO dumps of clean builds, `cargo rustc --release -p zax-harness --bin zax-harness --
--emit=asm`: the baseline `fff53974` extracted with `git archive` into its own directory with its
own `CARGO_TARGET_DIR`, against this tree.

**Rule 1, call sites.** `callsites.py`, the whole watched list plus the five the brief names:

| callee | baseline | this tree |
| --- | ---: | ---: |
| `CpuBus::read_code` | 0 defs, 0 sites | **0, 0** |
| `NesBus::zone_read_cycle` | 0 defs, 36 sites | **0, 36** |
| `NesBus::halted_zone_read_cycle` | 0 defs, 1 site | **0, 1** |
| `NesBus::read_cycle` | 0 defs, 65 sites | **0, 65** |
| `NesBus::run_dma` | 2 defs, 2 sites | **2, 2** |
| `Ppu2C02::increment_vram_address` | 0 defs, 0 sites | **0, 0** |

All five identical. **No callee gains a first call site anywhere in the list**, and
`increment_vram_address` is back to 0 definitions and 0 call sites — exactly `d96d73b7`'s shape,
so dropping `#[inline]` did **not** turn it back into an out-of-line function. That was the one
risk in the revert and it is answered.

One site moves, downward: `Ppu2C02::read_nametable_inner` **15 → 14** static sites. That is the
exact mirror of MF-4d step 6's own line ("`read_nametable_inner` goes 14 → **15**: one more
*static* site, from LLVM specialising the `$2007` arm's two branches"). Taking the branch out takes
the specialisation out. It is not a first call site and it is one fewer, not one more.

**Rule 2, post-LTO bodies.** `cmp4c.py` over the pinned list. Two symbols differ and **both are
smaller**:

| symbol | baseline | this tree | delta |
| --- | ---: | ---: | ---: |
| `Ppu2C02::read` | 319 | **263** | **−56** |
| `NesBus::write_internal` | 2 805 | **2 763** | −42 |

`Ppu2C02::read`'s 263 is **MF-4d step 5's own number**, to the instruction: step 6 measured
"`Ppu2C02::read` 263 → 319, +56, inside the register-7 arm", and the 56 are back out. The 42 of
`write_internal` are the same inlined increment on the write side (step 6 measured +56 there
against MF-4d step 5's tree; the baseline here is ZR-2's, whose `write_internal` MF-5a and ZR-1/2
have since changed, so the figure is not the same one).

**Everything else on the pinned list is IDENTICAL, raw**, including every symbol the lot's
performance bar names: `Ppu2C02::run_dots` 1 932 → 1 932, `NesBus::read_cycle` 182 → 182,
`NesBus::read_internal` 176 → 176, `NesBus::run_dma` 994 → 994, `NesBus::dmc_only_dma` 191 → 191,
`NesBus::zone_read_cycle` 18 → 18, `NesBus::halted_zone_read_cycle` 43 → 43, `Cpu6502::step`
916 → 916, `Board::ppu_read` 328 → 328, `Apu2A03::run_cpu_cycles` 875 → 875.

**The bar is met in the only direction a back-out can fail in.** Nothing grew: the two symbols that
moved are strictly shorter, on the `$2007` path alone, and no other code is touched. The
instruction count is measured in step 5.

### Tests

| check | result |
| --- | --- |
| `cargo fmt --check` | clean |
| `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings` | clean, **no new `allow`** |
| `cargo test --release -p zax-nes -p zax-harness` | **746 passed, 0 failed, 8 ignored** |

746 / 0 / 8 is ZR-2's count exactly: the test was renamed and inverted, not added or removed.

**Save states**: no field added, removed or reordered; nothing new serialised; `save_state` and
`load_state` untouched. The format does not move. No `unsafe`.
