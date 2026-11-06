# The board features of `zax-nes`

One Cargo feature per board that raises any figure (owner's rule, 2026-11-06), built by lot
**MF-6a3**. Each gates its board's code, its `Board` variant or `const` monomorphisation, every
dispatch arm, its `SUPPORTED_MAPPERS` entries, its `registry` arms, its `board_memory_sizes` arm,
its save-state block and its tests.

**The tolerance is at most +0.02 % per ROM on both runs** — PR-1's thirteen ROMs, and level 2's
five games on `core` *and* `total` — measured against the base commit of the lot that added the
board. A board inside it is in `zax-nes`'s `default`; a board above it is left out, and the
umbrella feature **`parked-boards`** then enables all the left-out ones at once.

**One board is parked, and MF-6c step 2 is the lot that parked it.** `parked-boards` was empty
through MF-6b and MF-6a3, wired so that a later lot which did park a board would find the switch
ready; it now holds **`board-taito-x1`**. Everything else is in `default`.

## The table

Costs are the figure the step that added the board measured, with the worst ROM named; `07325f6e`
is the end of MF-6b and `240a2718` the end of MF-5b-fix. Flipping a feature is one command, given
in the last column; it is `-p zax-nes` because that is the crate the feature belongs to, and the
same flag passed to `zax-harness` or `zax-app` as `--features zax-nes/<name>` reaches it too.

| feature | mappers | board | titles | measured cost, PR-1's thirteen | measured cost, level 2 | default | the command that flips it |
| --- | --- | --- | ---: | --- | --- | :-: | --- |
| `board-namcot108` | 76, 88, 95, 154, 206 | Namco 108 / DxROM, with four `const`/`bool` parameters | **45** | worst `m4` **+66 921 (+0.0012 %)** (MF-6a step 1, vs `240a2718`) | `smb3/core` **+45 707 (+0.0003 %)** | **on** | `cargo build -p zax-nes --no-default-features --features <the other eight>` |
| `board-small-boards` | 33, 70, 75, 89, 93, 97, 140, 152, 184, 185, 232 | nine boards behind **one** `Board` variant: Taito TC0190, Bandai 74161/7432, Konami VRC1, both Sunsoft-2 boards, Irem TAM-S1, Jaleco JF-11/14, Sunsoft-1, CNROM + diodes, Camerica BF9096 | **~30** | step 2 `smb3/core` **+433 086**; step 3 **+461 302**; step 4 adds `mm2/core` **+10 506** (MF-6a, vs `240a2718`) | the same three steps' `core` figures, +0.0001 % to +0.0030 % | **on** | as above |
| `board-94` | 94 | HVC-UN1ROM, a wiring of the UxROM board (its bank field shifted two bits) | 1 | in MF-6a step 4's `mm2/core` **+10 506** and `smb3/core` **+461 302** with 155 and 185 | the same | **on** | as above |
| `board-155` | 155 | the MMC1 on a board with no PRG-RAM disable wire | 1-2 | MF-6a2 step 2 traced step 4's `mm2/core` **+10 506** to this parameter — `Mmc1::update_banks` **+8 001** over 600 level-2 frames of *Mega Man 2* — and then **removed it**: the two-mask idiom left that function **215 instructions, five below** `240a2718`'s 220 | as above | **on** | as above |
| `board-105` | 105 | Nintendo NES-EVENT, holding an `Mmc1` | 1 | **all thirteen below** `9f9d9d41` (MF-6b step 3, set −21 123 914) | `smb1/total` **+3 149 624 (+0.023 %)**, `smb3/total` +0.010 % — **both entirely Lua's `ltable.c`**, and both gone when step 4 re-measured | **on** | as above |
| `board-158` | 158 | Tengen 800037, a `const` parameter of the RAMBO-1 | 1 | **all thirteen below** `9f9d9d41` (MF-6b step 5, set −19 980 904) | `smb3/total` **+1 233 269 (+0.005 %)**, of which Lua's `ltable.c` is +4 008 826 and this project's own symbols −3 104 672 | **on** | as above |
| `board-119` | 119 | Nintendo TQROM, a `const` parameter of `Mmc3Board` with a CHR-RAM chip beside its CHR-ROM | 2 | **with the multicart below**: worst `m0` **+58 307 (+0.0011 %)**, set total **−57 265** (MF-6a3 step 2, vs `07325f6e`) | all five `core` within **±6 699**, `smb1/core` **+0**; two `total` rises inside the same binary's own ±4.3 M spread on that column | **on** | as above |
| `board-mmc3-multicart` | 47, 37 | Nintendo's two MMC3 multicarts, one `const` monomorphisation with an outer register at $6000-$7FFF | 2 | as `board-119`, measured together | as `board-119` | **on** | as above |
| `board-116` | 116 | Gouder SOMARI-P / Huang ASIC, **holding** an `Mmc3`, an `Mmc1` and a `Vrc2Vrc4` | **6** | worst `m1` **+96 537 (+0.0020 %)**, set total **−7 291 714 (−0.009 %)**, `mmc5` **−0.066 %** (MF-6a3 step 4, vs `07325f6e`) | all five `core` **+253 to +16 832** (`smb3/core` is 253 instructions in 15.3 G); worst `total` `smb3` **+0.011 %** | **on** | as above |
| `board-153` | 153 | the Bandai FCG board with an LZ93D50 and 8 KiB of battery-backed WRAM, **two appended `bool`s on `BandaiFcg`** | 1 (*Famicom Jump II*) | worst `duelito` **+57 669 (+0.001 %)**, set total +37 631 (MF-6c step 1, vs `8496c503`) | **all five `core` +0**; worst `total` `smb3` +0.005 %, entirely Lua's `ltable.c` | **on** | as above |
| `board-taito-x1` | 80, 207, 82, 552 | the Taito X1-005 and X1-017, two `SmallBoard` variants; implies `board-small-boards` | **13** | `mmc5` **+7 456 756 (+0.066 %)**, every other ROM within **±31 000** (MF-6c step 2, vs `8496c503`) | **all five `core` +0**; worst `total` `smb3` **+0.003 %** | **off — parked** | `cargo build -p zax-nes --features board-taito-x1` |
| `parked-boards` | 80, 207, 82, 552 | the umbrella: every board left out of `default`. Today that is `board-taito-x1` | 13 | as that row | as that row | **off** | `cargo build -p zax-nes --features parked-boards` |

**Worst figure among the features in `default`: +0.0020 % on one ROM**, against a tolerance of
+0.02 %.

## The one parked board, and why

`board-taito-x1` is **+0.066 % on one ROM of one instrument and zero everywhere else**, and the
mechanism is a single instruction that has nothing to do with the Taito boards:
`Cartridge::default_cpu_read` grows past about 390 instructions, and LLVM then hoists
`leaq 240(%rdi), %rax` — `&self.board`, wanted only by the cold arms of the inlined
`Board::prg_ram_enabled` jump table, mapper 116's boxed payload among them — **above** that
dispatch, where it runs on every call. The MMC5 is the one board that maps PRG-RAM into
$8000-$DFFF, so every CPU read of an MMC5 game reaches that function.

**The figure is a two-state oscillation this project has measured three times in both
directions**: MF-6b steps 2-5 were charged `default_cpu_read` **+7 456 756** on `mmc5` for
`IrqBoard`'s sixth body, MF-6a3 step 4 was given **−7 456 756** back for its eighth and ninth, and
MF-6c step 2 was charged it again. `8496c503` sits on the favourable side of it by luck. MF-6c
step 2 measured **seven** shapes against it — including taking the decode from
`cpu_peek_override` to `try_read_prg`, and pinning one, two and three links of the chain — and
none held every ROM; its own write-up has the table.

So the park is the owner's rule applied, not a verdict on the boards: with the feature **on**, all
five level-2 `core` figures are **+0** and the worst `total` is `smb3` at +0.003 %, on the
instrument that loads a 3D pack. A later lot that adds any body to that chain may hand the figure
straight back.

## How to turn a board off

There is no `--no-feature` flag in Cargo, so turning one board off means naming the other eight:

    cargo build -p zax-nes --no-default-features --features \
        board-94,board-105,board-116,board-155,board-158,board-mmc3-multicart,board-namcot108
    # ... which is every board but `board-119`.

and `cargo test -p zax-nes --no-default-features` is the configuration with **every** board off —
**782 passed, 0 failed, 7 ignored**, against 836 by default. The registry then advertises **45**
mapper numbers instead of 69. All eleven configurations (each feature alone, none, all, and all
plus `parked-boards`) build, pass clippy `-D warnings` and pass their tests; MF-6a3 step 3's table
has the per-feature test counts.

A downstream crate reaches the features through its dependency: `--features zax-nes/board-119`.

## Three notes for whoever revisits these choices

**1. The granularity is the granularity the measurements exist at.** `board-small-boards` is
eleven mapper numbers because they are **one `Board` variant** — which is the whole reason
`SmallBoard` exists: MF-6a step 2 measured five *variants* at 0.067 % to 0.131 % on eight of
thirteen ROMs and one variant at nothing. Splitting it into nine features would need **nine
measurements no lot has made**: MF-6a priced its steps 2, 3 and 4, not its individual boards. The
same goes for `board-namcot108`'s five numbers, which are one board with four parameters.

**2. The three MF-6b steps that are *not* in the table are not an omission.** Steps 1 (mapper 73),
2 (65 and 67) and 4 (48) each shipped with every one of PR-1's thirteen ROMs and all five level-2
games at or below `9f9d9d41` on total *and* core — `l2-diff.py`'s own verdict line for all three
is "at or below the baseline on every game, total and core". No figure rose, so no feature. MF-6a2
added no board at all.

**3. A board's cost is not its own, and the lot that proved it is MF-6a3.** Three of its
measurements are the same phenomenon:

* two new `Board` discriminants cost `mmc5` **+8 994 278 (+0.080 %)** — one taken `jmp` per
  expansion-audio sample, because the 35-entry jump table of `Board::expansion_audio_output`
  grew by two and LLVM moved the fall-through off the MMC5's arm (step 1);
* the eighth and ninth bodies of `IrqBoard`'s inner match gave `Cartridge::default_cpu_read`
  **−7 456 756** on the same ROM, which is *to the instruction* the figure MF-6b steps 2 to 5
  charged it for the sixth (step 4);
* retrofitting nine features onto the existing boards moved thirteen ROMs by at most **±40 812**
  with **no symbol above 200 000 changing at all** (step 3) — the proof that the gating itself is
  free when every feature is on.

So a feature's measured cost is **the figure its own step saw in its own tree**, and turning two
features off does not subtract two costs. A reader who wants a board's cost in *today's* tree has
to measure that configuration; `rust/docs/PERFORMANCE-HISTORY.md` §2 and §3 have the method and
`rust/tools/perf/` the scripts.
