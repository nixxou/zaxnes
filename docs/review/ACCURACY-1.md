# ACCURACY-1 — who is right where Zaxnes and NesHawk disagree

Lot NESHAWK-REWRITE's capture-parity step found the two engines differing on *Super Mario Bros. 3*,
both *Zelda II*s, *Excitebike* and ten `holy-mapperel` MMC1 ROMs. Two emulators that disagree say
nothing about which is wrong, so this lot asks a third: **MesenCE 2.2.1**, the most accurate of the
three on AccuracyCoin (141/147, against NesHawk 130 and Zaxnes 128). No line of `zax-nes` changes.

## Method

Each game runs with no input for 1 500 frames on the three machines: Zaxnes (`zax-harness`), the
NesHawk port (`movie-sync --engine neshawk`, byte-exact with BizHawk 2.11.1 on Kirby) and MesenCE
(`Mesen.exe --testrunner` with a Lua script; `Debug.ScriptWindow.AllowIoOsAccess` lets the script
write its log). Compared: the work RAM outside the scratch `$00-$1F` and the stack page (the two
artefact classes `ram_trace` already sorts out), the 2D picture, and for SMB3 the position of the
MMC3 IRQ handler's first instruction.

**One frame of offset.** NesHawk runs a dead start-up frame (`NewDeadPPU`) that Zaxnes and MesenCE
do not: Zaxnes's frame N is NesHawk's frame N−1. With that alignment SMB3's work RAM is identical
on the two at frames 299-302. The capture-parity measurement compared the same frame number, so a
part of what it called "emulation" differences was this offset.

**New instrument: `zax-harness exec-trace`.** Powers a ROM on with no input and prints, each time
the next instruction is at one of the given addresses, the frame, scanline, dot and CPU cycle; with
`--follow n` it also disassembles the n instructions after each hit with their positions. It steps
instructions by hand through `NesCore::cpu_and_bus_mut` as `frame-trace` does, so the PPU is never
deferred and the positions are exact; it changes nothing in the core.

## Results

| Case | Measured | Verdict |
|---|---|---|
| SMB3, MMC3 IRQ | Cycles from NMI entry (`$F486`) to IRQ entry (`$F795`) over 1 488 frames: Zaxnes and NesHawk have the **same distribution** (mean 24 297.45, range 24 294-24 301); MesenCE mean 24 298.12. Handler at line 192, dots 285-296, on Zaxnes and MesenCE | **no IRQ timing difference** |
| SMB3, status bar | The bar's border lines are one line lower on Zaxnes (195/198/209) than on NesHawk and MesenCE (194/196/202). The IRQ handler does `LDA $2007` at line 193 dot ~278 with rendering on. On hardware that access also runs the fetch pipeline's coarse-X and Y increment; Zaxnes implements this behind the off-by-default feature **`ppu-2007-render-read`** (lot MF-4e-gate, Q-137/Q-139). With the feature, SMB3 is identical to NesHawk (RAM and picture) at frames 60, 300 and 1200 | **Zaxnes wrong by default; the fix exists** |
| Zelda II (USA, Europe), picture | 93-94 rows differ without the feature, 0-1 with it | **same cause** |
| Zelda II (USA, Europe), RAM | Zaxnes = MesenCE on **every** compared frame 400-1200 (0 bytes); NesHawk 3-12 bytes away (sound counters ±1) | **NesHawk wrong** |
| Excitebike | Zaxnes = MesenCE (0 bytes) to frame 600, 1-2 bytes to ~1000, 17-21 after; NesHawk 2-9 bytes from the start and its demo diverges near frame 825 | **NesHawk wrong** (Zaxnes's late drift from MesenCE is small and still to explain) |
| MMC1 `$E000` bit 4 (10 `holy-mapperel` ROMs) | The ROMs check that bit 4 disables PRG-RAM on MMC1B boards (nesdev: "MMC1B and later: PRG RAM chip enable"); Zaxnes passes them, NesHawk fails them because `SxROM.cs` honours `wram_disable` only when `_is_snrom` | **NesHawk wrong**, by the test ROMs' own verdict |

The MesenCE picture comparison is not used: its palette differs and a colour-mapping comparison
was inconclusive. The work RAM is unambiguous and is what the verdicts rest on.

## What this means for Zaxnes

* The one real Zaxnes defect found is **`$2007` access during rendering**, already written and
  measured (+0.000 % to +0.005 % on PR-1's set) but kept out of `default` because it moves the
  picture of SMB3 and Zelda II against v0.4.2 — the split line under the widescreen hack.
  It is also AccuracyCoin's `$2007 read w/ rendering`. Owner's decision: Standard (re-validate the
  SMB3 and Zelda II packs) or the Fidélité+ core only.
* The other capture-parity differences are NesHawk's, plus the one-frame offset above, which a
  future comparison must apply.
* Still open: Excitebike's 17-21 bytes of drift from MesenCE after frame ~1000.

Raw measurements (not committed): the session scratchpad's `acc/` directory — `smb3-*.txt`,
`mesen-ram*`, `scan-*`.
