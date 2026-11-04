# Lot MF-2 — AMD-061, and the mapper fixes that touch timing or memory accesses

Role R3 (clean-side implementer), worktree `wt-mappers`, branch `mapper-review`.

Sources of truth for this lot:

- `spec/changes/AMD-061-exactitude-des-cartes.md` (all of it, §§1-8),
- `spec/changes/AMD-021-round2.md` R2-001 / R2-002 / R2-003, `AMD-054` (REQ-CORE-073 and
  REQ-CORE-075 as amended), `REQ-CORE-004`,
- `docs/review/MR-1-mappers.md`, section **"## MR-1b — sourced re-check"** and its
  authoritative findings table (the MR-1 sections before it are superseded wherever MR-1b
  disagrees),
- `docs/progress/MF-1.md` (what the previous lot did and deliberately left),
- `docs/review/MF-T-test-roms.md` (the test-ROM campaign, to confirm a fix where a ROM covers
  it; `zax-harness test-roms --filter` re-runs a subset).

Build environment for every cargo command (the C: drive is nearly full):

    TMP=<worktree>/rust/target-mf/tmp  TEMP=<worktree>/rust/target-mf/tmp
    CARGO_TARGET_DIR=<worktree>/rust/target-mf

`zax-app` cannot build in this worktree (its demo ROM is git-ignored), so clippy is scoped to
`-p zax-nes`.

## Steps

The brief numbers its work 0-9; the commits number 1-10, so commit *k* is brief step *k-1*.

- [x] 1/10 — step 0: give back what MF-1 cost on the MMC1 CHR path
- [ ] 2/10 — step 1: AMD-061 §2 and §4 — mapper 15 mode 1, mapper 101, the AMD-021 criteria
- [ ] 3/10 — step 2: AMD-061 §3 and §5 — mapper 28 keeps its state across a reset
- [ ] 4/10 — step 3: AMD-061 §6 — the MMC5 audio
- [ ] 5/10 — step 4: MR-BUS-1, MR-66-1 and the bus-conflict half of MR-34-2
- [ ] 6/10 — step 5: MR-64-3, MR-64-4 — the RAMBO-1 IRQ prescaler and its delayed assertion
- [ ] 7/10 — step 6: MR-5-2, MR-5-4 — MMC5 ExRAM writes, and in-frame detection by bus sniffing
- [ ] 8/10 — step 7: MR-30-4, MR-30-6 — UNROM 512 self-flashing and four-screen CHR-RAM
- [ ] 9/10 — step 8: AMD-061 §8 — Q-113 (SXROM battery) and Q-114 (mapper 4 submapper 4)
- [ ] 10/10 — step 9: the three unknowns MR-64-1, MR-64-5, MR-18-1, and the close of the lot

## Out of scope

New boards: MR-4-3 (the Acclaim MC-ACC, mapper 4 submapper 3) and any missing mapper.

## Findings → fix → test → hot path (for R0)

