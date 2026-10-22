using System.Numerics;
using KeraLua;
using Zax.Engine3D;

namespace Zax.Scripting;

/// <summary>
/// The value types of SPEC-05 §4.3 — <c>Vector3</c>, <c>Vector2</c>, <c>Quaternion</c>,
/// <c>Color</c>, <c>IntVector2</c>, <c>IntVector3</c>, <c>Rect</c> — and their class tables.
/// </summary>
/// <remarks>
/// They are copies: a value read from an object and then changed leaves the object alone
/// (REQ-LUA-044). The operators and the equality tolerances are the ones §4.3 fixes
/// (REQ-LUA-096, REQ-LUA-102).
/// </remarks>
internal static partial class LuaApi
{
    private const float VectorEqualityTolerance = 1e-10f;
    private const float QuaternionEqualityTolerance = 1e-6f;
    private const float Degrees = 180f / MathF.PI;
    private const float Radians = MathF.PI / 180f;

    private static void RegisterValues(LuaBinder binder)
    {
        RegisterVector3(binder);
        RegisterVector2(binder);
        RegisterQuaternion(binder);
        RegisterColor(binder);
        RegisterIntVectors(binder);
        RegisterRect(binder);
    }

    private static void RegisterVector3(LuaBinder binder)
    {
        HostType type = binder.DeclareType(HostTypeId.Vector3, "Vector3", reference: false);
        Component(type, "x", 0);
        Component(type, "y", 1);
        Component(type, "z", 2);
        type.Property("magnitude", static (b, l) => Number(l, MathF.Sqrt(Read(b, l).SquaredLength)));
        type.Property("sqrMagnitude", static (b, l) => Number(l, Read(b, l).SquaredLength));
        type.Property("normalized", static (b, l) =>
        {
            Vec3 value = Read(b, l);
            float length = MathF.Sqrt(value.SquaredLength);
            b.PushVector3(l, length <= 1e-5f ? default : value * (1f / length));
            return 1;
        }, result: HostTypeId.Vector3);
        type.Equal = static (b, l) =>
        {
            Vec3 difference = Read(b, l) - ReadAt(b, l, 2);
            return difference.SquaredLength < VectorEqualityTolerance;
        };
        type.Add = static (b, l) =>
        {
            b.PushVector3(l, Operand(b, l, 1, "+") + Operand(b, l, 2, "+"));
            return 1;
        };
        type.Subtract = static (b, l) =>
        {
            b.PushVector3(l, Operand(b, l, 1, "-") - Operand(b, l, 2, "-"));
            return 1;
        };
        type.Negate = static (b, l) =>
        {
            Vec3 value = Read(b, l);
            b.PushVector3(l, new Vec3(-value.X, -value.Y, -value.Z));
            return 1;
        };
        type.Multiply = static (b, l) =>
        {
            // k * v and v * k; the component-by-component product is not part of §4.3.
            if (l.Type(1) == LuaType.Number)
            {
                b.PushVector3(l, ReadAt(b, l, 2) * (float)l.ToNumber(1));
                return 1;
            }

            if (l.Type(2) == LuaType.Number)
            {
                b.PushVector3(l, Read(b, l) * (float)l.ToNumber(2));
                return 1;
            }

            return b.InvalidArguments(l, "Vector3 *");
        };
        type.Divide = static (b, l) =>
        {
            if (l.Type(2) != LuaType.Number)
            {
                return b.InvalidArguments(l, "Vector3 /");
            }

            b.PushVector3(l, Read(b, l) * (1f / (float)l.ToNumber(2)));
            return 1;
        };
        type.Text = static (b, l) =>
        {
            Vec3 value = Read(b, l);
            return Text(l, $"({LuaNumberFormat.ToText(value.X)}, {LuaNumberFormat.ToText(value.Y)}, "
                + $"{LuaNumberFormat.ToText(value.Z)})");
        };

        HostType table = binder.DeclareType(HostTypeId.Vector3Class, "Vector3Classe");
        Constant(table, "zero", default);
        Constant(table, "one", Vec3.One);
        Constant(table, "back", new Vec3(0f, 0f, -1f));
        Constant(table, "forward", new Vec3(0f, 0f, 1f));
        Constant(table, "up", new Vec3(0f, 1f, 0f));
        Constant(table, "down", new Vec3(0f, -1f, 0f));
        Constant(table, "left", new Vec3(-1f, 0f, 0f));
        Constant(table, "right", new Vec3(1f, 0f, 0f));
        table.Function("Cross", static (b, l) =>
        {
            Vec3 a = b.CheckVector3(l, 1, "Cross");
            Vec3 c = b.CheckVector3(l, 2, "Cross");
            b.PushVector3(l, new Vec3(
                (a.Y * c.Z) - (a.Z * c.Y),
                (a.Z * c.X) - (a.X * c.Z),
                (a.X * c.Y) - (a.Y * c.X)));
            return 1;
        }, HostTypeId.Vector3);
        table.Function("Dot", static (b, l) =>
        {
            Vec3 a = b.CheckVector3(l, 1, "Dot");
            Vec3 c = b.CheckVector3(l, 2, "Dot");
            return Number(l, (a.X * c.X) + (a.Y * c.Y) + (a.Z * c.Z));
        });
        table.Function("Distance", static (b, l) =>
        {
            Vec3 a = b.CheckVector3(l, 1, "Distance");
            Vec3 c = b.CheckVector3(l, 2, "Distance");
            return Number(l, Vec3.Distance(a, c));
        });
        table.Function("Lerp", static (b, l) =>
        {
            Vec3 a = b.CheckVector3(l, 1, "Lerp");
            Vec3 c = b.CheckVector3(l, 2, "Lerp");
            float t = Math.Clamp((float)b.CheckNumber(l, 3, "Lerp"), 0f, 1f);
            b.PushVector3(l, a + ((c - a) * t));
            return 1;
        }, HostTypeId.Vector3);

        // Vector3(x, y [, z]); no argument gives the zero vector (§4.3).
        table.Call = static (b, l) =>
        {
            if (l.IsNoneOrNil(2))
            {
                b.PushVector3(l, default);
                return 1;
            }

            float x = (float)b.CheckNumber(l, 2, "Vector3");
            float y = (float)b.CheckNumber(l, 3, "Vector3");
            float z = (float)b.OptionalNumber(l, 4, "Vector3", 0.0);
            b.PushVector3(l, new Vec3(x, y, z));
            return 1;
        };
        table.CallResult = HostTypeId.Vector3;
        return;

        static Vec3 Read(LuaBinder binder, Lua state) => ReadAt(binder, state, 1);

        static Vec3 ReadAt(LuaBinder binder, Lua state, int index)
        {
            binder.TryGetVector3(state, index, out Vec3 value);
            return value;
        }

        static Vec3 Operand(LuaBinder binder, Lua state, int index, string what)
        {
            if (!binder.TryGetVector3(state, index, out Vec3 value))
            {
                binder.InvalidArguments(state, $"Vector3 {what}");
            }

            return value;
        }

        static void Component(HostType host, string name, int axis)
            => host.Property(
                name,
                (b, l) =>
                {
                    Vec3 value = ReadAt(b, l, 1);
                    return Number(l, axis switch { 0 => value.X, 1 => value.Y, _ => value.Z });
                },
                (b, l) => WriteComponent(b, l, axis, name));

        static void Constant(HostType host, string name, Vec3 value)
            => host.Property(
                name,
                (b, l) =>
                {
                    b.PushVector3(l, value);
                    return 1;
                },
                result: HostTypeId.Vector3);
    }

