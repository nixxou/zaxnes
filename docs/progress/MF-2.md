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
- [x] 2/10 — step 1: AMD-061 §2 and §4 — mapper 15 mode 1, mapper 101, the AMD-021 criteria
- [x] 3/10 — step 2: AMD-061 §3 and §5 — mapper 28 keeps its state across a reset
- [x] 4/10 — step 3: AMD-061 §6 — the MMC5 audio
- [x] 5/10 — step 4: MR-BUS-1, MR-66-1 and the bus-conflict half of MR-34-2
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
| AMD-061 §2 (mapper 15 mode 1) | `multicart.rs:139-146` (`bank \| 0x07`) | `amd_061_mapper_15_acceptance_vectors` | none: one OR inside `Mapper015::update_banks`, which runs on a register write to a mapper-15 cartridge | the mapper-15 multicarts, and only an image **larger than 128 KiB** running in mode 1 |
| AMD-061 §4 (mapper 101) | `multicart.rs:619-630` (8-bit field), `:659-672` (the range) | `amd_061_mapper_101_takes_eight_bits_from_6000_7fff` | none: both changes are in `JalecoJ87`, which only mapper 87 and 101 reach, and the range test got *simpler* (one `contains`, no branch on the board) | mapper-101 images with more than 32 KiB of CHR; mapper 87 does not move at all |
| AMD-061 §3 and §5, MR-28-3 (Q-112) | `multicart.rs:558-570` (`Action53::reset` keeps the five registers) | `amd_061_mapper_28_keeps_its_registers_across_a_reset`, `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` | none: `Mapper::reset` runs on a power cycle or a RESET, never per cycle. Five assignments **removed**, nothing added | the Action 53 multicarts, and only across a reset; no other board moves |
| AMD-061 §6.1 (no cut below period 8) | `mmc5_pulse_channel.rs:38-48` | `amd_061_a_square_below_period_eight_is_not_silenced` | none: `Mmc5PulseChannel` is a **separate type** from `PulseChannel`, so the APU's own pulses keep their cut and their per-cycle code is untouched | MMC5 titles driving a square below period 8 |
| AMD-061 §6.2 (reversed polarity, signed clip) | `mixer.rs:163-180` (`clip`), `mixer.rs:46-48` (`MIN_SCALE`) | `amd_061_the_board_output_is_subtracted_from_the_apu`, `amd_061_the_mix_of_a_game_without_expansion_audio_is_unchanged` | `clip` is on the APU's per-cycle path for **every** game: it is now one `sub` and one clamp where it was one `add` and one clamp — **the same three instructions**, the upper arm being unreachable once `M` is subtracted | every MMC5 title (the sign of its contribution); no other game, where `M` is 0 and `S` is still `A` |
| AMD-061 §6.3 (240 Hz length counter) | `mmc5_audio.rs:119-131` | `amd_061_the_length_counter_runs_at_240_hz_whatever_4017_holds` | none, and `Mmc5::on_cpu_cycle` is 6 instructions **shorter**: a field and a branch removed | MMC5 titles whose squares use the length counter |
| AMD-061 §6.3 ($5010 read mode) | `mmc5.rs:433-450` (`cpu_read_override`), `mmc5_audio.rs:77-92` | `amd_061_read_mode_takes_the_pcm_level_from_a_read_of_8000_bfff`, `amd_061_read_mode_raises_the_pcm_irq_on_a_byte_of_zero` | none for any other board: `Mmc5::rom_windows` already returns `NO_ROM_WINDOW` for all four windows, so MMC5 reads of $8000-$FFFF came through `cpu_read_override` already. `Board::cpu_read_override` is 50 instructions **smaller** | MMC5 titles that drive PCM from reads |
| AMD-061 §6.3 (PCM IRQ on the CPU line) | `mmc5.rs:55-65` (`irq_line`), `:188-192` (`refresh_irq_line`) | `amd_061_the_pcm_irq_flag_pulls_the_cpu_irq_line`, `amd_061_the_pcm_irq_needs_bit_7_of_5010_to_reach_the_cpu` | `Board::irq_asserted` is on the CPU's per-cycle interrupt check for every board: the line is **latched in a field**, so the MMC5 arm stays one `movzbl` like the other 25 and the function is 24 instructions where it was 25 | MMC5 titles enabling the PCM IRQ |
| AMD-061 §6.4 (levels) | `mixer.rs:137-142` (`MMC5_PCM_TABLE`), `mmc5_audio.rs:69-75` | `amd_061_the_squares_contribute_the_opposite_of_the_apu_pulses`, `amd_061_the_pcm_channel_follows_the_dmc_curve_over_a_full_byte` | none: the change is inside `Mmc5Audio::output`, which only an MMC5 cartridge reaches, and it is 8 instructions **shorter** (`Mmc5::expansion_audio_output` 56 → 48) | **every MMC5 title**: both the square and the PCM levels move. See Q-115 |
| MR-BUS-1, MR-66-1, MR-34-2 (bus-conflict half) | `registry.rs:113-140` (`has_bus_conflicts`), `mod.rs:417-430` (`cpu_write`), `mod.rs:178-184` (the field) | `mr_bus_1_*` x4, `mr_66_1_*`, `mr_34_2_*` | `Cartridge::cpu_write`, which only a CPU write to $4020-$FFFF reaches: **two instructions** (`cmpb` + `je`) for a board without conflicts, and the `>= $8000` half folded into the range test already there, for free | mappers 2/3/7 **submapper-2 dumps only**, 11, 30 (submapper 0 without battery, or 2), 34 BNROM, 66. No golden-core entry is on any of them |

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

