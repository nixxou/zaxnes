using Zax.Scripting;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Acceptance tests for the part of AMD-040 that lot V2-2 carries: the <c>CS.Nes3D.Info</c>
/// table of REQ-LUA-112, <c>Camera.ResetDistance</c> of REQ-LUA-113 and the two export
/// no-operations of REQ-LUA-115.
/// </summary>
/// <remarks>
/// REQ-LUA-114 — the new <c>Setting</c> members — belongs to the run-ahead and space-bending
/// lots and is deliberately absent here.
/// </remarks>
public sealed class Amd040LuaTests
{
    private static LuaRun Empty() => new LuaProfile().Script("a", string.Empty).Start();

    /// <summary>
    /// REQ-LUA-112 — <c>CS.Nes3D.Info.Version</c> starts with a decimal integer of at least 2
    /// and <c>Title</c> is not <c>"3dSenMaker"</c>, which is what a 2.0 profile script reads to
    /// pick its path.
    /// </summary>
    [Fact] // REQ-LUA-112
    public void The_info_table_answers_the_version_and_the_title()
    {
        using LuaRun run = Empty();

        Assert.Equal("table", run.TypeOf("CS"));
        Assert.Equal("table", run.TypeOf("CS.Nes3D"));
        Assert.Equal("table", run.TypeOf("CS.Nes3D.Info"));

        // The criterion, written as the script of a 2.0 profile writes it.
        Assert.True(run.Flag("tonumber(CS.Nes3D.Info.Version:match('^%d+')) >= 2"));
        Assert.True(run.Flag("CS.Nes3D.Info.Title ~= '3dSenMaker'"));

        Assert.Equal("2.0", run.Text("CS.Nes3D.Info.Version"));
        Assert.Equal("Zaxnes", run.Text("CS.Nes3D.Info.Title"));
    }

    /// <summary>
    /// REQ-LUA-112 — every other field of the three tables reads as nil, without an error.
    /// </summary>
    [Fact] // REQ-LUA-112
    public void Every_other_field_of_the_info_table_reads_as_nil()
    {
        using LuaRun run = Empty();

        Assert.Equal("nil", run.TypeOf("CS.Nes3D.Info.AppId"));
        Assert.Equal("nil", run.TypeOf("CS.Nes3D.Autre"));
        Assert.Equal("nil", run.TypeOf("CS.Autre"));
        Assert.Equal("nil", run.TypeOf("CS.Nes3D.Info.Version2"));
        Assert.Empty(run.Log.Errors);
    }

    /// <summary>
    /// REQ-LUA-112 — a write into the three tables raises no error and has no effect on the
    /// program: it does not even change what the next read gives.
    /// </summary>
    [Fact] // REQ-LUA-112
    public void A_write_into_the_info_table_has_no_effect_and_no_error()
    {
        using LuaRun run = Empty();

        Assert.Null(run.Error("CS.Nes3D.Info.Version = '1.0'"));
        Assert.Null(run.Error("CS.Nes3D.Info.Title = '3dSenMaker'"));
        Assert.Null(run.Error("CS.Nes3D.Info.AppId = 7"));
        Assert.Null(run.Error("CS.Nes3D = nil"));
        Assert.Null(run.Error("CS.Quelque = 1"));

        Assert.Equal("2.0", run.Text("CS.Nes3D.Info.Version"));
        Assert.Equal("Zaxnes", run.Text("CS.Nes3D.Info.Title"));
        Assert.Equal("nil", run.TypeOf("CS.Nes3D.Info.AppId"));
        Assert.Equal("nil", run.TypeOf("CS.Quelque"));

        // The metatable is hidden, so the shield cannot be taken off either.
        Assert.Equal("CS", run.Text("getmetatable(CS.Nes3D.Info)"));
    }

