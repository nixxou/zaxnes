using KeraLua;
using Zax.Engine3D;
using Zax.Profiles;

namespace Zax.Scripting;

/// <summary>
/// The type <c>Instance</c> of SPEC-05 §4.2 — a pattern instance of the profile or a frame
/// instance — together with its custom data, its texture slot and the bound vectors and
/// matrices of §4.3.
/// </summary>
internal static partial class LuaApi
{
    private static void RegisterInstance(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        HostType type = binder.DeclareType(HostTypeId.Instance, "Instance");

        // Transforms: bound vectors, modifiable in place (REQ-LUA-044).
        BoundVector(type, "Offset", Vec3RefKind.InstanceOffset);
        type.Alias("offset", "Offset");
        BoundVector(type, "Scale", Vec3RefKind.InstanceScale);
        type.Alias("scale", "Scale");
        BoundVector(type, "Rot", Vec3RefKind.InstanceRotation);
        type.Alias("rot", "Rot");
        BoundVector(type, "Pivot", Vec3RefKind.InstancePivot);
        type.Alias("pivot", "Pivot");
        BoundVector(type, "DeformAmp", Vec3RefKind.InstanceDeformAmplitude);
        BoundVector(type, "DeformPivot", Vec3RefKind.InstanceDeformPivot);

        Descriptors(type, "DeformSpeed", HostTypeId.Mat4Ref, (int)Mat4RefKind.DeformSpeed);
        Descriptors(type, "DeformOffset", HostTypeId.Mat4Ref, (int)Mat4RefKind.DeformOffset);
        Descriptors(type, "texOffset", HostTypeId.Vec2Ref, (int)Vec2RefKind.TexOffset);
        Descriptors(type, "texTilting", HostTypeId.Vec2Ref, (int)Vec2RefKind.TexTilting);
        Descriptors(type, "CData", HostTypeId.CustomData, 0);

        type.Property(
            "Alpha",
            (b, l) => Number(l, Self(b, l)?.Alpha ?? 0f),
            (b, l) =>
            {
                if (Self(b, l) is { } instance)
                {
                    instance.Alpha = (float)AssignedNumber(b, l, "Alpha");
                }
            });
        type.Alias("alpha", "Alpha");

        type.Property(
            "Enable",
            (b, l) => Boolean(l, Self(b, l)?.Enable ?? false),
            (b, l) =>
            {
                if (Self(b, l) is { } instance)
                {
                    instance.Enable = AssignedBoolean(l);
                }
            });

        type.Property(
            "RenderStyle",
            (b, l) => Number(l, (int)(Self(b, l)?.Style ?? RenderStyle.Hidden)),
            (b, l) =>
            {
                if (Self(b, l) is { } instance)
                {
                    instance.Style = (RenderStyle)AssignedInteger(b, l, "RenderStyle");
                }
            });

        // Reading gives the custom palette when it is >= 0 and the shape's otherwise; writing
        // sets the custom palette (§4.2, REQ-E3D-192).
        type.Property(
            "Palette",
            (b, l) => Number(l, Self(b, l) switch
            {
                FrameInstance frame => frame.PaletteTable,
                { } instance => Math.Max(instance.CustomPalette, 0),
                _ => 0,
            }),
            (b, l) =>
            {
                if (Self(b, l) is { } instance)
                {
                    instance.CustomPalette = AssignedInteger(b, l, "Palette");
                }
            });

        type.Property(
            "customPalette",
            (b, l) => Number(l, Self(b, l)?.CustomPalette ?? -1),
            (b, l) =>
            {
                if (Self(b, l) is { } instance)
                {
                    instance.CustomPalette = AssignedInteger(b, l, "customPalette");
                }
            });

        type.Property(
            "Layer",
            (b, l) => Number(l, (int)(Self(b, l)?.Layer ?? ZLayer.Unidentified)),
            (b, l) =>
            {
                if (Self(b, l) is { } instance)
                {
                    instance.Layer = (ZLayer)AssignedInteger(b, l, "Layer");
                }
            });
        type.Alias("layer", "Layer");

        // Writing true moves the instance to the UI layer; writing false has no effect (§4.2).
        type.Property(
            "UI",
            (b, l) => Boolean(l, Self(b, l)?.Layer == ZLayer.Ui),
            (b, l) =>
            {
                if (Self(b, l) is { } instance && AssignedBoolean(l))
                {
                    instance.Layer = ZLayer.Ui;
                }
            });
        type.Alias("ui", "UI");

        Flag(type, "CastShadow", static i => i.CastShadow, static (i, v) => i.CastShadow = v);
        Flag(type, "RecvShadow", static i => i.ReceiveShadow, static (i, v) => i.ReceiveShadow = v);
        Flag(type, "antiShadow", static i => i.AntiShadow, static (i, v) => i.AntiShadow = v);
        Flag(type, "flip", static i => i.Flip, static (i, v) => i.Flip = v);
        Flag(type, "solid", static i => i.Solid, static (i, v) => i.Solid = v);
        Flag(type, "fullWarp", static i => i.FullWarp, static (i, v) => i.FullWarp = v);

        type.Property("CastingShadow", (b, l) => Boolean(l, EffectiveShadow(runtime, Self(b, l), cast: true)));
        type.Property("ReceivingShadow", (b, l) => Boolean(l, EffectiveShadow(runtime, Self(b, l), cast: false)));

        type.Property(
            "sizeZ",
            (b, l) => Number(l, Self(b, l)?.Depth ?? 0),
            (b, l) =>
            {
                if (Self(b, l) is { } instance)
                {
                    instance.Depth = AssignedInteger(b, l, "sizeZ");
                }
            });

        type.Property(
            "geo",
            (b, l) => Number(l, (int)(Self(b, l)?.Geometry ?? Geo3DType.Default)),
            (b, l) =>
            {
                if (Self(b, l) is { } instance)
                {
                    instance.Geometry = (Geo3DType)AssignedInteger(b, l, "geo");
                }
            });

        type.Property(
            "fillrate",
            (b, l) => Number(l, Self(b, l)?.FillRate ?? 0f),
            (b, l) =>
            {
                if (Self(b, l) is { } instance)
                {
                    instance.FillRate = (float)AssignedNumber(b, l, "fillrate");
                }
            });

        type.Property(
            "emptyColor",
            (b, l) => Number(l, Self(b, l)?.EmptyColor ?? 0),
            (b, l) =>
            {
                if (Self(b, l) is { } instance)
                {
                    instance.EmptyColor = AssignedInteger(b, l, "emptyColor");
                }
            });

        // The texture slot: shared between a pattern instance and its frame instances, and
        // assignable so that an instance borrows another's texture (REQ-LUA-097).
        type.Property(
            "slot",
            (b, l) =>
            {
                if (Self(b, l) is not { } instance)
                {
                    l.PushNil();
                    return 1;
                }

                b.PushObject(l, HostTypeId.Slot, instance.TextureOwner);
                return 1;
            },
            (b, l) =>
            {
                if (Self(b, l) is not { } instance)
                {
                    return;
                }

                if (b.ObjectAt(l, 3, HostTypeId.Slot) is PatternInstance source)
                {
                    instance.TextureSource = source;
                    return;
                }

                if (l.IsNil(3))
                {
                    instance.TextureSource = null;
                    return;
                }

                b.InvalidArguments(l, "slot");
            },
            HostTypeId.Slot);

        type.Property(
            "Shape2D",
            (b, l) =>
            {
                b.PushObject(l, HostTypeId.Shape, SelfFrameInstance(b, l)?.Shape);
                return 1;
            },
            result: HostTypeId.Shape);
        type.Alias("shape", "Shape2D");

        type.Property(
            "refPattern3D",
            (b, l) =>
            {
                PatternInstance? instance = Self(b, l);
                b.PushObject(l, HostTypeId.Instance, instance?.Reference ?? instance);
                return 1;
            },
            static (_, _) =>
            {
                // Listed as writable in §4.2, but the link from a frame instance to the pattern
                // instance it copies is the engine's (REQ-E3D-190) and no profile writes it; the
                // write is accepted and ignored rather than raising.
            },
            HostTypeId.Instance);

        type.Property(
            "refPattern",
            (b, l) =>
            {
                PushPattern(b, l, ReferencePatternOf(runtime, Self(b, l)));
                return 1;
            },
            result: HostTypeId.Pattern);

        // The carrying pattern: the registry pattern of a pattern instance, the frame pattern
        // of a frame instance (§4.2, type Pattern).
        type.Property(
            "pattern",
            (b, l) =>
            {
                if (SelfFrameInstance(b, l) is { } frame)
                {
                    b.PushObject(l, HostTypeId.Pattern, frame.Shape, FramePatternMember);
                    return 1;
                }

                PushPattern(b, l, ReferencePatternOf(runtime, Self(b, l)));
                return 1;
            },
            static (_, _) =>
            {
                // Listed as writable in §4.2, but rebinding an instance to another pattern has
                // no meaning in this model and no profile does it; the write is accepted and
                // ignored rather than raising (REQ-LUA-042 would make it an error otherwise).
            },
            HostTypeId.Pattern);

        type.Property("Index", (b, l) => Number(l, Self(b, l)?.Index ?? 0));
        type.Alias("index", "Index");

        type.Property("Sp", (b, l) => Boolean(l, SelfFrameInstance(b, l)?.Sprite ?? false));
        type.Property("Bg", (b, l) => Boolean(l, SelfFrameInstance(b, l)?.Background ?? false));

        Geometry(type, "BottomLeft", static i => i.BottomLeft);
        Geometry(type, "TopRight", static i => i.TopRight);
        Geometry(type, "BL", static i => i.BL);
        Geometry(type, "TR", static i => i.TR);
        Geometry(type, "size", static i => i.Size);
        type.Alias("Size", "size");
        Geometry(type, "OriPos", static i => i.OriPos);
        Geometry(type, "OriPosWoLayer", static i => i.GetPosWithoutLayer(i.DeformPivot));
        Geometry(type, "Pos", static i => i.Pos);

        type.Property(
            "Center",
            (b, l) =>
            {
                b.PushVector3(l, SelfFrameInstance(b, l)?.Center ?? default);
                return 1;
            },
            (b, l) =>
            {
                if (SelfFrameInstance(b, l) is { } frame)
                {
                    frame.Center = b.CheckVector3(l, 3, "Center");
                }
            },
            HostTypeId.Vector3);

        type.Property(
            "tSize",
            (b, l) =>
            {
                b.PushIntVector2(l, CarrierTileSize(runtime, Self(b, l)));
                return 1;
            },
            result: HostTypeId.IntVector2);

        type.Property(
            "constraintSize",
            (b, l) =>
            {
                b.PushIntVector2(l, CarrierConstraint(runtime, Self(b, l)));
                return 1;
            },
            (b, l) =>
            {
                if (!b.TryGetIntVector2(l, 3, out Cell value))
                {
                    b.InvalidArguments(l, "constraintSize");
                    return;
                }

                SetCarrierConstraint(runtime, Self(b, l), value);
            },
            HostTypeId.IntVector2);

        type.Property(
            "constraintWidth",
            (b, l) => Number(l, CarrierConstraint(runtime, Self(b, l)).X),
            (b, l) =>
            {
                PatternInstance? instance = Self(b, l);
                Cell current = CarrierConstraint(runtime, instance);
                SetCarrierConstraint(
                    runtime,
                    instance,
                    new Cell(AssignedInteger(b, l, "constraintWidth"), current.Y));
            });

        type.Property(
            "constraintHeight",
            (b, l) => Number(l, CarrierConstraint(runtime, Self(b, l)).Y),
            (b, l) =>
            {
                PatternInstance? instance = Self(b, l);
                Cell current = CarrierConstraint(runtime, instance);
                SetCarrierConstraint(
                    runtime,
                    instance,
                    new Cell(current.X, AssignedInteger(b, l, "constraintHeight")));
            });

        type.Property(
            "selfConnected",
            (b, l) => Boolean(l, SelfFrameInstance(b, l) is { } frame
                ? frame.Shape.SelfConnected
                : ReferencePatternOf(runtime, Self(b, l))?.SelfConnected ?? true),
            (b, l) =>
            {
                bool value = AssignedBoolean(l);
                if (SelfFrameInstance(b, l) is { } frame)
                {
                    frame.Shape.SelfConnected = value;
                }
                else if (ReferencePatternOf(runtime, Self(b, l)) is { } pattern)
                {
                    pattern.SetSelfConnected(value);
                }
            });

        type.Property(
            "skipGeo",
            (b, l) => Boolean(l, SelfFrameInstance(b, l) is { } frame
                ? frame.Shape.SkipGeometry
                : ReferencePatternOf(runtime, Self(b, l))?.SkipGeometry ?? false),
            (b, l) =>
            {
                bool value = AssignedBoolean(l);
                if (SelfFrameInstance(b, l) is { } frame)
                {
                    frame.Shape.SkipGeometry = value;
                }
                else if (ReferencePatternOf(runtime, Self(b, l)) is { } pattern)
                {
                    pattern.SkipGeometry = value;
                }
            });

        type.Property("TagStr", (b, l) => Text(l, Self(b, l)?.TagString ?? string.Empty));

        // The two tag-derived flags of SPEC-06. The tag string decides them, so a write is
        // accepted without effect; no built-in profile writes either (both P3).
        type.Property(
            "IsStatic",
            (b, l) => Boolean(l, !(Self(b, l)?.HasTag(ProfileTags.StaticTag) ?? false)),
            static (_, _) => { });
        type.Property(
            "LAxis",
            (b, l) => Boolean(l, !(Self(b, l)?.HasTag(ProfileTags.GroundAxisTag) ?? false)),
            static (_, _) => { });

        type.Property("AnimEnable", (b, l) => Boolean(l, Self(b, l)?.DeformEnabled ?? false));

        RegisterInstanceMethods(binder, runtime, type);
        return;

        static PatternInstance? Self(LuaBinder binder, Lua state) => SelfInstance(binder, state);

        static int Descriptor(LuaBinder binder, Lua state, HostTypeId id, int member)
        {
            if (SelfInstance(binder, state) is not { } instance)
            {
                state.PushNil();
                return 1;
            }

            binder.PushDescriptor(state, id, instance, member);
            return 1;
        }

        static void BoundVector(HostType host, string name, Vec3RefKind kind)
            => Descriptors(host, name, HostTypeId.Vec3Ref, (int)kind);

        static void Descriptors(HostType host, string name, HostTypeId id, int member)
            => host.Property(name, (b, l) => Descriptor(b, l, id, member), result: id);

        static void Flag(
            HostType host,
            string name,
            Func<PatternInstance, bool> read,
            Action<PatternInstance, bool> write)
            => host.Property(
                name,
                (b, l) => Boolean(l, SelfInstance(b, l) is { } i && read(i)),
                (b, l) =>
                {
                    if (SelfInstance(b, l) is { } i)
                    {
                        write(i, AssignedBoolean(l));
                    }
                });

        static void Geometry(HostType host, string name, Func<FrameInstance, Vec3> read)
            => host.Property(
                name,
                (b, l) =>
                {
                    b.PushVector3(l, SelfFrameInstance(b, l) is { } frame ? read(frame) : default);
                    return 1;
                },
                result: HostTypeId.Vector3);
    }