## Step 1 — AMD-061 §2 and §4

### Mapper 15, mode 1 (§2)

`Mapper015::update_banks` fixed **the last 16 KiB bank of the image** at `$C000` in mode 1
(`map_prg_16k(2, -1)`). AMD-061 §2 replaces that with the last 16 KiB of the 128 KiB block `B`
selects: the register's bits 5-0 are PRG A19..A14, so forcing A14-A16 high and leaving A17-A19
to `B` is `B | 7` in 16 KiB bank numbers. That is Mesen's reading, which MR-1b's table had
already recorded as the sourced one; only the amendment's owner could change it, and §2 does.

- The other three modes, the 6-bit `B` field, the absent XOR by `h` and the mirroring bit are
  explicitly **maintained** by §2, and none of them is touched.
- MF-1's MR-15-1 CHR-RAM write protection (modes 0 and 3) is what §2 now requires in so many
  words, and it is left exactly as MF-1 wrote it.
- `req_core_100_mapper_15_has_four_prg_layouts` keeps its four vectors unchanged: its image is
  128 KiB, i.e. *one* block, so the last bank of the image and the last bank of the block are
  the same bank. Only its comment moves. The 1 MiB acceptance test is what separates the two
  readings, and it now carries §2's two new vectors (`$05` → 5 / 7 and `$0A` → 10 / 15) in place
  of R2-002's single `$05` → 5 / 63.

### Mapper 101 (§4)

Two changes, both of which MR-1b had found Mesen right about and R2-003 wrong about:

- **the range**: `$6000-$7FFF` only. The board shared `JalecoJ87` with mapper 87 and took an
  extra arm for mapper 101, `address & 0x6000 == 0x6000`, which also matched `$E000-$FFFF`.
  Both boards now use the one `(0x6000..=0x7FFF)` test, so the decode no longer branches on the
  board at all.
- **the width**: the whole byte, not `value & 3`. `BankWindows::map_chr_8k` already wraps on the
  CHR size, which is §4's "modulo la taille de la CHR", so no masking is needed. R2-003's
  `value & 3` capped the board at 32 KiB of CHR, on a board the source calls an "oversize
  extension" of the J87.