| finding / § | fix (file:line) | test | hot path touched, and why other games do not pay | games whose output changes |
| --- | --- | --- | --- | --- |
| step 0 (no finding: MF-1's cost) | `mmc5.rs:177-186` (`refresh_background_chr_set`), `:41-48` (the field), `:196-199` (`chr_address`) | the MF-1 MR-5-1 tests, unchanged | `Board::ppu_peek` / `ppu_read`: **restored instruction-for-instruction** on the MMC1 arm; `Mmc5::ppu_peek` is 3 instructions *shorter* than at `74ce9bc` | none — behaviour is identical to MF-1 |

## Step 0 — giving back what MF-1 cost

### What R0 measured

Callgrind of MF-1 (`ca68140`) against `74ce9bc`, 300 frames, two MMC1 games: Mega Man 2 core
+0.192 %, PPU +0.356 %; Zelda II core +0.088 %, PPU +0.156 %. `Board::ppu_peek` about
+2 instructions per call (6.34 M calls, +12.7 M) and `Ppu2C02::read_pattern_planes` about +32
per call (390 752 calls, +12.5 M).

### The two numbers are one regression

`read_pattern_planes` calls `peek_vram` 16 times, and each one reaches `Board::ppu_peek`:
390 752 × 16 ≈ 6.25 M, which is the 6.34 M calls of `ppu_peek`. So its "+32 per call" is
16 × (+2), the *inclusive* cost of the callee, and `read_pattern_planes`'s own body is
**unchanged** — 106 instructions at `74ce9bc` and at `ca68140`. Likewise the `run_dots`,
`catch_up_ppu` and `execute` entries R0 saw are the same +12.7 M propagating up. There is one
regression, in `Board::ppu_peek`, and Zelda II's being half of Mega Man 2's is the first clue
to why (below).

### How it was found

No callgrind on this host, so the measure was the generated code, with the static instruction
count of `Board::ppu_peek` as the bisection signal:

    RUSTFLAGS="--emit=asm" cargo rustc --release -p zax-nes --lib

(the crate's `.s` carries the real codegen for these functions even under `lto = "fat"`), then
counting instructions per symbol. Over MF-1's ten commits:

| commit | step | `Board::ppu_peek` |
| --- | --- | --- |
| `74ce9bc` | before the lot | **237** |
| `1e789ca4` … `3e3b0001` | 1-3 | 237 |
| `564cf15b` | **4, the MMC5** | **344** |
| `b00e7d69` | 5 | 344 |
| `053ee9ee` | **6, the VRC** | **349** |
| `b64de407` … `ca68140` | 7-10 | 349 |

Reverting MF-1's three MMC5 hunks one at a time put the whole jump on exactly one of them: the
new `use_b` expression in `Mmc5::chr_address` (reverting it alone takes `ca68140` from 349 back
to 243). The ExRAM open-bus hunk and the MR-5-3 scanline hunk cost nothing here.

### The cause

**`Mmc5::chr_address` had started reading the byte that holds `Board`'s discriminant.**

`Board` is a 26-variant enum whose largest variant is `Mmc5` (its 1 KiB of ExRAM), so rustc
gives it the *niche-filling* layout: the tag is not a field of its own, it is written into a
niche inside the `Mmc5` variant. The niche it picks is `Mmc5::in_frame`, the board's last
`bool` — the dispatch prologue of every `Board` method reads it and branches on
"tag ≥ 2 → one of the other 25 boards, tag ≤ 1 → this really is an `Mmc5` and the byte is the
bool":

    movzbl  1353(%rcx), %eax     ; the tag == Mmc5::in_frame
    subb    $2, %al
    movl    $5, %r10d            ; index 5 = the Mmc5 arm
    cmovael %eax, %r10d          ; tag >= 2 -> index tag-2

MF-1's MR-5-1 fix made `use_b` read `self.in_frame`:

    let use_b = self.sprite_size16 && fetch == ChrFetch::Background
        && (self.in_frame || self.last_set_was_b);

so the MMC5's CHR body now loaded the enum tag's own byte. That is correct, and it is cheap for
the MMC5 — but it made `Mmc5::ppu_peek` un-inlinable into `Board::ppu_peek`, and the resulting
reshape of that one function flipped LLVM's block placement for **all 24 other arms** from
tail-*duplication* to tail-*merging*: 45 basic blocks became 98. The MMC1 arm went from

    LBB_5:  movq 64(%rdx),%r9 / testq / jne  -> LBB_38: addq $56,%rdx / testq / je
            -> LBB_47: 6 instructions, cmpq, jb -> LBB_51: movzbl, retq

to a shape where the emptiness test and the bounds check each sat in a *shared* block reached
by an extra `jmp`. Counting the two real paths:

| MMC1 CHR fetch | `74ce9bc` | MF-1 | this step |
| --- | --- | --- | --- |
| CHR-ROM (Zelda II) | 16 | 17 | **16** |
| CHR-RAM (Mega Man 2) | 17 | 19 | **17** |

which is exactly R0's "+2 per call" on Mega Man 2, and exactly why Zelda II's figures are half
of it: a CHR-ROM board paid +1, a CHR-RAM board +2. Every board using the default
`Mapper::ppu_peek` paid, not only the MMC1.

### The fix

`Mmc5` gains one derived `bool`, `background_uses_set_b`, holding everything in MR-5-1's three
rules that is known *before* a fetch, recomputed by `Mmc5::refresh_background_chr_set` at the
five places its three inputs change (`$5120-$5127` and `$5128-$512B` in `write_control`,
`on_ppu_control`, both arms of `on_ppu_scanline`, `reset`, and `load_registers`). The fetch
then reads one bool — and not the one carrying the tag:

    let use_b = self.background_uses_set_b && fetch == ChrFetch::Background;

`sprite_size16 && Background && (in_frame || last_set_was_b)` and
`(sprite_size16 && (in_frame || last_set_was_b)) && Background` are the same predicate, so
**MF-1's behaviour is kept exactly**; the MR-5-1 tests are untouched and still pass.

The field must stay *before* `in_frame` in the struct, so that `in_frame` remains the board's
last `bool` and keeps the niche. That is written in a comment on the field itself, because a
future reader moving it would silently re-create this regression.

### What this costs, and what it gives back

| symbol | `74ce9bc` | MF-1 | this step |
| --- | --- | --- | --- |
| `Board::ppu_peek` | 237 | 349 | 240 |
| `Board::ppu_read` | 238 | 339 | 241 |
| `Mmc5::ppu_peek` | 86 | 90 | **83** |
| `Ppu2C02::read_pattern_planes` | 106 | 106 | 106 |
| `Ppu2C02::step_dot` | 1390 | 1392 | 1401 |

- **The MMC1 arm of `Board::ppu_peek` and of `Board::ppu_read` is instruction-for-instruction
  what it was at `74ce9bc`**, on both the CHR-ROM and the CHR-RAM path (table above). The
  remaining +3 in the whole symbol is static only: it is the other arms' own MF-1 fixes.
- `Mmc5::ppu_peek` is **3 instructions shorter than before MF-1**: the three bools and two ANDs
  per fetch became one bool load. The MMC5 pays instead a handful of instructions per *scanline*
  and per `$2000` / `$512x` write, which is where the inputs actually change.
- `step_dot`'s +9 is `Cartridge::on_ppu_scanline` inlined, i.e. MF-1's MR-5-3 counter plus this
  step's refresh call, and **all of it sits behind the MMC5 tag check**: a non-MMC5 cartridge
  executes `testq %r13,%r13 / je` then `movzbl <tag> / cmpb $1 / ja` and leaves — the same three
  instructions as at `74ce9bc`. The first 45 instructions of `step_dot`, which every dot runs,
  are byte-identical to the baseline apart from one branch label.
- `Board::try_read_prg` is 145 → 155 static (MF-1, not this step), but its **MMC1 arm is
  unchanged**: the same `testw` plus seven instructions, with only the branch polarity inverted.
- `size_of::<Board>()` is **1360 before and after** — the new bool lands in existing padding —
  and `size_of::<Cartridge>()` stays 1632, so no other structure's field offsets move.

Nothing was added to the `Board` dispatch, to a `Mapper` trait method, or to any per-cycle,
per-CPU-access or per-PPU-fetch path of a board this step does not touch. R0 re-measures.

### Save states

No format version bump and no new saved field: `background_uses_set_b` is derived, so
`Mmc5::save_registers` does not write it and `load_registers` recomputes it from the three bools
it already reads. A state written before this lot — or before MF-1 — loads unchanged.

### Tests

`cargo fmt --check` clean, `cargo clippy -p zax-nes --all-targets -- -D warnings` clean,
`cargo test --release -p zax-nes`: **559 passed, 0 failed, 11 ignored** — the same counts MF-1
closed with, the behaviour being identical.
