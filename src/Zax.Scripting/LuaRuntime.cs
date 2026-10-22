using KeraLua;

namespace Zax.Scripting;

/// <summary>
/// Owns one Lua 5.4 state. One instance per emulation instance; no shared or static state.
/// </summary>
/// <remarks>
/// <para>
/// P0 scaffold: this type only loads and runs chunks, which is what the P0 gate needs in
/// order to prove that the native Lua 5.4 library resolves on Windows x64 and that the
/// language level required by the embedded profiles (<c>goto</c> and labels, so Lua 5.2 or
/// later) is available.
/// </para>
/// <para>
/// The API actually exposed to profile scripts is specified in SPEC-05 and implemented in
/// lot L5. None of it is anticipated here.
/// </para>
/// </remarks>
public sealed class LuaRuntime : IDisposable
{
    private Lua? _state;

    /// <summary>Creates a fresh Lua state with the standard libraries opened.</summary>
    public LuaRuntime()
    {
        _state = new Lua(openLibs: true);
    }

    private Lua State => _state ?? throw new ObjectDisposedException(nameof(LuaRuntime));

    /// <summary>
    /// The <c>_VERSION</c> string reported by the embedded interpreter, for example "Lua 5.4".
    /// </summary>
    public string LanguageVersion => EvaluateString("return _VERSION");

    /// <summary>
    /// Runs a chunk and discards any result.
    /// </summary>
    /// <exception cref="LuaScriptException">The chunk failed to load or raised an error.</exception>
    public void Execute(string chunk)
    {
        ArgumentNullException.ThrowIfNull(chunk);

        Lua state = State;
        state.SetTop(0);
        if (state.DoString(chunk))
        {
            throw new LuaScriptException(DescribeError(state));
        }
    }

    /// <summary>
    /// Runs a chunk and returns its first result converted to a 64-bit integer.
    /// </summary>
    /// <exception cref="LuaScriptException">
    /// The chunk failed, returned nothing, or returned a value that is not an integer.
    /// </exception>
    public long EvaluateInteger(string chunk)
    {
        Lua state = RunForResult(chunk);
        if (!state.IsNumber(-1))
        {
            throw new LuaScriptException($"Chunk returned '{state.TypeName(-1)}', expected a number.");
        }

        return state.ToInteger(-1);
    }

    /// <summary>
    /// Runs a chunk and returns its first result converted to a string.
    /// </summary>
    /// <exception cref="LuaScriptException">The chunk failed or returned nothing.</exception>
    public string EvaluateString(string chunk)
    {
        Lua state = RunForResult(chunk);
        return state.ToString(-1) ?? string.Empty;
    }

    private Lua RunForResult(string chunk)
    {
        ArgumentNullException.ThrowIfNull(chunk);

        Lua state = State;
        state.SetTop(0);
        if (state.DoString(chunk))
        {
            throw new LuaScriptException(DescribeError(state));
        }

        if (state.GetTop() < 1)
        {
            throw new LuaScriptException("Chunk returned no value.");
        }

        return state;
    }

    private static string DescribeError(Lua state)
        => state.GetTop() >= 1
            ? state.ToString(-1) ?? "Unknown Lua error."
            : "Unknown Lua error.";

    /// <inheritdoc />
    public void Dispose()
    {
        _state?.Dispose();
        _state = null;
    }
}
