using System.Globalization;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Zax.Settings;

/// <summary>
/// Turns a setting value into the JSON of SPEC-09 §4.5.1 and back: booleans as
/// <c>true</c>/<c>false</c>, numbers as numbers, enumerations as the label of the column
/// "Domaine" (the number is accepted as well on reading), colours as <c>"#RRGGBBAA"</c> and
/// vectors as arrays of numbers.
/// </summary>
public static class SettingsJson
{
    /// <summary>How the file is written: indented UTF-8 without a byte-order mark (§4.5).</summary>
    public static JsonSerializerOptions WriteOptions { get; } = new() { WriteIndented = true };

    /// <summary>The JSON form of <paramref name="value"/> for <paramref name="descriptor"/>.</summary>
    public static JsonNode? ToJson(SettingDescriptor descriptor, SettingValue value)
    {
        ArgumentNullException.ThrowIfNull(descriptor);

        return descriptor.Kind switch
        {
            SettingKind.Bool => JsonValue.Create(value.AsBool),
            SettingKind.Int => JsonValue.Create(value.AsInt),
            SettingKind.Float => JsonValue.Create(value.AsFloat),
            SettingKind.Enum => descriptor.Domain.LabelOf(value.AsDouble) is { } label
                ? JsonValue.Create(label)
                : JsonValue.Create(value.AsInt),
            SettingKind.Color => JsonValue.Create(value.AsColor.ToString()),
            SettingKind.Vector3 => new JsonArray(
                JsonValue.Create(value.AsVector3.X),
                JsonValue.Create(value.AsVector3.Y),
                JsonValue.Create(value.AsVector3.Z)),
            SettingKind.Rect => new JsonArray(
                JsonValue.Create(value.AsRect.X),
                JsonValue.Create(value.AsRect.Y),
                JsonValue.Create(value.AsRect.Width),
                JsonValue.Create(value.AsRect.Height)),
            SettingKind.String => JsonValue.Create(value.AsString),

            // An opaque value has no JSON form; nothing of that kind is persisted anyway.
            _ => null,
        };
    }

    /// <summary>
    /// Reads <paramref name="node"/> as a value of <paramref name="descriptor"/>. Returns false
    /// when the node has the wrong type, in which case REQ-PERS-022 asks for the default to be
    /// kept, silently and for that value alone.
    /// </summary>
    public static bool TryFromJson(SettingDescriptor descriptor, JsonNode? node, out SettingValue value)
    {
        ArgumentNullException.ThrowIfNull(descriptor);
        value = descriptor.DefaultValue;

        if (node is null)
        {
            return false;
        }

        switch (descriptor.Kind)
        {
            case SettingKind.Bool:
                if (TryBool(node) is { } flag)
                {
                    value = SettingValue.OfBool(flag);
                    return true;
                }

                return false;

            case SettingKind.Int:
                if (TryNumber(node) is { } whole)
                {
                    value = SettingValue.OfInt((int)Math.Round(whole, MidpointRounding.AwayFromZero));
                    return true;
                }

                return false;

            case SettingKind.Float:
                if (TryNumber(node) is { } real)
                {
                    value = SettingValue.OfFloat((float)real);
                    return true;
                }

                return false;

            case SettingKind.Enum:
                if (TryNumber(node) is { } number)
                {
                    value = SettingValue.OfEnum((int)number);
                    return true;
                }

                if (TryText(node) is { } text)
                {
                    if (descriptor.Domain.ValueOf(text) is { } fromLabel)
                    {
                        value = SettingValue.OfEnum((int)fromLabel);
                        return true;
                    }

                    // A label the catalogue dropped: AMD-025 renamed the four TV box values
                    // into two, and a file written before it must still be read.
                    foreach (SettingOption legacy in descriptor.LegacyOptions)
                    {
                        if (string.Equals(legacy.Label, text, StringComparison.OrdinalIgnoreCase))
                        {
                            value = SettingValue.OfEnum((int)legacy.Value);
                            return true;
                        }
                    }
                }

                return false;

            case SettingKind.Color:
                if (TryText(node) is { } colour && SettingColor.TryParse(colour, out SettingColor parsed))
                {
                    value = SettingValue.OfColor(parsed);
                    return true;
                }

                return false;

            case SettingKind.Vector3:
                if (TryNumbers(node, 3) is { } vector)
                {
                    value = SettingValue.OfVector3(
                        new SettingVector3((float)vector[0], (float)vector[1], (float)vector[2]));
                    return true;
                }

                return false;

            case SettingKind.Rect:
                if (TryNumbers(node, 4) is { } rect)
                {
                    value = SettingValue.OfRect(new SettingRect(
                        (int)rect[0],
                        (int)rect[1],
                        (int)rect[2],
                        (int)rect[3]));
                    return true;
                }

                return false;

            case SettingKind.String:
                if (TryText(node) is { } content)
                {
                    value = SettingValue.OfString(content);
                    return true;
                }

                return false;

            default:
                return false;
        }
    }

    private static bool? TryBool(JsonNode node) => node.GetValueKind() switch
    {
        JsonValueKind.True => true,
        JsonValueKind.False => false,
        _ => null,
    };

    private static double? TryNumber(JsonNode node) => node.GetValueKind() switch
    {
        JsonValueKind.Number when node.AsValue().TryGetValue(out double value) => value,
        _ => null,
    };

    private static string? TryText(JsonNode node) => node.GetValueKind() == JsonValueKind.String
        ? node.GetValue<string>()
        : null;

    private static double[]? TryNumbers(JsonNode node, int count)
    {
        if (node is not JsonArray array || array.Count != count)
        {
            return null;
        }

        double[] values = new double[count];
        for (int i = 0; i < count; i++)
        {
            if (array[i] is not { } element || TryNumber(element) is not { } value)
            {
                return null;
            }

            values[i] = value;
        }

        return values;
    }

    /// <summary>
    /// The invariant text form of a number, for the libretro labels and the log lines.
    /// </summary>
    public static string Number(double value) => value.ToString("0.######", CultureInfo.InvariantCulture);
}
