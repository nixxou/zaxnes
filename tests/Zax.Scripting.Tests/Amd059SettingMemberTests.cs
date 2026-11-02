using Zax.Settings;

namespace Zax.Scripting.Tests;

/// <summary>
/// AMD-059 REQ-LUA-116, the four scalar members 3dSen 2.0 added to <c>Setting</c>:
/// <c>LibraryView</c>, <c>MaxQueuedFrames</c>, <c>RefreshRate</c> and <c>Msaa</c>, with the
/// normalisations of the table and the promise that no numeric write ever raises.
/// </summary>
/// <remarks>
/// That promise is the point of the requirement: in Zaxnes before this lot the four members did
/// not exist, so a write raised « no such field » and the faulty script lost every one of its
/// hooks (REQ-LUA-062). A test that only checked the read value would miss that.
/// </remarks>
public sealed class Amd059SettingMemberTests
{
    private static ScriptSession Session() => new ScriptProfileBuilder().Script("a", string.Empty).Start();

    /// <summary>REQ-LUA-116: the defaults a 2.0 script reads before writing anything.</summary>
    [Theory] // REQ-LUA-116
    [InlineData("LibraryView", 0)]
    [InlineData("MaxQueuedFrames", 2)]
    [InlineData("RefreshRate", 90)]
    [InlineData("Msaa", 2)]
    public void The_four_members_read_their_default(string member, int expected)
    {
        using ScriptSession session = Session();
        Assert.Equal(expected, session.Runtime.EvaluateNumber($"return Setting.{member}"));
    }

    /// <summary>
    /// REQ-LUA-116 — <c>MaxQueuedFrames</c>: 1 or less gives 1, anything above gives 2. It is
    /// the setting of AMD-052 REQ-LOOP-077, so <c>video.max_queued_frames</c> holds the same.
    /// </summary>
    [Theory] // REQ-LUA-116, REQ-LOOP-077
    [InlineData(5, 2)]
    [InlineData(0, 1)]
    [InlineData(1, 1)]
    [InlineData(2, 2)]
    [InlineData(-7, 1)]
    [InlineData(1000, 2)]
    public void The_queue_depth_is_one_below_two_and_two_above(int written, int expected)
    {
        using ScriptSession session = Session();
        session.Runtime.Evaluate($"Setting.MaxQueuedFrames = {written}");

        Assert.Equal(expected, session.Runtime.EvaluateNumber("return Setting.MaxQueuedFrames"));
        Assert.Equal(expected, session.Runtime.Settings.Model.GetInt(SettingKeys.VideoMaxQueuedFrames));
    }

    /// <summary>
    /// REQ-LUA-116 — <c>RefreshRate</c>: the nearest of {72, 90, 120} and, at equal distance,
    /// the smaller one. 81 is halfway between 72 and 90, 105 between 90 and 120.
    /// </summary>
    [Theory] // REQ-LUA-116
    [InlineData(100, 90)]
    [InlineData(81, 72)]
    [InlineData(105, 90)]
    [InlineData(200, 120)]
    [InlineData(0, 72)]
    [InlineData(72, 72)]
    [InlineData(90, 90)]
    [InlineData(120, 120)]
    [InlineData(82, 90)]
    [InlineData(106, 120)]
    [InlineData(-50, 72)]
    public void The_refresh_rate_lands_on_the_nearest_rung(int written, int expected)
    {
        using ScriptSession session = Session();
        session.Runtime.Evaluate($"Setting.RefreshRate = {written}");

        Assert.Equal(expected, session.Runtime.EvaluateNumber("return Setting.RefreshRate"));
        Assert.Equal(expected, session.Runtime.Settings.Model.GetInt(SettingKeys.VrRefreshRate));
    }

