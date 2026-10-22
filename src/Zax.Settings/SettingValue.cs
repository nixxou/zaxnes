using System.Globalization;

namespace Zax.Settings;

/// <summary>Three reals, for <c>light.direction</c> (SPEC-09 §4.1.3).</summary>
/// <param name="X">First component.</param>
/// <param name="Y">Second component.</param>
/// <param name="Z">Third component.</param>
public readonly record struct SettingVector3(float X, float Y, float Z)
{
    /// <summary>Whether every component is zero, which the domain "non nul" refuses.</summary>
    public bool IsZero => X == 0f && Y == 0f && Z == 0f;
}

/// <summary>An RGBA colour of four reals in 0..1 (SPEC-09 §4.1.3, §4.1.4).</summary>
/// <param name="R">Red.</param>
/// <param name="G">Green.</param>
/// <param name="B">Blue.</param>
/// <param name="A">Alpha.</param>
public readonly record struct SettingColor(float R, float G, float B, float A)
{
    /// <summary>Opaque black, the default of <c>scene.custom_bg_color</c>.</summary>
    public static SettingColor Black => new(0f, 0f, 0f, 1f);

    /// <summary>Opaque white, the default of <c>light.color</c>.</summary>
    public static SettingColor White => new(1f, 1f, 1f, 1f);

    /// <summary>The colour of a packed 24-bit RGB value, fully opaque.</summary>
    public static SettingColor FromRgb(int packed) => new(
        ((packed >> 16) & 0xFF) / 255f,
        ((packed >> 8) & 0xFF) / 255f,
        (packed & 0xFF) / 255f,
        1f);

    /// <summary>The <c>#RRGGBBAA</c> form the settings file uses (SPEC-09 §4.5.1).</summary>
    public override string ToString()
    {
        static int Byte(float value) => (int)Math.Round(Math.Clamp(value, 0f, 1f) * 255f);
        return string.Format(
            CultureInfo.InvariantCulture,
            "#{0:X2}{1:X2}{2:X2}{3:X2}",
            Byte(R),
            Byte(G),
            Byte(B),
            Byte(A));
    }

    /// <summary>Parses the <c>#RRGGBBAA</c> form; <c>#RRGGBB</c> is accepted as opaque.</summary>
    public static bool TryParse(string? text, out SettingColor color)
    {
        color = Black;
        if (string.IsNullOrWhiteSpace(text))
        {
            return false;
        }

        ReadOnlySpan<char> digits = text.AsSpan().Trim();
        if (digits.Length > 0 && digits[0] == '#')
        {
            digits = digits[1..];
        }

        if (digits.Length is not (6 or 8))
        {
            return false;
        }

        Span<float> channels = [0f, 0f, 0f, 1f];
        for (int i = 0; i < digits.Length / 2; i++)
        {
            if (!byte.TryParse(digits.Slice(i * 2, 2), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out byte v))
            {
                return false;
            }

            channels[i] = v / 255f;
        }

        color = new SettingColor(channels[0], channels[1], channels[2], channels[3]);
        return true;
    }
}

/// <summary>Four whole numbers, for <c>ui.window_rect</c> (REQ-PERS-067).</summary>
/// <param name="X">Left edge.</param>
/// <param name="Y">Top edge.</param>
/// <param name="Width">Width.</param>
/// <param name="Height">Height.</param>
public readonly record struct SettingRect(int X, int Y, int Width, int Height);