    /// <summary>
    /// Writes one component of a value userdata in place. The value is a copy, so this changes
    /// only the Lua value the script holds (REQ-LUA-044).
    /// </summary>
    private static unsafe void WriteComponent(LuaBinder binder, Lua state, int axis, string name)
    {
        float component = (float)AssignedNumber(binder, state, name);
        float* box = (float*)state.ToUserData(1);
        box[axis + 1] = component;
    }

    private static unsafe void WriteIntComponent(LuaBinder binder, Lua state, int axis, string name)
    {
        int component = AssignedInteger(binder, state, name);
        int* box = (int*)state.ToUserData(1);
        box[axis + 1] = component;
    }

    private static void RegisterVector2(LuaBinder binder)
    {
        HostType type = binder.DeclareType(HostTypeId.Vector2, "Vector2", reference: false);
        type.Property(
            "x",
            static (b, l) => Number(l, Read(b, l).X),
            static (b, l) => WriteComponent(b, l, 0, "x"));
        type.Property(
            "y",
            static (b, l) => Number(l, Read(b, l).Y),
            static (b, l) => WriteComponent(b, l, 1, "y"));
        type.Property("magnitude", static (b, l) =>
        {
            Vec2 value = Read(b, l);
            return Number(l, MathF.Sqrt((value.X * value.X) + (value.Y * value.Y)));
        });
        type.Equal = static (b, l) =>
        {
            Vec2 a = Read(b, l);
            Vec2 c = ReadAt(b, l, 2);
            float dx = a.X - c.X;
            float dy = a.Y - c.Y;
            return (dx * dx) + (dy * dy) < VectorEqualityTolerance;
        };
        type.Add = static (b, l) =>
        {
            Vec2 a = ReadAt(b, l, 1);
            Vec2 c = ReadAt(b, l, 2);
            b.PushVector2(l, new Vec2(a.X + c.X, a.Y + c.Y));
            return 1;
        };
        type.Subtract = static (b, l) =>
        {
            Vec2 a = ReadAt(b, l, 1);
            Vec2 c = ReadAt(b, l, 2);
            b.PushVector2(l, new Vec2(a.X - c.X, a.Y - c.Y));
            return 1;
        };
        type.Negate = static (b, l) =>
        {
            Vec2 value = Read(b, l);
            b.PushVector2(l, new Vec2(-value.X, -value.Y));
            return 1;
        };
        type.Multiply = static (b, l) =>
        {
            if (l.Type(1) == LuaType.Number)
            {
                Vec2 value = ReadAt(b, l, 2);
                float k = (float)l.ToNumber(1);
                b.PushVector2(l, new Vec2(value.X * k, value.Y * k));
                return 1;
            }

            if (l.Type(2) == LuaType.Number)
            {
                Vec2 value = Read(b, l);
                float k = (float)l.ToNumber(2);
                b.PushVector2(l, new Vec2(value.X * k, value.Y * k));
                return 1;
            }

            return b.InvalidArguments(l, "Vector2 *");
        };
        type.Divide = static (b, l) =>
        {
            if (l.Type(2) != LuaType.Number)
            {
                return b.InvalidArguments(l, "Vector2 /");
            }

            Vec2 value = Read(b, l);
            float k = (float)l.ToNumber(2);
            b.PushVector2(l, new Vec2(value.X / k, value.Y / k));
            return 1;
        };
        type.Text = static (b, l) =>
        {
            Vec2 value = Read(b, l);
            return Text(l, $"({LuaNumberFormat.ToText(value.X)}, {LuaNumberFormat.ToText(value.Y)})");
        };

        HostType table = binder.DeclareType(HostTypeId.Vector2Class, "Vector2Classe");
        table.Property(
            "zero",
            static (b, l) =>
            {
                b.PushVector2(l, default);
                return 1;
            },
            result: HostTypeId.Vector2);
        table.Property(
            "one",
            static (b, l) =>
            {
                b.PushVector2(l, Vec2.One);
                return 1;
            },
            result: HostTypeId.Vector2);
        table.Call = static (b, l) =>
        {
            if (l.IsNoneOrNil(2))
            {
                b.PushVector2(l, default);
                return 1;
            }

            float x = (float)b.CheckNumber(l, 2, "Vector2");
            float y = (float)b.CheckNumber(l, 3, "Vector2");
            b.PushVector2(l, new Vec2(x, y));
            return 1;
        };
        table.CallResult = HostTypeId.Vector2;
        return;

        static Vec2 Read(LuaBinder binder, Lua state) => ReadAt(binder, state, 1);

        static Vec2 ReadAt(LuaBinder binder, Lua state, int index)
        {
            binder.TryGetVector2(state, index, out Vec2 value);
            return value;
        }
    }

