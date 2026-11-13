# FIDELITY — fidelity to the console and its documentation (progress)

Owner, 2026-11-13: work on the gaps found against Mesen, the goal being the console and the
documentation rather than Mesen itself, and `AccuracyCoin` passing entirely if possible.
Branch `feat/fidelity`, from `dev` at `27e2c3e2`. Study behind it: `docs/review/NMI-MESEN.md`
(branch `feat/core-tiers`).

Reference before the lot: test ROMs 212 pass / 7 fail / 1 unsupported / 111 manual, AccuracyCoin
129/147 (132 with `ppu-exact-sprite-eval`), golden-core `bc8497d3…`.

| Step | What | AccuracyCoin | Test ROMs | golden-core |
|---|---|---|---|---|
| 1 | Secondary OAM not cleared on the pre-render line (nesdev, "PPU sprite evaluation") | 129 | same | same |
| 2 | `$4015` bit 6 raised for two cycles even with the frame IRQ inhibited (`Frame Counter IRQ` I-M) | **130** | same | same |
| 3 | A DMC fetch caught by a `$4015` switch-off still halts the CPU for 1, 3 or 4 cycles (`Explicit DMA Abort`; `DMA_DMC_GHOST` in `nes_bus.rs`) | **131** | same | same |
| 4 | The fetch a one-byte sample asks for and loses costs one halted cycle, none on a write cycle (`Implicit DMA Abort`) | **132** | same | same |
| 5 | A DMA read that also reaches `$4016`/`$4017`: the register's answer is on the bus for the other read, and only the driven pins conflict (`APU Register Activation` 6 and 7) | **133** | same | same |
| 6 | A DMC fetch that also reaches `$4015` leaves the address's own byte on the pins; a controller port keeps its own low bits in a conflict (`DMC DMA Bus Conflicts`) | **134** | same | same |
| 7 | `ppu-exact-sprite-eval` in `default` (measured +0.04 % to +0.13 % at MF-4e, inside the per-release budget; to re-measure) | **137** | same | same |
| 8 | `$2004` during rendering: the secondary OAM is read back on even dots once the copy is over, full or not, and dots 321-340 answer from its index 0 (`$2004 Stress Test`) | **138** | same | same |
| 9 | OAM row corruption when the rendering is switched off inside a line (`OAM Corruption`), as a core setting **off by default**: `NesCore::set_oam_corruption`, `ZAXNES_OAM_CORRUPTION=1` for the campaign. On, it gives **139** and golden-core is unchanged, but blargg's `sprite_overflow_tests/5.Emulator` fails its test 3 (rendering switched off at line 126 dot 8, row 5 marked) — the model is incomplete, as Mesen's own note says | 138 | same | same |

## Rows still failing (default build, 9)

`$2007 Stress Test` 2, `OAM Corruption` 2, `ALE + Read` 2, `BG Serial In` 2,
`Hybrid Addresses` 2, `Stale BG Shift Registers` 3, `Stale Sprite Shift Regs` 3, `Frozen OAM2
Increment` 2, `Misaligned OAM2 Address` 4. All nine are PPU rows about what the pipeline does when
rendering is switched on and off inside a line, or about the exact dot of a `$2004`/`$2007` access.

## Still to do outside AccuracyCoin

* Power-on: the PPU "at the top of the picture" (nesdev, "PPU power up state") instead of the
  start of vblank; moves every game's timeline (golden-core, TAS sync, widescreen recipes).
* The sprite-overflow bit of `$2002` in *The Legend of Zelda* (set in Mesen, clear here).
* `$2002` open-bus bits and `$2007` reads during rendering: differ from Mesen, both follow test
  ROMs; to be settled against the documentation.
* Performance: every step measured before it stays in `default`.
