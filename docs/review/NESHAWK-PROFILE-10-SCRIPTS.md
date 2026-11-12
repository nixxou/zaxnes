# The Lua instruments of lot NESHAWK-PROFILE-10

Both live in `G:/3DSen/ext-tools/scripts/` and write into `G:/3DSen/test-movies/bizhawk-runs/`.
Run as

    G:/3DSen/ext-tools/BizHawk-2.11.1/EmuHawk.exe --movie="<bk2>" --lua="<script>" "<rom>"

with the *Kirby* `.bk2` (`keyword-kirbysadventure-noabilities.bk2`) and
`Hoshi no Kirby - Yume no Izumi no Monogatari [Japan][!!].nes`.

## `q185.lua` — the four marks over BizHawk frames 25 to 9 360

`q183.lua`'s four once-a-frame cycle marks, same file format, over the window the three shipped
items reach: the NMI vector's target `$C341`, its `RTI` at `$C3A1`, `$C06E` (`STA $39`, the last
instruction before the spin loop) and `$C074` (`PLA`, the loop's exit). `$C06E` and `$C074` are
each executed **exactly once a frame**, which is what makes four `event.onmemoryexecute`
registrations on four **named** addresses enough — no `onmemoryexecuteany`.

**Registered late**, at frame 25: the boot runs at full speed with no Lua call at all, and the file
starts where the comparison does. 48 s, 818 KB, `bizhawk-runs/q185.txt`.

Block numbering is `q182d`/`q182e`/`q183`'s: `## frame f` holds **this tree's frame f + 1**.

The Zaxnes side, and the comparison:

    zax-harness frame-trace <movie> <rom> --frame 26 --frames 9336 --profile neshawk \
        --event-log zx185.txt --event-pc C3A1 --event-pc C06E --event-pc C074

19 s. Both instruments report an *execute* at the cycle the instruction begins on, and BizHawk's
`emu.totalexecutedcycles()` is a **1-based** count where this core's `CYC` is the **0-based** index
of the cycle about to run — so **the two machines in step read a residual of −1**, and that is the
figure to look for, not 0.

## `q186.lua` — the whole bus stream over BizHawk frames 25 to 27

`q184.lua` over the three frames that straddle the first difference. Same format,
`<cycle> r|w|x <addr> <val>`, with `event.onmemoryread`/`write` on `nil` and
`event.onmemoryexecuteany`. 5 s, 2.1 MB. **An intermediate**: read the window out of it and delete
it.

The Zaxnes side is `frame-trace --frame 26 --frames 3 --out`, and the two are aligned **execute to
execute**, which is what names the first instruction whose residual leaves −1.

## What they measured, in one line each

| | |
|---|---|
| `q185.txt` | the four marks hold −1 through BizHawk frame **26** and leave it at frame **27**; 1 866 of the 9 336 frames read −1 on all four, in runs |
| `q186.txt` | from frame 26's `$C06E` the two machines run the **same 8 363 instructions at exactly −1** and part on whether the vblank NMI displaces the `BEQ $C070` — BizHawk's own records show the taken branch's three cycles (`C072`, `C073`, `C074`) and then the interrupt sequence, where this core's seven cycles run **in place of** the branch |

## The two reference files are kept

`bizhawk-runs/q185.txt` is a reference and not an intermediate — it is the four marks over 9 336
frames and no later lot needs to take it again. `bizhawk-runs/q186.txt` is the intermediate.
`bizhawk-runs/README-references.txt` lists the per-frame work-RAM references; **never produce
another one of those**.