Mapper 87 keeps its own two swapped bits and its own range, so no mapper-87 dump moves.
`req_core_100_jaleco_j87_swaps_the_two_bits_of_its_chr_bank` is untouched and still passes.

`swap_bits` now means only "the J87 bit order", not "the J87 address decode"; it stays a
load-time field.

### Save states

No new field on either board and no change of meaning for an existing one: `Mapper015` still
saves `mode` and `value`, `JalecoJ87` still saves `register`. A state written before this lot
loads and plays identically, except that a mapper-15 state in mode 1 on an image larger than
128 KiB now shows the block's last bank at `$C000` — which is the fix.

### Tests

`cargo fmt --check` clean, `cargo clippy -p zax-nes --all-targets -- -D warnings` clean,
`cargo test --release -p zax-nes`: 559 passed, 0 failed, 11 ignored.

## Step 2 — AMD-061 §3 and §5, the reset

`Action53::reset` wrote AMD-021 R2-001's power-on values over the five registers. AMD-061 §3
says the opposite — "l'état de la carte (registres externe, interne, de mode et de sélection)
est **conservé**" — and §5 makes room for it by amending REQ-CORE-004's "mapper revenu dans son
état initial" to "…sauf pour une carte dont la documentation publique prescrit que son état
survit au signal RESET". The source is nesdev's *INES Mapper 028*, "The mapper state is
unchanged on reset", which is the finding MR-28-3 raised and the ticket Q-112 carried.

The method now only refreshes the windows, which it must: `Cartridge::reset`
(`cartridges/mod.rs:533`) has just put `memory.mirroring` back to the header's value, and
`Action53::update_banks` is what re-asserts the mode's own mirroring over it.

**The power-on state does not move.** `Mapper::reset` serves three callers — `Cartridge::create`
at load, `NesCore::reset` (a power cycle, DEC-02-Q3) and `NesCore::soft_reset` (AMD-044
REQ-CORE-122) — and R2-001's values are written by `Action53::new`, which runs once per load,
immediately before the first `reset`. So a freshly loaded mapper-28 cartridge still powers on
at outer 63 / inner 15 / mode 0, i.e. with the last 16 KiB bank of the image at `$C000-$FFFF`,
which is the only thing §3 still *requires* of the power-on state. What changes is the second
and later reset of a running cartridge, which is the fix.

**Every other board is untouched**, which §5 requires in as many words (and it names the MMC1
as explicitly *not* exempt). `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state`
sweeps `NesCore::SUPPORTED_MAPPERS`, writes seven registers across `$5000-$E000` on each board,
resets, and asserts the PRG windows, the CHR windows and the mirroring are all back where they
powered on. Mapper 28 is the single `continue`. The older, narrower
`req_core_004_a_reset_puts_the_mapper_back_to_its_power_on_banks` (mappers 1, 4, 16, 64, with
the MMC1 and MMC3 helpers and the IRQ check) is kept as it was.

### Save states

No field added, removed or re-interpreted: `Action53` still saves and loads the same five
registers in the same order. A state written before this lot loads unchanged.

### Tests

`cargo fmt --check` clean, `cargo clippy -p zax-nes --all-targets -- -D warnings` clean,
`cargo test --release -p zax-nes`: **561 passed** (the two new ones), 0 failed, 11 ignored.

## Step 3 — AMD-061 §6, the MMC5 audio

Six changes, every one of them a divergence the amendment withdraws. The board's own files are
`apu/mmc5_audio.rs` and `apu/mmc5_pulse_channel.rs`; `apu/mixer.rs` and
`cartridges/mappers/mmc5.rs` are shared, and the paragraphs below say what that cost.

### §6.1 — no cut below period 8

