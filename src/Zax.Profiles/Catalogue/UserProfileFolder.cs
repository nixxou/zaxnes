using System.Security.Cryptography;

namespace Zax.Profiles.Catalogue;

/// <summary>A user profile found in the user profile folder (SPEC-03 REQ-PROF-062).</summary>
/// <param name="Id">
/// Identifier: field 7 of the file, or — when that field is absent or empty — the identifier
/// derived from the bytes of the file (AMD-037 §2,
/// <see cref="UserProfileFolder.DerivedIdentifier"/>).
/// </param>
/// <param name="Title">
/// Title: field 8, or the file name without its extension when field 8 is empty or absent.
/// </param>
/// <param name="FilePath">Full path of the <c>.3dn</c> file.</param>
/// <param name="Header">The header fields that were read.</param>
/// <param name="Origin">
/// Which of the two scanned folders the file came from (AMD-033 §2). It defaults to
/// <see cref="ProfileOrigin.User"/>, the only folder that existed before.
/// </param>
/// <param name="IdIsDerived">
/// True when <paramref name="Id"/> was derived from the content because the file carries none
/// (AMD-037 §2). The file is never rewritten, so the identifier is computed again at every
/// read.
/// </param>
public sealed record UserProfile(
    string Id,
    string Title,
    string FilePath,
    ProfileHeader Header,
    ProfileOrigin Origin = ProfileOrigin.User,
    bool IdIsDerived = false);

/// <summary>What an import did (SPEC-03 REQ-PROF-063, SPEC-09 REQ-PERS-041).</summary>
public enum ProfileImportOutcome
{
    /// <summary>The identifier was new: the file was copied and a game added.</summary>
    Added,

    /// <summary>
    /// The identifier belonged to a user profile: its file was deleted and replaced, and its
    /// library entry (ROM, title, date, cover) is kept.
    /// </summary>
    Replaced,

    /// <summary>The identifier belongs to a built-in profile: nothing was done.</summary>
    AlreadyBuiltIn,

    /// <summary>The file was refused: unreadable, not a profile, or without an identifier.</summary>
    Rejected,
}

/// <summary>The outcome of an import.</summary>
/// <param name="Outcome">What was done.</param>
/// <param name="Profile">The imported profile, when one was added or replaced.</param>
/// <param name="Message">The message to show, empty when there is nothing to say.</param>
public sealed record ProfileImportResult(ProfileImportOutcome Outcome, UserProfile? Profile, string Message);

/// <summary>
/// The texts the original shows when an import fails. SPEC-10 REQ-UI-084 is the only normative
/// source for them, oddities included: two spaces before "is" in the version message, and no
/// space between the path and "has" in the format message.
/// </summary>
public static class ProfileImportMessages
{
    /// <summary>Shown for a file whose signature is not <c>"3dn"</c>.</summary>
    public const string NotA3dnFile = "This file is not a 3dn file";

    /// <summary>Shown for a profile whose format version is not supported.</summary>
    public static string UnsupportedVersion(string version)
        => $"3dn format version {version}  is not supported";

    /// <summary>Shown for a file without an identifier, or in a format too old to use.</summary>
    /// <remarks>
    /// AMD-029 §3, closing Q-068: the last words name the product, « Zaxnes ». That is the one
    /// change R0 made to the text REQ-UI-084 fixes word for word; the two oddities above are
    /// kept.
    /// </remarks>
    public static string ObsoleteFormat(string path)
        => $"{path}has an obsolate or wrong profile format. "
        + "This profile is not compatible with the current version of Zaxnes.";

    /// <summary>Logged when a profile is skipped because its identifier is already taken.</summary>
    public static string DuplicatedProfile(string path, string id)
        => $"{path}: duplicated profile, the identifier \"{id}\" is already in the library";

    /// <summary>
    /// Logged once per file when a scanned profile carries no identifier and takes the one
    /// derived from its content instead of being ignored (AMD-037 §2).
    /// </summary>
    public static string DerivedIdentifier(string path, string id)
        => $"{path}: the profile carries no identifier; using \"{id}\", derived from its content";

