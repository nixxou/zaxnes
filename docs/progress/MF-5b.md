# Lot MF-5b — the VRC7 board, with its FM audio

Baseline `a577f336` (MF-5a's end). **782 tests**, 0 failed, 8 ignored, measured at the lot's start
and reproduced here before anything was written.

No file under `crates/zax-nes/src/cartridges/mappers/` handles iNES mapper 85, and
`SUPPORTED_MAPPERS` has not got it. REQ-CORE-075 makes expansion audio optional (P3) and Q-118
lists the VRC7 among the optional chips; MF-5a's VRC6 (`VrcIrq`, `expansion_audio_output`, the
`Board::on_cpu_cycle` latch) and the Sunsoft 5B are the patterns this lot follows.

## Steps

1. The VRC7 board (mapper 85), with the VRC IRQ **shared** with `vrc2_vrc4.rs` and `vrc6.rs`.
2. The VRC7 audio: the OPLL subset — six two-operator FM channels, the custom patch and the
   fifteen built-in ones, the envelope generator, the two LFOs.
3. Save states for the board and the audio block, behind the existing block versioning.
4. The ROM check.
5. The suites, the campaign ROM by ROM against MF-5a's, and the campaign document.

## The ROM situation, settled first because it shapes every step

**There is no mapper-85 image anywhere in `assets-free/test-roms`.** Every `.nes` under the tree
was opened and its iNES header decoded, the NES 2.0 submapper byte included: **330 images carry a
valid header**, and between them

    mappers 0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 22, 28, 34, 66, 69, 78, 118, 180

which is MF-5a step 7's list to the image. Mapper 85: **zero images**. `holy-mapperel` has no
`M85` target — its 43 files stop at `M180` — and no file anywhere in the tree has `vrc` or `85`
in its name.

So this lot cannot be driven by a ROM at any step, and that is stated here rather than discovered
at step 4: **every behavioural claim below rests on a unit test written against the nesdev pages
and the YM2413 application manual.** R0 runs the two commercial titles: *Lagrange Point*
(submapper 2, the VRC7a) and *Tiny Toon Adventures 2* (J) (submapper 1, the VRC7b).

## The sources, and the one that is missing

| source | what it gives |
| --- | --- |
| nesdev, "VRC7" | the whole board: the register map, the two address lines, `$E000`'s four bits |
| nesdev, "VRC7 audio" | the register map of the chip, the semantics of every operator field, the 26 internal registers, the fifteen built-in patches as hex, the multiplier table, the feedback table, the two waveforms, the frequency law, 49716 Hz |
| nesdev, "VRC IRQ" | the counter, already implemented as `VrcIrq` (MF-5a step 1) |
| YM2413 application manual (smspower, bitsavers) | `RATE = 4 × R + Rks`, the TL and SL step sizes in dB, the KSL law in dB/octave, the two LFO **frequencies**, "rises exponentially during attack time and varies linearly everywhere else" |
| YM2413 reverse-engineering notes 2015-03-20 (smspower) | the envelope's **0.375 dB per step over 128 levels**, the shared envelope counter, and the `eg_shift`/`eg_select` advance tables for rates 4-59 |
| `third_party/mesen2` | `Konami/VRC7.h`, the board — adapted. **Its `Audio/Vrc7Audio.h` is a 91-line shell around `emu2413`, and `Shared/Utilities/` is not in the copied tree** (`third_party/README.md`: only `Core/NES` and `Core/Shared/Audio` were copied). So there is **no OPLL reference implementation in reach**, and the FM core below is built from the documentation alone. |

That last row is why this lot opens two tickets rather than one: the board is documented to the
bit, and the FM core has four quantities no allowed source gives numerically.

## Status

- [ ] step 1 — the VRC7 board, mapper 85
- [ ] step 2 — the VRC7 audio
- [ ] step 3 — save states
- [ ] step 4 — the ROM check: **no mapper-85 image exists** (the sweep is above)
- [ ] step 5 — the suites and the campaign
