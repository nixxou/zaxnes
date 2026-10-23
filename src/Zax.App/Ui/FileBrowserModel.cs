namespace Zax.App.Ui;

/// <summary>What one row of the file browser is.</summary>
internal enum FileEntryKind
{
    /// <summary>A logical drive, shown at the root under Windows (SPEC-10 REQ-UI-101).</summary>
    Drive,

    /// <summary>A sub-folder of the current folder.</summary>
    Folder,

    /// <summary>A file the filter keeps.</summary>
    File,
}

/// <summary>One row of the file browser.</summary>
/// <param name="Name">What the row shows.</param>
/// <param name="Path">The full path the row stands for.</param>
/// <param name="Kind">Drive, folder or file.</param>
internal sealed record FileEntry(string Name, string Path, FileEntryKind Kind);

/// <summary>
/// The file browser of SPEC-10 §3.11, kept to what AMD-010 REQ-UI-201 needs: a modal dialog with
/// a path trail, the drives at the root, folders then filtered files, a search box, and the last
/// folder remembered across sessions (REQ-UI-100 to REQ-UI-105).
/// </summary>
/// <remarks>
/// <para>
/// Folders and files that cannot be read — no rights, a drive with no medium — are left out with
/// no message, which is what REQ-UI-102 asks. Nothing here throws: the browser is the one place
/// a user can point the program at an unreadable part of the file system.
/// </para>
/// <para>
/// Writing the remembered folder is the caller's business: it owns the setting
/// <c>ui.last_browse_dir</c> of REQ-PERS-066.
/// </para>
/// </remarks>
internal sealed class FileBrowserModel
{
    /// <summary>The root row label under Windows (REQ-UI-101).</summary>
    public static string ComputerLabel => UiStrings.Get(UiKeys.BrowserComputer);

    private readonly List<FileEntry> _entries = [];
    private readonly List<(string Label, string Path)> _trail = [];

    private string _search = string.Empty;
    private string? _directory;

    /// <summary>The title the caller gives the dialog (REQ-UI-100).</summary>
    public string Title { get; private set; } = UiText.OpenRomTitle;

    /// <summary>
    /// The accepted extensions, lower case and without the dot; empty means every file.
    /// </summary>
    public IReadOnlyList<string> Filter { get; private set; } = [];

    /// <summary>Index of the chosen filter in <see cref="Filters"/>.</summary>
    public int FilterIndex { get; set; }

    /// <summary>The filters the dialog offers, in order.</summary>
    public IReadOnlyList<(string Label, string[] Extensions)> Filters { get; private set; } = [];

    /// <summary>
    /// The current folder, or null at the root, where the drives are listed (REQ-UI-101).
    /// </summary>
    public string? Directory => _directory;

    /// <summary>The rows of the current folder: drives, then folders, then files.</summary>
    public IReadOnlyList<FileEntry> Entries => _entries;

    /// <summary>One step per component of the current path, the root first (REQ-UI-101).</summary>
    public IReadOnlyList<(string Label, string Path)> Trail => _trail;

    /// <summary>The selected row, or null.</summary>
    public FileEntry? Selected { get; set; }

    /// <summary>
    /// Whether a folder is an acceptable answer, which is what the 3dSen import of AMD-030 §2
    /// needs: the user names the installation folder, or the bundle inside it.
    /// </summary>
    public bool AllowFolders { get; private set; }

    /// <summary>A line the dialog shows above the list, or an empty string.</summary>
    public string Hint { get; private set; } = string.Empty;

    /// <summary>
    /// The file name field of REQ-UI-100, read only in this dialog. In folder mode it shows the
    /// folder the dialog would answer with.
    /// </summary>
    public string FileName => Selected switch
    {
        { Kind: FileEntryKind.File } entry => entry.Name,
        { Kind: FileEntryKind.Folder or FileEntryKind.Drive } entry when AllowFolders => entry.Name,
        _ => AllowFolders && _directory is not null ? _directory : string.Empty,
    };

    /// <summary>
    /// Whether <see cref="Confirm"/> would answer something: a selected file, or — in folder
    /// mode — a selected folder or the folder being shown.
    /// </summary>
    public bool CanConfirm => Selected is { Kind: FileEntryKind.File }
        || (AllowFolders && (Selected is not null || _directory is not null));

    /// <summary>What the dialog returns when it closes with a validation, or null.</summary>
    public string? Result { get; private set; }

    /// <summary>
    /// The search text of REQ-UI-103: it filters the <b>files</b> whose name contains it, case
    /// and surrounding spaces ignored; folders always stay.
    /// </summary>
    public string Search
    {
        get => _search;
        set
        {
            _search = value ?? string.Empty;
            Reload();
        }
    }

