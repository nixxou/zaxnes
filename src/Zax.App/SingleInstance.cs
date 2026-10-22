using System.Globalization;
using System.Security.Cryptography;
using System.Text;

namespace Zax.App;

/// <summary>
/// The single-instance guard of SPEC-07 REQ-LOOP-054: a named system mutex the first instance
/// holds for its whole run, so a second launch closes instead of opening a second window over the
/// same user root.
/// </summary>
/// <remarks>
/// <para>
/// The requirement is a recommendation (<b>DEVRAIT</b>) and offers two behaviours: activate the
/// first instance, or close. Closing is what is implemented — activating a foreign window means
/// an inter-process channel and a platform call that the rest of the host has no use for, and the
/// acceptance criterion ("launch the executable twice → a single window") is met either way.
/// </para>
/// <para>
/// The name carries a hash of the <b>user root</b>, because what two instances must not share is
/// that root: its settings file, its library, its slot index and its battery files are all
/// written without locking. Two runs on two different roots — a test run with
/// <c>--userdata</c>, or a second user on the same machine — are therefore allowed, which is
/// also what keeps the test suite able to start the application.
/// </para>
/// <para>
/// A mutex is named per session on Windows unless prefixed with <c>Global\</c>; the local prefix
/// is deliberate, so one user's running copy does not stop another's.
/// </para>
/// </remarks>
internal sealed class SingleInstance : IDisposable
{
    private Mutex? _mutex;
    private bool _held;

    private SingleInstance(Mutex mutex, bool held)
    {
        _mutex = mutex;
        _held = held;
    }

    /// <summary>The message a second launch prints before closing.</summary>
    public const string AlreadyRunningMessage =
        "Zaxnes: another instance is already running on this data folder; this one closes (REQ-LOOP-054).";

    /// <summary>Whether this process is the one instance that may run.</summary>
    public bool IsOwner => _held;

    /// <summary>
    /// Tries to become the single instance for <paramref name="userDataRoot"/>.
    /// </summary>
    /// <param name="userDataRoot">
    /// The user root the instance will write to, or null for the default one. Only its identity
    /// matters, so it does not have to exist yet.
    /// </param>
    /// <returns>
    /// The guard. <see cref="IsOwner"/> is false when another instance already holds the name,
    /// and the caller must then exit. A platform that refuses named mutexes altogether yields a
    /// guard that owns nothing to release and reports itself the owner, so the program still
    /// starts — the requirement is a recommendation, not a precondition.
    /// </returns>
    public static SingleInstance Acquire(string? userDataRoot)
    {
        string name = NameFor(userDataRoot);
        try
        {
            Mutex mutex = new(initiallyOwned: false, name);

            // No wait: the question is whether someone else holds it right now.
            bool held = mutex.WaitOne(TimeSpan.Zero, exitContext: false);
            return new SingleInstance(mutex, held);
        }
        catch (Exception exception) when (exception is WaitHandleCannotBeOpenedException
            or UnauthorizedAccessException
            or IOException
            or NotSupportedException)
        {
            // An abandoned mutex means the previous owner died without releasing it, and this
            // instance inherits it; anything else means the platform will not give us the
            // name at all, and the guard then lets the program run.
            return new SingleInstance(null!, held: true);
        }
        catch (AbandonedMutexException)
        {
            return new SingleInstance(null!, held: true);
        }
    }

    /// <summary>The mutex name: a fixed prefix plus a hash of the user root.</summary>
    public static string NameFor(string? userDataRoot)
    {
        string root = string.IsNullOrEmpty(userDataRoot)
            ? "<default>"
            : Path.GetFullPath(userDataRoot).TrimEnd(Path.DirectorySeparatorChar);

        byte[] digest = SHA256.HashData(Encoding.UTF8.GetBytes(root.ToLowerInvariant()));
        return string.Create(
            CultureInfo.InvariantCulture,
            $"Local\\Zax.Emulator.{Convert.ToHexString(digest, 0, 8)}");
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_mutex is { } mutex)
        {
            if (_held)
            {
                try
                {
                    mutex.ReleaseMutex();
                }
                catch (ApplicationException)
                {
                    // Already released, or never owned by this thread; nothing to repair.
                }
            }

            mutex.Dispose();
            _mutex = null;
        }

        _held = false;
    }
}
