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
/// What an import <i>would</i> do, read from the file without writing anything
/// (<see cref="UserProfileFolder.Inspect"/>). AMD-056 REQ-UI-214 needs this: the replacement of
/// a user profile happens only once the confirmation dialog has been answered, so the outcome
/// has to be known before any file moves.
/// </summary>
/// <param name="Outcome">
/// What <see cref="UserProfileFolder.Apply"/> would do. Only <see cref="ProfileImportOutcome.Added"/>
/// and <see cref="ProfileImportOutcome.Replaced"/> write anything.
/// </param>
/// <param name="SourcePath">The chosen file, as it was given.</param>
/// <param name="Message">
/// The text of REQ-UI-084 that names the cause of a refusal, empty otherwise.
/// </param>
/// <param name="Id">The identifier the file carries, empty when it was refused.</param>
/// <param name="Title">
/// The title the library entry would take: field 8, or the file name <b>without</b> its
/// extension (REQ-PERS-034). Not the <c>&lt;titre&gt;</c> of the REQ-UI-214 dialogs, whose
/// fallback is the file name <i>with</i> its extension.
/// </param>
/// <param name="Header">The header that was read; <c>default</c> for a refusal.</param>
/// <param name="Replaced">
/// The user profile whose identifier this file takes over, when there is one — the entry whose
/// displayed name the confirmation of REQ-UI-214 names.
/// </param>
/// <param name="Destination">Where the file would be copied, empty when nothing would be.</param>
public sealed record ProfileImportPlan(
    ProfileImportOutcome Outcome,
    string SourcePath,
    string Message,
    string Id,
    string Title,
    ProfileHeader Header,
    UserProfile? Replaced,
    string Destination)
{
    /// <summary>Whether <see cref="UserProfileFolder.Apply"/> would write anything.</summary>
    public bool Writes => Outcome is ProfileImportOutcome.Added or ProfileImportOutcome.Replaced;
}

/// <summary>
/// The texts the original shows when an import fails. SPEC-10 REQ-UI-084 is the only normative
/// source for them, and one oddity survives its rewording by AMD-056 §2: the two spaces before
/// "is" in the version message.
/// </summary>
/// <remarks>
/// AMD-056 §1 reads two corrections out of the reference program 2.0 that AMD-053 had missed —
/// the version message carries the <b>sanitised</b> version and not the raw string of the file,
/// and the format message has a space after the path and spells « obsolete ». The « obsolate »
/// of the earlier reading was a misreading, not an oddity of the original.
/// </remarks>
public static class ProfileImportMessages
{
    /// <summary>Shown for a file whose signature is not <c>"3dn"</c>.</summary>
    public const string NotA3dnFile = "This file is not a 3dn file";

    /// <summary>
    /// Shown for a profile whose format version is not supported. The version is sanitised as
    /// REQ-PROF-021 sanitises it — digits, <c>.</c> and <c>$</c> only — so a file writing
    /// <c>"v1"</c> reads « 3dn format version 1 » (AMD-056 §2, which supersedes the answer of
    /// Q-089 on the form of <c>&lt;v&gt;</c>).
    /// </summary>
    /// <remarks>
    /// The sanitising happens here rather than at the call sites, so that every text carrying a
    /// version — the import of REQ-UI-212 and the load of REQ-PERS-040 — spells it the same way.
    /// The two spaces before "is" are the original's and stay.
    /// </remarks>
    public static string UnsupportedVersion(string version)
        => $"3dn format version {ProfileVersionRules.Sanitize(version)}  is not supported";

    /// <summary>Shown for a file without an identifier, or in a format too old to use.</summary>
    /// <remarks>
    /// AMD-029 §3, closing Q-068: the last words name the product, « Zaxnes ». AMD-056 §2 adds
    /// the space after the path and the correct spelling of « obsolete ».
    /// </remarks>
    public static string ObsoleteFormat(string path)
        => $"{path} has an obsolete or wrong profile format. "
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
    /// Extension 3dSen 2.1 gives a « 3dh » profile (AMD-062 REQ-PROF-062). Both extensions are
    /// scanned; which format a file is in is decided by its <b>content</b>, never by its name
    /// (REQ-PROF-070), so a « 3dh » profile named <c>.3dn</c> — which is what the import from an
    /// installation writes — is read just the same.
    /// </summary>
    public const string HashExtension = ".3dh";

    /// <summary>The extensions a profile folder is scanned for (REQ-PROF-062).</summary>
    public static IReadOnlyList<string> Extensions { get; } = [Extension, HashExtension];

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
        List<string> paths = [];
        foreach (string extension in Extensions)
        {
            paths.AddRange(_files.ListFiles(Directory, extension));
        }

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
    /// Imports the file at <paramref name="sourcePath"/> into the folder: the reading of
    /// <see cref="Inspect"/> followed at once by the writing of <see cref="Apply"/>.
    /// </summary>
    /// <param name="sourcePath">The chosen file.</param>
    /// <param name="builtInIds">Identifiers of the built-in profiles.</param>
    /// <param name="userProfiles">The user profiles already known, from <see cref="Scan"/>.</param>
    /// <remarks>
    /// This is the <b>non-interactive</b> import: it replaces an existing user profile without
    /// asking. The interface no longer takes this path — AMD-056 REQ-UI-214 has it inspect, put
    /// up a dialog, and apply only once the dialog says so — but everything with nowhere to show
    /// a dialog still does, the acceptance harness included.
    /// </remarks>
    public ProfileImportResult Import(
        string sourcePath,
        IReadOnlyCollection<string>? builtInIds = null,
        IReadOnlyList<UserProfile>? userProfiles = null)
        => Apply(Inspect(sourcePath, builtInIds, userProfiles));

