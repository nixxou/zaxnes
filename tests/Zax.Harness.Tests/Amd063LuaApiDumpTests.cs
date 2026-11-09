using Zax.Harness;

namespace Zax.Harness.Tests;

/// <summary>
/// AMD-063 against <c>Zax.Harness lua-api</c>: the dump must show the two members the amendment
/// adds to host types, and it still says nothing about the global <c>CS</c>.
/// </summary>
/// <remarks>
/// The dump is read back from the very types the runtime binds
/// (<see cref="Zax.Scripting.ScriptApiSurface"/>), so <c>Pattern.HasPlaceholders</c> and
/// <c>Instance:IsSolidTile</c> appear without a line of command code changing. The third member,
/// <c>CS.Nes3D.CameraControl.RotOverride</c> (REQ-LUA-119), is on the <c>CS</c> table, which is
/// made of plain Lua tables and not of host types: it is no more in the dump than
/// <c>DistanceMax</c> was after AMD-059, and <c>Amd063LuaMemberTests</c> covers it instead.
/// </remarks>
public sealed class Amd063LuaApiDumpTests
{
    private static List<string> Dump()
    {
        string path = Path.Combine(Path.GetTempPath(), $"zax-lua-api-{Guid.NewGuid():N}.txt");
        try
        {
            Assert.Equal(0, LuaApiExport.Run(["--out", path]));
            return [.. File.ReadAllLines(path)];
        }
        finally
        {
            File.Delete(path);
        }
    }

    /// <summary>
    /// REQ-LUA-120 — <c>Pattern.HasPlaceholders</c> is a read-only property of the type
    /// <c>Pattern</c>, and REQ-LUA-121 — <c>Instance.IsSolidTile</c> is a method.
    /// </summary>
    [Theory] // AMD-063, REQ-LUA-120, REQ-LUA-121
    [InlineData("member Pattern.HasPlaceholders prop L ")]
    [InlineData("member Instance.IsSolidTile appel - ")]
    public void The_two_new_members_of_the_host_types_are_in_the_dump(string line)
        => Assert.Contains(line, Dump());

    /// <summary>
    /// REQ-LUA-119 — the dump does not describe <c>CS</c>, so <c>RotOverride</c> is not in it.
    /// The non-change is asserted rather than assumed.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-119, REQ-LUA-112
    public void The_cs_table_is_still_outside_the_dump()
    {
        List<string> lines = Dump();

        Assert.DoesNotContain(lines, line => line.Contains("RotOverride", StringComparison.Ordinal));
        Assert.DoesNotContain(lines, line => line.Contains("CameraControl", StringComparison.Ordinal));
        Assert.DoesNotContain(lines, line => line.StartsWith("global CS ", StringComparison.Ordinal));
    }

    /// <summary>
    /// The two members are the only ones the lot adds to the API surface: the counts move by
    /// exactly two, from the 446 members AMD-059 left.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-120, REQ-LUA-121
    public void The_lot_adds_exactly_two_members()
    {
        List<string> lines = Dump();
        string summary = Assert.Single(lines, line => line.StartsWith("# ", StringComparison.Ordinal));
        Assert.Equal("# 35 globals, 46 types, 448 members", summary);
    }
}
