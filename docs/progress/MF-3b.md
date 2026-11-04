# Lot MF-3b — giving back what MF-3 cost on boards it did not touch

R0 measured MF-3's end (`42b541ef`) against MF-2's end (`7cbd9c52`) with callgrind, 300 frames,
clean builds from `git archive` into fresh directories, on two MMC1 games that run no line of any
MF-3 board. Noise: zero instructions.

| game | stage | MF-2 | MF-3 | delta |
| --- | ---: | ---: | ---: | ---: |
| Mega Man 2 | core | 7 748 902 954 | 7 769 371 539 | +0.26 % |
| Mega Man 2 | PPU | 4 173 842 423 | 4 186 618 218 | +0.31 % |
| Zelda II | core | 7 869 887 987 | 7 909 541 238 | +0.50 % |
| Zelda II | PPU | 4 351 862 083 | 4 383 870 136 | +0.74 % |

The lot's job: an MMC1 game executes the same number of instructions as at `7cbd9c52`, core and
PPU, with every MF-3 behaviour and test kept.

## Steps

0. The diagnosis: the instrument, the cause of the core delta and of the PPU delta.
1. `Cpu6502::end_cycle` inlined again — the whole core delta.
2. `Board::on_ppu_address_bus`: the no-op board's fast path restored.
3. `Ppu2C02::read_nametable_as` folded back into `read_nametable_inner`.
4. The residual PPU delta: `Board::ppu_read`, `catch_up_ppu`.
5. The suites, and the rule for future lots that add a board.

## Step 0 — the diagnosis

### The instrument, and why MF-3's was not enough

MF-3 measured the **static instruction count per symbol** of the crate's assembly
(the method MF-2 step 0 established, there being no callgrind on this host):

    TMP=<worktree>/rust/target-mf/tmp TEMP=$TMP CARGO_TARGET_DIR=<worktree>/rust/target-mf
    touch rust/crates/zax-nes/src/lib.rs
    cargo rustc --release -p zax-nes --lib -- --emit=asm

That measure is **blind to the one thing that went wrong**. A function can keep its size to the
byte, keep its body character for character, and still cost the program a hundred and sixty
million instructions more — by ceasing to be *inlined* and becoming a `callq` on a hot path. The
symbol is then not bigger; it is merely *reached* differently, and MF-3's per-step tables, which
compared sizes, reported "byte-identical" for exactly that case.

This lot therefore measures **two** things from the same dump: the instruction count per symbol,
and the **number of call sites per callee** —

    grep -oE "call[q]?\s+\S*<symbol>\S*" <dump>.s | wc -l

A callee going from 0 call sites to N is the signature of an outlining regression, and it is
invisible to a size diff. Both baselines were built per R0's prescription — `git archive` of the
commit into a brand-new directory with its own `CARGO_TARGET_DIR`, never a reused target folder
(the artefact that gave R0 a false regression on MF-2).

Baseline `7cbd9c52` checked genuinely pre-MF-3: zero occurrences of `Fme7`, `McAcc` or
`Mapper078` in its dump, against 67 at `42b541ef`.

### The core delta: `Cpu6502::end_cycle` stopped being inlined

The call-site diff over the whole crate, `7cbd9c52` → `42b541ef`, has exactly one entry on any hot
path:

| callee | call sites before | after |
| --- | ---: | ---: |
| `Cpu6502::end_cycle::<NesBus>` | **0** | **75** |

Everything else that moved is cold: `RawVec::grow_one`, `StateReader::read_array`,
`Cartridge::refresh_nametable_pages` (1 → 11, a load-time and CPU-write path), and the
`update_banks` of the boards MF-3 added. No hot callee gained or lost a call site.

So at `7cbd9c52` `end_cycle` **had no symbol at all** — it was inlined into all 75 of its call
sites, spread over `execute` (2 674 instructions), `resolve` (1 251), `branch` (339), `step`
(240), `indexed` (105), `push` (83) and `pull` (78). At `42b541ef` it is a real function of 76
instructions, called once per CPU cycle from 75 places. That is R0's
`Cpu6502::end_cycle` 0 → **159.9 M exclusive**, and the moves R0 saw around it — `execute`
−168.4 M, `branch` −30.5 M, `resolve` −21.3 M, `indexed` −6.9 M all folding into `step` +87.1 M —
are the same event seen from the callers' side.

