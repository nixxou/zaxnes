using Zax.Scripting;
using Zax.Settings;

namespace Zax.Acceptance.Tests;

/// <summary>
/// The acceptance criteria of AMD-059: the members 3dSen 2.0 added to <c>Setting</c>
/// (REQ-LUA-116) and the complement of the <c>CS</c> table (REQ-LUA-117), each criterion
/// written as the amendment writes it.
/// </summary>
/// <remarks>
/// What the amendment is about is a crash, not a value: before this lot each of these paths
/// raised in Zaxnes — « cannot set …, no such field » in writing, the index of a nil in
/// reading — and a raise inside a profile script costs it every hook it registered
/// (REQ-LUA-062). The last test of the file is that end-to-end check on a real profile script
/// with hooks and frames.
/// </remarks>
public sealed class Amd059Tests
{
    private static LuaRun Empty() => new LuaProfile().Script("a", string.Empty).Start();

    /// <summary>
    /// REQ-LUA-116, criterion 1: <c>Setting.MaxQueuedFrames = 5</c> gives 2;
    /// <c>= 0</c> gives 1 and <c>video.max_queued_frames</c> holds 1.
    /// </summary>
    [Fact] // REQ-LUA-116, REQ-LOOP-077
    public void REQ_LUA_116_the_queue_depth_criterion()
    {
        using LuaRun run = Empty();

        Assert.Equal(2, run.Num("Setting.MaxQueuedFrames"));

        run.Do("Setting.MaxQueuedFrames = 5");
        Assert.Equal(2, run.Num("Setting.MaxQueuedFrames"));

        run.Do("Setting.MaxQueuedFrames = 0");
        Assert.Equal(1, run.Num("Setting.MaxQueuedFrames"));
        Assert.Equal(1, run.Settings.Model.GetInt(SettingKeys.VideoMaxQueuedFrames));
    }

    /// <summary>
    /// REQ-LUA-116, criterion 2: <c>RefreshRate = 100</c> gives 90, <c>= 81</c> gives 72,
    /// <c>= 105</c> gives 90, <c>= 200</c> gives 120.
    /// </summary>
    [Theory] // REQ-LUA-116
    [InlineData(100, 90)]
    [InlineData(81, 72)]
    [InlineData(105, 90)]
    [InlineData(200, 120)]
    public void REQ_LUA_116_the_refresh_rate_criterion(int written, int expected)
    {
        using LuaRun run = Empty();

        Assert.Equal(90, run.Num("Setting.RefreshRate"));
        run.Do($"Setting.RefreshRate = {written}");
        Assert.Equal(expected, run.Num("Setting.RefreshRate"));
    }

    /// <summary>
    /// REQ-LUA-116, criterion 3: <c>Msaa = 3</c> gives 2, <c>= 1</c> gives 0, <c>= 8</c> gives
    /// 4, and <c>video.anti_alias</c> never moves.
    /// </summary>
    [Fact] // REQ-LUA-116
    public void REQ_LUA_116_the_msaa_criterion()
    {
        using LuaRun run = Empty();
        int antiAlias = run.Settings.Model.GetInt(SettingKeys.VideoAntiAlias);

        Assert.Equal(2, run.Num("Setting.Msaa"));

        run.Do("Setting.Msaa = 3");
        Assert.Equal(2, run.Num("Setting.Msaa"));

        run.Do("Setting.Msaa = 1");
        Assert.Equal(0, run.Num("Setting.Msaa"));

        run.Do("Setting.Msaa = 8");
        Assert.Equal(4, run.Num("Setting.Msaa"));

        Assert.Equal(antiAlias, run.Settings.Model.GetInt(SettingKeys.VideoAntiAlias));
        Assert.Equal(antiAlias, run.Num("Setting.AntiAlias"));
    }

    /// <summary>REQ-LUA-116, criterion 4: <c>Setting.LibraryView = 7</c> gives 7.</summary>
    [Fact] // REQ-LUA-116
    public void REQ_LUA_116_the_library_view_criterion()
    {
        using LuaRun run = Empty();

        Assert.Equal(0, run.Num("Setting.LibraryView"));
        run.Do("Setting.LibraryView = 7");
        Assert.Equal(7, run.Num("Setting.LibraryView"));
    }