    private static void RegisterQuaternion(LuaBinder binder)
    {
        HostType type = binder.DeclareType(HostTypeId.Quaternion, "Quaternion", reference: false);
        type.Property(
            "x",
            static (b, l) => Number(l, Read(b, l).X),
            static (b, l) => WriteComponent(b, l, 0, "x"));
        type.Property(
            "y",
            static (b, l) => Number(l, Read(b, l).Y),
            static (b, l) => WriteComponent(b, l, 1, "y"));
        type.Property(
            "z",
            static (b, l) => Number(l, Read(b, l).Z),
            static (b, l) => WriteComponent(b, l, 2, "z"));
        type.Property(
            "w",
            static (b, l) => Number(l, Read(b, l).W),
            static (b, l) => WriteComponent(b, l, 3, "w"));
        type.Property(
            "eulerAngles",
            static (b, l) =>
            {
                b.PushVector3(l, ToEuler(Read(b, l)));
                return 1;
            },
            result: HostTypeId.Vector3);
        type.Equal = static (b, l) =>
        {
            Quaternion a = Read(b, l);
            Quaternion c = ReadAt(b, l, 2);
            float dot = (a.X * c.X) + (a.Y * c.Y) + (a.Z * c.Z) + (a.W * c.W);
            return MathF.Abs(dot) > 1f - QuaternionEqualityTolerance;
        };
        type.Multiply = static (b, l) =>
        {
            Quaternion left = ReadAt(b, l, 1);
            if (b.TryGetQuaternion(l, 2, out Quaternion right))
            {
                // q1 * q2 applies q2 first, then q1 (§4.3).
                b.PushQuaternion(l, left * right);
                return 1;
            }

            if (b.TryGetVector3(l, 2, out Vec3 vector))
            {
                Vector3 rotated = Vector3.Transform(new Vector3(vector.X, vector.Y, vector.Z), left);
                b.PushVector3(l, new Vec3(rotated.X, rotated.Y, rotated.Z));
                return 1;
            }

            return b.InvalidArguments(l, "Quaternion *");
        };
        type.Text = static (b, l) =>
        {
            Quaternion value = Read(b, l);
            return Text(l, $"({LuaNumberFormat.ToText(value.X)}, {LuaNumberFormat.ToText(value.Y)}, "
                + $"{LuaNumberFormat.ToText(value.Z)}, {LuaNumberFormat.ToText(value.W)})");
        };

        HostType table = binder.DeclareType(HostTypeId.QuaternionClass, "QuaternionClasse");
        table.Property(
            "identity",
            static (b, l) =>
            {
                b.PushQuaternion(l, Quaternion.Identity);
                return 1;
            },
            result: HostTypeId.Quaternion);
        table.Function("Euler", static (b, l) =>
        {
            if (b.TryGetVector3(l, 1, out Vec3 angles))
            {
                b.PushQuaternion(l, FromEuler(angles.X, angles.Y, angles.Z));
                return 1;
            }

            float x = (float)b.CheckNumber(l, 1, "Euler");
            float y = (float)b.CheckNumber(l, 2, "Euler");
            float z = (float)b.CheckNumber(l, 3, "Euler");
            b.PushQuaternion(l, FromEuler(x, y, z));
            return 1;
        }, HostTypeId.Quaternion);
        table.Function("AngleAxis", static (b, l) =>
        {
            float angle = (float)b.CheckNumber(l, 1, "AngleAxis");
            Vec3 axis = b.CheckVector3(l, 2, "AngleAxis");
            float length = MathF.Sqrt(axis.SquaredLength);
            if (length <= 1e-6f)
            {
                b.PushQuaternion(l, Quaternion.Identity);
                return 1;
            }

            Vector3 unit = new(axis.X / length, axis.Y / length, axis.Z / length);
            b.PushQuaternion(l, Quaternion.CreateFromAxisAngle(unit, angle * Radians));
            return 1;
        }, HostTypeId.Quaternion);
        table.Function("Lerp", static (b, l) =>
        {
            Quaternion a = Argument(b, l, 1, "Lerp");
            Quaternion c = Argument(b, l, 2, "Lerp");
            float t = Math.Clamp((float)b.CheckNumber(l, 3, "Lerp"), 0f, 1f);
            float dot = (a.X * c.X) + (a.Y * c.Y) + (a.Z * c.Z) + (a.W * c.W);
            if (dot < 0f)
            {
                c = new Quaternion(-c.X, -c.Y, -c.Z, -c.W);
            }

            Quaternion mixed = new(
                (a.X * (1f - t)) + (c.X * t),
                (a.Y * (1f - t)) + (c.Y * t),
                (a.Z * (1f - t)) + (c.Z * t),
                (a.W * (1f - t)) + (c.W * t));
            b.PushQuaternion(l, Quaternion.Normalize(mixed));
            return 1;
        }, HostTypeId.Quaternion);
        table.Function("Slerp", static (b, l) =>
        {
            Quaternion a = Argument(b, l, 1, "Slerp");
            Quaternion c = Argument(b, l, 2, "Slerp");
            float t = Math.Clamp((float)b.CheckNumber(l, 3, "Slerp"), 0f, 1f);
            b.PushQuaternion(l, Quaternion.Slerp(a, c, t));
            return 1;
        }, HostTypeId.Quaternion);
        return;

        static Quaternion Read(LuaBinder binder, Lua state) => ReadAt(binder, state, 1);

        static Quaternion ReadAt(LuaBinder binder, Lua state, int index)
        {
            binder.TryGetQuaternion(state, index, out Quaternion value);
            return value;
        }

        static Quaternion Argument(LuaBinder binder, Lua state, int index, string member)
        {
            if (!binder.TryGetQuaternion(state, index, out Quaternion value))
            {
                binder.InvalidArguments(state, member);
            }

            return value;
        }
    }

