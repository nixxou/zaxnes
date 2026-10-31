using System.Globalization;
using System.Text;

namespace Zax.App.Persistence;

/// <summary>
/// The run log of SPEC-09 §3.8 — the <c>logs/</c> folder, « rotation libre » — and the sink the
/// per-frame lines of AMD-053 REQ-UI-213 go into.
/// </summary>
/// <remarks>
/// <para>
/// A normal launch never borrows the console of the shell it was started from (AMD-032 §5), so
/// anything the program has to say over a whole session has to be written somewhere: this is
/// that file. The console output is unchanged — what goes to the console still goes there — and
/// <c>-emustats</c> is the one sink that writes <b>only</b> here, which is what its criterion
/// asks for.
/// </para>
/// <para>
/// Rotation is the simplest one the folder's « libre » allows: the file of the previous run is
/// kept as <c>&lt;name&gt;.1</c> and the new run starts a fresh one. Nothing of the log is ever
/// read back by the program, so a failure to write costs one lost line and never an exception —
/// a read-only root (REQ-PERS-073) simply has no log.
/// </para>
/// </remarks>
public sealed class RunLog : IDisposable
{
    /// <summary>Name of the log file inside <see cref="UserDataPaths.LogsDirectory"/>.</summary>
    public const string FileName = "zaxnes.log";

    private readonly object _gate = new();
    private StreamWriter? _writer;

    private RunLog(StreamWriter? writer, string path)
    {
        _writer = writer;
        Path = path;
    }

    /// <summary>The file being written, whether or not it could be opened.</summary>
    public string Path { get; }

    /// <summary>Whether lines really reach the disk.</summary>
    public bool IsOpen => _writer is not null;

    /// <summary>
    /// Opens the log of this run under <paramref name="paths"/>, rotating the previous one.
    /// Never throws: a root that refuses writes gives a log that drops everything.
    /// </summary>
    public static RunLog Open(UserDataPaths paths)
    {
        ArgumentNullException.ThrowIfNull(paths);

        string path = System.IO.Path.Combine(paths.LogsDirectory, FileName);
        try
        {
            Directory.CreateDirectory(paths.LogsDirectory);
            if (File.Exists(path))
            {
                File.Move(path, path + ".1", overwrite: true);
            }

            // Shared for reading: the log is meant to be looked at while the program runs.
            FileStream stream = new(path, FileMode.Create, FileAccess.Write, FileShare.ReadWrite);
            StreamWriter writer = new(stream, new UTF8Encoding(false)) { AutoFlush = true };

            return new RunLog(writer, path);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            return new RunLog(null, path);
        }
    }

    /// <summary>Writes one line, stamped with the time of day. Does nothing when closed.</summary>
    public void Write(string line)
    {
        if (_writer is null)
        {
            return;
        }

        lock (_gate)
        {
            try
            {
                _writer?.WriteLine(string.Create(
                    CultureInfo.InvariantCulture,
                    $"{DateTimeOffset.Now:HH:mm:ss.fff} {line}"));
            }
            catch (Exception exception) when (exception is IOException or ObjectDisposedException)
            {
                // A log that cannot be written must never stop the program.
            }
        }
    }

    /// <summary>How many lines the file holds; -1 when it cannot be read.</summary>
    public int CountLines()
    {
        try
        {
            lock (_gate)
            {
                _writer?.Flush();
            }

            if (!File.Exists(Path))
            {
                return 0;
            }

            using FileStream stream = new(Path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
            using StreamReader reader = new(stream);
            int lines = 0;
            while (reader.ReadLine() is not null)
            {
                lines++;
            }

            return lines;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            return -1;
        }
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        lock (_gate)
        {
            _writer?.Dispose();
            _writer = null;
        }
    }
}
