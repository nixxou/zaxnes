# Super Mario Bros. (World): the widescreen hack

This document describes completely the modification of *Super Mario Bros. (World)* that Zaxnes
ships as a built-in patch for the game's **advanced** widescreen mode. The appendices hold the new
code and the build script, which writes every hook after checking the bytes it replaces: with them
the ROM can be rebuilt identically.

The background of the margins is not the hack's work: Zaxnes' Super Mario Bros. margin decoder
lays the area out by running the game's own area parser in a sandbox (see
[`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md), section 4). In the **simple**
mode the ROM is not patched at all and the decoder alone widens the game; the advanced mode adds
this patch: the enemies, platforms and firebars of the margins.

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Super Mario Bros. (World)*, NROM, 32 KiB PRG, 8 KiB CHR |
| Source data (iNES header excluded) | CRC-32 `D445F698` |
| Target data | CRC-32 `5A8BA83E` (the hack's version in savestate headers; `tools/build.py` prints it), 56 KiB: the game's 32 KiB of PRG, a 16 KiB PRG bank added, the 8 KiB of CHR |
| Changes | hooks in the game's PRG (same size as what they replace); the new code in the added bank |
| Patch | `rust/crates/zax-nes/patches/smb1-world-widescreen.bps`, made against the data without header; the CHR-ROM, moved after the new bank, is copied from the player's ROM (BPS SourceCopy), so the patch carries only the hack's bytes |
| Zaxnes declaration | `builtin_patches.rs`: `advanced: true` (applied only in the advanced mode), `boot_window: Some((0x82, 0x00))`, `idle_loop: Some((0x8057, 0x8059))` |

The game has no room left for the code that would unlock the extension. Zaxnes therefore opens
it at power-on and reset for this patch (the boot window): unlocked, the PRG window showing the
added bank (bank 2, page 0) at `$5800-$5FFF`, and on to `$7FFF` (`$5007` bit 4; the game has no
PRG-RAM there). NROM maps the first 32 KiB at `$8000-$FFFF` as before, so the game runs as it was;
its hooks call the new code in the window. The data grows by one PRG bank; Zaxnes raises the
PRG bank count of the player's header accordingly when it applies the patch.

---

**Hack pack.** Zaxnes ships this hack as a hack pack,
`rust/crates/zax-nes/patches/smb1-world-widescreen.zxpk` (`tools/build.py <rom> --pack <out.zxpk>`;
`docs/HACK-PACKS.md`, `WIDESCREEN-EXTENSION.md` 7.1): the code goes to the extension's memory,
which the window shows from power-on (`$5008` = `$80`, page 0), and the hooks, each with the game's
bytes it replaces, are checked and applied by Zaxnes; the ROM keeps its size (no bank added,
nothing moved or copied). The option "Invincible" is a component of its own (`LDA #$02` in place of
`LDA InjuryTimer`), applied with or without the advanced mode. The level selector needs the hack's
code: it stays with the widescreen. The BPS patch described here is kept to compare.

## 2. What the game does, and what changes

The game runs everything in its NMI; its main loop is `JMP $8057` (the yield's idle loop).
Enemy slots 0-4 (5 is the special one: power-ups, Bowser's flame...), each variable an array
indexed by the slot: `Enemy_Flag $0F`, `Enemy_ID $16`, `Enemy_State $1E`, `Enemy_MovingDir $46`,
`Enemy_X_Speed $58`, `Enemy_PageLoc $6E`, `Enemy_X_Position $87`, `Enemy_Y_Speed $A0`,
`Enemy_Y_HighPos $B6`, `Enemy_Y_Position $CF`, ... Screen left edge `ScreenLeft_PageLoc $071A` /
`ScreenLeft_X_Pos $071C`, right edge `$071B` / `$071D`. Every relative coordinate the game uses
(drawing, bounding boxes, collisions) is 8-bit: `x - screen left`.

| Part | The game | The hack |
|---|---|---|
| NMI (`$8082`) | | the extension's settings each frame: `$5003 = min($5002, 248)`, extents 0 (the decoder draws the background), `$5007 = $16` (sprites free of the extents, second sprite page, window to `$7FFF`), `$5006` = 255 with the option "No slowdown" else 0 |
| an enemy's sprites (`SprObjectOffscrChk`, `$EB64`) | hides the sprite columns of an enemy off the screen | within the wide view (`-M-16 <= x < 256+M`, x the enemy's 16-bit screen x): its six sprites get their high byte (left column the enemy's x, right column 8 pixels on) and only its vertical offscreen bits apply |
| an enemy too far (`OffscreenBoundsCheck`, `TooFar`, `$D6D2`) | erases an enemy 72 pixels past an edge | an enemy of a reserved kind within the wide view goes to the **reserve** instead (then erased as the game does, its slot freed at the same moment); a reserve entry stays, or is erased past the view |
| the reserve (start of `GameEngine`, `$AEFE`) | | each entry runs once a frame, before the game's enemies, in a free slot (or slot 4 borrowed, its enemy set aside meanwhile): the game's own frame of its kind minus its collisions with Mario and the other enemies (see below); its six sprites are copied to the second sprite page with their high bytes and hidden in the first; back within the game's range in a free slot, it stays there |
| previews (after the reserve) | creates an enemy of its list when the screen's right edge comes 48 pixels near, if a slot is free | the list is read ahead, unconsumed: each enemy of a reserved kind not yet created, within the wide view, is drawn still (the game's drawing routine in a free or borrowed slot) in the second page |
| large platforms (`DrawLargePlatform`, `$E60A`) | hides each sprite off the screen (`GetXOffscreenBits`) | within the wide view: the six sprites get their high byte (8 pixels apart), none hidden |
| small platforms (`DrawSmallPlatform`, `$EDB3`) | hides the columns off the screen | within the wide view: high bytes (three columns, two rows), horizontal bits left out |
| firebars (`ProcFirebar` `$CD3F`, centre `$CD85`, parts `$CDD4`, `FirebarCollision` `$CE0D`) | skips a firebar off the screen; a part's x is the centre's 8-bit x plus its offset, hidden when that wrapped; each part checked against Mario | drawn when the centre is within 96 pixels of the view; each part's high byte from the centre's 16-bit x plus the signed offset, shown within the view; a part off the screen does not touch Mario |
| level select (`$826C`, `$829C`) | the hidden world select (B on the title screen once the game is beaten) | with the option on: B goes through every level, 1-1 to 8-4, shown at `$2073` ("w-l"); the second quest is not turned on |
| injury (`InjurePlayer`, `$D92C`) | | with the option "Invincible": as if the injury timer ran; falls and the timer still kill |

**Reserved kinds** (`wide_reserved_kinds`): Koopas `$00` `$03`, Buzzy Beetle `$02`, Hammer Bro
`$05`, Goomba `$06`, Bloober `$07`, Bullet Bill `$08`, Cheep Cheeps `$0A` `$0B`, Podoboo `$0C`,
Piranha Plant `$0D`, Paratroopas `$0E` `$0F` `$10`, Spiny `$12`, short firebars `$1B`-`$1E`, large
platforms `$25`-`$2A`, small platforms `$2B` `$2C`. Not: Lakitu (`$11`: the game sends him again
once erased), the balance (`$24`: two linked slots), long firebars (`$1F`-`$22`: a second slot for
their sprites), Bowser and the other special objects. Those keep the game's limits (72 pixels).

**Collisions.** The game's bounding boxes and collisions use 8-bit screen x: an object 100 pixels
left of the screen would collide at x 156. The reserve's runs therefore leave out
`EnemiesCollision` and `PlayerEnemyCollision` (normal enemies: `RunNormalEnemies` minus those) and
the platforms' collision with Mario (`RunLargePlatform` / `RunSmallPlatform` minus their bounding
box and collision); a firebar's parts off the screen are skipped by `FirebarCollision`.