    /// <summary>
    /// REQ-LUA-113 — <c>Camera.ResetDistance</c> is the framed distance <c>d₀</c> of REQ-SCN-069
    /// at the current FOV, before the bound of REQ-SCN-011: 189.65 at 70°, 320.60 at 45°. It is
    /// readable at any time and a write to <c>Camera.Distance</c> leaves it alone.
    /// </summary>
    [Fact] // REQ-LUA-113, REQ-SCN-069
    public void The_reset_distance_follows_the_fov_and_ignores_the_current_distance()
    {
        using LuaRun run = Empty();

        Assert.Equal(70f, run.Settings.Fov, 3);
        Assert.Equal(189.65, run.Num("Camera.ResetDistance"), 0.05);

        // Reading it does not need a reset, and the current distance has no say in it.
        run.Do("Camera.Distance = 50");
        Assert.Equal(50, run.Num("Camera.Distance"), 3);
        Assert.Equal(189.65, run.Num("Camera.ResetDistance"), 0.05);

        // It follows the FOV.
        run.Do("Setting.Fov = 45");
        Assert.Equal(320.60, run.Num("Camera.ResetDistance"), 0.05);
        run.Do("Setting.Fov = 95");
        Assert.Equal(121.69, run.Num("Camera.ResetDistance"), 0.05);
        run.Do("Setting.Fov = 70");
        Assert.Equal(189.65, run.Num("Camera.ResetDistance"), 0.05);
    }

    /// <summary>
    /// REQ-LUA-113 — the member is read only, which the wording of REQ-LUA-042 reports.
    /// </summary>
    [Fact] // REQ-LUA-113, REQ-LUA-042
    public void The_reset_distance_cannot_be_written()
    {
        using LuaRun run = Empty();

        Assert.Contains(
            "cannot set ResetDistance, it is read only",
            run.MustFail("Camera.ResetDistance = 10"),
            StringComparison.Ordinal);
    }

    /// <summary>
    /// REQ-LUA-113 — the formula the host-free camera uses is the one the scene owns
    /// (REQ-SCN-069): the two copies of <c>132.7964 / tan(FOV / 2)</c> agree.
    /// </summary>
    [Fact] // REQ-LUA-113, REQ-SCN-069
    public void The_framed_distance_of_the_script_camera_is_the_scene_formula()
    {
        foreach (float fov in new[] { 29.1f, 45f, 70f, 90f, 95f, 150f })
        {
            Assert.Equal(
                Zax.Render.SceneConstants.ResetDistance(fov),
                ScriptCamera.FramedDistance(fov),
                1e-3f);
        }
    }

    /// <summary>
    /// REQ-LUA-115 — <c>BuildExportMeshes</c> and <c>ReleaseExportMeshes</c> exist on
    /// <c>Instance</c>, do nothing and raise no error: a 2.0 script that calls them keeps every
    /// hook it registered (REQ-LUA-062).
    /// </summary>
    [Fact] // REQ-LUA-115
    public void The_export_mesh_methods_exist_and_do_nothing()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "gear")
            .Script(
                "gear",
                """
                built = 0
                function BuildPattern()
                  built = built + 1
                  pattern:BuildExportMeshes()
                  pattern:ReleaseExportMeshes()
                  depth = pattern.sizeZ
                end
                function UpdateS()
                  kept = shape
                end
                """,
                tags: "gear")
            .Start();

        // The hook ran to its end, so the two calls raised nothing (REQ-LUA-060 would have
        // stopped the chunk and REQ-LUA-062 would have dropped the hooks).
        Assert.Equal(1, run.Num("built"));
        Assert.Equal(3, run.Num("depth"));
        Assert.Empty(run.Log.Errors);

        // And on a frame instance, where nothing visible changes either.
        run.Frame(new SyntheticBatch().Background('A', 16, 32));
        Assert.Equal("userdata", run.TypeOf("kept"));

        float before = run.Engine.Patterns.Permanent[0].Instances[0].Scale.Z;
        Assert.Null(run.Error("kept:BuildExportMeshes()"));
        Assert.Null(run.Error("kept:ReleaseExportMeshes()"));
        Assert.Equal(before, run.Engine.Patterns.Permanent[0].Instances[0].Scale.Z, 5);
        Assert.Empty(run.Log.Errors);

        // They take no argument and mean nothing: extra arguments are tolerated.
        Assert.Null(run.Error("kept:BuildExportMeshes(1, 'x')"));
    }
}
