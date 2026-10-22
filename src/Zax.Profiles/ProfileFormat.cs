using Zax.Profiles.Wire;

namespace Zax.Profiles;

/// <summary>
/// A decoded profile together with what the writer needs to rewrite it byte for byte.
/// </summary>
public sealed class ProfileDocument
{
    /// <summary>Creates a document.</summary>
    public ProfileDocument(GameProfile profile, ProfileWireLayout layout, int unknownFieldCount)
    {
        ArgumentNullException.ThrowIfNull(profile);
        ArgumentNullException.ThrowIfNull(layout);
        Profile = profile;
        Layout = layout;
        UnknownFieldCount = unknownFieldCount;
    }

    /// <summary>The profile.</summary>
    public GameProfile Profile { get; }

    /// <summary>The field sets its file carried (REQ-PROF-051).</summary>
    public ProfileWireLayout Layout { get; }

    /// <summary>
    /// How many fields outside the schema were met and skipped. The 101 built-in profiles
    /// contain none (REQ-PROF-001).
    /// </summary>
    public int UnknownFieldCount { get; }

    /// <summary>Writes the profile back with the canonical writer.</summary>
    public byte[] Save() => ProfileWireCodec.Encode(Profile, Layout);
}

/// <summary>
/// The header fields of a profile file, the subset a library entry needs
/// (SPEC-03 REQ-PROF-062, §4.1).
/// </summary>
/// <param name="Signature">Field 1, which must be <c>"3dn"</c>.</param>
/// <param name="Id">Field 7.</param>
/// <param name="Title">Field 8.</param>
/// <param name="Author">Field 9, author of the profile.</param>
/// <param name="Publisher">Field 10, publisher of the game.</param>
/// <param name="Description">Field 11.</param>
public readonly record struct ProfileHeader(
    string? Signature,
    string? Id,
    string? Title,
    string? Author,
    string? Publisher,
    string? Description)
{
    /// <summary>
    /// True when the signature is acceptable: <c>"3dn"</c>, or absent, which is not an error
    /// (REQ-PROF-020).
    /// </summary>
    public bool HasAcceptableSignature
        => Signature is null || string.Equals(Signature, GameProfile.RequiredSignature, StringComparison.Ordinal);

    /// <summary>True when the file carries a non-empty identifier, which an import requires.</summary>
    public bool HasId => !string.IsNullOrEmpty(Id);
}

/// <summary>Why a profile was refused (SPEC-03 REQ-PROF-033).</summary>
public enum ProfileLoadError
{
    /// <summary>The profile loaded.</summary>
    None = 0,

    /// <summary>Malformed protobuf, truncated file, unknown reference key, key out of sequence.</summary>
    Malformed,

    /// <summary>The signature field is present and is not <c>"3dn"</c> (REQ-PROF-020).</summary>
    WrongSignature,

    /// <summary>The version is <c>"1"</c>, which this format does not support (REQ-PROF-021).</summary>
    UnsupportedVersion,

    /// <summary>The settings message is missing (REQ-PROF-025).</summary>
    MissingSettings,

    /// <summary>Light, clipping or layers are missing from the settings (REQ-PROF-025).</summary>
    IncompleteSettings,

    /// <summary>The settings do not hold exactly six layers (REQ-PROF-043).</summary>
    WrongLayerCount,
}

/// <summary>The outcome of loading a profile: all of it, or none of it.</summary>
public sealed class ProfileLoadResult
{
    private ProfileLoadResult(ProfileDocument? document, ProfileLoadError error, string? message)
    {
        Document = document;
        Error = error;
        Message = message;
    }

    /// <summary>The decoded profile, or <see langword="null"/> when it was refused.</summary>
    public ProfileDocument? Document { get; }

    /// <summary>Why it was refused, or <see cref="ProfileLoadError.None"/>.</summary>
    public ProfileLoadError Error { get; }

    /// <summary>What to write in the log when it was refused.</summary>
    public string? Message { get; }

    /// <summary>True when the profile loaded.</summary>
    public bool IsSuccess => Document is not null;

    /// <summary>The profile, or <see langword="null"/> when it was refused.</summary>
    public GameProfile? Profile => Document?.Profile;

    /// <summary>
    /// The profile, or the empty profile the game falls back to so that emulation carries on
    /// without a specific 3D conversion (REQ-PROF-033).
    /// </summary>
    public GameProfile ProfileOrEmpty() => Document?.Profile ?? GameProfile.CreateEmpty();

    internal static ProfileLoadResult Success(ProfileDocument document)
        => new(document, ProfileLoadError.None, null);

    internal static ProfileLoadResult Failure(ProfileLoadError error, string message)
        => new(null, error, message);
}

/// <summary>Where the library takes its log lines (REQ-PROF-033, REQ-PROF-062).</summary>
public interface IProfileLog
{
    /// <summary>Notes something worth knowing.</summary>
    void Information(string message);

    /// <summary>Notes a file that was ignored or a value that was adjusted.</summary>
    void Warning(string message);

    /// <summary>Notes a file that was refused.</summary>
    void Error(string message);
}