**What an entry keeps** (`EACH_FIELD`, 23 arrays per slot): `$0F`, `$16`, `$1E`, `$46`, `$58`,
`$6E`, `$87`, `$A0`, `$B6`, `$CF`, `Enemy_SprAttrib $03C5`, `Enemy_X_MoveForce $0401`,
`Enemy_YMF_Dummy $0417`, `Enemy_Y_MoveForce $0434`, `Enemy_CollisionBits $0491`,
`Enemy_BoundBoxCtrl $049A`, `EnemyFrameTimer $078A`, `EnemyIntervalTimer $0796`,
`ShellChainCounter $0125`, `HammerBroJumpTimer $3C`, `HammerThrowingTimer $03A2`,
`FirebarSpinDirection $34`, `FirebarSpinSpeed $0388` (the kinds' other variables are aliases of
these). The game counts down its slots' timers in the NMI; the reserve counts its entries' the
same way (frame timers every frame, interval timers when `IntervalTimerControl $077F` was just
reloaded to `$14`), none while `TimerControl $0747` is set. The reserve belongs to one area
(`AreaData $E7/$E8`) and goes forward with the screen: another area or the screen going back
(Mario restarting, a castle loop) empties it.

**Previews.** The list (`EnemyData $E9`, offset `EnemyDataOffset $0739`, page
`EnemyObjectPageLoc $073A`, page select `$073B`) is read as `ProcessEnemyData` reads it: 2-byte
records, the first byte the column (high nibble) and row (low nibble; `$0F` a page control, `$0E`
an area pointer of 3 bytes, skipped), the second the next-page bit (7), the hard mode bit (6,
skipped unless `SecondaryHardMode $06CC`) and the kind (a Goomba is a Buzzy Beetle in primary hard
mode, `$076A`). Records left of the screen's right edge are skipped as the game does; reading stops
past the wide view, or after 16 records. A single enemy shows at its column, y = row x 16 (+ 8
for kinds below `$15`, as `CheckpointEnemyID` adds); a group (`$37`-`$3E`: Goombas or Koopas, 2 or
3, low or high) shows 48 pixels before its record, 24 pixels apart (the game creates it at the
screen's right edge when its record comes 48 pixels near; the real ones may land up to 15 pixels
off). A preview is drawn facing left, state 0 (1 for a red Koopa).

**The second sprite page** (`$5300`, high bytes at `$5400 + 4n`) holds ten groups of six sprites:
0-5 the reserve's entries, 6-9 the previews; unused groups are hidden (y `$F8`).

## 3. Variables (extension RAM)