    /// <summary>
    /// <c>Euler = Ry(y) · Rx(x) · Rz(z)</c> with <c>Rₐ(θ) = (sin(θ/2)·â, cos(θ/2))</c> (§4.3).
    /// </summary>
    private static Quaternion FromEuler(float x, float y, float z)
    {
        Quaternion qx = Quaternion.CreateFromAxisAngle(Vector3.UnitX, x * Radians);
        Quaternion qy = Quaternion.CreateFromAxisAngle(Vector3.UnitY, y * Radians);
        Quaternion qz = Quaternion.CreateFromAxisAngle(Vector3.UnitZ, z * Radians);
        return qy * qx * qz;
    }

    /// <summary>
    /// The angles <c>(ax, ay, az)</c> in <c>[0, 360)</c> such that
    /// <see cref="FromEuler"/> rebuilds the same rotation, <c>ax</c> being the principal
    /// <c>asin</c> value (§4.3, type <c>Quaternion</c>).
    /// </summary>
    private static Vec3 ToEuler(Quaternion q)
    {
        if (q.LengthSquared() > 0f)
        {
            q = Quaternion.Normalize(q);
        }

        // Entries of R = Ry(ay)·Rx(ax)·Rz(az) read off the quaternion.
        float sinX = Math.Clamp(2f * ((q.W * q.X) - (q.Y * q.Z)), -1f, 1f);
        float x = MathF.Asin(sinX);
        float y;
        float z;
        if (MathF.Abs(sinX) > 1f - 1e-6f)
        {
            // cos(ax) is zero: ay and az are no longer independent, so az is taken as 0.
            y = MathF.Atan2(
                2f * ((q.W * q.Y) - (q.X * q.Z)),
                1f - (2f * ((q.Y * q.Y) + (q.Z * q.Z))));
            z = 0f;
        }
        else
        {
            z = MathF.Atan2(
                2f * ((q.X * q.Y) + (q.W * q.Z)),
                1f - (2f * ((q.X * q.X) + (q.Z * q.Z))));
            y = MathF.Atan2(
                2f * ((q.X * q.Z) + (q.W * q.Y)),
                1f - (2f * ((q.X * q.X) + (q.Y * q.Y))));
        }

        return new Vec3(Normalise(x * Degrees), Normalise(y * Degrees), Normalise(z * Degrees));

        static float Normalise(float degrees)
        {
            float wrapped = degrees % 360f;
            return wrapped < 0f ? wrapped + 360f : wrapped;
        }
    }