    /// <summary>
    /// Opens the dialog (REQ-UI-100, REQ-UI-105): the search is cleared and the folder is the one
    /// remembered, or the root when it has gone.
    /// </summary>
    /// <param name="title">The title.</param>
    /// <param name="filters">The filters, the first one chosen.</param>
    /// <param name="startDirectory">The remembered folder of <c>ui.last_browse_dir</c>.</param>
    /// <param name="allowFolders">
    /// Whether a folder is an acceptable answer (AMD-030 §2: the 3dSen installation folder).
    /// </param>
    /// <param name="hint">A line shown above the list, or null.</param>
    public void Open(
        string title,
        IReadOnlyList<(string Label, string[] Extensions)> filters,
        string? startDirectory,
        bool allowFolders = false,
        string? hint = null)
    {
        ArgumentNullException.ThrowIfNull(filters);

        Title = title;
        Filters = filters;
        FilterIndex = 0;
        Filter = filters.Count > 0 ? filters[0].Extensions : [];
        Result = null;
        Selected = null;
        AllowFolders = allowFolders;
        Hint = hint ?? string.Empty;
        _search = string.Empty;

        GoTo(Exists(startDirectory) ? startDirectory : null);
    }

    /// <summary>Chooses another filter and reloads the list.</summary>
    public void ChooseFilter(int index)
    {
        if (Filters.Count == 0)
        {
            return;
        }

        FilterIndex = Math.Clamp(index, 0, Filters.Count - 1);
        Filter = Filters[FilterIndex].Extensions;
        Selected = null;
        Reload();
    }

    /// <summary>Goes to <paramref name="directory"/>, or to the root when it is null.</summary>
    public void GoTo(string? directory)
    {
        _directory = Exists(directory) ? System.IO.Path.GetFullPath(directory!) : null;
        Selected = null;
        Reload();
        RebuildTrail();
    }

    /// <summary>Goes to the parent folder, or to the root from a drive root.</summary>
    public void GoUp()
    {
        if (_directory is null)
        {
            return;
        }

        string? parent = System.IO.Path.GetDirectoryName(_directory);
        GoTo(string.IsNullOrEmpty(parent) ? null : parent);
    }

    /// <summary>
    /// Opens a row (REQ-UI-104): a drive or a folder becomes the current folder, a file
    /// validates the dialog.
    /// </summary>
    /// <returns>True when the dialog is done and <see cref="Result"/> holds the path.</returns>
    public bool Activate(FileEntry entry)
    {
        ArgumentNullException.ThrowIfNull(entry);

        if (entry.Kind == FileEntryKind.File)
        {
            Result = entry.Path;
            return true;
        }

        GoTo(entry.Path);
        return false;
    }

    /// <summary>
    /// The validation button of REQ-UI-104: it only acts when a file is selected. In folder mode
    /// it also answers with the selected folder, or with the folder being shown when no row is
    /// selected (AMD-030 §2).
    /// </summary>
    public bool Confirm()
    {
        if (Selected is { Kind: FileEntryKind.File } file)
        {
            Result = file.Path;
            return true;
        }

        if (!AllowFolders)
        {
            return false;
        }

        if (Selected is { } folder)
        {
            Result = folder.Path;
            return true;
        }

        if (_directory is null)
        {
            return false;
        }

        Result = _directory;
        return true;
    }

