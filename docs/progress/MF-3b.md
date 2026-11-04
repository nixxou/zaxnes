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