    private static void RegisterColor(LuaBinder binder)
    {
        HostType type = binder.DeclareType(HostTypeId.Color, "Color", reference: false);
        type.Property(
            "r",
            static (b, l) => Number(l, Read(b, l).R),
            static (b, l) => WriteComponent(b, l, 0, "r"));
        type.Property(
            "g",
            static (b, l) => Number(l, Read(b, l).G),
            static (b, l) => WriteComponent(b, l, 1, "g"));
        type.Property(
            "b",
            static (b, l) => Number(l, Read(b, l).B),
            static (b, l) => WriteComponent(b, l, 2, "b"));
        type.Property(
            "a",
            static (b, l) => Number(l, Read(b, l).A),
            static (b, l) => WriteComponent(b, l, 3, "a"));
        type.Equal = static (b, l) => Read(b, l) == ReadAt(b, l, 2);
        type.Add = static (b, l) =>
        {
            LuaColor a = ReadAt(b, l, 1);
            LuaColor c = ReadAt(b, l, 2);
            b.PushColor(l, new LuaColor(a.R + c.R, a.G + c.G, a.B + c.B, a.A + c.A));
            return 1;
        };
        type.Subtract = static (b, l) =>
        {
            LuaColor a = ReadAt(b, l, 1);
            LuaColor c = ReadAt(b, l, 2);
            b.PushColor(l, new LuaColor(a.R - c.R, a.G - c.G, a.B - c.B, a.A - c.A));
            return 1;
        };
        type.Multiply = static (b, l) =>
        {
            if (l.Type(1) == LuaType.Number)
            {
                return Scale(b, l, ReadAt(b, l, 2), (float)l.ToNumber(1));
            }

            if (l.Type(2) == LuaType.Number)
            {
                return Scale(b, l, ReadAt(b, l, 1), (float)l.ToNumber(2));
            }

            LuaColor a = ReadAt(b, l, 1);
            LuaColor c = ReadAt(b, l, 2);
            b.PushColor(l, new LuaColor(a.R * c.R, a.G * c.G, a.B * c.B, a.A * c.A));
            return 1;
        };
        type.Text = static (b, l) =>
        {
            LuaColor value = Read(b, l);
            return Text(l, $"({LuaNumberFormat.ToText(value.R)}, {LuaNumberFormat.ToText(value.G)}, "
                + $"{LuaNumberFormat.ToText(value.B)}, {LuaNumberFormat.ToText(value.A)})");
        };

        HostType table = binder.DeclareType(HostTypeId.ColorClass, "ColorClasse");
        table.Property(
            "black",
            static (b, l) =>
            {
                b.PushColor(l, LuaColor.Black);
                return 1;
            },
            result: HostTypeId.Color);
        table.Property(
            "white",
            static (b, l) =>
            {
                b.PushColor(l, LuaColor.White);
                return 1;
            },
            result: HostTypeId.Color);
        table.Call = static (b, l) =>
        {
            float r = (float)b.CheckNumber(l, 2, "Color");
            float g = (float)b.CheckNumber(l, 3, "Color");
            float blue = (float)b.CheckNumber(l, 4, "Color");
            float alpha = (float)b.OptionalNumber(l, 5, "Color", 1.0);
            b.PushColor(l, new LuaColor(r, g, blue, alpha));
            return 1;
        };
        table.CallResult = HostTypeId.Color;
        return;

        static LuaColor Read(LuaBinder binder, Lua state) => ReadAt(binder, state, 1);

        static LuaColor ReadAt(LuaBinder binder, Lua state, int index)
        {
            binder.TryGetColor(state, index, out LuaColor value);
            return value;
        }

        static int Scale(LuaBinder binder, Lua state, LuaColor value, float k)
        {
            binder.PushColor(state, new LuaColor(value.R * k, value.G * k, value.B * k, value.A * k));
            return 1;
        }
    }

