using System.Globalization;
using Zax.Render;
using Zax.Settings;

namespace Zax.Harness;

/// <summary>
/// The style options of the <c>render</c> command: <c>--style</c>, <c>--style-param</c> and
/// <c>--style-params</c>, which select the visual style of AMD-055 REQ-SCN-078 and set the
/// twelve parameters of REQ-SCN-079 from the command line, so that an Enhanced frame can be
/// captured headless.
/// </summary>
/// <remarks>
/// <para>
/// The values are the ones the settings use: the identifier <c>render.visual_style</c> stores
/// (0 Classic, 4 Enhanced, 1 accepted in writing as an alias of 4) and the Lua identifiers of
/// the twelve parameters (<c>spl.ao</c>, <c>ao.strength</c>, …), which are the arguments of
/// <c>Setting:Get/SetVisualStyleParameter</c> (REQ-LUA-114) and are normative for that reason.
/// The stable <c>render.style.*</c> keys of AMD-055 §4 are accepted as well, so a capture can be
/// written with either spelling.
/// </para>
/// <para>
/// Only the options the user really passed are applied, and they are applied <b>after</b> the
/// scene is built — therefore after the profile's scripts have run (REQ-LUA-114 writes land in
/// the session layer of the settings model, REQ-PERS-010) and before the first frame is
/// rendered. A parameter the command line does not name keeps what the profile and its scripts
/// decided, which is the precedence of the other options of the command (see
/// <see cref="SceneOverrides"/>).
/// </para>
/// <para>
/// The four curvature parameters of AMD-043 (<c>spl.bend_zx</c>, <c>spl.bend_zy</c>,
/// <c>spl.arc_yx</c>, <c>spl.arc_center</c>) are deliberately <b>not</b> accepted here: the
/// curvature is not an effect of the style and acts in Classic as in Enhanced (REQ-SCN-078), so
/// it keeps its own option, <c>--bend</c>.
/// </para>
/// </remarks>
internal sealed class StyleOptions
{
    /// <summary>The values written, by stable key, in the order they were written.</summary>
    private readonly Dictionary<string, int> _parameters = new(StringComparer.Ordinal);

    /// <summary>
    /// The style identifier <c>--style</c> asked for, already resolved through the alias of
    /// REQ-SCN-078 (so 1 is held as 4), or null when the option was left out.
    /// </summary>
    public int? Style { get; private set; }

    /// <summary>The parameters the command line named, by stable key.</summary>
    public IReadOnlyDictionary<string, int> Parameters => _parameters;

    /// <summary>Whether no style option was passed at all.</summary>
    public bool IsEmpty => Style is null && _parameters.Count == 0;

    /// <summary>Whether <c>--style</c> selected Enhanced.</summary>
    public bool Enhanced => Style == SettingsCatalogue.EnhancedStyle;

    /// <summary>
    /// The style identifiers <c>--style</c> accepts, for the usage text and the error messages:
    /// the numbers of REQ-SCN-078 and the labels of the setting's own domain.
    /// </summary>
    public static string AcceptedStyles
        => "0 or 'classic', 4 or 'enhanced', and 1 as an alias of Enhanced";

    /// <summary>
    /// The twelve parameter identifiers REQ-SCN-079 defines, in the order of its table.
    /// </summary>
    public static IEnumerable<string> AcceptedParameters
        => SettingsCatalogue.StyleParameterOrder.Select(static parameter => parameter.Id);

    /// <summary>
    /// Reads the value of <c>--style</c>: a style identifier, or the label of one.
    /// </summary>
    /// <param name="value">The argument that followed the option.</param>
    /// <exception cref="ArgumentException">The value is not a style identifier.</exception>
    public void SetStyle(string? value)
    {
        string text = (value ?? string.Empty).Trim();
        SettingDomain domain = SettingsCatalogue.VisualStyleDomain;

        // A label of the domain ("Classic", "Enhanced"), which is how the options window names
        // the two styles, is accepted beside the identifier.
        if (domain.ValueOf(text) is { } labelled)
        {
            Style = (int)labelled;
            return;
        }

        if (!int.TryParse(text, NumberStyles.Integer, CultureInfo.InvariantCulture, out int asked))
        {
            throw new ArgumentException(
                $"--style expects a style identifier ({AcceptedStyles}), got '{value}'.");
        }

        // The alias of REQ-SCN-078: 1 is accepted in writing and read back as 4. Anything the
        // domain does not list would be put back to Classic by the setting — a silent 0 is not
        // what a capture wants, so it is refused here instead.
        if (!IsStyleIdentifier(domain, asked))
        {
            throw new ArgumentException(
                $"--style: {asked} is not a style identifier ({AcceptedStyles}).");
        }

        Style = asked == SettingsCatalogue.EnhancedStyleAlias
            ? SettingsCatalogue.EnhancedStyle
            : asked;
    }

