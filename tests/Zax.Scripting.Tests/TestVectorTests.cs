using Zax.Engine3D;
using Zax.Profiles;

namespace Zax.Scripting.Tests;

/// <summary>The test vectors of SPEC-05 §5.2, TV-LUA-01 to TV-LUA-16.</summary>
public sealed class TestVectorTests
{
    /// <summary>
    /// TV-LUA-01: two scripts <c>b</c> (tags <c>t</c>) and <c>a</c> (tags <c>*</c>) in that file
    /// order, a frame with two scriptable instances <c>i1</c> (tag <c>t</c>) and <c>i2</c> (tag
    /// <c>u</c>). The chunks run in file order, the hooks in script order, and the
    /// <c>UpdateS</c> calls are grouped per script.
    /// </summary>
    [Fact]
    public void TV_LUA_01_hook_order()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A], tags: "t")
            .Pattern([Img.B], tags: "u")
            .Script("b", """
                log[#log + 1] = "b.chunk"
                function Start() log[#log + 1] = "b.Start" end
                function Update() log[#log + 1] = "b.Update" end
                function UpdateS() log[#log + 1] = "b.UpdateS(" .. shape.TagStr .. ")" end
                function LateUpdate() log[#log + 1] = "b.LateUpdate" end
                """, tags: "t")
            .Script("a", """
                log[#log + 1] = "a.chunk"
                function Start() log[#log + 1] = "a.Start" end
                function Update() log[#log + 1] = "a.Update" end
                function UpdateS() log[#log + 1] = "a.UpdateS(" .. shape.TagStr .. ")" end
                function LateUpdate() log[#log + 1] = "a.LateUpdate" end
                """, tags: "*")
            .Prelude("log = {}")
            .Load();

        session.Runtime.InvokeStart();
        session.Frame(new Batch().BgCell(Img.A, 4, 4).BgCell(Img.B, 12, 4), 1);

        Assert.Equal(
            [
                "b.chunk",
                "a.chunk",
                "a.Start",
                "b.Start",
                "a.Update",
                "b.Update",
                "a.UpdateS(t)",
                "a.UpdateS(u)",
                "b.UpdateS(t)",
                "a.LateUpdate",
                "b.LateUpdate",
            ],
            Log(session));
    }

    /// <summary>
    /// REQ-LUA-002: a script whose tags match no instance still gets <c>Start</c>,
    /// <c>Update</c> and <c>LateUpdate</c>, and never <c>UpdateS</c>.
    /// </summary>
    [Fact]
    public void A_script_linked_to_nothing_still_gets_the_frame_hooks_REQ_LUA_002()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A], tags: "t")
            .Script("only", """
                function Start() log[#log + 1] = "Start" end
                function Update() log[#log + 1] = "Update" end
                function UpdateS() log[#log + 1] = "UpdateS" end
                function LateUpdate() log[#log + 1] = "LateUpdate" end
                """, tags: "nothing")
            .Prelude("log = {}")
            .Load();

        session.Runtime.InvokeStart();
        session.Frame(new Batch().BgCell(Img.A, 4, 4), 1);

        Assert.Equal(["Start", "Update", "LateUpdate"], Log(session));
    }

    /// <summary>
    /// TV-LUA-02 and REQ-LUA-012: two scripts that define the same non-reserved global function
    /// share the last definition, and the first script's hook calls that one.
    /// </summary>
    [Fact]
    public void TV_LUA_02_function_collision()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Script("bg", """
                function Connect() return "bg" end
                function Start() picked = Connect() end
                """)
            .Script("foot_body", """function Connect(foot, body) return "foot_body" end""")
            .Start();

        Assert.Equal("foot_body", session.Runtime.EvaluateText("return picked"));
    }

    /// <summary>
    /// TV-LUA-03 and REQ-LUA-007: a disabled script gets <c>Start</c> and <c>End</c> but no
    /// frame hook, and what its <c>Start</c> did stays in place.
    /// </summary>
    [Fact]
    public void TV_LUA_03_disabled_script()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A], tags: "t")
            .Script(
                "hud",
                """
                function Start()
                  Setting.Layer[3].CastShadow = true
                  log[#log + 1] = "Start"
                end
                function Update() log[#log + 1] = "Update" end
                function UpdateS() log[#log + 1] = "UpdateS" end
                function End() log[#log + 1] = "End" end
                """,
                enabled: false)
            .Prelude("log = {}")
            .Load();

        session.Runtime.InvokeStart();
        session.Frame(new Batch().BgCell(Img.A, 4, 4), 1);
        session.Runtime.InvokeEnd();

        Assert.True(session.Engine.Settings.Layer(3).CastShadow);
        Assert.Equal(["Start", "End"], Log(session));
    }

    /// <summary>
    /// REQ-LUA-007: a script may switch another on; the change applies from the next hook call.
    /// </summary>
    [Fact]
    public void A_script_can_switch_another_on_REQ_LUA_007()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Script("ov", "function Update() log[#log + 1] = 'ov' end", enabled: false)
            .Script("scene", """
                function Start() sOv = Script:Get("ov") end
                function Update()
                  log[#log + 1] = "scene"
                  sOv.Enable = scene == 1
                end
                """)
            .Prelude("log = {}\nscene = 0")
            .Start();

        session.Frame(new Batch(), 1);
        session.Runtime.Evaluate("scene = 1");
        session.Frame(new Batch(), 2);
        session.Frame(new Batch(), 3);

        Assert.Equal(["scene", "scene", "ov", "scene"], Log(session));
    }

    /// <summary>
    /// TV-LUA-04: <c>Save</c> captures <c>PData[1]</c> to <c>PData[64]</c> as single-precision
    /// reals, with a numeric string converted and anything else stored as 0; <c>PData[0]</c> is
    /// neither saved nor restored.
    /// </summary>
    [Fact]
    public void TV_LUA_04_persistent_data()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Script("p", """
                function Save()
                  PData[1] = 0.1
                  PData[0] = 7
                  PData[2] = "12"
                  PData[3] = true
                  PData[4] = nil
                end
                function Load() loaded = 1 end
                """)
            .Start();

        session.Runtime.InvokeSave();
        byte[] block = session.Runtime.CapturePersistentData();
        Assert.Equal(PersistentData.BlockSize, block.Length);

        session.Runtime.Evaluate("PData[0] = 99; PData[1] = 5");
        session.Runtime.RestorePersistentData(block);
        session.Runtime.InvokeLoad();

        Assert.Equal(0.100000001490116, session.Runtime.ReadPersistentData(1), 12);
        Assert.Equal(12.0, session.Runtime.ReadPersistentData(2));
        Assert.Equal(0.0, session.Runtime.ReadPersistentData(3));
        Assert.Equal(0.0, session.Runtime.ReadPersistentData(4));
        Assert.Equal(99.0, session.Runtime.ReadPersistentData(0));
        Assert.Equal("float", session.Runtime.EvaluateText("return math.type(PData[2])"));
        Assert.Equal(1.0, session.Runtime.EvaluateNumber("return loaded"));
    }

    /// <summary>REQ-LUA-070: the shape of <c>PData</c> and the two helper functions.</summary>
    [Fact]
    public void Persistent_data_helpers_REQ_LUA_070()
    {
        using ScriptSession session = new ScriptProfileBuilder().Script("a", "-- nothing").Start();

        Assert.Equal(64.0, session.Runtime.EvaluateNumber("return #PData"));
        Assert.Equal(64.0, session.Runtime.EvaluateNumber("return pdata_count"));
        session.Runtime.Evaluate("PData[1] = 11; SetPData(64, 5)");
        Assert.Equal(5.0, session.Runtime.EvaluateNumber("return GetPData(0)"));
        Assert.Equal(5.0, session.Runtime.EvaluateNumber("return PData[1]"));
    }

    /// <summary>
    /// TV-LUA-05: the positions of a background instance of one tile at <c>pStart = (16, 32)</c>
    /// with depth 8, layer L1 and centred pivots.
    /// </summary>
    [Fact]
    public void TV_LUA_05_positions()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Grid(2, 2, [Img.A, Img.B, Img.C, Img.D], tags: "t", depth: 8)
            .Script("p", """
                function UpdateS()
                  bl = shape.BottomLeft
                  tr = shape.TopRight
                  bl2 = shape.BL
                  tr2 = shape.TR
                  size = shape.size
                  ori = shape.OriPos
                  pos = shape:GetPos()
                end
                """)
            .Start();

        // The four cells of the pattern at screen (16, 32) to (31, 47), the box
        // TV-LUA-05 describes.
        session.Frame(
            new Batch()
                .BgCell(Img.A, 2, 4)
                .BgCell(Img.B, 3, 4)
                .BgCell(Img.C, 2, 5)
                .BgCell(Img.D, 3, 5),
            1);

        Assert.Empty(session.Log.Errors);
        AssertVector(session, "bl", 16, 201, -4);
        AssertVector(session, "tr", 31, 216, 4);
        AssertVector(session, "bl2", 16, 200, -4);
        AssertVector(session, "tr2", 31, 215, 4);
        AssertVector(session, "size", 16, 16, 8);
        AssertVector(session, "ori", 23.5, 208.5, 0);
        AssertVector(session, "pos", 24, 208, 4);
    }

    /// <summary>TV-LUA-06: the strict overlap test and its slack.</summary>
    [Fact]
    public void TV_LUA_06_overlap()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A], tags: "a")
            .Pattern([Img.B], tags: "b")
            .Script("p", """
                function LateUpdate()
                  local ia = Frame:GetShapeWithTag("a")
                  local ib = Frame:GetShapeWithTag("b")
                  plain = ia:Overlap(ib)
                  slack = ia:Overlap(ib, 1)
                end
                """)
            .Start();

        // A at pStart (0, 0) pEnd (7, 7); B at pStart (7, 0) pEnd (14, 7).
        session.Frame(new Batch().Sprite(Img.A, 0, 0).Sprite(Img.B, 7, 0), 1);
        Assert.Empty(session.Log.Errors);
        Assert.False(session.Runtime.EvaluateNumber("if plain then return 1 else return 0 end") == 1.0);
        Assert.True(session.Runtime.EvaluateNumber("if slack then return 1 else return 0 end") == 1.0);

        // B moved to pStart (9, 0): even with the slack the boxes no longer touch.
        session.Frame(new Batch().Sprite(Img.A, 0, 0).Sprite(Img.B, 9, 0), 2);
        Assert.False(session.Runtime.EvaluateNumber("if slack then return 1 else return 0 end") == 1.0);
    }

    /// <summary>
    /// TV-LUA-07: the deformation a script predicts through <c>GetDeform</c>.
    /// </summary>
    [Fact]
    public void TV_LUA_07_deformation()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Grid(2, 2, [Img.A, Img.B, Img.C, Img.D], tags: "t", depth: 8)
            .Script("p", """
                function UpdateS()
                  shape.DeformAmp.content = Vector3(0, 0, 1)
                  for r = 0, 2 do
                    for c = 0, 3 do
                      shape.DeformSpeed[r + 4 * c] = 0
                    end
                  end
                  shape.DeformSpeed.m23 = 2
                  time = 0.25
                  d = shape:GetDeform(shape.OriPos)
                end
                """)
            .Start();

        session.Frame(
            new Batch()
                .BgCell(Img.A, 2, 4)
                .BgCell(Img.B, 3, 4)
                .BgCell(Img.C, 2, 5)
                .BgCell(Img.D, 3, 5),
            1);

        Assert.Empty(session.Log.Errors);
        Assert.Equal(0.0, session.Runtime.EvaluateNumber("return d.x"), 4);
        Assert.Equal(0.0, session.Runtime.EvaluateNumber("return d.y"), 4);
        Assert.Equal(7.02066, session.Runtime.EvaluateNumber("return d.z"), 4);
    }

    /// <summary>TV-LUA-08: the vector type.</summary>
    [Fact]
    public void TV_LUA_08_vectors()
    {
        using ScriptSession session = new ScriptProfileBuilder().Script("a", "-- nothing").Load();

        Assert.Equal(1.0, Truth(session, "Vector3(1, 2) == Vector3(1, 2, 0)"));
        Assert.Equal(0.0, session.Runtime.EvaluateNumber("return Vector3.Cross(Vector3.back, Vector3(1, 0, 0)).x"));
        Assert.Equal(-1.0, session.Runtime.EvaluateNumber("return Vector3.Cross(Vector3.back, Vector3(1, 0, 0)).y"));
        Assert.Equal(0.0, session.Runtime.EvaluateNumber("return Vector3.Cross(Vector3.back, Vector3(1, 0, 0)).z"));
        Assert.Equal(5.0, session.Runtime.EvaluateNumber("return Vector3(3, 4, 0).magnitude"));
        Assert.Equal(2.0, session.Runtime.EvaluateNumber("return (Vector3(1, 1, 1) * 2).y"));
    }

    /// <summary>TV-LUA-09: the quaternion type.</summary>
    [Fact]
    public void TV_LUA_09_quaternions()
    {
        using ScriptSession session = new ScriptProfileBuilder().Script("a", "-- nothing").Load();

        Assert.Equal(0.0, session.Runtime.EvaluateNumber("return Quaternion.Euler(0, 90, 0).eulerAngles.x"), 4);
        Assert.Equal(90.0, session.Runtime.EvaluateNumber("return Quaternion.Euler(0, 90, 0).eulerAngles.y"), 4);
        Assert.Equal(0.0, session.Runtime.EvaluateNumber("return Quaternion.Euler(0, 90, 0).eulerAngles.z"), 4);
        Assert.Equal(
            90.0,
            session.Runtime.EvaluateNumber(
                "return (Quaternion.Euler(0, 0, 30) * Quaternion.Euler(0, 0, 60)).eulerAngles.z"),
            4);
        Assert.Equal(
            330.0,
            session.Runtime.EvaluateNumber("return Quaternion.Euler(-30, 0, 0).eulerAngles.x"),
            4);
        Assert.Equal(
            0.0,
            session.Runtime.EvaluateNumber(
                "return (Quaternion.AngleAxis(90, Vector3(0, 1, 0)) * Vector3(1, 0, 0)).x"),
            4);
        Assert.Equal(
            -1.0,
            session.Runtime.EvaluateNumber(
                "return (Quaternion.AngleAxis(90, Vector3(0, 1, 0)) * Vector3(1, 0, 0)).z"),
            4);
        Assert.Equal(
            90.0,
            session.Runtime.EvaluateNumber(
                "return Quaternion.Lerp(Quaternion.identity, Quaternion.Euler(0, 90, 0), 2).eulerAngles.y"),
            4);
    }

    /// <summary>
    /// TV-LUA-10: a colour and the palette of the published frame. The colour of NES index
    /// <c>$16</c> is <c>#FC3800</c> (SPEC-04 REQ-E3D-251).
    /// </summary>
    [Fact]
    public void TV_LUA_10_colours_and_palette()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A], tags: "t")
            .Script("p", """
                function LateUpdate()
                  alpha = Color(1, 0, 0).a
                  published = Palette.FrameColorIndex(5)
                  current = Palette.FrameColorIndex(5, 1)
                  colour = Palette.FrameColor(5)
                end
                """)
            .Start();

        // Entry 5 of the frame palette is PPU byte 5 (REQ-E3D-250).
        Batch batch = new Batch().BgCell(Img.A, 4, 4).Palette(5, 0x16);
        batch.ConvertAndPublish(session.Engine, 1);

        // The first frame has nothing published yet, so the lagging read gives 0; the second
        // frame sees the palette of the first (REQ-LUA-087, DEC-05-Q4).
        batch.ConvertAndPublish(session.Engine, 2);

        Assert.Empty(session.Log.Errors);
        Assert.Equal(1.0, session.Runtime.EvaluateNumber("return alpha"));
        Assert.Equal(0x16, session.Runtime.EvaluateNumber("return published"));
        Assert.Equal(0x16, session.Runtime.EvaluateNumber("return current"));
        Assert.Equal(252.0 / 255.0, session.Runtime.EvaluateNumber("return colour.r"), 5);
        Assert.Equal(56.0 / 255.0, session.Runtime.EvaluateNumber("return colour.g"), 5);
        Assert.Equal(0.0, session.Runtime.EvaluateNumber("return colour.b"), 5);
        Assert.Equal(1.0, session.Runtime.EvaluateNumber("return colour.a"));
    }

    /// <summary>TV-LUA-11: every number the host hands over is a Lua float.</summary>
    [Fact]
    public void TV_LUA_11_numbers()
    {
        using ScriptSession session = new ScriptProfileBuilder().Script("a", "-- nothing").Start();
        session.Memory.Write(0x6C, 42);
        session.Runtime.SetFrameGlobals(120, 2f);

        Assert.Equal("float", session.Runtime.EvaluateText("return math.type(frame)"));
        Assert.Equal("120", session.Runtime.EvaluateText("return tostring(frame)"));
        Assert.True(double.IsNaN(session.Runtime.EvaluateNumber("return math.fmod(5, Frame.shapeCount)")));
        Assert.Equal(42.0, session.Runtime.EvaluateNumber("return Script:ReadMem(0x6C + 0.5)"));
        Assert.Equal(42.0, session.Runtime.EvaluateNumber("return Script:ReadMem(65536 + 0x6C)"));
        Assert.Equal("float", session.Runtime.EvaluateText("return math.type(Script:ReadMem(0x6C))"));
    }

    /// <summary>
    /// TV-LUA-12: a hook that raises is logged once, the script stops receiving hooks, the game
    /// goes on, and a relaunch of the same game calls neither <c>End</c> nor <c>Start</c> of it.
    /// </summary>
    [Fact]
    public void TV_LUA_12_errors()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Script("e", """
                function Update() local x = nil; return x.y end
                function Start() log[#log + 1] = "e.Start" end
                function End() log[#log + 1] = "e.End" end
                function LateUpdate() log[#log + 1] = "e.LateUpdate" end
                """)
            .Script("g", "function Update() log[#log + 1] = 'g.Update' end")
            .Prelude("log = {}")
            .Load();

        session.Runtime.InvokeStart();
        session.Frame(new Batch(), 1);
        session.Frame(new Batch(), 2);

        string error = Assert.Single(session.Log.Errors);
        Assert.Contains("'e'", error);
        Assert.Contains("Update", error);
        Assert.Contains("attempt to index", error);
        Assert.Contains("e:1:", error);

        // The relaunch of the same game: End then Start, with the error flag kept.
        session.Runtime.InvokeEnd();
        session.Runtime.InvokeStart();

        Assert.Equal(["e.Start", "g.Update", "g.Update"], Log(session));
    }

    /// <summary>TV-LUA-13: unknown members.</summary>
    [Fact]
    public void TV_LUA_13_unknown_members()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A], tags: "t")
            .Script("p", """
                function UpdateS()
                  typo1 = shape.hiden
                  typo2 = shape2D.Hiden
                  okSet, errSet = pcall(function () shape.NoSuch = 1 end)
                  okPos, errPos = pcall(function () return Frame:GetShape(Frame.shapeCount) end)
                  okRead, errRead = pcall(function () return shape.Pos end)
                  okWrite, errWrite = pcall(function () shape.Pos = Vector3.zero end)
                end
                """)
            .Start();

        session.Frame(new Batch().BgCell(Img.A, 4, 4), 1);

        Assert.Equal("nil", session.Runtime.EvaluateText("return type(typo1)"));
        Assert.Equal("nil", session.Runtime.EvaluateText("return type(typo2)"));
        Assert.Equal(0.0, Truth(session, "okSet"));
        Assert.Contains("cannot set NoSuch, no such field", session.Runtime.EvaluateText("return errSet"));
        Assert.Equal(0.0, Truth(session, "okPos"));
        Assert.Equal(1.0, Truth(session, "okRead"));
        Assert.Equal(0.0, Truth(session, "okWrite"));
    }

    /// <summary>TV-LUA-14: the shared list of the tag queries.</summary>
    [Fact]
    public void TV_LUA_14_shared_list()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A], tags: "a")
            .Pattern([Img.B], tags: "b")
            .Script("p", """
                function LateUpdate()
                  local x = Frame:GetShapesWithTag("a")
                  n = x.Count
                  local y = Frame:GetShapesWithTag("b")
                  afterwards = x.Count
                  same = x == y
                end
                """)
            .Start();

        session.Frame(
            new Batch()
                .Sprite(Img.A, 8, 16)
                .Sprite(Img.A, 40, 16)
                .Sprite(Img.A, 72, 16)
                .Sprite(Img.B, 104, 16),
            1);

        Assert.Empty(session.Log.Errors);
        Assert.Equal(3.0, session.Runtime.EvaluateNumber("return n"));
        Assert.Equal(1.0, session.Runtime.EvaluateNumber("return afterwards"));
        Assert.Equal(1.0, Truth(session, "same"));
    }

    /// <summary>
    /// TV-LUA-15: the profile's settings are applied after the chunks, so a chunk-level write
    /// is overwritten; the same write in <c>Start</c> survives (REQ-LUA-015).
    /// </summary>
    [Fact]
    public void TV_LUA_15_load_order()
    {
        ScriptProfileBuilder builder = new ScriptProfileBuilder()
            .Pattern([Img.A])
            .Script("chunk", "Setting.EmptyTile = true");
        builder.Settings.EmptyTile = false;

        using ScriptSession session = builder.Load();
        Assert.False(session.Engine.Settings.EmptyTile);

        ScriptProfileBuilder other = new ScriptProfileBuilder()
            .Pattern([Img.A])
            .Script("start", "function Start() Setting.EmptyTile = true end");
        other.Settings.EmptyTile = false;

        using ScriptSession second = other.Start();
        Assert.True(second.Engine.Settings.EmptyTile);
    }

    /// <summary>
    /// TV-LUA-16: the savestate sequence, <c>End</c> then <c>PData</c> then <c>Start</c> then
    /// <c>Load</c>, in script order (REQ-LUA-072, DEC-R2-T6).
    /// </summary>
    [Fact]
    public void TV_LUA_16_savestate_sequence()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Script("_scene", """
                scene = 0
                function Save() PData[6] = scene; log[#log + 1] = "Save" end
                function Start() log[#log + 1] = "Start(" .. tostring(PData[6]) .. ")" end
                function Load() scene = PData[6]; log[#log + 1] = "Load" end
                function End() log[#log + 1] = "End" end
                """)
            .Prelude("log = {}")
            .Start();

        session.Runtime.Evaluate("scene = 2");
        session.Runtime.InvokeSave();
        byte[] block = session.Runtime.CapturePersistentData();

        session.Runtime.Evaluate("scene = 0; PData[6] = 0");
        session.Runtime.InvokeEnd();
        session.Runtime.RestorePersistentData(block);
        session.Runtime.InvokeStart();
        session.Runtime.InvokeLoad();

        Assert.Equal(["Start(0)", "Save", "End", "Start(2)", "Load"], Log(session));
        Assert.Equal(2.0, session.Runtime.EvaluateNumber("return scene"));
    }

    /// <summary>
    /// REQ-LUA-073 and §4.6: the persistent block is 256 bytes of little-endian singles.
    /// </summary>
    [Fact]
    public void Persistent_block_layout_REQ_LUA_073()
    {
        using ScriptSession session = new ScriptProfileBuilder().Script("a", "-- nothing").Start();
        session.Runtime.Evaluate("PData[1] = 1.5; PData[64] = -2");

        byte[] block = session.Runtime.CapturePersistentData();
        Assert.Equal(256, block.Length);
        Assert.Equal(1.5f, BitConverter.ToSingle(block, 0));
        Assert.Equal(-2f, BitConverter.ToSingle(block, 252));
    }

    private static double Truth(ScriptSession session, string expression)
        => session.Runtime.EvaluateNumber($"if {expression} then return 1 else return 0 end");

    private static void AssertVector(ScriptSession session, string name, double x, double y, double z)
    {
        Assert.Equal(x, session.Runtime.EvaluateNumber($"return {name}.x"), 4);
        Assert.Equal(y, session.Runtime.EvaluateNumber($"return {name}.y"), 4);
        Assert.Equal(z, session.Runtime.EvaluateNumber($"return {name}.z"), 4);
    }

    private static string[] Log(ScriptSession session)
    {
        int count = (int)session.Runtime.EvaluateNumber("return #log");
        string[] entries = new string[count];
        for (int i = 0; i < count; i++)
        {
            entries[i] = session.Runtime.EvaluateText($"return log[{i + 1}]");
        }

        return entries;
    }
}