    /// <summary>The member selector that marks the frame pattern view of a shape (§4.2).</summary>
    internal const int FramePatternMember = 1;

    private static void RegisterInstanceMethods(LuaBinder binder, ProfileScriptRuntime runtime, HostType type)
    {
        _ = binder;

        type.Function("GetPos", (b, l) =>
        {
            if (SelfFrameInstance(b, l) is not { } frame)
            {
                b.PushVector3(l, default);
                return 1;
            }

            Vec3 pivot = l.IsNoneOrNil(2) ? frame.Pivot : b.CheckVector3(l, 2, "GetPos");
            b.PushVector3(l, frame.GetPos(runtime.Engine.Settings, pivot));
            return 1;
        }, HostTypeId.Vector3);

        type.Function("GetOriginalPos", (b, l) =>
        {
            Vec3 pivot = b.CheckVector3(l, 2, "GetOriginalPos");
            b.PushVector3(l, SelfFrameInstance(b, l)?.GetOriginalPos(pivot) ?? default);
            return 1;
        }, HostTypeId.Vector3);

        type.Function("GetPosWoLayer", (b, l) =>
        {
            Vec3 pivot = b.CheckVector3(l, 2, "GetPosWoLayer");
            b.PushVector3(l, SelfFrameInstance(b, l)?.GetPosWithoutLayer(pivot) ?? default);
            return 1;
        }, HostTypeId.Vector3);

        type.Function("CalculatePos", (b, l) =>
        {
            SelfFrameInstance(b, l)?.CalculatePos(runtime.Engine.Settings);
            return 0;
        });

        type.Function("ContainsTag", (b, l) =>
        {
            string tag = b.CheckString(l, 2, "ContainsTag");
            return Boolean(l, SelfInstance(b, l)?.ContainsTag(tag) ?? false);
        });

        type.Function("Overlap", (b, l) =>
        {
            if (SelfFrameInstance(b, l) is not { } frame)
            {
                return Boolean(l, false);
            }

            if (b.ObjectAt(l, 2, HostTypeId.Instance) is FrameInstance other)
            {
                int delta = l.IsNoneOrNil(3) ? 0 : b.CheckInteger(l, 3, "Overlap");
                return Boolean(l, frame.Overlap(other, delta));
            }

            if (b.TryGetRect(l, 2, out Rect rectangle))
            {
                return Boolean(l, frame.Overlap(rectangle));
            }

            return b.InvalidArguments(l, "Overlap");
        });

        type.Function("OverlapRect", (b, l) =>
        {
            Rect rectangle = b.CheckRect(l, 2, "OverlapRect");
            return Boolean(l, SelfFrameInstance(b, l)?.OverlapRect(rectangle) ?? false);
        });

        type.Function("GetDeform", (b, l) =>
        {
            Vec3 point = b.CheckVector3(l, 2, "GetDeform");
            b.PushVector3(
                l,
                SelfFrameInstance(b, l) is { } frame
                    ? Deformation.Evaluate(frame, point, frame.OriPos, FrameTime(l))
                    : default);
            return 1;
        }, HostTypeId.Vector3);

        type.Function("GetDeformation", (b, l) =>
        {
            Vec3 point = b.CheckVector3(l, 2, "GetDeformation");
            b.PushVector3(
                l,
                SelfFrameInstance(b, l) is { } frame
                    ? Deformation.Evaluate(
                        frame,
                        point,
                        frame.GetPosWithoutLayer(frame.DeformPivot),
                        FrameTime(l))
                    : default);
            return 1;
        }, HostTypeId.Vector3);

        _ = runtime;
    }