    /// <summary>
    /// Reads one <c>--style-param &lt;key&gt;=&lt;value&gt;</c>. Passing the option twice for
    /// the same key keeps the last value, as a repeated option normally does.
    /// </summary>
    /// <param name="assignment">The argument that followed the option.</param>
    /// <exception cref="ArgumentException">
    /// The assignment is malformed, the key is not one of the twelve parameters of REQ-SCN-079,
    /// or the value is not one the parameter's domain holds.
    /// </exception>
    public void AddParameter(string? assignment)
    {
        string text = (assignment ?? string.Empty).Trim();
        int equals = text.IndexOf('=', StringComparison.Ordinal);
        if (equals <= 0 || equals == text.Length - 1)
        {
            throw new ArgumentException(
                $"--style-param expects <key>=<value>, got '{assignment}'.");
        }

        string id = text[..equals].Trim();
        string value = text[(equals + 1)..].Trim();
        string key = KeyOf(id);
        _parameters[key] = ValueOf(key, id, value);
    }

    /// <summary>
    /// Reads the <c>--style-params "k=v;k=v"</c> form, a list separated by semicolons — or by
    /// commas, which a shell quotes less often. An empty entry is skipped.
    /// </summary>
    /// <param name="list">The argument that followed the option.</param>
    public void AddParameters(string? list)
    {
        foreach (string entry in (list ?? string.Empty).Split(
            [';', ','],
            StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries))
        {
            AddParameter(entry);
        }
    }

    /// <summary>
    /// Applies what was passed onto the style block of a scene, the way
    /// <c>EmulationHost.PublishStyle</c> does in the application: the style itself when
    /// <c>--style</c> was given, then the parameters the command line named, each of them
    /// already normalised by its own domain.
    /// </summary>
    /// <param name="style">The style block of the scene to render.</param>
    public void ApplyTo(StyleSettings style)
    {
        ArgumentNullException.ThrowIfNull(style);

        if (Style is { } identifier)
        {
            style.Enhanced = identifier == SettingsCatalogue.EnhancedStyle;
        }

        foreach ((string key, int value) in _parameters)
        {
            Publish(style, key, value);
        }
    }

    /// <summary>
    /// One line describing what the command line asked for, for the console report of the
    /// capture; an empty string when nothing was asked.
    /// </summary>
    public string Describe()
    {
        if (IsEmpty)
        {
            return string.Empty;
        }

        List<string> parts = [];
        if (Style is { } identifier)
        {
            parts.Add(
                $"{SettingsCatalogue.VisualStyleDomain.LabelOf(identifier) ?? "?"} ({identifier})");
        }

        // In the order of the table of REQ-SCN-079, whatever order the options came in.
        foreach ((string id, string key) in SettingsCatalogue.StyleParameterOrder)
        {
            if (_parameters.TryGetValue(key, out int value))
            {
                parts.Add(string.Create(CultureInfo.InvariantCulture, $"{id}={value}"));
            }
        }

        return string.Join(", ", parts);
    }

    /// <summary>
    /// The style block of a scene as <c>render.json</c> writes it: the style in force and the
    /// twelve parameters, by their Lua identifier, so that a headless capture of the Enhanced
    /// style can be read back without guessing what produced it.
    /// </summary>
    /// <param name="style">The style block of the captured scene.</param>
    public static string Json(StyleSettings style)
    {
        ArgumentNullException.ThrowIfNull(style);

        IEnumerable<string> entries = SettingsCatalogue.StyleParameterOrder.Select(
            parameter => string.Create(
                CultureInfo.InvariantCulture,
                $"\"{parameter.Id}\": {Read(style, parameter.Key)}"));
        return "\"style\": { \"visualStyle\": "
            + (style.Enhanced ? SettingsCatalogue.EnhancedStyle : SettingsCatalogue.ClassicStyle)
                .ToString(CultureInfo.InvariantCulture)
            + ", " + string.Join(", ", entries) + " }";
    }

    /// <summary>Whether <paramref name="value"/> is a style identifier or one of its aliases.</summary>
    private static bool IsStyleIdentifier(SettingDomain domain, int value)
        => (domain.Options?.Any(option => (int)option.Value == value) ?? false)
            || (domain.Aliases?.ContainsKey(value) ?? false);

