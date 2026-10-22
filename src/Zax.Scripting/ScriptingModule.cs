namespace Zax.Scripting;

/// <summary>
/// P0 scaffolding marker for the profile scripting runtime.
/// </summary>
/// <remarks>
/// The hook set, the API surface exposed to profile scripts, the script environment and
/// the error handling are implemented in lot L5 against SPEC-05. Nothing of that API is
/// invented here.
/// </remarks>
public static class ScriptingModule
{
    /// <summary>Stable identifier of this module, used by the headless harness and tests.</summary>
    public const string Name = "Zax.Scripting";

    /// <summary>Short human-readable role of this module.</summary>
    public const string Role = "Lua 5.4 runtime for profile scripts";
}
