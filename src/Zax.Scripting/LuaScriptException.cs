namespace Zax.Scripting;

/// <summary>
/// Raised when a Lua chunk fails to load or fails during execution.
/// </summary>
public sealed class LuaScriptException : Exception
{
    /// <summary>Creates the exception with the message reported by the Lua state.</summary>
    public LuaScriptException(string message)
        : base(message)
    {
    }

    /// <summary>Creates the exception with the message reported by the Lua state and an inner cause.</summary>
    public LuaScriptException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}
