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

## Rows still failing (default build)

`$2004 Stress Test` 2, `$2007 Stress Test` 2, `ALE + Read` 2, `APU Register Activation` 6,
`Address $2004 behavior` 8 *, `Arbitrary Sprite zero` 2 *, `BG Serial In` 2, `DMC DMA Bus
Conflicts` 2, `Frozen OAM2 Increment` 2, `Hybrid Addresses` 2, `Misaligned OAM behavior` 1 *, `Misaligned OAM2 Address` 3, `OAM Corruption` 2,
`Stale BG Shift Registers` 3, `Stale Sprite Shift Regs` 3.
(* pass with `ppu-exact-sprite-eval`, out of `default` for its cost.)

## Still to do outside AccuracyCoin

* Power-on: the PPU "at the top of the picture" (nesdev, "PPU power up state") instead of the
  start of vblank; moves every game's timeline (golden-core, TAS sync, widescreen recipes).
* The sprite-overflow bit of `$2002` in *The Legend of Zelda* (set in Mesen, clear here).
* `$2002` open-bus bits and `$2007` reads during rendering: differ from Mesen, both follow test
  ROMs; to be settled against the documentation.
* Performance: every step measured before it stays in `default`.
