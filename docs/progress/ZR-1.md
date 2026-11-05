# Lot ZR-1 — a code fetch from the widescreen hack zone must not read the PPU registers

Baseline: `ee5fd0ce` (MF-5a's end). First bad commit: `acfa6e20` (MF-4a-bis 2/3). Last good:
`c4577878` (MF-4a 5/5).

A regression lot. It changes no behaviour outside the widescreen extension's execution zone, and
it must change no instruction count outside it either.

## Steps

1. Reproduce, and confirm or refute R0's hypothesis.
2. The fix: a zone-only halted path, and the MF-3b check on the five functions.
3. The unit test: a zone fetch under a held DMC request and under a pending transfer.
4. The harness-level regression test on Super Mario Bros. 3, gated on the ROM.
5. The A/B against `c4577878`: SMB3, Ninja Gaiden III, the test-ROM campaign, the instruction count.
6. The suites.

## Step 1 — reproduce, and the diagnosis

### The command, and the two pictures

    zax-harness render --rom "<roms>/Super Mario Bros. 3 (USA).nes" --run 1500 \
      --press <R0's script> --poke 1000:7CFF=01 --advanced --widescreen 128 \
      --out <dir> --png-nes <file>.png

Reproduced at the first attempt. `c4577878` draws the row of ground above the status bar; the
baseline draws it broken, and the harness's own summary line counts **22 colours against 21** —
the picture has lost one. The two PNGs differ in **51 rows and 9 999 pixels**, all of them
between y = 169 and y = 220, which is that row of ground and what the engine builds under it.

### The counter, and what it refutes

A throwaway `[AtomicU64; 9]` in `nes_bus.rs`, incremented in `NesBus::zone_read_cycle` on the
`dma != 0` side and indexed by the PPU register the halted address decodes to, printed by the
`render` command and then reverted — it is in no commit. Over the 1 500 frames above:

| zone fetches halted by a DMA unit | 3 301 |
| --- | ---: |
| of them, a real PPU-register read made on behalf of the fetch | **3 301** |

and by register:

| `$2000` | `$2001` | `$2002` | `$2003` | `$2004` | `$2005` | `$2006` | `$2007` |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 320 | 423 | **420** | 453 | 440 | 452 | 382 | **411** |

Not one of the 3 301 was absorbed by the extension's own `$3000`-`$3FFF` register window
(`NesBus::zone_data_address`), so every one of them reached `Ppu2C02::read`. R0's hypothesis is
**confirmed**, and with the numbers: 420 reads of `$2002`, each clearing the `w` latch, and 411
of `$2007`, each moving `v` and the read buffer — and, since MF-4d step 6, moving `v` by a coarse
X and a Y at once when they land during rendering.

### Where the bug came from

`acfa6e20` made `halted_read_cycle` end in `read_internal(address)` on **both** its paths,
because a read cycle's read now happens there instead of at the call site. `zone_read_cycle`,
added by the same commit, reuses it. For every other address that is right — the byte does come
from the bus. For a zone address it is the one thing that must not happen: the byte comes from
the pack's code buffer, and at `c4577878` the only read the bus saw was `run_dma`'s own repeated
read of the halted address.

### The A/B that proves the fix is the whole fix, and names what is left

Eight clean `git archive` builds, each in its own directory with its own `CARGO_TARGET_DIR`,
same command, SHA-256 of the frame-1500 PNG. "+fix" is this lot's change transplanted onto that
commit.

| build | frame-1500 PNG |
| --- | --- |
| `c4577878` MF-4a 5/5 — **the reference** | `a923573ab7b9b3f8…` |
| `acfa6e20` MF-4a-bis 2/3 | `1a50620dd8470ac1…` |
| `a5994fd1` MF-4b 5/5 | `1a50620dd8470ac1…` |
| `64550944` PR-1 6/6 | `1a50620dd8470ac1…` |
| `ad1ad765` MF-4d 7/7 | `10bf5cf06266d70f…` |
| `ee5fd0ce` MF-5a 2/7 — **the baseline** | `10bf5cf06266d70f…` |
| `acfa6e20` **+fix** | `a923573ab7b9b3f8…` ✔ the reference |
| `64550944` **+fix** | `a923573ab7b9b3f8…` ✔ the reference |
| `d96d73b7` MF-4d 5/7 **+fix** | `a923573ab7b9b3f8…` ✔ the reference |
| `a758f923` MF-4d 6/7 **+fix** | `a1516fbf96fa8de7…` |
| `ad1ad765` **+fix** | `a1516fbf96fa8de7…` |
| `ee5fd0ce` **+fix** (this lot) | `a1516fbf96fa8de7…` |

Two things follow, and the second is the one that has to be said plainly.

* **The fix is the whole fix.** Transplanted onto `acfa6e20` it restores `c4577878`'s picture
  **byte for byte**, and it still does at `64550944` and at MF-4d step 5 — so everything
  MF-4a-bis, MF-4a-ter, MF-4b, MF-4c and PR-1 did leaves this picture untouched, and the zone
  read is the entire regression.
* **At the baseline the fixed picture is not `c4577878`'s, and must not be.** The lot's
  instruction asks for frames 1 500 and 1 800 to match `c4577878`, and they cannot: one
  **deliberate** change sits in between. The bisect above puts it in MF-4d, and inside MF-4d
  between steps 5 and 6 — `a758f923`, "a fetching PPU moves `v` by a coarse X and a Y at once".
  MF-4d recorded that move itself: its step 7 reads "**Measured, step 7: `Super Mario Bros. 3`
  and `Zelda II` both move**", its table gives SMB3 960 → 992 tile records, and its own verdict
  line says "three ROMs move, two of them commercial, bisected to step 6's `$2007` half …
  flagged for R0's pack A/B". So the hack picture moves for the same reason the golden picture
  did, and for the pack the flag was raised about.

  The reference this lot can therefore hold at the baseline is **`c4577878` + MF-4d**, which is
  `ad1ad765`+fix — and `ee5fd0ce`+fix matches it byte for byte, so MF-5a moves nothing here
  either. Step 5 states both A/Bs that way.

### The control

The same command without `--advanced --widescreen 128`, 1 500 frames, `c4577878` against the
fixed baseline: **the same SHA-256**. Outside the hack there is no difference to find, which is
what makes the two pictures above attributable to the zone path alone.

## Step 2 — the fix

### What it is

`NesBus::zone_read_cycle` stops sharing `halted_read_cycle` and calls a zone-only cold half,
`NesBus::halted_zone_read_cycle`, which is `halted_read_cycle` **minus the CPU's own read**:

* `promote_dmc_wait(self.cycles)` — the cycle that has just run is where a halt would land;
* nothing to do when no unit may halt, where `halted_read_cycle` would `read_internal`;
* `dmc_only_dma` or `run_dma` otherwise, unchanged, **including the halt's own repeated read of
  the zone address**. That read is the hardware's: the CPU is stopped with `$2xxx` on the address
  bus whatever the fetched byte came from, and it is the read the bus saw at `c4577878`;
* the cycle the CPU finally fetches on, and return. The byte comes from the pack's code buffer in
  `read_code`, as it always did.

Two details, both deliberate:

* **`dma_gave_back` is not stamped.** It is read by the five `SH*` stores about *their own dummy
  read*, which is a data read and never an instruction fetch, so a zone fetch has no business
  recording one. It did not exist at `c4577878` at all.
* **The `$4000`-`$401F` test is dropped, not kept.** A zone address is in `$2000`-`$3FFF` by
  construction (`Wide::zone_offset`), so `halted_read_cycle`'s `(address & 0xFFE0) != 0x4000` is
  answered here once and for all — the halted address is never one of the 2A03's own registers.

`NesBus::read`, `read_cycle`, `halted_read_cycle`, `read_code` and `run_dma` are **not touched**:
`acfa6e20`'s gain for every non-zone read is kept whole.

### The MF-3b check on the five functions

Clean `git archive` builds of the baseline `ee5fd0ce` and of this tree, each with its own
`CARGO_TARGET_DIR`. Instructions per symbol and **call sites per callee**, MF-3b rule 1, on the
pre-LTO lib dump (`cargo rustc --release -p zax-nes --lib -- --emit=asm`) and — MF-3b rule 2,
because all of this is on the CPU read path — on the **post-LTO** dump of the shipped binary
(`cargo rustc --release -p zax-harness --bin zax-harness -- --emit=asm`). The two dumps agree
symbol for symbol; the post-LTO figures:

| symbol | ins, baseline | ins, fixed | call sites, baseline | call sites, fixed |
| --- | ---: | ---: | ---: | ---: |
| `NesBus::read_cycle` | 182 | **182** | 65 | **65** |
| `NesBus::halted_read_cycle` | 53 | **53** | 0 | **0** |
| `NesBus::zone_read_cycle` | 18 | **18** | 36 | **36** |
| `NesBus::run_dma` | 511 | **511** | 1 | 2 |
| `NesBus::dmc_only_dma` | 102 | **102** | 1 | 2 |
| `NesBus::read_internal` | 176 | **176** | 14 | **14** |
| `NesBus::tick` | 79 | **79** | 84 | **84** |
| `NesBus::halted_zone_read_cycle` | — | 43 | — | 0 |
| `Cpu6502::step` | 916 | **916** | 2 | **2** |
| `Cpu6502::resolve` | 1 402 | **1 402** | 3 | **3** |
| `Cpu6502::branch` | 377 | **377** | 0 | **0** |
| `Cpu6502::execute_without_operand` | 2 333 | **2 333** | 0 | **0** |

`CpuBus::read_code` has no symbol in either dump: it is `#[inline(always)]` and inlined
everywhere, and `zone_read_cycle`'s **36 call sites — unchanged** are its fetch sites, which is
the same statement seen from the callee's side.

Over the whole binary dump: **903 690 → 903 733 instructions, +43**, which is
`halted_zone_read_cycle` and nothing else, and **exactly two** callees moved a call site —
`run_dma` and `dmc_only_dma`, each 1 → 2, both the new cold path's own. Neither gains its *first*
call site, so neither is MF-3b's stop-the-lot event, and neither is reachable from a read that is
not a zone fetch. Every other symbol in the binary is identical.

The cost is therefore **zero instructions on every path that is not a halted zone fetch**, which
is every instruction of every game without hack code, and on a halted zone fetch it is one
`callq` to a 43-instruction function instead of one `callq` to a 53-instruction one.

### The suites at this step

`cargo fmt --check` clean. `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean. `cargo test --release -p zax-nes -p zax-harness`: **741 passed, 0 failed, 7 ignored** —
the baseline's own count, no test added or removed yet.

No `unsafe`. No state field added, removed or reordered, and nothing new is serialised: the save
format does not move.

## Step 3 — the unit tests

Three, in `exec_zone_tests.rs` (REQ-CORE-113), and **all three fail on the baseline**, each by
exactly one `$2007` read. They are built on one idea: of the eight registers, `$2007` is the only
one whose read has an effect that **accumulates** — `v` steps once per read — so `v` counts the
reads for the test, and no assertion ever has to name how many repeats the DMA itself makes.

| test | the state | what it asserts |
| --- | --- | --- |
| `…_a_zone_fetch_under_a_held_dmc_request_touches_no_ppu_register` | `$4015` has started a sample, so the request is **held**: `dma` non-zero, no unit may halt | `v`, the read buffer **and** `w` are the control's, the fetched byte is the pack's, and the held fetch still runs once it is due |
| `…_a_halted_zone_fetch_reads_one_time_fewer_than_the_data_read_it_stands_for` | the DMC fetch is **due**: the read cycle is halted | `v` moved **one less** than for the data read the fetch stands for, and both completed the fetch |
| `…_a_zone_fetch_halted_by_an_oam_transfer_lets_it_finish_and_reads_nothing_itself` | a `$4014` transfer is pending — the other unit, the other path through `run_dma`, 512 cycles | the same one-less relation, plus the 256 bytes land in OAM byte for byte |

Two notes on how they are written:

* **The held case uses a control, the halted cases use a counterpart.** With a held request no
  unit may take the bus, so the fetch must be indistinguishable from a cycle spent writing CPU
  RAM — a clone doing exactly that is the control, and `v`, the buffer and `w` must all match it.
  Once a unit *does* halt, the DMA's own repeated read legitimately moves the PPU, so there the
  counterpart is the **data read the fetch stands for**: same state, same cycle, `bus.read`
  instead of `bus.read_code`. The difference between them is the CPU's own read, and it must be
  exactly one.
* **`w` is only decidable in the held case.** A halt's repeated read of `$2002` clears the toggle
  whether the access was a fetch or a data read, so the halted tests cannot tell the two apart on
  `w` and do not pretend to; the held test can, and does, with the `$2006` double-write probe the
  REQ-CORE-112 test already uses.

The OAM assertion masks bits 2-4 of each sprite's attribute byte, which are not implemented and
read back as 0 (nesdev, "PPU OAM") — found by the test, not assumed.

`cargo fmt --check` clean. `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean (`cast_sign_loss` joins the file's existing `cast_possible_truncation` allow: the PPU's
scroll accessors are masked bit-fields and never negative).
`cargo test --release -p zax-nes -p zax-harness`: **744 passed, 0 failed, 7 ignored** — 741 plus
these three.

## Step 4 — the harness-level regression test

`crates/zax-harness/tests/widescreen_zone_tests.rs`, one test, `#[ignore]`d and gated on
`ZAXNES_ROMS` exactly as `background_row_split_tests.rs` gates its own Super Mario Bros. 3 run:
the folder is read from the environment, an absent ROM prints `skipped:` and returns.

It is R0's command, less the part that needs a GPU. `render::rom::capture` is what
`render::build_scenes` calls behind `--rom`, so the same `RomRequest` — 1 500 frames, R0's press
script, the `$7CFF` poke at frame 1 000, `advanced`, `widescreen: 128`, the no-`--profile`
profile `profile::load(None, None)` — runs the identical emulation; only `render::run`'s GL pass
is skipped. The assertion is on `SyntheticScene::nes_picture`, the composed colour indices, 512 ×
240 of them with the margins: **the picture itself, with no PNG encoder and no JSON format
between it and the golden**, hashed with a four-line FNV-1a so the golden needs no crate.

| build | `fnv1a(nes_picture)` at frame 1 500 |
| --- | ---: |
| baseline `ee5fd0ce` | 10 895 653 189 805 518 951 — **fails** |
| this tree | **7 496 683 613 019 477 804** — passes |

**Why the golden is not `c4577878`'s number**, which is the lot's first choice and is not
available: MF-4d step 6 moves this picture deliberately. The test's own doc comment carries that
reasoning and the table of step 1, so the next person to see it move has the argument in front of
them rather than a bare number. The value pinned is `c4577878` **+ MF-4d step 6**, which step 1
shows is what `a758f923`+fix and `ad1ad765`+fix both produce.

The two cheap structural assertions in front of the hash — 240 rows, 512 × 240 indices — fail
first and more legibly if the margins or the geometry ever move, so a hash mismatch means the
pixels and not the shape.

`cargo test --release -p zax-nes -p zax-harness`: **744 passed, 0 failed, 8 ignored** — the new
test is the eighth ignored, and runs under `-- --ignored` with `ZAXNES_ROMS` set.

## Step 5 — nothing else moved

Five builds take part, all clean `git archive` extractions into their own directory with their
own `CARGO_TARGET_DIR`: `c4577878` (the reference), `d96d73b7`+fix (MF-4d step 5), `a758f923`+fix
(step 6), `ad1ad765`+fix (MF-4d's end), the baseline `ee5fd0ce`, and this tree.

### The test-ROM campaign, ROM by ROM

`zax-harness test-roms <assets-free/test-roms> <out.json> --no-shots`, three runs on the same
collection on the same host, compared with `campdiff.py`:

| comparison | rows | differing |
| --- | ---: | ---: |
| MF-4d's end (`ad1ad765`) → this tree | 331 / 331, same set | **0** |
| baseline (`ee5fd0ce`) → this tree | 331 / 331, same set | **0** |

214 pass, 5 fail, 0 timeout, 1 unsupported, 111 manual on all three — MF-4c's and MF-4d's four
totals. Identical as it had to be: no test ROM runs hack-zone code, so no test ROM reaches the
function this lot added.

### Super Mario Bros. 3, `--advanced --widescreen 128`

| check | result |
| --- | --- |
| frame **1 800** PNG | `7eb550a0d6568949…` on this tree, on `c4577878` **and** on `ad1ad765`+fix — identical to the reference, exactly as the brief asks |
| frame **1 500** PNG | `a1516fbf96fa8de7…` on this tree and on `ad1ad765`+fix; `c4577878` is `a923573ab7b9b3f8…`, which `d96d73b7`+fix reproduces |
| every 5th frame to **1 600** (321 dumps) vs `ad1ad765`+fix | **0 of 321 differ** |
| every 5th frame to 1 600 vs `c4577878` | 76 of 321 differ |
| the same 321 dumps, `c4577878` vs **`d96d73b7`+fix** | **0 of 321 differ** |

The last row is what settles the fourth. With the fix in place, **every** one of the 321 dumps is
`c4577878`'s up to MF-4d step 5; the 76 appear at step 6 and nowhere else. Frame 1 800 does not
move at all, so the instruction's two named frames come out one matched and one attributed.

### Ninja Gaiden III, `--advanced --widescreen 128 --run 700 --dump-every 5`

141 dumps, R0's press script.

| comparison | differing | first |
| --- | ---: | --- |
| baseline `ee5fd0ce` vs `c4577878` — **the regression** | 36 of 141 | frame **370** |
| `c4577878` vs `d96d73b7`+fix (MF-4d step 5) | **0 of 141** | — |
| `d96d73b7`+fix vs `a758f923`+fix (step 6) | 24 of 141 | frame 370 |
| `a758f923`+fix vs this tree | **0 of 141** | — |
| this tree vs `ad1ad765`+fix | **0 of 141** | — |
| this tree vs `c4577878` | 24 of 141 | frame 370 |

R0's **frame 370** is reproduced to the frame. The same shape as SMB3: with the fix, Ninja Gaiden
III is `c4577878` on all 141 dumps through MF-4d step 5, and what remains appears at step 6 — the
deliberate `$2007` rule — and nowhere after it. Both games' residuals start at frame 370 and
frame 15 respectively for the same reason the bug did: the status-bar split is where the pack's
code, the DMC samples and the `$2007` reads all meet.

### The instruction count, and a finding about the instrument

PR-1's scripts, two clean builds in the WSL2 Ubuntu, thirteen ROMs, 300 frames, no warm-up,
whole process.

| ROM | baseline `ee5fd0ce` | this tree | delta |
| --- | ---: | ---: | ---: |
| `duelito` | 7 100 519 901 | 7 100 494 416 | −25 485 |
| `m0` | 5 544 847 634 | 5 544 867 832 | +20 198 |
| `m1` | 4 915 888 869 | 4 915 930 628 | +41 759 |
| `m4` | 5 502 417 322 | 5 502 457 559 | +40 237 |
| `metroid` | 5 154 555 237 | 5 154 558 329 | +3 092 |
| `mm2` | 6 038 161 478 | 6 038 160 116 | −1 362 |
| `mmc5` | 11 420 255 552 | 11 420 272 617 | +17 065 |
| `oamstr` | 3 339 407 192 | 3 339 425 387 | +18 195 |
| `readbuf` | 6 851 901 549 | 6 851 867 013 | −34 536 |
| `smb1` | 6 032 328 877 | 6 032 340 399 | +11 522 |
| `smb3` | 8 005 056 054 | 8 005 071 601 | +15 547 |
| `vblnmi` | 3 328 544 719 | 3 328 552 077 | +7 358 |
| `z2` | 6 311 029 381 | 6 310 969 915 | −59 466 |
| **total** | 79 544 913 765 | 79 544 967 889 | **+54 124 (+0.00007 %)** |

`callgrind-diff.py`'s verdict line says *SLOWER on 9 ROMs*, and that verdict is **wrong**. Three
measurements say so, in increasing order of force.

**1. Four ROMs got faster.** A change that adds instructions cannot take 59 466 off Zelda II.
The deltas are two-directional, which is the signature of noise and not of cost.

**2. No symbol of the emulator moved — on any ROM, at threshold 1.** Comparing the two runs
symbol by symbol and keeping everything whose name contains `zax`, `Cpu6502` or `Ppu2C02`, the
**entire** list on all thirteen ROMs is two entries: the settings catalogue's
`HashMap<String, SettingId>` and the `CATALOGUE` initialiser that fills it, ±a few hundred
instructions in both directions, built once at startup. Not one `zax_nes` symbol appears — no
CPU, PPU, APU, bus or board. `halted_zone_read_cycle` has no exclusive count at all: no ROM of
the set runs hack-zone code, so nothing calls it. Above the per-symbol threshold the only movers
in the whole process are Lua (`luaH_getshortstr`, `getgeneric`, `luaS_newlstr`, `luaH_set`),
libc (`__memcpy_avx_unaligned_erms`, `__memcmp_avx2_movbe`) and `flt2dec`.

**3. The same binary, against itself, moves more than the change does.** `zax-harness-fix` was
copied to a second label and the whole set re-run:

```
m1       4,915,930,628  ->  4,915,880,918   -49,710  -0.001
m4       5,502,457,559  ->  5,502,408,445   -49,114  -0.001
metroid  5,154,558,329  ->  5,154,578,205   +19,876  +0.000  SLOWER
TOTAL   79,544,967,889  -> 79,544,896,128   -71,761
VERDICT: SLOWER on 8 ROM(s)
```

One binary compared with itself swings by up to **49 710** instructions on a ROM and by 71 761 in
total — **more than the 54 124 of the change** — and the tool declares it slower on eight ROMs.
The per-symbol diff of that self-comparison names the same movers: `getgeneric` ±43 940,
`luaH_getshortstr` ±29 204, the catalogue's `HashMap`.

**The mechanism**, and why it is not the change's: both of those tables are hashed with a seed
taken from an address (`hashbrown`'s `RandomState`, Lua's `makeseed`). A different seed puts the
same keys in different buckets and costs a different number of probes, and the probing is what
`getgeneric` and `luaH_getshortstr` count. It varies between two runs of one binary and it varies
between two binaries; it has nothing to do with emulation, which never hashes a string.

**So PR-1's premise needs a footnote.** `PERFORMANCE-HISTORY.md` §3 and PR-1 step 1 both state
that the Ir count is "deterministic to the instruction: the same binary twice gives the same
number". For the symbols that matter it still is — `NesBus::read_internal` and its neighbours do
not move by one instruction between runs. For the **whole-process total** of `zax-harness` it is
no longer true: the floor is about **±50 000 per ROM**, which is 1 × 10⁻⁵ and far below anything
a per-cycle regression produces (MF-3b's was +20 400 000 on one ROM, four hundred times this),
but far above a verdict line that compares exact integers. Out of this lot's scope to edit that
document — flagged here for R0 to fold into §3, together with the suggestion that the verdict
compare the per-symbol emulator counts rather than the process total.

**The bar, answered on the measure that resolves it.** "Identical to the instruction, or lower,
on every ROM without hack code": the generated code of every non-zone path is byte-identical
(step 2's post-LTO dump, symbol for symbol and call site for call site), and no emulator symbol
executes a different number of instructions on any of the thirteen ROMs. The named, measured cost
on a zone fetch alone is one `callq` into a 43-instruction function in place of one `callq` into
a 53-instruction one — which is lower, and which only a game running hack code ever pays.

## Step 6 — the suites

| suite | result |
| --- | --- |
| `cargo fmt --check` | clean |
| `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings` | clean |
| `cargo test --release -p zax-nes -p zax-harness` | **744 passed, 0 failed, 8 ignored** (741 + 3 unit + 1 ignored harness test) |
| the same with `--include-ignored --test-threads=1`, `ZAXNES_ROMS` and `ZAXNES_TEST_ROMS` set | **750 passed, 2 failed**, both failing identically on the baseline (below) |
| the test-ROM campaign | 331 rows, **0 differences** against MF-4d's and against the baseline (step 5) |
| `golden-core`, 330 free ROMs, 1 200 frames | baseline against this tree: **byte-identical** |
| `golden-core`, 18 commercial ROMs, 120 frames | baseline against this tree: **byte-identical** |

Golden-core runs without hack code, so "nothing may move" is met in the strongest form available:
the two JSONL files compare equal with `diff`, line for line, on 348 ROMs.

**Save states**: no field added, removed or reordered, nothing new serialised, no change to
`save_state` or `load_state`. The format does not move.

No `unsafe`. No Q ticket: nothing in the spec was ambiguous here — the zone's rule is the
extension's own, and the hardware's behaviour on a halt is the one `run_dma` already implements.

### The two ignored tests that fail, and one of them matters

Both fail **identically on the baseline** `ee5fd0ce`, so neither is this lot's; both are only
reachable with `--include-ignored` and a commercial ROM, which is why no earlier lot's suite line
mentions them. Out of scope to fix here, and reported for R0.

**1. `builtin_patches::tests::the_zelda_ii_patch_applies_to_the_usa_rom`** — the second
assertion, `apply(&hack).is_none()`: an already-patched image is patched again. A patch-detection
matter, nothing to do with the core.

**2. `amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line`** — and this one is a real
regression in a shipped lot. The criterion is that Super Mario Bros. 3 never emits a background
row on its split line; it now emits one on **4 986 of 5 000 frames**. Bisected here, each build
clean, the test being a no-hack `NesCore` run that this lot's change cannot reach:

| build | the test |
| --- | --- |
| `64550944` PR-1 6/6 | **passes** |
| `d96d73b7` MF-4d 5/7 | **passes** |
| `a758f923` MF-4d 6/7 | **fails**, 4 986 / 5 000 |
| `ee5fd0ce` baseline, and this tree | **fails**, 4 986 / 5 000 — the same number |

It is MF-4d step 6 again, the same `$2007`-while-fetching rule as the picture moves of step 5 —
and this time it breaks a **documented requirement**, AMD-021 R2-005, on the very game and the
very scanline (193) MF-4d's own probe named. MF-4d's step 7 weighed the rule against its picture
moves and kept it; it did not have this row, because the test needs a commercial ROM and is
`#[ignore]`d. MF-4d itself wrote that the half "is isolated to four lines of
`increment_vram_address`, so backing it out alone is a one-line revert if the A/B says to" — that
decision is R0's, and it now has one more input.

### For R0

1. The lot's instruction "SMB3 frames 1 500 and 1 800 match `c4577878`" holds for 1 800 exactly;
   1 500 is `c4577878` + MF-4d step 6, demonstrated by six builds in step 1 and by the dense
   dumps in step 5. The same for Ninja Gaiden III: identical to `c4577878` through MF-4d step 5.
2. `PERFORMANCE-HISTORY.md` §3's "deterministic to the instruction" needs the footnote step 5
   measured: the whole-process count has a ±50 000 per-ROM floor from address-seeded hashing.
3. `amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line` has been failing since
   `a758f923`.
