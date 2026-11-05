# Lot ZR-2 — the DMA halt's re-read must not reach the PPU from the hack zone either

Baseline: `1f3d06c1` (ZR-1's end). Reference picture: `c4577878` (MF-4a 5/5).

ZR-1 removed the **CPU's own** read of a zone address and restored the picture up to MF-4d step 5.
What is left is the **halt's and the dummy cycles' re-read** of the same address inside
[`NesBus::run_dma`] and [`NesBus::dmc_only_dma`], which R0's ZR-1 instruction said to keep. That
instruction was wrong: the hack zone does not exist on real hardware, so no hardware behaviour is
preserved by reading `$2002`/`$2007` there. The rule becomes **code running from the zone never
reaches the PPU registers through the CPU's fetch path — not by the fetch, and not by any DMA halt
or dummy re-read of the fetch address.** The hack's own explicit loads and stores are not fetches
and are out of the rule.

## Steps

1. Confirm: count the PPU-register reads `run_dma` makes on a zone address, SMB3 and Ninja Gaiden III.
2. The fix: the re-reads take the zone path too, and the MF-3b check on the five functions.
3. The unit tests: an OAM-halted and a DMC-halted zone fetch during rendering move nothing.
4. Re-pin the harness regression test of ZR-1 step 4 on the restored picture.
5. The pictures: frame 1 500 against `c4577878`, then every 5th frame of both games.
6. The suites, the campaign, golden-core and the instruction count.

## Step 1 — R0's diagnosis is confirmed, with the numbers

### The two commands

ZR-1's, run on a build of the branch tip:

    zax-harness render --rom "<demo/roms>/Super Mario Bros. 3 (USA).nes" --run 1500 \
      --press <R0's smb3 script> --poke 1000:7CFF=01 --advanced --widescreen 128 \
      --out <dir> --png-nes <file>.png

    zax-harness render --rom "<demo/roms>/Ninja Gaiden III-TASoD [USA][!].nes" --run 700 \
      --press <R0's ng3 script> --advanced --widescreen 128 --out <dir> --png-nes <file>.png

### The counter

A throwaway `[AtomicU64; 12]` in `nes_bus.rs` and a throwaway `zr2_zone` flag that
`halted_zone_read_cycle` raises around its `run_dma` / `dmc_only_dma` call, incremented at **all
five** sites where either of those two reads the halted CPU address — `run_dma`'s halt read and
its idle-cycle repeat, `dmc_only_dma`'s halt read, and `dmc_fetch_cycles`' dummy and alignment
reads — indexed by the register the address decodes to, printed at the end of `rom::capture` and
then reverted. It is in no commit, and the frame-1500 PNG it produced is byte-identical to ZR-1's
(`a1516fbf96fa8de7…`), so the instrument changes nothing.

| at the branch tip, with the hack | SMB3, 1 500 frames | Ninja Gaiden III, 700 frames |
| --- | ---: | ---: |
| zone fetches a DMA unit **halted** | 1 077 | 1 312 |
| **re-reads of the fetch address** those halts made | **3 166** | **3 810** |
| of them absorbed by the extension's `$3000`-`$3FFF` window | **0** | **0** |
| of them reaching `Ppu2C02::read` | **3 166** | **3 810** |

and by register:

| | `$2000` | `$2001` | `$2002` | `$2003` | `$2004` | `$2005` | `$2006` | `$2007` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| SMB3 | 346 | 460 | **441** | 348 | 389 | 384 | 356 | **442** |
| NG3 | 502 | 413 | **432** | 440 | 510 | 482 | 480 | **551** |

**R0's diagnosis is confirmed.** ZR-1 took away the CPU's own read and left the DMA's: 3 166 reads
on SMB3's 1 500 frames, every one of them reaching `Ppu2C02::read`, 441 of `$2002` clearing `w`
and 442 of `$2007` moving `v` and the read buffer — and, since MF-4d step 6, moving `v` by a coarse
X and a Y at once when they land during rendering. Nothing comes from elsewhere: there is no
remaining change to find.

Why ZR-1 counted 3 301 and this counts 1 077 halts: ZR-1's counter sat in `zone_read_cycle` on the
`dma != 0` side, which also counts the fetches where a **held** DMC request makes `dma` non-zero
and no unit may take the bus. This one counts after `halted_zone_read_cycle`'s early return, so it
counts the fetches a unit actually halted. 3 166 / 1 077 ≈ 2.94 reads a halt, which is the halt,
the dummy and (usually) the alignment cycle of a lone DMC fetch.

### The picture that is left

Frame 1 500, the reference `c4577878` (`a923573ab7b9b3f8…`) against the tip
(`a1516fbf96fa8de7…`): **51 scanlines differ, y = 169 to 220**, and the harness's own summary line
counts **22 colours against 21**. That is the same footprint, to the scanline, that ZR-1 step 1
measured for the whole regression — R0's three symptoms by eye (the band of sky cutting the ground,
the truncated right platform, the overflowing status-bar band) are what is left of it.
