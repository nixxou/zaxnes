namespace Zax.Profiles.Catalogue;

/// <summary>
/// The file operations the catalogue, the user profile folder and the game library need.
/// </summary>
/// <remarks>
/// Injected rather than used directly so that the library stays testable and free of ambient
/// state (REQ-ARCH-011, SPEC-09 REQ-PERS-082: reading and writing go through an abstract
/// storage interface provided by the host). Paths are plain strings and are combined with
/// <see cref="Path"/> by the callers.
/// </remarks>
public interface IProfileFileSystem
{
    /// <summary>True when the file exists.</summary>
    bool FileExists(string path);

    /// <summary>True when the directory exists.</summary>
    bool DirectoryExists(string path);

    /// <summary>Creates the directory and its parents; does nothing when it exists.</summary>
    void CreateDirectory(string path);

    /// <summary>
    /// The files of a directory whose name ends with <paramref name="extension"/>
    /// (case-insensitive), as full paths. An empty list when the directory is missing.
    /// </summary>
    IReadOnlyList<string> ListFiles(string directory, string extension);

    /// <summary>Reads a file whole.</summary>
    byte[] ReadAllBytes(string path);

    /// <summary>Reads a UTF-8 text file whole.</summary>
    string ReadAllText(string path);

    /// <summary>Writes a file, replacing it when it exists.</summary>
    void WriteAllBytes(string path, byte[] contents);

    /// <summary>Writes a UTF-8 text file, replacing it when it exists.</summary>
    void WriteAllText(string path, string contents);

    /// <summary>Deletes a file; does nothing when it is already gone.</summary>
    void DeleteFile(string path);

    /// <summary>Copies a file, optionally over an existing one.</summary>
    void CopyFile(string sourcePath, string destinationPath, bool overwrite);

    /// <summary>
    /// Moves or renames a file, optionally over an existing one. Used for the atomic write of
    /// SPEC-09 REQ-PERS-021 and for setting a corrupted file aside (REQ-PERS-037).
    /// </summary>
    void MoveFile(string sourcePath, string destinationPath, bool overwrite);
}

/// <summary>The real file system.</summary>
public sealed class PhysicalProfileFileSystem : IProfileFileSystem
{
    /// <summary>A shared instance; the type holds no state.</summary>
    public static PhysicalProfileFileSystem Instance { get; } = new();

    /// <inheritdoc/>
    public bool FileExists(string path) => File.Exists(path);

    /// <inheritdoc/>
    public bool DirectoryExists(string path) => Directory.Exists(path);

    /// <inheritdoc/>
    public void CreateDirectory(string path) => Directory.CreateDirectory(path);

    /// <inheritdoc/>
    public IReadOnlyList<string> ListFiles(string directory, string extension)
    {
        if (!Directory.Exists(directory))
        {
            return [];
        }

        List<string> files = [];
        foreach (string path in Directory.EnumerateFiles(directory))
        {
            if (path.EndsWith(extension, StringComparison.OrdinalIgnoreCase))
            {
                files.Add(path);
            }
        }

        return files;
    }

    /// <inheritdoc/>
    public byte[] ReadAllBytes(string path) => File.ReadAllBytes(path);

    /// <inheritdoc/>
    public string ReadAllText(string path) => File.ReadAllText(path);

    /// <inheritdoc/>
    public void WriteAllBytes(string path, byte[] contents) => File.WriteAllBytes(path, contents);

    /// <inheritdoc/>
    public void WriteAllText(string path, string contents) => File.WriteAllText(path, contents);

    /// <inheritdoc/>
    public void DeleteFile(string path)
    {
        if (File.Exists(path))
        {
            File.Delete(path);
        }
    }

    /// <inheritdoc/>
    public void CopyFile(string sourcePath, string destinationPath, bool overwrite)
        => File.Copy(sourcePath, destinationPath, overwrite);

    /// <inheritdoc/>
    public void MoveFile(string sourcePath, string destinationPath, bool overwrite)
        => File.Move(sourcePath, destinationPath, overwrite);
}

/// <summary>
/// Where the user's data lives (SPEC-01 REQ-ARCH-033, SPEC-09 REQ-PERS-070 to REQ-PERS-072).
/// </summary>
/// <remarks>
/// The root is a parameter: deciding it — <c>%LOCALAPPDATA%\Zaxnes</c>, the portable
/// <c>userdata\</c> folder or a command-line override — belongs to the application, not here.
/// </remarks>
public sealed class UserDataLayout
{
    /// <summary>The folder of the imported user profiles (SPEC-09 REQ-PERS-072).</summary>
    public const string DefaultProfilesFolderName = "profiles";

    /// <summary>The library file (SPEC-09 §4.5.2).</summary>
    public const string LibraryFileName = "library.json";

    /// <summary>The cover-art folder (SPEC-09 REQ-PERS-072).</summary>
    public const string CoversFolderName = "covers";

    /// <summary>
    /// Creates a layout under <paramref name="root"/>.
    /// </summary>
    /// <param name="root">The user data root.</param>
    /// <param name="profilesFolderName">
    /// Name of the user profile folder. It defaults to <c>profiles</c>, the name SPEC-09
    /// REQ-PERS-072 fixes for the reimplementation; SPEC-03 REQ-PROF-062 names the original's
    /// <c>CustomProfiles</c> instead, which is why this is a parameter (see Q-008).
    /// </param>
    public UserDataLayout(string root, string profilesFolderName = DefaultProfilesFolderName)
    {
        // Q-008: SPEC-03 REQ-PROF-062 names CustomProfiles, SPEC-09 REQ-PERS-072 names
        // profiles; the second is the normative layout of the reimplementation, and the name
        // stays a parameter until R0 answers.
        ArgumentException.ThrowIfNullOrEmpty(root);
        ArgumentException.ThrowIfNullOrEmpty(profilesFolderName);
        Root = root;
        ProfilesDirectory = Path.Combine(root, profilesFolderName);
        LibraryFile = Path.Combine(root, LibraryFileName);
        CoversDirectory = Path.Combine(root, CoversFolderName);
    }

    /// <summary>The user data root.</summary>
    public string Root { get; }

    /// <summary>Folder scanned for <c>*.3dn</c> files at start-up.</summary>
    public string ProfilesDirectory { get; }

    /// <summary>The library file.</summary>
    public string LibraryFile { get; }

    /// <summary>The cover-art folder.</summary>
    public string CoversDirectory { get; }

    /// <summary>Creates the folders that are missing (SPEC-09 REQ-PERS-072).</summary>
    public void EnsureDirectories(IProfileFileSystem files)
    {
        ArgumentNullException.ThrowIfNull(files);
        files.CreateDirectory(Root);
        files.CreateDirectory(ProfilesDirectory);
        files.CreateDirectory(CoversDirectory);
    }
}
