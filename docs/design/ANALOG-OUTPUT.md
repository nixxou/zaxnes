# ANALOG-OUTPUT — Fidélité+ part D (design, 2026-11-15)

Part D of the Fidélité+ goal (`docs/design/HALF-CYCLE.md`, "The goal"): what leaves the console's
connectors, sound and picture, as the console makes it. Placement by the owner's rule: what costs
per sample or per pixel goes behind an option; what is free goes in the common base.

## Sound: where Zaxnes stands

* The APU already runs per CPU cycle and its output is band-limited step synthesis
  (`apu/band_limited_resampler.rs`): every level change lands at its exact CPU cycle in the output,
  so "the APU at the CPU rate" is not an approximation today.
* The analog path of one measured console is an option (`audio.console_profile`, BeesNES's 14
  profiles, FIDELITY-LEADS E1). E1 lists what is still approximate:

| E1 | Approximation | Fidélité+ (D1) |
|---|---|---|
| (1) | the low-pass at the output rate, its zero (144-189 kHz) dropped | the low-pass on the band-limited signal before decimation, pole and zero (per-step cost: option) |
| (3) | HPF0 after our resampler, not inside it | HPF0 in the resampler's kernel path, as BeesNES's sample box (a few samples of phase) |
| (4) | expansion audio (VRC6, MMC5, N163, 5B, VRC7, FDS) added after the gains, not filtered per console | BeesNES's order: mapper audio between HPF1 and HPF2 (`PostHpf`, `LSNApu2A0X.h:1919-1931`) |
| (5) | the filters' state not in the savestate | in the state block (free per sample; a format change, so with a version) |
| (2) | the profile's overall volume (0.34-4.5) not applied, only its sign | stays: our output is normalised to full scale, applying it would clip |

Judge: BeesNES's own claim is checked against recordings with MDFourier. D1's judge is a
bit-for-bit comparison of our filter chain against BeesNES's chain (`Src/Apu/LSNApu2A0X.h`, MIT)
fed the same APU levels, per profile, plus the existing audio tests (default "none" unchanged).

## Picture: where Zaxnes stands

* The core gives palette indices and emphasis bits per pixel; colours come from a palette, and
  the CRT look is the shaders' lot (FIDELITY-LEADS H, `feat/librashader`), not the core's.
* D2 would generate the composite signal itself (the 2C02's 12 phases per pixel, levels, colour
  burst, the colour artefacts and the dot crawl of odd frames), in the core's Fidélité+ path, and
  decode it like a television (the NTSC decoder's comb / notch). References: nesdev "NTSC video",
  BeesNES's NTSC/PAL filters (`Src/Filters`), the 2C02 netlist's video output pins (measured
  levels).
* D3, latency: hand the picture to the display line by line as the PPU makes it (a "beam
  racing" presentation), so the console's own sub-frame latency is reproduced on a fast display.
  Needs the real-time loop of B5.

## Plan

| Step | What | Placement |
|---|---|---|
| D1a | expansion audio between HPF1 and HPF2 per console (E1-4) | free per sample: common base, inside the profile option |
| D1b | filter state in the savestate (E1-5) | common base, state version bump |
| D1c | HPF0 inside the resampler path (E1-3) | inside the profile option |
| D1d | low-pass with its zero before decimation (E1-1) | Fidélité+ (per-step cost) |
| D2 | composite video from the PPU's signal, TV decoder | Fidélité+ (per-pixel cost) |
| D3 | line-by-line presentation | with B5 |

D2b step 1 (2026-11-15): colour emphasis reaches the 2D picture at all (it never did; FIDELITY-LEADS
H3), common base since it costs nothing per pixel: a log of the `$2001` writes that change the bits,
a plane for frames with emphasis, the palette scaled by the composite decoder's ratio, in the GL
and wgpu uploads and the harness's flat PNG. Next, D2b step 2: the picture through the generator
and a line decoder (colour artefacts, dot crawl), Fidélité+ only.

D2b step 2 (2026-11-15): `video.composite_signal` (Fidelite+, off) decodes the 2D picture from the
generated signal (`composite::RowDecoder`: luma over one subcarrier cycle, I/Q over two). The hue
reference was fitted to TriCNES's plain palette (+45 degrees, error 627 -> 165 over the coloured
entries; at 0 the sky of Super Mario Bros. decoded magenta) and pinned by a test. Cost of the decode,
measured in release: 7.1 ms a frame first, 1.2 ms with tables for the levels and the demodulation;
the standard path pays nothing (the option is a branch per displayed frame). Harness:
`ZAXNES_COMPOSITE=1` writes `<scene>-composite.png` next to the flat picture.
D1b (2026-11-15): the profile filters' memory in the savestate (formats 14 / 15, FIDELITY-LEADS
E1 (5)); and the filters are no longer rebuilt at rest by every ratio recomputation (a click at each
fine pacing adjustment of the host before). Standard path: untouched (no profile, format 10 / 11).

D1c (2026-11-15): closed without a code change — BeesNES's HPF0 is at the output rate after its
sinc, like ours after the band-limited synthesis, same formula; the order against the low-pass is
immaterial for linear filters (test, < 1e-12). It found D1d's real content: BeesNES's CPU-rate
low-pass is never off (disabled or above Nyquist: `output / 2 + 100` Hz) and one extra pole at that
corner follows it on every profile; ours drops both, and the three zeros.

D1d (2026-11-15): the low-pass with its zero and BeesNES's output pole at its fitting rate
(22 150 Hz), as one filter fitted at the output rate (FIDELITY-LEADS E1-1): within 0.002 dB of the
model in band, 14 samples of delay, per-sample cost inside the profile option (not per step, so no
Fidelite+ gate needed). Savestate filter block 432 bytes (format 14 / 15, unreleased).

D1a's cost on the standard path (callgrind, clean builds against 6c9cb163, PR-1 set): first
+0.89 % on `mmc5` (the profile test in the setter the MMC5 calls on every level change), then
+0.417 % with that branch out of line, and **-0.057 %** at 7edd528e, where the setter is back to
its pre-profile form and the profile's expansion step is made on the mix's cache miss the move
forces (every other ROM of the set -0.09 % to -0.19 %).