    /// <summary>
    /// Logged once per file when a user profile is, byte for byte, a built-in profile of the
    /// catalogue in use: it adds no game (AMD-037 §2).
    /// </summary>
    public static string SameAsBuiltIn(string path, string id)
        => $"{path}: identical to the built-in profile \"{id}\"; no game added";
}

/// <summary>
/// The folder of imported user profiles: the <c>*.3dn</c> scan at start-up (SPEC-03
/// REQ-PROF-062, SPEC-09 REQ-PERS-040) and the import of a chosen file (REQ-PROF-063,
/// REQ-PERS-041).
/// </summary>
/// <remarks>
/// <para>
/// Only the header fields of each file are read, which is all a library entry needs; the
/// patterns, the scripts and the settings are skipped. A file that cannot be read, that is not
/// a profile or whose identifier is already in the library is ignored and logged, and never
/// stops the program from starting.
/// </para>
/// <para>
/// A file carrying no identifier is <b>not</b> ignored: it takes the identifier derived from its
/// content (AMD-037 §2, <see cref="DerivedIdentifier"/>). Only the import of a chosen file still
/// refuses it (REQ-PROF-063).
/// </para>
/// </remarks>
public sealed class UserProfileFolder
{
    /// <summary>Extension of a user profile file.</summary>
    public const string Extension = ".3dn";

    /// <summary>
    /// How many bytes of the SHA-256 digest a derived identifier encodes (AMD-037 §2): nine
    /// bytes, which is twelve base64 characters and no padding.
    /// </summary>
    public const int DerivedIdentifierBytes = 9;

    private readonly IProfileFileSystem _files;
    private readonly IProfileLog _log;
    private readonly IBuiltInProfileIndex? _builtIns;

    /// <summary>Creates a view of the folder at <paramref name="directory"/>.</summary>
    /// <param name="files">The file system to read through.</param>
    /// <param name="directory">The folder to scan.</param>
    /// <param name="log">Where rejected files are reported.</param>
    /// <param name="origin">
    /// What the profiles of this folder are tagged with (AMD-033 §2). Only
    /// <see cref="ProfileFolders"/> passes <see cref="ProfileOrigin.ProgramFolder"/>; the folder
    /// of the executable is read-only, so <see cref="Import"/> and <see cref="Delete"/> are
    /// never used on it.
    /// </param>
    /// <param name="builtIns">
    /// The profile files of the catalogue in use, so that a user file which is one of them
    /// byte for byte adds no game (AMD-037 §2). Null when there is nothing to compare with.
    /// </param>
    public UserProfileFolder(
        IProfileFileSystem files,
        string directory,
        IProfileLog? log = null,
        ProfileOrigin origin = ProfileOrigin.User,
        IBuiltInProfileIndex? builtIns = null)
    {
        ArgumentNullException.ThrowIfNull(files);
        ArgumentException.ThrowIfNullOrEmpty(directory);
        _files = files;
        Directory = directory;
        _log = log ?? NullProfileLog.Instance;
        Origin = origin;
        _builtIns = builtIns;
    }

    /// <summary>
    /// The identifier derived from the bytes of a profile file (AMD-037 §2): the first nine
    /// bytes of their SHA-256 digest in standard base64, which is twelve characters of
    /// <c>A–Z a–z 0–9 + /</c> with no padding — the format of REQ-PROF-052.
    /// </summary>
    /// <remarks>
    /// It is a pure function of the content: the file is never rewritten, and the same file read
    /// again at the next start yields the same identifier. A catalogue identifier can never be
    /// mistaken for one of these, being one to three digits long (AMD-037 §3).
    /// </remarks>
    public static string DerivedIdentifier(ReadOnlySpan<byte> bytes)
    {
        Span<byte> digest = stackalloc byte[SHA256.HashSizeInBytes];
        SHA256.HashData(bytes, digest);
        return Convert.ToBase64String(digest[..DerivedIdentifierBytes]);
    }

    /// <summary>The folder.</summary>
    public string Directory { get; }

    /// <summary>What the profiles found here are tagged with (AMD-033 §2).</summary>
    public ProfileOrigin Origin { get; }