`WV_M $5200` margin, `WV_T0 $5201` / `WV_T1 $5202` an object's 16-bit screen x, `WV_ENTRY $5203`,
`WV_SLOT $5204`, `WV_BORROW $5205`, `WV_RUNNING $5206`, `WV_FAR $5207`, `WV_SHOWN $5208`,
`WV_COUNT $520B`, `WV_AREA $520C` (2), `WV_SCREEN $520E` (2), `WV_SAVE $5210` (23, a borrowed
slot's enemy), `WV_T2 $5227`, `WV_HI6 $5228` (6, the high bytes of the object drawn last),
`WV_T3 $522E`, `WV_FB_OFF $522F`, `WV_LEVEL $5230`, the previews' `WV_P_* $5231-$523F`, the reserve
`RES $5240` (field f of entry e at `$5240 + 6 f + e`, 23 fields), the second sprite page
`$5300-$54FF`. The code is at `$5800`-`$7FFF` (the window).

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length.

| Address | Replaces | With |
|---|---|---|
| `$8082` | `LDA $0778` (NMI) | `JSR wide_nmi` |
| `$EB64` | `LDX $08 / LDA $03D1` | `JSR wide_enemy_sprites` |
| `$D6D2` | `JSR EraseEnemyObject` (`TooFar`) | `JSR wide_too_far` |
| `$AEFE` | `JSR ProcFireball_Bubble` (start of `GameEngine`) | `JSR wide_reserve` |
| `$826C` | `LDX WorldSelectEnableFlag` | `JSR wide_select_on` |
| `$829C` | `LDX WorldSelectNumber` | `JMP wide_next_level` |
| `$D92C` | `LDA InjuryTimer` | `JSR wide_injury` |
| `$E60A` | `JSR GetXOffscreenBits` (in `DrawLargePlatform`) | `JSR wide_large_platform_bits` |
| `$EDB3` | `LDA $03D1` (in `DrawSmallPlatform`) | `JSR wide_small_platform_bits` |
| `$CD3F` | `LDA $03D1` (in `ProcFirebar`) | `JSR wide_firebar_bits` |
| `$CD85` | `LDA #$01 / STA $00` (the firebar's centre) | `JSR wide_firebar_center / NOP` |
| `$CDD4` | the 26 bytes of the part's distance test (`CMP Enemy_Rel_XPos` ... `BNE SetVFbr`) | `JSR wide_firebar_ball / BCC $CDEE / LDA #$F8 / BNE $CE03` |
| `$CE0D` | `LDA StarInvincibleTimer` (`FirebarCollision`) | `JSR wide_firebar_hurts` |

Routines of the game the hack calls: `EraseEnemyObject $C998`, `GetEnemyOffscreenBits $F1AF`,
`RelativeEnemyPosition $F152`, `EnemyGfxHandler $E87D`, `GetEnemyBoundBox $E243`,
`EnemyToBGCollisionDet $DFC1`, `EnemyMovementSubs $C905`, `OffscreenBoundsCheck $D67A`,
`ProcFireball_Bubble $B624`, `GetXOffscreenBits $F1F6`, `LargePlatformSubroutines $C982`,
`DrawLargePlatform $E5C8`, `DrawSmallPlatform $ED66`, `MoveSmallPlatform $D655`,
`ProcFirebar $CD3C`; and the title screen's `NullJoypad $82BB`.

## 5. Options

Zaxnes writes their bits at `$500A` every frame (the game's hacks screen, "Advanced mode" on).

| Bit | Key | Label | Default | Effect |
|---|---|---|---|---|
| 0 | `level_select` | Level select (B on the title screen) / Sélecteur de niveau | off | a testing aid: B goes through the 32 levels (level 2 of worlds 1, 2, 4 and 7 starts past its entrance area) |
| 1 | `invincible` | Invincible | off | a testing aid: no injury; falls and the timer still kill |
| 2 | `no_slowdown` | No slowdown / Sans ralentissements | on | `$5006 = 255`: no lag frame; with Zaxnes' yield (`idle_loop`), a frame that does not need it costs nothing more |

## 6. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Super Mario Bros. (World).nes" build/smb1wide.bps
python -I tools/make_doc.py <zaxnes>/rust/docs/hacks/smb1/WIDESCREEN-HACK.md
```

`build.py` assembles `src/wide.s` for `$5800` (a 16 KiB bank, `$FF` fill), writes the hooks into the
game's PRG (checked), the target data (PRG, the bank, CHR), `build/smb1wide.nes` (header with 3 PRG
banks) and the BPS patch (`make_bps.py`: SourceRead, SourceCopy for the moved CHR, TargetRead,
TargetCopy for runs of one value). `WIDE_SKIP=<hex addresses>` leaves hooks out, for testing.

## 7. Limits

* The reserve holds 6 entries and the previews 4 (the second page's 64 sprites); past that, an
  enemy is erased as the game does and a preview is not shown.
* A reserve entry coming back within the game's range while the five slots are taken stays in the
  reserve: it does not touch Mario (rare).
* Shells and fireballs do not hit the reserve's entries.
* The game keeps two screens of terrain in its block buffer: an enemy far in the left margin (with a
  very large margin) may meet the wrong terrain.
* Groups' previews may be up to 15 pixels off where the game creates them.
* Two objects of the screen on the same lines can exceed the PPU's eight sprites per line: the game
  flickers as on the console.

### A. `src/wide.s`

```asm
; Super Mario Bros. (World): the widescreen hack (Zaxnes, advanced mode).
;
; This code is a 16 KiB PRG bank added after the game's 32 KiB; the widescreen extension's PRG
; window shows it at $5800-$7FFF ($5007 bit 4), Zaxnes opening it at power-on (the patch's boot
; window: bank 2, page 0). The game reaches it through hooks in its own code (tools/build.py).
; The background of the margins is not the hack's work: Zaxnes' margin decoder draws it.

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

MAX_MARGIN      = 248           ; the most the extension shows sprites in

; Extension RAM ($5200-$57FF; $5800 up is the window).
WV_M            = $5200         ; margin of this frame
WV_T0           = $5201         ; scratch
WV_T1           = $5202
WV_ENTRY        = $5203         ; the reserve entry at hand
WV_SLOT         = $5204         ; the enemy slot it runs in
WV_BORROW       = $5205         ; non-zero: that slot's own enemy is set aside meanwhile
WV_RUNNING      = $5206         ; non-zero while a reserve entry runs
WV_FAR          = $5207         ; set by the game's off-screen check during a run: 1 kept, 2 erased
WV_SHOWN        = $5208         ; set when the object drawn last shows (WV_HI6)
WV_COUNT        = $520B
WV_T2           = $5227         ; scratch
WV_T3           = $522E
WV_AREA         = $520C         ; 2 bytes: the area the reserve belongs to ($E7/$E8)
WV_SCREEN       = $520E         ; 2 bytes: the screen's left edge last frame (page, x)
WV_SAVE         = $5210         ; the borrowed slot's enemy (FIELDS bytes, to $5226)
RES_COUNT       = 6             ; reserve entries (second page groups 0-5)
PREVIEW_COUNT   = 4             ; preview groups (6-9)
RES             = $5240         ; field f of entry e at RES + RES_COUNT f + e
RES_FLAG        = RES           ; field 0, Enemy_Flag: 0 = empty
EXTRA_OAM       = $5300         ; the extension's second sprite page: 6 sprites per entry
EXTRA_HIGH      = $5400         ; + 4 n: high byte of the x of its sprite n

; The game.
OBJECT_OFFSET   = $08
ENEMY_FLAG      = $0F           ; + slot
ENEMY_ID        = $16
ENEMY_STATE     = $1E
ENEMY_PAGE      = $6E           ; + slot
ENEMY_X         = $87           ; + slot
ENEMY_SPR_ATTR  = $03C5
ENEMY_REL_X     = $03AE         ; Enemy_Rel_XPos
ENEMY_OFFSCREEN = $03D1         ; Enemy_OffscreenBits: d0-d3 horizontal, d4-d7 vertical
ENEMY_SPR_DATA  = $06E5         ; + slot: OAM offset of the enemy's six sprites
AREA_DATA       = $E7
SCREEN_PAGE     = $071A         ; ScreenLeft_PageLoc
SCREEN_X        = $071C         ; ScreenLeft_X_Pos
TIMER_CONTROL   = $0747
INTERVAL_CONTROL = $077F
MIRROR_PPU_CTRL = $0778
SPRITES         = $0200
ENEMY_SLOTS     = 5             ; slots 0-4; 5 is the special one (power-ups)

WORLD_NUMBER    = $075F
AREA_NUMBER     = $0760         ; the internal area number (level 2 of worlds 1, 2, 4, 7 has an
LEVEL_NUMBER    = $075C         ; entrance area before it)
OFF_WORLD       = $0766         ; the other player's
OFF_AREA        = $0767
OFF_LEVEL       = $0763
VRAM_BUFFER1    = $0300         ; its length, then PPU address, count, tiles, 0
INJURY_TIMER    = $079E
DASH_TILE       = $28

; The patch's options ($500A).
OPT_LEVEL_SELECT = $01          ; the title screen's B selects any level
OPT_INVINCIBLE  = $02           ; a testing aid: Mario is never hurt
OPT_NO_SLOWDOWN = $04           ; nearly a frame more of CPU time: the game never runs late
NO_SLOWDOWN_LINES = 255

WV_HI6          = $5228         ; 6 bytes: the high byte of the x of each sprite of the object drawn
                                ; last, when WV_SHOWN
WV_FB_OFF       = $522F         ; the firebar part drawn last is off the screen: no collision
WV_LEVEL        = $5230         ; the level selected on the title screen, 0-31
WV_P_COUNT      = $5231         ; previews drawn this frame
WV_P_OFF        = $5232         ; the list read ahead: offset, page, page select
WV_P_PAGE       = $5233
WV_P_SEL        = $5234
WV_P_LIM        = $5235         ; 2 bytes: the wide view's right edge (x, page)
WV_P_LEFT       = $5237         ; list entries left to read
WV_P_B0         = $5238         ; the record at hand
WV_P_B1         = $5239
WV_P_X          = $523A         ; its x (page WV_P_PAGE)
WV_P_ID         = $523B         ; the enemy to show, its y, its place, how many (group)
WV_P_Y          = $523C
WV_P_EX         = $523D
WV_P_EPAGE      = $523E
WV_P_N          = $523F

SCREEN_RIGHT_PAGE = $071B       ; ScreenRight_PageLoc
SCREEN_RIGHT_X  = $071D         ; ScreenRight_X_Pos
ENEMY_DATA      = $E9           ; the area's enemy list
ENEMY_DATA_OFFSET = $0739
ENEMY_PAGE_LOC  = $073A         ; EnemyObjectPageLoc
ENEMY_PAGE_SEL  = $073B         ; EnemyObjectPageSel
PRIMARY_HARD    = $076A
SECONDARY_HARD  = $06CC
ENEMY_MOVING_DIR = $46          ; + slot: 1 right, 2 left
ENEMY_Y_HIGH    = $B6
ENEMY_Y         = $CF
GOOMBA          = $06
BUZZY_BEETLE    = $02
RED_KOOPA       = $03

FIRST_FIREBAR   = $1B           ; short firebars $1B-$1E; long ones $1F-$22 use a second slot
LAST_SHORT_FIREBAR = $1E
LAST_FIREBAR    = $22
STAR_TIMER      = $079F
FIREBAR_RADIUS  = 96            ; how far a firebar's parts reach from its centre, at most
FIRST_LARGE_PLATFORM = $25      ; $25-$2A (not $24, the balance, two linked slots)
LAST_LARGE_PLATFORM = $2A
FIRST_SMALL_PLATFORM = $2B      ; $2B-$2C
LAST_SMALL_PLATFORM = $2C

; Routines.
NULL_JOYPAD     = $82BB         ; the title screen, after its world selection
PROC_FIREBALL_BUBBLE = $B624
ERASE_ENEMY     = $C998
GET_OFFSCREEN_BITS = $F1AF
RELATIVE_POSITION = $F152
ENEMY_GFX       = $E87D
GET_BOUND_BOX   = $E243
ENEMY_BG_COLLISION = $DFC1
ENEMY_MOVEMENT  = $C905
OFFSCREEN_BOUNDS = $D67A
GET_X_OFFSCREEN_BITS = $F1F6
LARGE_PLATFORM_SUBS = $C982     ; LargePlatformSubroutines
DRAW_LARGE_PLATFORM = $E5C8
DRAW_SMALL_PLATFORM = $ED66
MOVE_SMALL_PLATFORM = $D655
PROC_FIREBAR    = $CD3C

; The enemy variables a reserve entry keeps, per slot (each is an array indexed by the slot).
.macro EACH_FIELD m
    m $0F, 0                    ; Enemy_Flag
    m $16, 1                    ; Enemy_ID
    m $1E, 2                    ; Enemy_State
    m $46, 3                    ; Enemy_MovingDir
    m $58, 4                    ; Enemy_X_Speed
    m $6E, 5                    ; Enemy_PageLoc
    m $87, 6                    ; Enemy_X_Position
    m $A0, 7                    ; Enemy_Y_Speed
    m $B6, 8                    ; Enemy_Y_HighPos
    m $CF, 9                    ; Enemy_Y_Position
    m $03C5, 10                 ; Enemy_SprAttrib
    m $0401, 11                 ; Enemy_X_MoveForce
    m $0417, 12                 ; Enemy_YMF_Dummy
    m $0434, 13                 ; Enemy_Y_MoveForce
    m $0491, 14                 ; Enemy_CollisionBits
    m $049A, 15                 ; Enemy_BoundBoxCtrl
    m $078A, 16                 ; EnemyFrameTimer
    m $0796, 17                 ; EnemyIntervalTimer
    m $0125, 18                 ; ShellChainCounter
    m $3C, 19                   ; HammerBroJumpTimer
    m $03A2, 20                 ; HammerThrowingTimer
    m $34, 21                   ; FirebarSpinDirection
    m $0388, 22                 ; FirebarSpinSpeed
.endmacro
FIELDS          = 23
RES_FRAME_TIMER = RES + RES_COUNT * 16
RES_INTERVAL_TIMER = RES + RES_COUNT * 17

.macro TO_ENTRY address, field         ; slot X -> entry Y
    LDA     address,x
    STA     RES + RES_COUNT * field,y
.endmacro
.macro TO_SLOT address, field          ; entry Y -> slot X
    LDA     RES + RES_COUNT * field,y
    STA     address,x
.endmacro
.macro TO_SAVE address, field          ; slot X -> WV_SAVE
    LDA     address,x
    STA     WV_SAVE + field
.endmacro
.macro FROM_SAVE address, field        ; WV_SAVE -> slot X
    LDA     WV_SAVE + field
    STA     address,x
.endmacro

.segment "CODE"

; ---------------------------------------------------------------------------- ;
; NMI ($8082, in place of LDA Mirror_PPU_CTRL_REG1): the extension's settings for the frame.
wide_nmi:
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
    LDA     MIRROR_PPU_CTRL
    RTS

; ---------------------------------------------------------------------------- ;
; Where enemy X is on the screen, 16-bit, into WV_T0 (low) and WV_T1 (high); C set when some of it
; (16 pixels wide) is within the margins or the screen: -M-16 <= x < 256+M.
wide_in_view:
    SEC
    LDA     ENEMY_X,x
    SBC     SCREEN_X
    STA     WV_T0
    LDA     ENEMY_PAGE,x
    SBC     SCREEN_PAGE
    STA     WV_T1
    BEQ     @in                         ; on the screen
    CMP     #$01
    BEQ     @right
    CMP     #$FF
    BNE     @out
    LDA     WV_T0                       ; left: x + 16 + M >= 0
    CLC
    ADC     #16
    BCS     @in
    ADC     WV_M
    RTS                                 ; C: within
@right:
    LDA     WV_T0                       ; 256 .. 256+M-1
    CMP     WV_M
    BCS     @out
@in:
    SEC
    RTS
@out:
    CLC
    RTS

; ---------------------------------------------------------------------------- ;
; An enemy's sprites ($EB64, SprObjectOffscrChk, in place of LDX ObjectOffset / LDA
; Enemy_OffscreenBits). The game hides the sprite columns of an enemy off the screen. One within
; the margins shows there: the high byte of the x of its six sprites (left column at the enemy's
; x, right column 8 pixels on) goes to the extension (WV_HI6 and WV_SHOWN for the reserve), and
; its horizontal offscreen bits are left out; its vertical ones stay.
; Out: X the enemy slot, A the offscreen bits to apply.
wide_enemy_sprites:
    LDX     OBJECT_OFFSET
    LDA     #$00
    STA     WV_SHOWN
    JSR     wide_in_view
    BCC     @hidden
    INC     WV_SHOWN
    LDY     ENEMY_SPR_DATA,x
    LDA     WV_T1                       ; left column
    STA     WV_HI6
    STA     WV_HI6+2
    STA     WV_HI6+4
    LDA     WV_T0                       ; right column: 8 pixels on
    CLC
    ADC     #$08
    LDA     WV_T1
    ADC     #$00
    STA     WV_HI6+1
    STA     WV_HI6+3
    STA     WV_HI6+5
    JSR     wide_put_high
    LDA     ENEMY_OFFSCREEN
    AND     #$F0
    RTS
@hidden:
    LDA     ENEMY_OFFSCREEN
    RTS

; ---------------------------------------------------------------------------- ;
; The reserve: enemies past the 72 pixels the game keeps them alive in off the screen, within the
; margins. They take no enemy slot: each frame, before the game's enemies, each one runs in a free
; slot (or one borrowed, its own enemy set aside meanwhile) through the game's own routines minus
; the collisions with Mario and the other enemies (which compare 8-bit screen x), then goes back
; to its entry; its sprites go to the second sprite page. One back within the game's own range
; stays in its slot when it ran in a free one.

; Only these kinds go to the reserve (the others are erased as the game does).
wide_reserved_kinds:
    .byte   $00, $02, $03, $06, $0E, $12 ; Koopas, Buzzy Beetle, Goomba, Paratroopa (jumping), Spiny
    .byte   $05, $07, $08, $0A, $0B, $0C ; Hammer Bro, Bloober, Bullet Bill, Cheep Cheeps, Podoboo
    .byte   $0D, $0F, $10               ; Piranha Plant, Paratroopas (red, green flying)
    .byte   $25, $26, $27, $28, $29, $2A ; large platforms (not the balance, $24)
    .byte   $2B, $2C                    ; small platforms
    .byte   $1B, $1C, $1D, $1E          ; short firebars
    ; Not Lakitu ($11): the game sends him again when it has erased him.
RESERVED_KINDS  = * - wide_reserved_kinds

; The game's off-screen check ($D6D2, TooFar, in place of JSR EraseEnemyObject): an enemy too far
; from the screen. A game enemy of a reserved kind within the margins goes to the reserve (its slot
; freed as the game does); a reserve entry within them stays (WV_FAR 1); any other is erased.
wide_too_far:
    LDA     WV_RUNNING
    BEQ     @game
    JSR     wide_in_view                ; a reserve entry
    BCC     @gone
    LDA     #1
    STA     WV_FAR
    RTS
@gone:
    LDA     #2
    STA     WV_FAR
    JMP     ERASE_ENEMY
@game:
    JSR     wide_in_view
    BCS     @within
    JMP     ERASE_ENEMY
@within:
    LDA     ENEMY_ID,x
    LDY     #RESERVED_KINDS - 1
@kind:
    CMP     wide_reserved_kinds,y
    BEQ     @free_entry
    DEY
    BPL     @kind
    JMP     ERASE_ENEMY
@free_entry:
    LDY     #RES_COUNT - 1
@find:
    LDA     RES_FLAG,y
    BEQ     @store
    DEY
    BPL     @find
    JMP     ERASE_ENEMY
@store:
    EACH_FIELD TO_ENTRY
    JMP     ERASE_ENEMY

; The reserve's frame ($AEFE, start of GameEngine, in place of JSR ProcFireball_Bubble).
wide_reserve:
    JSR     wide_reserve_check
    LDA     #1
    STA     WV_RUNNING
    LDY     #RES_COUNT - 1
@entry:
    STY     WV_ENTRY
    LDA     RES_FLAG,y
    BNE     @busy
    JSR     wide_hide_entry
    JMP     @next
@busy:
    ; Its timers, as the game counts its slots' (the master timer control stops them all).
    LDA     TIMER_CONTROL
    BNE     @timed
    LDA     RES_FRAME_TIMER,y
    BEQ     @frame_done
    SEC
    SBC     #1
    STA     RES_FRAME_TIMER,y
@frame_done:
    LDA     INTERVAL_CONTROL            ; just reloaded: the interval timers counted
    CMP     #$14
    BNE     @timed
    LDA     RES_INTERVAL_TIMER,y
    BEQ     @timed
    SEC
    SBC     #1
    STA     RES_INTERVAL_TIMER,y
@timed:
    ; A free slot, or slot 4 borrowed.
    LDX     #ENEMY_SLOTS - 1
@slot:
    LDA     ENEMY_FLAG,x
    BEQ     @free
    DEX
    BPL     @slot
    LDX     #ENEMY_SLOTS - 1
    JSR     wide_save_slot
    LDA     #1
    BNE     @borrowed
@free:
    LDA     #0
@borrowed:
    STA     WV_BORROW
    STX     WV_SLOT
    LDY     WV_ENTRY
    EACH_FIELD TO_SLOT
    STX     OBJECT_OFFSET
    LDA     #0
    STA     WV_FAR
    JSR     wide_run_enemy
    LDX     WV_SLOT
    LDY     WV_ENTRY
    JSR     wide_entry_sprites
    LDX     WV_SLOT
    LDY     WV_ENTRY
    ; Back in the game's range, run in a free slot: it stays there.
    LDA     ENEMY_FLAG,x
    BEQ     @out
    LDA     WV_FAR
    ORA     WV_BORROW
    BNE     @out
    LDA     #0
    STA     RES_FLAG,y
    JMP     @next
@out:
    EACH_FIELD TO_ENTRY                 ; an erased one leaves Enemy_Flag 0: the entry is free
    LDA     WV_BORROW
    BEQ     @release
    JSR     wide_restore_slot
    JMP     @next
@release:
    LDA     #0
    STA     ENEMY_FLAG,x
    STA     ENEMY_STATE,x
@next:
    LDY     WV_ENTRY
    DEY
    BMI     @done
    JMP     @entry
@done:
    LDA     #0
    STA     WV_RUNNING
    JSR     wide_previews
    JMP     PROC_FIREBALL_BUBBLE

; The game's frame of a normal enemy (RunNormalEnemies) without its collisions with Mario and the
; other enemies; X the slot.
wide_run_enemy:
    LDA     ENEMY_ID,x
    CMP     #FIRST_FIREBAR
    BCC     @not_firebar
    CMP     #LAST_FIREBAR + 1
    BCS     @not_firebar
    JSR     PROC_FIREBAR                ; RunFirebarObj
    JMP     OFFSCREEN_BOUNDS
@not_firebar:
    CMP     #FIRST_LARGE_PLATFORM
    BCC     @normal
    CMP     #FIRST_SMALL_PLATFORM
    BCC     @large
    JMP     wide_run_small_platform
@large:
    JMP     wide_run_large_platform
@normal:
    LDA     #$00
    STA     ENEMY_SPR_ATTR,x
    JSR     GET_OFFSCREEN_BITS
    JSR     RELATIVE_POSITION
    JSR     ENEMY_GFX
    JSR     GET_BOUND_BOX
    JSR     ENEMY_BG_COLLISION
    LDY     TIMER_CONTROL
    BNE     @still
    JSR     ENEMY_MOVEMENT
@still:
    JMP     OFFSCREEN_BOUNDS

; The game's frame of a large platform (RunLargePlatform) without its collision with Mario.
wide_run_large_platform:
    JSR     GET_OFFSCREEN_BITS
    JSR     RELATIVE_POSITION
    LDA     TIMER_CONTROL
    BNE     @still
    JSR     LARGE_PLATFORM_SUBS
@still:
    JSR     RELATIVE_POSITION
    JSR     DRAW_LARGE_PLATFORM
    JMP     OFFSCREEN_BOUNDS

; The game's frame of a small platform (RunSmallPlatform) without its collision with Mario.
wide_run_small_platform:
    JSR     GET_OFFSCREEN_BITS
    JSR     RELATIVE_POSITION
    JSR     RELATIVE_POSITION
    JSR     DRAW_SMALL_PLATFORM
    JSR     MOVE_SMALL_PLATFORM
    JMP     OFFSCREEN_BOUNDS

; WV_HI6 to the extension for the six sprites at OAM offset Y.
wide_put_high:
    LDA     WV_HI6
    STA     WIDE_SPRHI,y
    LDA     WV_HI6+1
    STA     WIDE_SPRHI+4,y
    LDA     WV_HI6+2
    STA     WIDE_SPRHI+8,y
    LDA     WV_HI6+3
    STA     WIDE_SPRHI+12,y
    LDA     WV_HI6+4
    STA     WIDE_SPRHI+16,y
    LDA     WV_HI6+5
    STA     WIDE_SPRHI+20,y
    RTS

; WV_HI6 for six sprites side by side from the object's x (WV_T0/T1), 8 pixels apart; with
; WV_T2 = 3, three side by side twice (a small platform).
wide_row_high:
    LDA     WV_T0
    STA     WV_T3
    LDA     WV_T1
    LDY     #0
@sprite:
    STA     WV_HI6,y
    PHA
    LDA     WV_T3
    CLC
    ADC     #8
    STA     WV_T3
    PLA
    ADC     #0
    INY
    CPY     WV_T2
    BNE     @sprite
    CPY     #6
    BEQ     @done
    LDA     WV_HI6                      ; the second row of a small platform
    STA     WV_HI6+3
    LDA     WV_HI6+1
    STA     WV_HI6+4
    LDA     WV_HI6+2
    STA     WV_HI6+5
@done:
    RTS

; A large platform's sprites (DrawLargePlatform, $E60A, in place of JSR GetXOffscreenBits, X the
; slot + 1): within the wide view, its six sprites get their high byte and none is hidden.
wide_large_platform_bits:
    DEX
    JSR     wide_in_view
    BCC     @native
    LDA     #6
    STA     WV_T2
    JSR     wide_row_high
    INC     WV_SHOWN
    LDY     ENEMY_SPR_DATA,x
    JSR     wide_put_high
    INX
    LDA     #0
    RTS
@native:
    LDA     #0
    STA     WV_SHOWN
    INX
    JMP     GET_X_OFFSCREEN_BITS

; A small platform's sprites (DrawSmallPlatform, $EDB3, in place of LDA Enemy_OffscreenBits, Y the
; OAM offset): within the wide view, its sprites get their high byte and none is hidden
; horizontally.
wide_small_platform_bits:
    LDX     OBJECT_OFFSET
    LDA     #0
    STA     WV_SHOWN
    TYA
    PHA
    JSR     wide_in_view
    BCC     @native
    LDA     #3
    STA     WV_T2
    JSR     wide_row_high
    INC     WV_SHOWN
    PLA
    TAY
    JSR     wide_put_high
    LDA     ENEMY_OFFSCREEN
    AND     #$F1
    RTS