"Frequency values less than 8 do not silence the MMC5 pulse channels; they can output ultrasonic
frequencies" (nesdev, "MMC5 audio"). The test drops out of `Mmc5PulseChannel::output`, which is a
**separate type** from `PulseChannel`: the APU's own pulses keep their cut, and nothing on their
per-cycle path moves. That separation already existed for the sweep unit, and this is now its
second reason.

### §6.2 — reversed polarity and the signed clip

"The polarity of all MMC5 channels is reversed compared to the APU", so `S = A − M` with `M` a
positive quantity, clipped to `-32 768..=32 767`.

**This is the one change on a path every game runs, and it costs nothing.** `mixer::clip` was
`min(A + M, 32767)`: an `add`, a `cmp` and a `cmov`. It is now `max(A − M, -32768)`: a `sub`, a
`cmp` and a `cmov`. The upper arm is *gone, not skipped*: `A` is two table entries that total at
most 32 767 and `M` is non-negative, so `A − M ≤ 32 767` for every pair the mixer can produce,
and emitting the test would be dead code. A game without expansion audio has `M = 0`, so `S` is
`A` exactly as before, and `amd_061_the_mix_of_a_game_without_expansion_audio_is_unchanged` pins
it.

### §6.3 — the three AMD-054 divergences on the MMC5, withdrawn

- **The length counters go to 240 Hz**, the envelope's own cadence ("Length counter operates
  twice as fast as the APU length counter"; both "fixed to a 240Hz update rate"). The
  `length_turn` field that used to select every other clock is **removed**, so
  `Mmc5::on_cpu_cycle` is six instructions shorter than before.
- **The read mode of $5010 is emulated.** A CPU read of $8000-$BFFF feeds the byte to the PCM
  level, and a byte of 0 raises the PCM IRQ flag instead. The hook costs no other board anything,
  because `Mmc5::rom_windows` already answers `NO_ROM_WINDOW` for all four windows (the board
  decodes its own PRG), so every MMC5 read of $8000-$FFFF already came through
  `Mapper::cpu_read_override` rather than through `Cartridge::read_rom_window`'s fast path. Out
  of read mode the guard is one bool test, on the MMC5's own path.
- **The PCM IRQ drives the CPU's /IRQ line**, `Cart /IRQ line = NOT(irqTrip AND irqEnable)`, and
  the read of $5010 that acknowledges the flag releases it.

### §6.4 — the levels, and why Mesen was not needed

`M = r(K · P(q1 + q2)) + r(K · T(pcm))`, on the APU's own two curves and its own `K`. The 8 000
and the 31 of AMD-054 are gone, with `MMC5_FULL_SCALE`, `MMC5_PCM_STEP`, `MMC5_NORMALISATION` and
`MMC5_PULSE_TABLE`.

§6.4 says to take Mesen 2's PCM ratio *if the public documentation fixes none*. **It fixes one**,
so Mesen's is not used:

> the PCM channel is equivalent in volume to the APU with equivalent input, and inverted, but
> using the extra high bit of input it can become twice as loud

"The APU with equivalent input" is the DMC, whose level enters `T`; "the extra high bit" is the
eighth bit of $5011, which the seven-bit DMC has not got. So the PCM channel reads `T` over a
whole byte — `MMC5_PCM_TABLE`, whose first 203 entries are `TND_TABLE` itself and whose 53 others
are that eighth bit. No new constant, no fitted ratio.

**§6.4 and §6.5 contradict each other, and Q-115 carries it.** §6.4's parenthesis says `P` is
"appliquée voie par voie", which gives 4 890 + 4 890 = 9 780 for two squares at volume 15; §6.5's
second criterion asks for the 8 462 that two APU pulses give, which is `P` on the summed index.
The criteria are what the amendment declares normative, and one shared index is also what
"equivalent in volume to the corresponding APU channels" means for a *pair* of APU pulses, so the
fix evaluates `P` once on `q1 + q2`. The ticket also records that **Mesen satisfies neither**
criterion — its board mix is linear, `-(sq1 + sq2 + pcm)` then `× 43`, calibrated so that *two*
squares match two APU pulses and leaving *one* square 13 % low — so it could not have been the
oracle here anyway; and it asks what a $5011 write should do in read mode, where the amendment
leaves AMD-054's clause unamended and Mesen ignores the write.