    /// <summary>
    /// Reads every <c>*.3dn</c> of the folder and keeps the ones with a usable header and an
    /// identifier — carried or derived (AMD-037 §2) — that is still free.
    /// </summary>
    /// <param name="reservedIds">
    /// Identifiers already taken, the built-in ones first: a built-in profile always wins over
    /// a user profile (SPEC-09 REQ-PERS-030).
    /// </param>
    /// <remarks>
    /// The files are visited in ordinal order of their names so that, between two user profiles
    /// with the same identifier, the winner does not depend on the file system
    /// (REQ-PROF-062, notes).
    /// </remarks>
    public IReadOnlyList<UserProfile> Scan(IReadOnlyCollection<string>? reservedIds = null)
    {
        List<string> paths = [.. _files.ListFiles(Directory, Extension)];
        paths.Sort(StringComparer.Ordinal);

        HashSet<string> taken = new(reservedIds ?? [], StringComparer.Ordinal);
        List<UserProfile> profiles = [];
        foreach (string path in paths)
        {
            UserProfile? profile = TryRead(path);
            if (profile is null)
            {
                continue;
            }

            if (!taken.Add(profile.Id))
            {
                _log.Warning(ProfileImportMessages.DuplicatedProfile(path, profile.Id));
                continue;
            }

            profiles.Add(profile);
        }

        return profiles;
    }

    /// <summary>
    /// Imports the file at <paramref name="sourcePath"/> into the folder.
    /// </summary>
    /// <param name="sourcePath">The chosen file.</param>
    /// <param name="builtInIds">Identifiers of the built-in profiles.</param>
    /// <param name="userProfiles">The user profiles already known, from <see cref="Scan"/>.</param>
    /// <remarks>
    /// AMD-053 REQ-UI-212: the file is read as REQ-PROF-062 reads one — the header alone, no
    /// pattern and no script — and the message of a refusal is chosen by its <b>cause</b>. The
    /// four rows of the table are, in the order they can be told apart: an unreadable file or a
    /// malformed protobuf, a signature that is not <c>"3dn"</c>, an unsupported version, and a
    /// missing identifier.
    /// </remarks>
    public ProfileImportResult Import(
        string sourcePath,
        IReadOnlyCollection<string>? builtInIds = null,
        IReadOnlyList<UserProfile>? userProfiles = null)
    {
        ArgumentException.ThrowIfNullOrEmpty(sourcePath);

        if (!_files.FileExists(sourcePath))
        {
            return Reject(ProfileImportMessages.ObsoleteFormat(sourcePath));
        }

        ProfileHeader? header = ProfileFormat.LoadHeader(
            _files.ReadAllBytes(sourcePath), _log, sourcePath);
        if (header is null)
        {
            // Row 3 of the table: a malformed protobuf, or a file that could not be read at all.
            return Reject(ProfileImportMessages.ObsoleteFormat(sourcePath));
        }

        if (!header.Value.HasAcceptableSignature)
        {
            // Row 1: `Type` is present and is not "3dn" (REQ-PROF-033).
            return Reject(ProfileImportMessages.NotA3dnFile);
        }

        if (header.Value.HasUnsupportedVersion)
        {
            // Row 2: `Ver` reduces to "1" (REQ-PROF-021). The message carries the version as
            // the file spells it, not as the sanitising of REQ-PROF-021 reduces it. Q-089.
            return Reject(ProfileImportMessages.UnsupportedVersion(header.Value.Version ?? string.Empty));
        }

        // Row 4: an absent or empty identifier is refused — there would be no way to key the
        // library entry, the slots or the cover art on it (REQ-PROF-063, REQ-PERS-041 point 1).
        if (!header.Value.HasId)
        {
            return Reject(ProfileImportMessages.ObsoleteFormat(sourcePath));
        }

        string id = header.Value.Id!;
        if (builtInIds is not null && builtInIds.Contains(id))
        {
            _log.Information(
                $"{sourcePath}: the identifier \"{id}\" belongs to a built-in profile; nothing imported");
            return new ProfileImportResult(ProfileImportOutcome.AlreadyBuiltIn, null, string.Empty);
        }

        string destination = Path.Combine(Directory, Path.GetFileName(sourcePath));
        UserProfile? replaced = FindById(userProfiles, id);
        if (replaced is not null
            && !string.Equals(replaced.FilePath, destination, StringComparison.OrdinalIgnoreCase))
        {
            // Same identifier under another file name: the old file goes away.
            _files.DeleteFile(replaced.FilePath);
        }

        _files.CreateDirectory(Directory);
        if (!string.Equals(sourcePath, destination, StringComparison.OrdinalIgnoreCase))
        {
            _files.CopyFile(sourcePath, destination, overwrite: true);
        }

        UserProfile imported = new(
            id,
            TitleOf(header.Value, destination),
            destination,
            header.Value);
        return new ProfileImportResult(
            replaced is null ? ProfileImportOutcome.Added : ProfileImportOutcome.Replaced,
            imported,
            string.Empty);
    }

