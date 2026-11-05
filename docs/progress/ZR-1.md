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
