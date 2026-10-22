using System.Security.Cryptography;

namespace Zax.Profiles;

/// <summary>
/// The normalisations SPEC-03 REQ-PROF-052 asks for when a <b>new</b> profile is written out,
/// outside the profile editor.
/// </summary>
/// <remarks>
/// <para>
/// Writing an existing profile back needs none of this: it is written exactly as it was read
/// (REQ-PROF-051). These rules apply when the program exports a profile it did not read from a
/// file.
/// </para>
/// <para>
/// One rule of REQ-PROF-052 is not here: a variant whose texture equals the default texture
/// built from the tiles must be written as the null marker. Computing that default texture is
/// part of the conversion engine (SPEC-04), so the rule belongs with it; everything else is
/// below.
/// </para>
/// </remarks>
public static class ProfileExport
{
    /// <summary>Length of a generated identifier, in characters.</summary>
    public const int IdentifierLength = 12;

    /// <summary>
    /// Generates a profile identifier: nine cryptographically random bytes in standard base64,
    /// which is twelve characters of <c>A–Z a–z 0–9 + /</c> with no padding.
    /// </summary>
    /// <remarks>
    /// This randomness is outside the determinism of REQ-ARCH-012 on purpose, as REQ-PROF-052
    /// says: an export happens outside the game loop and influences neither the emulation nor
    /// the conversion.
    /// </remarks>
    public static string NewIdentifier()
    {
        byte[] entropy = RandomNumberGenerator.GetBytes(9);
        return Convert.ToBase64String(entropy);
    }

    /// <summary>
    /// Prepares <paramref name="profile"/> to be written as a new file: current signature and
    /// version, an identifier when it has none, a title when it is empty, the scripts in their
    /// sorted order, and the patterns split between the permanent and the temporary list
    /// according to their own flag.
    /// </summary>
    /// <param name="profile">The profile to normalise, modified in place.</param>
    /// <param name="romFileName">
    /// Name of the ROM file, used as the title when the title is the empty string. Ignored when
    /// null.
    /// </param>
    public static void Normalize(GameProfile profile, string? romFileName = null)
    {
        ArgumentNullException.ThrowIfNull(profile);

        profile.Signature = GameProfile.RequiredSignature;
        profile.VersionText = GameProfile.CurrentVersionText;

        if (string.IsNullOrEmpty(profile.Id))
        {
            profile.Id = NewIdentifier();
        }

        if (profile.Title is not null && profile.Title.Length == 0 && romFileName is not null)
        {
            profile.Title = romFileName;
        }

        profile.SortScripts();
        SplitPatternsByFamily(profile);
    }

    /// <summary>
    /// Moves every pattern into the list its <see cref="ProfilePattern.Permanent"/> flag calls
    /// for, keeping the relative order of each family (REQ-PROF-023, REQ-PROF-052).
    /// </summary>
    public static void SplitPatternsByFamily(GameProfile profile)
    {
        ArgumentNullException.ThrowIfNull(profile);

        List<ProfilePattern> permanent = [];
        List<ProfilePattern> temporary = [];
        foreach (ProfilePattern pattern in profile.AllPatterns)
        {
            (pattern.Permanent ? permanent : temporary).Add(pattern);
        }

        profile.PermanentPatterns.Clear();
        foreach (ProfilePattern pattern in permanent)
        {
            profile.PermanentPatterns.Add(pattern);
        }

        profile.TemporaryPatterns.Clear();
        foreach (ProfilePattern pattern in temporary)
        {
            profile.TemporaryPatterns.Add(pattern);
        }
    }
}
