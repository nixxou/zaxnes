using KeraLua;
using Zax.Engine3D;

namespace Zax.Scripting;

/// <summary>
/// The types <c>Shape</c>, <c>Pattern</c> and <c>Liste&lt;T&gt;</c> of SPEC-05 §4.2 and §4.3.
/// </summary>
internal static partial class LuaApi
{
    private static void RegisterShape(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        HostType type = binder.DeclareType(HostTypeId.Shape, "Shape");

        Tiles(type, "tStart", static s => s.TileStart);
        Tiles(type, "tEnd", static s => s.TileEnd);
        Tiles(type, "tSize", static s => s.TileSize);
        type.Alias("TStart", "tStart");
        type.Alias("TEnd", "tEnd");
        type.Alias("TSize", "tSize");

        Tiles(type, "Start", static s => s.PixelStart);
        Tiles(type, "End", static s => s.PixelEnd);
        type.Alias("pStart", "Start");
        type.Alias("pEnd", "End");
        Tiles(type, "Size", static s => s.PixelSize);

        type.Property(
            "BottomLeft",
            (b, l) =>
            {
                b.PushVector2(l, SelfShape(b, l)?.BottomLeft ?? default);
                return 1;
            },
            result: HostTypeId.Vector2);
        type.Property(
            "TopRight",
            (b, l) =>
            {
                b.PushVector2(l, SelfShape(b, l)?.TopRight ?? default);
                return 1;
            },
            result: HostTypeId.Vector2);
        type.Alias("BL", "BottomLeft");
        type.Alias("TR", "TopRight");

        type.Property("Bg", (b, l) => Boolean(l, SelfShape(b, l)?.Background ?? false));
        type.Alias("bg", "Bg");
        type.Property("Hidden", (b, l) => Boolean(l, SelfShape(b, l)?.Hidden ?? false));
        type.Alias("hidden", "Hidden");
        type.Property("Palette", (b, l) => Number(l, SelfShape(b, l)?.Palette ?? 0));
        type.Alias("palIndex", "Palette");
        type.Property("TileCount", (b, l) => Number(l, SelfShape(b, l)?.TileCount ?? 0));
        type.Property("Inside", (b, l) => Boolean(l, SelfShape(b, l)?.Inside ?? false));
        type.Property("AtBorder", (b, l) => Boolean(l, SelfShape(b, l)?.AtBorder ?? true));
        type.Property("Completed", (b, l) => Boolean(l, SelfShape(b, l)?.Completed ?? false));
        type.Property("Age", (_, l) => Number(l, FrameShape.Age));
        type.Property("InsCount", (b, l) => Number(l, SelfShape(b, l)?.Instances.Count ?? 0));
        type.Alias("Count", "InsCount");

        type.Property(
            "shape3D",
            (b, l) =>
            {
                FrameShape? shape = SelfShape(b, l);
                b.PushObject(
                    l,
                    HostTypeId.Instance,
                    shape is { Instances.Count: > 0 } ? shape.Instances[0] : null);
                return 1;
            },
            result: HostTypeId.Instance);

        type.Property(
            "refPattern",
            (b, l) =>
            {
                b.PushObject(l, HostTypeId.Pattern, SelfShape(b, l)?.ReferencePattern);
                return 1;
            },
            result: HostTypeId.Pattern);

        // A shape's own pattern is the frame pattern of §4.2: the same object seen as a Pattern.
        type.Property(
            "pattern",
            (b, l) =>
            {
                b.PushObject(l, HostTypeId.Pattern, SelfShape(b, l), FramePatternMember);
                return 1;
            },
            result: HostTypeId.Pattern);

        // layer, geo and sizeZ are those of instance 0 of the frame pattern (§4.2).
        type.Property("layer", (b, l) => Number(l, (int)(FirstInstance(b, l)?.Layer ?? ZLayer.Unidentified)));
        type.Property("geo", (b, l) => Number(l, (int)(FirstInstance(b, l)?.Geometry ?? Geo3DType.Default)));
        type.Property("sizeZ", (b, l) => Number(l, FirstInstance(b, l)?.Depth ?? 0));

        type.Property(
            "center",
            (b, l) =>
            {
                b.PushVector3(l, SelfShape(b, l)?.Center ?? default);
                return 1;
            },
            result: HostTypeId.Vector3);

        type.ElementType = HostTypeId.Instance;
        type.Indexer = (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "Shape");
            FrameShape? shape = SelfShape(b, l);
            int count = shape?.Instances.Count ?? 0;
            b.CheckBounds(l, index, count, "Shape");
            b.PushObject(l, HostTypeId.Instance, shape!.Instances[index]);
            return 1;
        };

