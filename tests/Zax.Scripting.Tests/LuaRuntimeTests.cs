namespace Zax.Scripting.Tests;

/// <summary>
/// Proves that the native Lua library loads on Windows x64 and that the language level the
/// embedded profiles need is actually available.
/// </summary>
/// <remarks>
/// PLAN.md section 4, decision of 2026-10-20: KeraLua / Lua 5.4 was chosen over MoonSharp
/// because the inventory of the 579 scripts found <c>goto</c> and labels in 2 profiles, which
/// requires Lua 5.2 or later. These tests are the executable form of that decision.
/// </remarks>
public sealed class LuaRuntimeTests
{
    [Fact]
    public void Native_library_loads_and_reports_lua_5_4()
    {
        using LuaRuntime runtime = new();

        Assert.Equal("Lua 5.4", runtime.LanguageVersion);
    }

    /// <summary>
    /// The feature the runtime choice hinges on: a forward <c>goto</c> to a <c>::continue::</c>
    /// label at the end of a loop body. Rejected outright by Lua 5.1 and by MoonSharp 5.2 era
    /// grammars that omit labels.
    /// </summary>
    [Fact]
    public void Goto_and_labels_are_supported()
    {
        using LuaRuntime runtime = new();

        const string chunk = """
            local sum = 0
            for i = 1, 10 do
              if i % 2 == 0 then goto continue end
              sum = sum + i
              ::continue::
            end
            return sum
            """;

        // 1 + 3 + 5 + 7 + 9
        Assert.Equal(25L, runtime.EvaluateInteger(chunk));
    }

    [Fact]
    public void Goto_can_jump_out_of_nested_loops()
    {
        using LuaRuntime runtime = new();

        const string chunk = """
            local visited = 0
            for i = 1, 4 do
              for j = 1, 4 do
                if i * j > 6 then goto done end
                visited = visited + 1
              end
            end
            ::done::
            return visited
            """;

        // (1,1..4) and (2,1..3) are visited, then i*j = 8 jumps out: 4 + 3 = 7.
        Assert.Equal(7L, runtime.EvaluateInteger(chunk));
    }

    [Fact]
    public void Each_runtime_owns_an_independent_state()
    {
        using LuaRuntime first = new();
        using LuaRuntime second = new();

        first.Execute("shared = 'first'");

        Assert.Equal("nil", second.EvaluateString("return type(shared)"));
    }

    [Fact]
    public void Disposed_runtime_rejects_further_use()
    {
        LuaRuntime runtime = new();
        runtime.Dispose();

        Assert.Throws<ObjectDisposedException>(() => runtime.Execute("return 1"));
    }
}
