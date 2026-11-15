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