/// <summary>A log that drops everything, the default when none is given.</summary>
public sealed class NullProfileLog : IProfileLog
{
    /// <summary>The single instance.</summary>
    public static NullProfileLog Instance { get; } = new();

    private NullProfileLog()
    {
    }

    /// <inheritdoc/>
    public void Information(string message)
    {
    }

    /// <inheritdoc/>
    public void Warning(string message)
    {
    }

    /// <inheritdoc/>
    public void Error(string message)
    {
    }
}

/// <summary>
/// Loading and saving profiles, with the rules a running program needs: the version rules of
/// REQ-PROF-021, the settings requirements of REQ-PROF-025 and REQ-PROF-043, the script order
/// of REQ-PROF-024, and the all-or-nothing refusal of REQ-PROF-033 — a bad file never throws
/// and never leaves half a profile behind.
/// </summary>
public static class ProfileFormat
{
    /// <summary>
    /// Loads a profile. On failure the result carries the reason and a message, the log is
    /// written, and nothing of the file is kept.
    /// </summary>
    public static ProfileLoadResult Load(
        ReadOnlySpan<byte> bytes,
        IProfileLog? log = null,
        string? sourceName = null)
    {
        log ??= NullProfileLog.Instance;
        string source = sourceName ?? "<profile>";
        ProfileLoadResult result = LoadCore(bytes, source);
        if (!result.IsSuccess)
        {
            log.Error($"{source}: {result.Message}");
        }

        return result;
    }

    /// <summary>Writes a profile with the canonical writer (REQ-PROF-051).</summary>
    public static byte[] Save(GameProfile profile, ProfileWireLayout? layout = null)
        => ProfileWireCodec.Encode(profile, layout);

    /// <summary>
    /// Reads the header of a profile file (REQ-PROF-062), or returns <see langword="null"/>
    /// when the bytes are unreadable. A file whose signature is wrong is reported through
    /// <see cref="ProfileHeader.HasAcceptableSignature"/>.
    /// </summary>
    public static ProfileHeader? LoadHeader(
        ReadOnlySpan<byte> bytes,
        IProfileLog? log = null,
        string? sourceName = null)
    {
        try
        {
            return ProfileWireCodec.DecodeHeader(bytes);
        }
        catch (Exception exception) when (IsDecodeFailure(exception))
        {
            (log ?? NullProfileLog.Instance).Warning(
                $"{sourceName ?? "<profile>"}: unreadable profile header ({exception.Message}).");
            return null;
        }
    }

    private static ProfileLoadResult LoadCore(ReadOnlySpan<byte> bytes, string source)
    {
        ProfileDocument document;
        try
        {
            document = ProfileWireCodec.Decode(bytes);
        }
        catch (Exception exception) when (IsDecodeFailure(exception))
        {
            return ProfileLoadResult.Failure(
                ProfileLoadError.Malformed, $"not a readable profile ({exception.Message})");
        }

        GameProfile profile = document.Profile;

        if (profile.Signature is not null
            && !string.Equals(profile.Signature, GameProfile.RequiredSignature, StringComparison.Ordinal))
        {
            return ProfileLoadResult.Failure(
                ProfileLoadError.WrongSignature,
                $"signature \"{profile.Signature}\" is not \"{GameProfile.RequiredSignature}\"");
        }

        if (profile.FormatVersion == ProfileFormatVersion.Unsupported)
        {
            return ProfileLoadResult.Failure(
                ProfileLoadError.UnsupportedVersion,
                $"format version \"{profile.VersionText}\" is not supported");
        }

        if (profile.Settings is not { } settings)
        {
            return ProfileLoadResult.Failure(
                ProfileLoadError.MissingSettings, "the embedded settings are missing");
        }

        if (settings.Light is null || settings.Clipping is null || settings.Layers is null)
        {
            return ProfileLoadResult.Failure(
                ProfileLoadError.IncompleteSettings,
                "the embedded settings lack the light, the clipping or the layers");
        }

        if (settings.Layers.Layers.Count != LayerSettingsSet.LayerCount)
        {
            return ProfileLoadResult.Failure(
                ProfileLoadError.WrongLayerCount,
                $"the embedded settings hold {settings.Layers.Layers.Count} layers, not {LayerSettingsSet.LayerCount}");
        }

        // DEC-03-Q03: a profile without the editor's memorised slots is accepted and the five
        // default slots are rebuilt. No built-in profile is in that case.
        if (settings.SavedPatterns.Count == 0)
        {
            foreach (RecordedPatternSlot slot in RecordedPatternSlot.DefaultShapeSlots())
            {
                settings.SavedPatterns.Add(slot);
            }
        }

        if (settings.SavedAnimPatterns.Count == 0)
        {
            foreach (RecordedPatternSlot slot in RecordedPatternSlot.DefaultDeformSlots())
            {
                settings.SavedAnimPatterns.Add(slot);
            }
        }

        profile.SortScripts();
        return ProfileLoadResult.Success(document);
    }

    private static bool IsDecodeFailure(Exception exception)
        => exception is ProfileFormatException
        or IndexOutOfRangeException
        or ArgumentException
        or OverflowException
        or InvalidOperationException;
}