    /// <summary>
    /// The <c>t</c> of the deformation formula: the global <c>time</c> as the scripts see it
    /// (REQ-LUA-083 point 3), which a script may have reassigned (REQ-LUA-026).
    /// </summary>
    private static float FrameTime(Lua state)
    {
        state.GetGlobal("time");
        float time = state.Type(-1) == LuaType.Number ? (float)state.ToNumber(-1) : 0f;
        state.Pop(1);
        return time;
    }

    private static void RegisterReferences(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        _ = runtime;

        HostType vec3 = binder.DeclareType(HostTypeId.Vec3Ref, "Vec3R");
        Component(vec3, "x", 0);
        Component(vec3, "y", 1);
        Component(vec3, "z", 2);
        vec3.Property(
            "content",
            (b, l) =>
            {
                b.PushVector3(l, b.ReadVec3Ref(l, 1));
                return 1;
            },
            (b, l) => b.WriteVec3Ref(l, 1, b.CheckVector3(l, 3, "content")),
            HostTypeId.Vector3);
        vec3.Indexer = (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "Vec3R");
            Vec3 value = b.ReadVec3Ref(l, 1);
            return Number(l, index switch
            {
                0 => value.X,
                1 => value.Y,
                2 => value.Z,
                _ => b.Raise(l, $"index {index} is out of range for Vec3R"),
            });
        };
        vec3.IndexSetter = (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "Vec3R");
            float component = (float)AssignedNumber(b, l, "Vec3R");
            Vec3 value = b.ReadVec3Ref(l, 1);
            switch (index)
            {
                case 0: value.X = component; break;
                case 1: value.Y = component; break;
                case 2: value.Z = component; break;
                default:
                    b.Raise(l, $"index {index} is out of range for Vec3R");
                    return;
            }

