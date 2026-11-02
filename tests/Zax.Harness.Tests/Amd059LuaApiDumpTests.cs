using Zax.Harness;

namespace Zax.Harness.Tests;

/// <summary>
/// AMD-059 against <c>Zax.Harness lua-api</c>: the dump the clean side compares with the tables
/// of SPEC-05 §4 must show the members the amendment adds.
/// </summary>
/// <remarks>
/// <para>
/// The dump is read back from the very types the runtime binds
/// (<see cref="Zax.Scripting.ScriptApiSurface"/>), so the new members of <c>SettingAPI</c> and
/// the new list type appear without a line of command code changing. These tests are what makes
/// that claim checkable rather than assumed.
/// </para>
/// <para>
/// The global <c>CS</c> of REQ-LUA-112 and REQ-LUA-117 is <b>not</b> in the dump, and was not
/// after AMD-040 either: it is made of plain sealed Lua tables rather than of host types, and
/// the three line kinds the command publishes describe globals, constant tables of §4.4 and
/// members of host types. The tests of <c>Amd059CsTableTests</c> cover it instead.
/// </para>
/// </remarks>
public sealed class Amd059LuaApiDumpTests
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

    /// <summary>REQ-LUA-116: the four scalar members are in the dump, readable and writable.</summary>
    [Theory] // REQ-LUA-116
    [InlineData("member SettingAPI.LibraryView prop LE ")]
    [InlineData("member SettingAPI.MaxQueuedFrames prop LE ")]
    [InlineData("member SettingAPI.RefreshRate prop LE ")]
    [InlineData("member SettingAPI.Msaa prop LE ")]
    public void The_four_scalar_members_are_in_the_dump(string line)
        => Assert.Contains(line, Dump());

    /// <summary>
    /// REQ-LUA-116: <c>VisualStyleParameters</c> is in the dump with its list type, and the list
    /// and its element carry the rows of the requirement.
    /// </summary>
    [Fact] // REQ-LUA-116
    public void The_style_parameter_list_is_in_the_dump()
    {
        List<string> lines = Dump();

        Assert.Contains("member SettingAPI.VisualStyleParameters prop LE Liste<ParamStyle>", lines);
        Assert.Contains("member Liste<ParamStyle>.[i] index L ParamStyle", lines);
        Assert.Contains("member Liste<ParamStyle>.Count prop L ", lines);
        Assert.Contains("member ParamStyle.Style prop L ", lines);
        Assert.Contains("member ParamStyle.ParameterId prop L ", lines);
        Assert.Contains("member ParamStyle.Value prop L ", lines);
    }

    /// <summary>
    /// <c>Setting.Msaa</c> is a member of its own next to <c>AntiAlias</c>: the dump shows both,
    /// which is the divergence (D) of AMD-059 §3 made visible.
    /// </summary>
    [Fact] // REQ-LUA-116
    public void The_dump_shows_msaa_and_anti_alias_side_by_side()
    {
        List<string> lines = Dump();

        Assert.Contains("member SettingAPI.Msaa prop LE ", lines);
        Assert.Contains("member SettingAPI.AntiAlias prop LE ", lines);
    }

    /// <summary>
    /// The trailing count line follows the surface the runtime installs, the two types of
    /// REQ-LUA-116 included, and the dump still covers the 360 member rows of SPEC-05 §4
    /// (REQ-LUA-080).
    /// </summary>
    [Fact] // REQ-LUA-116, REQ-LUA-080
    public void The_count_line_follows_the_installed_surface()
    {
        List<string> lines = Dump();
        string last = lines[^1];
        Zax.Scripting.ScriptApiSurface api = Zax.Scripting.ScriptApiSurface.Describe();

        Assert.StartsWith("# ", last, StringComparison.Ordinal);
        Assert.Contains($"{api.Types.Count} types", last, StringComparison.Ordinal);
        Assert.Contains($"{api.MemberCount} members", last, StringComparison.Ordinal);

        // The two types AMD-059 adds are among them.
        Assert.Contains("Liste<ParamStyle>", api.Types.Keys);
        Assert.Contains("ParamStyle", api.Types.Keys);

        int members = lines.Count(static line => line.StartsWith("member ", StringComparison.Ordinal));
        Assert.True(members >= 360, $"only {members} member lines");
    }
}