/// <summary>
/// The value of one setting, of any of the shapes of <see cref="SettingKind"/>.
/// </summary>
/// <remarks>
/// Scalars live in a double, which holds every <c>int</c>, every <c>float</c> and the two
/// booleans without loss; the compound shapes live in the reference slot. Equality is by value,
/// which is what REQ-PERS-009 needs to decide whether a write really changed anything.
/// </remarks>
public readonly struct SettingValue : IEquatable<SettingValue>
{
    private readonly double _number;
    private readonly object? _reference;

    private SettingValue(SettingKind kind, double number, object? reference)
    {
        Kind = kind;
        _number = number;
        _reference = reference;
    }

    /// <summary>The shape of this value.</summary>
    public SettingKind Kind { get; }

    /// <summary>A boolean value.</summary>
    public static SettingValue OfBool(bool value) => new(SettingKind.Bool, value ? 1d : 0d, null);

    /// <summary>A whole value.</summary>
    public static SettingValue OfInt(int value) => new(SettingKind.Int, value, null);

    /// <summary>A real value.</summary>
    public static SettingValue OfFloat(float value) => new(SettingKind.Float, value, null);

    /// <summary>An enumerated value, held as its number (REQ-PERS-004).</summary>
    public static SettingValue OfEnum(int value) => new(SettingKind.Enum, value, null);

    /// <summary>A colour value.</summary>
    public static SettingValue OfColor(SettingColor value) => new(SettingKind.Color, 0d, value);

    /// <summary>A three-component vector.</summary>
    public static SettingValue OfVector3(SettingVector3 value) => new(SettingKind.Vector3, 0d, value);

    /// <summary>A text value; null becomes the empty string.</summary>
    public static SettingValue OfString(string? value) => new(SettingKind.String, 0d, value ?? string.Empty);

    /// <summary>A rectangle value.</summary>
    public static SettingValue OfRect(SettingRect value) => new(SettingKind.Rect, 0d, value);

    /// <summary>A value this model only stores and hands back (REQ-PERS-016).</summary>
    public static SettingValue OfOpaque(object? value) => new(SettingKind.Opaque, 0d, value);

    /// <summary>The boolean reading: any non-zero scalar is true.</summary>
    public bool AsBool => Kind switch
    {
        SettingKind.Bool or SettingKind.Int or SettingKind.Float or SettingKind.Enum => _number != 0d,
        _ => false,
    };

    /// <summary>The whole reading, rounded to the nearest.</summary>
    public int AsInt => Kind switch
    {
        SettingKind.Bool or SettingKind.Int or SettingKind.Enum => (int)_number,
        SettingKind.Float => (int)Math.Round(_number, MidpointRounding.AwayFromZero),
        _ => 0,
    };

    /// <summary>The real reading.</summary>
    public float AsFloat => Kind switch
    {
        SettingKind.Bool or SettingKind.Int or SettingKind.Float or SettingKind.Enum => (float)_number,
        _ => 0f,
    };

    /// <summary>The scalar as stored, without narrowing.</summary>
    public double AsDouble => _number;

    /// <summary>The colour reading, black when the value is not a colour.</summary>
    public SettingColor AsColor => _reference is SettingColor colour ? colour : SettingColor.Black;

    /// <summary>The vector reading, zero when the value is not a vector.</summary>
    public SettingVector3 AsVector3 => _reference is SettingVector3 vector ? vector : default;

    /// <summary>The text reading, empty when the value is not text.</summary>
    public string AsString => _reference as string ?? string.Empty;

    /// <summary>The rectangle reading.</summary>
    public SettingRect AsRect => _reference is SettingRect rect ? rect : default;

    /// <summary>The stored object of an opaque value, or null.</summary>
    public object? AsObject => _reference;

    /// <summary>Whether this value carries nothing at all.</summary>
    public bool IsEmpty => Kind == SettingKind.Opaque && _reference is null && _number == 0d;

    /// <summary>
    /// Reshapes this value to <paramref name="kind"/>, so that a number read from a file can
    /// fill an enumerated or boolean setting (SPEC-09 §4.5.1).
    /// </summary>
    public SettingValue CoerceTo(SettingKind kind)
    {
        if (Kind == kind)
        {
            return this;
        }

        return kind switch
        {
            SettingKind.Bool => OfBool(AsBool),
            SettingKind.Int => OfInt(AsInt),
            SettingKind.Float => OfFloat(AsFloat),
            SettingKind.Enum => OfEnum(AsInt),
            SettingKind.Color => _reference is SettingColor ? this : OfColor(AsColor),
            SettingKind.Vector3 => _reference is SettingVector3 ? this : OfVector3(AsVector3),
            SettingKind.String => OfString(_reference as string),
            SettingKind.Rect => _reference is SettingRect ? this : OfRect(AsRect),
            _ => OfOpaque(_reference),
        };
    }

    /// <inheritdoc/>
    public bool Equals(SettingValue other)
    {
        if (Kind != other.Kind)
        {
            return false;
        }

        return _reference is null && other._reference is null
            ? _number.Equals(other._number)
            : Equals(_reference, other._reference);
    }

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is SettingValue other && Equals(other);

    /// <inheritdoc/>
    public override int GetHashCode() => HashCode.Combine(Kind, _number, _reference);

    /// <summary>Value equality.</summary>
    public static bool operator ==(SettingValue left, SettingValue right) => left.Equals(right);

    /// <summary>Value inequality.</summary>
    public static bool operator !=(SettingValue left, SettingValue right) => !left.Equals(right);

    /// <inheritdoc/>
    public override string ToString() => Kind switch
    {
        SettingKind.Bool => AsBool ? "true" : "false",
        SettingKind.Int or SettingKind.Enum => AsInt.ToString(CultureInfo.InvariantCulture),
        SettingKind.Float => AsFloat.ToString("0.######", CultureInfo.InvariantCulture),
        SettingKind.Color => AsColor.ToString(),
        SettingKind.Vector3 => FormattableString.Invariant($"({AsVector3.X}, {AsVector3.Y}, {AsVector3.Z})"),
        SettingKind.String => AsString,
        SettingKind.Rect => FormattableString.Invariant(
            $"({AsRect.X}, {AsRect.Y}, {AsRect.Width}, {AsRect.Height})"),
        _ => _reference?.ToString() ?? string.Empty,
    };
}