            b.WriteVec3Ref(l, 1, value);
        };
        vec3.Text = static (b, l) =>
        {
            Vec3 value = b.ReadVec3Ref(l, 1);
            return Text(l, $"({LuaNumberFormat.ToText(value.X)}, {LuaNumberFormat.ToText(value.Y)}, "
                + $"{LuaNumberFormat.ToText(value.Z)})");
        };

        HostType vec2 = binder.DeclareType(HostTypeId.Vec2Ref, "Vec2R");
        vec2.Property(
            "x",
            static (b, l) => Number(l, ReadVec2Ref(b, l).X),
            static (b, l) =>
            {
                Vec2 value = ReadVec2Ref(b, l);
                value.X = (float)AssignedNumber(b, l, "x");
                WriteVec2Ref(b, l, value);
            });
        vec2.Property(
            "y",
            static (b, l) => Number(l, ReadVec2Ref(b, l).Y),
            static (b, l) =>
            {
                Vec2 value = ReadVec2Ref(b, l);
                value.Y = (float)AssignedNumber(b, l, "y");
                WriteVec2Ref(b, l, value);
            });
        vec2.Property(
            "content",
            static (b, l) =>
            {
                b.PushVector2(l, ReadVec2Ref(b, l));
                return 1;
            },
            static (b, l) =>
            {
                if (!b.TryGetVector2(l, 3, out Vec2 value))
                {
                    b.InvalidArguments(l, "content");
                    return;
                }

                WriteVec2Ref(b, l, value);
            },
            HostTypeId.Vector2);
        vec2.Indexer = static (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "Vec2R");
            Vec2 value = ReadVec2Ref(b, l);
            return Number(l, index switch
            {
                0 => value.X,
                1 => value.Y,
                _ => b.Raise(l, $"index {index} is out of range for Vec2R"),
            });
        };
        vec2.IndexSetter = static (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "Vec2R");
            float component = (float)AssignedNumber(b, l, "Vec2R");
            Vec2 value = ReadVec2Ref(b, l);
            switch (index)
            {
                case 0: value.X = component; break;
                case 1: value.Y = component; break;
                default:
                    b.Raise(l, $"index {index} is out of range for Vec2R");
                    return;
            }

            WriteVec2Ref(b, l, value);
        };

        // Mat4R: mRC is row R, column C; the numeric index is column-major, i = R + 4·C (§4.3).
        HostType matrix = binder.DeclareType(HostTypeId.Mat4Ref, "Mat4R");
        for (int row = 0; row < 4; row++)
        {
            for (int column = 0; column < 4; column++)
            {
                int r = row;
                int c = column;
                matrix.Property(
                    $"m{row}{column}",
                    (b, l) => Number(l, ReadMatrix(b, l)[r, c]),
                    (b, l) =>
                    {
                        Mat4 value = ReadMatrix(b, l);
                        value[r, c] = (float)AssignedNumber(b, l, $"m{r}{c}");
                        WriteMatrix(b, l, value);
                    });
            }
        }

        matrix.Indexer = static (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "Mat4R");
            if ((uint)index >= 16u)
            {
                return b.Raise(l, $"index {index} is out of range for Mat4R");
            }

            return Number(l, ReadMatrix(b, l)[index & 3, index >> 2]);
        };
        matrix.IndexSetter = static (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "Mat4R");
            if ((uint)index >= 16u)
            {
                b.Raise(l, $"index {index} is out of range for Mat4R");
                return;
            }

            Mat4 value = ReadMatrix(b, l);
            value[index & 3, index >> 2] = (float)AssignedNumber(b, l, "Mat4R");
            WriteMatrix(b, l, value);
        };

        // CData: indices 0 to 3 are row 3 of DeformSpeed, 4 and up row 3 of DeformOffset
        // (REQ-LUA-082). A negative index raises.
        HostType custom = binder.DeclareType(HostTypeId.CustomData, "CData");
        custom.Indexer = static (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "CData");
            if (index < 0)
            {
                return b.Raise(l, $"index {index} is out of range for CData");
            }

            return Number(l, b.Resolve(b.HandleAt(l, 1)) is PatternInstance instance
                ? instance.CustomData(index)
                : 0f);
        };
        custom.IndexSetter = static (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "CData");
            if (index < 0)
            {
                b.Raise(l, $"index {index} is out of range for CData");
                return;
            }

            if (b.Resolve(b.HandleAt(l, 1)) is PatternInstance instance)
            {
                instance.SetCustomData(index, (float)AssignedNumber(b, l, "CData"));
            }
        };

        // The texture slot of REQ-LUA-097: an opaque object with the texture size in pixels.
        HostType slot = binder.DeclareType(HostTypeId.Slot, "SlotTexture");
        slot.Property("w", (b, l) => Number(l, SlotSize(b, l, runtime).X));
        slot.Property("h", (b, l) => Number(l, SlotSize(b, l, runtime).Y));
        return;

        static void Component(HostType host, string name, int axis)
            => host.Property(
                name,
                (b, l) =>
                {
                    Vec3 value = b.ReadVec3Ref(l, 1);
                    return Number(l, axis switch { 0 => value.X, 1 => value.Y, _ => value.Z });
                },
                (b, l) =>
                {
                    Vec3 value = b.ReadVec3Ref(l, 1);
                    float component = (float)AssignedNumber(b, l, name);
                    switch (axis)
                    {
                        case 0: value.X = component; break;
                        case 1: value.Y = component; break;
                        default: value.Z = component; break;
                    }

                    b.WriteVec3Ref(l, 1, value);
                });
    }

    private static Vec2 ReadVec2Ref(LuaBinder binder, Lua state)
    {
        (object? owner, int kind) = binder.RefTargetAt(state, 1);
        if (owner is not PatternInstance instance)
        {
            return default;
        }

        return (Vec2RefKind)kind == Vec2RefKind.TexOffset ? instance.TexOffset : instance.TexTilting;
    }

    private static void WriteVec2Ref(LuaBinder binder, Lua state, Vec2 value)
    {
        (object? owner, int kind) = binder.RefTargetAt(state, 1);
        if (owner is not PatternInstance instance)
        {
            return;
        }

        if ((Vec2RefKind)kind == Vec2RefKind.TexOffset)
        {
            instance.TexOffset = value;
        }
        else
        {
            instance.TexTilting = value;
        }
    }

    private static Mat4 ReadMatrix(LuaBinder binder, Lua state)
    {
        (object? owner, int kind) = binder.RefTargetAt(state, 1);
        if (owner is not PatternInstance instance)
        {
            return default;
        }

        return (Mat4RefKind)kind == Mat4RefKind.DeformSpeed ? instance.DeformSpeed : instance.DeformOffset;
    }

    private static void WriteMatrix(LuaBinder binder, Lua state, Mat4 value)
    {
        (object? owner, int kind) = binder.RefTargetAt(state, 1);
        if (owner is not PatternInstance instance)
        {
            return;
        }

        if ((Mat4RefKind)kind == Mat4RefKind.DeformSpeed)
        {
            instance.DeformSpeed = value;
        }
        else
        {
            instance.DeformOffset = value;
        }
    }

    private static Cell SlotSize(LuaBinder binder, Lua state, ProfileScriptRuntime runtime)
    {
        if (binder.Resolve(binder.HandleAt(state, 1)) is not PatternInstance instance)
        {
            return default;
        }

        if (runtime.Engine.Geometry.TextureOf(instance) is { } texture)
        {
            return new Cell(texture.Width, texture.Height);
        }

        // Before the geometry is built the slot is the size the default texture will have:
        // 8W × 8H texels of the carrying pattern (SPEC-04B REQ-E3D-426).
        EnginePattern? pattern = runtime.Engine.FindPattern(instance);
        return pattern is null ? default : new Cell(8 * pattern.Width, 8 * pattern.Height);
    }

    private static EnginePattern? ReferencePatternOf(ProfileScriptRuntime runtime, PatternInstance? instance)
    {
        if (instance is FrameInstance frame)
        {
            return frame.Shape.ReferencePattern;
        }

        return instance is null ? null : runtime.Engine.FindPattern(instance);
    }

    private static Cell CarrierTileSize(ProfileScriptRuntime runtime, PatternInstance? instance)
    {
        if (instance is FrameInstance frame)
        {
            return frame.Shape.TileSize;
        }

        EnginePattern? pattern = ReferencePatternOf(runtime, instance);
        return pattern is null ? default : new Cell(pattern.Width, pattern.Height);
    }

    private static Cell CarrierConstraint(ProfileScriptRuntime runtime, PatternInstance? instance)
    {
        if (instance is FrameInstance frame)
        {
            return frame.Shape.ConstraintSize;
        }

        return ReferencePatternOf(runtime, instance)?.ConstraintSize ?? new Cell(99, 99);
    }

    private static void SetCarrierConstraint(
        ProfileScriptRuntime runtime,
        PatternInstance? instance,
        Cell value)
    {
        if (instance is FrameInstance frame)
        {
            frame.Shape.ConstraintSize = value;
            return;
        }

        if (ReferencePatternOf(runtime, instance) is { } pattern)
        {
            pattern.ConstraintSize = value;
        }
    }

    private static bool EffectiveShadow(
        ProfileScriptRuntime runtime,
        PatternInstance? instance,
        bool cast)
    {
        if (instance is not FrameInstance frame)
        {
            return false;
        }

        ConversionSettings settings = runtime.Engine.Settings;
        if (!settings.Shadow || frame.Layer == ZLayer.Ui)
        {
            return false;
        }

        LayerSetting layer = settings.Layer(frame.EffectiveLayer(settings));
        return cast
            ? frame.CastShadow && layer.CastShadow
            : frame.ReceiveShadow && layer.ReceiveShadow;
    }

    private static void PushPattern(LuaBinder binder, Lua state, EnginePattern? pattern)
        => binder.PushObject(state, HostTypeId.Pattern, pattern);
}

