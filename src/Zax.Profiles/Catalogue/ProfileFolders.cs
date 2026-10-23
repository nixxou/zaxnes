namespace Zax.Profiles.Catalogue;

/// <summary>Which of the two scanned folders a user profile came from (AMD-033 §1).</summary>
public enum ProfileOrigin
{
    /// <summary>The user's profile folder, <c>&lt;user root&gt;/profiles</c> (REQ-PERS-040).</summary>
    User,

    /// <summary>The <c>profiles</c> folder next to the executable, which is only ever read.</summary>
    ProgramFolder,
}

/// <summary>
/// The two folders of profiles scanned at every start (AMD-033): the user's one and the
/// <c>profiles</c> folder next to the executable, merged by the rules of AMD-033 §2.
/// </summary>
/// <remarks>
/// <para>
/// The second folder is read in every mode — normal, portable and <c>--userdata</c> — and is
/// never written to, moved or deleted: imports keep going to the user folder alone (§1). A
/// folder that is missing or that cannot be listed is not an error; it simply yields nothing.
/// </para>
/// <para>
/// Merging, in order: a file of the <b>same name</b> as one of the user folder is skipped
/// without even being read; then the usual rules of <see cref="UserProfileFolder"/> apply, with
/// the user folder winning every tie. Two byte-identical files appear once and silently, while
/// two different files carrying the same identifier are reported once.
/// </para>
/// </remarks>
public sealed class ProfileFolders
{
    private readonly IProfileFileSystem _files;
    private readonly IProfileLog _log;

    /// <summary>Creates a view over the two folders.</summary>
    /// <param name="files">The file system to read through.</param>
    /// <param name="userDirectory">The user's profile folder, the one imports write to.</param>
    /// <param name="programDirectory">
    /// The <c>profiles</c> folder next to the executable, or <see langword="null"/> when there is
    /// none to scan. The same folder under both names counts as one.
    /// </param>
    /// <param name="log">Where skipped files are reported.</param>
    public ProfileFolders(
        IProfileFileSystem files,
        string userDirectory,
        string? programDirectory = null,
        IProfileLog? log = null)
    {
        ArgumentNullException.ThrowIfNull(files);
        ArgumentException.ThrowIfNullOrEmpty(userDirectory);

        _files = files;
        _log = log ?? NullProfileLog.Instance;
        UserDirectory = userDirectory;
        ProgramDirectory = string.IsNullOrEmpty(programDirectory)
            || IsSameDirectory(userDirectory, programDirectory)
                ? null
                : programDirectory;
    }

    /// <summary>The user's profile folder.</summary>
    public string UserDirectory { get; }

    /// <summary>The folder beside the executable, or null when there is nothing else to scan.</summary>
    public string? ProgramDirectory { get; }

    /// <summary>
    /// Scans both folders and returns the profiles to show, the user's ones first.
    /// </summary>
    /// <param name="reservedIds">
    /// Identifiers already taken, the built-in ones first: a built-in profile always wins
    /// (SPEC-09 REQ-PERS-030).
    /// </param>
    public IReadOnlyList<UserProfile> Scan(IReadOnlyCollection<string>? reservedIds = null)
    {
        UserProfileFolder user = new(_files, UserDirectory, _log);
        List<UserProfile> profiles = [.. user.Scan(reservedIds)];

        if (ProgramDirectory is null)
        {
            return profiles;
        }

        // The listing of the user folder, not the profiles it yielded: an unreadable or refused
        // file of the user folder still hides the one of the same name (AMD-033 §2). The scan is
        // flat, so the "relative path" of the rule is the file name.
        HashSet<string> userNames = new(StringComparer.OrdinalIgnoreCase);
        foreach (string path in ListFiles(UserDirectory))
        {
            userNames.Add(Path.GetFileName(path));
        }

        HashSet<string> taken = new(reservedIds ?? [], StringComparer.Ordinal);
        Dictionary<string, string> pathById = new(StringComparer.Ordinal);
        foreach (UserProfile profile in profiles)
        {
            taken.Add(profile.Id);
            pathById[profile.Id] = profile.FilePath;
        }

        UserProfileFolder program = new(_files, ProgramDirectory, _log, ProfileOrigin.ProgramFolder);
        List<string> paths = [.. ListFiles(ProgramDirectory)];
        paths.Sort(StringComparer.Ordinal);

        foreach (string path in paths)
        {
            if (userNames.Contains(Path.GetFileName(path)))
            {
                // Same name: the user's file alone counts, and this one is not even read.
                continue;
            }

            if (program.TryRead(path) is not { } profile)
            {
                continue;
            }

            if (!taken.Add(profile.Id))
            {
                // Byte-identical files are the same profile twice and appear once, with nothing
                // to report; two different files for one identifier are worth a line.
                if (!HaveSameContent(path, pathById.GetValueOrDefault(profile.Id)))
                {
                    _log.Warning(ProfileImportMessages.DuplicatedProfile(path, profile.Id));
                }

                continue;
            }

            pathById[profile.Id] = path;
            profiles.Add(profile);
        }

        return profiles;
    }

    /// <summary>
    /// The <c>*.3dn</c> files of a folder; empty when it is missing or cannot be listed, which
    /// AMD-033 §1 makes a normal case rather than an error.
    /// </summary>
    private IReadOnlyList<string> ListFiles(string directory)
    {
        try
        {
            return _files.ListFiles(directory, UserProfileFolder.Extension);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            _log.Warning($"{directory}: the profile folder could not be listed ({exception.Message})");
            return [];
        }
    }

    private bool HaveSameContent(string left, string? right)
    {
        if (right is null)
        {
            // The identifier belongs to a built-in profile, whose file is not ours to compare.
            return false;
        }

        try
        {
            return _files.ReadAllBytes(left).AsSpan().SequenceEqual(_files.ReadAllBytes(right));
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            return false;
        }
    }

    private static bool IsSameDirectory(string left, string? right)
        => right is not null
        && string.Equals(
            right.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar),
            left.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar),
            StringComparison.OrdinalIgnoreCase);
}
