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

D2c (2026-11-16): the PAL 2C07's composite signal, measured on the Visual 2C07 netlist
(`ext-tools/visual2c07-tools/benchvid.cpp`: rendering off, each palette colour and emphasis value
put on screen by register writes on the pins, the video DAC's level select read every master
half-clock; 64 colours x 8 emphases x 2 lines; `vidmodel.py` reproduces all 1 024 records):

* colour `h` is high for six of the 12 subcarrier phases from `h` on odd lines and from `5 - h` on
  even ones (the phase mirrored every other line, PAL's V switch); the burst six phases from 10 on
  odd lines, 7 on even (90 degrees apart, the PAL swinging burst);
* emphasis bit 0 attenuates six phases from 5 on odd lines and 1 on even, bit 1 the reverse, bit 2
  from 9 on both; `$xE` / `$xF` output the blanking level and take no emphasis;
* a pixel is 10 half-clocks, a line 3 410 (2 phases more than whole cycles), a frame of 312 lines a
  whole number of cycles: the colour pattern stands still from frame to frame (no dot crawl);
* the picture starts with two border pixels at the blanking level on each side (252 coloured).

`composite::pal_signal`, `RowDecoder::decode_row_pal` (the set's decoder: U and V against the
burst's mean axis, V switched every other line, then the PAL-D delay line averaging each line's
chroma with the one above), selected by `NesCore::video_phase` returning `PAL_VIDEO` on the 2C07;
application and harness (`ZAXNES_COMPOSITE=1`, and `ZAXNES_REGION=pal` to force the region of a
dump whose header says NTSC). Same option and cost as D2b (`video.composite_signal`, off).

Choices and limits: the netlist gives the DAC's select, not the ladder's voltages, so the 2C02's
measured levels are used; which line the set takes as V-inverted is read from the burst's swing,
and only one of the two readings gives sensible hues (the other mirrors every hue about the U
axis), recorded as such; the resulting hues relative to the burst are the 2C02's turned by -15
degrees. The same bench on the 2C02 netlist puts colour `$x8` in phase with the burst, as nesdev
says, which fixes the bench's direction of rotation. The two border pixels are not drawn by the
core's picture (it keeps 256 columns). Dendy keeps the NTSC generator (its PPU is not measured).

## D3 design (2026-11-16): the picture handed over as the PPU draws it

What the console does: the PPU sends each line to the television as it draws it, so the bottom
of the picture reaches the screen ~16 ms after the top, and a button read during the frame can
show on lines drawn after the read. What Zaxnes does with B5: the machine runs in real time
between two refreshes and the finished frame is shown at the next refresh; a line the PPU drew
early in the frame waits for the rest (up to one refresh) before anyone sees it.

D3 hands the screen the lines already drawn before the refresh, "beam racing" as WinUAE and
GroovyMAME do (frame slices):

1. The refresh is split into N slices (default 4, at most 8). During B5's wait, after each
   machine slice, the lines the PPU has finished are uploaded to the texture (the dirty band only).
2. With vsync off and tearing allowed (GL swap interval 0 / wgpu `Immediate`), each slice is
   presented when the display's own scanout is about to reach it: the tear line then falls in the
   part of the screen that does not change, and is invisible.
3. The scanout position comes from the last refresh's timestamp and the period (B5 already keeps
   both: `last_swap`, `paced_budget`); where the platform reports it exactly (Windows:
   `D3DKMTGetScanLine`, DXGI frame statistics) the estimate is corrected.

Placement: an option off by default (`video.beam_racing`), Fidelite+, only with `emu.realtime_input`
on, a display whose refresh is the game's (60 Hz for NTSC, 50 Hz for PAL), and the 2D picture: the
3D scene is built from whole frames and keeps them. Not with run-ahead (which shows a future frame).

Risks to settle before code: (a) the driver may queue presents and defeat the timing (measure
with `padbench`'s timestamps and, ideally, a photodiode); (b) composition (Windows DWM, Wayland)
may force vsync on a windowed surface: fullscreen exclusive or a flip-model swapchain with tearing
support is required, so the option says when it cannot run; (c) the per-slice upload and present
cost (a GL band upload ~0.1 ms, four presents a frame) against the +1 % budget, which only applies
to the default path (the option is off).

Judge: the latency from a button to its first visible pixel, measured with `padbench`'s timer on
a test ROM that changes the backdrop when the button is read: today ~1 refresh + scanout; with D3
the scanout part only. Expected gain: about half a refresh on average (8 ms at 60 Hz).

Judge ROM (2026-11-16, `ext-tools/lagtest-rom/lagtest.nes`, outside the repository like the OAM
measurement ROM): a white screen that polls the pad all frame long (a strobe and read every ~30
CPU cycles, no NMI) and sets the three emphasis bits while A is held, so the picture darkens from
the line being drawn when the press is seen. Checked in the harness (`--press 5-60:a`: dark;
without: white). With the real-time pad on, a press lands mid-frame and the split line shows the
line it was read on; a camera or light sensor against padbench's timestamps gives the
button-to-photon latency, before and after D3, and on a console.

D3a (2026-11-16), the step before beam racing, without tearing: `emu.late_present` ("Late
present", off). With the real-time pad, `Application::frame` used to draw the picture first and
then spend the wait before the refresh running the machine, so the refresh showed a machine
nearly one refresh old. With the option the wait comes first (`run_the_realtime_pad(margin)`) and
the scene and draw follow, so the refresh shows the machine as it stood just before it, as
RetroArch's frame delay does. The margin is the last frame's scene-and-draw time plus 2 ms, at
least the pad's 2.5 ms, at most three quarters of the refresh. Expected gain: up to one refresh
(~10 ms at 60 Hz on average less the margin), no tear line; risk: a draw that overruns the margin
misses a refresh, hence off by default. The frames finished during the wait count for this
displayed frame (3D interpolation, rate log). Judge: `lagtest.nes` with the camera, as for D3;
not measured yet (needs the owner's pad and screen). Smoke-checked (2026-11-16, release build,
scratch user data, ``--vsync 1``, ``lagtest.nes``, RTX 3050, 60 Hz): 600 frames in 10.5 s with the
option on and off alike, 595 emulated in both: no refresh missed, the pacing unchanged.

