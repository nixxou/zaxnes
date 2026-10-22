namespace Zax.Profiles;

/// <summary>
/// A Lua script of a profile (SPEC-03 REQ-PROF-038, <c>Script</c>). Compilation and execution
/// belong to SPEC-05; a script that fails to compile does not stop the profile from loading.
/// </summary>
public sealed class ProfileScript
{
    /// <summary>Creates a script.</summary>
    public ProfileScript(string? name, bool enabled, string? source, string? tagString)
    {
        Name = name;
        Enabled = enabled;
        Source = source;
        TagString = tagString;
    }

    /// <summary>
    /// Script name. The file holds it twice, in fields 1 and 5; field 1 is authoritative and
    /// field 5 is redundant (REQ-PROF-038), and both are written back from this value.
    /// </summary>
    public string? Name { get; set; }

    /// <summary>
    /// Whether the script receives the frame hooks. A disabled script still receives
    /// <c>Start</c> and <c>End</c> (SPEC-05 REQ-LUA-007).
    /// </summary>
    public bool Enabled { get; set; }

    /// <summary>
    /// The <c>error</c> flag as stored. It is always false in the built-in profiles and SPEC-05
    /// REQ-LUA-001 requires ignoring it at load time: every script starts without error.
    /// </summary>
    public bool StoredErrorFlag { get; set; }

    /// <summary>Full Lua source, or <see langword="null"/> when the field is absent.</summary>
    public string? Source { get; set; }

    /// <summary>Raw tag string, or <see langword="null"/> when the field is absent.</summary>
    public string? TagString { get; set; }

    /// <summary>Tags of this script, split and trimmed (REQ-PROF-036).</summary>
    public IReadOnlyList<string> Tags => ProfileTags.Parse(TagString);

    /// <summary>True when this script is bound to every variant (SPEC-05 REQ-LUA-005).</summary>
    public bool BindsToEveryVariant => ProfileTags.IsUniversal(TagString);

    /// <summary>True when this script is bound to <paramref name="variant"/> (REQ-LUA-006).</summary>
    public bool Binds(PatternVariant variant)
    {
        ArgumentNullException.ThrowIfNull(variant);
        return ProfileTags.Binds(TagString, variant.TagString);
    }

    /// <inheritdoc/>
    public override string ToString() => Name ?? "<unnamed>";
}
