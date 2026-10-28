# Mega Man 6 (USA): the widescreen hack

This document describes completely the modification of *Mega Man 6 (USA)* that Zaxnes ships as a
built-in patch, the advanced mode of its widescreen. The appendices hold the new code and the build
script, which writes every hook after checking the bytes it replaces: with them the ROM can be
rebuilt identically.

The background of the margins is not the hack's work: the emulator's Mega Man 6 margin decoder
reads the stage map in PRG-ROM through the pointers the game keeps in RAM (see
[`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md), section 4); it widens the game on
its own, the simple mode. The hack does the objects: enemies that spawn, sleep, live and show in the
margins, the way the Mega Man 3 to 5 hacks do them, in Mega Man 6's own engine.

At the screen's edges the decoder redraws the screen's first two columns and its last one: the game
draws the column coming past the right edge on a single nametable's width, over the screen's first
columns, which it hides 8 pixels of (with a fine X, part of column 1 too: a stripe of that column's
colours showed there), and the last column showed the colours of the column being drawn past the
edge. Only rows of a camera on a screen's top are decoded. With the video setting that hides the
overscan, the decoder asks for the screen's top and bottom 8 lines to stay hidden with a vertical
margin too, and the vertical margin is drawn black: the game draws the rows coming in at those edges
a piece at a time while it scrolls to another section, which a television hid. The picture then ends
where it does without a vertical margin.

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Mega Man 6 (USA)*, MMC3 (mapper 4), 512 KiB PRG (64 banks of 8 KiB, banks 62 and 63 fixed at `$C000` and `$E000`), CHR-RAM |
| Source data (iNES header excluded) | CRC-32 `988798A8` |
| Target data | CRC-32 `9E1E3AAA` (the hack's version in savestate headers; `tools/build.py` prints it) |
| Changes | PRG grown to 544 KiB (68 banks of 8 KiB): the game's banks 0-63 where they were, the hack's code as banks 64-65 (16 KiB bank 32), the fixed banks copied to banks 66 and 67 (Zaxnes' MMC3 fixes the last two banks, whatever their count); hooks in banks `$38`, `$3A`, 62 and 63 (both copies of the fixed banks) |
| Patch | `rust/crates/zax-nes/patches/megaman6-usa-widescreen.bps`, made against the data without header; the fixed banks' copies are SourceCopy, the added banks never read from the source, no game data in the patch |

The code runs from the widescreen extension's PRG window at `$5800-$7FFF` (10 KiB), which Zaxnes
opens on bank 32, page 0 at power-on for this patch (`boot_window`): the game has no free room for
code that would unlock the extension. The window covers `$6000-$7FFF` (`$5007` bit 4, set by the
hack every frame): the game reads and writes nothing there (no PRG-RAM; checked over its code and
in play). The game only ever selects banks below
64, so the added banks never show at `$8000` or `$A000`.

---

## 2. What the game does, and what changes

The game runs as tasks under a scheduler; it waits for the NMI when none is ready (`$C57D`: `$F2 =
0`, then `LDA $F2 / BEQ` at `$C581`, Zaxnes' idle loop for this patch). The NMI updates the screen
for a frame the game finished (`$C013`); otherwise it is a lag frame. Each frame `$D1D4` starts the
sprites (`$8C`, the next one's offset, at 0), then draws the energy bars (`$CF4A`) and the objects
(`$E1E1`) one after the other, in turns every other frame (the flicker), the rest of OAM hidden at
the end (`$E26D`).

Objects: 23 slots, 23 bytes apart: `$03A0` type (0 free), `$03B7` flags (bit 0 off the screen: not
drawn; bits 6-7 the flips), `$03FC` the stage list's entry an enemy came from (`$FF` none),
`$046F`/`$0486` x screen / pixel, `$04B4`/`$04CB` y. Slot 0 is Mega Man, 1-7 his weapons and the
like, 8-21 the enemies the list spawns (`$E59B`), 22 the list's controllers (type `$80`, put
straight in that slot: what lets the camera on, and the like). Each object runs as a script (`$E434` resumes
them, slot 22 down to 0); a script removes its object when the test `$E485` finds it off the
screen. The game's collisions measure 16-bit distances between objects. The camera is `$57:$56`;
it stops at its screen's edge unless `$71` lets it on (bit 0 right, bit 1 left), which objects of
the stage set as Mega Man goes. The stage list (bank `$3A`) is entries sorted by place, one list for
each way (`$74`/`$78`/`$7C`/`$7E` screen, x, y, type going right; `$76`/`$7A` going left), `$0649`
and `$064A` the next entries right and left of the spawn window.

| Game routine | Original | Hack |
|---|---|---|
| the list's spawns (bank `$3A` `$8120`, the window, then the walk from `$813A`) | entries between camera + 8 and camera + 254 spawn, the walk keeping its place in `$0649` / `$064A` | two walks: the original's, its window and its pointers, for the controllers alone (type `$80`, all in slot 22: the stage counts on one at a time in the window; with a wider one a controller took the slot of the next, which stayed marked spawned, and the camera never went on); then one between camera + 8 - M and camera + 254 + M, within the part of the stage the camera may show (`$71`), for the enemies alone, with pointers of its own (taken from the game's whenever the game sets them); first, whether the camera moved since the last frame (if so, the enemies hidden show) |
| an entry's spawn (bank `$3A` `$81A7`) | | only the kind of the walk under way |
| the part of the stage enemies live in (`wide_part`) | | the screens beside the camera's, as far as a join of two screens that are not side by side, whatever `$71` (`wide_reach`, worked out each frame before the walks, as Zaxnes' decoder does for the margins: the stage list's controllers, their actions without a screen of their own scrolling up or down from their screen, doors to a screen of their own, or doors that load the stage's other tiles (`$73` with bit 7); bank `$39` at `$8000` and the list's bank `$3A` at `$A000` through the game's `$CBC0`, its banks put back as its NMI does) |
| a list object's spawn (bank `$3A` `$81EA`) | | it sleeps when its place is off the original screen; an enemy spawned while the camera stands (the stage's start, while the game is still copying the enemies' tiles to CHR-RAM: they showed as a jumble) is hidden until the camera moves |
| object loop (`$E43A`, `LDA $03A0,X / BEQ` next) | resumes every object's script | a free slot: no sleep left in it (an object made there later is awake); an object asleep is skipped while off the original screen (past the margins it is removed), but an enemy runs its script's first frame, which sets up how it looks, then goes to the **reserve**; on the original screen it wakes for good |
| the reserve (after the list's spawns, before the objects run; drawn before the rest of OAM is hidden) | | enemies asleep in the margins, out of the game's 14 enemy slots: 8 entries of the 29 object tables (`$03A0`-`$0624`, 23 bytes apart); an entry reaching the original screen takes a free slot again and wakes (it waits at the edge while none is), one past the margins is dropped; each is drawn as the drawer draws an object, in a free slot or in slot 21 borrowed (its object set aside meanwhile), its animation stepping on |
| an entry's free slot (bank `$3A` `$81C4`, `JSR $E590`) | none while a slot holds the entry | none while the reserve holds it either |
| the game empties its slots (`$D6F1`, a scroll to another screen; hooked at the end of its loop, `$D6FD`) | | the reserve emptied too (also whenever the game sets its list pointers) |
| "on the screen" (`$E485`) | y on the screen, x - camera within `[4, 256)` | the same for slots 0-7 and the controller (22); an enemy also within `[LO, HI)` |
| an object's place (`$E24B`, in the drawer) | not drawn when flagged off the screen (`$03B7` bit 0), its y off the screen or its x - camera (low byte) below 4 | the same on the original screen; an enemy off it is drawn within `[LO, HI)`, the flag aside (the game sets it only for objects it never draws: those off the screen it removes), unless it is hidden or, asleep, on a screen other than the one whose enemies' tiles the game loaded to CHR-RAM (`$0680`, `$FF` while loading) and the next one: the original spawns no enemy elsewhere, and those of a further screen showed as a jumble |
| an object's sprites (`$E315`, the jump to one of the four loops: none, horizontal, vertical, both flips) | x = 8-bit screen x + or - the offset, the sprite hidden when it overflows; the loop stops when OAM is full | the same four loops for each page (eight), 16-bit x, shown within `[-M, 256 + M)`, its high byte to `$5100 + 4 n` (or `$5400 + 4 n`); the loops go on to the extension's second page (`$5300`) when the first is full, and stop when both are |
| the frame's sprites (`$D1D4`, `$E1DD`: `$8C = 0`) | | also the first page in use and its high bytes back to 0 (the energy bars' writer writes none); X kept, as by the code replaced (the vertical scroll's loop, `$D7CA`, counts with it through `$D701`: changed, it ran 256 frames instead of 120 and the scroll down went on and on) |
| bars and objects in turns (`$D1D8`) | the energy bars before the objects one frame, after them the next | the bars always first (nothing is left over to flicker) |
| the rest of OAM hidden (`$E26D`) | from `$8C` to the end | on the page in use, and the second page whole while the first is |
| Mega Man's damage (bank `$38` `$8F72`) | none while `$A2` or `$96` is set | nor with the option "Invincible" |

`LO = max(-M, part start - camera)`, `HI = min(256 + M, part end - camera)`; M is the margin; the
part is the camera's screen, extended to the stage's start or end where `$71` lets the camera go on
(the screen it reaches and the next one on the right). Mega Man's weapons (slots 0-7) keep the
original limits. With M = 0 every routine does what the code it replaces does (the sprite loops
draw an h-flipped object near the left edge partly instead of not at all only off the original
screen).

Margin: `M = min($5002, 248)` (248: the widest the extension's 8-bit registers carry; sprites show
that far with `$5007` bit 1).

CPU time: 64 lines of overclock (`$5006`) pay for the hack's work and the enemies of the margins;
255 with the option "No slowdown" (on by default). Measured on Plant Man's stage (500 frames,
holding right and jumping, counted with a temporary trace of the NMI's updates): the game with the
hooks' sprite routines left out runs late 19 frames; the hack at a margin of 0 without overclock
75; at a 128-pixel margin without overclock 148, with 64 lines 62, with 255 none. The reserve,
measured on the first stage of a movie from its start (600 frames at a 128-pixel
margin, the game's own count of the frames it finished, `$F3`): with 64 lines 64 frames late
against 54 without it, with 255 none.

## 3. The extension

Set every frame the NMI updates the screen (`$C01F`, `LDA $FB / STA $A000` becomes `JSR wide_nmi /
NOP` x 2, which writes `$5003 = M`, `$5004 = $5005 = 0` (the nametable columns are hidden: the
decoder draws the background), `$5007 = $1E` (sprites free of those extents, second sprite page,
high bytes kept from one page to the next: every sprite the hack places gets one, those of the
energy bars keep the 0 of the frame's start; the window on to `$7FFF`), `$5006 = 64` (255 with "No slowdown"), then does the
replaced instructions).

Variables (extension RAM): `WV_M $5200`, the live range `WV_LO $5201`/`$5202` and `WV_HI
$5203`/`$5204` (signed 16-bit), the object's screen x `WV_S $5205`/`$5206`, scratch `WV_T0 $5207`,
`WV_T1 $5208`, `WV_PAGE $5209` (bit 7: the second page is in use), `WV_FULL $520A` (both pages
full), the part of the stage `WV_FIRST $520B` and `WV_END $520C` (screens), the sprites' x base
`WV_X $520D`/`$520E`, `WV_YH $520F`, `WV_DORMANT $5220 + slot` (the list entry of an enemy asleep,
`$FF` awake: a slot another object took since is awake), `WV_STARTED $5240 + slot`, `WV_HIDDEN
$5260 + slot`, the camera of the last spawns `WV_CAM $5210`/`$5211`, `WV_MOVED $5212`, the walk
under way `WV_PASS $5213`, the game's list pointers as its walk left them `WV_O $5214`/`$5215`, the
enemies' walk's own `WV_E $5216`/`$5217`, `WV_RES $5218`, `WV_BORROW $5219`, where the camera stops past `$71` the first and last screens of that part `WV_LSTOP $521A` and `WV_RSTOP $521B`, the two joins `wide_reach` tests and whether they are apart `WV_JL $5238`, `WV_JR $5239`, `WV_AL $523A`, `WV_AR $523B`, the controller at hand `WV_OWN $523C`, `WV_FWD $523D`, `WV_AT $523E`, `WV_ENTRY $523F`, `WV_TILES $5257`, the game's banks while they are worked out `WV_BANKS $521C`/`$521D`; the reserve:
`RES_USED $5280 + e`, `RES_ENTRY $5288 + e`, `RES_HIDDEN $5290 + e`, its object tables at `$5500 +
8 f + e`, the borrowed slot's at `$5600 + f`.

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length. Banks 62 and
63 are the fixed ones (`$C000`, `$E000`), written into both their copies (62 and 66, 63 and 67);
banks `$38` and `$3A` run at `$8000`.

| Address | Replaces | With |
|---|---|---|
| `$C01F` | `LDA $FB / STA $A000` | `JSR wide_nmi` |
| `$D1D4`, `$E1DD` | `LDA #$00 / STA $8C` | `JSR wide_frame_start` |
| `$D1D8` | `LDA $F3 / AND #$01 / BEQ $D1EC` | `JMP $D1EC` |
| `$E24B` | `LDA $03B7,X / AND #$01 / BNE / LDA $04B4,X / BNE / LDA $0486,X / SEC / SBC $F7 / CMP #$04 / BCC / STA $00` | `JSR wide_place / BCS $E269` |
| `$E315` | `LDX $8C / LDY #0 / JMP ($0008)` | `JMP wide_sprites` |
| `$E26D` | `LDX $8C / LDA #$F0` (head of the loop hiding the rest of OAM) | `JMP wide_hide_rest` |
| `$E485` | `LDA $04CB,X` (head of the test "on the screen") | `JMP wide_on_screen` |
| `$E43A` | `LDA $03A0,X` | `JSR wide_think` |
| bank `$3A` `$8120` | `$03:$02 = camera + 254`, `$01:$00 = camera + 8` (26 bytes) | `JMP wide_spawns` |
| bank `$3A` `$81A7` | `TYA / AND #$07 / TAX` | `JSR wide_spawn_filter` |
| bank `$3A` `$81C4` | `JSR $E590` | `JSR wide_free_slot` |
| `$D6FD` | `DEX / BNE $D6F3 / RTS` (the end of the loop emptying the slots; its branch comes back to `$D6F3`, so not its head) | `JMP wide_slots_cleared` |
| bank `$3A` `$81EA` | `LDA #$00 / STA $04B4,X` | `JSR wide_spawned` |
| bank `$38` `$8F72` | `LDA $A2 / BNE $8F95 / LDA $96 / BNE $8F95` | `JSR wide_hurt / BNE $8F95` |

## 5. Options

The options the player can turn on or off (the game's hacks screen, with the advanced mode on);
Zaxnes writes their bits at `$500A` every frame, the hack reads them there (`WIDE_OPTIONS`).

| Bit | Key | Label | Default | Effect |
|---|---|---|---|---|
| 1 | `invincible` | Invincible | off | a testing aid: Mega Man loses no health (the knockback stays; pits and spikes still kill) |
| 2 | `no_slowdown` | No slowdown / Sans ralentissements | on | 255 lines of overclock instead of 64 |

The options live in Zaxnes (`rust/crates/zax-nes/src/builtin_patches.rs`, `MEGAMAN4_OPTIONS`, shared
with Mega Man 4 and 5); the ROM does not depend on them being set.

## 6. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Mega Man 6 (USA).nes" build/mm6wide.bps
```

assembles `src/wide.s` for `$5800` (one 16 KiB bank, the code within its first 10 KiB, `-g`, labels
from `ld65 -Ln`), lays the PRG out in 68 banks of 8 KiB, writes the hooks, `build/mm6wide.nes` and
the BPS patch (`tools/make_bps.py`: the fixed banks' copies are SourceCopy from the player's ROM,
the added banks and the hooks TargetRead). `WIDE_SKIP=<hex addresses>` leaves hooks out, for
testing.

## 7. Limits

* No sprite in the vertical margins: the extension carries a high byte for x only.
* Only the enemies (slots 8 up) live in the margins; Mega Man's weapons keep the original screen.
* An enemy asleep in a margin hangs where its entry puts it, in the pose of its first frame, until
  the original screen reaches it.
* The reserve holds 8 enemies; past that, an enemy asleep stays in a game slot, as before it.
* An enemy spawned in a margin at the stage's start shows only once the camera moves.
* An enemy asleep two screens or more ahead of the screen whose enemies' tiles are loaded (`$0680`)
  is not drawn until the camera reaches the screen before it.
* The part of the stage enemies live in follows the joins the stage's controllers give (as
  Zaxnes' decoder's margins): a controller of another kind of settings (a boss's) says nothing.
* The hack's sprite loops cost time: without overclock the game runs late more often than the
  original; "No slowdown" (on by default) leaves no lag frame.

---

## Appendices

### A. `src/wide.s`

```asm
; Mega Man 6 (USA): the widescreen hack (Zaxnes, advanced mode).
;
; This code is a 16 KiB PRG bank added to the game (PRG bank 32 of 34 in 16 KiB units: the game's
; 8 KiB banks 0-63 stay where they were and its two fixed banks are copied to banks 66 and 67, the
; ones MMC3 fixes at $C000 and $E000); the widescreen extension's PRG window shows its first 2 KiB
; at $5800-$5FFF, Zaxnes opening it at power-on (the patch's boot window: bank 32, page 0). The
; window goes on over $6000-$7FFF ($5007 bit 4): the game reads and writes nothing there. The game
; reaches the code through hooks in its own (tools/build.py). The background of the margins is
; Zaxnes' margin decoder's work; the hack does the objects, the way the Mega Man 3 to 5 hacks do
; them, in Mega Man 6's own engine (objects run as scripts, each removing itself through the test
; "on the screen" at $E485):
;
;  * an enemy of the stage's list (object slots 8-21) spawns when it comes within the margin of
;    the screen, and lives and shows until it leaves it, all within the part of the stage the
;    camera may scroll over ($71);
;  * the list's controllers (type $80, always slot 22: what lets the camera on, down a shaft...)
;    spawn there too and sleep until the original screen reaches them, then live as the original
;    does: off the original screen, they remove themselves;
;  * one that spawns in a margin sleeps there: its script runs its first frame (which sets up how
;    it looks), then not again until it comes onto the original screen; awake, it stays awake; one
;    that spawns while the camera stands (the stage's start: its tiles are still on their way to
;    CHR-RAM) is not drawn until the camera moves, nor one on a screen whose enemies' tiles the
;    game has not loaded yet (it loads those of the camera's screen and the next one);
;  * an enemy asleep in a margin, once set up, waits in the reserve, out of the game's slots (the
;    hack draws it), and takes a free slot again when it reaches the original screen;
;  * every object's sprites get the high byte of their 16-bit x, and the sprites that no longer
;    fit in the console's 64 go on to the extension's second page instead of being dropped;
;  * the game's collisions measure 16-bit distances: an enemy in a margin only touches what is
;    near it;
;  * Mega Man and his weapons (slots 0-7) keep the original limits.
;
; With a margin of 0 every routine does exactly what the code it replaces does.

; ---------------------------------------------------------------------------- ;
; The widescreen extension (see Zaxnes, docs/WIDESCREEN-EXTENSION.md).

WIDE_HOST       = $5002         ; R: the margin the host offers
WIDE_REQ        = $5003         ; W: the margin asked for
WIDE_LEFT       = $5004         ; nametable columns shown left of the screen
WIDE_RIGHT      = $5005         ; and right of it
WIDE_OVERCLOCK  = $5006         ; extra scanlines of CPU time per frame
WIDE_LAYOUT     = $5007         ; bit 1 sprites free of the extents, bit 2 second sprite page,
                                ; bit 3 high bytes kept
WIDE_OPTIONS    = $500A         ; R: the options the player turned on
WIDE_SPRHI      = $5100         ; + 4 n: high byte of the x of sprite n, for the next OAM DMA
EXTRA_OAM       = $5300         ; the second sprite page, OAM format
EXTRA_SPRHI     = $5400         ; + 4 n: high byte of the x of its sprite n

OPT_INVINCIBLE  = $02           ; a testing aid: Mega Man takes no damage
OPT_NO_SLOWDOWN = $04           ; nearly a frame more of CPU time: the game never runs late

MAX_MARGIN      = 248           ; the most the extension shows sprites in
OVERCLOCK_LINES = 64            ; pays for the hack's work and the enemies of the margins
NO_SLOWDOWN_LINES = 255
FIRST_ENEMY     = $08           ; the slots the stage's list spawns into: 8-21
CONTROLLER      = $16           ; the slot of the list's type $80 entries (the stage's controllers)

; Variables in the extension's RAM ($5200-$57FF).
WV_M            = $5200         ; margin of this frame
WV_LOL          = $5201         ; screen x range objects may live in: [LO, HI), signed 16-bit
WV_LOH          = $5202
WV_HIL          = $5203
WV_HIH          = $5204
WV_SL           = $5205         ; screen x of the object at hand, signed 16-bit
WV_SH           = $5206
WV_T0           = $5207         ; scratch
WV_T1           = $5208
WV_PAGE         = $5209         ; bit 7: the sprites being placed go to the second page
WV_FULL         = $520A         ; non-zero once both pages are full
WV_FIRST        = $520B         ; the first screen of the part of the stage the camera may show
WV_END          = $520C         ; the screen past its last
WV_XL           = $520D         ; the sprites' x base, signed 16-bit
WV_XH           = $520E
WV_YH           = $520F         ; the high byte of the sprite's y
WV_DORMANT      = $5220         ; + slot: the list entry ($03FC + slot) of an enemy asleep, $FF awake
WV_STARTED      = $5240         ; + slot: non-zero once an enemy asleep has run its script's first frame
WV_HIDDEN       = $5260         ; + slot: non-zero while an enemy asleep is not drawn (see wide_spawned)
WV_CAML         = $5210         ; the camera of the last spawn window, to tell whether it moved
WV_CAMH         = $5211
WV_MOVED        = $5212         ; non-zero when the camera moved since the last spawn window
WV_PASS         = $5213         ; the walk of the stage list under way: $80 controllers, 0 enemies
WV_O49          = $5214         ; the game's list pointers as its walk left them
WV_O4A          = $5215
WV_E49          = $5216         ; the enemies' walk's list pointers
WV_E4A          = $5217
WV_RES          = $5218         ; the reserve entry at hand
WV_BORROW       = $5219         ; non-zero while slot BORROWED holds a reserve entry being drawn
WV_LSTOP        = $521A         ; the first screen beside the camera's that enemies live in
WV_RSTOP        = $521B         ; and the last (see wide_reach)
WV_JL           = $5238         ; the joins wide_reach tests: before the camera's screen ($FF: none)
WV_JR           = $5239         ; and after the last it shows
WV_AL           = $523A         ; non-zero when that join's screens are not side by side
WV_AR           = $523B
WV_OWN          = $523C         ; the controller at hand: its screen, whether it scrolls to the
WV_FWD          = $523D         ; next one, where its settings are read, its entry in the list
WV_AT           = $523E
WV_ENTRY        = $523F
WV_TILES        = $5257         ; non-zero when the action at hand loads the stage's other tiles
WV_BANKS        = $521C         ; the game's banks at $8000 and $A000 ($4A, $4B) while wide_reach reads
RES_COUNT       = 8             ; the reserve: enemies asleep in the margins, out of the game's slots
RES_USED        = $5280         ; + entry: non-zero when in use
RES_ENTRY       = $5288         ; + entry: its stage list entry
RES_HIDDEN      = $5290         ; + entry: its WV_HIDDEN
RES_DATA        = $5500         ; object table f of entry e at RES_DATA + RES_COUNT f + e
SAVE_DATA       = $5600         ; + f: the borrowed slot's object tables while it draws an entry
OBJ_TABLES      = 29            ; the object tables, 23 bytes apart from $03A0 (slots 0-22)
BORROWED        = $15           ; the slot borrowed to draw the reserve when none is free

; The game.
OAM             = $0200
CAM_LO          = $56           ; the camera's x: pixel and screen
LIST_RIGHT      = $0649         ; the stage list's next entries right and left of the spawn window
LIST_LEFT       = $064A
LIST_TYPE       = $7E           ; pointer to the stage list's entry types
LIST_SCREENS    = $74           ; pointers to the stage list's screens, x and y (the controllers'
LIST_XS         = $78           ; settings), in bank $3A at $8000
LIST_YS         = $7C
CONTROLLER_BANK = $39           ; the controllers' script and settings ($824D, $82EB / $836B)
SETTINGS_LO     = $82EB         ; + setting (the entry's y): the controller's settings, low and high
SETTINGS_HI     = $836B
BANK_8000       = $4A           ; the game's banks at $8000 and $A000 and its bank register, as it
BANK_A000       = $4B           ; keeps them (its NMI puts them back)
BANK_SELECT     = $4C
MMC3_SELECT     = $8000
MMC3_BANK       = $8001
LIST_WALK       = $813A         ; bank $3A: the walk of the list, through the window $00-$03, to an RTS
SLOTS_CLEAR_LOOP = $D6F3        ; the game's loop emptying slots 21 to 1
FREE_SLOT       = $E590         ; a free enemy slot for list entry Y in X, carry clear; set: none
SET_BANK        = $CBC0         ; the 8 KiB banks Y at $8000 and Y + 1 at $A000
DRAW_SPRITES    = $E2A9         ; an object's sprites, X the slot, $00 its screen x
OBJ_ANIM_BANK   = $0458         ; + slot: its animation's bank
OBJ_ANIM_END    = $04F9         ; + slot: its animation's length (steps of 2)
OBJ_ANIM_STEP   = $0510         ; + slot: its animation's step
OBJ_ANIM_LO     = $053E         ; + slot: its animation's data, low and high
OBJ_ANIM_HI     = $0527
OBJ_ANIM_TIME   = $0555         ; + slot: frames left of its animation's step
CHR_SCREEN      = $0680         ; the screen whose enemies' tiles the game loaded to CHR-RAM ($FF: loading)
CAM_HI          = $57
SCROLL          = $71           ; bit 0: the camera may go on right of its screen, bit 1 left
OBJ_TYPE        = $03A0         ; + slot: 0 free
OBJ_FLAGS       = $03B7         ; + slot: bit 0 off the screen (not drawn), bits 6-7 flips
OBJ_ENTRY       = $03FC         ; + slot: the stage list's entry an enemy came from ($FF none)
OBJ_HEALTH      = $03E5
OBJ_XH          = $046F         ; + slot: x, screen and pixel
OBJ_X           = $0486
OBJ_YH          = $04B4         ; + slot: y, screen and pixel
OBJ_Y           = $04CB
OAM_NEXT        = $8C           ; where the next sprite goes in OAM
FLIPS           = $8D           ; the object's attributes: bit 6 horizontal flip, bit 7 vertical

; The game's routines.
HIDE_REST_DONE  = $E27A         ; after the loop hiding the rest of OAM
HURT_SKIP       = $8F95         ; bank $38: the damage routine's RTS

.segment "CODE"
.org $5800

; ---------------------------------------------------------------------------- ;
; NMI ($C01F, in place of LDA $FB / STA $A000, the update of a frame the game finished): the
; extension's settings for the next frame.
wide_nmi:
    LDA     WIDE_HOST
    CMP     #MAX_MARGIN
    BCC     @margin
    LDA     #MAX_MARGIN
@margin:
    STA     WV_M
    STA     WIDE_REQ
    LDA     #$00
    STA     WIDE_LEFT
    STA     WIDE_RIGHT
    LDA     #$1E                        ; sprites free of the extents, second sprite page, high
                                        ; bytes kept (every sprite placed gets one), the window
                                        ; on to $7FFF (the hack's code; the game has no PRG-RAM)
    STA     WIDE_LAYOUT
    LDA     WIDE_OPTIONS
    AND     #OPT_NO_SLOWDOWN
    BEQ     @normal
    LDA     #NO_SLOWDOWN_LINES
    BNE     @overclock
@normal:
    LDA     #OVERCLOCK_LINES
@overclock:
    STA     WIDE_OVERCLOCK
    LDA     $FB
    STA     $A000
    RTS

; ---------------------------------------------------------------------------- ;
; The screen x of object X (its x minus the camera's), into WV_SL/WV_SH.
wide_screen_x:
    SEC
    LDA     OBJ_X,x
    SBC     CAM_LO
    STA     WV_SL
    LDA     OBJ_XH,x
    SBC     CAM_HI
    STA     WV_SH
    RTS

; The part of the stage enemies live in, in screens: [WV_FIRST, WV_END), the screens side by side
; with the camera's that wide_reach found (whatever $71, which says where the camera may go now).
wide_part:
    LDA     WV_LSTOP
    STA     WV_FIRST
    LDA     WV_RSTOP
    CLC
    ADC     #$01
    STA     WV_END
    RTS

; Whether enemy X lives: carry clear when its screen x (WV_S, set) is within
;   [max(-M, part start - camera), min(256 + M, part end - camera)).
wide_inside:
    JSR     wide_part
    SEC                                 ; LO = part start - camera
    LDA     #$00
    SBC     CAM_LO
    STA     WV_LOL
    LDA     WV_FIRST
    SBC     CAM_HI
    STA     WV_LOH
    CLC                                 ; LO + M < 0: LO = -M
    LDA     WV_LOL
    ADC     WV_M
    LDA     WV_LOH
    ADC     #$00
    BPL     @hi
    SEC
    LDA     #$00
    SBC     WV_M
    STA     WV_LOL
    LDA     #$00
    SBC     #$00
    STA     WV_LOH
@hi:
    SEC                                 ; HI = part end - camera
    LDA     #$00
    SBC     CAM_LO
    STA     WV_HIL
    LDA     WV_END
    SBC     CAM_HI
    STA     WV_HIH
    SEC                                 ; HI - 256 > M: HI = 256 + M
    LDA     WV_HIL
    SBC     WV_M
    LDA     WV_HIH
    SBC     #$01
    BMI     @test
    BNE     @cap
    LDA     WV_HIL
    CMP     WV_M
    BCC     @test
    BEQ     @test
@cap:
    LDA     WV_M
    STA     WV_HIL
    LDA     #$01
    STA     WV_HIH
@test:
    SEC                                 ; S - LO >= 0
    LDA     WV_SL
    SBC     WV_LOL
    LDA     WV_SH
    SBC     WV_LOH
    BVC     :+
    EOR     #$80
:   BMI     @outside
    SEC                                 ; S - HI < 0
    LDA     WV_SL
    SBC     WV_HIL
    LDA     WV_SH
    SBC     WV_HIH
    BVC     :+
    EOR     #$80
:   BPL     @outside
    CLC
    RTS
@outside:
    SEC
    RTS

; ---------------------------------------------------------------------------- ;
; Whether object X is on the screen ($E485, the whole routine; the scripts remove an object it
; finds off): carry clear when it is. The original: its y within the screen, its x - camera within
; [4, 256). An enemy: also its x within the margins (not a controller: slot 22).
wide_on_screen:
    LDA     OBJ_Y,x
    CMP     #$F8
    BCS     @off
    LDA     OBJ_YH,x
    BNE     @off
    JSR     wide_screen_x
    LDA     WV_SH
    BNE     @margin
    LDA     WV_SL
    CMP     #$04
    BCS     @on
@margin:
    CPX     #FIRST_ENEMY
    BCC     @off
    CPX     #CONTROLLER
    BCS     @off
    JMP     wide_inside
@on:
    CLC
    RTS
@off:
    SEC
    RTS

; An object's place on the screen ($E24B, in place of the tests of the drawer and $00 = x -
; camera; then BCS: not drawn): carry clear when it is drawn, $00 the low byte of its screen x.
; The original: not flagged off the screen ($03B7 bit 0), y on the screen, x - camera (low byte)
; at least 4. An enemy whose x is off the original screen: within the margins, unless flagged
; (an object the game never draws) or, asleep, hidden or on a screen whose tiles are not loaded.
wide_place:
    JSR     wide_screen_x
    LDA     WV_SL
    STA     $00
    LDA     WV_SH
    BEQ     @original
    CPX     #FIRST_ENEMY
    BCC     @original
    CPX     #CONTROLLER
    BCS     @original
    LDA     OBJ_YH,x
    BNE     @hidden
    LDA     OBJ_FLAGS,x                 ; an object the game does not draw (the game never sets
    AND     #$01                        ; it for one off the screen: those it removes)
    BNE     @hidden
    LDA     WV_DORMANT,x
    CMP     #$FF
    BEQ     @inside
    LDA     WV_HIDDEN,x                 ; asleep, spawned while the camera stood: not yet
    BNE     @hidden
    LDA     CHR_SCREEN                  ; asleep: its tiles are in CHR-RAM only on the screen
    CMP     #$FF                        ; whose graphics the game loaded or the next one (the
    BEQ     @hidden                     ; original spawns no enemy elsewhere); $FF: loading
    CMP     OBJ_XH,x
    BEQ     @inside
    CLC
    ADC     #$01
    CMP     OBJ_XH,x
    BNE     @hidden
@inside:
    JMP     wide_inside
@original:
    LDA     #$00                        ; the low byte alone, as the original
    STA     WV_SH
    LDA     OBJ_FLAGS,x
    AND     #$01
    BNE     @hidden
    LDA     OBJ_YH,x
    BNE     @hidden
    LDA     $00
    CMP     #$04
    BCC     @hidden
    CLC
    RTS
@hidden:
    SEC
    RTS

; ---------------------------------------------------------------------------- ;
; Sleeping enemies. An enemy of the stage list that spawns in a margin shows but its script does
; not run until it comes onto the original screen, but for its first frame: the script sets up the
; object's look there (shown before, it was a jumble of the tiles in place); from then on it lives
; as the original does, margins included.

; The end of a list enemy's spawn (bank $3A $81EA, in place of LDA #0 / STA $04B4,X; X the slot):
; it sleeps when it spawned off the original screen.
wide_spawned:
    LDA     #$00
    STA     OBJ_YH,x
    JSR     wide_screen_x
    LDA     WV_SH
    BEQ     @awake
    LDA     OBJ_ENTRY,x
    JMP     @set
@awake:
    LDA     #$FF
@set:
    STA     WV_DORMANT,x
    LDA     #$00
    STA     WV_STARTED,x
    LDA     WV_MOVED                    ; spawned while the camera stands: hidden
    EOR     #$01
    STA     WV_HIDDEN,x
    RTS

; The object loop ($E43A, in place of LDA $03A0,X; then BEQ: next slot): an enemy asleep skips
; its script while off the original screen (past the margins it is removed, as its script would),
; but for its first frame; on it, it wakes.
wide_think:
    CPX     #FIRST_ENEMY
    BCC     @run
    LDA     OBJ_TYPE,x                  ; a free slot: no sleep of an object gone left in it (an
    BNE     @object                     ; object made there later is not asleep)
    LDA     #$FF
    STA     WV_DORMANT,x
    LDA     #$00
    RTS
@object:
    LDA     WV_DORMANT,x
    CMP     #$FF
    BEQ     @run
    CMP     OBJ_ENTRY,x                 ; another object took the slot since: awake
    BNE     @wake
    JSR     wide_screen_x
    LDA     WV_SH
    BEQ     @wake
    JSR     wide_inside
    BCC     @first
    LDA     #$00                        ; past the margins: removed ($E456)
    STA     OBJ_TYPE,x
    LDA     #$FF
    STA     WV_DORMANT,x
    STA     OBJ_ENTRY,x
    STA     OBJ_HEALTH,x
@asleep:
    LDA     #$00                        ; as if the slot were free
    RTS
@first:
    CPX     #CONTROLLER                 ; a controller does nothing before its time
    BCS     @asleep
    LDA     WV_STARTED,x
    BNE     @park
    INC     WV_STARTED,x                ; its first frame: the script runs, the enemy still asleep
    BNE     @run
@park:
    JSR     wide_park                   ; then into the reserve, its slot freed, if there is room
    LDA     #$00
    RTS
@wake:
    LDA     #$FF
    STA     WV_DORMANT,x
@run:
    LDA     OBJ_TYPE,x
    RTS

; ---------------------------------------------------------------------------- ;
; The stage list's spawns (bank $3A $8120, the routine's start, jumped to: it sets $03:$02 =
; camera + 254, $01:$00 = camera + 8, then walks the list from $813A to an RTS). The list (entries
; sorted by place, one list for each way, $0649 / $064A the next entries right and left of the
; window) is walked twice, each walk with its own pointers:
;  * the original's, with its window, for the controllers (type $80) alone: they all share slot 22
;    and the stage counts on one at a time in the window (a wider one let one take the slot of the
;    next, which stayed marked spawned: the camera never went on, $71);
;  * the hack's, between camera + 8 - M and camera + 254 + M within the part of the stage the
;    camera may show, for the enemies alone, its pointers kept in WV_E49 / WV_E4A (taken from the
;    game's whenever the game set its own: a new stage, a restart, a scroll to another screen).
; wide_spawn_filter, at the spawn of each entry, keeps the walk's own kind. Whether the camera
; moved since the last call is kept first: if it did, the enemies hidden show.
wide_spawns:
    LDA     #$00                        ; whether the camera moved: the hidden enemies show
    STA     WV_MOVED
    LDA     CAM_LO
    CMP     WV_CAML
    BNE     @moved
    LDA     CAM_HI
    CMP     WV_CAMH
    BEQ     @window
@moved:
    INC     WV_MOVED
    LDA     CAM_LO
    STA     WV_CAML
    LDA     CAM_HI
    STA     WV_CAMH
    TXA                                 ; X kept, as by the code replaced
    PHA
    LDX     #CONTROLLER - 1
    LDA     #$00
@show:
    STA     WV_HIDDEN,x
    DEX
    CPX     #FIRST_ENEMY
    BCS     @show
    PLA
    TAX
@window:
    JSR     wide_reach
    LDA     LIST_RIGHT                  ; the game set its pointers: the enemies' walk starts there
    CMP     WV_O49
    BNE     @reset
    LDA     LIST_LEFT
    CMP     WV_O4A
    BEQ     @controllers
@reset:
    JSR     wide_reserve_clear          ; a new start: the reserve's enemies are gone
    LDA     LIST_RIGHT
    STA     WV_E49
    LDA     LIST_LEFT
    STA     WV_E4A
@controllers:
    CLC                                 ; the original window: camera + 8, camera + 254
    LDA     CAM_LO
    ADC     #$FE
    STA     $02
    LDA     CAM_HI
    ADC     #$00
    STA     $03
    CLC
    LDA     CAM_LO
    ADC     #$08
    STA     $00
    LDA     CAM_HI
    ADC     #$00
    STA     $01
    LDA     #$80
    STA     WV_PASS
    JSR     LIST_WALK
    LDA     LIST_RIGHT
    STA     WV_O49
    LDA     LIST_LEFT
    STA     WV_O4A
    LDA     WV_E49                      ; the enemies' walk, with its own pointers
    STA     LIST_RIGHT
    LDA     WV_E4A
    STA     LIST_LEFT
    JSR     wide_enemy_window
    LDA     #$00
    STA     WV_PASS
    JSR     LIST_WALK
    LDA     LIST_RIGHT
    STA     WV_E49
    LDA     LIST_LEFT
    STA     WV_E4A
    LDA     WV_O49                      ; the game's pointers back
    STA     LIST_RIGHT
    LDA     WV_O4A
    STA     LIST_LEFT
    JMP     wide_reserve_frame

; The enemies' window: $01:$00 = camera + 8 - M, $03:$02 = camera + 254 + M, within the part.
wide_enemy_window:
    JSR     wide_part
    CLC                                 ; left = camera + 8 - M
    LDA     CAM_LO
    ADC     #$08
    STA     $00
    LDA     CAM_HI
    ADC     #$00
    STA     $01
    SEC
    LDA     $00
    SBC     WV_M
    STA     $00
    LDA     $01
    SBC     #$00
    STA     $01
    BCC     @first                      ; before the stage
    CMP     WV_FIRST
    BCS     @right
@first:
    LDA     #$00                        ; before the part: its start
    STA     $00
    LDA     WV_FIRST
    STA     $01
@right:
    CLC                                 ; right = camera + 254 + M
    LDA     CAM_LO
    ADC     #$FE
    STA     $02
    LDA     CAM_HI
    ADC     #$00
    STA     $03
    CLC
    LDA     $02
    ADC     WV_M
    STA     $02
    LDA     $03
    ADC     #$00
    STA     $03
    CMP     WV_END
    BCC     @rts
    LDA     #$FF                        ; past the part: its last pixel
    STA     $02
    LDA     WV_END
    SEC
    SBC     #$01
    STA     $03
@rts:
    RTS

; An entry's spawn (bank $3A $81A7, in place of TYA / AND #$07 / TAX; Y the entry): only the walk's
; own kind, controllers (type $80) or enemies; another returns from $81A7 at once, unspawned.
wide_spawn_filter:
    LDA     (LIST_TYPE),y
    AND     #$80
    CMP     WV_PASS
    BNE     @other
    TYA                                 ; the code replaced
    AND     #$07
    TAX
    RTS
@other:
    PLA                                 ; out of $81A7 too
    PLA
    RTS

; ---------------------------------------------------------------------------- ;
; Sprites. The console has 64; the extension a second page of 64 at $5300, which the game's
; sprite loop moves on to when the first is full, instead of stopping.

; The frame's sprites begin ($D1D4 and $E1DD, in place of LDA #0 / STA $8C): on the first page,
; its high bytes back to 0 (the energy bars' writer, $CF4A, writes none). X is kept, as by the
; code replaced.
wide_frame_start:
    TXA                                 ; X kept: $E1DD's callers count with it (the
    PHA                                 ; vertical scroll's loop, $D7CA, through $D701)
    LDA     #$00
    STA     OAM_NEXT
    STA     WV_PAGE
    STA     WV_FULL
    TAX
@clear:
    STA     WIDE_SPRHI,x
    INX
    INX
    INX
    INX
    BNE     @clear
    PLA
    TAX
    LDA     #$00                        ; A = 0, as the code replaced leaves it
    RTS

; The rest of OAM hidden ($E26D, in place of LDX $8C / LDA #$F0 and the loop): on the page in
; use, and the second page whole while the first is.
wide_hide_rest:
    JSR     wide_reserve_draw
    LDX     OAM_NEXT
    LDA     #$F0
    BIT     WV_PAGE
    BMI     @second
@first:
    STA     OAM,x
    INX
    INX
    INX
    INX
    BNE     @first
@second:
    LDY     WV_FULL
    BNE     @done
@hide:
    STA     EXTRA_OAM,x
    INX
    INX
    INX
    INX
    BNE     @hide
@done:
    JMP     HIDE_REST_DONE

; The sprite loop of one object ($E315, in place of LDX $8C / LDY #0 / JMP ($0008), the jump to
; one of the four loops: none, horizontal, vertical, both flips): the original's, on the page in
; use, the x in 16 bits: a sprite whose x is past the margins is hidden; every sprite placed gets
; the high byte of its x. $00 and WV_SH the object's screen x, $01 its y, $02 sprites less one,
; ($0C) the tiles and attributes, ($0E) the offsets, $8D its attributes and flips. Back to the
; drawer with X 0 once both pages are full (non-zero otherwise).
wide_sprites:
    LDA     $00
    STA     WV_XL
    LDA     WV_SH
    STA     WV_XH
    BIT     FLIPS
    BVC     @vertical
    LDA     WV_XH                       ; flipped: the base 8 pixels left
    BNE     @shift
    LDA     $00                         ; on the screen and within 8 pixels of its left edge:
    CMP     #$08                        ; nothing, as the original
    BCC     @nothing
@shift:
    SEC
    LDA     WV_XL
    SBC     #$08
    STA     WV_XL
    LDA     WV_XH
    SBC     #$00
    STA     WV_XH
@vertical:
    BIT     FLIPS
    BPL     @start
    LDA     $01                         ; flipped: the base 8 pixels up, nothing above the screen
    SEC
    SBC     #$08
    STA     $01
    BCS     @start
@nothing:
    LDX     #$01
    RTS
@start:
    LDX     #$00
    LDA     WV_FULL
    BEQ     :+
    RTS
:   LDX     OAM_NEXT
    LDY     #$00
    BIT     WV_PAGE
    BMI     @second
    BIT     FLIPS
    BMI     @first_v
    BVS     :+
    JMP     wide_s1n
:   JMP     wide_s1h
@first_v:
    BVS     :+
    JMP     wide_s1v
:   JMP     wide_s1b
@second:
    BIT     FLIPS
    BMI     @second_v
    BVS     :+
    JMP     wide_s2n
:   JMP     wide_s2h
@second_v:
    BVS     :+
    JMP     wide_s2v
:   JMP     wide_s2b

; One of the loops: base and high the page's OAM and high bytes, vflip and hflip 0 or 1, sprite
; and skip its labels, wrap where to go once the page is full (the second page's same loop, at its
; skip; or wide_sprites_full).
.macro SPRITE_LOOP base, high, vflip, hflip, sprite, skip, wrap
.local nx, hx, far, right, show, next
sprite:
    LDA     ($0C),y                     ; tile
    STA     base+1,x
.if vflip
    LDA     $01                         ; y = $01 - offset, on the screen only
    SEC
    SBC     ($0E),y
    STA     base,x
    ROR
    EOR     ($0E),y
    ASL
    INY
    BCC     skip
.else
    LDA     ($0E),y                     ; y = $01 + offset, on the screen only
    CLC
    ADC     $01
    STA     base,x
    ROR
    EOR     ($0E),y
    INY
    ASL
    BCS     skip
.endif
.if hflip
    LDA     WV_XL                       ; x = base - offset
    SEC
    SBC     ($0E),y
    STA     base+3,x
    LDA     ($0E),y
    BMI     nx
    LDA     WV_XH
    SBC     #$00
    JMP     hx
nx:
    LDA     WV_XH
    SBC     #$FF
.else
    LDA     ($0E),y                     ; x = base + offset
    BMI     nx
    CLC
    ADC     WV_XL
    STA     base+3,x
    LDA     WV_XH
    ADC     #$00
    JMP     hx
nx:
    CLC
    ADC     WV_XL
    STA     base+3,x
    LDA     WV_XH
    ADC     #$FF
.endif
hx:
    BNE     far
show:
    STA     high,x
    LDA     ($0C),y                     ; attributes: with the object's, or flipped by them
.if vflip || hflip
    EOR     FLIPS
.else
    ORA     FLIPS
.endif
    STA     base+2,x
    INX
    INX
    INX
    INX
    BNE     skip
    JMP     wrap
skip:
    INY
    DEC     $02
    BMI     next
    JMP     sprite
next:
    STX     OAM_NEXT
    LDX     #$01
    RTS
far:
    CMP     #$01
    BEQ     right
    CMP     #$FF
    BNE     skip
    PHA
    LDA     base+3,x                    ; x < 0: x + M >= 0?
    CLC
    ADC     WV_M
    PLA
    BCC     skip
    BCS     show
right:
    PHA
    LDA     base+3,x                    ; 256 <= x: below 256 + M?
    CMP     WV_M
    PLA
    BCS     skip
    BCC     show
.endmacro

; The first page full: on to the second, the same loop.
.macro TO_SECOND skip
    LDA     #$80
    STA     WV_PAGE
    JMP     skip
.endmacro

    SPRITE_LOOP OAM, WIDE_SPRHI, 0, 0, wide_s1n, wide_s1n_skip, wide_s1n_wrap
wide_s1n_wrap:
    TO_SECOND wide_s2n_skip
    SPRITE_LOOP OAM, WIDE_SPRHI, 0, 1, wide_s1h, wide_s1h_skip, wide_s1h_wrap
wide_s1h_wrap:
    TO_SECOND wide_s2h_skip
    SPRITE_LOOP OAM, WIDE_SPRHI, 1, 0, wide_s1v, wide_s1v_skip, wide_s1v_wrap
wide_s1v_wrap:
    TO_SECOND wide_s2v_skip
    SPRITE_LOOP OAM, WIDE_SPRHI, 1, 1, wide_s1b, wide_s1b_skip, wide_s1b_wrap
wide_s1b_wrap:
    TO_SECOND wide_s2b_skip
    SPRITE_LOOP EXTRA_OAM, EXTRA_SPRHI, 0, 0, wide_s2n, wide_s2n_skip, wide_sprites_full
    SPRITE_LOOP EXTRA_OAM, EXTRA_SPRHI, 0, 1, wide_s2h, wide_s2h_skip, wide_sprites_full
    SPRITE_LOOP EXTRA_OAM, EXTRA_SPRHI, 1, 0, wide_s2v, wide_s2v_skip, wide_sprites_full
    SPRITE_LOOP EXTRA_OAM, EXTRA_SPRHI, 1, 1, wide_s2b, wide_s2b_skip, wide_sprites_full

; Both pages full: out, as the original with OAM full.
wide_sprites_full:
    INC     WV_FULL
    STX     OAM_NEXT
    RTS

; ---------------------------------------------------------------------------- ;
; Invincibility (option bit 1, a testing aid): Mega Man takes no damage.

; Bank $38 $8F72, in place of LDA $A2 / BNE / LDA $96 / BNE (no damage while either is set; then
; BNE $8F95): invincible, no damage either.
wide_hurt:
    LDA     WIDE_OPTIONS
    AND     #OPT_INVINCIBLE
    ORA     $A2
    ORA     $96
    RTS

; ---------------------------------------------------------------------------- ;
; The part of the stage the enemies live in: the screens beside the camera's, as far as a join of
; two screens that are not side by side, as Zaxnes' decoder draws the margins (the margins show at
; most one screen past the two the camera may straddle: the join before its screen, and the one
; after the last it shows). A stage is a straight list of screens; its controllers (type $80)
; say which joins are not side by side: their settings (entry y of $82EB / $836B in bank $39) are
; $71 (or, after bit 7, a routine's address then $71), two bytes, then actions of three bytes
; (kind, $73, $72) and a screen of their own with bit 7, up to a 0 or a byte with bit 7: 1 and 3 a
; scroll down to the next or previous screen, 2 and 4 up, 5 and 6 a door right or left to a
; screen. Without a screen of their own they act from the controller's screen (x's), a controller
; that scrolls to the next one (1, 2) taking its scroll back (3, 4) from that next one; with one,
; a door leaves the join after (5) or before (6) its screen apart; without, a door does too when
; it loads the stage's other tiles ($73 with bit 7, which the game stores in $064F: the two sides
; do not show together, a machine room's tiles swapped for the stage's at its door out). Settings with $71 past 3 or a
; kind past 6 (a boss's) say nothing. The list (bank $3A) is read at $A000, bank $39 at $8000
; ($CBC0 maps both), the game's banks put back after.

; A byte of the list whose pointer is at X (zero page), entry Y: $A000 + its offset in bank $3A.
wide_list_byte:
    LDA     $00,x
    STA     $00
    LDA     $01,x
    CLC
    ADC     #$20
    STA     $01
    LDA     ($00),y
    RTS

wide_reach:
    TXA                                 ; X kept, and the game's scratch $00 / $01
    PHA
    LDA     $00
    PHA
    LDA     $01
    PHA
    LDA     BANK_8000
    STA     WV_BANKS
    LDA     BANK_A000
    STA     WV_BANKS + 1
    LDY     #CONTROLLER_BANK
    JSR     SET_BANK
    LDA     CAM_HI                      ; the joins: before the camera's screen, after the last
    SEC                                 ; it shows (its screen, or the next it straddles)
    SBC     #$01
    STA     WV_JL
    LDA     CAM_LO
    CMP     #$01
    LDA     CAM_HI
    ADC     #$00
    STA     WV_JR
    LDA     #$00
    STA     WV_AL
    STA     WV_AR
    TAY
@entry:
    STY     WV_ENTRY
    LDX     #LIST_SCREENS
    JSR     wide_list_byte
    CMP     #$FF
    BEQ     @done
    STA     WV_OWN
    LDX     #LIST_TYPE
    JSR     wide_list_byte
    CMP     #$80
    BNE     @next
    LDX     #LIST_XS                    ; on its screen: its x's low byte within [4, $FE]
    JSR     wide_list_byte
    CMP     #$04
    BCC     @next
    CMP     #$FF
    BEQ     @next
    LDX     #LIST_YS
    JSR     wide_list_byte
    JSR     wide_controller
@next:
    LDY     WV_ENTRY
    INY
    BNE     @entry
@done:
    LDA     CAM_HI                      ; the first screen: the one before the camera's, beside it
    BEQ     @first
    LDY     WV_AL
    BNE     @first
    SEC
    SBC     #$01
@first:
    STA     WV_LSTOP
    LDA     WV_JR                       ; the last: the one after, beside it
    LDY     WV_AR
    BNE     @last
    CLC
    ADC     #$01
@last:
    STA     WV_RSTOP
    LDA     #$06                        ; the game's banks back, as its NMI puts them back
    STA     BANK_SELECT
    STA     MMC3_SELECT
    LDA     WV_BANKS
    STA     BANK_8000
    STA     MMC3_BANK
    LDA     #$07
    STA     BANK_SELECT
    STA     MMC3_SELECT
    LDA     WV_BANKS + 1
    STA     BANK_A000
    STA     MMC3_BANK
    PLA
    STA     $01
    PLA
    STA     $00
    PLA
    TAX
    RTS

; The joins the controller of settings A on screen WV_OWN leaves apart: WV_AL / WV_AR set for
; WV_JL / WV_JR. Two walks of its actions: whether it is a scroll controller's and scrolls to the
; next screen, then the joins.
wide_controller:
    TAY
    LDA     SETTINGS_LO,y
    STA     $00
    LDA     SETTINGS_HI,y
    STA     $01
    LDY     #$00
    LDA     ($00),y
    BPL     @plain
    LDY     #$02                        ; a routine's address first
    LDA     ($00),y
    LDY     #$05
    BNE     @scroll
@plain:
    LDY     #$03
@scroll:
    CMP     #$04                        ; $71 past 3: not a scroll controller's
    BCC     :+
    RTS
:   STY     WV_AT
    LDA     #$00
    STA     WV_FWD
@check:
    LDA     ($00),y
    BEQ     @apply
    BMI     @apply
    CMP     #$07                        ; a kind past 6: not a scroll controller's
    BCC     :+
    RTS
:   TAX
    INY
    INY
    INY
    LDA     ($00),y
    BPL     :+
    INY                                 ; a screen of its own
    BNE     @check
:   CPX     #$03                        ; 1, 2: it scrolls to the next screen
    BCS     @check
    INC     WV_FWD
    BNE     @check
@apply:
    LDY     WV_AT
@action:
    LDA     ($00),y
    BEQ     @rts
    BMI     @rts
    TAX
    INY
    LDA     ($00),y                     ; its $73: bit 7, the stage's other tiles loaded
    AND     #$80
    STA     WV_TILES
    INY
    INY
    LDA     ($00),y
    BPL     @own
    INY                                 ; with a screen of its own: a door leaves a join apart
    LDA     WV_OWN
    CPX     #$05
    BEQ     @mark
    CPX     #$06
    BNE     @action
    SEC
    SBC     #$01
    JMP     @mark
@own:
    LDA     WV_OWN                      ; without: 1, 2 the join after its screen; 3, 4 that one
    CPX     #$03                        ; too when it scrolls to the next, else the one before
    BCC     @mark
    CPX     #$05
    BCS     @door
    LDX     WV_FWD
    BNE     @mark
    SEC
    SBC     #$01
@door:
    LDA     WV_TILES                    ; 5, 6: a door across the join after or before its
    BEQ     @action                     ; screen, apart only when it loads the stage's other
    LDA     WV_OWN                      ; tiles (the screens do not show together)
    CPX     #$05
    BEQ     @mark
    SEC
    SBC     #$01
@mark:
    CMP     WV_JL
    BNE     :+
    INC     WV_AL
:   CMP     WV_JR
    BNE     @action
    INC     WV_AR
    JMP     @action
@rts:
    RTS

; ---------------------------------------------------------------------------- ;
; The reserve. An enemy asleep in a margin, once its script's first frame has set it up, leaves
; the game's slots: its object tables wait in the reserve, which the hack draws, until it reaches
; the original screen and takes a free slot again (or leaves the margins and is dropped, as the
; game removes an enemy off the screen). The game's 14 enemy slots are for the enemies awake.

.macro SLOT_TO_RES                      ; slot X to reserve entry Y
.repeat OBJ_TABLES, f
    LDA     OBJ_TYPE + 23 * f,x
    STA     RES_DATA + RES_COUNT * f,y
.endrep
.endmacro

.macro RES_TO_SLOT                      ; reserve entry Y to slot X
.repeat OBJ_TABLES, f
    LDA     RES_DATA + RES_COUNT * f,y
    STA     OBJ_TYPE + 23 * f,x
.endrep
.endmacro

; Entry Y's screen x, from its tables, in WV_SL / WV_SH.
.macro RES_SCREEN_X
    SEC
    LDA     RES_DATA + RES_COUNT * ((OBJ_X - OBJ_TYPE) / 23),y
    SBC     CAM_LO
    STA     WV_SL
    LDA     RES_DATA + RES_COUNT * ((OBJ_XH - OBJ_TYPE) / 23),y
    SBC     CAM_HI
    STA     WV_SH
.endmacro

; Enemy X (asleep, set up) into the reserve, its slot freed; it stays when the reserve is full.
wide_park:
    LDY     #RES_COUNT - 1
@find:
    LDA     RES_USED,y
    BEQ     @free
    DEY
    BPL     @find
    RTS
@free:
    SLOT_TO_RES
    LDA     #$01
    STA     RES_USED,y
    LDA     OBJ_ENTRY,x
    STA     RES_ENTRY,y
    LDA     WV_HIDDEN,x
    STA     RES_HIDDEN,y
    LDA     #$00
    STA     OBJ_TYPE,x
    LDA     #$FF
    STA     OBJ_ENTRY,x
    STA     WV_DORMANT,x
    RTS

; The reserve emptied (a new start, or the game emptying its slots).
wide_reserve_clear:
    LDA     #$00
    LDY     #RES_COUNT - 1
@clear:
    STA     RES_USED,y
    DEY
    BPL     @clear
    RTS

; The game empties its object slots ($D6F1, a scroll to another screen): the reserve too, at the
; end of its loop ($D6FD, in place of DEX / BNE $D6F3 / RTS; the loop's branch comes back to $D6F3,
; so the hook cannot be at its head). Back with X 0, A $FF and Y as the original leaves them.
wide_slots_cleared:
    DEX
    BEQ     @done
    JMP     SLOTS_CLEAR_LOOP
@done:
    TYA
    PHA
    JSR     wide_reserve_clear
    PLA
    TAY
    LDX     #$00
    LDA     #$FF
    RTS

; A list entry's free slot ($81C4, in place of JSR $E590; Y the entry): none while the entry waits
; in the reserve (the game's own test: none while a slot holds it).
wide_free_slot:
    LDX     #RES_COUNT - 1
@entry:
    LDA     RES_USED,x
    BEQ     @next
    TYA
    CMP     RES_ENTRY,x
    BNE     @next
    SEC
    RTS
@next:
    DEX
    BPL     @entry
    JMP     FREE_SLOT

; The reserve's frame (after the list's spawns, before the objects run): an entry on the original
; screen wakes in a free slot (it waits at the edge while none is), one past the margins is
; dropped; the camera moving shows those hidden.
wide_reserve_frame:
    LDY     #RES_COUNT - 1
@entry:
    STY     WV_RES
    LDA     RES_USED,y
    BNE     @used
    JMP     @next
@used:
    LDA     WV_MOVED
    BEQ     @where
    LDA     #$00
    STA     RES_HIDDEN,y
@where:
    RES_SCREEN_X
    LDA     WV_SH
    BEQ     @screen
    JMP     @margin
@screen:
    LDX     #CONTROLLER - 1             ; on the original screen: a free slot
@slot:
    LDA     OBJ_TYPE,x
    BEQ     @wake
    DEX
    CPX     #FIRST_ENEMY
    BCS     @slot
    JMP     @next
@wake:
    RES_TO_SLOT
    LDA     #$FF
    STA     WV_DORMANT,x
    LDA     #$01
    STA     WV_STARTED,x
    LDA     #$00
    STA     WV_HIDDEN,x
    STA     RES_USED,y
    JMP     @next
@margin:
    JSR     wide_inside
    BCC     @next
    LDY     WV_RES                      ; past the margins: dropped
    LDA     #$00
    STA     RES_USED,y
@next:
    LDY     WV_RES
    DEY
    BMI     @rts
    JMP     @entry
@rts:
    RTS

; The reserve drawn (before the rest of OAM is hidden, $E26D): each entry in a free slot, or in
; slot BORROWED, whose own object waits in SAVE_DATA meanwhile, drawn as the drawer draws an
; object; its animation's progress goes back to the reserve.
wide_reserve_draw:
    LDY     #RES_COUNT - 1
@entry:
    STY     WV_RES
    LDA     RES_USED,y
    BNE     @used
    JMP     @next
@used:
    LDA     #$00
    STA     WV_BORROW
    LDX     #CONTROLLER - 1
@slot:
    LDA     OBJ_TYPE,x
    BNE     @taken
    JMP     @load
@taken:
    DEX
    CPX     #FIRST_ENEMY
    BCS     @slot
    LDX     #BORROWED                   ; none free: borrowed
    INC     WV_BORROW
.repeat OBJ_TABLES, f
    LDA     OBJ_TYPE + 23 * f,x
    STA     SAVE_DATA + f
.endrep
    LDA     WV_DORMANT,x
    STA     SAVE_DATA + OBJ_TABLES
    LDA     WV_HIDDEN,x
    STA     SAVE_DATA + OBJ_TABLES + 1
@load:
    RES_TO_SLOT
    LDA     RES_ENTRY,y
    STA     WV_DORMANT,x
    LDA     RES_HIDDEN,y
    STA     WV_HIDDEN,x
    JSR     wide_draw_one
    LDY     WV_RES
    SLOT_TO_RES
    LDA     WV_BORROW
    BNE     @restore
    JMP     @free
@restore:
.repeat OBJ_TABLES, f
    LDA     SAVE_DATA + f
    STA     OBJ_TYPE + 23 * f,x
.endrep
    LDA     SAVE_DATA + OBJ_TABLES
    STA     WV_DORMANT,x
    LDA     SAVE_DATA + OBJ_TABLES + 1
    STA     WV_HIDDEN,x
    JMP     @next
@free:
    LDA     #$00                        ; the free slot free again
    STA     OBJ_TYPE,x
    LDA     #$FF
    STA     OBJ_ENTRY,x
    STA     WV_DORMANT,x
@next:
    LDY     WV_RES
    DEY
    BMI     @rts
    JMP     @entry
@rts:
    RTS

; One object drawn, X its slot (kept: $E2A9 leaves in X whether OAM is full), as the drawer's
; loop does it ($E213-$E266): its animation's step counted down and stepped, its place
; (wide_place), its sprites ($E2A9).
wide_draw_one:
    TXA
    PHA
    JSR     @draw
    PLA
    TAX
    RTS
@draw:
    LDA     OBJ_FLAGS,x
    AND     #$02
    BNE     @rts
    LDA     OBJ_ANIM_BANK,x
    TAY
    JSR     SET_BANK
    DEC     OBJ_ANIM_TIME,x
    BNE     @place
    INC     OBJ_ANIM_STEP,x
    INC     OBJ_ANIM_STEP,x
    LDA     OBJ_ANIM_STEP,x
    CMP     OBJ_ANIM_END,x
    BCC     @step
    LDA     #$00
    STA     OBJ_ANIM_STEP,x
@step:
    TAY
    INY
    INY
    LDA     OBJ_ANIM_LO,x
    STA     $08
    LDA     OBJ_ANIM_HI,x
    STA     $09
    LDA     ($08),y
    STA     OBJ_ANIM_TIME,x
@place:
    JSR     wide_place
    BCS     @rts
    JSR     DRAW_SPRITES
@rts:
    RTS

wide_end:
.assert wide_end <= $8000, error, "widescreen code past the 10 KiB window"
```

### B. `tools/build.py`

```python
"""Builds the Mega Man 6 (USA) widescreen hack (advanced mode) and its BPS patch.

Usage: python -I tools/build.py <Mega Man 6 (USA).nes> [out.bps]

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH) for $5800: a 16 KiB
   PRG bank whose first 2 KiB the widescreen extension's PRG window shows at $5800-$5FFF (Zaxnes
   opens it at power-on for this patch: bank 32, page 0).
2. Lays the PRG-ROM out as 68 banks of 8 KiB: the game's 64 where they were, the hack's 16 KiB as
   banks 64-65, and the game's fixed banks 62 and 63 again as banks 66 and 67, the ones MMC3
   fixes at $C000 and $E000 (Zaxnes' MMC3 takes the last two banks, whatever their count).
3. Writes the hooks into the game's banks, each checked against the bytes it replaces (those of
   the fixed banks into both their copies).
4. Writes build/mm6wide.nes (header: 34 PRG banks of 16 KiB) and, if asked, the BPS patch (data
   without iNES header; the game's banks copied come from the player's ROM).
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCES = {0x988798A8: 'mm6wide'}
BANK_SIZE = 0x2000
PRG_BANKS = 64
PRG_SIZE = PRG_BANKS * BANK_SIZE
CHR_SIZE = 0
TARGET_BANKS = 68
HACK_BANK = 64
FIXED = (62, 63)
CODE_START = 0x5800
CODE_SIZE = 0x4000

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

NOP = 0xEA


def jsr(target):
    return bytes([0x20, target & 0xFF, target >> 8])


def jmp(target):
    return bytes([0x4C, target & 0xFF, target >> 8])


def hooks(sym):
    """The patches: (bank, address, original bytes, new bytes); banks 62 and 63 are the fixed
    ones ($C000, $E000), the others are seen at $8000."""
    out = []

    def put(bank, address, original, new):
        original = bytes.fromhex(original)
        new = bytes(new) + bytes([NOP]) * (len(original) - len(new))
        assert len(new) == len(original), hex(address)
        out.append((bank, address, original, new))

    # The NMI: the extension's settings.
    put(62, 0xC01F, 'a5 fb 8d 00 a0', jsr(sym['wide_nmi']))
    # Sprites: the frame's first, an object's place, its sprite loop, the rest of OAM hidden.
    put(63, 0xE1DD, 'a9 00 85 8c', jsr(sym['wide_frame_start']))
    put(62, 0xD1D4, 'a9 00 85 8c', jsr(sym['wide_frame_start']))
    # The energy bars always before the objects (the game takes turns, to flicker when OAM is
    # full; with the second page nothing is left over).
    put(62, 0xD1D8, 'a5 f3 29 01 f0 0e', jmp(0xD1EC))
    put(63, 0xE24B, 'bd b7 03 29 01 d0 17 bd b4 04 d0 12 bd 86 04 38 e5 f7 c9 04 90 08 85 00',
        jsr(sym['wide_place']) + bytes([0xB0, 0x19]))          # BCS $E269
    put(63, 0xE315, 'a6 8c a0 00 6c 08 00', jmp(sym['wide_sprites']))
    put(63, 0xE26D, 'a6 8c a9 f0', jmp(sym['wide_hide_rest']))
    # The test "on the screen" the scripts remove objects with, the object loop (sleeping
    # enemies).
    put(63, 0xE485, 'bd cb 04', jmp(sym['wide_on_screen']))
    put(63, 0xE43A, 'bd a0 03', jsr(sym['wide_think']))
    # The stage list's spawns (two walks), an entry's spawn (each walk its kind), a list enemy's
    # spawn.
    put(0x3A, 0x8120, 'a5 56 18 69 fe 85 02 a5 57 69 00 85 03 a5 56 18 69 08 85 00 a5 57 69 00 85 01',
        jmp(sym['wide_spawns']))
    put(0x3A, 0x81A7, '98 29 07 aa', jsr(sym['wide_spawn_filter']))
    put(0x3A, 0x81EA, 'a9 00 9d b4 04', jsr(sym['wide_spawned']))
    # The reserve: a list entry waiting there gets no slot; the game emptying its slots empties it.
    put(0x3A, 0x81C4, '20 90 e5', jsr(sym['wide_free_slot']))
    put(62, 0xD6FD, 'ca d0 f3 60', jmp(sym['wide_slots_cleared']))
    # Invincibility: Mega Man's damage.
    put(0x38, 0x8F72, 'a5 a2 d0 1f a5 96 d0 1b', jsr(sym['wide_hurt']) + bytes([0xD0, 0x1E]))  # BNE $8F95
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
        f'MEMORY {{ CODE: start = ${CODE_START:04X}, size = ${CODE_SIZE:04X}, '
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
    data = bytes(image[header:header + PRG_SIZE + CHR_SIZE])
    name = SOURCES.get(zlib.crc32(data))
    assert name, 'not Mega Man 6 (USA)'

    code, sym = assemble()
    assert len(code) == CODE_SIZE
    banks = [bytearray(data[n * BANK_SIZE:(n + 1) * BANK_SIZE]) for n in range(PRG_BANKS)]
    skip = {int(a, 16) for a in os.environ.get('WIDE_SKIP', '').split(',') if a}
    for bank, address, original, new in hooks(sym):
        if address in skip:
            continue
        at = address & (BANK_SIZE - 1)
        assert banks[bank][at:at + len(original)] == original, \
            f'bank {bank:02X} ${address:04X}: not the expected bytes'
        banks[bank][at:at + len(new)] = new
    layout = (banks + [bytearray(code[:BANK_SIZE]), bytearray(code[BANK_SIZE:])]
              + [banks[FIXED[0]], banks[FIXED[1]]])
    assert len(layout) == TARGET_BANKS
    target = b''.join(bytes(b) for b in layout) + data[PRG_SIZE:]

    out_header = bytearray(image[:header])
    if header:
        out_header[4] = TARGET_BANKS * BANK_SIZE // 0x4000
    out = os.path.join(ROOT, 'build', name + '.nes')
    open(out, 'wb').write(bytes(out_header) + target)
    print(f'{out}: data CRC {zlib.crc32(target):08X}')
    if len(sys.argv) > 2:
        sys.path.insert(0, os.path.join(ROOT, 'tools'))
        import make_bps
        fixed = (TARGET_BANKS - 2) * BANK_SIZE
        moved = [(fixed, fixed + 2 * BANK_SIZE, FIXED[0] * BANK_SIZE)]
        added = (PRG_SIZE, fixed)
        patch = make_bps.make(data, target, moved, [added])
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


def make(source, target, moved=(), fresh=()):
    """`moved`: (target start, target end, source start) of data the build moved; `fresh`:
    (target start, target end) of bytes the build made (the added banks), never read from the
    source even where a byte happens to match."""
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
            if any(start <= at < end for start, end in fresh):
                return False
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
