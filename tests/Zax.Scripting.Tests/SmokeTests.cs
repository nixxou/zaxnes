using Zax.Engine3D;

namespace Zax.Scripting.Tests;

/// <summary>First end-to-end checks of the runtime and of the binding layer.</summary>
public sealed class SmokeTests
{
    [Fact]
    public void A_chunk_runs_and_its_globals_survive()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Script("a", "n = (n or 0) + 1\nfunction Start() started = n end")
            .Start();

        Assert.Empty(session.Log.Errors);
        Assert.Equal(1.0, session.Runtime.EvaluateNumber("return started"));
    }

    [Fact]
    public void Globals_of_the_api_are_installed()
    {
        using ScriptSession session = new ScriptProfileBuilder().Script("a", "-- nothing").Load();

        Assert.Equal("userdata", session.Runtime.EvaluateText("return type(Frame)"));
        Assert.Equal("userdata", session.Runtime.EvaluateText("return type(Setting)"));
        Assert.Equal("table", session.Runtime.EvaluateText("return type(TLayer)"));
        Assert.Equal(3.0, session.Runtime.EvaluateNumber("return TLayer.L2"));
        Assert.Equal("nil", session.Runtime.EvaluateText("return type(io)"));
        Assert.Equal("function", session.Runtime.EvaluateText("return type(unpack)"));
    }

    [Fact]
    public void Value_types_build_and_compare()
    {
        using ScriptSession session = new ScriptProfileBuilder().Script("a", "-- nothing").Load();

        Assert.Equal(1.0, session.Runtime.EvaluateNumber("return Vector3(1, 2, 3).x"));
        Assert.Equal(
            1.0,
            session.Runtime.EvaluateNumber("if Vector3(1, 2) == Vector3(1, 2, 0) then return 1 else return 0 end"));
        Assert.Equal(5.0, session.Runtime.EvaluateNumber("return Vector3(3, 4, 0).magnitude"));
        Assert.Equal(-1.0, session.Runtime.EvaluateNumber("return Vector3.Cross(Vector3.back, Vector3(1, 0, 0)).y"));
        Assert.Equal(1.0, session.Runtime.EvaluateNumber("return Color(1, 0, 0).a"));
    }

    [Fact]
    public void Settings_are_live()
    {
        using ScriptSession session = new ScriptProfileBuilder().Script("a", "-- nothing").Load();

        session.Runtime.Evaluate("Setting.EmptyTile = true");
        Assert.True(session.Engine.Settings.EmptyTile);

        session.Runtime.Evaluate("Setting.Layer[3].CastShadow = true");
        Assert.True(session.Engine.Settings.Layer(3).CastShadow);

        session.Runtime.Evaluate("Clipping[1] = 16");
        Assert.Equal(16f, session.Engine.Settings.Clipping.Y);

        Assert.Equal(
            1.0,
            session.Runtime.EvaluateNumber("if Light == Setting.Light then return 1 else return 0 end"));
    }

    /// <summary>
    /// SPEC-09 REQ-PERS-008 §4.1.10 with AMD-020 (Q-052): the three static members of
    /// <c>app.engine_mode</c> are exposed in Lua, read only, and say that the engine always runs
    /// on a profile — the ROM mode is the editor's.
    /// </summary>
    [Fact] // REQ-PERS-008, Q-052
    public void The_engine_mode_members_of_paragraph_4_1_10_are_exposed_read_only()
    {
        using ScriptSession session = new ScriptProfileBuilder().Script("a", "-- nothing").Load();

        Assert.Equal("number", session.Runtime.EvaluateText("return type(Setting.EngineMode)"));
        Assert.Equal(1.0, session.Runtime.EvaluateNumber("return Setting.EngineMode"));
        Assert.Equal(
            1.0,
            session.Runtime.EvaluateNumber("if Setting.IsProfileMode then return 1 else return 0 end"));
        Assert.Equal(
            0.0,
            session.Runtime.EvaluateNumber("if Setting.IsRomMode then return 1 else return 0 end"));

        // Read only: an assignment is refused, and the value does not move.
        Assert.NotNull(Record.Exception(() => session.Runtime.Evaluate("Setting.EngineMode = 0")));
        Assert.Equal(1.0, session.Runtime.EvaluateNumber("return Setting.EngineMode"));
    }

    [Fact]
    public void Frame_hooks_see_the_instances()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A], tags: "t")
            .Script("a", """
                seen = 0
                function UpdateS()
                  seen = seen + 1
                  lastTag = shape.TagStr
                  offsetX = shape.Offset.x
                  shape.Offset.x = 5
                  afterX = shape.Offset.x
                end
                """)
            .Start();

        session.Frame(new Batch().BgCell(Img.A, 10, 5));

        Assert.Empty(session.Log.Errors);
        Assert.Equal(1.0, session.Runtime.EvaluateNumber("return seen"));
        Assert.Equal("t", session.Runtime.EvaluateText("return lastTag"));
        Assert.Equal(0.0, session.Runtime.EvaluateNumber("return offsetX"));
        Assert.Equal(5.0, session.Runtime.EvaluateNumber("return afterX"));
    }

    [Fact]
    public void Unknown_members_read_as_nil_and_writes_raise()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A], tags: "t")
            .Script("a", """
                function UpdateS()
                  typo = shape.hiden
                  ok, err = pcall(function () shape.NoSuch = 1 end)
                end
                """)
            .Start();

        session.Frame(new Batch().BgCell(Img.A, 10, 5));

        Assert.Equal("nil", session.Runtime.EvaluateText("return type(typo)"));
        Assert.Equal(0.0, session.Runtime.EvaluateNumber("if ok then return 1 else return 0 end"));
        Assert.Contains("cannot set NoSuch, no such field", session.Runtime.EvaluateText("return err"));
    }

    [Fact]
    public void Numbers_from_the_host_are_floats()
    {
        using ScriptSession session = new ScriptProfileBuilder().Script("a", "-- nothing").Start();
        session.Runtime.SetFrameGlobals(120, 2f);

        Assert.Equal("float", session.Runtime.EvaluateText("return math.type(frame)"));
        Assert.Equal("120", session.Runtime.EvaluateText("return tostring(frame)"));
        Assert.True(double.IsNaN(session.Runtime.EvaluateNumber("return math.fmod(5, Frame.shapeCount)")));
    }

    [Fact]
    public void A_hook_error_disables_the_script_and_the_game_goes_on()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Script("e", "function Update() local x = nil; return x.y end")
            .Script("g", "count = 0\nfunction Update() count = count + 1 end")
            .Start();

        session.Frame(new Batch(), 1);
        session.Frame(new Batch(), 2);
        session.Frame(new Batch(), 3);

        Assert.Single(session.Log.Errors);
        Assert.Contains("attempt to index", session.Log.Errors[0]);
        Assert.Contains("'e'", session.Log.Errors[0]);
        Assert.Equal(3.0, session.Runtime.EvaluateNumber("return count"));
        Assert.True(session.Runtime.FindScript("e")!.Failed);
    }

    [Fact]
    public void Identity_holds_across_accesses()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A], tags: "t")
            .Script("a", """
                function UpdateS()
                  sameShape = shape.Shape2D == shape.Shape2D
                  sameFrame = Frame:GetShape(0) == Frame:GetShape(0)
                  asKey = {}
                  asKey[shape.Shape2D] = 7
                  keyed = asKey[shape.Shape2D]
                end
                """)
            .Start();

        session.Frame(new Batch().BgCell(Img.A, 10, 5));

        Assert.Empty(session.Log.Errors);
        Assert.Equal(1.0, session.Runtime.EvaluateNumber("if sameShape then return 1 else return 0 end"));
        Assert.Equal(1.0, session.Runtime.EvaluateNumber("if sameFrame then return 1 else return 0 end"));
        Assert.Equal(7.0, session.Runtime.EvaluateNumber("return keyed"));
    }

    [Fact]
    public void Print_writes_the_dialect_format()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Script("a", """print("frame", 12.0, 0.4, true, nil)""")
            .Load();

        Assert.Equal("frame\t12\t0.4\ttrue\tnil", Assert.Single(session.Log.Prints));
    }

    [Fact]
    public void Build_pattern_runs_before_the_geometry()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A, Img.B], tags: "gear")
            .Script("build", "function BuildPattern() pattern.fillrate = 0.05 end", tags: "gear")
            .Load();

        EnginePattern pattern = session.Engine.Patterns.Permanent[0];
        Assert.Equal(0.05f, pattern.Instances[0].FillRate, 6);
    }
}