The arithmetic closes exactly. Mega Man 2's core delta is

    7 769 371 539 − 7 748 902 954 = +20 468 585

and R0's own summary of the CPU side is "net, about +20 M". **The entire core regression is this
one outlining**, and nothing else in the CPU is implicated.

### What pushed it over the threshold

`end_cycle` samples /NMI and /IRQ:

    self.irq_sample = bus.irq_asserted() && (self.p & Self::FLAG_I) == 0;

and `NesBus::irq_asserted` is
`self.apu.irq_asserted() || self.cartridge.as_ref().is_some_and(Cartridge::irq_asserted)`, which
reaches `Board::irq_asserted` — the 29-arm `dispatch!` over the `Board` enum. LLVM inlines that
whole dispatch into `end_cycle`, jump table and all. Its generated code at `42b541ef`:

    movzbl  10939(%rdx), %eax      ; the niche tag, inside the Mmc5 variant
    subb    $2, %al
    movzbl  %al, %eax
    movl    $5, %r8d
    cmovael %eax, %r8d
    movzbl  %r8b, %eax
    leaq    .LJTI7_0(%rip), %r8    ; 29 entries
    movslq  (%r8,%rax,4), %rax
    addq    %r8, %rax
    jmpq    *%rax

The shape is **not new** — the baseline's inlined copy inside `step` has the same prologue and the
same jump table (`.LJTI5_1`), reached through the same niche tag. Nothing about the dispatch got
slower per se. What changed is only its **size**: two more arms and one more distinct arm body
(the FME-7's M2 counter and the MC-ACC each need their own block, where most boards share the
"always false" block) grew the callee past the point where the inliner would still paste it into
75 call sites, so it outlined it once and called it instead.

This is why the cause was hard for MF-3 to see: no board's arm got slower, no dispatch got bigger
on any executed path, and every per-step size table was telling the truth. The cost is the call
itself — `callq`/`retq` plus the argument and register traffic around it — paid on **every CPU
cycle** of every game, on a board that can never assert an IRQ at all.

## Step 1 — `Cpu6502::end_cycle` inlined again

### The fix

