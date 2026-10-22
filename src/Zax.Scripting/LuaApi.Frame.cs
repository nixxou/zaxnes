using KeraLua;
using Zax.Engine3D;

namespace Zax.Scripting;

/// <summary>
/// The globals <c>Frame</c>, <c>Palette</c>, <c>Pattern</c> and <c>Script</c> of SPEC-05 §4.2.
/// </summary>
internal static partial class LuaApi
{
    private static void RegisterFrame(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        HostType type = binder.DeclareType(HostTypeId.FrameApi, "FrameAPI");
        ConversionEngine engine = runtime.Engine;

        type.Property("shapeCount", (_, l) => Number(l, engine.ScriptableCount));
        type.Property("frameCounter", (_, l) => Number(l, runtime.FrameCounter));
        type.Property("time", (_, l) => Number(l, runtime.FrameCounter / 60f));

        // The shared list of REQ-LUA-048: emptied and refilled at every call, the same object
        // every time.
        type.Function("GetShapesWithTag", (b, l) =>
        {
            RequireFrameApi(b, l, "GetShapesWithTag");
            string tag = b.CheckString(l, 2, "GetShapesWithTag");
            engine.CollectShapesWithTag(tag, runtime.InstanceQuery);
            b.PushDescriptor(l, HostTypeId.InstanceList, runtime, (int)ListKind.ShapesWithTag);
            return 1;
        }, HostTypeId.InstanceList);

        type.Function("GetShapeWithTag", (b, l) =>
        {
            RequireFrameApi(b, l, "GetShapeWithTag");
            string tag = b.CheckString(l, 2, "GetShapeWithTag");
            b.PushObject(l, HostTypeId.Instance, engine.GetShapeWithTag(tag));
            return 1;
        }, HostTypeId.Instance);

        type.Function("GetShape", (b, l) =>
        {
            RequireFrameApi(b, l, "GetShape");
            int index = b.CheckInteger(l, 2, "GetShape");
            b.CheckBounds(l, index, engine.ScriptableCount, "GetShape");
            b.PushObject(l, HostTypeId.Instance, engine.GetShape(index));
            return 1;
        }, HostTypeId.Instance);

        // Without a tag the third argument is the empty string, which matches no instance, so
        // the two-argument call always gives nil (REQ-LUA-084 point 5).
        type.Function("GetShapeWithPos", (b, l) =>
        {
            RequireFrameApi(b, l, "GetShapeWithPos");
            float x = (float)b.CheckNumber(l, 2, "GetShapeWithPos");
            float y = (float)b.CheckNumber(l, 3, "GetShapeWithPos");
            string tag = b.OptionalString(l, 4, "GetShapeWithPos", string.Empty);
            b.PushObject(l, HostTypeId.Instance, engine.GetShapeWithPos(x, y, tag));
            return 1;
        }, HostTypeId.Instance);

        // The result type depends on the first argument — an instance gives an instance, a
        // pattern gives a shape (REQ-LUA-086) — so no single result type is declared and an
        // access chain through one of them stops at the static check.
        type.Function("AddNewShape", (b, l) => Inject(b, l, runtime, InjectKind.Add));
        type.Function("CreateNewShape", (b, l) => Inject(b, l, runtime, InjectKind.Create));
        type.Function("InsertShape", (b, l) => Inject(b, l, runtime, InjectKind.Insert));

        type.Function("PinNewFrame", (b, l) =>
        {
            RequireFrameApi(b, l, "PinNewFrame");
            engine.PinNewFrame();
            return 0;
        });

        type.Function("ReleasePinnedFrame", (b, l) =>
        {
            RequireFrameApi(b, l, "ReleasePinnedFrame");
            engine.ReleasePinnedFrame();
            return 0;
        });

        type.Function("AddRectFromPinnedFrame", (b, l) =>
        {
            RequireFrameApi(b, l, "AddRectFromPinnedFrame");
            float x = (float)b.CheckNumber(l, 2, "AddRectFromPinnedFrame");
            float y = (float)b.CheckNumber(l, 3, "AddRectFromPinnedFrame");
            float width = (float)b.CheckNumber(l, 4, "AddRectFromPinnedFrame");
            float height = (float)b.CheckNumber(l, 5, "AddRectFromPinnedFrame");
            engine.AddRectFromPinnedFrame(x, y, width, height);
            return 0;
        });

        type.Function("RestoreFromPinnedFrame", (b, l) =>
        {
            RequireFrameApi(b, l, "RestoreFromPinnedFrame");
            if (l.IsNoneOrNil(2))
            {
                engine.RestoreFromPinnedFrame();
                return 0;
            }

            engine.RestoreFromPinnedFrame(b.CheckRect(l, 2, "RestoreFromPinnedFrame"));
            return 0;
        });

        // Either a rectangle or a position and a size (§4.2).
        type.Function("GetShapesFromPinnedFrame", (b, l) =>
        {
            RequireFrameApi(b, l, "GetShapesFromPinnedFrame");
            Rect rectangle;
            if (b.TryGetRect(l, 2, out Rect given))
            {
                rectangle = given;
            }
            else if (b.TryGetVector2(l, 2, out Vec2 origin) && b.TryGetVector2(l, 3, out Vec2 size))
            {
                rectangle = new Rect(origin.X, origin.Y, size.X, size.Y);
            }
            else
            {
                return b.InvalidArguments(l, "GetShapesFromPinnedFrame");
            }

            engine.CollectShapesFromPinnedFrame(rectangle, runtime.PinnedQuery);
            b.PushDescriptor(l, HostTypeId.ShapeList, runtime, (int)ListKind.ShapesFromPinnedFrame);
            return 1;
        }, HostTypeId.ShapeList);

        // FrameManager is the class table of the frame manager; FrameManager.Ins is Frame.
        HostType manager = binder.DeclareType(HostTypeId.FrameManagerApi, "FrameManagerClasse");
        manager.Property(
            "Ins",
            (b, l) =>
            {
                b.PushObject(l, HostTypeId.FrameApi, runtime.TableOf(HostTypeId.FrameApi));
                return 1;
            },
            result: HostTypeId.FrameApi);
    }