    /// <summary>
    /// REQ-LUA-116 — <c>Msaa</c>: 1 or less gives 0, 2 or 3 gives 2, 4 or more gives 4. It is a
    /// setting of its own: <c>video.anti_alias</c> never moves <b>(D)</b>.
    /// </summary>
    [Theory] // REQ-LUA-116
    [InlineData(3, 2)]
    [InlineData(1, 0)]
    [InlineData(8, 4)]
    [InlineData(0, 0)]
    [InlineData(2, 2)]
    [InlineData(4, 4)]
    [InlineData(-1, 0)]
    public void The_msaa_count_lands_on_zero_two_or_four(int written, int expected)
    {
        using ScriptSession session = Session();
        int antiAlias = session.Runtime.Settings.Model.GetInt(SettingKeys.VideoAntiAlias);

        session.Runtime.Evaluate($"Setting.Msaa = {written}");

        Assert.Equal(expected, session.Runtime.EvaluateNumber("return Setting.Msaa"));
        Assert.Equal(antiAlias, session.Runtime.Settings.Model.GetInt(SettingKeys.VideoAntiAlias));
        Assert.Equal(antiAlias, session.Runtime.EvaluateNumber("return Setting.AntiAlias"));
    }

    /// <summary>REQ-LUA-116 — <c>LibraryView</c> hands back exactly what was written.</summary>
    [Theory] // REQ-LUA-116
    [InlineData(7)]
    [InlineData(0)]
    [InlineData(-4)]
    [InlineData(123456)]
    public void The_library_view_is_kept_as_written(int written)
    {
        using ScriptSession session = Session();
        session.Runtime.Evaluate($"Setting.LibraryView = {written}");

        Assert.Equal(written, session.Runtime.EvaluateNumber("return Setting.LibraryView"));
        Assert.Equal(written, session.Runtime.Settings.Model.GetInt(SettingKeys.UiLibraryView));
    }

    /// <summary>
    /// REQ-LUA-116 — the fractional part of a written value is truncated, as REQ-LUA-040 asks of
    /// every integer member.
    /// </summary>
    [Fact] // REQ-LUA-116, REQ-LUA-040
    public void A_fractional_write_is_truncated()
    {
        using ScriptSession session = Session();

        session.Runtime.Evaluate("Setting.LibraryView = 7.9");
        Assert.Equal(7, session.Runtime.EvaluateNumber("return Setting.LibraryView"));

        session.Runtime.Evaluate("Setting.RefreshRate = 81.9");
        Assert.Equal(72, session.Runtime.EvaluateNumber("return Setting.RefreshRate"));

        session.Runtime.Evaluate("Setting.Msaa = 3.9");
        Assert.Equal(2, session.Runtime.EvaluateNumber("return Setting.Msaa"));

        session.Runtime.Evaluate("Setting.MaxQueuedFrames = 1.9");
        Assert.Equal(1, session.Runtime.EvaluateNumber("return Setting.MaxQueuedFrames"));
    }

    /// <summary>
    /// REQ-LUA-116 — « aucune de ces écritures NE DOIT lever d'erreur, quelle que soit la
    /// valeur numérique écrite » : the extremes included, since a raise would cost the script
    /// all its hooks (REQ-LUA-062).
    /// </summary>
    [Theory] // REQ-LUA-116, REQ-LUA-062
    [InlineData("LibraryView")]
    [InlineData("MaxQueuedFrames")]
    [InlineData("RefreshRate")]
    [InlineData("Msaa")]
    public void No_numeric_write_raises(string member)
    {
        using ScriptSession session = Session();

        foreach (string value in (string[])
            ["0", "1", "-1", "3", "5", "8", "100", "1e9", "-1e9", "0.5", "-0.5", "4294967296"])
        {
            session.Runtime.Evaluate($"Setting.{member} = {value}");
        }

        // The hooks of the script are still there: the runtime never put it into error.
        Assert.Empty(session.Log.Errors);
    }

    /// <summary>
    /// The four members are readable and assignable, so they appear in the API description the
    /// <c>lua-api</c> dump is built from (REQ-LUA-080).
    /// </summary>
    [Theory] // REQ-LUA-116
    [InlineData("LibraryView")]
    [InlineData("MaxQueuedFrames")]
    [InlineData("RefreshRate")]
    [InlineData("Msaa")]
    public void The_four_members_are_described_as_readable_and_writable(string member)
    {
        ScriptApiSurface api = ScriptApiSurface.Describe();
        ScriptApiMember found = api.Types["SettingAPI"].Members[member];

        Assert.True(found.Writable);
        Assert.False(found.Callable);
    }
}