### §6.5 — the criteria

`mmc5_audio_tests.rs` is rewritten to them. AMD-054's `req_core_075_the_board_produces_the_documented_integers`
(4 623 / 8 000 / 3 968) and `req_core_075_the_board_table_is_the_pulse_function_scaled_to_eight_thousand`
are gone, and in their place: the squares' two equalities with the APU's pulses, the PCM channel's
identity with the DMC's curve, a square of period 4 (and 0, 1 and 7) still audible with the APU's
own pulse still muted beside it, the subtraction and its floor, the 240 Hz length counter under
each of `$4017` = $00 / $80 / $C0, and the two IRQ paths. The envelope cadence test (15 to 0 in
15 × 1/240 s ± 5 %) is maintained as §6.5 requires, unchanged.

### The second instance of step 0's trap, caught by the same measure

`Mmc5::irq_asserted` was first written as `(irq_pending && irq_enabled) || audio.irq_asserted()`.
That reads four bytes and branches, where the other twenty-five arms of `Board::irq_asserted` are
a single `movzbl` each, and it grew that function from 25 to 32 instructions — enough to change
the inliner's mind inside `Cpu6502::step`, which went from 244 instructions to 1 331 as `execute`,
`resolve`, `pull`, `push` and `indexed` were folded into it and `end_cycle` was pushed out. That
is on **every game's** per-instruction path.

The board therefore **latches its /IRQ line** in one `bool`, `Mmc5::irq_line`, refreshed by
`refresh_irq_line` wherever one of its four inputs moves: the $5204 write, the $5204 read, the
$5010/$5015 read, the read-mode feed, a sound-register write, the scanline IRQ, `reset` and
`load_registers`. That is also closer to the hardware, which has one line and not a disjunction.
With it, no symbol appears or disappears against step 0's build, `Board::irq_asserted` is **24**
instructions where it was 25, and every CPU function is slightly *smaller* (`step` 244 → 240,
`execute` 2 720 → 2 674, `resolve` 1 279 → 1 251, `branch` 345 → 339).

### What else moved, and what did not

| symbol | step 0 | step 3 | why |
| --- | --- | --- | --- |
| `Board::ppu_peek` / `ppu_read` | 240 / 241 | 240 / 241 | untouched |
| `Board::try_read_prg` | 155 | 155 | untouched |
| `Board::irq_asserted` | 25 | **24** | the MMC5 arm is one load like the rest |
| `Board::cpu_read_override` | 125 | **75** | the MMC5 arm went out of line |
| `Board::on_ppu_scanline` | 36 | 44 | the MMC5 arm only; the non-MMC5 path is the same `movzbl` / `cmpb` / `ja` |
| `Ppu2C02::step_dot` | 1401 | 1409 | the inlined MMC5 scanline arm, behind that same tag check |
| `Mmc5::on_cpu_cycle` | 114 | **108** | `length_turn` gone |
| `Mmc5::expansion_audio_output` | 56 | **48** | one table read instead of a table read and a multiply |

### Save states

**No format version bump.** `Mmc5Audio` lost `length_turn` and `Mmc5` gained `irq_line`, and
neither moves the block:

- the byte `length_turn` occupied is still written and still read, as a reserved `false`, so the
  layout is identical in both directions — a state written now still loads on a pre-MF-2 build,
  which would read it as a `length_turn` that no longer matters;
- `irq_line` and `background_uses_set_b` are **derived**, so `save_registers` does not write them
  and `load_registers` recomputes both from registers the block already carries.

`req_core_090_the_board_sound_survives_a_savestate_round_trip` and
`req_core_091_a_block_written_before_the_board_sound_still_loads` both pass unchanged.