    private static void RegisterIntVectors(LuaBinder binder)
    {
        HostType type = binder.DeclareType(HostTypeId.IntVector2, "IntVector2", reference: false);
        type.Property(
            "x",
            static (b, l) => Number(l, Read(b, l).X),
            static (b, l) => WriteIntComponent(b, l, 0, "x"));
        type.Property(
            "y",
            static (b, l) => Number(l, Read(b, l).Y),
            static (b, l) => WriteIntComponent(b, l, 1, "y"));
        type.Equal = static (b, l) => Read(b, l) == ReadAt(b, l, 2);
        type.Add = static (b, l) =>
        {
            b.PushIntVector2(l, ReadAt(b, l, 1) + ReadAt(b, l, 2));
            return 1;
        };
        type.Subtract = static (b, l) =>
        {
            b.PushIntVector2(l, ReadAt(b, l, 1) - ReadAt(b, l, 2));
            return 1;
        };
        type.Divide = static (b, l) =>
        {
            if (l.Type(2) != LuaType.Number)
            {
                return b.InvalidArguments(l, "IntVector2 /");
            }

            int k = LuaNumberFormat.ToInt32(l.ToNumber(2));
            if (k == 0)
            {
                return b.Raise(l, "IntVector2 / 0");
            }

            Cell value = Read(b, l);
            b.PushIntVector2(l, new Cell(value.X / k, value.Y / k));
            return 1;
        };
        type.Text = static (b, l) =>
        {
            Cell value = Read(b, l);
            return Text(l, $"({value.X}, {value.Y})");
        };

        HostType table = binder.DeclareType(HostTypeId.IntVector2Class, "IntVector2Classe");
        table.Property(
            "zero",
            static (b, l) =>
            {
                b.PushIntVector2(l, default);
                return 1;
            },
            result: HostTypeId.IntVector2);
        table.Property(
            "one",
            static (b, l) =>
            {
                b.PushIntVector2(l, new Cell(1, 1));
                return 1;
            },
            result: HostTypeId.IntVector2);
        table.Call = static (b, l) =>
        {
            if (l.IsNoneOrNil(2))
            {
                b.PushIntVector2(l, default);
                return 1;
            }

            int x = b.CheckInteger(l, 2, "IntVector2");
            int y = b.CheckInteger(l, 3, "IntVector2");
            b.PushIntVector2(l, new Cell(x, y));
            return 1;
        };
        table.CallResult = HostTypeId.IntVector2;

        HostType three = binder.DeclareType(HostTypeId.IntVector3, "IntVector3", reference: false);
        three.Property(
            "x",
            static (b, l) => Number(l, ReadThree(b, l).X),
            static (b, l) => WriteIntComponent(b, l, 0, "x"));
        three.Property(
            "y",
            static (b, l) => Number(l, ReadThree(b, l).Y),
            static (b, l) => WriteIntComponent(b, l, 1, "y"));
        three.Property(
            "z",
            static (b, l) => Number(l, ReadThree(b, l).Z),
            static (b, l) => WriteIntComponent(b, l, 2, "z"));
        three.Equal = static (b, l) => ReadThree(b, l) == ReadThreeAt(b, l, 2);

        HostType threeTable = binder.DeclareType(HostTypeId.IntVector3Class, "IntVector3Classe");
        threeTable.Call = static (b, l) =>
        {
            int x = b.CheckInteger(l, 2, "IntVector3");
            int y = b.CheckInteger(l, 3, "IntVector3");
            int z = b.CheckInteger(l, 4, "IntVector3");
            b.PushIntVector3(l, new LuaIntVector3(x, y, z));
            return 1;
        };
        threeTable.CallResult = HostTypeId.IntVector3;
        return;

        static Cell Read(LuaBinder binder, Lua state) => ReadAt(binder, state, 1);

        static Cell ReadAt(LuaBinder binder, Lua state, int index)
        {
            binder.TryGetIntVector2(state, index, out Cell value);
            return value;
        }

        static LuaIntVector3 ReadThree(LuaBinder binder, Lua state) => ReadThreeAt(binder, state, 1);

        static unsafe LuaIntVector3 ReadThreeAt(LuaBinder binder, Lua state, int index)
        {
            if (binder.TypeIdAt(state, index) != HostTypeId.IntVector3)
            {
                return default;
            }

            int* box = (int*)state.ToUserData(index);
            return new LuaIntVector3(box[1], box[2], box[3]);
        }
    }