@native:
    PLA
    TAY
    LDA     ENEMY_OFFSCREEN
    RTS

; ---------------------------------------------------------------------------- ;
; Firebars. The game draws a firebar's parts at its centre's 8-bit screen x plus each part's offset,
; hides the parts that wrapped, skips a firebar off the screen, and checks each part against Mario
; with those 8-bit coordinates. Within the wide view a firebar is drawn, each part getting the high
; byte of its true x (the centre's 16-bit x plus the offset, signed), shown when within the view;
; a part off the screen does not touch Mario.

; ProcFirebar ($CD3F, in place of LDA Enemy_OffscreenBits; X the slot): within reach of the wide
; view, the firebar is not skipped.
wide_firebar_bits:
    LDA     #0
    STA     WV_SHOWN
    JSR     wide_firebar_near
    BCC     @native
    INC     WV_SHOWN
    LDA     ENEMY_OFFSCREEN
    AND     #$F7
    RTS
@native:
    LDA     ENEMY_OFFSCREEN
    RTS

; C set when the firebar X's centre is within FIREBAR_RADIUS of the wide view; its 16-bit x on the
; screen in WV_T0/T1.
wide_firebar_near:
    SEC
    LDA     ENEMY_X,x
    SBC     SCREEN_X
    STA     WV_T0
    LDA     ENEMY_PAGE,x
    SBC     SCREEN_PAGE
    STA     WV_T1
    BEQ     @near
    CMP     #$01
    BEQ     @right
    CMP     #$FF
    BNE     @far
    LDA     WV_T0                       ; left: x + RADIUS + M >= 0
    CLC
    ADC     #FIREBAR_RADIUS
    BCS     @near
    ADC     WV_M
    RTS
@right:
    LDA     WV_T0                       ; right: x - 256 < M + RADIUS
    SEC
    SBC     #FIREBAR_RADIUS
    BCC     @near
    CMP     WV_M
    BCS     @far
@near:
    SEC
    RTS
@far:
    CLC
    RTS

; The firebar's centre (ProcFirebar, $CD85, in place of LDA #$01 / STA $00; Y its OAM offset): the
; high byte of its sprite, whether it is off the screen.
wide_firebar_center:
    TXA
    PHA
    LDX     OBJECT_OFFSET
    JSR     wide_firebar_near
    LDA     #0
    JSR     wide_firebar_part
    PLA
    TAX
    LDA     #$01
    STA     $00
    RTS

; A part of a firebar (DrawFirebar_Collision, $CDD4, in place of the test of its distance from the
; centre; A its 8-bit x, Y its OAM offset): C clear when it shows (within the wide view).
wide_firebar_ball:
    STA     WV_T2
    TXA
    PHA
    LDX     OBJECT_OFFSET
    JSR     wide_firebar_near
    LDA     WV_T2                       ; the part's offset from the centre, signed
    SEC
    SBC     ENEMY_REL_X
    JSR     wide_firebar_part
    PLA
    TAX
    LDA     WV_T1                       ; within the view: -M-8 <= x < 256+M
    BEQ     @shown
    CMP     #$01
    BEQ     @right
    CMP     #$FF
    BNE     @hidden
    LDA     WV_T0
    CLC
    ADC     #8
    BCS     @shown
    ADC     WV_M
    BCS     @shown
    BCC     @hidden
@right:
    LDA     WV_T0
    CMP     WV_M
    BCS     @hidden
@shown:
    CLC
    RTS
@hidden:
    SEC
    RTS

; Adds the signed offset A to the centre's x (WV_T0/T1), gives the part's sprite (OAM offset Y) its
; high byte (kept in WV_HI6 too for one of the slot's six), and WV_FB_OFF whether it is off the
; screen.
wide_firebar_part:
    STA     WV_T3
    CLC
    ADC     WV_T0
    STA     WV_T0
    LDA     WV_T3
    AND     #$80                        ; the sign, extended
    BEQ     @positive
    LDA     #$FF
@positive:
    ADC     WV_T1
    STA     WV_T1
    STA     WIDE_SPRHI,y
    BEQ     @on_screen
    LDA     #1
@on_screen:
    STA     WV_FB_OFF
    TYA                                 ; one of the slot's own six sprites: kept for the reserve
    SEC
    SBC     ENEMY_SPR_DATA,x
    BCC     @done
    CMP     #24
    BCS     @done
    LSR
    LSR
    STA     WV_T3
    TYA
    PHA
    LDY     WV_T3
    LDA     WV_T1
    STA     WV_HI6,y
    PLA
    TAY
@done:
    RTS

; FirebarCollision ($CE0D, in place of LDA StarInvincibleTimer): a part off the screen does not
; touch Mario, as if he had a star.
wide_firebar_hurts:
    LDA     WV_FB_OFF
    ORA     STAR_TIMER
    RTS

; The six sprites the game drew for slot X go to entry Y's place in the second page, with their
; high bytes; they are hidden in the first.
wide_entry_sprites:
    LDA     ENEMY_SPR_DATA,x
    STA     WV_T0
    LDX     wide_entry_oam,y
    STX     WV_T1
    LDY     WV_T0
    LDA     #24
    STA     WV_COUNT
@copy:
    LDA     SPRITES,y
    STA     EXTRA_OAM,x
    INY
    INX
    DEC     WV_COUNT
    BNE     @copy
    LDY     WV_T0
    LDA     #$F8
    STA     SPRITES,y
    STA     SPRITES+4,y
    STA     SPRITES+8,y
    STA     SPRITES+12,y
    STA     SPRITES+16,y
    STA     SPRITES+20,y
    LDX     WV_T1
    LDY     #0
@high:
    LDA     WV_SHOWN
    BEQ     @hidden
    LDA     WV_HI6,y
@hidden:
    STA     EXTRA_HIGH,x
    INX
    INX
    INX
    INX
    INY
    CPY     #6
    BNE     @high
    RTS

; Entry Y's sprites hidden in the second page.
wide_hide_entry:
    LDX     wide_entry_oam,y
    LDA     #$F8
    STA     EXTRA_OAM,x
    STA     EXTRA_OAM+4,x
    STA     EXTRA_OAM+8,x
    STA     EXTRA_OAM+12,x
    STA     EXTRA_OAM+16,x
    STA     EXTRA_OAM+20,x
    RTS

; Slot X's enemy set aside in WV_SAVE, and put back.
wide_save_slot:
    EACH_FIELD TO_SAVE
    RTS
wide_restore_slot:
    EACH_FIELD FROM_SAVE
    RTS

wide_entry_oam:                         ; the second page's groups of six sprites
    .byte   0, 24, 48, 72, 96, 120, 144, 168, 192, 216

; The reserve belongs to one area and goes forward with the screen: another area, or the screen
; back (Mario restarting, a castle loop), empties it.
wide_reserve_check:
    LDA     AREA_DATA
    CMP     WV_AREA
    BNE     @clear
    LDA     AREA_DATA+1
    CMP     WV_AREA+1
    BNE     @clear
    LDA     SCREEN_X
    CMP     WV_SCREEN+1
    LDA     SCREEN_PAGE
    SBC     WV_SCREEN
    BPL     @keep
@clear:
    LDA     #0
    LDY     #RES_COUNT - 1
@empty:
    STA     RES_FLAG,y
    DEY
    BPL     @empty
    LDA     AREA_DATA
    STA     WV_AREA
    LDA     AREA_DATA+1
    STA     WV_AREA+1
@keep:
    LDA     SCREEN_PAGE
    STA     WV_SCREEN
    LDA     SCREEN_X
    STA     WV_SCREEN+1
    RTS

; ---------------------------------------------------------------------------- ;
; Previews: the enemies of the area's list the game has not created yet, within the wide view
; past the screen's right edge, shown standing where they will appear (as still as the game's own
; drawing leaves them: an animation, no movement). The list is read ahead as the game reads it
; (ProcessEnemyData: the page control rows $0F, the next-page bit, the area pointer rows $0E, the
; hard mode bit), without being consumed; the game creates each one when it would anyway. A group
; ($37-$3E: 2 or 3 Goombas or Koopas) appears at the screen's right edge when its record comes 48
; pixels near: shown 48 pixels before the record, 24 pixels apart. Each is drawn by the game's own
; routines in a free or borrowed slot, its sprites going to the extension's second page.

PREVIEW_ENTRIES = 16            ; list entries read ahead at most

wide_previews:
    LDA     #0
    STA     WV_P_COUNT
    LDA     ENEMY_DATA_OFFSET
    STA     WV_P_OFF
    LDA     ENEMY_PAGE_LOC
    STA     WV_P_PAGE
    LDA     ENEMY_PAGE_SEL
    STA     WV_P_SEL
    LDA     SCREEN_X                    ; the wide view's right edge: left edge + 256 + M
    CLC
    ADC     WV_M
    STA     WV_P_LIM
    LDA     SCREEN_PAGE
    ADC     #1
    STA     WV_P_LIM+1
    LDA     #PREVIEW_ENTRIES
    STA     WV_P_LEFT
@entry:
    LDY     WV_P_OFF
    LDA     (ENEMY_DATA),y
    CMP     #$FF
    BNE     @record
@done:
    JMP     wide_previews_hide
@record:
    STA     WV_P_B0
    INY
    LDA     (ENEMY_DATA),y
    STA     WV_P_B1
    BPL     @row                        ; the next-page bit, once
    LDA     WV_P_SEL
    BNE     @row
    INC     WV_P_SEL
    INC     WV_P_PAGE
@row:
    LDA     WV_P_B0
    AND     #$0F
    CMP     #$0F
    BNE     @not_page
    LDA     WV_P_SEL                    ; page control: the page, unless just set
    BNE     @page_set
    LDA     WV_P_B1
    AND     #$3F
    STA     WV_P_PAGE
@page_set:
    LDA     #1
    STA     WV_P_SEL
    LDA     #2
    BNE     @advance_kept
@not_page:
    CMP     #$0E
    BNE     @enemy
    LDA     #3                          ; an area pointer: skipped
    BNE     @advance
@enemy:
    ; Not yet created: from the screen's right edge on (the game skips those behind it).
    LDA     WV_P_B0
    AND     #$F0
    STA     WV_P_X
    CMP     SCREEN_RIGHT_X
    LDA     WV_P_PAGE
    SBC     SCREEN_RIGHT_PAGE
    BCC     @next2
    ; Beyond the wide view: neither it nor the next ones show.
    LDA     WV_P_X
    CMP     WV_P_LIM
    LDA     WV_P_PAGE
    SBC     WV_P_LIM+1
    BCS     @done
    LDA     WV_P_B1                     ; hard mode only
    AND     #$40
    BEQ     @kind
    LDA     SECONDARY_HARD
    BEQ     @next2
@kind:
    JSR     wide_preview_record
@next2:
    LDA     #2
@advance:
    LDX     #0
    STX     WV_P_SEL
@advance_kept:
    CLC
    ADC     WV_P_OFF
    STA     WV_P_OFF
    DEC     WV_P_LEFT
    BEQ     @last
    JMP     @entry
@last:
    JMP     wide_previews_hide

; The enemy (or group) of the record in WV_P_B0/B1 at WV_P_PAGE:WV_P_X.
wide_preview_record:
    LDA     WV_P_B1
    AND     #$3F
    CMP     #$37
    BCC     @single
    CMP     #$3F
    BCS     @single
    ; A group: Goombas ($37-$3A) or Koopas ($3B-$3E), 2 or 3 (bit 0), low or high (bit 1).
    SEC
    SBC     #$37
    STA     WV_T0
    LDY     #$00                        ; green Koopa
    CMP     #$04
    BCS     @group_kind
    LDY     #GOOMBA
    LDA     PRIMARY_HARD
    BEQ     @group_kind
    LDY     #BUZZY_BEETLE
@group_kind:
    STY     WV_P_ID
    LDY     #$B8                        ; y $B0 or $70, plus the 8 the game adds
    LDA     WV_T0
    AND     #$02
    BEQ     @group_y
    LDY     #$78
@group_y:
    STY     WV_P_Y
    LDA     WV_T0
    AND     #$01
    CLC
    ADC     #2
    STA     WV_P_N
    SEC                                 ; the first 48 pixels before the record
    LDA     WV_P_X
    SBC     #48
    STA     WV_P_EX
    LDA     WV_P_PAGE
    SBC     #0
    STA     WV_P_EPAGE
@group_one:
    JSR     wide_preview_one
    LDA     WV_P_EX
    CLC
    ADC     #24
    STA     WV_P_EX
    LDA     WV_P_EPAGE
    ADC     #0
    STA     WV_P_EPAGE
    DEC     WV_P_N
    BNE     @group_one
    RTS
@single:
    CMP     #GOOMBA                     ; a Goomba is a Buzzy Beetle in primary hard mode
    BNE     @single_kind
    LDY     PRIMARY_HARD
    BEQ     @single_kind
    LDA     #BUZZY_BEETLE
@single_kind:
    STA     WV_P_ID
    LDA     WV_P_B0                     ; row * 16, plus the 8 the game adds to enemies ($00-$14)
    ASL
    ASL
    ASL
    ASL
    LDY     WV_P_ID
    CPY     #$15
    BCS     @y
    CLC
    ADC     #8
@y:
    STA     WV_P_Y
    LDA     WV_P_X
    STA     WV_P_EX
    LDA     WV_P_PAGE
    STA     WV_P_EPAGE
    ; fall through

; One enemy of kind WV_P_ID at WV_P_EPAGE:WV_P_EX, y WV_P_Y, drawn still in the next preview
; group of the second page (a kind the reserve keeps only).
wide_preview_one:
    LDA     WV_P_COUNT
    CMP     #PREVIEW_COUNT
    BCS     @skip
    LDA     WV_P_ID
    LDY     #RESERVED_KINDS - 1
@kind:
    CMP     wide_reserved_kinds,y
    BEQ     @drawn
    DEY
    BPL     @kind
@skip:
    RTS
@drawn:
    LDX     #ENEMY_SLOTS - 1            ; a free slot, or slot 4 borrowed
@slot:
    LDA     ENEMY_FLAG,x
    BEQ     @free
    DEX
    BPL     @slot
    LDX     #ENEMY_SLOTS - 1
    JSR     wide_save_slot
    LDA     #1
    BNE     @borrowed
@free:
    LDA     #0
@borrowed:
    STA     WV_BORROW
    STX     WV_SLOT
    LDA     #1
    STA     ENEMY_FLAG,x
    STA     ENEMY_Y_HIGH,x
    LDA     WV_P_ID
    STA     ENEMY_ID,x
    LDY     #0                          ; a red Koopa walks in state 1
    CMP     #RED_KOOPA
    BNE     @state
    INY
@state:
    TYA
    STA     ENEMY_STATE,x
    STA     ENEMY_SPR_ATTR,x
    LDA     #2                          ; facing left
    STA     ENEMY_MOVING_DIR,x
    LDA     WV_P_EPAGE
    STA     ENEMY_PAGE,x
    LDA     WV_P_EX
    STA     ENEMY_X,x
    LDA     WV_P_Y
    STA     ENEMY_Y,x
    STX     OBJECT_OFFSET
    JSR     GET_OFFSCREEN_BITS
    JSR     RELATIVE_POSITION
    LDA     WV_P_ID
    CMP     #FIRST_FIREBAR
    BCC     @enemy
    CMP     #LAST_FIREBAR + 1
    BCS     @platform
    JSR     PROC_FIREBAR
    JMP     @drawn_copy
@platform:
    CMP     #FIRST_LARGE_PLATFORM
    BCC     @enemy
    CMP     #FIRST_SMALL_PLATFORM
    BCC     @large
    JSR     DRAW_SMALL_PLATFORM
    JMP     @drawn_copy
@large:
    JSR     DRAW_LARGE_PLATFORM
    JMP     @drawn_copy
@enemy:
    JSR     ENEMY_GFX
@drawn_copy:
    LDA     WV_P_COUNT
    CLC
    ADC     #RES_COUNT
    TAY
    LDX     WV_SLOT
    JSR     wide_entry_sprites
    LDX     WV_SLOT
    LDA     WV_BORROW
    BEQ     @release
    JSR     wide_restore_slot
    JMP     @counted
@release:
    LDA     #0
    STA     ENEMY_FLAG,x
    STA     ENEMY_STATE,x
@counted:
    INC     WV_P_COUNT
    RTS

; The preview groups not used this frame hidden.
wide_previews_hide:
    LDA     WV_P_COUNT
    CMP     #PREVIEW_COUNT
    BCS     @done
    CLC
    ADC     #RES_COUNT
    TAY
    JSR     wide_hide_entry
    INC     WV_P_COUNT
    BNE     wide_previews_hide
@done:
    RTS

;---------------------------------------------------------------------------- ;
; The level selector (option bit 0): the game's hidden world selection (B on the title screen,
; otherwise open once the game is beaten) through every level, 1-1 to 8-4, shown in place of the
; title screen's "1-1"; it does not turn the second quest on as the hidden one does.

