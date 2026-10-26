# Adventure Island (USA): the widescreen hack

This document describes completely the modification of *Adventure Island (USA)* that Zaxnes ships
as a built-in patch for the game's **advanced** widescreen mode. The appendices hold the new code
and the build script, which writes every hook after checking the bytes it replaces: with them the
ROM can be rebuilt identically.

The background of the margins is not the hack's work: Zaxnes' Adventure Island margin decoder lays
the round out from its tables in PRG-ROM (see
[`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md), section 4). In the **simple**
mode the ROM is not patched at all and the decoder alone widens the game; the advanced mode adds
this patch: the sprites and objects of the margins, the previews of the objects to come, and the
options.

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Adventure Island (USA)*, CNROM (mapper 3), 32 KiB PRG, 32 KiB CHR |
| Source data (iNES header excluded) | CRC-32 `F8A713BE` (the dump the [disassembly](https://github.com/cyneprepou4uk/NES-Games-Disassembly/tree/main/Adventure%20Island) rebuilds, SHA-1 of the iNES file `22EE75B8...`) |
| Target data | CRC-32 `0F609D55` (the hack's version in savestate headers; `tools/build.py` prints it), 80 KiB: the game's 32 KiB of PRG, a 16 KiB PRG bank added, the 32 KiB of CHR |
| Changes | hooks in the game's PRG (same size as what they replace); the new code in the added bank |
| Patch | `rust/crates/zax-nes/patches/adventure-island-usa-widescreen.bps`, made against the data without header; the CHR-ROM, moved after the new bank, is copied from the player's ROM (BPS SourceCopy), so the patch carries only the hack's bytes |
| Zaxnes declaration | `builtin_patches.rs`: `advanced: true` (applied only in the advanced mode), `boot_window: Some((0x82, 0x00))`, `idle_loop: Some((0xB759, 0xB75C))` |

The game has no free byte left in its PRG. Zaxnes therefore opens the extension at power-on and
reset for this patch (the boot window), as for Super Mario Bros.: unlocked, the PRG window showing
the added bank (bank 2, page 0) at `$5800-$5FFF`, and on to `$7FFF` (`$5007` bit 4; the game has
no PRG-RAM there). CNROM maps the first 32 KiB at `$8000-$FFFF` as before (its register only
switches the CHR), so the game runs as it was; its hooks call the new code in the window. The data
grows by one PRG bank; Zaxnes raises the PRG bank count of the player's header accordingly when it
applies the patch.

---

## 2. What the game does, and what changes

The main loop waits for the NMI at `$B759` (`LDA $0B / BNE`, the yield's idle loop); the NMI copies
the sprites (`$0700`, OAM DMA) only when the frame's logic is done. Objects live in 16 slots
(0 the player), each variable an array indexed by the slot: state `$0554` (0 free), kind `$0564`,
x `$0584` (low) and `$0594` (high), y `$05C4`, half widths `$05E4` (drawing) and `$0604`/`$0614`
(collisions). An object's x is a **screen** x, 16-bit: the game moves every object with the screen
as it scrolls (`sub_8E64`). Objects are created 64 pixels past the right edge (x = 320, `$8C5A`)
and removed past x = 400 or when their left edge (x minus half width) goes past the screen's left
edge (`$8EA7`).

| Part | The game | The hack |
|---|---|---|
| NMI (`$B717`) | | the extension's settings each frame: `$5003 = min($5002, 248)`, extents 0 (the decoder draws the background), `$5007 = $16` (sprites free of the extents, second sprite page, window to `$7FFF`), `$5006` = 255 with the option "No slowdown" else 0 |
| creating the objects (`JSR sub_8C45` at `$80EC`, the main loop) | creates the objects of the round's lists the screen comes 64 pixels near | first the **previews** of the objects to come within the wide view (see below), then the game's creation |
| an object's sprites (`sub_C301`, `$C305`) | gives up an object past the right edge (x high byte 1 and up) | gives up an object past the wide view (`x < -M-32` or `x >= 256+M`) |
| each sprite (`$C48B`, `$C4A9`) and a single-sprite object (`$C378`) | leaves out a sprite whose x is not on the screen (high byte not 0) | places a sprite within the wide view (`-M-8 < x < 256+M`) and writes the high byte of its x to the extension (`$5100` + its OAM offset); while a preview is captured, keeps the sprite instead (the tile and attributes the game would write) |
| the left edge (`$8EA7`) | removes an object whose left edge goes past the screen's | keeps it as a **ghost** while `x >= -M-32` (x high byte 0, or `$FF` with `x + 32 + M` carrying); a ghost back on the screen goes, as the game had it go; past the view it goes |
| an object's behaviour (`$8E8E`) | runs it | runs a ghost's too, but keeps the sound it asked for out (`$A0` restored) and creates nothing for it (`wide_find_slot` finds no slot) |
| two objects' collision (`sub_BFF2`) | | none with a ghost |
| a new object's slot (`sub_8DB4`, `$8DB6`) | the highest free slot, none when all are taken | the same, or else a ghost's slot (the ghost is gone) |
| an object's state (`$9534`, the `STA $0554,X` that creates and removes) | | the slot is no ghost any more |
| every object cleared (`sub_8C35`, `$8C41`) | | no ghost any more |
| title screen (`$AE51`) | Start begins; Select only sets the demo's timer again | with the option "Level select": Select goes through the 32 rounds, shown under "PUSH START" in the second sprite page (two digits, area and round) |
| new game (`$8078`) | area 0, round 0 | the round selected, with the option on (not in the demo); the second page hidden |
| an enemy's touch (`$C04D`) | kills (or takes the skateboard, or the fairy protects) | with the option "Invincible": no effect, as if the player were dead already; falls, the water and hunger still kill |

M is the margin of the frame (`WV_M`, at most 248). With M = 0 every routine does what the code it
replaces does: the game runs the same (checked: the same inputs on the original ROM and on the hack
without margins leave the same RAM, but for the stack's unused bytes).

**Why ghosts.** The game's own objects keep their exact behaviour: the hack changes nothing of what
they do on the screen. An object the game would have removed at the left edge only shows, still
moving, in the left margin: it touches nothing (the player, the axes, the other objects), makes no
sound and creates nothing, so whatever it does there cannot change the game. A ghost takes a slot
the game would have had free; a new object takes it back when it needs it. The game has no random
number generator, so running a ghost's behaviour changes nothing elsewhere. Checked: the same inputs
with a 96-pixel margin and the original lead to the same deaths and the same sections; the hack is
a few frames behind after a few thousand frames (the frames it takes longer, without the option
"No slowdown").

**The right margin.** Objects are created 64 pixels past the screen (x = 320) and live up to
x = 400: within a margin of 64 pixels they show, moving, before they reach the screen, as the game
runs them. Past it, the **previews** show each object still where the game will create it, from
the edge of the view on; the game's own object takes its place, at the same place, the frame the
game creates it.

**Previews.** The game reads its three object lists (`$7C`/`$7D` + list, bits `$0546`/`$0547`, the
next entry's position `$0540`/`$0541`, the lists 0, 2 and 4) one entry at a time as the screen
comes 64 pixels near it (`$8C5A`): its kind (6 bits), one or two extra bytes for the kinds `$38` and
up and `$3E` and up, then the next position (1 bit, then 10 bits, 0 ending the list, or 6 bits
added, x 8). The hack reads them ahead with the game's routine (`sub_8D74`) on a copy of a list's
state (the list's variables put back after), one entry a frame at most, and keeps up to 4 entries
per list with the state the list will have once the game has created each: when the game's state
becomes one of them, the entries before are dropped; any other state reads the list again.
An entry within the view (its x minus 256 under M + 32) gets a preview the first time it is seen,
one a frame at most: its object is created in a free slot as `$8CD8`-`$8CF0` would (the position
and kind in `$054C`-`$054F`, x in `$84`-`$85`, the ground's y in `$86`, `sub_8DB4`'s creation from
`$8DBF`: the kind's data and its own setup), then drawn there by the game (`sub_91EE`) while the
sprite hooks keep its sprites (8 at most, as offsets from its x and y). Then the slot's 20 arrays
(`$0554`-`$0684`), `$A0`, `$3E`, `$052E`-`$052F`, `$82`-`$87` and the drawing's `$58`-`$64` are put back;
the NMI does nothing meanwhile (`$0C` set, the game's own guard), nor does a new object get a slot
(`WV_GHOST_RUN`). A kind whose setup removes it, or that `sub_8EC9` does not draw (`$0684` 0: the
secret eggs, the invisible kinds), has no sprites. Kinds `$40` and up (the player, the checkpoint
signs, the weapons, the bosses) have none. Each frame the kept sprites are placed in the second
sprite page at the object's x (its entry's position minus the screen's left edge, + 256) and y
(moved as `sub_8E64` moves the objects' y with the screen, `$3D`); those past the view are left out.
The ground the creation needs (`sub_C1AD`) is known by the game for 5 parts of 128 pixels; past
them the hack works it out from the round's sections (`$1A`, 3 bytes each) as `$BEDE` does when it
loads the next part (the last part's y plus `$BF43` of the section's 2nd byte, the shape and
`$3E` from its 3rd byte, the slopes and steps of `sub_C1EA`). Previews are off with a margin under
72, while the player dies (`$72`) and while the game creates nothing (`$3A` negative).

## 3. Variables (extension RAM)

`WV_M $5200` margin, `WV_A $5201` the animation of `sub_C301` (kept), `WV_T0 $5202` / `WV_T1
$5203` scratch, `WV_LEVEL $5204` the round selected (0-31), `WV_MAGIC $5205` (`$A5` once the RAM is
set up: no ghost, the second page hidden), `WV_GHOST_RUN $5206` (a ghost's behaviour is running),
`WV_SOUND $5207` (the sound before it), `WV_GHOST $5210 + slot`. The previews: `WV_PREVIEW $5208`
(their sprites being captured), `$5209`-`$520F` and `$5220`-`$523F` the frame's work, `$5240`-`$5277`
their cache (8 entries: in use, seen, the list and position, y, sprites), `$5278`-`$5287` more work,
`$52C0`-`$52D3` the borrowed slot, `$52E0`-`$52EC` the drawing's variables, `$5500`-`$55B7` the
lists read ahead, `$5600`-`$56FF` the kept sprites (32 bytes an entry). The second sprite page holds
the title's two digits (`$5300`-`$5307`) or the previews. The code is at `$5800`-`$7FFF` (the
window).

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length.

| Address | Replaces | With |
|---|---|---|
| `$B717` | `LDA #<ram_oam / STA $2003` (NMI) | `JSR wide_nmi` |
| `$C305` | `LDY $59 / DEY / BPL $C372` (`sub_C301`) | `JSR wide_object_shown` |
| `$C48B` | `LDA $5B / BNE $C4C8` | `JSR wide_sprite_at` |
| `$C4A9` | `LDA $5B / BNE $C4C8` | `JSR wide_sprite_tayx` |
| `$C378` | `LDA $59 / BNE $C3A3` | `JSR wide_one_sprite` |
| `$8EA7` | the 13 bytes of "x - half width < 0" (`... / BMI $8EC2`) | `JSR wide_left_gone / BCS $8EC2` |
| `$8E8E` | `JSR sub_929E_object_handler` | `JSR wide_handler` |
| `$BFF2` | `LDA $0614,X / CLC / ADC $0614,Y` (`sub_BFF2`) | `JSR wide_collision` |
| `$8DB6` | the free slot search (`LDA $0554,X / BEQ / DEX / BNE`) | `JSR wide_find_slot / BCS $8DBF / RTS` |
| `$9534` | `STA $0554,X / RTS` | `JMP wide_object_state` |
| `$8C41` | `STX $0536` (`sub_8C35`) | `JSR wide_clear_ghosts` |
| `$80EC` | `JSR sub_8C45` (the main loop) | `JSR wide_spawn` |
| `$AE51` | `CMP #$10 / BNE $AE8F` (title screen) | `JSR wide_title_button` |
| `$8078` | `LDA #0 / STA $37 / STA $38` (new game) | `JSR wide_start_level` |
| `$C04D` | `LDA $72 / ORA $70` (an enemy's touch) | `JSR wide_injury` |

A routine that gives up what the game was doing (an object or a sprite left out, no collision, the
title's Select) drops its return address and goes where the game's branch went (`$C3A3`, `$C4C8`,
`$AE8F`, the caller of `sub_C301` or `sub_BFF2`).

## 5. Options

Zaxnes writes their bits at `$500A` every frame (the game's hacks screen, "Advanced mode" on).

| Bit | Key | Label | Default | Effect |
|---|---|---|---|---|
| 0 | `level_select` | Level select (Select on the title screen) / Sélecteur de niveau (Select sur l'écran titre) | off | a testing aid: Select goes through the 32 rounds (shown "area round" under "PUSH START"), Start begins there |
| 1 | `invincible` | Invincible | off | a testing aid: an enemy's touch does nothing; falls, the water and hunger still kill |
| 2 | `no_slowdown` | No slowdown / Sans ralentissements | on | `$5006 = 255`: no lag frame; with Zaxnes' yield (`idle_loop`), a frame that does not need it costs nothing more |

With the movies of Zaxnes (Tools menu, `.zmv`), the level selector and invincibility make a round
quick to reach and to go through when looking for what shows wrong in the margins.

## 6. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Adventure Island (USA).nes" build/aiwide.bps
python -I tools/make_doc.py <zaxnes>/rust/docs/hacks/adventure_island/WIDESCREEN-HACK.md
```

`build.py` assembles `src/wide.s` for `$5800` (a 16 KiB bank, `$FF` fill), writes the hooks into the
game's PRG (checked), the target data (PRG, the bank, CHR), `build/aiwide.nes` (header with 3 PRG
banks) and the BPS patch (`make_bps.py`: SourceRead, SourceCopy for the moved CHR, TargetRead,
TargetCopy for runs of one value). `WIDE_SKIP=<hex addresses>` leaves hooks out, for testing.
`make_doc.py` writes the appendices of this document from the sources.

## 7. Limits

* A preview is still: the object as created, its first picture; it moves from the frame the game
  creates it.
* No preview without a free slot to create it in (all 15 taken: the object shows from x = 320),
  nor for the checkpoint signs (`$8BDD`, not in the lists).
* The previews cost a little CPU time each frame, more the frame one is created or an entry read.
  Checked: the same inputs on the 32 rounds (2400 frames, margins 96, 128 and 248) leave the same
  RAM as the previous version of the hack (without previews), but in one round where the round is run again fast from its
  checkpoint after a death (frames the game overruns, with or without the hack): it ends one frame
  later there.
* Past the round's end (the boss's room, the last screen) the margins are empty.
* A ghost runs its behaviour without touching anything: an enemy that would have turned back in the
  margin goes back onto the screen and is removed there, as the game had removed it at the edge.
* More sprites show: the game still places 63 at most (`$C464`) and turns which ones are left out
  each frame, so a crowded screen flickers more than on the console.
* The platforms the game draws over the tables (the cloud platforms of the sea rounds) are the
  decoder's: it shows them ahead in the right margin from the object lists, at the row the ground
  gives them with the screen's vertical position of the moment (see the decoder's description).

## Appendices

### A. `src/wide.s`

```asm
; Adventure Island (USA): the widescreen hack (Zaxnes, advanced mode).
;
; This code is a 16 KiB PRG bank added after the game's 32 KiB; the widescreen extension's PRG
; window shows it at $5800-$7FFF ($5007 bit 4), Zaxnes opening it at power-on (the patch's boot
; window: bank 2, page 0). CNROM maps the first 32 KiB at $8000-$FFFF as before; the game reaches
; this code through hooks in its own (tools/build.py). The background of the margins is not the
; hack's work: Zaxnes' margin decoder draws it.
;
;  * every sprite of an object within the wide view shows, with the high byte of its x;
;  * an object the game removes as it leaves the screen's left edge stays in the left margin as a
;    ghost until it leaves the view: it goes on moving, but touches nothing, creates no object and
;    makes no sound, so the game is the original's; a new object takes a ghost's slot when no slot
;    is free;
;  * options: the level selector (Select on the title screen), invincibility, no slowdown.
;
; With a margin of 0 every routine does what the code it replaces does.

; ---------------------------------------------------------------------------- ;
; The widescreen extension (see Zaxnes, docs/WIDESCREEN-EXTENSION.md).

WIDE_HOST       = $5002         ; R: the margin the host offers
WIDE_REQ        = $5003         ; W: the margin asked for
WIDE_LEFT       = $5004         ; nametable columns shown left of the screen
WIDE_RIGHT      = $5005         ; and right of it
WIDE_OVERCLOCK  = $5006         ; extra scanlines of CPU time per frame
WIDE_LAYOUT     = $5007         ; bit 1 sprites free of the extents, 2 second page, 4 window to $7FFF
WIDE_OPTIONS    = $500A         ; R: the options the player turned on
WIDE_SPRHI      = $5100         ; + 4 n: high byte of the x of sprite n, for the next OAM DMA
EXTRA_OAM       = $5300         ; the second sprite page (the title's level, the previews)
EXTRA_HI        = $5400         ; + 4 n: high byte of the x of its sprite n

MAX_MARGIN      = 248           ; the most the extension shows sprites in

; Extension RAM ($5200-$57FF; $5800 up is the window).
WV_M            = $5200         ; margin of this frame
WV_A            = $5201         ; scratch
WV_T0           = $5202
WV_T1           = $5203
WV_LEVEL        = $5204         ; the level selected on the title screen, 0-31
WV_MAGIC        = $5205         ; MAGIC once the extension's RAM is set up
WV_GHOST_RUN    = $5206         ; non-zero while a ghost's behaviour runs
WV_SOUND        = $5207         ; the sound asked for before it
WV_PREVIEW      = $5208         ; non-zero while a preview's sprites are captured
WV_PV_N         = $5209         ; the second page's next sprite (offset)
WV_PV_FULL      = $520A         ; non-zero once the second page is full
WV_PV_LAST      = $520B         ; the offset the previous frame's previews went up to
WV_PV_S         = $520C         ; the object list at hand (0, 2, 4)
WV_PV_D         = $520D         ; 2 bytes: x of the entry's object minus 256
WV_PV_ID        = $520F         ; its kind
WV_GHOST        = $5210         ; + slot: non-zero while the object is a ghost in the left margin
WV_PV_L         = $5220         ; the list at hand / 2
WV_PV_I         = $5221         ; the list's entry at hand (in LC_*)
WV_PV_LIST      = $5222         ; 6 bytes: a list's state (LIST_PTR, LIST_BITS_LEFT, LIST_BITS, LIST_NEXT)
WV_PV_TEMP      = $5228         ; 4 bytes: $054C-$054F, kept
WV_PV_E         = $522C         ; the cache entry at hand
WV_PV_SLOT      = $522D         ; the free slot borrowed
WV_PV_X         = $522E         ; 2 bytes: the preview's x
WV_PV_GLOBALS   = $5230         ; 10 bytes: the game's variables an object's creation writes, kept
WV_PV_LIMIT     = $523A         ; 2 bytes: M + 32, how far past 256 an object may show
WV_PV_MADE      = $523C         ; non-zero once an entry was read or a preview created this frame
WV_PV_Y0        = $523D         ; the object's y as its sprites are captured
WV_PV_XOFF      = $523E         ; a captured sprite's x - the object's
WV_PV_DRAWING   = $52E0         ; 13 bytes: the drawing's variables ($58-$64), kept
MAGIC           = $A5

; The previews: one entry per object of the lists seen ahead, created once; its sprites captured
; then (at most PV_SPRITES), placed again from them each frame.
PV_MAX          = 8
PV_SPRITES      = 8
PV_USED         = $5240         ; + entry: non-zero when in use
PV_SEEN         = $5248         ; seen this frame
PV_KS           = $5250         ; key: the list, the entry's position (low, high)
PV_KLO          = $5258
PV_KHI          = $5260
PV_Y            = $5268         ; y (it moves with the screen as the objects' do)
PV_NSPR         = $5270         ; sprites captured
WV_PV_LAST_I    = $5278         ; past the list's last entry read
WV_PV_TO        = $5279         ; entries moved down: to, from, how many
WV_PV_FROM      = $527A
WV_PV_LEFT      = $527B         ; (also the sprites still to place)
WV_PV_SPR       = $527C         ; the sprite at hand in PV_SPR
WV_PV_SX        = $527D         ; 2 bytes: its x
WV_PV_SY        = $527F         ; its y
WV_GR_POS       = $5280         ; the ground past the game's 5 parts: x in its part of 128 pixels,
WV_GR_K         = $5281         ; the part (5 and up), the round's section read, the part it is,
WV_GR_SEC       = $5282         ; the ground's y, the section's 3rd byte
WV_GR_J         = $5283
WV_GR_H         = $5284
WV_GR_B3        = $5285
WV_GR_Y         = $5286         ; the ground's y at the object's x, and its GROUND_Y
WV_GR_3E        = $5287
PV_SPR          = $5600         ; + 32 x entry + 4 x sprite: y - PV_Y, tile, attributes, x - its x

; The lists read ahead: per list (L = 0-2), its state the last time it was read and the state the
; reading went on from (the shadow), and up to LC_MAX entries read (index 4 L + j).
LC_MAX          = 4
LC_STATE        = $5500         ; + 6 L
LC_SHADOW       = $5512         ; + 6 L
LC_COUNT        = $5524         ; + L: entries read
LC_END          = $5527         ; + L: non-zero once the shadow's list is over
LC_PLO          = $5530         ; + entry: its position (the list's LIST_NEXT before it)
LC_PHI          = $553C
LC_ID           = $5548         ; its kind
LC_E            = $5554         ; its extra bytes ($054E, $054F)
LC_F            = $5560
LC_AFTER        = $5570         ; + 6 x entry: the list's state once the game has created it
WV_SLOT_SAVE    = $52C0         ; 20 bytes: the borrowed slot's variables

; The game.
SCRATCH_LO      = $00           ; the object's half width, 16-bit (sub_8310), at the left test
SCRATCH_HI      = $01
DATA_PTR        = $00           ; the sprites' data (sub_C301)
PART_TILE       = $02           ; the first tile of the sprites ($60-$8F, $B0 and up)
PART_ATTR       = $03           ; their attributes
NMI_BUSY        = $0C           ; non-zero: the NMI does nothing (the game's guard against re-entry)
SCROLL_LO       = $06           ; the screen's left edge in the round: low, high
SCROLL_HI       = $3A
CAMERA_DY       = $3D           ; the vertical scrolling of this frame (added to every object's y)
GROUND_Y        = $3E           ; written by the ground lookup (sub_C1EA)
LIST_PTR        = $7C           ; + list: the object lists' read pointers
ROUND_SECTIONS  = $1A           ; the round's sections, 3 bytes each
SECTION         = $3B           ; the section of the ground's part 3 (sub_BD75 at $BEC8)
SECTION_END     = $2F           ; the round's sections
NEW_DATA        = $82           ; a new object: its data pointer, x (low, high), y, kind
NEW_X_LO        = $84
NEW_X_HI        = $85
NEW_Y           = $86
NEW_KIND        = $87
SPRITE_ANIM     = $58           ; the animation drawn (sub_C301)
SPRITE_Y        = $5D
PART_Y          = $60
LIST_NEXT_LO    = $0540         ; + list: the position of its next entry (low, high)
LIST_NEXT_HI    = $0541
LIST_BITS_LEFT  = $0546         ; + list: bits left in the byte read ($FF: the list is over)
LIST_BITS       = $0547
LIST_TEMP       = $054C         ; 4 bytes: the entry read ($054C, $054D, $054E, $054F)
OVERLAY_ROW     = $052E         ; a cloud platform the game is asked to draw ($052E, $052F)
GROUND_PART_Y   = $0506         ; + part (0-4, 128 pixels each from the screen's left edge | 128):
GROUND_PART_TYPE = $050B        ; the ground's y, its shape, and what sub_C1EA puts in GROUND_Y
OBJ_Y           = $05C4
OBJ_DX          = $05E4         ; + slot: the sprites' offsets from x and y
OBJ_DY          = $05F4
OBJ_TIMER       = $0624         ; + slot: the animation's timer and index
OBJ_INDEX       = $0634
OBJ_ANIM_LO     = $0644
OBJ_ANIM_HI     = $0654
OBJ_TYPE        = $0684         ; + slot: how sub_8EC9 draws it (0 not at all)
SPRITE_TILES    = $9F93         ; a single sprite's tile and attributes, by animation
AREA            = $37
ROUND           = $38
DEMO            = $40
SPRITE_X_HI     = $59           ; the object's sprites: x (high, low)
SPRITE_X_LO     = $5A
PART_X_HI       = $5B           ; the sprite being placed: x (high, low)
PART_X_LO       = $5C
OAM_INDEX       = $67
DEATH           = $72
FLAG_70         = $70
HIT_KIND        = $9E           ; $FF: a touch that is not an injury
SOUND           = $A0           ; the sound effect asked for this frame
OBJ_STATE       = $0554         ; + slot: 0 free
OBJ_X_LO        = $0584
OBJ_X_HI        = $0594
OBJ_WIDTH       = $0614         ; + slot, collisions

; Routines and places of the game.
OBJECT_HANDLER  = $929E
SPAWN_OBJECTS   = $8C45         ; creates the objects of the lists the screen reaches
READ_LIST       = $8D74         ; reads Y bits of list X into $00-$01
CREATE_IN_SLOT  = $8DBF         ; creates object NEW_KIND in slot X (its data, then its own setup)
DRAW_OBJECT     = $91EE         ; draws the object of slot X by its type
GROUND_AT       = $C1AD         ; the ground's y at the screen x $00-$01
GROUND_RISE     = $BF43         ; + 3 bits of a section's 2nd byte: the ground's change from the last
SPRITE_SKIP     = $C4C8         ; the sprite loop's next sprite, this one left out
ONE_SPRITE_END  = $C3A3
TITLE_WAIT      = $AE8F         ; the title screen's wait, its demo timer set again

; The options ($500A).
OPT_LEVEL_SELECT = $01
OPT_INVINCIBLE  = $02
OPT_NO_SLOWDOWN = $04
NO_SLOWDOWN_LINES = 255

GHOST_REACH     = 32            ; how far left of the view a ghost's x may be (its sprites' width)
DIGIT_TILE      = $F5           ; the sprite tile of 0 (the status bar's digits)

.segment "CODE"

; ---------------------------------------------------------------------------- ;
; NMI ($B717, in place of LDA #<ram_oam / STA $2003): the extension's settings for the frame.
wide_nmi:
    LDA     WV_MAGIC
    CMP     #MAGIC
    BEQ     @set
    JSR     wide_setup
@set:
    LDA     WIDE_HOST
    CMP     #MAX_MARGIN
    BCC     @margin
    LDA     #MAX_MARGIN
@margin:
    STA     WV_M
    STA     WIDE_REQ
    LDA     #$00                        ; no nametable columns: the decoder draws the background
    STA     WIDE_LEFT
    STA     WIDE_RIGHT
    LDA     #$16                        ; sprites free of those extents, second sprite page, the
    STA     WIDE_LAYOUT                 ; window on over $6000-$7FFF
    LDA     WIDE_OPTIONS                ; no slowdown (option bit 2)
    AND     #OPT_NO_SLOWDOWN
    BEQ     @overclock
    LDA     #NO_SLOWDOWN_LINES
@overclock:
    STA     WIDE_OVERCLOCK
    LDA     #$00
    STA     $2003
    RTS

; The extension's RAM at power-on: no ghost, the second page hidden.
wide_setup:
    LDX     #$0F
    LDA     #$00
@ghosts:
    STA     WV_GHOST,x
    DEX
    BPL     @ghosts
    STA     WV_LEVEL
    STA     WV_GHOST_RUN
    JSR     wide_hide_page
    LDA     #MAGIC
    STA     WV_MAGIC
    RTS

; The second page hidden, the previews' cache emptied.
wide_hide_page:
    LDX     #$00
    LDA     #$F8
@hide:
    STA     EXTRA_OAM,x
    INX
    INX
    INX
    INX
    BNE     @hide
    LDA     #$00
    STA     WV_PREVIEW
    STA     WV_PV_LAST
    LDX     #PV_MAX-1
@cache:
    STA     PV_USED,x
    DEX
    BPL     @cache
    LDX     #$02                        ; the lists read again
@lists:
    STA     LC_COUNT,x
    STA     LC_END,x
    DEX
    BPL     @lists
    LDA     #$AA                        ; a state no list has (bits left)
    STA     LC_STATE+2
    STA     LC_STATE+2+6
    STA     LC_STATE+2+12
    RTS

; ---------------------------------------------------------------------------- ;
; Sprites. The game places an object's sprites from its x (SPRITE_X, 16-bit, the screen's left
; edge at 0) and leaves out every sprite whose x is not on the screen. Within the wide view a
; sprite shows: its high byte goes to the extension.

; C set when the 16-bit x A:Y (high:low) of a sprite (8 pixels wide) is within the wide view:
; -M-8 < x < 256+M.
wide_x_shown:
    CMP     #$00
    BEQ     @shown
    CMP     #$01
    BEQ     @right
    CMP     #$FF
    BNE     @hidden
    LDA     WV_M
    BEQ     @hidden
    TYA
    CLC
    ADC     #$08
    BCS     @shown
    ADC     WV_M
    BCS     @shown
    BCC     @hidden
@right:
    CPY     WV_M
    BCC     @shown
@hidden:
    CLC
    RTS
@shown:
    SEC
    RTS

; An object's sprites (sub_C301, $C305, in place of LDY $59 / DEY / BPL RTS: the game gives up an
; object whose x is past the screen's right edge). Given up when it is past the wide view; A (the
; animation) kept.
wide_object_shown:
    STA     WV_A
    LDA     WV_PREVIEW                  ; a preview's sprites are all captured
    BNE     @shown
    LDA     SPRITE_X_HI
    BEQ     @shown
    CMP     #$01
    BEQ     @right
    CMP     #$FF
    BNE     @hidden
    LDA     SPRITE_X_LO                 ; left: x + 32 + M >= 0 (its sprites reach 32 pixels on)
    CLC
    ADC     #GHOST_REACH
    BCS     @shown
    ADC     WV_M
    BCS     @shown
    BCC     @hidden
@right:
    LDA     SPRITE_X_LO
    CMP     WV_M
    BCC     @shown
@hidden:
    PLA                                 ; out of sub_C301
    PLA
@shown:
    LDA     WV_A
    RTS

; One sprite of an object ($C48B and $C4A9, in place of LDA $5B / BNE skip; X its OAM offset):
; within the wide view it is placed with its high byte, otherwise left out. A preview's sprite is
; captured instead (wide_pv_capture), made as the game would make it: $C48B's sprites take their tile
; from PART_TILE plus the data's low 6 bits and their attributes from PART_ATTR flipped by its top
; 2 bits, $C4A9's the data as tile and PART_ATTR.
wide_sprite_at:
    LDA     WV_PREVIEW
    BNE     wide_sprite_capture
wide_sprite_view:
    STY     WV_T0
    LDY     PART_X_LO
    LDA     PART_X_HI
    JSR     wide_x_shown
    LDY     WV_T0
    BCC     wide_sprite_skip
    LDA     PART_X_HI
    STA     WIDE_SPRHI,x
    RTS
wide_sprite_capture:
    LDA     (DATA_PTR),y
    CMP     #$FF
    BEQ     wide_sprite_skip
    AND     #$C0
    EOR     PART_ATTR
    STA     WV_T1
    LDA     (DATA_PTR),y
    AND     #$3F
    CLC
    ADC     PART_TILE
    JSR     wide_pv_capture
wide_sprite_skip:
    PLA                                 ; out of the hook
    PLA
    JMP     SPRITE_SKIP

wide_sprite_tayx:
    LDA     WV_PREVIEW
    BEQ     wide_sprite_view
    LDA     PART_ATTR
    STA     WV_T1
    LDA     (DATA_PTR),y
    CMP     #$FF
    BEQ     wide_sprite_skip
    JSR     wide_pv_capture
    JMP     wide_sprite_skip

; A single-sprite object (sub_C301, $C378, in place of LDA $59 / BNE end).
wide_one_sprite:
    LDA     WV_PREVIEW
    BNE     @preview
    LDY     SPRITE_X_LO
    LDA     SPRITE_X_HI
    JSR     wide_x_shown
    BCC     @skip
    LDX     OAM_INDEX
    LDA     SPRITE_X_HI
    STA     WIDE_SPRHI,x
    RTS
@preview:
    LDA     SPRITE_Y                    ; as $C37C-$C39B make it
    CMP     #$F0
    BCS     @skip
    STA     PART_Y
    LDA     SPRITE_X_LO
    STA     PART_X_LO
    LDA     SPRITE_X_HI
    STA     PART_X_HI
    LDA     SPRITE_ANIM
    ASL
    TAY
    LDA     SPRITE_TILES+1,y
    STA     WV_T1
    LDA     SPRITE_TILES,y
    JSR     wide_pv_capture
@skip:
    PLA
    PLA
    JMP     ONE_SPRITE_END

; A preview's sprite captured into cache entry WV_PV_E: tile A, attributes WV_T1, y PART_Y, x PART_X
; (at most PV_SPRITES, x within 128 pixels of the object's); Y kept.
wide_pv_capture:
    STY     WV_T0
    STA     WV_A
    LDY     WV_PV_E
    LDA     PV_NSPR,y
    CMP     #PV_SPRITES
    BCS     @done
    LDA     PART_X_LO                   ; x - the object's x, within -128..127
    SEC
    SBC     WV_PV_X
    STA     WV_PV_XOFF
    LDA     PART_X_HI
    SBC     WV_PV_X+1
    BEQ     @positive
    CMP     #$FF
    BNE     @done
    LDA     WV_PV_XOFF
    BPL     @done
    BMI     @near
@positive:
    LDA     WV_PV_XOFF
    BMI     @done
@near:
    TYA                                 ; its place: 32 x entry + 4 x sprite
    ASL
    ASL
    ASL
    ADC     PV_NSPR,y
    ASL
    ASL
    TAY
    LDA     PART_Y
    SEC
    SBC     WV_PV_Y0
    STA     PV_SPR,y
    LDA     WV_A
    STA     PV_SPR+1,y
    LDA     WV_T1
    STA     PV_SPR+2,y
    LDA     WV_PV_XOFF
    STA     PV_SPR+3,y
    LDY     WV_PV_E
    LDA     PV_NSPR,y
    CLC
    ADC     #$01
    STA     PV_NSPR,y
@done:
    LDY     WV_T0
    RTS

; ---------------------------------------------------------------------------- ;
; Ghosts. The game removes an object whose left edge (x minus its half width) goes past the
; screen's left edge ($8EA7). Such an object stays as a ghost while it shows in the left margin:
; its behaviour still runs ($8E8E), but it collides with nothing ($BFF2), the objects it would
; create are not ($8DB6) and its sounds are not played, so the game is the original's. A ghost
; that comes back onto the screen goes, as the game had it go.

; The left test ($8EA7, in place of the 16-bit x - half width / BMI remove; $00/$01 the half width):
; C set when the object goes.
wide_left_gone:
    LDA     OBJ_X_LO,x
    SEC
    SBC     SCRATCH_LO
    LDA     OBJ_X_HI,x
    SBC     SCRATCH_HI
    BMI     @left
    LDA     WV_GHOST,x                  ; on the screen: a ghost back there goes
    BNE     @gone
    CLC
    RTS
@left:
    LDA     WV_M                        ; past the left edge: x + 32 + M >= 0 keeps it, a ghost
    BEQ     @gone
    LDA     OBJ_X_HI,x
    BEQ     @ghost
    CMP     #$FF
    BNE     @gone
    LDA     OBJ_X_LO,x
    CLC
    ADC     #GHOST_REACH
    BCS     @ghost
    ADC     WV_M
    BCC     @gone
@ghost:
    LDA     #$01
    STA     WV_GHOST,x
    CLC
    RTS
@gone:
    SEC
    RTS

; The object's behaviour ($8E8E, in place of JSR sub_929E_object_handler): a ghost's runs without
; its sounds or the objects it would create.
wide_handler:
    LDA     WV_GHOST,x
    BNE     @ghost
    JMP     OBJECT_HANDLER
@ghost:
    STA     WV_GHOST_RUN
    LDA     SOUND
    STA     WV_SOUND
    JSR     OBJECT_HANDLER
    LDA     WV_SOUND
    STA     SOUND
    LDA     #$00
    STA     WV_GHOST_RUN
    RTS

; Two objects' collision (sub_BFF2, in place of LDA $0614,X / CLC / ADC $0614,Y): none with a
; ghost.
wide_collision:
    LDA     WV_GHOST,x
    ORA     WV_GHOST,y
    BNE     @none
    LDA     OBJ_WIDTH,x
    CLC
    ADC     OBJ_WIDTH,y
    RTS
@none:
    PLA                                 ; out of sub_BFF2: no collision
    PLA
    CLC
    RTS

; A new object's slot (sub_8DB4, $8DB6, in place of its search): a free one, or else a ghost's;
; none for an object a ghost would create. C set when found, X the slot.
wide_find_slot:
    LDA     WV_GHOST_RUN
    BEQ     @search
    CLC
    RTS
@search:
    LDX     #$0F
@free:
    LDA     OBJ_STATE,x
    BEQ     @found
    DEX
    BNE     @free
    LDX     #$0F
@ghost:
    LDA     WV_GHOST,x
    BNE     @found
    DEX
    BNE     @ghost
    CLC
    RTS
@found:
    LDA     #$00
    STA     WV_GHOST,x
    SEC
    RTS

; An object's state set ($9534, in place of STA $0554,X / RTS: creation, removal): a ghost no more.
wide_object_state:
    STA     OBJ_STATE,x
    PHA
    LDA     #$00
    STA     WV_GHOST,x
    PLA
    RTS

; Every object cleared (sub_8C35, $8C41, in place of STX $0536): no ghost or preview either.
wide_clear_ghosts:
    STX     $0536
    LDA     #$00
    LDY     #$0F
@clear:
    STA     WV_GHOST,y
    DEY
    BPL     @clear
    JMP     wide_hide_page              ; no preview either

; ---------------------------------------------------------------------------- ;
; Previews. The game creates an object when the screen comes 64 pixels near its entry in the
; round's three object lists ($8C5A): at x = 320, on to 400 past a margin of 64. Before that, its
; preview shows it still where it will be created. The lists are read ahead as the game reads them
; (its own routine, on a copy of their state), an entry once only. An entry's object is created
; once, in a free slot (the game's data and its kind's setup), and drawn there by the game, its
; sprites captured; each frame they are placed again in the second sprite page as the screen
; scrolls. Every slot and variable the creation writes is put back: the game is the original's.

PV_REACH        = 32            ; an object's sprites reach 32 pixels left of its x
PV_LAST_KIND    = $3F           ; kinds past it (the player's, the bosses') have no preview
DRAWING_VARS    = 13            ; sub_C301's variables, $58-$64

; The main loop's object creation ($80EC, in place of JSR sub_8C45): the previews first (the
; entries due this frame still in the lists, shown once more; the game draws their objects from
; the next frame on), then the game's creation. Off, it costs a few tests.
wide_spawn:
    LDA     WV_M                        ; the objects show from 320 on: nothing to preview within it
    CMP     #64 + 8
    BCC     @off
    LDA     SCROLL_HI                   ; the game creates nothing there either
    BMI     @off
    LDA     DEATH                       ; nor while the player dies and the round starts again
    BNE     @off                        ; (the screen run fast from the checkpoint)
    JSR     wide_previews
    JMP     SPAWN_OBJECTS
@off:
    LDA     WV_PV_LAST                  ; the previews shown hidden
    BEQ     @spawn
    LDA     #$00
    STA     WV_PV_N
    STA     WV_PV_FULL
    LDX     #PV_MAX-1
@unseen:
    STA     PV_SEEN,x
    DEX
    BPL     @unseen
    JSR     wide_pv_end
@spawn:
    JMP     SPAWN_OBJECTS

wide_previews:
    LDA     #$00
    STA     WV_PV_N
    STA     WV_PV_FULL
    STA     WV_PV_MADE
    LDX     #PV_MAX-1
@unseen:
    STA     PV_SEEN,x
    DEX
    BPL     @unseen
    LDA     WV_M                        ; M + 32
    CLC
    ADC     #PV_REACH
    STA     WV_PV_LIMIT
    LDA     #$00
    ADC     #$00
    STA     WV_PV_LIMIT+1
    LDX     #$03
@keep_temp:
    LDA     LIST_TEMP,x
    STA     WV_PV_TEMP,x
    DEX
    BPL     @keep_temp
    LDA     #$02
@list:
    STA     WV_PV_L
    ASL
    STA     WV_PV_S
    JSR     wide_lc_sync
    JSR     wide_pv_list
    LDA     WV_PV_L
    SEC
    SBC     #$01
    BPL     @list
    LDX     #$03
@temp_back:
    LDA     WV_PV_TEMP,x
    STA     LIST_TEMP,x
    DEX
    BPL     @temp_back
    JMP     wide_pv_end

; WV_PV_D = the 16-pixel position A:X (high:low) - the screen's left edge: x - 256 of its object.
; N set when it is left of x = 256; else C set when it is past the view (x - 256 >= M + 32).
wide_pv_dist:
    PHA
    TXA
    AND     #$F0
    SEC
    SBC     SCROLL_LO
    STA     WV_PV_D
    PLA
    SBC     SCROLL_HI
    STA     WV_PV_D+1
    BMI     @left
    LDA     WV_PV_D
    CMP     WV_PV_LIMIT
    LDA     WV_PV_D+1
    SBC     WV_PV_LIMIT+1
    LDA     #$00                        ; N clear
@left:
    RTS

; The previews of list WV_PV_L's entries read, as far as the view goes.
wide_pv_list:
    LDA     WV_PV_L
    ASL
    ASL
    STA     WV_PV_I
    CLC
    LDX     WV_PV_L
    ADC     LC_COUNT,x
    STA     WV_PV_LAST_I                ; past its last entry
@entry:
    LDY     WV_PV_I
    CPY     WV_PV_LAST_I
    BCS     @done
    LDX     LC_PLO,y
    LDA     LC_PHI,y
    JSR     wide_pv_dist
    BMI     @next                       ; past already: the game will not create it
    BCS     @done
    JSR     wide_pv_entry
@next:
    INC     WV_PV_I
    BNE     @entry
@done:
    RTS

; ---------------------------------------------------------------------------- ;
; The lists read ahead.

; WV_PV_LIST = list WV_PV_S's state.
wide_lc_get:
    LDX     WV_PV_S
    LDA     LIST_PTR,x
    STA     WV_PV_LIST
    LDA     LIST_PTR+1,x
    STA     WV_PV_LIST+1
    LDA     LIST_BITS_LEFT,x
    STA     WV_PV_LIST+2
    LDA     LIST_BITS,x
    STA     WV_PV_LIST+3
    LDA     LIST_NEXT_LO,x
    STA     WV_PV_LIST+4
    LDA     LIST_NEXT_HI,x
    STA     WV_PV_LIST+5
    RTS

; List WV_PV_S's state = WV_PV_LIST.
wide_lc_put:
    LDX     WV_PV_S
    LDA     WV_PV_LIST
    STA     LIST_PTR,x
    LDA     WV_PV_LIST+1
    STA     LIST_PTR+1,x
    LDA     WV_PV_LIST+2
    STA     LIST_BITS_LEFT,x
    LDA     WV_PV_LIST+3
    STA     LIST_BITS,x
    LDA     WV_PV_LIST+4
    STA     LIST_NEXT_LO,x
    LDA     WV_PV_LIST+5
    STA     LIST_NEXT_HI,x
    RTS

; Z set when WV_PV_LIST equals the 6 bytes at LC_STATE + Y (LC_SHADOW and LC_AFTER follow
; LC_STATE: Y = their offset from it).
wide_lc_same:
    LDX     #$00
@byte:
    LDA     WV_PV_LIST,x
    CMP     LC_STATE,y
    BNE     @done
    INY
    INX
    CPX     #$06
    BNE     @byte
@done:
    RTS

; The 6 bytes at LC_STATE + Y = WV_PV_LIST.
wide_lc_store:
    LDX     #$00
@byte:
    LDA     WV_PV_LIST,x
    STA     LC_STATE,y
    INY
    INX
    CPX     #$06
    BNE     @byte
    RTS

; WV_PV_LIST = the 6 bytes at LC_STATE + Y.
wide_lc_load:
    LDX     #$00
@byte:
    LDA     LC_STATE,y
    STA     WV_PV_LIST,x
    INY
    INX
    CPX     #$06
    BNE     @byte
    RTS

; Y = 6 x A + the offset from LC_STATE of: list WV_PV_L's state, its shadow, entry A's state after.
wide_lc_state:
    LDA     WV_PV_L
    JSR     wide_x6
    RTS

wide_lc_shadow:
    LDA     WV_PV_L
    JSR     wide_x6
    TYA
    CLC
    ADC     #LC_SHADOW - LC_STATE
    TAY
    RTS

wide_lc_after:
    JSR     wide_x6
    TYA
    CLC
    ADC     #LC_AFTER - LC_STATE
    TAY
    RTS

; Y = 6 x A.
wide_x6:
    ASL
    STA     WV_T0
    ASL
    CLC
    ADC     WV_T0
    TAY
    RTS

; List WV_PV_L's entries read made to follow the game: those it has created since dropped, or all
; of them when its state is not one they lead to; then one more read, when the next one is within
; the view.
wide_lc_sync:
    JSR     wide_lc_get
    LDX     WV_PV_L
    LDA     WV_PV_LIST+2                ; the list is over
    BPL     @going
    LDA     #$00
    STA     LC_COUNT,x
    LDA     #$01
    STA     LC_END,x
    RTS
@going:
    JSR     wide_lc_state
    JSR     wide_lc_same
    BEQ     @read
    LDA     #$00                        ; which entry the game is past
    STA     WV_PV_I
@past:
    LDX     WV_PV_L
    LDA     WV_PV_I
    CMP     LC_COUNT,x
    BCS     @again
    LDA     WV_PV_L
    ASL
    ASL
    ADC     WV_PV_I
    JSR     wide_lc_after
    JSR     wide_lc_same
    BEQ     @drop
    INC     WV_PV_I
    BNE     @past
@again:
    LDX     WV_PV_L                     ; read again from the game's state
    LDA     #$00
    STA     LC_COUNT,x
    STA     LC_END,x
    JSR     wide_lc_shadow
    JSR     wide_lc_store
    JMP     @state
@drop:
    JSR     wide_lc_drop
@state:
    JSR     wide_lc_state
    JSR     wide_lc_store
@read:
    LDA     WV_PV_MADE                  ; one entry read or one creation a frame
    BNE     @done
    LDX     WV_PV_L
    LDA     LC_END,x
    BNE     @done
    LDA     LC_COUNT,x
    CMP     #LC_MAX
    BCS     @done
    JSR     wide_lc_shadow              ; the shadow's next entry within the view
    LDX     LC_STATE+4,y
    LDA     LC_STATE+5,y
    JSR     wide_pv_dist
    BMI     @in
    BCS     @done
@in:
    JMP     wide_lc_read
@done:
    RTS

; List WV_PV_L's entries 0 to WV_PV_I dropped (the game has created them).
wide_lc_drop:
    LDA     WV_PV_L
    ASL
    ASL
    STA     WV_PV_TO
    SEC
    ADC     WV_PV_I
    STA     WV_PV_FROM
    LDX     WV_PV_L
    LDA     LC_COUNT,x
    CLC
    SBC     WV_PV_I                     ; count - (I + 1)
    STA     LC_COUNT,x
    BEQ     @done
    STA     WV_PV_LEFT
@entry:
    LDY     WV_PV_FROM
    LDX     WV_PV_TO
    LDA     LC_PLO,y
    STA     LC_PLO,x
    LDA     LC_PHI,y
    STA     LC_PHI,x
    LDA     LC_ID,y
    STA     LC_ID,x
    LDA     LC_E,y
    STA     LC_E,x
    LDA     LC_F,y
    STA     LC_F,x
    LDA     WV_PV_FROM
    JSR     wide_lc_after
    JSR     wide_lc_load
    LDA     WV_PV_TO
    JSR     wide_lc_after
    JSR     wide_lc_store
    INC     WV_PV_TO
    INC     WV_PV_FROM
    DEC     WV_PV_LEFT
    BNE     @entry
@done:
    RTS

; One more entry of list WV_PV_L read from its shadow, with the game's routine on the list's own
; variables (the game's state kept meanwhile in the stack).
wide_lc_read:
    INC     WV_PV_MADE
    JSR     wide_lc_get
    LDX     #$05
@keep:
    LDA     WV_PV_LIST,x
    PHA
    DEX
    BPL     @keep
    JSR     wide_lc_shadow              ; the shadow in the list's variables
    JSR     wide_lc_load
    JSR     wide_lc_put
    LDX     WV_PV_L                     ; the entry, as $8C5A reads it
    LDA     WV_PV_L
    ASL
    ASL
    ADC     LC_COUNT,x
    STA     WV_PV_I
    TAY
    LDX     WV_PV_S
    LDA     LIST_NEXT_LO,x
    STA     LC_PLO,y
    LDA     LIST_NEXT_HI,x
    STA     LC_PHI,y
    LDY     #$06
    JSR     READ_LIST
    LDY     WV_PV_I
    LDA     $00
    STA     LC_ID,y
    CMP     #$38
    BCC     @next
    LDY     #$08
    JSR     READ_LIST
    LDY     WV_PV_I
    LDA     $00
    STA     LC_E,y
    LDA     LC_ID,y
    CMP     #$3E
    BCC     @next
    LDY     #$08
    JSR     READ_LIST
    LDY     WV_PV_I
    LDA     $00
    STA     LC_F,y
@next:
    LDY     #$01                        ; the next position: 1 bit, then 10 bits (0 ends the list)
    JSR     READ_LIST                   ; or 6 bits added, x 8
    LSR     $00
    BCC     @step
    LDY     #$0A
    JSR     READ_LIST
    LDA     $00
    ORA     $01
    BNE     @at
    LDA     #$FF                        ; over, as $8D39
    STA     LIST_BITS_LEFT,x
    LDY     WV_PV_L
    STA     LC_END,y
    BNE     @read
@at:
    JSR     wide_pv_x8
    LDA     $00
    STA     LIST_NEXT_LO,x
    LDA     $01
    STA     LIST_NEXT_HI,x
    JMP     @read
@step:
    LDY     #$06
    JSR     READ_LIST
    JSR     wide_pv_x8
    LDA     LIST_NEXT_LO,x
    CLC
    ADC     $00
    STA     LIST_NEXT_LO,x
    LDA     LIST_NEXT_HI,x
    ADC     $01
    STA     LIST_NEXT_HI,x
@read:
    JSR     wide_lc_get                 ; the state after it: the shadow's, and the game's once the
    LDA     WV_PV_I                     ; game has created it
    JSR     wide_lc_after
    JSR     wide_lc_store
    JSR     wide_lc_shadow
    JSR     wide_lc_store
    LDX     WV_PV_L
    INC     LC_COUNT,x
    LDX     #$00                        ; the game's state back
@back:
    PLA
    STA     WV_PV_LIST,x
    INX
    CPX     #$06
    BNE     @back
    JMP     wide_lc_put

wide_pv_x8:
    LDY     #$03
@shift:
    ASL     $00
    ROL     $01
    DEY
    BNE     @shift
    RTS

; ---------------------------------------------------------------------------- ;
; The previews' entries.

; The preview of entry WV_PV_I of list WV_PV_L (its object at x WV_PV_D + 256): its cache entry,
; created the first time it is seen (one a frame), then its sprites placed.
wide_pv_entry:
    LDA     WV_PV_D                     ; the preview's x
    STA     WV_PV_X
    LDX     WV_PV_D+1
    INX
    STX     WV_PV_X+1
    LDY     WV_PV_I
    LDX     #PV_MAX-1
@find:
    LDA     PV_USED,x
    BEQ     @other
    LDA     PV_KS,x
    CMP     WV_PV_L
    BNE     @other
    LDA     PV_KLO,x
    CMP     LC_PLO,y
    BNE     @other
    LDA     PV_KHI,x
    CMP     LC_PHI,y
    BEQ     @found
@other:
    DEX
    BPL     @find
    LDA     WV_PV_MADE                  ; a new one
    BNE     @none
    LDA     LC_ID,y
    CMP     #PV_LAST_KIND + 1
    BCS     @none
    LDX     #PV_MAX-1
@free:
    LDA     PV_USED,x
    BEQ     @create
    DEX
    BPL     @free
@none:
    RTS
@create:
    STX     WV_PV_E
    JSR     wide_pv_create
    BCC     @none
    LDX     WV_PV_E
    BCS     @place
@found:
    LDA     PV_Y,x                      ; the screen scrolled up or down as the objects' y
    CLC
    ADC     CAMERA_DY
    STA     PV_Y,x
@place:
    LDA     #$01
    STA     PV_SEEN,x
    LDA     PV_NSPR,x
    BEQ     @none
    STA     WV_PV_LEFT                  ; sprites to place
    LDA     PV_Y,x
    STA     WV_PV_Y0
    TXA
    ASL
    ASL
    ASL
    ASL
    ASL
    STA     WV_PV_SPR                   ; its first sprite
@sprite:
    LDY     WV_PV_SPR
    LDA     PV_SPR+3,y                  ; x = its x + the offset
    CLC
    ADC     WV_PV_X
    STA     WV_PV_SX
    LDA     #$00
    LDX     PV_SPR+3,y
    BPL     @high
    LDA     #$FF
@high:
    ADC     WV_PV_X+1
    STA     WV_PV_SX+1
    LDA     PV_SPR,y                    ; y = its y + the offset
    CLC
    ADC     WV_PV_Y0
    STA     WV_PV_SY
    LDY     WV_PV_SX
    LDA     WV_PV_SX+1
    JSR     wide_x_shown
    BCC     @hidden
    LDY     WV_PV_FULL
    BNE     @hidden
    LDX     WV_PV_SPR
    LDY     WV_PV_N
    LDA     WV_PV_SY
    STA     EXTRA_OAM,y
    LDA     PV_SPR+1,x
    STA     EXTRA_OAM+1,y
    LDA     PV_SPR+2,x
    STA     EXTRA_OAM+2,y
    LDA     WV_PV_SX
    STA     EXTRA_OAM+3,y
    LDA     WV_PV_SX+1
    STA     EXTRA_HI,y
    TYA
    CLC
    ADC     #$04
    STA     WV_PV_N
    BNE     @hidden
    INC     WV_PV_FULL
@hidden:
    LDA     WV_PV_SPR
    CLC
    ADC     #$04
    STA     WV_PV_SPR
    DEC     WV_PV_LEFT
    BNE     @sprite
    RTS

; A free slot in WV_PV_SLOT, X; C clear when none.
wide_pv_slot:
    LDX     #$0F
@search:
    LDA     OBJ_STATE,x
    BEQ     @found
    DEX
    BNE     @search
    CLC
    RTS
@found:
    STX     WV_PV_SLOT
    SEC
    RTS

; The ground's y at x WV_PV_X, as sub_C1AD gives it when the game creates an object there, in
; WV_GR_Y (and the GROUND_Y it leaves in WV_GR_3E). The game knows 5 parts of 128 pixels
; ((left edge | 128) + x) / 128: past them, the next parts as $BEDE would make them from the round's
; sections. C clear when it is past the round's end, or too far.
wide_pv_ground:
    LDA     SCROLL_LO
    ORA     #$80
    CLC
    ADC     WV_PV_X
    STA     WV_GR_POS
    LDA     WV_PV_X+1
    ADC     #$00
    ASL     WV_GR_POS
    ROL                                 ; the part
    CMP     #$05
    BCS     @far
    LDA     GROUND_Y
    PHA
    LDA     WV_PV_X
    STA     $00
    LDA     WV_PV_X+1
    STA     $01
    JSR     GROUND_AT
    STA     WV_GR_Y
    LDA     GROUND_Y
    STA     WV_GR_3E
    PLA
    STA     GROUND_Y
    SEC
    RTS
@far:
    CMP     #$08
    BCS     @cannot
    STA     WV_GR_K
    LSR     WV_GR_POS                   ; x within the part
    LDA     GROUND_PART_Y+4             ; part 4, and on (part n: section SECTION - 3 + n)
    STA     WV_GR_H
    LDA     SECTION
    CLC
    ADC     #$02
    STA     WV_GR_SEC
    LDA     #$05
    STA     WV_GR_J
@section:
    LDA     WV_GR_SEC
    CMP     SECTION_END
    BCS     @cannot
    STA     WV_T0
    ASL
    ADC     WV_T0
    TAY
    INY
    LDA     (ROUND_SECTIONS),y          ; 2nd byte: the ground's change
    LSR
    LSR
    AND     #$07
    TAX
    LDA     WV_GR_H
    CLC
    ADC     GROUND_RISE,x
    STA     WV_GR_H
    INY
    LDA     (ROUND_SECTIONS),y          ; 3rd byte: GROUND_Y (high 4 bits), the shape
    STA     WV_GR_B3
    LDA     WV_GR_J
    CMP     WV_GR_K
    BEQ     @part
    INC     WV_GR_J
    INC     WV_GR_SEC
    BNE     @section
@cannot:
    CLC
    RTS
@part:
    LDA     WV_GR_B3                    ; as sub_C1EA
    AND     #$F0
    STA     WV_GR_3E
    LDA     WV_GR_POS
    LSR
    LSR
    STA     WV_T0                       ; x / 4 for the slopes
    LDA     WV_GR_B3
    AND     #$0F
    CMP     #$02
    BEQ     @down
    CMP     #$03
    BEQ     @up
    CMP     #$04
    BCC     @flat
    CMP     #$07
    BCS     @flat
    TAY                                 ; 4-6: a step at 64
    LDA     WV_GR_POS
    CMP     #$40
    BCC     @flat
    LDA     #$E0
    CPY     #$04
    BEQ     @step
    LDA     #$20
@step:
    CLC
    ADC     WV_GR_H
    STA     WV_GR_Y
    SEC
    RTS
@down:
    LDA     WV_GR_H
    SEC
    SBC     WV_T0
    STA     WV_GR_Y
    SEC
    RTS
@up:
    LDA     WV_GR_H
    CLC
    ADC     WV_T0
    STA     WV_GR_Y
    SEC
    RTS
@flat:
    LDA     WV_GR_H
    STA     WV_GR_Y
    SEC
    RTS

; Entry WV_PV_I's object created in a free slot and drawn there, its sprites captured into cache
; entry WV_PV_E; the slot and the game's variables as they were after. C clear when it cannot be
; yet (no free slot, its ground not known).
wide_pv_create:
    JSR     wide_pv_ground
    BCS     @ground
@cannot:
    CLC
    RTS
@ground:
    JSR     wide_pv_slot
    BCC     @cannot
    LDA     #$01                        ; an NMI meanwhile does nothing: it must not see the game's
    STA     NMI_BUSY                    ; variables as the creation leaves them
    STA     WV_PV_MADE
    .repeat 20, k
        LDA     $0554 + k * $10, x
        STA     WV_SLOT_SAVE + k
    .endrepeat
    LDA     SOUND
    STA     WV_PV_GLOBALS
    LDA     GROUND_Y
    STA     WV_PV_GLOBALS+1
    LDA     OVERLAY_ROW
    STA     WV_PV_GLOBALS+2
    LDA     OVERLAY_ROW+1
    STA     WV_PV_GLOBALS+3
    LDY     #$05
@keep_new:
    LDA     NEW_DATA,y
    STA     WV_PV_GLOBALS+4,y
    DEY
    BPL     @keep_new
    LDX     #DRAWING_VARS - 1           ; the drawing's own variables
@keep_drawing:
    LDA     SPRITE_ANIM,x
    STA     WV_PV_DRAWING,x
    DEX
    BPL     @keep_drawing
    LDA     #$01                        ; it creates no object
    STA     WV_GHOST_RUN
    LDY     WV_PV_I                     ; the entry as $8C5A leaves it: $054C-$054D the position
    LDA     LC_PLO,y                    ; x 8 with the kind in the low 6 bits, $054E-$054F
    STA     LIST_TEMP
    LDA     LC_PHI,y
    STA     LIST_TEMP+1
    LDX     #$03
@x8:
    ASL     LIST_TEMP
    ROL     LIST_TEMP+1
    DEX
    BNE     @x8
    LDA     LC_ID,y
    STA     NEW_KIND
    ORA     LIST_TEMP
    STA     LIST_TEMP
    LDA     LC_E,y
    STA     LIST_TEMP+2
    LDA     LC_F,y
    STA     LIST_TEMP+3
    LDA     WV_PV_X                     ; then $8CD8-$8CF0
    STA     NEW_X_LO
    LDA     WV_PV_X+1
    STA     NEW_X_HI
    LDA     WV_GR_3E
    STA     GROUND_Y
    LDA     WV_GR_Y
    STA     NEW_Y
    LDX     WV_PV_SLOT
    JSR     CREATE_IN_SLOT
    LDY     WV_PV_E
    LDA     #$00
    STA     PV_NSPR,y
    LDX     WV_PV_SLOT
    LDA     OBJ_Y,x
    STA     PV_Y,y
    STA     WV_PV_Y0
    LDA     OBJ_STATE,x                 ; its setup removed it: nothing to show
    BEQ     @drawn
    LDA     OBJ_TYPE,x                  ; drawn by sub_8EC9 (type 0: not, or by its behaviour)
    BEQ     @drawn
    LDA     #$01
    STA     WV_PREVIEW
    JSR     DRAW_OBJECT
    LDA     #$00
    STA     WV_PREVIEW
@drawn:
    LDY     WV_PV_E
    LDA     WV_PV_L
    STA     PV_KS,y
    LDX     WV_PV_I
    LDA     LC_PLO,x
    STA     PV_KLO,y
    LDA     LC_PHI,x
    STA     PV_KHI,y
    LDA     #$01
    STA     PV_USED,y
    LDX     WV_PV_SLOT                  ; the slot and the variables back
    .repeat 20, k
        LDA     WV_SLOT_SAVE + k
        STA     $0554 + k * $10, x
    .endrepeat
    LDA     #$00
    STA     WV_GHOST,x
    STA     WV_GHOST_RUN
    LDA     WV_PV_GLOBALS
    STA     SOUND
    LDA     WV_PV_GLOBALS+1
    STA     GROUND_Y
    LDA     WV_PV_GLOBALS+2
    STA     OVERLAY_ROW
    LDA     WV_PV_GLOBALS+3
    STA     OVERLAY_ROW+1
    LDY     #$05
@new_back:
    LDA     WV_PV_GLOBALS+4,y
    STA     NEW_DATA,y
    DEY
    BPL     @new_back
    LDX     #DRAWING_VARS - 1
@drawing_back:
    LDA     WV_PV_DRAWING,x
    STA     SPRITE_ANIM,x
    DEX
    BPL     @drawing_back
    LDA     #$00
    STA     NMI_BUSY
    SEC
    RTS

; The frame's previews placed: the second page's sprites they no longer use hidden, the cache
; entries not seen forgotten.
wide_pv_end:
    LDA     WV_PV_FULL
    BNE     @full
    LDY     WV_PV_N
@hide:
    CPY     WV_PV_LAST
    BCS     @hidden
    LDA     #$F8
    STA     EXTRA_OAM,y
    INY
    INY
    INY
    INY
    BNE     @hide
@hidden:
    LDA     WV_PV_N
    STA     WV_PV_LAST
    JMP     @forgotten
@full:
    LDA     #$FF                        ; all of the page, the next frame
    STA     WV_PV_LAST
@forgotten:
    LDX     #PV_MAX-1
@forget:
    LDA     PV_SEEN,x
    BNE     @kept
    STA     PV_USED,x
@kept:
    DEX
    BPL     @forget
    RTS

; ---------------------------------------------------------------------------- ;
; The level selector (option bit 0): Select on the title screen goes through the 32 rounds, shown
; under the title as "area round"; Start begins there.

; The title screen's buttons ($AE51, in place of CMP #$10 / BNE wait; A the buttons pressed):
; Start goes on as the game does; Select chooses the next round.
wide_title_button:
    CMP     #$10
    BNE     @select
    RTS
@select:
    LDA     WIDE_OPTIONS
    AND     #OPT_LEVEL_SELECT
    BEQ     @wait
    LDA     WV_LEVEL
    CLC
    ADC     #$01
    AND     #31
    STA     WV_LEVEL
    JSR     wide_show_level
@wait:
    PLA
    PLA
    JMP     TITLE_WAIT

; The round selected, in the second sprite page: two digits.
wide_show_level:
    LDA     WV_LEVEL
    LSR
    LSR
    CLC
    ADC     #DIGIT_TILE + 1
    STA     EXTRA_OAM+1
    LDA     WV_LEVEL
    AND     #$03
    CLC
    ADC     #DIGIT_TILE + 1
    STA     EXTRA_OAM+5
    LDA     #$A8                        ; y
    STA     EXTRA_OAM
    STA     EXTRA_OAM+4
    LDA     #$00                        ; palette 0
    STA     EXTRA_OAM+2
    STA     EXTRA_OAM+6
    LDA     #$74                        ; x
    STA     EXTRA_OAM+3
    LDA     #$84
    STA     EXTRA_OAM+7
    RTS

; A new game ($8078, in place of LDA #0 / STA area / STA round): the round selected, with the
; option on (not in the demo).
wide_start_level:
    JSR     wide_hide_page
    LDA     #$00
    STA     AREA
    STA     ROUND
    LDA     DEMO
    BNE     @done
    LDA     WIDE_OPTIONS
    AND     #OPT_LEVEL_SELECT
    BEQ     @done
    LDA     WV_LEVEL
    LSR
    LSR
    STA     AREA
    LDA     WV_LEVEL
    AND     #$03
    STA     ROUND
@done:
    RTS

; ---------------------------------------------------------------------------- ;
; Invincibility (option bit 1, a testing aid): an enemy's touch ($C04D, in place of LDA $72 / ORA
; $70) is no injury, as if the player were dead already; falls, the water and hunger still kill.
wide_injury:
    LDA     HIT_KIND
    CMP     #$FF
    BEQ     @game
    LDA     WIDE_OPTIONS
    AND     #OPT_INVINCIBLE
    BNE     @done
@game:
    LDA     DEATH
    ORA     FLAG_70
@done:
    RTS

wide_end:
.assert wide_end <= $8000, error, "widescreen code past the window"
```

### B. `tools/build.py`

```python
"""Builds the Adventure Island (USA) widescreen hack (advanced mode) and its BPS patch.

Usage: python -I tools/build.py <Adventure Island (USA).nes> [out.bps]

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH) for $5800: a 16 KiB
   PRG bank added after the game's 32 KiB, which the widescreen extension's PRG window shows at
   $5800-$7FFF (Zaxnes opens it at power-on for this patch: the game has no room left for the
   code that would). CNROM maps the first 32 KiB at $8000-$FFFF, so the game runs as it was.
2. Writes the hooks into the game's PRG-ROM, each checked against the bytes it replaces.
3. Writes build/aiwide.nes (header: 3 PRG banks) and, if asked, the BPS patch (data without
   iNES header; the CHR-ROM, moved after the new bank, is copied from the player's ROM).
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCE_CRC = 0xF8A713BE
PRG_SIZE = 0x8000
CHR_SIZE = 0x8000
BANK_SIZE = 0x4000
CODE_START = 0x5800

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

BCS = 0xB0
NOP = 0xEA
RTS = 0x60


def jsr(target):
    return bytes([0x20, target & 0xFF, target >> 8])


def jmp(target):
    return bytes([0x4C, target & 0xFF, target >> 8])


def branch(opcode, at, target):
    offset = target - (at + 2)
    assert -128 <= offset <= 127, (hex(at), hex(target))
    return bytes([opcode, offset & 0xFF])


def hooks(sym):
    """The patches of the game's PRG-ROM: (address, original bytes, new bytes)."""
    out = []

    def put(address, original, new):
        original = bytes.fromhex(original)
        new = bytes(new) + bytes([NOP]) * (len(original) - len(new))
        assert len(new) == len(original), hex(address)
        out.append((address, original, new))

    # NMI: the extension's settings for the frame (LDA #<ram_oam / STA $2003 done there).
    put(0xB717, 'a9 00 8d 03 20', jsr(sym['wide_nmi']))
    # Sprites in the margins: an object past the screen, each of its sprites, a single sprite.
    put(0xC305, 'a4 59 88 10 68', jsr(sym['wide_object_shown']))
    put(0xC48B, 'a5 5b d0 39', jsr(sym['wide_sprite_at']))
    put(0xC4A9, 'a5 5b d0 1b', jsr(sym['wide_sprite_tayx']))
    put(0xC378, 'a5 59 d0 27', jsr(sym['wide_one_sprite']))
    # Ghosts: objects past the left edge kept in the left margin, without behaviour or collision.
    put(0x8EA7, 'bd 84 05 38 e5 00 bd 94 05 e5 01 30 0e',
        jsr(sym['wide_left_gone']) + branch(BCS, 0x8EAA, 0x8EC2))
    put(0x8E8E, '20 9e 92', jsr(sym['wide_handler']))
    put(0xBFF2, 'bd 14 06 18 79 14 06', jsr(sym['wide_collision']))
    put(0x8DB6, 'bd 54 05 f0 04 ca d0 f8',
        jsr(sym['wide_find_slot']) + branch(BCS, 0x8DB9, 0x8DBF) + bytes([RTS]))
    put(0x9534, '9d 54 05 60', jmp(sym['wide_object_state']))
    put(0x8C41, '8e 36 05', jsr(sym['wide_clear_ghosts']))
    # Previews: the objects of the round's lists, drawn still where the game will create them.
    put(0x80EC, '20 45 8c', jsr(sym['wide_spawn']))
    # Options: the level selector on the title screen, invincibility.
    put(0xAE51, 'c9 10 d0 3a', jsr(sym['wide_title_button']))
    put(0x8078, 'a9 00 85 37 85 38', jsr(sym['wide_start_level']))
    put(0xC04D, 'a5 72 05 70', jsr(sym['wide_injury']))
    return out


def tool(name):
    return [name] if shutil.which(name) else ['wsl', '-e', name]


def wsl_path(path):
    if shutil.which('ca65'):
        return path
    drive, rest = os.path.splitdrive(os.path.abspath(path))
    return '/mnt/' + drive[0].lower() + rest.replace('\\', '/')


def assemble():
    build = os.path.join(ROOT, 'build')
    os.makedirs(build, exist_ok=True)
    cfg = os.path.join(build, 'wide.cfg')
    open(cfg, 'w', newline='\n').write(
        f'MEMORY {{ CODE: start = ${CODE_START:04X}, size = ${BANK_SIZE:04X}, '
        'fill = yes, fillval = $FF, file = %O; }\n'
        'SEGMENTS { CODE: load = CODE, type = ro; }\n')
    obj = os.path.join(build, 'wide.o')
    binary = os.path.join(build, 'wide.bin')
    labels = os.path.join(build, 'wide.labels')
    subprocess.run(tool('ca65') + ['-g', wsl_path(os.path.join(ROOT, 'src', 'wide.s')), '-o', wsl_path(obj)],
                   check=True)
    subprocess.run(tool('ld65') + ['-C', wsl_path(cfg), '-o', wsl_path(binary), '-Ln', wsl_path(labels),
                                   wsl_path(obj)], check=True)
    sym = {}
    for line in open(labels):
        parts = line.split()
        if len(parts) >= 3 and parts[0] == 'al':
            sym[parts[2].lstrip('.')] = int(parts[1], 16)
    return open(binary, 'rb').read(), sym


def main():
    rom_path = sys.argv[1]
    image = bytearray(open(rom_path, 'rb').read())
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = image[header:header + PRG_SIZE + CHR_SIZE]
    assert zlib.crc32(data) == SOURCE_CRC, 'not Adventure Island (USA)'

    code, sym = assemble()
    assert len(code) == BANK_SIZE
    prg = bytearray(data[:PRG_SIZE])
    skip = {int(a, 16) for a in os.environ.get('WIDE_SKIP', '').split(',') if a}
    for address, original, new in hooks(sym):
        if address in skip:
            continue
        at = address - 0x8000
        assert prg[at:at + len(original)] == original, f'${address:04X}: not the expected bytes'
        prg[at:at + len(new)] = new
    target = bytes(prg) + code + bytes(data[PRG_SIZE:])

    out_header = bytearray(image[:header])
    if header:
        out_header[4] = (PRG_SIZE + BANK_SIZE) // BANK_SIZE
    out = os.path.join(ROOT, 'build', 'aiwide.nes')
    open(out, 'wb').write(bytes(out_header) + target)
    print(f'{out}: data CRC {zlib.crc32(target):08X}')
    if len(sys.argv) > 2:
        sys.path.insert(0, os.path.join(ROOT, 'tools'))
        import make_bps
        moved = [(PRG_SIZE + BANK_SIZE, PRG_SIZE + BANK_SIZE + CHR_SIZE, PRG_SIZE)]
        patch = make_bps.make(bytes(data), target, moved)
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