    private enum InjectKind
    {
        Add,
        Create,
        Insert,
    }

    /// <summary>
    /// <c>Frame:AddNewShape</c>, <c>CreateNewShape</c> and <c>InsertShape</c> (REQ-LUA-086):
    /// the first argument decides the variant — an instance gives back the instance of the same
    /// index of the new shape, a pattern gives back the shape.
    /// </summary>
    private static int Inject(LuaBinder binder, Lua state, ProfileScriptRuntime runtime, InjectKind kind)
    {
        string member = kind switch
        {
            InjectKind.Add => "AddNewShape",
            InjectKind.Create => "CreateNewShape",
            _ => "InsertShape",
        };

        RequireFrameApi(binder, state, member);
        Vec3 position = binder.CheckVector3(state, 3, member);
        int palette = binder.CheckInteger(state, 4, member);
        ConversionEngine engine = runtime.Engine;

        if (binder.ObjectAt(state, 2, HostTypeId.Instance) is PatternInstance instance)
        {
            FrameInstance result = kind switch
            {
                InjectKind.Add => engine.AddNewShape(instance, position, palette),
                InjectKind.Create => engine.CreateNewShape(instance, position, palette),
                _ => InsertInstance(engine, instance, position, palette),
            };

            binder.PushObject(state, HostTypeId.Instance, result);
            return 1;
        }

        if (binder.ObjectAt(state, 2, HostTypeId.Pattern) is EnginePattern pattern)
        {
            FrameShape shape = kind switch
            {
                InjectKind.Add => engine.AddNewShape(pattern, position, palette),
                InjectKind.Create => engine.CreateNewShape(pattern, position, palette),
                _ => engine.InsertShape(pattern, position, palette),
            };

            binder.PushObject(state, HostTypeId.Shape, shape);
            return 1;
        }

        // Passing nil where an object is needed raises an ordinary Lua error (REQ-LUA-047).
        return binder.InvalidArguments(state, member);
    }

    private static FrameInstance InsertInstance(
        ConversionEngine engine,
        PatternInstance instance,
        Vec3 position,
        int palette)
    {
        // InsertShape is CreateNewShape with the pattern's bottom-left corner removed
        // (REQ-E3D-232).
        EnginePattern? pattern = engine.FindPattern(instance);
        if (pattern is null)
        {
            return engine.CreateNewShape(instance, position, palette);
        }

        Vec2 corner = pattern.BottomLeft;
        Vec3 shifted = new(position.X - corner.X, position.Y - corner.Y, position.Z);
        return engine.CreateNewShape(instance, shifted, palette);
    }

    private static void RequireFrameApi(LuaBinder binder, Lua state, string member)
    {
        // Frame's members are methods: calling one with '.' leaves the first argument in the
        // receiver's place and must raise (REQ-LUA-081).
        if (binder.TypeIdAt(state, 1) != HostTypeId.FrameApi)
        {
            binder.InvalidArguments(state, member);
        }
    }