    /// <summary>Whether <paramref name="name"/> passes the current filter.</summary>
    public bool Accepts(string name)
    {
        if (Filter.Count == 0)
        {
            return true;
        }

        string extension = System.IO.Path.GetExtension(name);
        if (extension.Length <= 1)
        {
            return false;
        }

        foreach (string allowed in Filter)
        {
            if (string.Equals(extension[1..], allowed, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
        }

        return false;
    }

    /// <summary>
    /// The filters of the ROM dialog (SPEC-10 §4, AMD-010 REQ-UI-201). Built on each read, so
    /// that a dialog opened after a change of language shows the new wording (AMD-035).
    /// </summary>
    public static IReadOnlyList<(string Label, string[] Extensions)> RomFilters =>
    [
        (UiStrings.Get(UiKeys.BrowserFilterNesRoms), ["nes", "zip"]),
        (UiStrings.Get(UiKeys.BrowserFilterNesFiles), ["nes"]),
        (UiStrings.Get(UiKeys.BrowserFilterZip), ["zip"]),
        (UiStrings.Get(UiKeys.BrowserFilterAll), []),
    ];

    /// <summary>The filter of the profile import dialog (SPEC-10 §4).</summary>
    public static IReadOnlyList<(string Label, string[] Extensions)> ProfileFilters =>
    [
        (UiStrings.Get(UiKeys.BrowserFilterProfiles), ["3dn"]),
        (UiStrings.Get(UiKeys.BrowserFilterAll), []),
    ];

    /// <summary>
    /// The filters of the 3dSen import (AMD-030 §2, §3): the bundle of the installation, whose
    /// fingerprint is never assumed, or any file the user points at.
    /// </summary>
    public static IReadOnlyList<(string Label, string[] Extensions)> ThreeDSenFilters =>
    [
        (UiStrings.Get(UiKeys.BrowserFilterBundle), ["bundle"]),
        (UiStrings.Get(UiKeys.BrowserFilterAll), []),
    ];

    /// <summary>
    /// The filter of the cover-art import (SPEC-09 REQ-PERS-036). Only PNG is decoded, so
    /// <c>.jpg</c> is not offered — see <c>GameArt</c> and <c>spec/changes/Q-050.md</c>.
    /// </summary>
    // Q-050
    public static IReadOnlyList<(string Label, string[] Extensions)> CoverFilters =>
    [
        (UiStrings.Get(UiKeys.BrowserFilterPng), ["png"]),
        (UiStrings.Get(UiKeys.BrowserFilterAll), []),
    ];

    private static bool Exists([System.Diagnostics.CodeAnalysis.NotNullWhen(true)] string? directory)
    {
        if (string.IsNullOrWhiteSpace(directory))
        {
            return false;
        }

        try
        {
            return System.IO.Directory.Exists(directory);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException
            or ArgumentException or NotSupportedException)
        {
            return false;
        }
    }

    private void Reload()
    {
        _entries.Clear();

        if (_directory is null)
        {
            foreach (string drive in Drives())
            {
                _entries.Add(new FileEntry(drive, drive, FileEntryKind.Drive));
            }

            return;
        }

        string needle = _search.Trim();

        foreach (string path in Safely(() => System.IO.Directory.GetDirectories(_directory)))
        {
            string name = System.IO.Path.GetFileName(path);
            if (name.Length > 0)
            {
                _entries.Add(new FileEntry(name, path, FileEntryKind.Folder));
            }
        }

        _entries.Sort(static (a, b) => string.Compare(a.Name, b.Name, StringComparison.OrdinalIgnoreCase));

        List<FileEntry> files = [];
        foreach (string path in Safely(() => System.IO.Directory.GetFiles(_directory)))
        {
            string name = System.IO.Path.GetFileName(path);
            if (name.Length == 0 || !Accepts(name))
            {
                continue;
            }

            if (needle.Length > 0 && !name.Contains(needle, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            files.Add(new FileEntry(name, path, FileEntryKind.File));
        }

        files.Sort(static (a, b) => string.Compare(a.Name, b.Name, StringComparison.OrdinalIgnoreCase));
        _entries.AddRange(files);
    }

    private void RebuildTrail()
    {
        _trail.Clear();
        _trail.Add((ComputerLabel, string.Empty));

        if (_directory is null)
        {
            return;
        }

        // One button per component, each ending with the separator, as REQ-UI-101 describes.
        string root = System.IO.Path.GetPathRoot(_directory) ?? string.Empty;
        if (root.Length > 0)
        {
            _trail.Add((root, root));
        }

        string remainder = _directory[root.Length..]
            .Trim(System.IO.Path.DirectorySeparatorChar, System.IO.Path.AltDirectorySeparatorChar);
        if (remainder.Length == 0)
        {
            return;
        }

        string walked = root;
        foreach (string part in remainder.Split(
            [System.IO.Path.DirectorySeparatorChar, System.IO.Path.AltDirectorySeparatorChar],
            StringSplitOptions.RemoveEmptyEntries))
        {
            walked = System.IO.Path.Combine(walked, part);
            _trail.Add((part + System.IO.Path.DirectorySeparatorChar, walked));
        }
    }

    private static IEnumerable<string> Drives()
    {
        string[] names;
        try
        {
            names = System.IO.Directory.GetLogicalDrives();
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            return [];
        }

        List<string> ready = [];
        foreach (string name in names)
        {
            if (Exists(name))
            {
                ready.Add(name);
            }
        }

        return ready;
    }

    private static string[] Safely(Func<string[]> read)
    {
        try
        {
            return read();
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException
            or ArgumentException or NotSupportedException)
        {
            return [];
        }
    }
}
