using Zax.Profiles.Catalogue;

namespace Zax.Profiles.Tests;

/// <summary>A log that keeps every line, so a test can check what was reported.</summary>
internal sealed class RecordingProfileLog : IProfileLog
{
    public List<string> Informations { get; } = [];

    public List<string> Warnings { get; } = [];

    public List<string> Errors { get; } = [];

    public void Information(string message) => Informations.Add(message);

    public void Warning(string message) => Warnings.Add(message);

    public void Error(string message) => Errors.Add(message);

    public bool AnyContains(string fragment)
    {
        foreach (List<string> lines in (List<string>[])[Informations, Warnings, Errors])
        {
            foreach (string line in lines)
            {
                if (line.Contains(fragment, StringComparison.Ordinal))
                {
                    return true;
                }
            }
        }

        return false;
    }
}

/// <summary>
/// An in-memory file system, so the catalogue, the user profile folder and the library can be
/// tested without touching the disk.
/// </summary>
internal sealed class FakeProfileFileSystem : IProfileFileSystem
{
    private readonly Dictionary<string, byte[]> _files = new(StringComparer.OrdinalIgnoreCase);
    private readonly HashSet<string> _directories = new(StringComparer.OrdinalIgnoreCase);

    public IReadOnlyCollection<string> Paths => _files.Keys;

    /// <summary>
    /// Every file read, in order, so that a test can check a file was <b>not</b> read — the rule
    /// of AMD-033 §2 for a file hidden by one of the same name in the user folder.
    /// </summary>
    public List<string> Reads { get; } = [];

    /// <summary>
    /// Folders that refuse to be listed, standing in for a profile folder the account cannot
    /// read (AMD-033 §1).
    /// </summary>
    public HashSet<string> Unreadable { get; } = new(StringComparer.OrdinalIgnoreCase);

    public void AddFile(string path, byte[] contents)
    {
        _files[path] = contents;
        AddParentDirectories(path);
    }

    public void AddText(string path, string contents)
        => AddFile(path, System.Text.Encoding.UTF8.GetBytes(contents));

    public bool FileExists(string path) => _files.ContainsKey(path);

    public bool DirectoryExists(string path) => _directories.Contains(Normalise(path));

    public void CreateDirectory(string path) => _directories.Add(Normalise(path));

    public IReadOnlyList<string> ListFiles(string directory, string extension)
    {
        string normalised = Normalise(directory);
        if (Unreadable.Contains(normalised))
        {
            throw new UnauthorizedAccessException($"access to {directory} is denied");
        }

        List<string> matches = [];
        foreach (string path in _files.Keys)
        {
            string? parent = Path.GetDirectoryName(path);
            if (parent is not null
                && string.Equals(Normalise(parent), normalised, StringComparison.OrdinalIgnoreCase)
                && path.EndsWith(extension, StringComparison.OrdinalIgnoreCase))
            {
                matches.Add(path);
            }
        }

        // Deliberately not sorted: the scan is what must impose an order.
        matches.Reverse();
        return matches;
    }

    public byte[] ReadAllBytes(string path)
    {
        Reads.Add(path);
        return _files.TryGetValue(path, out byte[]? contents)
            ? contents
            : throw new FileNotFoundException(path);
    }

    public string ReadAllText(string path) => System.Text.Encoding.UTF8.GetString(ReadAllBytes(path));

    public void WriteAllBytes(string path, byte[] contents) => AddFile(path, contents);

    public void WriteAllText(string path, string contents) => AddText(path, contents);

    public void DeleteFile(string path) => _files.Remove(path);

    public void CopyFile(string sourcePath, string destinationPath, bool overwrite)
    {
        if (!overwrite && _files.ContainsKey(destinationPath))
        {
            throw new IOException($"{destinationPath} already exists");
        }

        AddFile(destinationPath, ReadAllBytes(sourcePath));
    }

    public void MoveFile(string sourcePath, string destinationPath, bool overwrite)
    {
        CopyFile(sourcePath, destinationPath, overwrite);
        _files.Remove(sourcePath);
    }

    private void AddParentDirectories(string path)
    {
        string? parent = Path.GetDirectoryName(path);
        while (!string.IsNullOrEmpty(parent))
        {
            _directories.Add(Normalise(parent));
            parent = Path.GetDirectoryName(parent);
        }
    }

    private static string Normalise(string path) => path.TrimEnd('\\', '/');
}
