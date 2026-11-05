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