/// <summary>
/// The deformation a script predicts through <c>GetDeform</c> and <c>GetDeformation</c>
/// (SPEC-05 REQ-LUA-083 point 3).
/// </summary>
/// <remarks>
/// The anchor the formula uses is deliberately not the pivot the rendered deformation applies
/// (SPEC-04B REQ-E3D-568): the difference, at most one pixel, is the original's and is kept.
/// </remarks>
internal static class Deformation
{
    /// <summary>
    /// <c>d_a = sin(r.x·V[k,0] + W[k,0]) · sin(r.y·V[k,1] + W[k,1]) · sin(r.z·V[k,2] + W[k,2])
    /// · sin(t·V[k,3] + W[k,3])</c>, the result scaled by <c>DeformAmp ⊙ size</c>.
    /// </summary>
    public static Vec3 Evaluate(FrameInstance instance, Vec3 point, Vec3 anchor, float time)
    {
        Vec3 size = instance.Size;
        Vec3 ratio = new(
            size.X == 0f ? 0f : (point.X - anchor.X) / size.X,
            size.Y == 0f ? 0f : (point.Y - anchor.Y) / size.Y,
            size.Z == 0f ? 0f : (point.Z - anchor.Z) / size.Z);

        Mat4 speed = instance.DeformSpeed;
        Mat4 phase = instance.DeformOffset;
        Vec3 amplitude = instance.DeformAmplitude;
        Span<float> axes = stackalloc float[3];
        for (int k = 0; k < 3; k++)
        {
            axes[k] = MathF.Sin((ratio.X * speed[k, 0]) + phase[k, 0])
                * MathF.Sin((ratio.Y * speed[k, 1]) + phase[k, 1])
                * MathF.Sin((ratio.Z * speed[k, 2]) + phase[k, 2])
                * MathF.Sin((time * speed[k, 3]) + phase[k, 3]);
        }

        return new Vec3(
            axes[0] * amplitude.X * size.X,
            axes[1] * amplitude.Y * size.Y,
            axes[2] * amplitude.Z * size.Z);
    }
}