    private static void RegisterPalette(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        HostType type = binder.DeclareType(HostTypeId.PaletteApi, "PaletteAPI");
        ConversionEngine engine = runtime.Engine;

        // Table functions, called with '.', so the entry is argument 1 (REQ-LUA-081).
        type.Function("FrameColorIndex", (b, l) => ReadEntry(b, l, engine, "FrameColorIndex"));
        type.Function("GetColorIndex", (b, l) => ReadEntry(b, l, engine, "GetColorIndex"));

        type.Function("SetColorIndex", (b, l) =>
        {
            int entry = Entry(b, l, 1, "SetColorIndex");
            int value = b.CheckInteger(l, 2, "SetColorIndex");
            bool current = b.OptionalNumber(l, 3, "SetColorIndex", 0.0) != 0.0;
            engine.SetColorIndex(entry, value & 0x3F, current);
            return 0;
        });

        type.Function("FrameColor", (b, l) =>
        {
            int entry = Entry(b, l, 1, "FrameColor");
            Rgb24 colour = engine.FrameColor(entry);
            b.PushColor(l, LuaColor.FromBytes(colour.R, colour.G, colour.B));
            return 1;
        }, HostTypeId.Color);

        type.Property(
            "bgColor",
            (b, l) =>
            {
                Rgb24 colour = engine.BackgroundColor(skyboxActive: runtime.Settings.SkyboxEnable);
                b.PushColor(l, LuaColor.FromBytes(colour.R, colour.G, colour.B));
                return 1;
            },
            result: HostTypeId.Color);
        return;

        static int ReadEntry(LuaBinder binder, Lua state, ConversionEngine engine, string member)
        {
            int entry = Entry(binder, state, 1, member);
            bool current = binder.OptionalNumber(state, 2, member, 0.0) != 0.0;
            return Number(state, engine.FrameColorIndex(entry, current));
        }

        static int Entry(LuaBinder binder, Lua state, int index, string member)
        {
            int entry = binder.CheckInteger(state, index, member);
            if ((uint)entry >= (uint)FramePalette.EntryCount)
            {
                binder.Raise(state, $"palette entry {entry} is out of range for {member}");
            }

            return entry;
        }
    }

    private static void RegisterPatternTable(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        HostType type = binder.DeclareType(HostTypeId.PatternApi, "PatternAPI");
        ConversionEngine engine = runtime.Engine;

        type.Function("GetPatternWithTag", (b, l) =>
        {
            string tag = b.CheckString(l, 1, "GetPatternWithTag");
            b.PushObject(l, HostTypeId.Instance, engine.GetPatternWithTag(tag));
            return 1;
        }, HostTypeId.Instance);

        type.Function("GetPatternsWithTag", (b, l) =>
        {
            string tag = b.CheckString(l, 1, "GetPatternsWithTag");
            runtime.PatternQuery.Clear();
            runtime.PatternQuery.AddRange(engine.GetPatternsWithTag(tag));
            b.PushDescriptor(l, HostTypeId.InstanceList, runtime, (int)ListKind.PatternsWithTag);
            return 1;
        }, HostTypeId.InstanceList);
    }

    private static void RegisterScriptTable(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        HostType type = binder.DeclareType(HostTypeId.ScriptApi, "ScriptAPI");

        type.Property("Count", (_, l) => Number(l, runtime.Scripts.Count));

        type.Function("Get", (b, l) =>
        {
            Require(b, l, "Get");
            string name = b.CheckString(l, 2, "Get");
            b.PushObject(l, HostTypeId.ScriptObj, runtime.FindScript(name));
            return 1;
        }, HostTypeId.ScriptObj);

        // GetScript takes a name or a 0-based position in script order (§4.2).
        type.Function("GetScript", (b, l) =>
        {
            Require(b, l, "GetScript");
            if (l.Type(2) == LuaType.Number)
            {
                int index = b.CheckInteger(l, 2, "GetScript");
                b.PushObject(
                    l,
                    HostTypeId.ScriptObj,
                    (uint)index < (uint)runtime.Scripts.Count ? runtime.Scripts[index] : null);
                return 1;
            }

            string name = b.CheckString(l, 2, "GetScript");
            b.PushObject(l, HostTypeId.ScriptObj, runtime.FindScript(name));
            return 1;
        }, HostTypeId.ScriptObj);

        type.Function("ReadMem", (b, l) =>
        {
            Require(b, l, "ReadMem");
            ushort address = LuaNumberFormat.ToAddress(b.CheckNumber(l, 2, "ReadMem"));
            return Number(l, runtime.Environment.Memory.Read(address));
        });

        type.Function("WriteMem", (b, l) =>
        {
            Require(b, l, "WriteMem");
            ushort address = LuaNumberFormat.ToAddress(b.CheckNumber(l, 2, "WriteMem"));
            byte value = LuaNumberFormat.ToByte(b.CheckNumber(l, 3, "WriteMem"));
            runtime.Environment.Memory.Write(address, value);
            return 0;
        });

        HostType script = binder.DeclareType(HostTypeId.ScriptObj, "ScriptObj");
        script.Property(
            "Enable",
            static (b, l) => Boolean(l, Self(b, l)?.Enabled ?? false),
            static (b, l) =>
            {
                if (Self(b, l) is { } found)
                {
                    found.Enabled = AssignedBoolean(l);
                }
            });
        script.Alias("enable", "Enable");
        script.Property("Name", static (b, l) => Text(l, Self(b, l)?.Name ?? string.Empty));
        script.Alias("name", "Name");
        script.Property("Error", static (b, l) => Boolean(l, Self(b, l)?.Failed ?? false));
        script.Property("TagStr", static (b, l) => Text(l, Self(b, l)?.TagString ?? string.Empty));
        return;

        static LoadedScript? Self(LuaBinder binder, Lua state)
            => binder.ObjectAt(state, 1, HostTypeId.ScriptObj) as LoadedScript;

        static void Require(LuaBinder binder, Lua state, string member)
        {
            if (binder.TypeIdAt(state, 1) != HostTypeId.ScriptApi)
            {
                binder.InvalidArguments(state, member);
            }
        }
    }
}
