# RU-3a — soft reset, frame pacing and GPU queue, APU mixing and MMC5 audio (AMD-044, AMD-052, AMD-054)

Branch `rustupdated`. The application is the Rust workspace `rust/`; the .NET lots V2-5
(`b82d2d3`, AMD-042 + AMD-044), V2-14 (`1a148d7`, AMD-052) and V2-16 (`3d95190`, AMD-054) of
`dotnetupdated` are read as a reference only. Run-ahead (AMD-041) is **not** in this lot (RU-3b).

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | Setup: sources read, requirement map, before benchmarks, amendments copied | done |
| 2 | AMD-044: the soft reset of the core, the command, the input action and the menu | todo |
| 3 | AMD-054: the non-linear mixing, PPU goldens, audio digests refreshed | todo |
| 4 | AMD-054: the MMC5's two squares and its PCM channel, at no cost for other boards | todo |
| 5 | AMD-052: the catch-up limit and the clock reset on a speed change | todo |
| 6 | AMD-052: the GPU frame queue and the anti-aliasing default | todo |
| 7 | Closing: fmt, clippy, crate tests, full PPU goldens, after benchmarks, journal | todo |

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

## Requirement map

### AMD-044 — soft reset

| Requirement | Rust crate / file |
|---|---|
| REQ-CORE-122 core soft reset | `zax-nes/src/nes_core.rs` (`soft_reset`), `cpu/mod.rs` (`soft_reset`), `ppu/mod.rs` (`soft_reset`), `apu/mod.rs` (`soft_reset`), `apu/triangle_channel.rs` (`reset_phase`), `apu/dmc_channel.rs` (`soft_reset`), `input/mod.rs` (`reset`) |
| REQ-CORE-006 amended (frame counter back to 0) | `zax-nes/src/nes_core.rs` |
| REQ-LOOP-072 command | `zax-app/src/emulation/commands.rs` (`Command::SoftReset`), `emulation/host/*` |
| REQ-INP-125 input action | `zax-app/src/input/` (extended actions), key config screen |
| Menu entry and labels | `zax-app/src/ui/` menu bar, `ui/strings` English / French |

### AMD-052 — pacing, GPU frame queue, anti-aliasing

| Requirement | Rust crate / file |
|---|---|
| REQ-LOOP-074 backlog of 2 | `zax-app/src/emulation/frame_pacer.rs` (`MAX_BACKLOG`) |
| REQ-LOOP-075 clock reset on `emu.game_speed` | `zax-app/src/emulation/host/`, `frame_pacer.rs` (`clock_resets`) |
| REQ-LOOP-077 `video.max_queued_frames` | `zax-app/src/graphics.rs` (new GPU frame queue), `zax-render` GL binding, `ui/options.rs` (`EXCLUDED`) |
| REQ-PERS-089 `video.anti_alias` default 1 | `zax-settings/src/table.rs`, `zax-app/src/ui/handheld/defaults.rs` |

### AMD-054 — non-linear APU mixing, MMC5 audio

| Requirement | Rust crate / file |
|---|---|
| REQ-CORE-073 amended, non-linear mixing | `zax-nes/src/apu/mixer.rs` (new), `apu/mod.rs` (`current_level`) |
| REQ-CORE-076 mixer test point | `zax-nes/src/apu/mod.rs` (`mixer_apu`, `mixer_expansion`, `mixer_output`), `zax-harness` |
| REQ-CORE-075 amended, MMC5 audio | `zax-nes/src/apu/mmc5_audio.rs` and `mmc5_pulse_channel.rs` (new), `cartridges/mappers/mmc5.rs`, `nes_bus.rs` |

## Before benchmarks (step 1)

`zax-harness bench-core`, release, x86 desktop, 1200 frames after 120 of warm-up, no profile.
The two MMC5 ROMs are the public test ROMs of `tests/roms` the lot names.

| ROM | core `run_frame` | total |
|---|---|---|
| `mmc5test/mmc5test.nes` (mapper 5) | 1.7909 ms/frame | 1.8442 ms/frame |
| `exram/mmc5exram.nes` (mapper 5) | 2.0429 ms/frame | 2.1991 ms/frame |
| `nestest.nes` (mapper 0) | 0.8695 ms/frame | 0.9371 ms/frame |

## Spec copies

`docs/spec/changes/`: AMD-044, AMD-052, AMD-054, and the three question tickets the .NET lots
wrote whose answers this lot reuses as they stand — Q-078 (soft reset: the PPU's write-inhibit
window is not modelled, the APU frame sequencer is reset), Q-087 (AMD-052: the harness's
anti-aliasing default, the latency criterion's bench, the two instruments), Q-091 (the DC blocker
runs after the resampler, not before). The SPEC chapter edits each amendment implies are applied
in the step that implements the requirement, as RU-2 did.

## Notes

### Reading of the three amendments against the Rust code

**AMD-054, the mixer.** `Apu2A03::current_level` is already lazy: it runs only when a channel
timer moved something or a register was written (`changed || level_dirty`), never per CPU cycle,
and it caches on `level_key`. The non-linear mixing therefore replaces four multiply-adds and a
clamp by two table indices and one addition **in the same place** — the owner's "two lookups per
sample, never per CPU cycle" with the lazy mixing of §5.1 kept exactly as it is. The amendment's
`A` and the float conversion `S / 32768` land there; the DC blocker stays in
`BandLimitedResampler` after the resampling (Q-091), which is also where the Rust port has always
had it.

**AMD-054, how `M` reaches the mixer.** The .NET gives the APU an `IExpansionAudio` reference and
reads it every time the level is recomputed. Rust cannot hand the APU a reference to the
cartridge, and a per-cycle `if board_has_expansion_audio` in `NesBus::tick` is **not** free: the
tick runs ~29 780 times per emulated frame, so three instructions there are ~90 k per frame
against a core of a few million — far over R0's 1 % gate. The design chosen instead:
- `NesBus` keeps a `bool` set once in `attach`, and the push happens inside `catch_up_apu` /
  `sync_apu`, which in the frame loop (`defer_ppu`, the only path gameplay takes) run **rarely**
  for a board without expansion audio. Nothing is added to `tick` itself.
- A board with expansion audio keeps `apu_quiet = 0`, so the APU never lags on it and the `M`
  pushed is always the current cycle's. The quiet-cycle skipping of §5.1 is untouched for every
  other board.
- `tick` steps the cartridge **before** the APU catch-up instead of after (the .NET made the same
  swap). That is a move, not a new branch. The two do not interact other than through the new
  `M`: the DMC DMA check stays after both. The goldens are the proof.

**AMD-052, the catch-up.** `MAX_BACKLOG` is used for two different things in Rust: the spec limit
of REQ-LOOP-012 and the clamp of the Rust-only `emu.catch_up` setting, whose domain is 0 to 5 and
whose handheld default must stay 5 (owner decision; perf history §7 "catch-up 2 on the handheld:
worse than both None and 5"). The two are split: the spec limit becomes 2 (REQ-LOOP-074), the
setting's domain stays 1 to 5.

**AMD-052, the GPU queue on handhelds.** Owner decision, recorded here as an assumed divergence
**(D)**: on PortMaster and Android the value 2 issues **no fence call at all** and keeps today's
behaviour, because the Mali driver keeps three frames of its own and bounding that queue could
cost frames. The value 1 works on every build.
