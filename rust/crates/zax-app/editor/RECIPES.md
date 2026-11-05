# Lua recipes

Ways of doing the usual things, each tried on a full game (Metal Force). The API is in
`LUA-API.txt`, the hooks in `spec/SPEC-05-lua.md` §3. In `UpdateS` the global `shape` is the
shape being converted; its tags are `shape.TagStr` (its pattern's tags, `""` for a tile no
pattern matched).

## Find what a shape is

```lua
local function has(tag)
  return string.find(shape.TagStr or "", tag, 1, true) ~= nil
end

function UpdateS()
  if has("hud") then return end
  if shape.Sp then             -- a sprite
    shape.Offset.z = 4
    return
  end
  if has("floor") and shape.Palette == 2 then ... end
end
```

## Give a shape a depth and a place in depth

`Offset.z` **replaces** the pattern's offset; `Scale.z` scales the volume around its middle. A
pattern's depth cannot be changed from Lua for a matched shape, its scale can:

```lua
-- the shape spans [z, z + wanted] whatever its pattern's depth
local function placeDepth(wanted, z)
  local depth = shape.Size.z
  if depth <= 0 then return end
  shape.Scale.z = wanted / depth
  shape.Offset.z = z + wanted / 2 - depth / 2
end
```

## Know where you are in the game (level, title screen)

Reading the RAM works but slows the movie's seek (the scripts then drive the core). Recognise
the screen by what it shows: count the shapes of each level's tags during a frame, decide in the
next `Update`.

```lua
local marks, counting = { 0, 0, 0 }, { 0, 0, 0 }
local level = 1

function Update()
  marks, counting = counting, { 0, 0, 0 }
  local best = 1
  for i = 2, #marks do if marks[i] > marks[best] then best = i end end
  if marks[best] > 0 then level = best end
end

function UpdateS()
  local tags = shape.TagStr or ""
  for i = 1, #counting do
    if string.find(tags, "level" .. i, 1, true) then counting[i] = counting[i] + 1 end
  end
  -- then act on `level`
end
```

Tag each level's classes `level1`, `level2`... (`tags` of a class of `classify`). A level that
shares many tiles with another needs a stricter rule (its own tiles only, or a ratio).

## Hide a tile and draw something else

```lua
local stars = {}

function Update() stars = {} end

function UpdateS()
  if string.find(shape.TagStr or "", "spike", 1, true) then
    local centre, size = shape.OriPos, shape.Size
    stars[#stars + 1] = { centre.x - (size.x - 1) / 2, centre.y - (size.y - 1) / 2 }
    shape.Enable = false                 -- the game's flat tile is not drawn
  end
end

local model = nil
function LateUpdate()
  model = model or Pattern.GetPatternWithTag("spike3d")   -- one-word tag, hand-written pattern
  if not model then return end
  for _, s in ipairs(stars) do
    Frame:CreateNewShape(model, Vector3(s[1], s[2], 0), 0)   -- left, bottom, z
  end
end
```

* `shape.OriPos` is the shape's **centre**, y counted from the bottom (screen row of a pixel =
  248 - y); its corner is `OriPos - (Size - 1) / 2`. `shape.Pos` is 0 on matched shapes.
* Touching tiles of one pattern make **one** shape: walk its tiles (`Size.x / 8` by `Size.y / 8`)
  to put one model per tile.
* A pattern made only to be drawn from Lua needs a tile picture no game tile has (else it
  matches the game's tiles too) and its look in a `texture`: invent planes (e.g. a hash of the
  name under the mask of the texture's pixels) and check them against your dumps.
* `CreateNewShape` adds the pattern's own offset to the position given.

## Repeat a motif in depth rather than extruding it

A thin slice repeated with gaps keeps a motif (grass, a fence, fire) where an extrusion smears
it. In `profile.json`: `"depth": 2, "copies": [{ "offset": [0, 0, 6] }, { "offset": [0, 0, 12] }]`.
For an animated or waving motif, put the copies from Lua instead (a slice pattern drawn at each
z with a small `y` shift that follows `math.sin(frame * speed - z * k)`).

## Animated tiles

Each step of an animation is a different picture. Dump consecutive frames (`step 1`) until the
animation loops, so that every step gets a pattern; a missing step shows with the default look.

## Margins

The NES hides its left column and often scrolls garbage at the screen's edges:

```lua
function Start()
  Clipping.x = 8.1    -- left (x), bottom (y), right (z), top (w), in pixels
end
```

`Setting.SameBgPalette = true` keeps touching tiles of different palettes in separate shapes.

## Zaxnes-only code (lighter on a handheld)

A profile must stay readable by 3dSen. Code for Zaxnes alone goes in comments that 3dSen skips
(`EDITOR-PROFILE-WORKFLOW.md` §4):

```lua
--[[#zax handheld
local ROWS = { 0, 16 }        -- fewer copies on a handheld
#zax skip-legacy
]]
-- #legacy-begin
local ROWS = { 0, 8, 16 }
-- #legacy-end
```

The views `3S` (as 3dSen reads the script) and `HH` (as a handheld does) of the editor's tool box
show each reading.