One attribute, in `cpu/cpu6502.rs:308-320`:

    #[allow(clippy::inline_always)] // Measured: a `callq` here costs 159.9 M instructions.
    #[inline(always)]
    fn end_cycle<B: CpuBus + ?Sized>(&mut self, bus: &B) {

with the reasoning above written on the function itself, because the next lot to add a board would
otherwise re-create the regression and have no way of knowing it had.

`#[inline(always)]` and not a restructuring of the IRQ question, deliberately. The alternative
considered was to latch at load whether the board can assert an IRQ at all — only twelve of the
thirty-six supported mapper numbers have a board IRQ (4, 5, 16, 18, 21, 22, 23, 25, 27, 64, 69,
118) — and put the dispatch behind that `bool`. It would take an MMC1 from eleven instructions a
CPU cycle to two, i.e. **below** the baseline. It was not done, for one reason: the dispatch would
have to leave `end_cycle` for the `bool` to shrink it, which puts a `callq` per CPU cycle on the
boards that *do* have an IRQ — mapper 4 among them, which is in the golden-core set. Trading an
MMC1 win for an MMC3 loss is not what this lot was asked for; the target is parity with
`7cbd9c52` for every board, and `#[inline(always)]` gives exactly that and nothing else.

### What it gives back, in the generated code

`Cpu6502::end_cycle` **has no symbol again**, and the whole-crate call-site diff against
`7cbd9c52` now has **no hot-path entry at all**: what is left is `RawVec::grow_one`,
`StateReader::read_array`, `do_reserve_and_handle`, the `panic_const_*` paths,
`Cartridge::refresh_nametable_pages` and the new boards' `update_banks` — load-time, savestate and
CPU-write code, none of it on a per-cycle or per-fetch path.

And the inlined copy is the baseline's, instruction for instruction. The executed path inside
`step`, `7cbd9c52` on the left and this step on the right, differs only in basic-block labels:

    movzbl  8(%rsi), %ecx          movzbl  8(%rsi), %ecx
    movzbl  10(%rsi), %eax         movzbl  10(%rsi), %eax
    movb    %al, 12(%rsi)          movb    %al, 12(%rsi)
    movzbl  9304(%rdi), %eax       movzbl  9304(%rdi), %eax
    cmpb    $1, %al                cmpb    $1, %al
    jne     .LBB5_24               jne     .LBB6_11
    ...                            ...
    cmpb    $0, 12000(%rdi)        cmpb    $0, 12000(%rdi)
    jne     .LBB5_36               jne     .LBB6_96
    testb   $1, 11934(%rdi)        testb   $1, 11934(%rdi)
    jne     .LBB5_36               jne     .LBB6_96
    cmpq    $-1, 9344(%rdi)        cmpq    $-1, 9344(%rdi)
    je      .LBB5_30               je      .LBB6_191
    movzbl  10939(%rdi), %eax      movzbl  10939(%rdi), %eax
    subb    $2, %al                subb    $2, %al

— same registers, same field displacements (the /NMI sample at 9304, the APU's two IRQ bytes at
12000 and 11934, the cartridge `Option` at 9344, the niche tag at 10939), same niche prologue.

Static counts move the way inlining 76 instructions into 75 places predicts, and the per-caller
`+3` is the two arms MF-3 appended to the jump table:

| function | `7cbd9c52` | MF-3 | this step |
| --- | ---: | ---: | ---: |
| `Cpu6502::end_cycle` | *no symbol* | 76, **75 call sites** | *no symbol* |
| `Cpu6502::resolve` | 1 251 | 584 | 1 345 |
| `Cpu6502::branch` | 339 | 184 | 364 |
| `Cpu6502::indexed` | 105 | *folded* | 108 |
| `Cpu6502::push` | 83 | *folded* | 86 |
| `Cpu6502::pull` | 78 | *folded* | 81 |
| `Cpu6502::step` | 240 | 1 331 | 867 |
| `Cpu6502::execute` | 2 674 | *folded into `step`* | *folded into `step`* |

`execute` stays folded into `step`, which is MF-3's arrangement and not this lot's business: MF-3
step 9 already measured that forcing it back out made the wall clock *worse* (1.078 ms/frame
against 1.042), and what callgrind counts is executed instructions, which the table above shows
are the baseline's.

**Expected effect on R0's figures: the whole core delta.** Mega Man 2 core +0.26 % and Zelda II
core +0.50 % should both go to zero, the +20.5 M being this and only this.

### Tests

`cargo fmt --check` clean; `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean (the `inline_always` lint is denied workspace-wide, hence the narrow `allow` with its
measured reason — the first and only one in the crate);
`cargo test --release -p zax-nes -p zax-harness`: **679 passed, 0 failed, 7 ignored**. No
behaviour changed: an attribute cannot change what the function computes.

## Step 2 — `Board::on_ppu_address_bus`: the no-op board's fast path restored

### The cause, in the generated code

R0's callgrind: `Board::on_ppu_address_bus` **+4.84 M**, "about one instruction more per call",
with `background_fetch` −5.7 M and `run_dots` −3.4 M moving with it. The function is 106
instructions at `7cbd9c52` and 155 at `42b541ef`, and its **prologue** is where the cost is.

`7cbd9c52` — the board-does-nothing answer is a bitset the compiler folded into the dispatch:

    movzbl  1355(%rcx), %eax     ; the niche tag
    subb    $2, %al
    movzbl  %al, %eax
    movl    $5, %edx
    cmovael %eax, %edx
    movzbl  %dl, %eax
    movl    $66584559, %edx      ; the boards with nothing to do
    btl     %eax, %edx
    jae     .LBB498_1            ; not in the set -> go and do the work
    retq                         ; in the set -> leave

`42b541ef` — a jump table:

    movzbl  1355(%rcx), %eax
    subb    $2, %al
    movzbl  %al, %eax
    movl    $5, %edx
    cmovael %eax, %edx
    movzbl  %dl, %eax
    leaq    .LJTI502_0(%rip), %rdx
    movslq  (%rdx,%rax,4), %rax
    addq    %rdx, %rax
    jmpq    *%rax                ; -> .LBB502_45: retq

MF-3's MC-ACC gave the dispatch a third distinct arm body (the ÷8 prescaler), and LLVM stopped
folding the question into a bitset. A board with nothing to do went from ten instructions and a
*predictable* not-taken branch to eleven and an **indirect jump**, on every PPU fetch.

**MF-3 step 8 measured the MMC3's own path and reported this as a win** — "the dispatch that
reaches it got one instruction shorter (a jump table replaced a bitset-plus-compare chain)" —
which is true for the MMC3, and exactly backwards for the twenty-six boards that only ever
return. That is the lesson of this step: on a dispatch, the arm that matters is the one most
games take, and for `on_ppu_address_bus` that arm is *the empty one*.

### The fix

Three of the twenty-nine `Board` variants override `Mapper::on_ppu_address_bus` — `Mmc3`
(mappers 4 and 118), `McAcc` (mapper 4 submapper 3) and `Rambo1` (mapper 64). The question is
therefore answerable at load time, which is what the sibling fields of `Cartridge` already do
(`nametable_pages_vary`, `bus_conflicts`). `cartridges/mod.rs`:

* a `watches_ppu_address_bus: bool` field (`:203-221`), with the story above on it, latched in
  `Cartridge::create` as `matches!(mapper, 4 | 64 | 118)` (`:274-277`);
* `Cartridge::on_ppu_address_bus` (`:566-578`) tests it and returns **before** dispatching.

Derived at load, so no savestate byte moves.

### What it gives back

The latch is inlined into all twelve call sites, and the no-op board's whole cost per fetch is
now one compare and one not-taken branch:

    cmpb    $1, 1650(%r8)
    jne     .LBB145_4          ; not a watcher -> the fetch carries on
    ...
    callq   Board::on_ppu_address_bus

| per PPU fetch, board with nothing to do | `7cbd9c52` | MF-3 | this step |
| --- | ---: | ---: | ---: |
| instructions before returning | 10 + `callq`/`retq` | 11 + `callq`/`retq` + indirect jump | **2, no call** |

So this is not parity but well under it: the twenty-six boards that do nothing no longer pay the
call, the niche prologue or the jump table at all. R0's +4.84 M should come back as a **negative**
number.

**What it costs, stated plainly:** an MMC3, an MC-ACC or a RAMBO-1 now runs the latch test before
the call it already made — about **+2 instructions per PPU fetch against `7cbd9c52`** for mappers
4, 64 and 118, the jump-table prologue MF-3 introduced being the other half of it. Removing that
too would mean matching the three variants directly in `Cartridge` and bypassing the `Board`
dispatch, which duplicates in a second place the knowledge of which boards care; it was not worth
it for two instructions. Neither of R0's callgrind games is on those mappers, and golden-core's
mapper 4 is a correctness check.

### The latch cannot go stale

A board added later with an A12 counter, whose mapper number nobody adds to the `matches!`, would
have its counter **silently never clocked** — a real bug, not a slow path. So the invariant is
tested rather than trusted:
`cartridges::tests::mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`
(`cartridges/mod.rs:684-760`, an in-crate `#[cfg(test)]` module, which is where it has to live to
reach `Cartridge`'s private fields and the `Board` dispatch).

For every entry of `SUPPORTED_MAPPERS`, and for submapper 0 and submapper 3 of each, it builds the
cartridge, drives twenty A12 rises straight at `Board::on_ppu_address_bus`, and requires
`watches_ppu_address_bus` to be `true` wherever that moved the board (`Board` derives
`PartialEq`, so "moved" is the whole board state, not a guess about which field). It checks the
invariant itself, so it covers a board that does not exist yet.

It also asserts that the boards which moved are exactly `[4, 4, 64, 64, 118, 118]` — both
submappers of each of the three — which is what stops the test passing vacuously if the sweep ever
stops reaching the hook. That assertion caught its own first draft: the expected count was 4, and
submapper 3 of 64 and of 118 is not a board of its own, so six cartridges move, not four.

### Tests

`cargo fmt --check` clean, `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean, `cargo test --release -p zax-nes -p zax-harness`: **680 passed, 0 failed, 7 ignored** — the
679 of step 1 plus the new latch test. The MMC3, MC-ACC, TxSROM and RAMBO-1 IRQ tests
(`mapper_irq_tests.rs`, `mr_4_3_*`) pass unchanged, which is the behavioural check that the latch
lets the boards that need the hook still receive it.

## Step 3 — `Ppu2C02::read_nametable_as` folded back into `read_nametable_inner`

### The instrument had to change first

R0's callgrind: `Ppu2C02::read_nametable_as` 0 → **3.18 M** exclusive, `read_nametable_inner`
+0.84 M. The same signature as step 1 — a function that stopped being inlined — but this one
**does not show up in the crate's own assembly at all**: `read_nametable_inner` tail-calls
`read_nametable_as` four times at `7cbd9c52` and four times at `42b541ef`, byte for byte.

The reason is in `rust/Cargo.toml`:

    [profile.release]
    lto = "fat"
    codegen-units = 1

`cargo rustc --release -p zax-nes --lib -- --emit=asm` emits **pre-LTO** codegen for the crate
alone. MF-2 step 0 recorded the belief that "the crate's `.s` carries the real codegen for these
functions even under `lto = "fat"`", and for the `Board` dispatches and the CPU it does. For
`read_nametable_as` it does not: the inlining happens during the final crate's LTO, after that
dump is written. **Any claim about this function made from the lib dump is unfalsifiable**, which
is how MF-3 reported it "byte-identical" at every one of its nine steps while R0's callgrind saw
it go out of line.

So this lot added a second instrument — the **post-LTO** assembly of a real binary:

    cargo rustc --release -p zax-harness --bin zax-harness -- --emit=asm
    # 39 MB, the whole program after LTO; then count defs and call sites per symbol

At `42b541ef` that dump says, for the functions this lot touches:

| symbol | defs | call sites |
| --- | ---: | ---: |
| `Cpu6502::end_cycle` | 0 | 0 — step 1's fix confirmed at the level that counts |
| `Ppu2C02::read_nametable_as` | **1** | **4** — out of line, R0's 3.18 M |
| `Ppu2C02::read_nametable_inner` | 1 | 15 |
| `Board::on_ppu_address_bus` | 1 | 15 |

The baseline's post-LTO dump was **not** built: it needs the whole dependency tree in a fresh
target directory (~3.4 GB) and G: has 6.4 GB free, which is the risk MF-3 step 9 already declined
to take. It is not needed — callgrind's "0 exclusive instructions" *is* the statement that the
function was never entered at `7cbd9c52`, and that is better evidence than a dump.

### The fix

`#[inline(always)]` on `Ppu2C02::read_nametable_as` (`ppu/mod.rs:1245-1270`), with the reasoning
on the function.

This is the counterpart of the `#[inline(never)]` MF-2 put on `read_nametable_inner`, not a
contradiction of it. The pair expresses one intent: **one** out-of-line call per nametable fetch,
with the whole body inside it. MF-2 pinned the outer boundary so `layout_of` could not be hoisted
into `read_pattern_planes`; nothing pinned the inner one, so the inliner was free to insert a
second call, and twice now it has — the field doc of `Cartridge::nametable_pages` records MF-2
finding this very function out of line when the UNROM 512 became a second board with a nametable
page of its own, and MF-3 did it again by adding boards.

### What it gives back

Post-LTO, after the fix:

| symbol | defs | call sites |
| --- | ---: | ---: |
| `Ppu2C02::read_nametable_as` | **0** | **0** — inlined, as at `7cbd9c52` |
| `Ppu2C02::read_nametable_inner` | 1 | 15 — still the one out-of-line call per fetch |
| `Ppu2C02::read_pattern_planes` | 1 | 3 — unchanged |

and in the lib dump the function it absorbed into is the only thing that moved:

| function | `7cbd9c52` | MF-3 | this step |
| --- | ---: | ---: | ---: |
| `Ppu2C02::read_nametable_inner` | 11 | 11 | **132** |
| `Ppu2C02::read_nametable_as` | 107 | 107 | *inlined* |
| `Ppu2C02::read_pattern_planes` | 106 | 106 | **106** |
| `Ppu2C02::paint_side_tile` | 1 277 | 1 277 | **1 277** |
| `Ppu2C02::wide_picture_into` | 901 | 901 | **901** |

`read_pattern_planes` is **106, exactly its baseline** — the shield MF-2 built is doing its job,
which is the thing to check when inlining anything on this path. The tile emitter's two big
functions do not move either, so the other call sites of `read_nametable_as` cost nothing.

**Expected effect on R0's figures: −3.18 M on `read_nametable_as` and −0.84 M on
`read_nametable_inner`**, i.e. the 4.0 M those two gained.

### Tests

`cargo fmt --check` clean, `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean, `cargo test --release -p zax-nes -p zax-harness`: **680 passed, 0 failed, 7 ignored**,
unchanged from step 2. An inlining attribute cannot change what the function reads.

## Step 4 — the residual PPU delta, and the lot measured end to end

### `Board::ppu_read` (+1.99 M) and `catch_up_ppu` (+1.96 M)

Neither is a regression of its own.

**`Board::ppu_read`.** Its prologue is the *same* niche-tag jump table at `7cbd9c52` and now (this
dispatch already had one before MF-3), and its MMC1 arm is instruction for instruction what it
was — the arm reached through jump-table slot 1, on both the CHR-ROM path Zelda II takes and the
CHR-RAM path Mega Man 2 takes:

    .LBB505_5:  (7cbd9c52)                .LBB511_7:  (now)
      movq  64(%rdx), %r9                   movq  64(%rdx), %r9
      testq %r9, %r9                        testq %r9, %r9
      jne   .LBB505_39                      jne   .LBB511_44
      movq  208(%rdx), %r9                  movq  208(%rdx), %r9
      addq  $200, %rdx                      addq  $200, %rdx
      testq %r9, %r9                        testq %r9, %r9
      jne   .LBB505_48                      jne   .LBB511_46
      jmp   .LBB505_41                      jmp   .LBB511_43
      ... addq 8(%rcx,%r8), %rax            ... addq 8(%rcx,%r8), %rax
      cmpq  %r9, %rax                       cmpq  %r9, %rax

Same registers, same displacements (64, 208, 200, 56, 8), same shifts and masks. What differs is
only which arms share the bounds-check tail and which duplicate it — `7cbd9c52` labels it
`.LBB505_49` and the current build falls through to it. That is the tail-merging / tail-duplication
flip MF-1, MF-2 and MF-3 all met on this family of functions, and MF-2 step 0 established which
way round is cheaper: *duplication*, which is what the current build has. R0's +1.99 M over ~6.3 M
calls is +0.3 instructions per call — an average over arms, not a cost on a path, and below what
a static count can resolve. There is nothing to fix here and nothing was changed.

**`catch_up_ppu`** is 168 instructions at `7cbd9c52` and 168 now, byte for byte. Its +1.96 M was
the *inclusive* cost of the callees above propagating up, exactly as MF-2 step 0 showed for
`read_pattern_planes` ("its +32 per call is 16 × (+2), the inclusive cost of the callee"). The
same goes for `background_fetch` −5.7 M and `run_dots` −3.4 M: code moving with
`on_ppu_address_bus`, not work appearing or disappearing.

### No new out-of-line call anywhere

`#[inline(always)]` on `end_cycle` made the CPU interpreter re-outline one level deeper:
`Cpu6502::execute` is now folded into `step`, and `execute_without_operand` (which existed before
and was inlined into `execute`) became a symbol of its own. Checked, because trading one
per-instruction call for another would have been pointless:

| | `7cbd9c52` | now |
| --- | --- | --- |
| `Cpu6502::execute` | 2 674 instructions, **1 call site** | folded into `step` |
| `Cpu6502::execute_without_operand` | inlined, no symbol | 2 248 instructions, **1 call site** |

One out-of-line boundary per instruction before, one after — and now only for the opcodes that
reach `execute_without_operand`, where before *every* instruction paid `callq execute`. The
interpreter's call count per instruction is unchanged or lower, and the per-CPU-*cycle* call
(`end_cycle`) is gone, which was the whole point.

### The lot's static table, `7cbd9c52` → MF-3 → now

| function | `7cbd9c52` | MF-3 | MF-3b | |
| --- | ---: | ---: | ---: | --- |
| `Cpu6502::end_cycle` | *inlined* | **76, 75 calls** | *inlined* | the core regression, undone |
| `Ppu2C02::read_nametable_as` | 107 (inlined by LTO) | 107 (**called**) | *inlined* | step 3 |
| `Ppu2C02::read_nametable_inner` | 11 | 11 | 132 | absorbed its callee; still `inline(never)` |
| `Ppu2C02::read_pattern_planes` | 106 | 106 | **106** | MF-2's shield intact |
| `Board::ppu_read` | 241 | 236 | 236 | MMC1 arm instruction-identical |
| `Board::ppu_peek` | 240 | 235 | 235 | likewise |
| `Board::on_ppu_address_bus` | 106 | 155 | 155 | now unreachable for 26 of 29 boards |
| `Ppu2C02::catch_up_ppu` | 168 | 168 | **168** | byte-identical |
| `Ppu2C02::run_dots` | 559 | 559 | **559** | byte-identical |
| `Ppu2C02::step_dot` | 1 409 | 1 403 | 1 413 | +4: the latch tests of step 2 |
| `Ppu2C02::background_fetch` | 187 | — | 194 | +7: likewise |

The `step_dot` +4 and `background_fetch` +7 are the **cheap half of step 2's trade**: a `cmpb` and
a not-taken `jne` inlined at each call site, in exchange for a `callq`, a niche prologue, a
jump-table indirect jump and a `retq` that no longer happen. Static size up, executed instructions
sharply down.

### Wall clock, as corroboration

`zax-harness bench-core <rom> --frames 3600 --warmup 300`, `core run_frame`, three runs each,
against the figures MF-3 step 9 recorded on this same host with this same command:

| ROM | before the boards (MF-3 step 2) | MF-3's end | **MF-3b** |
| --- | ---: | ---: | ---: |
| `M0_P32K_C8K_V` (NROM) | 0.819 | 0.860 | **0.789 / 0.795 / 0.800** |
| `M1_P128K` (mapper 1) | 0.826 | 0.861 | **0.792 / 0.805 / 0.806** |
| `M4_P128K` (mapper 4) | 0.997 | 1.042 | **1.001 / 1.002 / 0.998** |

MF-3's +4 % is gone. Mapper 1 and NROM land *below* the pre-lot figure, which is step 2 being an
improvement rather than a restoration; mapper 4 lands on its pre-lot figure, so the +2 instructions
per fetch that step 2 costs an MMC3 do not show above this host's noise.

**The caveat, stated rather than buried:** these are not interleaved runs against a
freshly-built baseline binary — the two left-hand columns are MF-3's recorded numbers, taken on
this host at another time, and a `git archive` build of `7cbd9c52`'s whole dependency tree needs
~3.4 GB against 6.4 GB free on G:. So this table is corroboration, not proof. **The generated code
above is the proof**, and callgrind is R0's instrument.

### What R0 should see

| R0's measurement | expected |
| --- | --- |
| Mega Man 2 / Zelda II **core** | back to `7cbd9c52` exactly; the +20.5 M was `end_cycle` and nothing else |
| `Cpu6502::end_cycle` exclusive | 159.9 M → **0** (no such function) |
| `Ppu2C02::read_nametable_as` exclusive | 3.18 M → **0**, and `read_nametable_inner` −0.84 M |
| `Board::on_ppu_address_bus` | not +4.84 M but a **large negative**: 26 of 29 boards no longer call it at all, ~9 instructions a fetch |
| Mega Man 2 / Zelda II **PPU** | **below** `7cbd9c52`, by roughly what that last row is worth |
| `Board::ppu_read` | the +1.99 M may partly remain: placement, not work (see above) |
| golden-core | unchanged; no board's behaviour was touched |