### Tests

`cargo fmt --check` clean, `cargo clippy -p zax-nes --all-targets -- -D warnings` clean,
`cargo test --release -p zax-nes`: **566 passed**, 0 failed, 11 ignored.
`cargo check --workspace --exclude zax-app --all-targets` clean, so removing the four MMC5 mixer
constants breaks no other crate.

## Step 4 — MR-BUS-1, MR-66-1 and the bus-conflict half of MR-34-2

A board with no logic to disable the ROM's output enable during a write leaves the ROM driving
the data bus, so the register latches "the bitwise AND of the written value and the value in ROM"
(nesdev, *NES 2.0 submappers*). We handed `write_register` the value unchanged on every board.

### Which boards, and on what evidence

`registry::has_bus_conflicts(mapper, header)`, decided once in `Cartridge::create`:

| mapper | conflicts when | source |
| --- | --- | --- |
| 2, 3, 7 | NES 2.0 **submapper 2** only | *NES 2.0 submappers*: submapper 2 "Bus conflicts occur, producing the bitwise AND of the written value and the value in ROM"; submapper 1 "Bus conflicts do not occur" |
| 11 | always | *INES Mapper 011*: "All commercially-released games are subject to bus conflicts (submapper 2)", with the *Color Dreams* board page giving the conflict-prevention logic to the *Free Fall* prototype alone |
| 30 | submapper 2, **or** submapper 0 without the battery bit | *UNROM 512*: submapper 2 "Bus conflicts present"; submapper 0 "The Battery bit must be specified if the game relies on bus conflicts not being present" |
| 34 | **BNROM** only, always | *INES Mapper 034*: BNROM's register takes "the value being written bitwise-AND'd with the PRG-ROM content"; the NINA boards do not |
| 66 | always | the `GxROM` board table (MR-66-1), which Mesen does not implement |

MR-1b asked that `mr_bus_1_uxrom_without_a_submapper_has_no_conflict` keep passing as written, and
it does: the submapper is the gate on mappers 2, 3 and 7, and most dumps of those boards carry no
submapper at all. Mapper 30's gate is the one that is not the submapper alone, and it is the one
MR-1b singled out as "unusual".

### Where it costs, and how much

One block in `Cartridge::cpu_write`, after `intercept_cpu_write` and the PRG-RAM window, before
`write_register`. For a board without conflicts the whole of it is **two instructions**:

    .LBB361_5:
            cmpb    $0, 1632(%rsi)     ; Cartridge::bus_conflicts
            je      .LBB361_9          ; -> write_register, value untouched

and the `address >= 0x8000` half of the condition costs **nothing at all**: LLVM folded it into
the `$6000-$7FFF` range test that was already there, so a write below `$6000` never even reaches
the bool. The bool itself sits at offset 1632, in the same cache line as `rom_windows` at 1600,
which the very next statement reads. On a board that *does* have conflicts the cost is a window
lookup, a bounds check and one `andb`.

- `Cartridge::cpu_write` 90 → 112 instructions static; `Cartridge::create` 3 284 → 3 326, at
  load. **Nothing else in the crate changed size, and no symbol appeared or disappeared** — no
  inlining cascade of the kind steps 0 and 3 had to undo.
- `size_of::<Cartridge>()` is unchanged: the new `bool` went into the padding after
  `rom_windows`, and `board` is still at offset 240, so `memory` is still at offset 0 and every
  other field offset held.
- No `Mapper` method is consulted per write, the `Board` dispatch is untouched, and nothing
  reaches a per-cycle, per-PPU-fetch or CPU-read path. A CPU write to the cartridge is the rarest
  bus event there is.

### Tests

The two `#[ignore]`d MR-BUS-1 tests are **un-ignored** and pass, and four more are added for the
boards MR-1b's re-check brought in:

