namespace Zax.Scripting.Tests;

/// <summary>
/// P0 gate checks for Zax.Scripting. No <c>REQ-</c> identifier is referenced because no SPEC-05
/// requirement exists yet.
/// </summary>
public sealed class ScaffoldTests
{
    [Fact]
    public void Module_is_identified()
    {
        Assert.Equal("Zax.Scripting", ScriptingModule.Name);
        Assert.False(string.IsNullOrWhiteSpace(ScriptingModule.Role));
    }

    [Fact]
    public void Runtime_reports_an_error_for_a_malformed_chunk()
    {
        using LuaRuntime runtime = new();

        LuaScriptException error = Assert.Throws<LuaScriptException>(() => runtime.Execute("this is not lua"));
        Assert.False(string.IsNullOrWhiteSpace(error.Message));
    }
}