    /// <summary>
    /// REQ-LUA-116, criterion 5: after <c>ResetVisualStyleParameters(4)</c> and two writes, the
    /// list holds two elements and element 0 is style 4, <c>spl.bend_zx</c>, value 30; then
    /// <c>= nil</c> gives a count of 0 and the parameter back at 0.
    /// </summary>
    [Fact] // REQ-LUA-116
    public void REQ_LUA_116_the_style_parameter_list_criterion()
    {
        using LuaRun run = Empty();

        run.Do("""
            Setting:ResetVisualStyleParameters(4)
            Setting:SetVisualStyleParameter(4, "spl.bend_zx", 40)
            Setting:SetVisualStyleParameter(4, "spl.ao", 2)
            """);

        Assert.Equal(2, run.Num("Setting.VisualStyleParameters.Count"));
        Assert.Equal(4, run.Num("Setting.VisualStyleParameters[0].Style"));
        Assert.Equal("spl.bend_zx", run.Text("Setting.VisualStyleParameters[0].ParameterId"));
        Assert.Equal(30, run.Num("Setting.VisualStyleParameters[0].Value"));

        run.Do("Setting.VisualStyleParameters = nil");

        Assert.Equal(0, run.Num("Setting.VisualStyleParameters.Count"));
        Assert.Equal(0, run.Num("Setting:GetVisualStyleParameter(4, 'spl.bend_zx')"));
    }

    /// <summary>
    /// REQ-LUA-116: « aucune de ces écritures NE DOIT lever d'erreur, quelle que soit la valeur
    /// numérique écrite ».
    /// </summary>
    [Theory] // REQ-LUA-116
    [InlineData("LibraryView")]
    [InlineData("MaxQueuedFrames")]
    [InlineData("RefreshRate")]
    [InlineData("Msaa")]
    public void REQ_LUA_116_no_numeric_write_raises(string member)
    {
        using LuaRun run = Empty();

        foreach (string value in (string[])["0", "1", "-1", "3", "5", "8", "100", "1e9", "-1e9", "0.5"])
        {
            Assert.Null(run.Error($"Setting.{member} = {value}"));
        }

        Assert.Empty(run.Log.Errors);
    }

    /// <summary>
    /// REQ-LUA-117, the criterion: <c>DistanceMax</c> gives 512, <c>DefaultMsaa</c> gives 2,
    /// <c>Setting.VisualStyle = CS.Nes3D.VisualStyle.SoftPixelLight</c> then the comparison with
    /// the same path is true, and <c>CS.Nes3D.CameraControl.Autre</c> is nil.
    /// </summary>
    [Fact] // REQ-LUA-117
    public void REQ_LUA_117_the_cs_table_criterion()
    {
        using LuaRun run = Empty();

        Assert.Equal(512, run.Num("CS.Nes3D.CameraControl.DistanceMax"));
        Assert.Equal(2, run.Num("CS.Nes3D.SettingManager.DefaultMsaa"));

        run.Do("Setting.VisualStyle = CS.Nes3D.VisualStyle.SoftPixelLight");
        Assert.True(run.Flag("Setting.VisualStyle == CS.Nes3D.VisualStyle.SoftPixelLight"));

        Assert.Equal("nil", run.TypeOf("CS.Nes3D.CameraControl.Autre"));
        Assert.Empty(run.Log.Errors);
    }

    /// <summary>
    /// REQ-LUA-117: the two style identifiers are numbers — <c>Classic</c> 0,
    /// <c>SoftPixelLight</c> 4 — and every other field of the three tables is nil.
    /// </summary>
    [Fact] // REQ-LUA-117
    public void REQ_LUA_117_the_style_identifiers_and_the_nil_rule()
    {
        using LuaRun run = Empty();

        Assert.Equal(0, run.Num("CS.Nes3D.VisualStyle.Classic"));
        Assert.Equal(4, run.Num("CS.Nes3D.VisualStyle.SoftPixelLight"));
        Assert.Equal(SettingsCatalogue.ClassicStyle, (int)run.Num("CS.Nes3D.VisualStyle.Classic"));
        Assert.Equal(SettingsCatalogue.EnhancedStyle, (int)run.Num("CS.Nes3D.VisualStyle.SoftPixelLight"));

        Assert.Equal("nil", run.TypeOf("CS.Nes3D.SettingManager.Autre"));
        Assert.Equal("nil", run.TypeOf("CS.Nes3D.VisualStyle.Autre"));
        Assert.Equal("nil", run.TypeOf("CS.Nes3D.VisualStyle.Enhanced"));

        // A write anywhere in them changes nothing and raises nothing.
        Assert.Null(run.Error("CS.Nes3D.VisualStyle.Classic = 9"));
        Assert.Equal(0, run.Num("CS.Nes3D.VisualStyle.Classic"));
        Assert.Empty(run.Log.Errors);
    }