| test | what it pins |
| --- | --- |
| `mr_bus_1_color_dreams_write_is_anded_with_the_rom_byte` | mapper 11, unconditional |
| `mr_bus_1_uxrom_submapper_2_write_is_anded_with_the_rom_byte` | mapper 2, submapper 2 |
| `mr_bus_1_uxrom_without_a_submapper_has_no_conflict` | the guard against over-reach, unchanged |
| `mr_66_1_gxrom_write_is_anded_with_the_rom_byte` **new** | mapper 66, and that the AND lets bits through from a non-zero bank |
| `mr_34_2_bnrom_write_is_anded_with_the_rom_byte_and_nina_is_not` **new** | BNROM has them, NINA-001 does not |
| `mr_bus_1_unrom512_conflicts_follow_the_submapper_and_the_battery_bit` **new** | all three mapper-30 cases |
| `mr_bus_1_a_board_without_conflicts_latches_the_written_value` **new** | mappers 3 and 7's submapper gate, and that an MMC1 is untouched |

**MR-1's CHR assertion in the Color Dreams test was wrong and is corrected.** It ended with
`assert_eq!([0; 8], core.chr_windows())`, which no CHR bank can produce: `chr_windows` reports the
eight 1 KiB banks a window shows, so 8 KiB bank 0 is `0..=7` and not eight zeros. The finding's
behaviour — the latch stays on CHR bank 0 instead of moving to the bank 1 that `$13` asks for — is
what the literal says now. Everything else in both tests is as MR-1 wrote it. This is the same
kind of correction MF-1 had to make to `mr_34_1_bnrom_has_no_register_at_7ffd`.

### Five existing tests wrote through a window the conflict now pulls to zero

The fixture fills 8 KiB PRG bank *n* with the byte *n*, so bank 0 is zero-filled and a register
write through `$8000` on a conflicting board latches nothing. Five tests of the *register decode*
— not of the conflict — were doing exactly that:

| test | how it was fixed |
| --- | --- |
| `req_core_100_color_dreams_splits_its_register_between_prg_and_chr` | `prg_bank_all_ones(0)` |
| `req_core_100_gxrom_splits_its_register_between_prg_and_chr` | `prg_bank_all_ones(0)` |
| `req_core_100_bnrom_switches_32_kib_from_8000` | `prg_bank_all_ones(0)` |
| `req_core_100_unrom512_switches_prg_chr_ram_and_mirroring` | `.battery()`, which is how a submapper-0 UNROM 512 declares it has no conflicts |
| `mr_30_1_unrom512_byte6_08_is_switchable_one_screen` | `.battery()`, same reason |

`MapperRom::prg_bank_all_ones(bank)` is the new fixture knob: it fills that one 8 KiB bank with
`$FF` so a conflicting write through it latches what the CPU wrote. It is needed because the
numbered banks only ever carry bits 0-3, so a register field above bit 3 — mapper 11's CHR field,
mapper 66's PRG field — could never survive a conflicting write through one. The three mapper-30
tests that use submappers 1, 3 and 4 need nothing: the sources give all three "No bus conflicts".

### Save states

Nothing added to any board and nothing to the block: `bus_conflicts` is derived from the ROM
header at load, like `CartridgeMemory::battery_offset`, so it is not saved and a state written
before this lot loads unchanged.

### What R0 should expect to move

The golden-core campaign has no entry on mapper 11, 30, 34 or 66, and the mapper-2, -3 and -7
entries are iNES dumps with no submapper, which `has_bus_conflicts` answers `false` for. So **no
golden-core entry should move**. A commercial Color Dreams or GxROM title would now AND its
register writes, which is the fix.

### Tests run

`cargo fmt --check` clean, `cargo clippy -p zax-nes --all-targets -- -D warnings` clean,
`cargo test --release -p zax-nes`: **572 passed**, 0 failed, **9 ignored** — two fewer than at
step 3, the two MR-BUS-1 tests having come off `#[ignore]`.