    /// <summary>
    /// Reads the file at <paramref name="sourcePath"/> and says what importing it would do,
    /// <b>without writing anything</b> (AMD-056 REQ-UI-214).
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
    public ProfileImportPlan Inspect(
        string sourcePath,
        IReadOnlyCollection<string>? builtInIds = null,
        IReadOnlyList<UserProfile>? userProfiles = null)
    {
        ArgumentException.ThrowIfNullOrEmpty(sourcePath);

        if (!_files.FileExists(sourcePath))
        {
            return Refusal(sourcePath, ProfileImportMessages.ObsoleteFormat(sourcePath));
        }

        ProfileHeader? header = ProfileFormat.LoadHeader(
            _files.ReadAllBytes(sourcePath), _log, sourcePath);
        if (header is null)
        {
            // Row 3 of the table: a malformed protobuf, or a file that could not be read at all.
            return Refusal(sourcePath, ProfileImportMessages.ObsoleteFormat(sourcePath));
        }

        if (!header.Value.HasAcceptableSignature)
        {
            // Row 1: `Type` is present and is not "3dn" (REQ-PROF-033).
            return Refusal(sourcePath, ProfileImportMessages.NotA3dnFile);
        }

        if (header.Value.HasUnsupportedVersion)
        {
            // AMD-062 REQ-PROF-063: the causes of a refused « 3dh » are those of REQ-PROF-072 —
            // here an envelope version outside "1", "2" and "5" — and the cause shown is the
            // third text of REQ-UI-084, as for a 3dn that cannot be read. The 3dn version
            // message would name a format version the file does not have.
            if (header.Value.IsHashEnvelope)
            {
                return Refusal(sourcePath, ProfileImportMessages.ObsoleteFormat(sourcePath));
            }

            // Row 2: `Ver` reduces to "1" (REQ-PROF-021). The message carries the version as the
            // sanitising of REQ-PROF-021 reduces it, which AMD-056 §2 read in the 2.0 and which
            // supersedes the answer of Q-089 on this point.
            return Refusal(
                sourcePath,
                ProfileImportMessages.UnsupportedVersion(header.Value.Version ?? string.Empty));
        }

        // Row 4: an absent or empty identifier is refused — there would be no way to key the
        // library entry, the slots or the cover art on it (REQ-PROF-063, REQ-PERS-041 point 1).
        if (!header.Value.HasId)
        {
            return Refusal(sourcePath, ProfileImportMessages.ObsoleteFormat(sourcePath));
        }

        string id = header.Value.Id!;
        if (builtInIds is not null && builtInIds.Contains(id))
        {
            return new ProfileImportPlan(
                ProfileImportOutcome.AlreadyBuiltIn,
                sourcePath,
                string.Empty,
                id,
                TitleOf(header.Value, sourcePath),
                header.Value,
                null,
                string.Empty);
        }

        string destination = Path.Combine(Directory, Path.GetFileName(sourcePath));
        UserProfile? replaced = FindById(userProfiles, id);
        return new ProfileImportPlan(
            replaced is null ? ProfileImportOutcome.Added : ProfileImportOutcome.Replaced,
            sourcePath,
            string.Empty,
            id,
            TitleOf(header.Value, destination),
            header.Value,
            replaced,
            destination);
    }

    /// <summary>
    /// Carries out what <see cref="Inspect"/> planned: the old file of a taken-over identifier
    /// goes, and the chosen file is copied into the folder. A plan that writes nothing — a
    /// refusal, or a built-in identifier — is only logged.
    /// </summary>
    /// <remarks>
    /// REQ-PERS-041 keeps the fact logged in every case, which is why the two outcomes that do
    /// nothing are logged here rather than in <see cref="Inspect"/>: inspecting is what the
    /// interface does to choose a dialog, and it may do it without importing anything.
    /// </remarks>
    public ProfileImportResult Apply(ProfileImportPlan plan)
    {
        ArgumentNullException.ThrowIfNull(plan);

        switch (plan.Outcome)
        {
            case ProfileImportOutcome.Rejected:
                _log.Warning(plan.Message);
                return new ProfileImportResult(ProfileImportOutcome.Rejected, null, plan.Message);

            case ProfileImportOutcome.AlreadyBuiltIn:
                _log.Information(
                    $"{plan.SourcePath}: the identifier \"{plan.Id}\" belongs to a built-in "
                    + "profile; nothing imported");
                return new ProfileImportResult(
                    ProfileImportOutcome.AlreadyBuiltIn, null, string.Empty);
        }

        if (plan.Replaced is { } replaced
            && !string.Equals(replaced.FilePath, plan.Destination, StringComparison.OrdinalIgnoreCase))
        {
            // Same identifier under another file name: the old file goes away.
            _files.DeleteFile(replaced.FilePath);
        }

        _files.CreateDirectory(Directory);
        if (!string.Equals(plan.SourcePath, plan.Destination, StringComparison.OrdinalIgnoreCase))
        {
            _files.CopyFile(plan.SourcePath, plan.Destination, overwrite: true);
        }

        UserProfile imported = new(plan.Id, plan.Title, plan.Destination, plan.Header);
        _log.Information(
            $"{plan.SourcePath}: profile \"{plan.Id}\" "
            + (plan.Outcome == ProfileImportOutcome.Replaced ? "replaced" : "imported"));
        return new ProfileImportResult(plan.Outcome, imported, string.Empty);
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

    private static ProfileImportPlan Refusal(string sourcePath, string message)
        => new(
            ProfileImportOutcome.Rejected,
            sourcePath,
            message,
            string.Empty,
            string.Empty,
            default,
            null,
            string.Empty);
}