; ChkWorldSel ($826C, in place of LDX WorldSelectEnableFlag): Z clear when the selector is on;
; A (the buttons) kept.
wide_select_on:
    PHA
    LDA     WIDE_OPTIONS
    AND     #OPT_LEVEL_SELECT
    TAX
    PLA
    CPX     #0
    RTS

; IncWorldSel ($829C, jumped to in place of LDX WorldSelectNumber): the next level.
wide_next_level:
    LDX     WV_LEVEL
    INX
    TXA
    AND     #31
    STA     WV_LEVEL
    LSR
    LSR
    STA     WORLD_NUMBER
    STA     OFF_WORLD
    TAY
    LDA     WV_LEVEL
    AND     #3
    STA     LEVEL_NUMBER
    STA     OFF_LEVEL
    BEQ     @area                       ; level 1: area 0
    CLC
    ADC     wide_entrance_areas,y
@area:
    STA     AREA_NUMBER
    STA     OFF_AREA
    LDA     #6                          ; "w-l" at $2073
    STA     VRAM_BUFFER1
    LDA     #$20
    STA     VRAM_BUFFER1+1
    LDA     #$73
    STA     VRAM_BUFFER1+2
    LDA     #3
    STA     VRAM_BUFFER1+3
    LDY     WORLD_NUMBER
    INY
    STY     VRAM_BUFFER1+4
    LDA     #DASH_TILE
    STA     VRAM_BUFFER1+5
    LDY     LEVEL_NUMBER
    INY
    STY     VRAM_BUFFER1+6
    LDA     #0
    STA     VRAM_BUFFER1+7
    JMP     NULL_JOYPAD