    private static void RegisterRect(LuaBinder binder)
    {
        HostType type = binder.DeclareType(HostTypeId.Rect, "Rect", reference: false);
        type.Property(
            "x",
            static (b, l) => Number(l, Read(b, l).X),
            static (b, l) => WriteComponent(b, l, 0, "x"));
        type.Property(
            "y",
            static (b, l) => Number(l, Read(b, l).Y),
            static (b, l) => WriteComponent(b, l, 1, "y"));
        type.Property(
            "width",
            static (b, l) => Number(l, Read(b, l).Width),
            static (b, l) => WriteComponent(b, l, 2, "width"));
        type.Property(
            "height",
            static (b, l) => Number(l, Read(b, l).Height),
            static (b, l) => WriteComponent(b, l, 3, "height"));
        type.Equal = static (b, l) => Read(b, l) == ReadAt(b, l, 2);
        type.Function("Overlaps", static (b, l) =>
        {
            Rect mine = Read(b, l);
            Rect other = b.CheckRect(l, 2, "Overlaps");
            return Boolean(l, mine.Overlaps(other));
        });
        type.Text = static (b, l) =>
        {
            Rect value = Read(b, l);
            return Text(l, $"({LuaNumberFormat.ToText(value.X)}, {LuaNumberFormat.ToText(value.Y)}, "
                + $"{LuaNumberFormat.ToText(value.Width)}, {LuaNumberFormat.ToText(value.Height)})");
        };

        HostType table = binder.DeclareType(HostTypeId.RectClass, "RectClasse");
        table.Call = static (b, l) =>
        {
            float x = (float)b.CheckNumber(l, 2, "Rect");
            float y = (float)b.CheckNumber(l, 3, "Rect");
            float width = (float)b.CheckNumber(l, 4, "Rect");
            float height = (float)b.CheckNumber(l, 5, "Rect");
            b.PushRect(l, new Rect(x, y, width, height));
            return 1;
        };
        table.CallResult = HostTypeId.Rect;
        return;

        static Rect Read(LuaBinder binder, Lua state) => ReadAt(binder, state, 1);

        static Rect ReadAt(LuaBinder binder, Lua state, int index)
        {
            binder.TryGetRect(state, index, out Rect value);
            return value;
        }
    }
}
