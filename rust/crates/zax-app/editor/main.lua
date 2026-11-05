-- The script of a new profile, made by the editor mode of Zaxnes.
-- The hooks (docs/spec/SPEC-05-lua.md §3): Start once when the game starts, then on every frame
-- Update, UpdateS once per shape of the frame (the global `shape`), LateUpdate; End at the end.
-- The API: docs/LUA-API.txt. Facts and pitfalls: docs/EDITOR-PROFILE-WORKFLOW.md.
--
-- z = 0 is the front and a positive z goes back; shape.OriPos is a shape's centre, its y counted
-- from the bottom (screen row = 248 - OriPos.y).

function Start()
  -- The left column the NES hides is drawn with stray colours while the screen scrolls.
  Clipping.x = 8.1
end

function Update()
end

function UpdateS()
  -- Every shape of the frame passes here: read shape.TagStr (the tags of its pattern),
  -- shape.Palette, shape.OriPos, shape.Size; change shape.Offset, shape.Scale, shape.Enable...
end

function LateUpdate()
  -- Shapes added with Frame:CreateNewShape go here, once per frame.
end