wide_entrance_areas:                    ; worlds whose level 2 starts with an entrance area
    .byte   1, 1, 0, 1, 0, 0, 1, 0

; ---------------------------------------------------------------------------- ;
; Invincibility (option bit 1, a testing aid): InjurePlayer ($D92C, in place of LDA InjuryTimer)
; finds the injury timer running. Falling off the screen and the time running out still kill.
wide_injury:
    LDA     WIDE_OPTIONS
    AND     #OPT_INVINCIBLE
    ORA     INJURY_TIMER
    RTS

wide_end:
.assert wide_end <= $8000, error, "widescreen code past the window"
```

### B. `tools/build.py`

```python
"""Builds the Super Mario Bros. (World) widescreen hack (advanced mode) and its BPS patch.

Usage: python -I tools/build.py <Super Mario Bros. (World).nes> [out.bps]
       python -I tools/build.py <Super Mario Bros. (World).nes> --pack <out.zxpk>

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH) for $5800: a 16 KiB
   PRG bank added after the game's 32 KiB, which the widescreen extension's PRG window shows at
   $5800-$5FFF (Zaxnes opens it at power-on for this patch: the game has no room left for the
   code that would). NROM maps the first 32 KiB at $8000-$FFFF, so the game runs as it was.
2. Writes the hooks into the game's PRG-ROM, each checked against the bytes it replaces.
3. Writes build/smb1wide.nes (header: 3 PRG banks) and, if asked, the BPS patch (data without
   iNES header; the CHR-ROM, moved after the new bank, is copied from the player's ROM).

With --pack, a Zaxnes hack pack (tools/make_pack.py) instead: the code (its used part) for the
extension's memory, which Zaxnes shows in the window from power-on, and the hooks as offsets in
the PRG-ROM, the ROM keeping its size. Invincibility is a component of its own, "invincible"
(LDA #$02, the injury timer found running), applied by Zaxnes while that option is on, with or
without the widescreen; the level selector needs the hack's code and stays in the "widescreen"
component with the other hooks.
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCE_CRC = 0xD445F698
PRG_SIZE = 0x8000
CHR_SIZE = 0x2000
BANK_SIZE = 0x4000
CODE_START = 0x5800

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

BCS, BCC, BEQ, BNE = 0xB0, 0x90, 0xF0, 0xD0
NOP = 0xEA


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

    # NMI: the extension's settings for the frame (LDA Mirror_PPU_CTRL_REG1 done there).
    put(0x8082, 'ad 78 07', jsr(sym['wide_nmi']))
    # An enemy's sprites: shown in the margins (SprObjectOffscrChk).
    put(0xEB64, 'a6 08 ad d1 03', jsr(sym['wide_enemy_sprites']))
    # The reserve: enemies too far for the game kept within the margins, run each frame.
    put(0xD6D2, '20 98 c9', jsr(sym['wide_too_far']))
    put(0xAEFE, '20 24 b6', jsr(sym['wide_reserve']))
    # Options: the level selector on the title screen, invincibility.
    put(0x826C, 'ae fc 07', jsr(sym['wide_select_on']))
    put(0x829C, 'ae 6b 07', jmp(sym['wide_next_level']))
    put(0xD92C, 'ad 9e 07', jsr(sym['wide_injury']))
    # Platforms' sprites in the margins.
    put(0xE60A, '20 f6 f1', jsr(sym['wide_large_platform_bits']))
    put(0xEDB3, 'ad d1 03', jsr(sym['wide_small_platform_bits']))
    # Firebars in the margins: drawn, their parts placed, no collision off the screen.
    put(0xCD3F, 'ad d1 03', jsr(sym['wide_firebar_bits']))
    put(0xCD85, 'a9 01 85 00', jsr(sym['wide_firebar_center']))
    put(0xCDD4, 'cd ae 03 b0 09 ad ae 03 38 e5 06 4c e6 cd 38 ed ae 03 c9 59 90 04 a9 f8 d0 15',
        jsr(sym['wide_firebar_ball']) + branch(BCC, 0xCDD7, 0xCDEE) + bytes([0xA9, 0xF8])
        + branch(BNE, 0xCDDB, 0xCE03))
    put(0xCE0D, 'ad 9f 07', jsr(sym['wide_firebar_hurts']))
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


# The hook of the option "Invincible" and its bytes in the pack's "invincible" component.
INVINCIBLE = {0xD92C: bytes([0xA9, 0x02, NOP])}     # LDA #$02: the injury timer running


def pack(data, code, sym, out):
    sys.path.insert(0, os.path.join(ROOT, 'tools'))
    import make_pack
    widescreen = []
    invincible = []
    for address, original, new in hooks(sym):
        offset = address - 0x8000
        assert data[offset:offset + len(original)] == original, f'${address:04X}: not the expected bytes'
        if address in INVINCIBLE:
            invincible.append((offset, original, INVINCIBLE[address]))
        else:
            widescreen.append((offset, original, bytes(new)))
    assert len(invincible) == len(INVINCIBLE)
    used = (sym['wide_end'] - CODE_START + 0xFF) & ~0xFF
    result = make_pack.make('Super Mario Bros. (World) widescreen', code[:used],
                            [('widescreen', widescreen), ('invincible', invincible)])
    open(out, 'wb').write(result)
    print(f'{out}: {len(result)} bytes, pack CRC {zlib.crc32(result):08X}')


def main():
    rom_path = sys.argv[1]
    image = bytearray(open(rom_path, 'rb').read())
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = image[header:header + PRG_SIZE + CHR_SIZE]
    assert zlib.crc32(data) == SOURCE_CRC, 'not Super Mario Bros. (World)'

    code, sym = assemble()
    assert len(code) == BANK_SIZE
    if len(sys.argv) > 3 and sys.argv[2] == '--pack':
        pack(bytes(data), code, sym, sys.argv[3])
        return
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
    out = os.path.join(ROOT, 'build', 'smb1wide.nes')
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