    /// <summary>
    /// The stable key the identifier <paramref name="id"/> designates — a Lua identifier of
    /// REQ-SCN-079, or the stable key itself.
    /// </summary>
    /// <exception cref="ArgumentException">The identifier is not a style parameter.</exception>
    private static string KeyOf(string id)
    {
        foreach ((string known, string key) in SettingsCatalogue.StyleParameterOrder)
        {
            if (string.Equals(id, known, StringComparison.OrdinalIgnoreCase)
                || string.Equals(id, key, StringComparison.OrdinalIgnoreCase))
            {
                return key;
            }
        }

        // The curvature parameters are the one plausible mistake: REQ-LUA-114 reaches them
        // through the same method, but they are not effects of the style (AMD-043, REQ-SCN-078).
        string hint = SettingsCatalogue.VisualStyleParameter(SettingsCatalogue.EnhancedStyle, id)
                is not null
            ? " It is a curvature parameter of AMD-043, which acts in both styles: use --bend."
            : string.Empty;

        throw new ArgumentException(
            $"--style-param: '{id}' is not a style parameter of AMD-055 REQ-SCN-079."
                + $"{hint} Expected one of: {string.Join(", ", AcceptedParameters)}.");
    }

    /// <summary>
    /// The whole number <paramref name="value"/> holds, refused unless the parameter's own
    /// domain holds it: the twelve parameters are whole numbers on a ladder of step 1
    /// (REQ-SCN-079), and a value off the ladder would be silently rounded by the setting.
    /// </summary>
    /// <exception cref="ArgumentException">The value is not one the domain holds.</exception>
    private static int ValueOf(string key, string id, string value)
    {
        SettingDomain domain = SettingsCatalogue.Get(key).Domain;
        string allowed = string.Create(
            CultureInfo.InvariantCulture,
            $"{domain.Minimum ?? 0d:0.####} to {domain.Maximum ?? 0d:0.####}");

        if (!int.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out int asked))
        {
            throw new ArgumentException(
                $"--style-param {id}: '{value}' is not a whole number ({allowed}).");
        }

        if (asked != (int)domain.Snap(asked))
        {
            throw new ArgumentException(
                $"--style-param {id}: {asked} is out of range ({allowed}).");
        }

        return asked;
    }

    /// <summary>Writes one parameter onto the style block, by its stable key.</summary>
    private static void Publish(StyleSettings style, string key, int value)
    {
        switch (key)
        {
            case SettingKeys.RenderStyleAo:
                style.AmbientOcclusion = value;
                break;
            case SettingKeys.RenderStyleAoStrength:
                style.AoStrength = value;
                break;
            case SettingKeys.RenderStyleAoRadius:
                style.AoRadius = value;
                break;
            case SettingKeys.RenderStyleAoThickness:
                style.AoThickness = value;
                break;
            case SettingKeys.RenderStyleAoQuality:
                style.AoQuality = value;
                break;
            case SettingKeys.RenderStylePixelShadow:
                style.PixelShadow = value != 0;
                break;
            case SettingKeys.RenderStyleVoxelGrid:
                style.VoxelGrid = value;
                break;
            case SettingKeys.RenderStyleReflections:
                style.Reflections = value != 0;
                break;
            case SettingKeys.RenderStyleSsrShine:
                style.SsrShine = value;
                break;
            case SettingKeys.RenderStyleSsrClarity:
                style.SsrClarity = value;
                break;
            case SettingKeys.RenderStyleSsrLength:
                style.SsrLength = value;
                break;
            case SettingKeys.RenderStyleSsrQuality:
                style.SsrQuality = value;
                break;
            default:
                throw new ArgumentException($"'{key}' is not a style parameter.", nameof(key));
        }
    }

    /// <summary>Reads one parameter back from the style block, by its stable key.</summary>
    private static int Read(StyleSettings style, string key) => key switch
    {
        SettingKeys.RenderStyleAo => style.AmbientOcclusion,
        SettingKeys.RenderStyleAoStrength => style.AoStrength,
        SettingKeys.RenderStyleAoRadius => style.AoRadius,
        SettingKeys.RenderStyleAoThickness => style.AoThickness,
        SettingKeys.RenderStyleAoQuality => style.AoQuality,
        SettingKeys.RenderStylePixelShadow => style.PixelShadow ? 1 : 0,
        SettingKeys.RenderStyleVoxelGrid => style.VoxelGrid,
        SettingKeys.RenderStyleReflections => style.Reflections ? 1 : 0,
        SettingKeys.RenderStyleSsrShine => style.SsrShine,
        SettingKeys.RenderStyleSsrClarity => style.SsrClarity,
        SettingKeys.RenderStyleSsrLength => style.SsrLength,
        SettingKeys.RenderStyleSsrQuality => style.SsrQuality,
        _ => throw new ArgumentException($"'{key}' is not a style parameter.", nameof(key)),
    };
}