    /// <summary>
    /// The end-to-end check: a profile script that writes every member AMD-059 adds, once, in
    /// its <c>Start</c> hook keeps all its hooks. The frame hooks go on firing over the frames
    /// that follow, nothing is logged, and no script is in error — which is the quantity
    /// <c>Zax.Harness</c> reports as « scripts: N went into error during the run », so that line
    /// cannot appear.
    /// </summary>
    [Fact] // REQ-LUA-116, REQ-LUA-117, REQ-LUA-062
    public void A_script_that_writes_every_new_member_keeps_its_hooks()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "gear")
            .Script(
                "gear",
                """
                started = 0
                frames = 0
                shapes = 0

                function Start()
                  started = started + 1

                  -- Every scalar member of REQ-LUA-116, written once.
                  Setting.LibraryView = 7
                  Setting.MaxQueuedFrames = 5
                  Setting.RefreshRate = 100
                  Setting.Msaa = 3

                  -- The list of REQ-LUA-116, read, written back and emptied.
                  Setting:SetVisualStyleParameter(CS.Nes3D.VisualStyle.SoftPixelLight, "spl.bend_zx", 40)
                  kept = Setting.VisualStyleParameters
                  count = kept.Count
                  first = kept[0].ParameterId
                  Setting.VisualStyleParameters = kept
                  Setting.VisualStyleParameters = nil

                  -- Every path of REQ-LUA-117, read.
                  distance = CS.Nes3D.CameraControl.DistanceMax
                  msaa = CS.Nes3D.SettingManager.DefaultMsaa
                  classic = CS.Nes3D.VisualStyle.Classic
                  enhanced = CS.Nes3D.VisualStyle.SoftPixelLight
                  Setting.VisualStyle = enhanced
                end

                function Update()
                  frames = frames + 1
                  queue = Setting.MaxQueuedFrames
                end

                function UpdateS()
                  shapes = shapes + 1
                end
                """,
                tags: "gear")
            .Start();

        // Start ran to its end: every write and every read landed.
        Assert.Equal(1, run.Num("started"));
        Assert.Equal(7, run.Num("Setting.LibraryView"));
        Assert.Equal(2, run.Num("Setting.MaxQueuedFrames"));
        Assert.Equal(90, run.Num("Setting.RefreshRate"));
        Assert.Equal(2, run.Num("Setting.Msaa"));
        Assert.Equal(1, run.Num("count"));
        Assert.Equal("spl.bend_zx", run.Text("first"));
        Assert.Equal(512, run.Num("distance"));
        Assert.Equal(2, run.Num("msaa"));
        Assert.Equal(0, run.Num("classic"));
        Assert.Equal(4, run.Num("enhanced"));
        Assert.Equal(4, run.Num("Setting.VisualStyle"));

        // The hooks are still alive: they fire on every frame that follows.
        for (int counter = 1; counter <= 5; counter++)
        {
            run.Frame(new SyntheticBatch().Background('A', 16, 32), counter);
        }

        Assert.Equal(5, run.Num("frames"));
        Assert.True(run.Num("shapes") >= 5, $"UpdateS ran {run.Num("shapes")} times");
        Assert.Equal(2, run.Num("queue"));

        // Nothing was logged, and no script went into error (REQ-LUA-062).
        Assert.Empty(run.Log.Errors);
        Assert.Empty(run.Log.Warnings);
        Assert.Equal(0, run.Runtime.Scripts.Count(static script => script.Failed));
        Assert.All(run.Runtime.Scripts, script => Assert.True(script.Active, script.Name));
    }
}