        type.Function("Shape3D", (b, l) =>
        {
            int index = l.IsNoneOrNil(2) ? 0 : b.CheckInteger(l, 2, "Shape3D");
            FrameShape? shape = SelfShape(b, l);
            int count = shape?.Instances.Count ?? 0;
            b.CheckBounds(l, index, count, "Shape3D");
            b.PushObject(l, HostTypeId.Instance, shape!.Instances[index]);
            return 1;
        }, HostTypeId.Instance);

        type.Function("CalculatePos", (b, l) =>
        {
            if (SelfShape(b, l) is { } shape)
            {
                foreach (FrameInstance instance in shape.Instances)
                {
                    instance.CalculatePos(runtime.Engine.Settings);
                }
            }

            return 0;
        });

        // Distance(s): the squared distance between the Pos of instance 0 of the two shapes.
        type.Function("Distance", (b, l) =>
        {
            FrameShape? mine = SelfShape(b, l);
            if (b.ObjectAt(l, 2, HostTypeId.Shape) is not FrameShape other)
            {
                return b.InvalidArguments(l, "Distance");
            }

            if (mine is not { Instances.Count: > 0 } || other.Instances.Count == 0)
            {
                return Number(l, 0.0);
            }

            Vec3 difference = mine.Instances[0].Pos - other.Instances[0].Pos;
            return Number(l, difference.SquaredLength);
        });

        return;

        static void Tiles(HostType host, string name, Func<FrameShape, Cell> read)
            => host.Property(
                name,
                (b, l) =>
                {
                    b.PushIntVector2(l, SelfShape(b, l) is { } shape ? read(shape) : default);
                    return 1;
                },
                result: HostTypeId.IntVector2);