    /// <summary>Deletes the file of a user profile (REQ-PROF-065, SPEC-09 REQ-PERS-042).</summary>
    public void Delete(UserProfile profile)
    {
        ArgumentNullException.ThrowIfNull(profile);
        _files.DeleteFile(profile.FilePath);
    }

    /// <summary>
    /// Reads the header of one file of the folder and makes a profile of it, or returns
    /// <see langword="null"/> when the file is unreadable, is not a profile, or is byte for byte
    /// a built-in profile of the catalogue — each of which is logged, never thrown
    /// (REQ-PROF-062, AMD-037 §2).
    /// </summary>
    /// <remarks>
    /// <para>
    /// A file without an identifier is kept and takes the one derived from its content
    /// (AMD-037 §2); it is no longer a reason to return <see langword="null"/>.
    /// </para>
    /// <para>
    /// Public so that <see cref="ProfileFolders"/> can apply the merge rules of AMD-033 §2
    /// file by file without repeating the reading rules.
    /// </para>
    /// </remarks>
    public UserProfile? TryRead(string path)
    {
        ArgumentException.ThrowIfNullOrEmpty(path);

        byte[] bytes;
        try
        {
            bytes = _files.ReadAllBytes(path);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            _log.Warning($"{path}: unreadable file ({exception.Message})");
            return null;
        }

        ProfileHeader? header = ProfileFormat.LoadHeader(bytes, _log, path);
        if (header is null)
        {
            return null;
        }

        if (!header.Value.HasAcceptableSignature)
        {
            _log.Warning($"{path}: ignored, {ProfileImportMessages.NotA3dnFile}");
            return null;
        }

        // AMD-037 §2: a user file that is a built-in profile of the catalogue in use adds no
        // game — the built-in one is shown once, with its catalogue metadata. True with or
        // without an identifier, and checked before one is derived.
        if (_builtIns?.FindByContent(bytes) is { } builtInId)
        {
            _log.Information(ProfileImportMessages.SameAsBuiltIn(path, builtInId));
            return null;
        }

        // AMD-037 §2: no identifier in the file is no longer a reason to ignore the game; the
        // identifier comes from the content instead, and the file is left untouched.
        bool derived = !header.Value.HasId;
        string id = derived ? DerivedIdentifier(bytes) : header.Value.Id!;
        if (derived)
        {
            _log.Information(ProfileImportMessages.DerivedIdentifier(path, id));
        }

        return new UserProfile(id, TitleOf(header.Value, path), path, header.Value, Origin, derived);
    }

    private static UserProfile? FindById(IReadOnlyList<UserProfile>? profiles, string id)
    {
        if (profiles is null)
        {
            return null;
        }

        foreach (UserProfile profile in profiles)
        {
            if (string.Equals(profile.Id, id, StringComparison.Ordinal))
            {
                return profile;
            }
        }

        return null;
    }

    private static string TitleOf(ProfileHeader header, string path)
        => string.IsNullOrEmpty(header.Title)
            ? Path.GetFileNameWithoutExtension(path)
            : header.Title;

    private ProfileImportResult Reject(string message)
    {
        _log.Warning(message);
        return new ProfileImportResult(ProfileImportOutcome.Rejected, null, message);
    }
}
