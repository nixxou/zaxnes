namespace Zax.Settings;

/// <summary>
/// The abstract blob store REQ-PERS-082 requires: the settings model reads and writes named
/// blobs and knows nothing of the file system. The desktop host maps a name to a file of the
/// user root; a libretro frontend maps it to the folders the host provides (REQ-PERS-086); a
/// test maps it to memory.
/// </summary>
public interface ISettingsStorage
{
    /// <summary>
    /// Whether writes can succeed. False puts the program in the degraded mode of
    /// REQ-PERS-073: it runs on in-memory defaults and refuses the write actions.
    /// </summary>
    bool IsWritable { get; }

    /// <summary>Whether a blob named <paramref name="name"/> exists.</summary>
    bool Exists(string name);

    /// <summary>
    /// The contents of <paramref name="name"/>, or null when it does not exist or cannot be
    /// read. An unreadable blob must not throw: the caller falls back to the defaults
    /// (REQ-PERS-022).
    /// </summary>
    byte[]? Read(string name);

    /// <summary>
    /// Replaces <paramref name="name"/> with <paramref name="contents"/> atomically: a
    /// temporary blob beside it, then a replace (REQ-PERS-021). Returns whether it worked.
    /// </summary>
    bool Write(string name, ReadOnlySpan<byte> contents);

    /// <summary>
    /// Renames <paramref name="name"/> to <paramref name="target"/>, overwriting it. Used to
    /// set an unreadable blob aside as <c>*.corrupt-&lt;timestamp&gt;</c> (REQ-PERS-022).
    /// </summary>
    bool Rename(string name, string target);

    /// <summary>
    /// Copies <paramref name="name"/> to <paramref name="target"/>. Used to keep the
    /// <c>&lt;name&gt;.v&lt;N&gt;.bak</c> copy a migration must leave behind (REQ-PERS-024).
    /// </summary>
    bool Copy(string name, string target);
}

/// <summary>A store that holds its blobs in memory, for tests and for the degraded mode.</summary>
public sealed class MemorySettingsStorage : ISettingsStorage
{
    private readonly Dictionary<string, byte[]> _blobs = new(StringComparer.OrdinalIgnoreCase);

    /// <summary>Creates an empty store.</summary>
    /// <param name="isWritable">Whether writes are accepted (REQ-PERS-073).</param>
    public MemorySettingsStorage(bool isWritable = true)
    {
        IsWritable = isWritable;
    }

    /// <inheritdoc/>
    public bool IsWritable { get; }

    /// <summary>The names currently held.</summary>
    public IReadOnlyCollection<string> Names => _blobs.Keys;

    /// <inheritdoc/>
    public bool Exists(string name) => _blobs.ContainsKey(name);

    /// <inheritdoc/>
    public byte[]? Read(string name) => _blobs.TryGetValue(name, out byte[]? bytes) ? bytes : null;

    /// <inheritdoc/>
    public bool Write(string name, ReadOnlySpan<byte> contents)
    {
        if (!IsWritable)
        {
            return false;
        }

        _blobs[name] = contents.ToArray();
        return true;
    }

    /// <inheritdoc/>
    public bool Rename(string name, string target)
    {
        if (!IsWritable || !_blobs.Remove(name, out byte[]? bytes))
        {
            return false;
        }

        _blobs[target] = bytes;
        return true;
    }

    /// <inheritdoc/>
    public bool Copy(string name, string target)
    {
        if (!IsWritable || !_blobs.TryGetValue(name, out byte[]? bytes))
        {
            return false;
        }

        _blobs[target] = bytes;
        return true;
    }

    /// <summary>Puts <paramref name="text"/> under <paramref name="name"/>, for a test.</summary>
    public void Put(string name, string text)
        => _blobs[name] = new System.Text.UTF8Encoding(encoderShouldEmitUTF8Identifier: false).GetBytes(text);

    /// <summary>The contents of <paramref name="name"/> as text, or null.</summary>
    public string? Text(string name)
        => _blobs.TryGetValue(name, out byte[]? bytes) ? System.Text.Encoding.UTF8.GetString(bytes) : null;
}