        static FrameInstance? FirstInstance(LuaBinder binder, Lua state)
        {
            FrameShape? shape = SelfShape(binder, state);
            return shape is { Instances.Count: > 0 } ? shape.Instances[0] : null;
        }
    }

    private static void RegisterPattern(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        _ = runtime;
        HostType type = binder.DeclareType(HostTypeId.Pattern, "Pattern");

        type.Property(
            "Disable",
            static (b, l) => Boolean(l, PatternOf(b, l) is { } pattern && pattern.Disabled),
            static (b, l) =>
            {
                // A frame pattern lives for one frame and is never a candidate, so the flag
                // only means something on a registry pattern (REQ-E3D-093).
                if (PatternOf(b, l) is { } pattern)
                {
                    pattern.Disabled = AssignedBoolean(l);
                }
            });

        type.Property(
            "tSize",
            static (b, l) =>
            {
                b.PushIntVector2(l, Grid(b, l));
                return 1;
            },
            result: HostTypeId.IntVector2);

        type.Property(
            "Size",
            static (b, l) =>
            {
                b.PushVector2(l, PixelSize(b, l));
                return 1;
            },
            result: HostTypeId.Vector2);

        type.Property(
            "BottomLeft",
            static (b, l) =>
            {
                b.PushVector2(l, Corner(b, l, bottomLeft: true));
                return 1;
            },
            result: HostTypeId.Vector2);
        type.Property(
            "TopRight",
            static (b, l) =>
            {
                b.PushVector2(l, Corner(b, l, bottomLeft: false));
                return 1;
            },
            result: HostTypeId.Vector2);
        type.Alias("BL", "BottomLeft");
        type.Alias("TR", "TopRight");

        type.Property("count", static (b, l) => Number(l, Instances(b, l).Count));

        type.Property(
            "pattern3D",
            static (b, l) =>
            {
                IReadOnlyList<PatternInstance> instances = Instances(b, l);
                b.PushObject(l, HostTypeId.Instance, instances.Count > 0 ? instances[0] : null);
                return 1;
            },
            result: HostTypeId.Instance);

        type.Property(
            "pattern3DList",
            static (b, l) =>
            {
                object? owner = b.Resolve(b.HandleAt(l, 1));
                if (owner is null)
                {
                    l.PushNil();
                    return 1;
                }

                b.PushDescriptor(l, HostTypeId.InstanceList, owner, (int)ListKind.PatternInstances);
                return 1;
            },
            result: HostTypeId.InstanceList);

        type.Property(
            "constraintSize",
            static (b, l) =>
            {
                b.PushIntVector2(l, PatternOf(b, l)?.ConstraintSize ?? ShapeOf(b, l)?.ConstraintSize ?? default);
                return 1;
            },
            static (b, l) =>
            {
                if (!b.TryGetIntVector2(l, 3, out Cell value))
                {
                    b.InvalidArguments(l, "constraintSize");
                    return;
                }

                if (PatternOf(b, l) is { } pattern)
                {
                    pattern.ConstraintSize = value;
                }
                else if (ShapeOf(b, l) is { } shape)
                {
                    shape.ConstraintSize = value;
                }
            },
            HostTypeId.IntVector2);
        type.Alias("CSize", "constraintSize");

        type.Property(
            "SelfConnected",
            static (b, l) => Boolean(l, PatternOf(b, l)?.SelfConnected ?? ShapeOf(b, l)?.SelfConnected ?? true),
            static (b, l) =>
            {
                bool value = AssignedBoolean(l);
                if (PatternOf(b, l) is { } pattern)
                {
                    pattern.SetSelfConnected(value);
                }
                else if (ShapeOf(b, l) is { } shape)
                {
                    shape.SelfConnected = value;
                }
            });

        type.Property(
            "SkipGeo",
            static (b, l) => Boolean(l, PatternOf(b, l)?.SkipGeometry ?? ShapeOf(b, l)?.SkipGeometry ?? false),
            static (b, l) =>
            {
                bool value = AssignedBoolean(l);
                if (PatternOf(b, l) is { } pattern)
                {
                    pattern.SkipGeometry = value;
                }
                else if (ShapeOf(b, l) is { } shape)
                {
                    shape.SkipGeometry = value;
                }
            });

        type.Property("permanent", static (b, l) => Boolean(l, PatternOf(b, l)?.Permanent ?? false));

        type.ElementType = HostTypeId.Instance;
        type.Indexer = static (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "Pattern");
            IReadOnlyList<PatternInstance> instances = Instances(b, l);
            b.CheckBounds(l, index, instances.Count, "Pattern");
            b.PushObject(l, HostTypeId.Instance, instances[index]);
            return 1;
        };
    }

    /// <summary>The registry pattern a <c>Pattern</c> userdata names, or null for a frame pattern.</summary>
    private static EnginePattern? PatternOf(LuaBinder binder, Lua state)
        => binder.MemberAt(state, 1) == FramePatternMember
            ? null
            : binder.Resolve(binder.HandleAt(state, 1)) as EnginePattern;

    /// <summary>The shape a frame-pattern userdata names, or null.</summary>
    private static FrameShape? ShapeOf(LuaBinder binder, Lua state)
        => binder.MemberAt(state, 1) == FramePatternMember
            ? binder.Resolve(binder.HandleAt(state, 1)) as FrameShape
            : null;

    private static IReadOnlyList<PatternInstance> Instances(LuaBinder binder, Lua state)
    {
        if (PatternOf(binder, state) is { } pattern)
        {
            return pattern.Instances;
        }

        return ShapeOf(binder, state)?.Instances ?? (IReadOnlyList<PatternInstance>)[];
    }

    private static Cell Grid(LuaBinder binder, Lua state)
    {
        if (PatternOf(binder, state) is { } pattern)
        {
            return new Cell(pattern.Width, pattern.Height);
        }

        return ShapeOf(binder, state)?.TileSize ?? default;
    }

    private static Vec2 PixelSize(LuaBinder binder, Lua state)
    {
        if (PatternOf(binder, state) is { } pattern)
        {
            return new Vec2(
                pattern.PixelEnd.X - pattern.PixelStart.X + 1,
                pattern.PixelEnd.Y - pattern.PixelStart.Y + 1);
        }

        if (ShapeOf(binder, state) is { } shape)
        {
            Cell size = shape.PixelSize;
            return new Vec2(size.X, size.Y);
        }

        return default;
    }

    private static Vec2 Corner(LuaBinder binder, Lua state, bool bottomLeft)
    {
        if (PatternOf(binder, state) is { } pattern)
        {
            return bottomLeft ? pattern.BottomLeft : pattern.TopRight;
        }

        if (ShapeOf(binder, state) is { } shape)
        {
            return bottomLeft ? shape.BottomLeft : shape.TopRight;
        }

        return default;
    }

    private static void RegisterLists(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        HostType instances = binder.DeclareType(HostTypeId.InstanceList, "Liste<Instance>");
        instances.Property("Count", (b, l) => Number(l, InstanceListOf(b, l, runtime).Count));
        instances.ElementType = HostTypeId.Instance;
        instances.Indexer = (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "Liste<Instance>");
            IReadOnlyList<PatternInstance> list = InstanceListOf(b, l, runtime);
            b.CheckBounds(l, index, list.Count, "Liste<Instance>");
            b.PushObject(l, HostTypeId.Instance, list[index]);
            return 1;
        };
        instances.Function("Contains", (b, l) =>
        {
            object? wanted = b.ObjectAt(l, 2, HostTypeId.Instance);
            return Boolean(l, wanted is not null && IndexIn(InstanceListOf(b, l, runtime), wanted) >= 0);
        });
        instances.Function("IndexOf", (b, l) =>
        {
            object? wanted = b.ObjectAt(l, 2, HostTypeId.Instance);
            return Number(l, wanted is null ? -1 : IndexIn(InstanceListOf(b, l, runtime), wanted));
        });

        HostType shapes = binder.DeclareType(HostTypeId.ShapeList, "Liste<Shape>");
        shapes.Property("Count", (b, l) => Number(l, ShapeListOf(b, l, runtime).Count));
        shapes.ElementType = HostTypeId.Shape;
        shapes.Indexer = (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "Liste<Shape>");
            IReadOnlyList<FrameShape> list = ShapeListOf(b, l, runtime);
            b.CheckBounds(l, index, list.Count, "Liste<Shape>");
            b.PushObject(l, HostTypeId.Shape, list[index]);
            return 1;
        };
        shapes.Function("Contains", (b, l) =>
        {
            object? wanted = b.ObjectAt(l, 2, HostTypeId.Shape);
            return Boolean(l, wanted is not null && IndexIn(ShapeListOf(b, l, runtime), wanted) >= 0);
        });
        shapes.Function("IndexOf", (b, l) =>
        {
            object? wanted = b.ObjectAt(l, 2, HostTypeId.Shape);
            return Number(l, wanted is null ? -1 : IndexIn(ShapeListOf(b, l, runtime), wanted));
        });
        return;

        static int IndexIn<T>(IReadOnlyList<T> list, object wanted)
            where T : class
        {
            for (int i = 0; i < list.Count; i++)
            {
                if (ReferenceEquals(list[i], wanted))
                {
                    return i;
                }
            }

            return -1;
        }
    }

    private static IReadOnlyList<PatternInstance> InstanceListOf(
        LuaBinder binder,
        Lua state,
        ProfileScriptRuntime runtime)
        => (ListKind)binder.MemberAt(state, 1) switch
        {
            ListKind.ShapesWithTag => runtime.InstanceQuery,
            ListKind.PatternsWithTag => runtime.PatternQuery,
            ListKind.PatternInstances => binder.Resolve(binder.HandleAt(state, 1)) switch
            {
                EnginePattern pattern => pattern.Instances,
                FrameShape shape => shape.Instances,
                _ => [],
            },
            _ => [],
        };

    private static IReadOnlyList<FrameShape> ShapeListOf(
        LuaBinder binder,
        Lua state,
        ProfileScriptRuntime runtime)
    {
        _ = binder;
        _ = state;
        return runtime.PinnedQuery;
    }
}
