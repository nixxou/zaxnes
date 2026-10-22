using System.Globalization;

namespace Zax.Settings;

/// <summary>
/// Everything the model knows about one setting: its stable key, its shape, its default, its
/// domain, its scope, when it takes effect, how Lua names it and how it is discretised for
/// libretro (SPEC-09 §4.1, REQ-PERS-001, REQ-ARCH-021).
/// </summary>
public sealed class SettingDescriptor
{
    /// <summary>Creates a descriptor.</summary>
    /// <param name="key">Stable key of REQ-PERS-080.</param>
    /// <param name="kind">Shape of the value.</param>
    /// <param name="defaultValue">The value of the column "Défaut".</param>
    /// <param name="scope">Scope of the column "Portée".</param>
    /// <param name="effect">Moment of the column "Effet".</param>
    /// <param name="theme">Grouping for the options window and the libretro categories.</param>
    public SettingDescriptor(
        string key,
        SettingKind kind,
        SettingValue defaultValue,
        SettingScope scope,
        SettingEffect effect,
        SettingTheme theme)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(key);
        if (!IsValidKey(key))
        {
            throw new ArgumentException($"'{key}' is not a stable key of REQ-PERS-080", nameof(key));
        }

        Key = key;
        Kind = kind;
        DefaultValue = defaultValue.CoerceTo(kind);
        Scope = scope;
        Effect = effect;
        Theme = theme;
    }

    /// <summary>Stable key, for instance <c>camera.fov</c> (REQ-PERS-080).</summary>
    public string Key { get; }

    /// <summary>Shape of the value.</summary>
    public SettingKind Kind { get; }

    /// <summary>Value of the column "Défaut" (REQ-PERS-002).</summary>
    public SettingValue DefaultValue { get; }

    /// <summary>Scope of the column "Portée" (SPEC-09 §2).</summary>
    public SettingScope Scope { get; }

    /// <summary>Moment of the column "Effet" (REQ-PERS-084).</summary>
    public SettingEffect Effect { get; }

    /// <summary>Theme of the column "Thème" (REQ-PERS-083).</summary>
    public SettingTheme Theme { get; }

    /// <summary>Domain of the column "Domaine" (REQ-PERS-003).</summary>
    public SettingDomain Domain
    {
        get;

        // Refusing null catches a static initialiser that reads a shared domain declared after
        // the catalogue's own table, which would otherwise silently widen the domain.
        init => field = value ?? throw new ArgumentNullException(nameof(Domain));
    } = SettingDomain.Unbounded;

    /// <summary>
    /// The Lua access path with <c>Setting.</c> left off, as the column "Lua" writes it, or
    /// null when the setting is not exposed to scripts (REQ-PERS-008).
    /// </summary>
    public string? LuaPath { get; init; }

    /// <summary>Extra Lua names for the same setting, such as <c>SamePalette</c>.</summary>
    public IReadOnlyList<string> LuaAliases { get; init; } = [];

    /// <summary>
    /// Whether Lua may assign this setting. The members marked "lecture" in §4.2 may not
    /// (REQ-PERS-007).
    /// </summary>
    public bool IsLuaWritable { get; init; } = true;

    /// <summary>
    /// Field number in the settings message embedded in profiles, as the column "N°" writes it
    /// (for instance <c>6.1.3</c>), or null when the setting is absent from that message.
    /// </summary>
    public string? ProfileField { get; init; }

    /// <summary>Label the desktop settings panel shows, or null when it shows none.</summary>
    public string? UiLabel { get; init; }

    /// <summary>
    /// The finite ordered list of values exposed as a libretro core option, or null when the
    /// setting is not exposed (column "libretro", REQ-PERS-081, SPEC-09 §4.6).
    /// </summary>
    public IReadOnlyList<SettingOption>? LibretroValues { get; init; }

    /// <summary>Whether the setting is exposed as a libretro core option.</summary>
    public bool IsLibretroExposed => LibretroValues is { Count: > 0 };

    /// <summary>
    /// Labels a stored file may still carry from an earlier version of the catalogue, each
    /// mapped to the value it becomes. They are accepted when a settings file is read, never
    /// written and never shown.
    /// </summary>
    /// <remarks>
    /// AMD-025 is what this exists for: the TV box setting had four values and now has two,
    /// and a <c>settings.json</c> written before the amendment holds « TV 2 » or « TV 3 »
    /// where the catalogue now knows only « TV ». Without the list the value would fall back
    /// to the default, which is not what « toute valeur héritée 2 ou 3 est ramenée à 1 » says.
    /// </remarks>
    public IReadOnlyList<SettingOption> LegacyOptions { get; init; } = [];

    /// <summary>
    /// The core option key: <c>zaxnes_</c> and the stable key with <c>.</c>, <c>[</c> and
    /// <c>]</c> replaced by <c>_</c> (REQ-PERS-083, prefix set by AMD-029 §3).
    /// </summary>
    public string LibretroKey => BuildLibretroKey(Key);

    /// <summary>
    /// Whether this setting is persisted in <c>settings.json</c>: the user preferences and
    /// <c>ui.fullscreen</c> (REQ-PERS-020).
    /// </summary>
    public bool IsPersisted => Scope == SettingScope.User;

    /// <summary>The label of <paramref name="value"/> in the domain, or its number.</summary>
    public string Format(SettingValue value) => Kind switch
    {
        SettingKind.Enum or SettingKind.Int when Domain.LabelOf(value.AsDouble) is { } label => label,
        SettingKind.Bool => Domain.LabelOf(value.AsBool ? 1d : 0d) ?? (value.AsBool ? "true" : "false"),
        _ => value.ToString(),
    };

    /// <summary>
    /// Prefix every libretro core option key carries (REQ-PERS-083). AMD-029 §3 settles it on
    /// the product name, which closes Q-068.
    /// </summary>
    public const string LibretroKeyPrefix = "zaxnes_";

    /// <summary>
    /// The core option key of an arbitrary stable key, so that a test can check the mapping
    /// without a descriptor (REQ-PERS-083).
    /// </summary>
    public static string BuildLibretroKey(string key)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(key);
        Span<char> buffer = stackalloc char[key.Length];
        for (int i = 0; i < key.Length; i++)
        {
            buffer[i] = key[i] is '.' or '[' or ']' ? '_' : key[i];
        }

        return string.Concat(LibretroKeyPrefix, new string(buffer));
    }

    /// <summary>
    /// Whether <paramref name="key"/> is a stable key (REQ-PERS-080).
    /// </summary>
    /// <remarks>
    /// The grammar REQ-PERS-080 writes, <c>[a-z0-9_]+(\.[a-z0-9_\[\]]+)*</c>, forbids brackets
    /// in the first segment, yet §4.1.6 publishes the keys <c>layer[i].offset</c>, whose first
    /// segment is <c>layer[i]</c>. The published keys win and brackets are accepted in every
    /// segment; see Q-034.
    /// </remarks>
    // Q-034
    public static bool IsValidKey(string key)
    {
        if (string.IsNullOrEmpty(key))
        {
            return false;
        }

        bool atSegmentStart = true;
        foreach (char c in key)
        {
            if (c == '.')
            {
                if (atSegmentStart)
                {
                    return false;
                }

                atSegmentStart = true;
                continue;
            }

            if (c is not (>= 'a' and <= 'z' or >= '0' and <= '9' or '_' or '[' or ']'))
            {
                return false;
            }

            atSegmentStart = false;
        }

        return !atSegmentStart;
    }

    /// <summary>
    /// The ordered list of a continuous setting's libretro values: <paramref name="minimum"/> to
    /// <paramref name="maximum"/> in steps of <paramref name="step"/>, each labelled with
    /// <paramref name="format"/> (REQ-PERS-081).
    /// </summary>
    public static IReadOnlyList<SettingOption> Steps(
        double minimum,
        double maximum,
        double step,
        string format = "0.####")
    {
        if (step <= 0d)
        {
            throw new ArgumentOutOfRangeException(nameof(step));
        }

        int count = (int)Math.Round((maximum - minimum) / step) + 1;
        List<SettingOption> options = new(count);
        for (int i = 0; i < count; i++)
        {
            double value = minimum + (i * step);

            // Steps such as 0.05 do not land on exact binary values; rounding to the step's own
            // precision keeps the labels and the parsed values in agreement.
            value = Math.Round(value, 6);
            options.Add(new SettingOption(value, value.ToString(format, CultureInfo.InvariantCulture)));
        }

        return options;
    }
}
