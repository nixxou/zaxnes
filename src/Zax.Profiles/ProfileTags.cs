namespace Zax.Profiles;

/// <summary>
/// Parsing of the comma-separated tag strings carried by 3D variants
/// (<c>ShapeParams.TagStr</c>) and by scripts (<c>Script.TagStr</c>), SPEC-03 REQ-PROF-036 and
/// SPEC-05 REQ-LUA-005.
/// </summary>
public static class ProfileTags
{
    /// <summary>The tag that binds a script to every variant.</summary>
    public const string Wildcard = "*";

    /// <summary>Special variant tag: the variant casts shadows (SPEC-06).</summary>
    public const string CastShadowTag = "!cast";

    /// <summary>Special variant tag: the variant receives shadows (long spelling).</summary>
    public const string ReceiveShadowTag = "!receiv";

    /// <summary>Special variant tag: the variant receives shadows (short spelling).</summary>
    public const string ReceiveShadowShortTag = "!recv";

    /// <summary>Special variant tag: the variant is static (SPEC-06).</summary>
    public const string StaticTag = "!static";

    /// <summary>Special variant tag: ground axis (SPEC-06).</summary>
    public const string GroundAxisTag = "_gaxis";

    /// <summary>The five special variant tags of REQ-PROF-036.</summary>
    public static IReadOnlyList<string> SpecialVariantTags { get; } =
    [
        CastShadowTag, ReceiveShadowTag, ReceiveShadowShortTag, StaticTag, GroundAxisTag,
    ];

    /// <summary>
    /// Splits a tag string on commas, trims each element and drops the empty ones
    /// (REQ-PROF-036). A null or empty string gives an empty list.
    /// </summary>
    public static IReadOnlyList<string> Parse(string? tagString)
    {
        if (string.IsNullOrEmpty(tagString))
        {
            return [];
        }

        List<string> tags = [];
        foreach (string part in tagString.Split(','))
        {
            string tag = part.Trim();
            if (tag.Length != 0)
            {
                tags.Add(tag);
            }
        }

        return tags;
    }

    /// <summary>
    /// True when a script's tag string binds it to every variant, that is when it contains the
    /// character <c>*</c> anywhere (SPEC-05 REQ-LUA-005).
    /// </summary>
    public static bool IsUniversal(string? tagString)
        => tagString is not null && tagString.Contains('*', StringComparison.Ordinal);

    /// <summary>
    /// True when a script with <paramref name="scriptTagString"/> is bound to a variant with
    /// <paramref name="variantTagString"/>: the script is universal, or one of its tags equals
    /// one of the variant's tags (exact, case-sensitive) — SPEC-05 REQ-LUA-006.
    /// </summary>
    public static bool Binds(string? scriptTagString, string? variantTagString)
    {
        if (IsUniversal(scriptTagString))
        {
            return true;
        }

        IReadOnlyList<string> scriptTags = Parse(scriptTagString);
        if (scriptTags.Count == 0)
        {
            return false;
        }

        IReadOnlyList<string> variantTags = Parse(variantTagString);
        foreach (string tag in scriptTags)
        {
            foreach (string other in variantTags)
            {
                if (string.Equals(tag, other, StringComparison.Ordinal))
                {
                    return true;
                }
            }
        }

        return false;
    }
}
