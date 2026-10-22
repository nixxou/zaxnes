namespace Zax.Profiles;

/// <summary>
/// A loaded game profile: the 3D conversion of one NES game (SPEC-03 §4.1, root message).
/// </summary>
/// <remarks>
/// <para>
/// This is the model the rest of the program works with. It holds every field of the format,
/// including the ones only the out-of-scope editor and VR functions use, so that a profile can
/// be rewritten without loss (REQ-PROF-004, REQ-PROF-050). It knows nothing about the wire
/// encoding: reference envelopes, nullable cell groups, masked tile planes and field presence
/// all live in <see cref="Wire.ProfileWireCodec"/>.
/// </para>
/// <para>
/// For a <b>built-in</b> profile the identity, title, publisher and author shown in the
/// library come from the catalogue, not from the metadata fields here (REQ-PROF-022,
/// REQ-PROF-060).
/// </para>
/// </remarks>
public sealed class GameProfile
{
    /// <summary>The only accepted value of the signature field (REQ-PROF-020).</summary>
    public const string RequiredSignature = "3dn";

    /// <summary>The version a newly exported profile carries (REQ-PROF-052).</summary>
    public const string CurrentVersionText = "3";

    /// <summary>
    /// Signature (field 1, <c>Type</c>). Must be <see cref="RequiredSignature"/> or absent;
    /// anything else makes the file invalid.
    /// </summary>
    public string? Signature { get; set; }

    /// <summary>Version string as stored (field 2, <c>Ver</c>).</summary>
    public string? VersionText { get; set; }

    /// <summary>The version classification of <see cref="VersionText"/> (REQ-PROF-021).</summary>
    public ProfileFormatVersion FormatVersion => ProfileVersionRules.Classify(VersionText);

    /// <summary>
    /// True for a "V2 format" profile. Exposed to scripts as <c>IsV2Format</c> (SPEC-05) and
    /// it moves the reference point of the deformation pivot (SPEC-04B).
    /// </summary>
    public bool IsV2Format => FormatVersion == ProfileFormatVersion.V2;

    /// <summary>
    /// The Lua scripts (field 3). After a load they are sorted by name with
    /// <see cref="ScriptNameComparer"/>, which is the hook call order (REQ-PROF-024).
    /// </summary>
    public IList<ProfileScript> Scripts { get; } = [];

    /// <summary>
    /// Patterns filed as permanent (field 4). The effective family of a pattern is its own
    /// <see cref="ProfilePattern.Permanent"/> flag, not the list it sits in (REQ-PROF-023).
    /// </summary>
    public IList<ProfilePattern> PermanentPatterns { get; } = [];

    /// <summary>Patterns filed as temporary (field 5).</summary>
    public IList<ProfilePattern> TemporaryPatterns { get; } = [];

    /// <summary>
    /// The embedded settings (field 6). A profile whose settings are missing or incomplete is
    /// refused (REQ-PROF-025), so this is never null on a profile that loaded.
    /// </summary>
    public ProfileSettings? Settings { get; set; }

    /// <summary>Profile identifier (field 7, <c>Id</c>), twelve characters when present.</summary>
    public string? Id { get; set; }

    /// <summary>Title (field 8, <c>Title</c>).</summary>
    public string? Title { get; set; }

    /// <summary>Author of the profile (field 9, <c>Maker</c>).</summary>
    public string? Author { get; set; }

    /// <summary>Publisher of the game (field 10, <c>GameDev</c>).</summary>
    public string? Publisher { get; set; }

    /// <summary>Description (field 11, <c>Des</c>).</summary>
    public string? Description { get; set; }

    /// <summary>
    /// Every pattern of fields 4 and 5, in that order, each one once even when a file lists the
    /// same pattern object twice through a shared reference (REQ-PROF-023).
    /// </summary>
    public IEnumerable<ProfilePattern> AllPatterns
    {
        get
        {
            HashSet<ProfilePattern> seen = new(ReferenceEqualityComparer.Instance);
            foreach (ProfilePattern pattern in PermanentPatterns)
            {
                if (seen.Add(pattern))
                {
                    yield return pattern;
                }
            }

            foreach (ProfilePattern pattern in TemporaryPatterns)
            {
                if (seen.Add(pattern))
                {
                    yield return pattern;
                }
            }
        }
    }

    /// <summary>
    /// The distinct tile objects of the profile's grids, in the order they are first met. Tile
    /// objects are shared inside a file, so this is smaller than the number of filled cells.
    /// </summary>
    public IEnumerable<ProfileTile> DistinctTileObjects
    {
        get
        {
            HashSet<ProfileTile> seen = new(ReferenceEqualityComparer.Instance);
            foreach (ProfilePattern pattern in AllPatterns)
            {
                if (pattern.Grid is null)
                {
                    continue;
                }

                foreach (ProfileTile? cell in pattern.Grid.Cells)
                {
                    if (cell is not null && seen.Add(cell))
                    {
                        yield return cell;
                    }
                }
            }
        }
    }

    /// <summary>
    /// Sorts <see cref="Scripts"/> into the order of REQ-PROF-024 / REQ-LUA-004. The loader
    /// calls it; call it again after adding a script.
    /// </summary>
    public void SortScripts()
    {
        List<ProfileScript> sorted = [.. Scripts];
        sorted.Sort(ScriptNameComparer.Instance);
        Scripts.Clear();
        foreach (ProfileScript script in sorted)
        {
            Scripts.Add(script);
        }
    }

    /// <summary>
    /// Builds the empty profile a game falls back to when its profile cannot be loaded
    /// (REQ-PROF-033): no pattern, no script, complete default settings.
    /// </summary>
    public static GameProfile CreateEmpty()
        => new()
        {
            Signature = RequiredSignature,
            VersionText = CurrentVersionText,
            Settings = new ProfileSettings
            {
                Light = new LightSettings
                {
                    Direction = new Vec3F(1f, -1f, 1f),
                    Color = new ArgbColor(-1),
                    Intensity = 1f,
                },
                Clipping = new Vec4F(0.1f, 8.1f, 8.1f, 0.1f),
                Layers = LayerSettingsSet.CreateDefault(),
                UseNesBackgroundColor = true,
            },
        };
}

/// <summary>Version rules of the profile format (SPEC-03 REQ-PROF-021).</summary>
public static class ProfileVersionRules
{
    /// <summary>
    /// Removes from a version string every character other than a digit, <c>.</c> and
    /// <c>$</c>, as the reader must before comparing it.
    /// </summary>
    public static string Sanitize(string? versionText)
    {
        if (string.IsNullOrEmpty(versionText))
        {
            return string.Empty;
        }

        Span<char> kept = versionText.Length <= 64 ? stackalloc char[versionText.Length] : new char[versionText.Length];
        int length = 0;
        foreach (char c in versionText)
        {
            if (char.IsAsciiDigit(c) || c == '.' || c == '$')
            {
                kept[length++] = c;
            }
        }

        return new string(kept[..length]);
    }

    /// <summary>
    /// Classifies a version string: <c>"1"</c> is unsupported, <c>"2"</c> is the V2 format, and
    /// anything else — including <c>"3"</c>, <c>"2.0"</c> and an absent version — is the current
    /// format.
    /// </summary>
    public static ProfileFormatVersion Classify(string? versionText)
        => Sanitize(versionText) switch
        {
            "1" => ProfileFormatVersion.Unsupported,
            "2" => ProfileFormatVersion.V2,
            _ => ProfileFormatVersion.Current,
        };
}
