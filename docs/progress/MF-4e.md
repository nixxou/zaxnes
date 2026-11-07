# Lot MF-4e — the PPU's sprite-evaluation and shift-register pipelines, the `$2007`-during-rendering rule, and the VS. System's parked cost

Baseline and performance reference: **`11407930`** (MF-VS's end). The default build at that commit
has `board-vs-system` **parked** (`parked-boards`), which MF-VS step 7 measured at +0.083 % on
`duelito` with the feature in `default`.

The lot has four distinct jobs and they are not equally risky:

1. **`$2007` during rendering** (Q-125). The hardware rule is not in doubt; what MF-4d got wrong
   was feeding the *pipeline's* increment into the **captured row**, which is what broke AMD-021
   R2-005 and the owner's widescreen picture. Both must hold here.
2. **Sprite evaluation and the shift registers**, per dot, for the thirteen `AccuracyCoin`
   sub-tests plus `Address $2004 behavior`.
3. **Performance**, the lot's largest risk: per-dot PPU code is on every game's hot path. Above
   +0.02 % per ROM on either run it goes behind its own Cargo feature, never dropped.
4. **The VS. System**, re-measured once the per-dot code has been reorganised.

## The fourteen `AccuracyCoin` rows in scope, as the baseline reports them

`11407930`, `zax-harness test-roms --filter AccuracyCoin`: **128/147**, nineteen failures. Five are
APU/DMA and out of scope; the other fourteen are this lot's:

| row | page | code |
| --- | --- | ---: |
| `$2007 read w/ rendering` | PPU Misc. | 2 |
| `$2007 Stress Test` | PPU Misc. | 2 |
| `$2004 Stress Test` | PPU Misc. | 2 |
| `Address $2004 behavior` | PPU Misc. | 8 |
| `Arbitrary Sprite zero` | Sprite Evaluation | 2 |
| `Misaligned OAM behavior` | Sprite Evaluation | 1 |
| `OAM Corruption` | Sprite Evaluation | 2 |
| `Stale Sprite Shift Regs` | Advanced Sprite Evaluation | 3 |
| `Frozen OAM2 Increment` | Advanced Sprite Evaluation | 2 |
| `Misaligned OAM2 Address` | Advanced Sprite Evaluation | 3 |
| `Stale BG Shift Registers` | Advanced Background Evaluation | 3 |
| `BG Serial In` | Advanced Background Evaluation | 2 |
| `ALE + Read` | Advanced Background Evaluation | 2 |
| `Hybrid Addresses` | Advanced Background Evaluation | 2 |

Out of scope, named so the tally is accounted for: `APU Register Activation` 6, `Frame Counter IRQ`
13, `DMC DMA Bus Conflicts` 2, `Explicit DMA Abort` 2, `Implicit DMA Abort` 2. 14 + 5 = 19, and
147 − 19 = 128.

## Steps

* [ ] **0/7** — the plan, the fourteen rows named, and the reading.
* [ ] **1/7** — reproduce `11407930`: PR-1's thirteen and the level-2 ten on clean builds, and the
  128/147 tally.
* [ ] **2/7** — the `$2007` read during rendering: Q-125's rule **and** AMD-021 R2-005, the
  inverted unit test flipped, the owner's widescreen state whole.
* [ ] **3/7** — sprite evaluation per dot and the shift-register pipelines; each row gained named.
* [ ] **4/7** — performance: both runs, the post-LTO checks, and the feature if the tolerance says
  so. `Ppu2C02::read`'s +11 instructions while the function is open.
* [ ] **5/7** — the VS. System re-measured; into `default` if it is within tolerance on both runs.
* [ ] **6/7** — the check: fmt, clippy, the suites, the ignored suite, the campaign row by row
  against MF-VS's 331, golden-core over the commercial originals.

## Step 0 — the plan

### What MF-4d actually got wrong, and why it is not a per-dot matter

Q-125's closing section asks lot MF-4e to make the increment depend on "the exact dot the read
falls on". Reading the code the back-out left, that is **not** where the regression was. The
hardware rule and R2-005 do not collide on `v` at all; they collide on `v_row_start`.

`Ppu2C02::increment_vram_address` does two things:

    self.v = self.v.wrapping_add(step) & 0x7FFF;
    self.v_row_start = self.v;          // "a CPU write reaches the row of the line it is made in"
    self.v_row_next = self.v;

`v_row_start` is what `capture_background_row` reads, and a row is emitted **iff its fine Y is 1**.
MF-4d step 6 made the `$2007` access perform a coarse X increment *and a Y increment*; the Y half
moves fine Y, and because the result was copied straight into `v_row_start`, a split line whose
fine Y was 0 became a line whose fine Y was 1 — one spurious row per frame, which is exactly
R2-005 criterion 2's 4 986 frames out of 5 000, and exactly the half-block shift the owner saw in
the widescreen capture (the margin decoder reads the captured row's `v`).

R2-005's own text says which address changes are kept out of line L's row: "la recopie horizontale
du registre temporaire et **l'incrément vertical de fin de ligne**". The increment a `$2007` access
borrows **is the fetch pipeline's own increment** — the same logic, driven from the same signals,
which is why the nesdev page calls it "an odd way" rather than a new behaviour. So the reconciliation
is one line, not a dot table:

> **the PPUCTRL step is a CPU address change and reaches line L's row; the pipeline increment a
> `$2007` access borrows during rendering does not, exactly as the dot-256 increment does not.**

`Ppu2C02::increment_y` already honours that — it touches neither `v_row_start` nor `v_row_next` —
so the rule to write is the same rule, applied to the same two fields. R2-005 says "incrément
vertical **de fin de ligne**", and this one is not at the end of a line, so the reading is an
extension of the text rather than the text; it is filed as a question ticket with the provisional
choice marked.

### The performance shape

MF-VS §5.16 located the VS cost in `Ppu2C02::run_dots` and `Ppu2C02::background_fetch`, "per-dot
paths this lot never touches". This lot **does** touch them, so the two measurements are not
independent: step 4 must be read before step 5, and step 5's board figure is a figure against
*this* lot's own default, not against `11407930`.

`Ppu2C02::read`'s +11 instructions per register read, which MF-4d-fix gave back and Q-125 asked
this lot to win back, is **already won**: MF-5a put `#[inline(never)]` on `Ppu2C02::read_data`,
whose doc comment prices the remainder at one `movq` per register read (+0.0068 % on `ppu_vbl_nmi`).
Step 4 re-measures rather than re-derives it, and says what is left.
