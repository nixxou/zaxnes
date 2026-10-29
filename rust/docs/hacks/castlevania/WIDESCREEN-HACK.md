# Castlevania (USA): the widescreen hack

This document describes completely the modification of *Castlevania (USA)* and of its *Rev 1*
that Zaxnes ships as two built-in patches (advanced mode): one source, built for each. The
appendices hold the new code and the build script, which writes every hook after checking the
bytes it replaces: with them both ROMs can be rebuilt identically.

The background of the margins is not the hack's work: the emulator's Castlevania margin decoder
reads the rooms' maps and the stages' blocks in PRG-ROM (see
[`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md), section 4), and widens the
unpatched game on its own. The hack is the game's **advanced mode**: the objects and the enemies,
which live and show in the margins, and the options.

---

## 1. Identity

| | |
|---|---|
| Source ROMs | *Castlevania (USA)* and *Castlevania (USA) (Rev 1)*, UNROM (mapper 2), 128 KiB PRG (8 banks of 16 KiB, bank 7 fixed at `$C000`-`$FFFF`, the others switched at `$8000`), 8 KiB CHR-RAM |
| Source data (iNES header excluded) | base build CRC-32 `0AC1AA8F`, Rev 1 CRC-32 `B668C7FC` |
| Target data | base build CRC-32 `66E5E83F`, Rev 1 CRC-32 `8C4BB48C` (the hack's version in savestate headers; `tools/build.py` prints it) |
| Changes | 2 KiB of code at PRG offset `$15F00`-`$166FF` (bank 5 from its page `$1F`, `$FF` fill in both builds), hooks in banks 0, 6 and 7; size and layout unchanged |
| Patches | `rust/crates/zax-nes/patches/castlevania-usa-widescreen.bps` and `castlevania-usa-rev1-widescreen.bps`, made against the data without header |

The two builds share their data banks 0-4 byte for byte; their code (banks 5-7) was rebuilt, so
the routines the hack calls and the hooks of the fixed bank lie elsewhere in each (Rev 1's are
`$24` or `$25` bytes further, the NMI's 4). `src/wide.s` takes Rev 1's addresses with the ca65 symbol `REV1`; `tools/build.py` tells
the build by the data CRC-32 and picks its hooks. The hooks of banks 0 and 6 are the same in both.

The ROM keeps its size and layout: a state made without the hack restores with it as it is.

---

**Hack pack.** Zaxnes ships this hack as a hack pack,
`rust/crates/zax-nes/patches/castlevania-usa-widescreen`, `castlevania-usa-rev1-widescreen`,
`castlevania-europe-widescreen`, `castlevania-europe-ntsc-widescreen` and
`akumajou-dracula-japan-widescreen.zxpk` (`tools/build.py <rom>
--pack <out.zxpk> [--ntsc]`, with `ZAXNES_TOOLS` set to Zaxnes' `rust/tools`; `docs/HACK-PACKS.md`,
`WIDESCREEN-EXTENSION.md` 7.1). The packs are format 2, written by Zaxnes' shared builder
(`hackpack.py`): UxROM, each hook a 16 KiB bank and a CPU address. The widescreen is in four
groups: "core" (Base), and, each needing it, "edge spawns", "platforms" and "copies" (the four
places an object takes its parent's x: a ROM hack that put something else at one of them, The Holy
Relics, takes the rest). Checks (`CHECKS`) hold the game's code the hack's code relies on: the
state dispatch's start, the routine moving an object by the scroll, the reads of the rooms'
lengths and of each stage's graphics block, the graphics loader's start, the spawners' loop.

One source makes the four builds' packs: the USA and its Rev 1 (NTSC), the Japanese build
(Akumajou Dracula, `-D JAPAN`: its code banks rebuilt, most routines `$60`-`$70` further, a shorter
state dispatch, a hit's damage read from a table, `SBC $E7C3,Y`; the same RAM and states), and the
European build
(`-D EUROPE`, its code banks rebuilt from the Rev 1's, most routines `$0C` further; the USA data
banks and RAM). The European pack is PAL; the same pack as an NTSC one (`--ntsc`, its own entry)
takes the ROMs of the European code whose header says NTSC: the GoodSet dumps and translations,
the European Virtual Console, and ROM hacks built on that code (Horror of Dracula, Overflow
Darkness, Prince of Darkness, Reborn, Hell's Fury, The Holy Relics). Against a set of 260
Castlevania ROMs: 241 take a whole pack, 8 in part (3 The Holy Relics without "copies", 5 trainer
builds without the aid), 11 none (the MMC3 and MMC5 conversions, The Red Wizard, PlayChoice-10 and
VS. System). Run 2400 frames with inputs, each accepted ROM plays as it does without the pack, and
its frames go on (The Holy Relics, whose title these inputs do not pass, runs as it does without
the pack). The code goes to the extension's memory instead of the game's free space described here (the source
assembled for `$2000`, without the reset hook and the NMI's unlock: Zaxnes unlocks the extension
and shows the code in its execution zone from power-on, `$500B` = `$80`, to instruction fetches
only; the code reads no data of its own, its variables are in the extension's RAM at `$5200`), and the hooks, each with
the game's bytes it replaces, are checked and applied by Zaxnes; the ROM keeps its size (nothing
written in its free space). The option "Invincible" is a component of its own (`LDA #$02` in place
of the health subtraction), applied with or without the advanced mode. The level selector needs the
hack's code: it stays with the widescreen. The BPS patch described here is kept to compare.

## 2. The extension and the code's place

The fixed bank has no room for code that would unlock the extension. The hack's code is the free
end of PRG bank 5 instead, which the extension's PRG window shows at `$5800`-`$5FFF`, and the
built-in patch opens that window from power-on (`BuiltinPatch::boot_window`, `$5008 = $85`,
`$5009 = $1F`): the hack needs Zaxnes, and its code runs whatever bank the game maps at `$8000`.

Everything the game does runs in the NMI; the main loop only stirs the random number
(`idle_loop` `$C030`-`$C03E`, in both builds). The NMI's call of the game's state dispatch
(`JSR $C1A1`, Rev 1 `$C1E4`) becomes `JSR wide_frame`, which counts the frame, writes the settings
(`wide_settings`), runs the level select and jumps to the dispatch.

`wide_settings`, each frame: `M = min($5002, 248)` (`WV_M`), `$5003 = M`, `$5004 = $5005 = 0` (no
nametable columns: the decoder draws the background), `$5007 = $02` (sprites free of those
extents), `$5006 = 255` with the option "No slowdown" (`$500A` bit 2), else 0. Then:

* the **objects' margin** `MO = min(M + 16, 120)` (`WV_MO`, 0 when M is 0) and `256 - MO`
  (`WV_MOL`): how far past an edge an object lives. 120 keeps the two sides of the 9-bit x apart
  (below 128);
* the **9-bit objects' margin** `FAR = max(MO, 64)` (`WV_FAR`) and `256 - FAR` (`WV_FARL`): the
  game keeps some objects 64 pixels past the edges already, never less;
* the **room's end** in screens (`WV_END`): a screen past the camera's last, read from the
  lengths table (`$FB8C`, Rev 1 `$FBB0`) as the game does at `$CFF0`: the high nibble of the
  stage's byte for room 0, the low one for room 1.

Variables (extension RAM): `WV_M $5200`, the screen x of the object being drawn
`WV_XLO $5201`/`WV_XHI $5202` (16-bit), `WV_FRAME $5203`, `WV_MO $5204`, `WV_MOL $5205`, scratch
`WV_T $5206`, `WV_END $5207`, `WV_FAR $5208`, `WV_FARL $5209`, the x of the spawner being run
`WV_SPX $520A`; per object slot the 16-bit screen x of an object followed from frame to frame,
`WX_LO $5220 + slot`, `WX_HI $5240 + slot`, and the frame it was last followed
`WX_SEEN $5260 + slot`.

---

## 3. What the game does, and what changes

Objects: 28 slots (`$00`-`$1B`): Simon, the enemies, the candles and the objects they hold, Simon's
weapons (`$14`-`$16`), and the special objects (`$17`-`$1B`, the moving platforms among them). Per
slot: `$0300` flags (bit 0 the x's bit 8, bit 7 hidden), `$038C` x, `$0434` type, `$031C` sprite,
`$0450` facing, `$0584` the kind of one of Simon's weapons (1 the cross). The camera is
`$2F:$2E`; `$51:$52` the scroll of the frame (high byte the sign); `$16`/`$17` the bounds of the
screen an object is checked against.

The game has two kinds of x:

* the objects it keeps in **9 bits**: the candles and torches, the spawners' objects, and those
  its check at `$EAB9` runs (Medusa heads, bats, fish...). x 0-255 is on the screen, 256-383
  right of it, 384-511 left of it, as its own `$EF45` reads it. The game keeps them alive from 64
  pixels left of the screen to 64 right of it, and hides those on the screen within 8 pixels of an
  edge, whose sprites would wrap;
* the objects it removes at the edges of the screen (most enemies): an 8-bit x moved by their own
  routines, which know nothing of bit 8, and checked after the routine against `[$16, $17)` once
  the scroll has moved them (`$EA0C`, Rev 1 `$EA30`: `JSR $EAA7` then `CMP $17 / BCS gone /
  CMP $16 / BCS kept`; the objects moved by `$EE70` jump to those compares themselves).

The hack draws the first kind past the edges, and follows the second kind in 16 bits.

| Game routine | Original | Hack |
|---|---|---|
| sprite builder, per object (bank 0 `$806D`, the same in both builds) | draws an object with flags 0 (on the screen) | with a margin, an object off the screen (flags 1) too, its 16-bit screen x read from its 9-bit x, unless that is outside the room (`wide_visible`: past the room's ends the game keeps objects the margins would show over nothing) |
| sprite builder, per sprite (bank 0 `$811C`) | x = 8-bit x + offset | the object's 16-bit x + offset, high byte to `$5100` + OAM offset (`wide_sprite_x`) |
| bounds after an object's routine (`$EA0C`, Rev 1 `$EA30`) | the scroll moves it, then it goes when its x leaves `[$16, $17)` | followed in 16 bits (`wide_follow`), moved by the scroll with the carry, bit 0 of its flags set to its x's bit 8; it goes when it leaves `[-MO, 256 + MO)`, by giving the game's compares `$FF` (gone) or `$80` (kept). The cross keeps the game's bounds (`wide_bounds`) |
| terrain under an object (`$EC11`, Rev 1 `$EC35`) | room x = 8-bit x + distance + camera | from its 16-bit x, held at the edge of the screen it is past (`wide_terrain_x`) |
| spawners of the room (`$F3AF`, Rev 1 `$F3D3`, and the return the loop pushes, `$F3F9`, Rev 1 `$F41D`) | a kind's routine puts the enemy it makes at an edge place (x 8 or 18 left, 238 or 248 right) or at the spawner's own place | before each routine the new slot is cleared and the spawner's x kept (`wide_before_spawner`); after it, an enemy just made at an edge place, more than 16 pixels from its spawner, is put MO further out, its bit 8 set when that crosses 0 or 255 (`wide_after_spawner`): it comes in from the margin |
| Simon's weapons against the objects (`$E5D6`, Rev 1 `$E5FA`) | 8-bit x compared, objects on the screen only | a weapon past an edge (bit 0 of its flags) touches nothing (`wide_weapon_hits`) |
| 9-bit objects near an edge (`$EAD1`, Rev 1 `$EAF5`) | hidden within 8 pixels of an edge | with a margin, not hidden (`wide_hide_near_edge`) |
| 9-bit objects off the screen (`$EAD9`, Rev 1 `$EAFD`) | removed 64 pixels past an edge | removed past FAR (`wide_far`) |
| an object made at another's place (`$E02B`, `$ECFF`, `$ED6E`, `$EFC5`; Rev 1 `$E050`, `$ED23`, `$ED92`, `$EFE9`: shots, pieces, points) | the 8-bit x copied, the child's flags just cleared | the x's bit 8 copied too (`wide_copy_x`, `wide_copy_x_back`): one made by an object past an edge is made there |
| candles and torches near an edge (`$DD2B`, Rev 1 `$DD50`) | hidden | with a margin, not hidden (`wide_edge_hide`) |
| moving platforms (stages 4, 5 and 10, slots `$17`-`$1A`; bank 6 `$AB5B`, the same in both builds) | their place on the screen is 16-bit (`$11:$10` + 16); the game keeps an 8-bit x and, near an edge, draws only the piece of them on the screen (a platform past the left edge showed at the right one too) | the 16-bit x kept for the sprite builder; within the margin, drawn whole (sprite `$C9`, facing 0; `wide_platform_x`) |

`wide_follow`: an object followed last frame gets its 16-bit x from last frame's by the change of
its 8-bit x (a change of more than 32 pixels is a new place on the screen, high byte 0); one not
followed last frame reads it from its 9-bit x. The special objects (`$17`-`$1B`) are followed in
`wide_visible`, the platforms' routine having set their 16-bit x when it ran this frame.

The game's collisions with Simon only take objects on the screen (`$0300` 0), so nothing in a
margin touches him; with the weapons kept from the other edge, an object of the margins neither
hurts nor is hurt. An enemy only shoots while on the screen (the game's own check before `$ED38`).
Weapons and the enemies' shots go on through the margins; the cross reaches an edge only at the
end of a room (Simon stays in the middle of the screen elsewhere), where the margin is outside the
room, and turns back there as in the game.

An object past an edge looks for its ground at the edge: the game keeps the terrain in a ring that
follows the nametables (`$05A0`), drawn only where the game has drawn, so an enemy past an edge
walks on the edge's ground until it comes onto the screen.

With M = 0 every routine does exactly what the code it replaces does.

CPU time: the game's split under its status bar is a sprite 0 wait early in the NMI, far from the
overclock (at line 240) and before the hack's code. Measured on a movie of stage 1 and its hall,
5,400 frames at a 112-pixel margin: the original game lags only in a room change (the same with
the hack), the hack without the overclock some 110 frames more, none with it.

---

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length. Bank 7 is
the fixed bank; banks 0 and 6 are those the game maps at `$8000` while the routines run.

| Bank | Base build | Rev 1 | Replaces | With |
|---|---|---|---|---|
| 0 | `$806D` | `$806D` | `LDA $0300,X` | `JSR wide_visible` |
| 0 | `$811C` | `$811C` | `CLC / ADC $02 / STA $0203,X` | `JSR wide_sprite_x` |
| 6 | `$AB5B` | `$AB5B` | `LDA $10 / CLC / ADC #$10 / STA $038C,X` | `JSR wide_platform_x` |
| 7 | `$C0A0` | `$C0A4` | `JSR $C1A1` (Rev 1 `JSR $C1E4`) | `JSR wide_frame` |
| 7 | `$E734` | `$E758` | `SEC / SBC $4B / STA $45` | `JSR wide_damage` |
| 7 | `$EA0C` | `$EA30` | `JSR $EAA7` (Rev 1 `JSR $EACB`) | `JSR wide_bounds` |
| 7 | `$EC11` | `$EC35` | `CLC / ADC $2E / STA $12 / LDA $2F / ADC #0 / STA $13` | `JSR wide_terrain_x` |
| 7 | `$F3AF` | `$F3D3` | `STY $0C / LDA $0166,Y` | `JSR wide_before_spawner` |
| 7 | `$F3F9` | `$F41D` | the word `$F358` (Rev 1 `$F37C`), the return the spawners' loop pushes | `wide_after_spawner - 1` |
| 7 | `$E5D6` | `$E5FA` | `LDA $0584,Y / CMP #6` | `JSR wide_weapon_hits` |
| 7 | `$EAD1` | `$EAF5` | `LDA #$80 / STA $0300,X` | `JSR wide_hide_near_edge` |
| 7 | `$EAD9` | `$EAFD` | `CPY #$40 / BCC kept / CPY #$C0 / BCS kept` (kept: `$EAD6`, Rev 1 `$EAFA`) | `JSR wide_far / BCS kept` |
| 7 | `$E02B`, `$ECFF`, `$ED6E` | `$E050`, `$ED23`, `$ED92` | `LDA $038C,X / STA $038C,Y` | `JSR wide_copy_x` |
| 7 | `$EFC5` | `$EFE9` | `LDA $038C,Y / STA $038C,X` | `JSR wide_copy_x_back` |
| 7 | `$DD2B` | `$DD50` | `LDA $0300,X / ORA #$80 / STA $0300,X` | `JSR wide_edge_hide` |

The spawners' loop returns to the next spawner through an `RTS` after it pushes a word read from
the table at `$F3F9` (data, not code): the hack puts its own address minus one there, and
`wide_after_spawner` ends with `JMP $F359` (Rev 1 `$F37D`), where the word led.

---

## 5. Options

The options the player can turn on or off (the game's hacks screen, with the advanced mode on);
Zaxnes writes their bits at `$500A` every frame, the hack reads them there.

| Bit | Key | Label | Default | Effect |
|---|---|---|---|---|
| 0 | `level_select` | Level select (Select: next stage) / Sélecteur de niveau (Select : stage suivant) | off | a testing aid: Select in a stage (state `$05`, `$F4` clear) goes on to the next stage (after the last, stage 18, the first), as a routine the base build keeps unused does (`$CF61`): the state that begins a stage (`$06`) with the graphics of the stage's block (`$C861`, then `$FCFF`; Rev 1 `$C8B0`, `$FD23`), and the life that state takes given back |
| 1 | `invincible` | Invincible | off | a testing aid: a hit takes nothing from Simon's health (`wide_damage`); pits still kill |
| 2 | `no_slowdown` | No slowdown / Sans ralentissements | on | 255 lines of overclock instead of none: the game never runs late (no lag frame) |

The options live in Zaxnes (`rust/crates/zax-nes/src/builtin_patches.rs`, `CASTLEVANIA_OPTIONS`,
shared by both patches); all 0 is the hack with the game's slowdowns.

---

## 6. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Castlevania (USA).nes" build/castlevania-usa-widescreen.bps
python -I tools/build.py "Castlevania (USA) (Rev 1).nes" build/castlevania-usa-rev1-widescreen.bps
```

`build.py` tells the build by the data CRC-32, assembles `src/wide.s` (`-g`, with `-D REV1` for
Rev 1, labels from `ld65 -Ln`) into the WINDOW segment (`$5800`, 2 KiB), checks that the place in
bank 5 is still `$FF`, writes the code and the hooks, `build/castlevania-usa.nes` (or
`castlevania-usa-rev1.nes`) and the BPS patch (`tools/make_bps.py`, SourceRead / TargetRead, a run
of `$FF` as a TargetCopy). `WIDE_SKIP=<hex addresses>` leaves hooks out, for testing;
`WIDE_DEFINES` passes more `-D` symbols to ca65.

---

## 7. Limits

* An object lives at most MO pixels past an edge (120 at most): with a margin wider than 104
  pixels, enemies still appear and go inside the margin rather than at its edge.
* Only the enemies a spawner makes at an edge place come from the margins; one made at the
  spawner's own place (a panther on its ledge) appears where the game puts it.
* An enemy past an edge walks on the edge's ground, not on the ground under it in the margin (the
  terrain ring holds nothing past what the game has drawn), until it comes onto the screen.
* Nothing in a margin touches Simon or his weapons, and an enemy in a margin does not shoot.
* The cross keeps the game's bounds; it reaches an edge only at the end of a room.
* Objects outside the room (past its ends, where the game still keeps some) are not drawn.
* No vertical margin (the decoder draws the sides only).

---

## Appendices

### A. `src/wide.s`

```asm
; Castlevania (USA), both builds: the widescreen hack (Zaxnes, advanced mode).
;
; One source for the two USA builds: their data banks 0-4 are the same, their code (banks 5-7)
; was rebuilt, so the hooks and the routines they reach lie elsewhere in each. REV1 (a ca65 -D
; symbol) picks Rev 1's addresses; the base build is the default.
;
; The code is WINDOW, 2 KiB of the free end of PRG bank 5 (from its page $1F, free in both
; builds), which the extension's PRG window shows at $5800-$5FFF. The game's fixed bank has no
; room for code that would unlock the extension, so the built-in patch opens the window from
; power-on (BuiltinPatch::boot_window): the hack needs Zaxnes. In the Zaxnes hack pack
; (tools/build.py --pack), WINDOW is the pack's code instead, in the extension's memory, which the
; window shows from power-on: nothing goes in bank 5.
;
; The ROM keeps its size and layout: a state of the unpatched game restores into the patched one.
;
; The background of the margins is not the hack's work: Zaxnes' margin decoder draws it.

; ---------------------------------------------------------------------------- ;
; The widescreen extension (see Zaxnes, docs/WIDESCREEN-EXTENSION.md).

WIDE_HOST       = $5002         ; R: the margin the host offers
WIDE_REQ        = $5003         ; W: the margin asked for
WIDE_LEFT       = $5004         ; nametable columns shown left of the screen
WIDE_RIGHT      = $5005         ; and right of it
WIDE_OVERCLOCK  = $5006         ; extra scanlines of CPU time per frame
WIDE_LAYOUT     = $5007         ; bit 1: sprites free of the extents
WIDE_OPTIONS    = $500A         ; R: the options the player turned on
WIDE_SPRHI      = $5100         ; + 4 n: high byte of the x of sprite n, for the next OAM DMA

MAX_MARGIN      = 248           ; the most the extension shows sprites in
MAX_OBJECTS     = 120           ; the most an object goes past an edge (the 9-bit x keeps both
                                ; sides apart below 128)

; Extension RAM ($5200-$57FF).
WV_M            = $5200         ; margin of this frame
WV_XLO          = $5201         ; the screen x of the object being drawn, 16-bit
WV_XHI          = $5202
WV_FRAME        = $5203         ; counts the frames
WV_MO           = $5204         ; the objects' margin: min(M + 16, 120)
WV_MOL          = $5205         ; and 256 - it
WV_T            = $5206         ; scratch
WV_END          = $5207         ; the room's end, in screens
WV_FAR          = $5208         ; the 9-bit objects' margin: max(MO, 64)
WV_FARL         = $5209         ; and 256 - it
WV_SPX          = $520A         ; the x of the spawner being run
; Per object slot: the 16-bit screen x of an object the game removes at the edges of the screen,
; followed from frame to frame, and the frame it was last followed.
WX_LO           = $5220
WX_HI           = $5240
WX_SEEN         = $5260

; The options ($500A).
OPT_LEVEL_SELECT = $01          ; Select in a stage: the next stage
OPT_INVINCIBLE  = $02           ; Simon never hurt (pits still kill)
OPT_NO_SLOWDOWN = $04           ; 255 lines of overclock: no lag frame

; ---------------------------------------------------------------------------- ;
; The game.

.if .defined(JAPAN)
; Akumajou Dracula (Japan): the same RAM and states, its code banks rebuilt (most routines $60-$70
; further), a shorter state dispatch, and a hit's damage read from a table (DAMAGE_TABLE,Y).
DISPATCH        = $C20A
MOVE_SCROLL     = $EB38
SPAWNER_NEXT    = $F3EA
ROOM_LENGTHS    = $FC1B
LOAD_BLOCK      = $FD8E
BLOCK_OF_STAGE  = $C8D4
DAMAGE_TABLE    = $E7C3
.elseif .defined(EUROPE)
; Castlevania (Europe): the USA builds' data banks, its code banks rebuilt (as the Rev 1's, most
; routines $0C further), the same RAM.
DISPATCH        = $C1E4
MOVE_SCROLL     = $EAD7
SPAWNER_NEXT    = $F389
ROOM_LENGTHS    = $FBB7
LOAD_BLOCK      = $FD2A
BLOCK_OF_STAGE  = $C8B0
.elseif .defined(REV1)
DISPATCH        = $C1E4         ; the game's state dispatch, called by the NMI
MOVE_SCROLL     = $EACB         ; an object's x moved by the scroll of the frame
SPAWNER_NEXT    = $F37D         ; the spawners' loop, on to the next one
ROOM_LENGTHS    = $FBB0         ; per stage: the camera's last screen in its rooms 0 and 1
LOAD_BLOCK      = $FD23         ; loads the graphics of block A
BLOCK_OF_STAGE  = $C8B0         ; the block of each stage
.else
DISPATCH        = $C1A1
MOVE_SCROLL     = $EAA7
SPAWNER_NEXT    = $F359
ROOM_LENGTHS    = $FB8C
LOAD_BLOCK      = $FCFF
BLOCK_OF_STAGE  = $C861
.endif

; An object slot ($00-$1B): its x is 9-bit, bit 0 of its flags the high bit (x 256-511 is off
; the screen: the game keeps its objects alive from 64 pixels left of the screen to 64 right of
; it, and a little further while they leave).
OBJ_FLAGS       = $0300         ; bit 0: x bit 8; bit 7: hidden
FIRST_SPECIAL   = $17           ; the special objects' slots, $17-$1B
OBJ_X           = $038C
OBJ_TYPE        = $0434
OBJ_SPRITE      = $031C
OBJ_FACING      = $0450
OBJ_KIND        = $0584         ; for Simon's weapons: 1 the cross (it turns back at the edges)
TYPE_WEAPON     = $17           ; Simon's weapons (slots $14-$16)
KIND_CROSS      = $01

SPAWNER         = $0C           ; the spawner being run
NEW_SLOT        = $0D           ; the slot its routine made an enemy in
SPAWNER_KIND    = $0166         ; + spawner: its kind
SPAWNER_X       = $0172         ; + spawner: its x on the screen

STATE           = $18           ; the game's state
STAGE           = $28           ; the stage, 0-18
ROOM            = $46           ; its room, 0 or 1
LIVES           = $2A           ; the lives, plus one
HEALTH          = $45           ; Simon's health, 64 full
DAMAGE          = $4B           ; what a hit takes
BUSY            = $F4
EDGE_LEFT       = $16           ; an object's bounds on the screen, as the game checks them
EDGE_RIGHT      = $17
CAMERA_LO       = $2E           ; the camera's x in the room
CAMERA_HI       = $2F
SCROLL_HIGH     = $51           ; the scroll of the frame: $51 its high byte (sign), $52 low
TERRAIN_LO      = $12           ; the room x the terrain is looked up at
TERRAIN_HI      = $13
PRESSED         = $F5           ; the buttons pressed this frame

STATE_PLAYING   = $05
STATE_NEW_STAGE = $06           ; starts stage STAGE, a life less
LAST_STAGE      = $12
BUTTON_SELECT   = $20

.segment "WINDOW"

; ---------------------------------------------------------------------------- ;
; The NMI's call of the game's state dispatch: the level select, then the game.
;
; Select in a stage goes on to the next stage (after the last, the first), as a routine the base
; build keeps unused does ($CF61): the stage starts again from the state that begins a stage,
; with the graphics of its block, and the life that state takes is given back.

wide_frame:
        INC WV_FRAME
        JSR wide_settings
        LDA WIDE_OPTIONS
        AND #OPT_LEVEL_SELECT
        BEQ @game
        LDA STATE
        CMP #STATE_PLAYING
        BNE @game
        LDA BUSY
        BNE @game
        LDA PRESSED
        AND #BUTTON_SELECT
        BEQ @game
        LDA #STATE_NEW_STAGE
        STA STATE
        INC LIVES
        LDX STAGE
        INX
        CPX #LAST_STAGE + 1
        BCC :+
        LDX #0
:       STX STAGE
        LDA BLOCK_OF_STAGE,X
        JSR LOAD_BLOCK
@game:  JMP DISPATCH

; ---------------------------------------------------------------------------- ;
; A hit on Simon (in place of SEC / SBC DAMAGE / STA HEALTH): none while invincible.

wide_damage:
        LDA WIDE_OPTIONS
        AND #OPT_INVINCIBLE
        BNE @keep
        LDA HEALTH
        SEC
.ifdef JAPAN
        SBC DAMAGE_TABLE,Y      ; Y as the game set it
.else
        SBC DAMAGE
.endif
        STA HEALTH
@keep:  RTS

; ---------------------------------------------------------------------------- ;
; The extension's settings, every frame: the margin, no nametable columns (Zaxnes' margin decoder
; draws the background), sprites free of them, the overclock (255 lines with the option "No
; slowdown": Zaxnes ends it as soon as the game waits in its main loop, so a frame that does not
; need it costs nothing).

wide_settings:
        LDA WIDE_HOST
        CMP #MAX_MARGIN
        BCC :+
        LDA #MAX_MARGIN
:       STA WIDE_REQ
        STA WV_M
        LDA #0
        STA WIDE_LEFT
        STA WIDE_RIGHT
        LDA #$02
        STA WIDE_LAYOUT
        LDA WIDE_OPTIONS
        AND #OPT_NO_SLOWDOWN
        BEQ :+
        LDA #255
:       STA WIDE_OVERCLOCK
        ; The objects' margin: past it the game's objects go.
        LDA WV_M
        BEQ @mo
        CLC
        ADC #16
        BCS @max
        CMP #MAX_OBJECTS
        BCC @mo
@max:   LDA #MAX_OBJECTS
@mo:    STA WV_MO
        LDA #0
        SEC
        SBC WV_MO
        STA WV_MOL
        ; The objects the game keeps in 9 bits go 64 pixels past the edges: never less.
        LDA WV_MO
        CMP #64
        BCS :+
        LDA #64
:       STA WV_FAR
        LDA #0
        SEC
        SBC WV_FAR
        STA WV_FARL
        ; The room's end: a screen past the camera's last (the high nibble of the stage's length
        ; for room 0, the low one for room 1, as the game reads it at $CFF0).
        LDX STAGE
        LDA ROOM_LENGTHS,X
        LDY ROOM
        BNE :+
        LSR A
        LSR A
        LSR A
        LSR A
:       AND #$0F
        CLC
        ADC #1
        STA WV_END
        RTS

; ---------------------------------------------------------------------------- ;
; The sprite builder (bank 0, the same in both builds), for each object (in place of LDA OBJ_FLAGS,X
; before BNE: Z set draws it). The game draws an object with flags 0 (on the screen); with a margin
; an object off the screen (flags 1) is drawn too, its 16-bit screen x read from its 9-bit x: 256-383
; right of the screen, 384-511 left of it, unless that is outside the room (the game keeps objects
; past its ends, which the margins would show over nothing). Its x is kept for its sprites (WV_XLO,
; WV_XHI).

wide_visible:
        LDA WV_M
        BEQ @game
        CPX #FIRST_SPECIAL
        BCS @special
@game:  LDA OBJ_X,X
        STA WV_XLO
        LDA OBJ_FLAGS,X
        BEQ @screen
        CMP #$01
        BNE @done               ; hidden: not drawn (Z clear)
        LDA WV_M
        BEQ @narrow
        LDA OBJ_X,X
        BMI @left
        LDA #$01
        BNE @high
@left:  LDA #$FF
@high:  STA WV_XHI
        LDA OBJ_X,X             ; its room x's screen: outside the room, not drawn
        CLC
        ADC CAMERA_LO
        LDA WV_XHI
        ADC CAMERA_HI
        BMI @narrow
        CMP WV_END
        BCS @narrow
        LDA #0                  ; Z set: drawn
@done:  RTS
@narrow:
        LDA #$01                ; no margin: off the screen is not drawn
        RTS
@screen:
        STA WV_XHI
        RTS
; A special object (slots $17-$1B: the moving platforms of stages 4, 5 and 10): the game puts its
; 8-bit x on the screen, flags 0, so near an edge it wraps to the other edge. The platforms' routine
; keeps its 16-bit x this frame (wide_platform_x); otherwise it is followed in 16 bits here.
@special:
        LDA OBJ_FLAGS,X
        AND #$FE
        BNE @done               ; hidden
        LDA OBJ_X,X
        STA WV_XLO
        LDA WX_SEEN,X
        CMP WV_FRAME
        BNE :+
        LDA WX_HI,X             ; set by the platforms' routine this frame
        STA WV_XHI
        JMP @kept
:       JSR wide_follow
@kept:  LDA OBJ_X,X
        STA WX_LO,X
        LDA WV_FRAME
        STA WX_SEEN,X
        LDA WV_XHI
        STA WX_HI,X
        BEQ @done               ; on the screen: drawn (Z set)
        CMP #$01
        BEQ @high
        CMP #$FF
        BEQ @high
        LDA #$01                ; far: not drawn
        RTS

; One sprite's x (in place of CLC / ADC $02 / STA $0203,X; A the sprite's signed offset, X its
; place in the sprite page): the object's 16-bit x plus the offset, its high byte to the extension.

wide_sprite_x:
        PHA
        CLC
        ADC WV_XLO
        STA $0203,X
        PLA
        AND #$80
        BEQ :+
        LDA #$FF
:       ADC WV_XHI
        STA WIDE_SPRHI,X
        RTS

; ---------------------------------------------------------------------------- ;
; The objects the game removes at the edges of the screen (most of its enemies: their x is 8-bit,
; moved by their own routines, which know nothing of its bit 8). With a margin they live on past
; the edges, up to the objects' margin, followed in 16 bits.
;
; The game's check after an object's routine (in place of its JSR MOVE_SCROLL, before CMP EDGE_RIGHT
; / BCS gone / CMP EDGE_LEFT / BCS kept, which stay: the objects moved by $EE70 jump to those
; compares themselves): the scroll moves it, then it goes when its x leaves [EDGE_LEFT, EDGE_RIGHT)
; of the screen, or, with a margin, when it leaves the objects' margin past the edges, A then $FF
; (gone) or $80 (kept) for the game's compares; bit 0 of its flags is kept its x's bit 8, as for
; the game's other objects.

wide_bounds:
        LDA WV_M
        BEQ @game
        ; The cross keeps the game's bounds: it only reaches an edge at the end of a room, where the
        ; margin past it is outside the room, and it turns back there (Simon stays in the middle
        ; of the screen elsewhere, and the cross comes back before an edge).
        LDA OBJ_TYPE,X
        CMP #TYPE_WEAPON
        BNE @wide
        LDA OBJ_KIND,X
        CMP #KIND_CROSS
        BNE @wide
@game:  JMP MOVE_SCROLL           ; A its x, as the game compares it
@wide:  JSR wide_follow
        JSR MOVE_SCROLL         ; C: the carry of its x
        LDA WV_XHI
        ADC SCROLL_HIGH
        STA WV_XHI
        STA WX_HI,X
        LDA OBJ_X,X
        STA WX_LO,X
        LDA WV_FRAME
        STA WX_SEEN,X
        LDA OBJ_FLAGS,X
        AND #$FE
        LDY WV_XHI
        BEQ :+
        ORA #$01
:       STA OBJ_FLAGS,X
        LDA WV_XHI
        BEQ @keep
        CMP #$01
        BEQ @right
        CMP #$FF
        BNE @gone
        LDA OBJ_X,X             ; left of the screen: kept from 256 - MO up
        CMP WV_MOL
        BCS @keep
        BCC @gone
@right: LDA OBJ_X,X             ; right of it: kept below MO
        CMP WV_MO
        BCC @keep
@gone:  LDA #$FF                ; past EDGE_RIGHT: gone
        RTS
@keep:  LDA #$80                ; within the screen's bounds: kept
        RTS

; The 16-bit screen x of object X (WV_XLO, WV_XHI): followed from last frame's by the change of its
; 8-bit x (a change of more than 32 pixels is a new place on the screen), or, for an object not
; followed last frame, read from its 9-bit x (256-383 right of the screen, 384-511 left of it).

wide_follow:
        LDA OBJ_X,X
        STA WV_XLO
        LDA WX_SEEN,X
        CLC
        ADC #1
        CMP WV_FRAME
        BNE @read
        LDA OBJ_X,X
        SEC
        SBC WX_LO,X
        STA WV_T
        CLC
        ADC #32
        CMP #64
        BCS @new
        LDA WX_LO,X
        CLC
        ADC WV_T
        LDA WV_T
        AND #$80
        BEQ :+
        LDA #$FF
:       ADC WX_HI,X
        STA WV_XHI
        RTS
@new:   LDA #0
        STA WV_XHI
        RTS
@read:  LDA OBJ_FLAGS,X
        AND #$01
        BEQ @high
        LDA OBJ_X,X
        BMI @left
        LDA #$01
        BNE @high
@left:  LDA #$FF
@high:  STA WV_XHI
        RTS

; The room x where an object looks for walls and floors (in place of CLC / ADC CAMERA_LO / STA
; TERRAIN_LO / LDA CAMERA_HI / ADC #0 / STA TERRAIN_HI; A its 8-bit x plus or minus the distance it
; looks at): with a margin, from its 16-bit screen x, held at the edge of the screen it is past.
; The game keeps the terrain in a ring that follows the nametables ($05A0), drawn only where the
; game has drawn: an enemy past an edge walks on the edge's ground until it comes onto the screen.

wide_terrain_x:
        PHA
        LDA WV_M
        BNE @wide
        PLA
        CLC
        ADC CAMERA_LO
        STA TERRAIN_LO
        LDA CAMERA_HI
        ADC #0
        STA TERRAIN_HI
        RTS
@wide:  PLA
        SEC
        SBC OBJ_X,X             ; the distance, signed
        STA WV_T
        LDA WV_T
        PHA
        JSR wide_follow
        LDA WV_XHI
        BEQ @screen
        BMI @left
        LDA #$FF
        BNE :+
@left:  LDA #0
:       STA WV_XLO
        LDA #0
        STA WV_XHI
@screen:
        LDA WV_XLO
        CLC
        ADC CAMERA_LO
        STA TERRAIN_LO
        LDA WV_XHI
        ADC CAMERA_HI
        STA TERRAIN_HI
        PLA
        STA WV_T
        CLC
        ADC TERRAIN_LO
        STA TERRAIN_LO
        LDA WV_T
        AND #$80
        BEQ :+
        LDA #$FF
:       ADC TERRAIN_HI
        STA TERRAIN_HI
        RTS

; The spawners of the room (the game's loop over them, $F352 in the base build): some kinds' routines
; put the enemy they make at an edge of the screen (zombies and the like: x 8 or 18 left, 238 or
; 248 right), others at the spawner's own place (a panther on its ledge); each keeps the slot in
; NEW_SLOT. Before each routine (in place of STY SPAWNER / LDA SPAWNER_KIND,Y) NEW_SLOT is cleared
; and the spawner's x kept; after it (the routine's return, which the loop pushes, comes here) an
; enemy just made at one of those edge places, away from its spawner, is put the objects' margin
; further out, so it comes in from the margin.

wide_before_spawner:
        STY SPAWNER
        LDA #$FF
        STA NEW_SLOT
        LDA SPAWNER_X,Y
        STA WV_SPX
        LDA SPAWNER_KIND,Y
        RTS

wide_after_spawner:
        LDA WV_M
        BEQ @done
        LDX NEW_SLOT
        BMI @done
        LDA OBJ_TYPE,X
        BEQ @done
        LDA OBJ_FLAGS,X
        AND #$01
        BNE @done
        LDA OBJ_X,X             ; at the spawner's own place: where the game puts it
        SEC
        SBC WV_SPX
        CLC
        ADC #16
        CMP #32
        BCC @done
        LDA OBJ_X,X
        CMP #$08
        BEQ @left
        CMP #$12
        BEQ @left
        CMP #$EE
        BEQ @right
        CMP #$F8
        BNE @done
@right: CLC
        ADC WV_MO
        STA OBJ_X,X
        BCC @done
        BCS @off
@left:  SEC
        SBC WV_MO
        STA OBJ_X,X
        BCS @done
@off:   LDA OBJ_FLAGS,X
        ORA #$01
        STA OBJ_FLAGS,X
@done:  JMP SPAWNER_NEXT

; The candles and torches near an edge (in place of LDA OBJ_FLAGS,X / ORA #$80 / STA OBJ_FLAGS,X):
; the game hides them there, their sprites would wrap to the other side; drawn in 16 bits, with a
; margin they stay.

wide_edge_hide:
        LDA WV_M
        BNE :+
        LDA OBJ_FLAGS,X
        ORA #$80
        STA OBJ_FLAGS,X
:       RTS

; ---------------------------------------------------------------------------- ;
; Simon's weapons against the objects (in place of LDA OBJ_KIND,Y / CMP #6 before BCS skip; Y the
; weapon's slot, $14-$16): the game compares their 8-bit x and only takes objects on the screen.
; A weapon past an edge (the margins keep it on its way) would meet an object at the other edge:
; it touches nothing there, as the objects of the margins touch nothing.

wide_weapon_hits:
        LDA OBJ_FLAGS,Y
        AND #$01
        BEQ :+
        LDA #$FF                ; past an edge: skipped
        BNE :++
:       LDA OBJ_KIND,Y
:       CMP #6
        RTS

; ---------------------------------------------------------------------------- ;
; The objects the game keeps in 9 bits (its check after their routine, $EAB9: Medusa heads, bats,
; fish...). On the screen within 8 pixels of an edge the game hides them (in place of LDA #$80 /
; STA OBJ_FLAGS,X), their sprites would wrap; drawn in 16 bits, with a margin they stay.

wide_hide_near_edge:
        LDA WV_M
        BNE :+
        LDA #$80
        STA OBJ_FLAGS,X
:       RTS

; Off the screen the game removes them 64 pixels past an edge (in place of CPY #$40 / BCC kept /
; CPY #$C0 / BCS kept; Y their x's low byte, C set keeps them): with a margin, past the objects'
; margin if that is further.

wide_far:
        LDA WV_M
        BNE @wide
        CPY #$40
        BCC @keep
        CPY #$C0
        RTS
@wide:  TYA
        CMP WV_FAR
        BCC @keep
        CMP WV_FARL
        RTS
@keep:  SEC
        RTS

; ---------------------------------------------------------------------------- ;
; An object made at another's place (shots, pieces, points: in place of LDA OBJ_X,X / STA OBJ_X,Y,
; X the parent and Y the child, or the other way round): the game copies its 8-bit x only, the
; child's flags just cleared. With a margin the parent may be past an edge: its x's bit 8 goes
; with it, so the child is not made at the other edge.

wide_copy_x:
        LDA OBJ_X,X
        STA OBJ_X,Y
        LDA WV_M
        BEQ :+
        LDA OBJ_FLAGS,X
        AND #$01
        STA WV_T
        LDA OBJ_FLAGS,Y
        AND #$FE
        ORA WV_T
        STA OBJ_FLAGS,Y
:       RTS

wide_copy_x_back:
        LDA OBJ_X,Y
        STA OBJ_X,X
        LDA WV_M
        BEQ :+
        LDA OBJ_FLAGS,Y
        AND #$01
        STA WV_T
        LDA OBJ_FLAGS,X
        AND #$FE
        ORA WV_T
        STA OBJ_FLAGS,X
:       RTS

; ---------------------------------------------------------------------------- ;
; The moving platforms (stages 4, 5 and 10, slots $17-$1A; bank 6 $AB5B, the same in both builds,
; in place of LDA PLAT_LO / CLC / ADC #$10 / STA OBJ_X,X): the game has their place on the screen in
; 16 bits ($11:$10, plus 16) and, near an edge, draws only the piece of them on the screen. Their
; 16-bit x is kept for the sprite builder; with a margin, one within the view is drawn whole.

PLAT_LO         = $10
PLAT_HI         = $11
PLATFORM_WHOLE  = $C9

wide_platform_x:
        LDA PLAT_LO
        CLC
        ADC #$10
        STA OBJ_X,X
        LDA PLAT_HI
        ADC #0
        STA WV_T
        LDA WV_M
        BEQ @done
        LDA OBJ_X,X
        STA WX_LO,X
        LDA WV_T
        STA WX_HI,X
        LDA WV_FRAME
        STA WX_SEEN,X
        LDA WV_T
        BEQ @whole
        CMP #$01
        BEQ @right
        CMP #$FF
        BNE @done
        LDA OBJ_X,X             ; left of the screen: within the margin
        CMP WV_MOL
        BCS @whole
        RTS
@right: LDA OBJ_X,X             ; right of it: within the margin
        CMP WV_MO
        BCS @done
@whole: LDA #PLATFORM_WHOLE
        STA OBJ_SPRITE,X
        LDA #0
        STA OBJ_FACING,X
@done:  RTS
```

### B. `tools/build.py`

```python
"""Builds the Castlevania (USA) widescreen hack (advanced mode) and its BPS patch, for one of the two
USA builds.

Usage: python -I tools/build.py <Castlevania (USA).nes or (Rev 1)> [out.bps]
       python -I tools/build.py <Castlevania (USA).nes, (Rev 1), (Europe) or Akumajou Dracula (Japan)>
           --pack <out.zxpk> [--ntsc]

The build is told by the ROM's data CRC-32. The two builds share their data banks 0-4; their code
(banks 5-7) was rebuilt, so each has its own hook addresses (HOOKS) and src/wide.s its own
addresses of the game's routines (ca65 -D REV1).

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH): the WINDOW segment
   for $5800, 2 KiB the widescreen extension's PRG window shows there, taken from the free end of
   PRG bank 5 (from its page $1F). WIDE_DEFINES (comma-separated) is passed to ca65 as -D symbols.
2. Writes it into the game's free space (checked to be all $FF) and the hooks into its code
   (each checked against the bytes it replaces). The ROM keeps its size and layout.
3. Writes build/<build>.nes and, if asked, the BPS patch (data without iNES header). WIDE_SKIP
   (comma-separated hex addresses) leaves hooks out, for testing.

With --pack, a Zaxnes hack pack, format 2, instead, written by Zaxnes' shared builder
(tools/hackpack.py of Zaxnes, its folder in the environment variable ZAXNES_TOOLS): NTSC, UxROM,
each hook a 16 KiB bank and a CPU address, the widescreen in four groups: "core" (Base), "edge
spawns", "platforms" and "copies" (each needing the core), with checks of the game's code the hack relies on
(CHECKS). The
WINDOW segment is the pack's code, assembled for $2000: the extension's execution zone, which
Zaxnes shows to instruction fetches only at $2000-$3FFF from power-on (data reads there reach the
PPU: the code reads no data of its own, its variables are in the extension's RAM at $5200); the
hooks, each with the game's bytes it replaces, checked by Zaxnes before it applies them.
Nothing goes in the game's free space and the ROM keeps its size. The damage hook is a component of its own,
"invincible" (LDA #$02 in place of the subtraction), applied by Zaxnes while that option is on,
with or without the widescreen.
"""

import os
import shutil
import subprocess
import sys
import zlib

PRG_SIZE = 0x20000
BANK_16K = 0x4000
WINDOW_START = 0x5800
WINDOW_SIZE = 0x800
WINDOW_BANK = 5
WINDOW_PAGE = 0x1F
ZONE_START = 0x2000             # the pack's code: the extension's execution zone
WINDOW_AT = WINDOW_BANK * BANK_16K + WINDOW_PAGE * 0x100

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

NOP = 0xEA

# The data CRC-32 of the two builds.
BASE = 0x0AC1AA8F
REV1 = 0xB668C7FC
EUROPE = 0xA93527E2             # Castlevania (Europe): a pack only (--pack)
JAPAN = 0xA0F99BB8              # Akumajou Dracula (Japan): a pack only (--pack)


def jsr(target):
    return bytes([0x20, target & 0xFF, target >> 8])


def offset(bank, address):
    """Where `address` of the game's 16 KiB PRG bank `bank` lies in the PRG-ROM (bank 7 is fixed
    at $C000, the others are seen at $8000)."""
    return bank * BANK_16K + (address & (BANK_16K - 1))


# The group of the pack each hook is in, by (bank, address), when not the "core" (hooks()).
HOOK_GROUP = {}


def hooks(build, sym):
    """The patches of the game's PRG-ROM: (bank, address, original bytes, new bytes)."""
    out = []

    def put(bank, address, original, new, group='core'):
        original = bytes.fromhex(original)
        new = bytes(new) + bytes([NOP]) * (len(original) - len(new))
        assert len(new) == len(original), hex(address)
        out.append((bank, address, original, new))
        if group != 'core':
            HOOK_GROUP[(bank, address)] = group

    # The sprite builder (bank 0, the same in both builds): objects off the screen drawn in the
    # margins, their sprites' x 16-bit.
    put(0, 0x806D, 'bd 00 03', jsr(sym['wide_visible']))
    put(0, 0x811C, '18 65 02 9d 03 02', jsr(sym['wide_sprite_x']))
    # The moving platforms (bank 6, the same in both builds): their 16-bit x kept, whole in the view.
    put(6, 0xAB6A if build == JAPAN else 0xAB5B, 'a5 10 18 69 10 9d 8c 03',
        jsr(sym['wide_platform_x']), 'platforms')
    if build == BASE:
        # The NMI's call of the game's state dispatch: the level select first.
        put(7, 0xC0A0, '20 a1 c1', jsr(sym['wide_frame']))
        # A hit on Simon: SEC / SBC $4B / STA $45.
        put(7, 0xE734, '38 e5 4b 85 45', jsr(sym['wide_damage']))
        fixed = {'bounds': (0xEA0C, '20 a7 ea'), 'terrain': 0xEC11, 'hide': 0xDD2B,
                 'spawner': 0xF3AF, 'spawner_return': (0xF3F9, '58 f3'),
                 'weapon': 0xE5D6, 'hide9': 0xEAD1, 'far9': 0xEAD9,
                 'copies': (0xE02B, 0xECFF, 0xED6E), 'copy_back': 0xEFC5}
    elif build == REV1:
        put(7, 0xC0A4, '20 e4 c1', jsr(sym['wide_frame']))
        put(7, 0xE758, '38 e5 4b 85 45', jsr(sym['wide_damage']))
        fixed = {'bounds': (0xEA30, '20 cb ea'), 'terrain': 0xEC35, 'hide': 0xDD50,
                 'spawner': 0xF3D3, 'spawner_return': (0xF41D, '7c f3'),
                 'weapon': 0xE5FA, 'hide9': 0xEAF5, 'far9': 0xEAFD,
                 'copies': (0xE050, 0xED23, 0xED92), 'copy_back': 0xEFE9}
    elif build == JAPAN:
        # Japan: its own build of the code; a hit's damage from a table (SEC / SBC $E7C3,Y /
        # STA $45).
        put(7, 0xC0CA, '20 0a c2', jsr(sym['wide_frame']))
        put(7, 0xE7BC, '38 f9 c3 e7 85 45', jsr(sym['wide_damage']))
        fixed = {'bounds': (0xEA9D, '20 38 eb'), 'terrain': 0xECA2, 'hide': 0xDDAB,
                 'spawner': 0xF440, 'spawner_return': (0xF48A, 'e9 f3'),
                 'weapon': 0xE657, 'hide9': 0xEB62, 'far9': 0xEB6A,
                 'copies': (0xE0AB, 0xED90, 0xEDFF), 'copy_back': 0xF056}
    else:
        # Europe: the Rev 1's code, most of it $0C further.
        put(7, 0xC0A4, '20 e4 c1', jsr(sym['wide_frame']))
        put(7, 0xE764, '38 e5 4b 85 45', jsr(sym['wide_damage']))
        fixed = {'bounds': (0xEA3C, '20 d7 ea'), 'terrain': 0xEC41, 'hide': 0xDD5B,
                 'spawner': 0xF3DF, 'spawner_return': (0xF429, '88 f3'),
                 'weapon': 0xE606, 'hide9': 0xEB01, 'far9': 0xEB09,
                 'copies': (0xE05B, 0xED2F, 0xED9E), 'copy_back': 0xEFF5}
    # After an object's routine: the bounds it is removed past (its JSR MOVE_SCROLL, base build
    # $EA0C; the compares after it stay, the objects moved by $EE70 jump to them), past the
    # margins.
    address, call = fixed['bounds']
    put(7, address, call, jsr(sym['wide_bounds']))
    # Where an object looks for walls and floors: from its 16-bit x.
    put(7, fixed['terrain'], '18 65 2e 85 12 a5 2f 69 00 85 13', jsr(sym['wide_terrain_x']))
    # The room's spawners: before each kind's routine (STY $0C / LDA $0166,Y), and the return the
    # loop pushes for it (the word read by LDA / PHA, data), into the hack: enemies made at an edge
    # come in from the margins.
    put(7, fixed['spawner'], '84 0c b9 66 01', jsr(sym['wide_before_spawner']), 'edge spawns')
    address, word = fixed['spawner_return']
    target = sym['wide_after_spawner'] - 1
    put(7, address, word, bytes([target & 0xFF, target >> 8]), 'edge spawns')
    # Simon's weapons: one past an edge touches nothing (LDA $0584,Y / CMP #6).
    put(7, fixed['weapon'], 'b9 84 05 c9 06', jsr(sym['wide_weapon_hits']))
    # The objects the game keeps in 9 bits: not hidden near the edges, and kept as far as the
    # margins (CPY #$40 / BCC / CPY #$C0 / BCS $EAD6, base build).
    put(7, fixed['hide9'], 'a9 80 9d 00 03', jsr(sym['wide_hide_near_edge']))
    put(7, fixed['far9'], 'c0 40 90 f9 c0 c0 b0 f5', jsr(sym['wide_far']) + bytes([0xB0, 0xF8]))
    # An object made at another's place takes its x's bit 8 too (LDA $038C,X / STA $038C,Y).
    # A group of their own: a ROM hack that put something else at one of them (The Holy Relics)
    # still takes the rest, an object born in a margin then copying its parent's 8-bit x only.
    for address in fixed['copies']:
        put(7, address, 'bd 8c 03 99 8c 03', jsr(sym['wide_copy_x']), 'copies')
    put(7, fixed['copy_back'], 'b9 8c 03 9d 8c 03', jsr(sym['wide_copy_x_back']), 'copies')
    # Candles and torches are not hidden near the edges.
    put(7, fixed['hide'], 'bd 00 03 09 80 9d 00 03', jsr(sym['wide_edge_hide']))
    return out


def tool(name):
    return [name] if shutil.which(name) else ['wsl', '-e', name]


def wsl_path(path):
    if shutil.which('ca65'):
        return path
    drive, rest = os.path.splitdrive(os.path.abspath(path))
    return '/mnt/' + drive[0].lower() + rest.replace('\\', '/')


def assemble(name, defines, start=WINDOW_START):
    build = os.path.join(ROOT, 'build', name)
    os.makedirs(build, exist_ok=True)
    cfg = os.path.join(build, 'wide.cfg')
    window_bin = os.path.join(build, 'window.bin')
    open(cfg, 'w', newline='\n').write(
        'MEMORY {\n'
        f'  WINDOW: start = ${start:04X}, size = ${WINDOW_SIZE:04X}, fill = yes, fillval = $FF, '
        f'file = "{wsl_path(window_bin)}";\n'
        '}\n'
        'SEGMENTS {\n'
        '  WINDOW: load = WINDOW, type = ro;\n'
        '}\n')
    obj = os.path.join(build, 'wide.o')
    labels = os.path.join(build, 'wide.labels')
    flags = []
    for symbol in defines + [d for d in os.environ.get('WIDE_DEFINES', '').split(',') if d]:
        flags += ['-D', symbol]
    subprocess.run(tool('ca65') + ['-g'] + flags +
                   [wsl_path(os.path.join(ROOT, 'src', 'wide.s')), '-o', wsl_path(obj)], check=True)
    subprocess.run(tool('ld65') + ['-C', wsl_path(cfg), '-Ln', wsl_path(labels), wsl_path(obj)],
                   check=True)
    sym = {}
    for line in open(labels):
        parts = line.split()
        if len(parts) >= 3 and parts[0] == 'al':
            sym[parts[2].lstrip('.')] = int(parts[1], 16)
    return open(window_bin, 'rb').read(), sym


# The hook of the option "Invincible" (SEC / SBC DAMAGE / STA HEALTH, bank 7) per build: in the
# pack, a component of its own.
INVINCIBLE = {BASE: 0xE734, REV1: 0xE758, EUROPE: 0xE764, JAPAN: 0xE7BC}


# The game's code and data the hack's code relies on, by group, per build: (bank, address, length),
# checked by Zaxnes to be the reference ROM's bytes (a ROM hack that changed them gets the groups
# that still fit). Core: the state dispatch the NMI hook calls on to (its start), the routine that
# moves an object's x by the scroll (whole), the game's read of the rooms' lengths (the table is
# where the hack reads it), and for the level select the graphics loader (its start) and the
# game's read of each stage's block. Edge spawns: the spawners' loop, on to the next one.
CHECKS = {
    BASE: {'core': [(7, 0xC1A1, 8), (7, 0xEAA7, 18), (7, 0xCFF2, 3), (7, 0xFCFF, 12),
                    (7, 0xC7AA, 3)],
           'edge spawns': [(7, 0xF359, 6)]},
    REV1: {'core': [(7, 0xC1E4, 8), (7, 0xEACB, 18), (7, 0xD017, 3), (7, 0xFD23, 12),
                    (7, 0xC7F9, 3)],
           'edge spawns': [(7, 0xF37D, 6)]},
    EUROPE: {'core': [(7, 0xC1E4, 8), (7, 0xEAD7, 18), (7, 0xD017, 3), (7, 0xFD2A, 12),
                      (7, 0xC7F9, 3)],
             'edge spawns': [(7, 0xF389, 6)]},
    JAPAN: {'core': [(7, 0xC20A, 7), (7, 0xEB38, 18), (7, 0xD069, 3), (7, 0xFD8E, 12),
                     (7, 0xC81D, 3)],
            'edge spawns': [(7, 0xF3EA, 6)]},
}

# The region each build's pack is made for.
REGION = {BASE: 'ntsc', REV1: 'ntsc', EUROPE: 'pal', JAPAN: 'ntsc'}


def pack_place(offset, address):
    """The place of format 2 for `offset` in the PRG-ROM, run at `address`: its 16 KiB bank, and
    the address with the offset's place in that bank (an 8 KiB bank's half of it)."""
    return offset // 0x4000, (address & 0xC000) | (offset & 0x3FFF)


def pack(data, crc, window, sym, out, ntsc=False):
    tools = os.environ.get('ZAXNES_TOOLS')
    if not tools or not os.path.isfile(os.path.join(tools, 'hackpack.py')):
        sys.exit('ZAXNES_TOOLS: the rust/tools folder of Zaxnes (its hackpack.py) is needed')
    sys.path.insert(0, tools)
    import hackpack
    # --ntsc: the European pack for the ROMs of its code whose header says NTSC.
    pack = hackpack.Pack('', b'', region='ntsc' if ntsc else REGION[crc], mappers=(2,))
    widescreen = pack.component('widescreen')
    groups = {
        'core': widescreen.group('core', label='Base', label_fr='Base'),
        'edge spawns': widescreen.group('edge spawns', requires=['core'], label='Edge spawns',
                                        label_fr='Apparitions au bord'),
        'platforms': widescreen.group('platforms', requires=['core'], label='Moving platforms',
                                      label_fr='Plateformes mobiles'),
        'copies': widescreen.group('copies', requires=['core'], label='Objects born in the margins',
                                   label_fr='Objets nés dans les marges'),
    }
    core = groups['core']
    for name, checks in CHECKS[crc].items():
        for bank, address, length in checks:
            groups[name].check(bank, address, length)
    aid = pack.component('invincible').group('invincible', label='Invincible',
                                             label_fr='Invincible')
    for bank, address, original, new in hooks(crc, sym):
        at = offset(bank, address)
        assert data[at:at + len(original)] == original, f'${address:04X}: not the expected bytes'
        if address == INVINCIBLE[crc]:
            # LDA #$02, health kept: A as wide_damage leaves it.
            aid.hook(*pack_place(at, address), bytes(original), bytes([0xA9, 0x02, NOP, NOP, NOP]))
        else:
            group = groups[HOOK_GROUP.get((bank, address), 'core')]
            group.hook(*pack_place(at, address), bytes(original), bytes(new))
    assert len(aid.hooks) == 1
    game = {BASE: 'Castlevania (USA)', REV1: 'Castlevania (USA) (Rev 1)',
            EUROPE: 'Castlevania (Europe)', JAPAN: 'Akumajou Dracula (Japan)'}[crc]
    # Zaxnes reads $FF past the code's end: its trailing fill is left out.
    used = (len(window.rstrip(b'\xff')) + 0xFF) & ~0xFF
    pack.name = game + (' (NTSC)' if ntsc and REGION[crc] != 'ntsc' else '') + ' widescreen'
    pack.code = window[:used]
    result = pack.build(data, 0x20000)
    open(out, 'wb').write(result)
    print(f'{out}: {len(result)} bytes ({used} of code, {len(core.hooks)} hooks), '
          f'pack CRC {zlib.crc32(result):08X}')


def main():
    rom_path = sys.argv[1]
    image = bytearray(open(rom_path, 'rb').read())
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = image[header:header + PRG_SIZE]
    crc = zlib.crc32(data)
    if crc == BASE:
        name, defines = 'castlevania-usa', []
    elif crc == REV1:
        name, defines = 'castlevania-usa-rev1', ['REV1']
    elif crc == EUROPE:
        name, defines = 'castlevania-europe', ['EUROPE']
    elif crc == JAPAN:
        name, defines = 'akumajou-dracula-japan', ['JAPAN']
    else:
        sys.exit(f'{rom_path}: not Castlevania (USA), its Rev 1, (Europe) or Akumajou Dracula '
                 f'(Japan) (data CRC {crc:08X})')

    if len(sys.argv) > 3 and sys.argv[2] == '--pack':
        zone, sym = assemble(name, defines, ZONE_START)
        pack(bytes(data), crc, zone, sym, sys.argv[3], '--ntsc' in sys.argv[4:])
        return
    if crc in (EUROPE, JAPAN):
        sys.exit('Castlevania (Europe) and Akumajou Dracula (Japan): a pack only (--pack)')
    window, sym = assemble(name, defines)
    assert len(window) == WINDOW_SIZE
    prg = bytearray(data)
    assert prg[WINDOW_AT:WINDOW_AT + WINDOW_SIZE] == bytes([0xFF]) * WINDOW_SIZE, \
        'bank 5: the space is not free'
    prg[WINDOW_AT:WINDOW_AT + WINDOW_SIZE] = window

    skip = {int(a, 16) for a in os.environ.get('WIDE_SKIP', '').split(',') if a}
    for bank, address, original, new in hooks(crc, sym):
        if address in skip:
            continue
        at = offset(bank, address)
        assert prg[at:at + len(original)] == original, f'${address:04X}: not the expected bytes'
        prg[at:at + len(new)] = new
    target = bytes(prg)

    out = os.path.join(ROOT, 'build', name + '.nes')
    open(out, 'wb').write(bytes(image[:header]) + target)
    print(f'{out}: data CRC {zlib.crc32(target):08X}')
    if len(sys.argv) > 2:
        sys.path.insert(0, os.path.join(ROOT, 'tools'))
        import make_bps
        patch = make_bps.make(bytes(data), target)
        open(sys.argv[2], 'wb').write(patch)
        print(f'{sys.argv[2]}: {len(patch)} bytes')


if __name__ == '__main__':
    main()
```

### C. `tools/make_bps.py`

```python
"""Writes a BPS patch from an original ROM's data to the widescreen build's, both without their
iNES header.

Only the bytes the hack writes travel in the patch (TargetRead); everything else is copied from the
player's own ROM: SourceRead where a byte sits where it was, SourceCopy for data the build moved
(the CHR-ROM, after the PRG bank the hack adds). The patch carries no data of the game.
"""

import zlib


def number(value):
    out = bytearray()
    while True:
        low = value & 0x7F
        value >>= 7
        if value == 0:
            out.append(0x80 | low)
            return bytes(out)
        out.append(low)
        value -= 1


def make(source, target, moved=()):
    """`moved`: (target start, target end, source start) of data the build moved."""
    patch = bytearray(b'BPS1')
    patch += number(len(source)) + number(len(target)) + number(0)
    source_relative = 0
    target_relative = 0

    def origin(position):
        """Where target byte `position` comes from in the source, when it may come from there."""
        for start, end, from_ in moved:
            if start <= position < end:
                return from_ + position - start
        return position

    position = 0
    while position < len(target):
        def same(at):
            at_source = origin(at)
            return at_source < len(source) and source[at_source] == target[at]

        copied = same(position)
        shifted = origin(position) != position
        end = position
        while end < len(target) and same(end) == copied and (origin(end) != end) == shifted:
            end += 1
        length = end - position
        pending_start = position
        if not copied:
            # Bytes of the hack; a run of one value (the bank's $FF fill) as its first byte, then
            # a TargetCopy of it.
            run = position
            while position < end:
                run = position + 1
                while run < end and target[run] == target[position]:
                    run += 1
                if run - position >= 8:
                    if pending_start < position:
                        patch += number(((position - pending_start - 1) << 2) | 1) + target[pending_start:position]
                    patch += number(1) + target[position:position + 1]
                    delta = position - target_relative
                    patch += number(((run - position - 2) << 2) | 3)
                    patch += number((abs(delta) << 1) | (1 if delta < 0 else 0))
                    target_relative = position + (run - position - 1)
                    pending_start = run
                position = run
            if pending_start < end:
                patch += number(((end - pending_start - 1) << 2) | 1) + target[pending_start:end]
        elif not shifted:
            patch += number(((length - 1) << 2) | 0)
        else:
            start = origin(position)
            delta = start - source_relative
            patch += number(((length - 1) << 2) | 2)
            patch += number((abs(delta) << 1) | (1 if delta < 0 else 0))
            source_relative = start + length
        position = end
    patch += zlib.crc32(source).to_bytes(4, 'little')
    patch += zlib.crc32(target).to_bytes(4, 'little')
    patch += zlib.crc32(bytes(patch)).to_bytes(4, 'little')
    return bytes(patch)
```
