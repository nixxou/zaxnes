using System.Globalization;
using Zax.Scripting;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for SPEC-05 §3.9 to §3.12: the persistent data, the exposed API and
/// its member tables, the derived quantities, and the dialect rules.
/// </summary>
public sealed class LuaApiTests
{
    private static LuaRun Empty() => new LuaProfile().Script("a", string.Empty).Start();

    private static LuaRun Framed(out Zax.Engine3D.ConversionFrame frame)
    {
        LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script("a", "function UpdateS() kept = shape; kept2D = shape2D end", tags: "*")
            .Start();
        frame = run.Frame(new SyntheticBatch().Background('A', 16, 32));
        return run;
    }

    // ------------------------------------------------- §3.9 persistent data, savestates

    /// <summary>
    /// REQ-LUA-070 — <c>PData</c> is an ordinary Lua table of 64 zeros, <c>pdata_count</c> is 64
    /// and <c>GetPData</c>/<c>SetPData</c> wrap with <c>(i mod 64) + 1</c>.
    /// </summary>
    [Fact] // REQ-LUA-070
    public void PData_is_a_table_of_64_zeros_with_wrapping_accessors()
    {
        using LuaRun run = Empty();

        Assert.Equal("table", run.TypeOf("PData"));
        Assert.Equal(64, run.Num("#PData"));
        Assert.Equal(64, run.Num("pdata_count"));
        for (int i = 1; i <= 64; i++)
        {
            Assert.Equal(0, run.Num($"PData[{i}]"));
        }

        run.Do("PData[1] = 11; PData[64] = 99");
        Assert.Equal(11, run.Num("GetPData(0)"));
        Assert.Equal(99, run.Num("GetPData(63)"));

        run.Do("SetPData(64, 5)");
        Assert.Equal(5, run.Num("PData[1]"));
        run.Do("SetPData(0, 7)");
        Assert.Equal(7, run.Num("PData[1]"));
    }

    /// <summary>
    /// REQ-LUA-071, TV-LUA-04 — a save calls <c>Save</c> of every active script in script order
    /// and <b>then</b> captures the 64 entries as single-precision reals; a non-numeric value
    /// (nil included) stores 0, a numeric string its value, and <c>PData[0]</c> and
    /// <c>PData[65]</c> are not stored.
    /// </summary>
    [Fact] // REQ-LUA-071, REQ-LUA-073, TV-LUA-04
    public void TV_LUA_04_the_persistent_block_is_captured_after_Save()
    {
        using LuaRun run = new LuaProfile()
            .Script("!prelude", "order = {}", tags: string.Empty)
            .Script("b", "function Save() order[#order + 1] = 'b' end")
            .Script("a", """
                function Save()
                  order[#order + 1] = 'a'
                  PData[1] = 0.1
                  PData[0] = 7
                  PData[2] = "12"
                  PData[3] = true
                  PData[4] = nil
                  PData[65] = 5
                end
                """)
            .Start();

        byte[] block = run.Runtime.CapturePersistentData();
        run.Runtime.InvokeSave();
        byte[] afterSave = run.Runtime.CapturePersistentData();

        Assert.Equal("a|b", run.Text("table.concat(order, '|')"));
        Assert.Equal(256, block.Length);
        Assert.Equal(256, afterSave.Length);

        Assert.Equal(0.1f, BitConverter.ToSingle(afterSave, 0), 1e-9f);
        Assert.Equal(12f, BitConverter.ToSingle(afterSave, 4), 1e-6f);
        Assert.Equal(0f, BitConverter.ToSingle(afterSave, 8));
        Assert.Equal(0f, BitConverter.ToSingle(afterSave, 12));

        // Restoring puts the single-precision values back as floats.
        run.Do("PData[0] = 99; PData[1] = 5");
        run.Runtime.RestorePersistentData(afterSave);

        Assert.Equal(
            (double)0.1f,
            run.Num("PData[1]"),
            12);
        Assert.Equal(12, run.Num("PData[2]"));
        Assert.Equal(0, run.Num("PData[3]"));
        Assert.Equal(0, run.Num("PData[4]"));
        Assert.Equal(99, run.Num("PData[0]"));
        Assert.Equal("float", run.Text("math.type(PData[2])"));
    }

    /// <summary>
    /// REQ-LUA-071 — a <b>disabled</b> script does not receive <c>Save</c>.
    /// </summary>
    [Fact] // REQ-LUA-071
    public void Save_skips_an_inactive_script()
    {
        using LuaRun run = new LuaProfile()
            .Script("a", "ran = 0\nfunction Save() ran = ran + 1 end", enabled: false)
            .Start();

        run.Runtime.InvokeSave();
        Assert.Equal(0, run.Num("ran"));
    }

    /// <summary>
    /// REQ-LUA-073 — the block is 64 IEEE-754 single-precision reals in little-endian order,
    /// 256 bytes: <c>PData[1] = 1.5</c> and <c>PData[64] = −2</c> land in the first and last
    /// four bytes.
    /// </summary>
    [Fact] // REQ-LUA-073
    public void The_persistent_block_is_256_little_endian_floats()
    {
        using LuaRun run = Empty();
        run.Do("PData[1] = 1.5; PData[64] = -2");

        byte[] block = run.Runtime.CapturePersistentData();

        Assert.Equal(256, block.Length);
        Assert.Equal(new byte[] { 0x00, 0x00, 0xC0, 0x3F }, block[..4]);
        Assert.Equal(new byte[] { 0x00, 0x00, 0x00, 0xC0 }, block[252..]);
        Assert.True(BitConverter.IsLittleEndian);
    }

    /// <summary>
    /// REQ-LUA-072 — on a savestate load the 64 entries are put back <b>first</b>, then
    /// <c>Start</c> of every script that is not in error, then <c>Load</c> of every active
    /// script, both in script order. The auto part of the [ROM] criterion: the sequence is
    /// driven through the real session of the host.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LUA-072, REQ-LOOP-040, REQ-LOOP-044
    public void A_savestate_load_restores_PData_then_Start_then_Load()
    {
        Zax.Profiles.GameProfile profile = Zax.Profiles.GameProfile.CreateEmpty();
        profile.Title = "verifier-savestate";
        profile.Scripts.Add(new Zax.Profiles.ProfileScript("!prelude", true, "order = {}", string.Empty));
        profile.Scripts.Add(new Zax.Profiles.ProfileScript(
            "_scene",
            true,
            """
            function Save() PData[6] = scene end
            function Start() order[#order + 1] = 'Start:' .. math.floor(PData[6]) end
            function Load() order[#order + 1] = 'Load:' .. math.floor(PData[6]); scene = PData[6] end
            function End() order[#order + 1] = 'End' end
            """,
            "*"));

        using HostDriver host = HostDriver.Create(profile);
        string id = host.EntryIdByTitle("verifier-savestate");
        host.Launch(id, host.WriteRom(LoopRom.Counter()));

        Zax.Scripting.ProfileScriptRuntime scripts = host.Session!.Scripts;
        scripts.Evaluate("scene = 2; order = {}");

        host.Run(Zax.App.Loop.ZaxCommands.QuickSave);
        scripts.Evaluate("scene = 0");
        host.Run(Zax.App.Loop.ZaxCommands.QuickLoad);

        // The profile does not change, so the Lua state survives: End, then PData, then Start,
        // then Load (REQ-LUA-072, DEC-R2-T6).
        Zax.Scripting.ProfileScriptRuntime reloaded = host.Session!.Scripts;
        Assert.Equal(
            "End|Start:2|Load:2",
            reloaded.EvaluateText("return table.concat(order, '|')"));
        Assert.Equal(2, reloaded.EvaluateNumber("return scene"));
        Assert.Equal(2, reloaded.ReadPersistentData(6));
    }

    /// <summary>
    /// REQ-LUA-075 — <c>retro_unserialize</c> restores the 64 entries then calls <c>Load</c> of
    /// every active script, with <b>no</b> <c>End</c> and no <c>Start</c>; <c>retro_serialize</c>
    /// calls <c>Save</c> once per active script.
    /// </summary>
    [Fact] // REQ-LUA-075, REQ-LOOP-063
    public void The_libretro_serialisation_calls_Save_and_Load_but_never_Start_or_End()
    {
        Zax.Profiles.GameProfile profile = Zax.Profiles.GameProfile.CreateEmpty();
        profile.Scripts.Add(new Zax.Profiles.ProfileScript("!prelude", true, "order = {}", string.Empty));
        profile.Scripts.Add(new Zax.Profiles.ProfileScript(
            "a",
            true,
            """
            function Save() order[#order + 1] = 'Save'; PData[1] = 3 end
            function Load() order[#order + 1] = 'Load' end
            function Start() order[#order + 1] = 'Start' end
            function End() order[#order + 1] = 'End' end
            """,
            "*"));
        profile.Scripts.Add(new Zax.Profiles.ProfileScript(
            "b",
            true,
            """
            function Save() order[#order + 1] = 'Save' end
            function Load() order[#order + 1] = 'Load' end
            """,
            "*"));
        profile.Scripts.Add(new Zax.Profiles.ProfileScript(
            "c",
            false,
            """
            function Save() order[#order + 1] = 'off' end
            function Load() order[#order + 1] = 'off' end
            """,
            "*"));

        Zax.Render.SceneState scene = new();
        using SessionDriver session = SessionDriver.CreateOnProfile(profile, scene);
        session.Start();
        session.Scripts.Evaluate("order = {}");

        byte[] block = session.CaptureScriptState();
        session.Scripts.RestorePersistentData(block);
        session.Scripts.InvokeLoad();

        Assert.Equal(
            "Save|Save|Load|Load",
            session.Scripts.EvaluateText("return table.concat(order, '|')"));
        Assert.Equal(3, session.Scripts.ReadPersistentData(1));
    }

    // -------------------------------------------------------- §3.10 the exposed API

    /// <summary>
    /// REQ-LUA-080, REQ-LUA-051 — every global of §4.1 is installed before the first chunk runs.
    /// The list is read out of the specification document itself.
    /// </summary>
    [SpecFact] // REQ-LUA-080, REQ-LUA-051
    public void Every_global_of_the_specification_is_installed()
    {
        using LuaRun run = Empty();

        // shape, shape2D, pattern and self are nil until a hook assigns them (§4.1).
        HashSet<string> assignedByHooks = ["shape", "shape2D", "pattern", "self"];
        List<string> missing = [];

        IReadOnlyList<SpecGlobalRow> globals = SpecTables.Globals();
        Assert.True(globals.Count >= 40, $"only {globals.Count} global rows were read from §4.1.");

        foreach (SpecGlobalRow global in globals)
        {
            if (assignedByHooks.Contains(global.Name))
            {
                continue;
            }

            if (run.TypeOf(global.Name) == "nil")
            {
                missing.Add($"{global.Name} ({global.Priority})");
            }
        }

        Assert.Empty(missing);
    }

    /// <summary>
    /// REQ-LUA-080 — for every member row of §4.2 and §4.3, the runtime declares the member with
    /// the genre and the access right the document gives. P1 and P2 rows are required; missing
    /// P3 rows are reported in the failure message but do not fail the test, since §3.10 only
    /// says they SHOULD be provided.
    /// </summary>
    [SpecFact] // REQ-LUA-080, REQ-LUA-081
    public void Every_member_of_the_specification_tables_is_declared_with_the_right_genre()
    {
        ScriptApiSurface surface = ScriptApiSurface.Describe();
        IReadOnlyList<SpecMemberRow> rows = SpecTables.Members();
        Assert.True(rows.Count >= 330, $"only {rows.Count} member rows were read from §4.2–§4.3.");

        List<string> required = [];
        List<string> optional = [];

        foreach (SpecMemberRow row in rows)
        {
            foreach (string typeName in Implementation(row.Type))
            {
                string? problem = Check(surface, typeName, row);
                if (problem is null)
                {
                    continue;
                }

                (row.Priority == "P3" ? optional : required).Add(problem);
            }
        }

        Assert.True(
            required.Count == 0,
            $"{required.Count} required rows of §4.2–§4.3 are not honoured:\n"
                + string.Join('\n', required)
                + (optional.Count > 0 ? $"\n(and {optional.Count} P3 rows)" : string.Empty));
    }

    /// <summary>
    /// REQ-LUA-080 — the P3 rows of §4.2 and §4.3 are reported, not required. This test records
    /// how many of them the runtime provides.
    /// </summary>
    [SpecFact] // REQ-LUA-080
    public void The_optional_members_of_the_specification_tables_are_also_provided()
    {
        ScriptApiSurface surface = ScriptApiSurface.Describe();
        List<string> missing = [];

        foreach (SpecMemberRow row in SpecTables.Members())
        {
            if (row.Priority != "P3")
            {
                continue;
            }

            foreach (string typeName in Implementation(row.Type))
            {
                if (Check(surface, typeName, row) is { } problem)
                {
                    missing.Add(problem);
                }
            }
        }

        Assert.True(missing.Count == 0, string.Join('\n', missing));
    }

    /// <summary>REQ-LUA-046 — the constant tables of §4.4 hold the documented integers.</summary>
    [SpecFact] // REQ-LUA-046, REQ-LUA-080
    public void Every_constant_of_section_4_4_has_the_documented_value()
    {
        using LuaRun run = Empty();
        Dictionary<string, string> tables = new(StringComparer.Ordinal)
        {
            ["TLayerEnum"] = "TLayer",
            ["AlignEnum"] = "Align",
            ["CamModeEnum"] = "CamMode",
        };

        IReadOnlyList<(string Table, string Name, double Value)> constants = SpecTables.Constants();
        Assert.True(constants.Count >= 13, $"only {constants.Count} constant rows were read.");

        foreach ((string table, string name, double value) in constants)
        {
            string global = tables[table];
            Assert.Equal(value, run.Num($"{global}.{name}"));
        }
    }

    /// <summary>
    /// REQ-LUA-081 — an instance method is called with <c>:</c>; calling it with <c>.</c> without
    /// passing the object raises a Lua error. A class-table function is called with <c>.</c>.
    /// </summary>
    [Fact] // REQ-LUA-081
    public void Instance_methods_need_the_colon_and_table_functions_the_dot()
    {
        using LuaRun run = Framed(out _);

        Assert.Null(run.Error("local s = Frame:GetShape(0)"));
        Assert.NotNull(run.Error("local s = Frame.GetShape(0)"));
        Assert.Null(run.Error("local c = Palette.FrameColor(0)"));
        Assert.Null(run.Error("local p = Pattern.GetPatternWithTag(\"t\")"));
    }

    /// <summary>
    /// REQ-LUA-082 — <c>CData[i]</c> for <c>i</c> = 0 to 3 is row 3 of <c>DeformSpeed</c>, for
    /// <c>i ≥ 4</c> row 3 of <c>DeformOffset</c> at column <c>i mod 4</c>; a negative index
    /// raises.
    /// </summary>
    [Fact] // REQ-LUA-082
    public void CData_maps_onto_row_three_of_the_two_deformation_matrices()
    {
        using LuaRun run = Framed(out _);

        run.Do("kept.CData[2] = 7");
        Assert.Equal(7, run.Num("kept.DeformSpeed.m32"));

        run.Do("kept.DeformOffset.m31 = 5");
        Assert.Equal(5, run.Num("kept.CData[5]"));
        Assert.Equal(5, run.Num("kept.CData[9]"));

        run.Do("kept.CData[0] = 1; kept.CData[1] = 2; kept.CData[3] = 4");
        Assert.Equal(1, run.Num("kept.DeformSpeed.m30"));
        Assert.Equal(2, run.Num("kept.DeformSpeed.m31"));
        Assert.Equal(4, run.Num("kept.DeformSpeed.m33"));

        run.Do("kept.CData[4] = 8");
        Assert.Equal(8, run.Num("kept.DeformOffset.m30"));

        Assert.NotNull(run.Error("local x = kept.CData[-1]"));
    }

    /// <summary>
    /// TV-LUA-05 — the derived positions of an instance of a background shape at
    /// <c>pStart = (16, 32)</c>, <c>pEnd = (31, 47)</c>, depth 8, layer L1 front, no offset and
    /// the pivots at the centre (REQ-LUA-083).
    /// </summary>
    [Fact] // REQ-LUA-083, TV-LUA-05
    public void TV_LUA_05_the_derived_positions_of_the_test_vector()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["AA", "AA"], tags: "t", depth: 8)
            .Script("a", "function UpdateS() s = shape end", tags: "*")
            .Start();

        // A 2 × 2 tile block at cell (2, 4): pixels x 16..31, y 32..47.
        run.Frame(new SyntheticBatch()
            .Background('A', 16, 32)
            .Background('A', 24, 32)
            .Background('A', 16, 40)
            .Background('A', 24, 40));

        Assert.Equal(16, run.Num("s.Shape2D.pStart.x"));
        Assert.Equal(32, run.Num("s.Shape2D.pStart.y"));
        Assert.Equal(31, run.Num("s.Shape2D.pEnd.x"));
        Assert.Equal(47, run.Num("s.Shape2D.pEnd.y"));

        Assert.Equal(16, run.Num("s.BottomLeft.x"));
        Assert.Equal(201, run.Num("s.BottomLeft.y"));
        Assert.Equal(-4, run.Num("s.BottomLeft.z"));
        Assert.Equal(31, run.Num("s.TopRight.x"));
        Assert.Equal(216, run.Num("s.TopRight.y"));
        Assert.Equal(4, run.Num("s.TopRight.z"));

        Assert.Equal(16, run.Num("s.BL.x"));
        Assert.Equal(200, run.Num("s.BL.y"));
        Assert.Equal(-4, run.Num("s.BL.z"));
        Assert.Equal(31, run.Num("s.TR.x"));
        Assert.Equal(215, run.Num("s.TR.y"));
        Assert.Equal(4, run.Num("s.TR.z"));

        Assert.Equal(16, run.Num("s.size.x"));
        Assert.Equal(16, run.Num("s.size.y"));
        Assert.Equal(8, run.Num("s.size.z"));

        Assert.Equal(23.5, run.Num("s.OriPos.x"), 4);
        Assert.Equal(208.5, run.Num("s.OriPos.y"), 4);
        Assert.Equal(0, run.Num("s.OriPos.z"), 4);

        Assert.Equal(24, run.Num("s:GetPos().x"), 4);
        Assert.Equal(208, run.Num("s:GetPos().y"), 4);
        Assert.Equal(4, run.Num("s:GetPos().z"), 4);
    }
    /// <summary>
    /// TV-LUA-06, REQ-LUA-083 point 4 — <c>Overlap</c> uses strict inequalities on the screen
    /// boxes and <c>Overlap(o, d)</c> widens the test by <c>d</c>. Two background shapes one
    /// pixel apart (boxes x in [0, 7] and x in [8, 15]) therefore do not overlap, do not overlap
    /// at <c>d = 1</c> (7 + 1 &gt; 8 is false) and do overlap at <c>d = 2</c>.
    /// <c>Overlap(rect)</c> uses the origin <c>BottomLeft</c> and <c>OverlapRect(rect)</c> the
    /// origin <c>BL</c>, one unit lower: the one-pixel gap REQ-LUA-083 keeps on purpose.
    /// </summary>
    [Fact] // REQ-LUA-083, TV-LUA-06
    public void TV_LUA_06_overlap_uses_strict_inequalities()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Pattern(["B"], tags: "t")
            .Script("s", string.Empty, tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 0, 0).Background('B', 8, 0));

        run.Do("""
            a = Frame:GetShape(0)
            b = Frame:GetShape(1)
            if a.Shape2D.pStart.x > b.Shape2D.pStart.x then a, b = b, a end
            """);

        Assert.Equal(0, run.Num("a.Shape2D.pStart.x"));
        Assert.Equal(7, run.Num("a.Shape2D.pEnd.x"));
        Assert.Equal(8, run.Num("b.Shape2D.pStart.x"));
        Assert.Equal(15, run.Num("b.Shape2D.pEnd.x"));

        Assert.False(run.Flag("a:Overlap(b)"));
        Assert.False(run.Flag("a:Overlap(b, 1)"));
        Assert.True(run.Flag("a:Overlap(b, 2)"));
        Assert.True(run.Flag("a:Overlap(a)"));

        double bottom = run.Num("a.BottomLeft.y");
        Assert.Equal(bottom - 1, run.Num("a.BL.y"), 4);

        Assert.True(run.Flag($"a:Overlap(Rect(4, {Fmt.N(bottom + 4)}, 8, 8))"));
        Assert.False(run.Flag($"a:Overlap(Rect(8, {Fmt.N(bottom)}, 8, 8))"));
        Assert.False(run.Flag($"a:Overlap(Rect(0, {Fmt.N(bottom + 8)}, 8, 8))"));
        Assert.True(run.Flag($"a:OverlapRect(Rect(0, {Fmt.N(bottom - 1)}, 8, 8))"));
    }


    /// <summary>
    /// REQ-LUA-083 point 5 — <c>ContainsTag</c> answers true for <c>"*"</c> and for an exact,
    /// case-sensitive tag of the instance.
    /// </summary>
    [Fact] // REQ-LUA-083
    public void ContainsTag_matches_the_star_and_an_exact_tag()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "gear,gear1")
            .Script("s", string.Empty, tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 0, 80));

        Assert.True(run.Flag("Frame:GetShape(0):ContainsTag(\"*\")"));
        Assert.True(run.Flag("Frame:GetShape(0):ContainsTag(\"gear\")"));
        Assert.True(run.Flag("Frame:GetShape(0):ContainsTag(\"gear1\")"));
        Assert.False(run.Flag("Frame:GetShape(0):ContainsTag(\"Gear\")"));
        Assert.False(run.Flag("Frame:GetShape(0):ContainsTag(\"other\")"));
    }

    /// <summary>
    /// REQ-LUA-084 — the frame queries read the scriptable instances of the frame being
    /// converted, and <c>GetShapeWithPos</c> with two arguments always returns nil because the
    /// omitted tag is the empty string.
    /// </summary>
    [Fact] // REQ-LUA-084
    public void The_frame_queries_read_the_scriptable_instances()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "a")
            .Pattern(["B"], tags: "a,b")
            .Script("s", string.Empty, tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 0, 80).Background('B', 32, 80));

        Assert.Equal(2, run.Num("Frame.shapeCount"));
        Assert.Equal(2, run.Num("Frame:GetShapesWithTag(\"a\").Count"));
        Assert.True(run.Flag("Frame:GetShapeWithTag(\"b\"):ContainsTag(\"b\")"));
        run.Do("""
            first = Frame:GetShape(0)
            px = (first.BottomLeft.x + first.TopRight.x) / 2
            py = (first.BottomLeft.y + first.TopRight.y) / 2
            twoArguments = Frame:GetShapeWithPos(px, py)
            threeArguments = Frame:GetShapeWithPos(px, py, "*")
            """);
        Assert.Equal("nil", run.TypeOf("twoArguments"));
        Assert.True(run.Flag("threeArguments == first"));
        Assert.NotNull(run.Error("local x = Frame:GetShape(Frame.shapeCount)"));
        Assert.Equal(2, run.Num("FrameManager.Ins.shapeCount"));
        Assert.True(run.Flag("FrameManager.Ins == Frame"));
    }

    /// <summary>
    /// REQ-LUA-085 — <c>Pattern.GetPatternWithTag</c> returns a <b>pattern instance</b>, walking
    /// the permanent patterns in registry order then their instances; temporary patterns are not
    /// walked.
    /// </summary>
    [Fact] // REQ-LUA-085
    public void The_pattern_queries_walk_the_permanent_registry_only()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "square", instanceCount: 2)
            .Pattern(["B"], tags: "temp", permanent: false)
            .Script("s", string.Empty, tags: "*")
            .Start();

        Assert.Equal("userdata", run.TypeOf("Pattern.GetPatternWithTag(\"square\")"));
        Assert.Equal(2, run.Num("Pattern.GetPatternsWithTag(\"square\").Count"));
        Assert.Equal("nil", run.TypeOf("Pattern.GetPatternWithTag(\"temp\")"));

        // It is a pattern instance: its refPattern3D is itself (§4.2).
        Assert.True(run.Flag(
            "Pattern.GetPatternWithTag(\"square\").refPattern3D == Pattern.GetPatternWithTag(\"square\")"));

        // Disabling the pattern takes it out of the candidates.
        run.Do("Pattern.GetPatternWithTag(\"square\").pattern.Disable = true");
        Assert.True(run.Engine.Patterns.Permanent[0].Disabled);
    }

    /// <summary>
    /// REQ-LUA-086 — the injection functions choose their variant on the first argument: an
    /// instance gives back the instance of the same index of the new shape, a pattern gives back
    /// the new shape with every instance active.
    /// </summary>
    [Fact] // REQ-LUA-086
    public void The_injection_functions_choose_their_variant_on_the_first_argument()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "cloud", instanceCount: 2)
            .Script("s", string.Empty, tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 0, 80));

        run.Do("""
            before = Frame.shapeCount
            p = Pattern.GetPatternWithTag("cloud")
            fromInstance = Frame:AddNewShape(p, Vector3(100, 100, 0), 5)
            fromPattern = Frame:AddNewShape(p.pattern, Vector3(120, 100, 0), 5)
            """);

        // The instance variant hands back one instance, the pattern variant a whole shape.
        Assert.Equal(0, run.Num("fromInstance.Index"));
        Assert.Equal(2, run.Num("fromPattern.Count"));
        Assert.Equal(5, run.Num("fromInstance.Palette"));

        // An injected shape is not scriptable: the scriptable count does not move.
        Assert.Equal(run.Num("before"), run.Num("Frame.shapeCount"));
    }

    /// <summary>
    /// REQ-LUA-087 — the frame palette: <c>sel</c> absent or 0 reads the published frame, a
    /// non-zero <c>sel</c> the frame being converted.
    /// </summary>
    [Fact] // REQ-LUA-087
    public void The_frame_palette_selects_the_published_or_the_current_frame()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script(
                "s",
                """
                function Update()
                  current = Palette.FrameColorIndex(5, 1)
                  published = Palette.FrameColorIndex(5)
                  explicitZero = Palette.FrameColorIndex(5, 0)
                  alias = Palette.GetColorIndex(5)
                  colour = Palette.FrameColor(5)
                end
                """,
                tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Palette(5, 0x16).Background('A', 0, 80), counter: 1);
        run.Frame(new SyntheticBatch().Palette(5, 0x27).Background('A', 0, 80), counter: 2);

        // During the second frame's hooks the current frame holds 0x27 and the published one —
        // the frame converted before it — still holds 0x16: the one-frame lag of DEC-05-Q4.
        Assert.Equal(0x27, run.Num("current"));
        Assert.Equal(0x16, run.Num("published"));
        Assert.Equal(0x16, run.Num("explicitZero"));
        Assert.Equal(0x16, run.Num("alias"));

        Zax.Engine3D.Rgb24 expected = Zax.Engine3D.NesPalette.Color(0x16);
        Assert.Equal(expected.R / 255.0, run.Num("colour.r"), 4);
        Assert.Equal(expected.G / 255.0, run.Num("colour.g"), 4);
        Assert.Equal(expected.B / 255.0, run.Num("colour.b"), 4);
        Assert.Equal(1, run.Num("colour.a"), 4);

        // SetColorIndex with a non-zero selector writes the frame being converted.
        run.Do("Palette.SetColorIndex(5, 0x30, 1)");
        Assert.Equal(0x30, run.Num("Palette.FrameColorIndex(5, 1)"));
    }

    /// <summary>
    /// TV-LUA-10 — <c>Color(1, 0, 0).a</c> is 1, and the published frame colour of entry 5 of
    /// the TV-E3D-09 palette is <c>Color(252/255, 56/255, 0, 1)</c>, the NES colour 0x16.
    /// </summary>
    [Fact] // REQ-LUA-087, REQ-LUA-096, TV-LUA-10
    public void TV_LUA_10_colours_and_the_frame_palette()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script("s", "function Update() colour = Palette.FrameColor(5) end", tags: "*")
            .Start();

        Assert.Equal(1, run.Num("Color(1, 0, 0).a"));

        run.Frame(new SyntheticBatch().Palette(5, 0x16).Background('A', 0, 80), counter: 1);
        run.Frame(new SyntheticBatch().Palette(5, 0x16).Background('A', 0, 80), counter: 2);

        Assert.Equal(252 / 255.0, run.Num("colour.r"), 4);
        Assert.Equal(56 / 255.0, run.Num("colour.g"), 4);
        Assert.Equal(0, run.Num("colour.b"), 4);
    }

    /// <summary>
    /// REQ-LUA-088 — the camera: <c>Rot</c> is the bound orientation of the current mode in
    /// degrees, <c>Distance</c> is bounded to <c>[0, 512 × SceneScale]</c>, <c>Mode</c> is 0 or
    /// 1 and <c>SetTarget</c>/<c>LookAt</c> use scene coordinates.
    /// </summary>
    [Fact] // REQ-LUA-088
    public void The_camera_api_has_the_documented_effects()
    {
        using LuaRun run = Empty();

        run.Do("Camera.Rot.x = 30; Camera.Rot.y = -20");
        Assert.Equal(30f, run.Camera.Angles.X, 4);
        Assert.Equal(-20f, run.Camera.Angles.Y, 4);

        run.Do("Camera.Distance = 1000");
        Assert.Equal(512, run.Num("Camera.Distance"));
        run.Do("Camera.Distance = -5");
        Assert.Equal(0, run.Num("Camera.Distance"));

        run.Do("Camera:SetTarget(128, 120, 0)");
        Assert.Equal(128, run.Num("Camera.LookAt.x"));
        Assert.Equal(120, run.Num("Camera.LookAt.y"));
        Assert.Equal(0, run.Num("Camera.LookAt.z"));

        run.Do("Camera:SetTarget(Vector3(10, 20, 30))");
        Assert.Equal(10, run.Num("Camera:GetTarget().x"));

        run.Do("Camera.Mode = 1");
        Assert.Equal(1, run.Num("Camera.Mode"));
        Assert.True(run.Flag("Camera.IsFPV"));
        Assert.False(run.Flag("Camera.IsTPV"));
        run.Do("Camera:SetTPV()");
        Assert.Equal(0, run.Num("Camera.Mode"));
        run.Do("Camera:ToggleMode()");
        Assert.Equal(1, run.Num("Camera.Mode"));

        int resets = run.Camera.ResetCount;
        run.Do("Camera:ResetView()");
        Assert.Equal(resets + 1, run.Camera.ResetCount);
    }

    /// <summary>
    /// REQ-LUA-089 — the settings globals always designate the current objects, and the
    /// documented access paths work.
    /// </summary>
    [Fact] // REQ-LUA-089
    public void The_setting_globals_designate_the_current_objects()
    {
        using LuaRun run = Empty();

        Assert.True(run.Flag("Light == Setting.Light"));
        Assert.True(run.Flag("Layer == Setting.Layer"));
        Assert.True(run.Flag("Clipping == Setting.Clipping"));

        run.Do("Setting.EmptyTile = true");
        Assert.True(run.Settings.EmptyTile);
        run.Do("Setting.Clipping[0] = 16.1");
        Assert.Equal(16.1f, run.Settings.Clipping(0), 4);
        Assert.Equal(16.1, run.Num("Clipping.x"), 4);
        run.Do("Setting.Layer[3].CastShadow = true; Setting.Layer[2].Offset = 7")
            ;
        Assert.True(run.Settings.Layer(3).CastShadow);
        Assert.Equal(7f, run.Settings.Layer(2).Offset, 4);
        run.Do("Light.Direction = Vector3(1, -1, 2); Light.Intensity = 1.2");
        Assert.Equal(1.2f, run.Settings.Light.Intensity, 4);
        Assert.Equal(2f, run.Settings.Light.Direction.Z, 4);
        run.Do("Setting.CustomBgColor = Color(0, 0, 0)");
        Assert.Equal(0, run.Num("Setting.CustomBgColor.r"));
        Assert.Equal(0, run.Num("Setting:GetInputType(0)"));
        run.Do("Setting:SetInputType(0, 1)");
        Assert.Equal(1, run.Num("Setting:GetInputType(1)"));
    }

    /// <summary>
    /// AMD-031 §3 — a script that reads or writes the TV box setting raises no error: the
    /// reading is always « none » and the writing has no effect.
    /// </summary>
    [Fact] // AMD-031 §3
    public void AMD_031_the_TV_box_setting_reads_none_and_ignores_every_write()
    {
        using LuaRun run = Empty();

        // « Aucun » is 0, both before and after a write of any of the old model numbers.
        Assert.Equal(0, run.Num("Setting.TVBox"));
        foreach (int written in new[] { 1, 2, 3, -1 })
        {
            Assert.Null(run.Error($"Setting.TVBox = {written}"));
            Assert.Equal(0, run.Num("Setting.TVBox"));
            Assert.Equal(0, run.Settings.TVBox);
        }

        // The notification identifier of SPEC-09 §4.3 is still published (REQ-PERS-014), so a
        // script that watches the setting still runs.
        Assert.Equal("TVBoxId", run.Text("Setting.TVBoxId"));
    }

    /// <summary>
    /// REQ-LUA-090 — the backdrop members read and write the state of SPEC-06, and the state
    /// goes back to its defaults at the end of each game.
    /// </summary>
    [Fact] // REQ-LUA-090
    public void The_backdrop_members_read_and_write_and_reset_at_the_end_of_a_game()
    {
        using LuaRun run = Empty();

        run.Do("""
            Backdrop.Enable = true
            Backdrop.OfsZ = 17.1
            Backdrop.RecvShadow = false
            Backdrop.SizeX = 384
            Backdrop.Color = Color(1, 0, 0)
            """);

        Assert.Equal(17.1, run.Num("Backdrop.OfsZ"), 4);
        Assert.True(run.Backdrop.Enabled);
        Assert.False(run.Backdrop.ReceiveShadow);
        Assert.Equal(384f, run.Backdrop.Size.X, 4);
        Assert.Equal(17.1f, run.Backdrop.Offset.Z, 4);

        run.Backdrop.Reset();

        Assert.Equal(0, run.Num("Backdrop.OfsZ"));
        Assert.False(run.Flag("Backdrop.Enable"));
        Assert.True(run.Flag("Backdrop.RecvShadow"));
        Assert.Equal(256, run.Num("Backdrop.SizeX"));
        Assert.Equal(240, run.Num("Backdrop.SizeY"));
        Assert.Equal(1, run.Num("Backdrop.SizeZ"));
        Assert.Equal(0, run.Num("Backdrop.Color.r"));
        Assert.Equal(1, run.Num("Backdrop.Color.a"));
    }

    /// <summary>
    /// REQ-LUA-091 — the script pad, and the VR members that must return zero vectors on a
    /// desktop without VR.
    /// </summary>
    [Fact] // REQ-LUA-091, REQ-LUA-095
    public void The_script_pad_and_the_VR_members_are_neutral_on_desktop()
    {
        using LuaRun run = Empty();

        run.Do("ScriptPad:Enable(true); ScriptPad.PadIndex = 1");
        Assert.True(run.Runtime.ScriptPadEnabled);
        Assert.Equal(1, run.Runtime.ScriptPadIndex);
        Assert.Equal(1, run.Num("ScriptPad.PadIndex"));
        Assert.Equal(0, run.Num("ScriptPad.State"));

        Assert.Equal(0, run.Num("ScriptPad:HandPos(0).y"));
        Assert.Equal(0, run.Num("ScriptPad:RotPos(0).x"));
        Assert.Equal(0, run.Num("ScriptPad.Head.vel.x"));
        Assert.Equal(0, run.Num("ScriptPad.LeftHand.pos.x"));
        Assert.Equal(0, run.Num("ScriptPad.LH.rot.y"));
        Assert.Equal(0, run.Num("ScriptPad.RightHand.acc.z"));
        Assert.Equal(0, run.Num("ScriptPad.RH.pos.y"));
    }

    /// <summary>REQ-LUA-092 — the two player objects report the action masks of SPEC-08.</summary>
    [Fact] // REQ-LUA-092
    public void The_gamepad_objects_report_the_player_masks()
    {
        using LuaRun run = Empty();
        run.Input.SetPlayerState(0, 32);
        run.Input.SetPlayerState(1, 8);

        Assert.Equal(32, run.Num("Gamepad[0].State"));
        Assert.Equal(8, run.Num("Gamepad[1].State"));
        Assert.Equal(40, run.Num("Gamepad.State"));
        Assert.True(run.Flag("Gamepad[0].State == 32"));
        Assert.True(run.Flag("Gamepad1 == Gamepad[0]"));
        Assert.True(run.Flag("Gamepad2 == Gamepad[1]"));
        Assert.True(run.Flag("Gamepad[0]:Pressed(32)"));
        Assert.False(run.Flag("Gamepad[0]:Pressed(8)"));
        Assert.Equal(32, run.Num("Gamepad:GetInput(0)"));
    }

    /// <summary>
    /// REQ-LUA-093 — <c>NotiCenter.Notify</c> raises a notification with the given duration and
    /// severity, 4 s and normal by default; <c>Show</c> is <c>Notify(msg, 4, severity)</c>.
    /// </summary>
    [Fact] // REQ-LUA-093
    public void The_notification_centre_takes_its_defaults_from_the_specification()
    {
        using LuaRun run = Empty();

        run.Do("NotiCenter.Notify(\"Use Double Punch To Start/UpperCut\")");
        run.Do("NotiCenter.Notify(\"warn\", 2, 1)");
        run.Do("NotiCenter.Show(\"shown\")");
        run.Do("NotiCenter.Show(\"error\", 2)");

        Assert.Equal(4, run.Notifications.Count);

        Assert.Equal("Use Double Punch To Start/UpperCut", run.Notifications[0].Message);
        Assert.Equal(4f, run.Notifications[0].Seconds, 4);
        Assert.Equal(ScriptNotificationSeverity.Normal, run.Notifications[0].Severity);

        Assert.Equal(2f, run.Notifications[1].Seconds, 4);
        Assert.Equal(ScriptNotificationSeverity.Warning, run.Notifications[1].Severity);

        Assert.Equal(4f, run.Notifications[2].Seconds, 4);
        Assert.Equal(ScriptNotificationSeverity.Normal, run.Notifications[2].Severity);

        Assert.Equal(4f, run.Notifications[3].Seconds, 4);
        Assert.Equal(ScriptNotificationSeverity.Error, run.Notifications[3].Severity);
    }

    /// <summary>REQ-LUA-094 — the core memory is read and written without side effects.</summary>
    [Fact] // REQ-LUA-094
    public void The_core_memory_is_readable_and_writable()
    {
        using LuaRun run = Empty();
        run.Memory.Write(0x70, 0x42);

        Assert.Equal(0x42, run.Num("Script:ReadMem(0x70)"));
        run.Do("Script:WriteMem(0x70, 0x99)");
        Assert.Equal(0x99, run.Memory.Read(0x70));
        Assert.Equal(0x99, run.Num("Script:ReadMem(0x70)"));
    }

    /// <summary>
    /// REQ-LUA-095 — <c>Video.SceneScale</c> reads 1, a write has no geometric effect but a
    /// value other than 1 and other than the last one written triggers the view reset of
    /// REQ-SCN-002; the profile metadata is stored for the session.
    /// </summary>
    [Fact] // REQ-LUA-095
    public void SceneScale_reads_one_and_resets_the_view_and_the_metadata_is_stored()
    {
        using LuaRun run = Empty();

        int resets = run.Camera.ResetCount;
        run.Do("Video.SceneScale = 0.02");
        Assert.Equal(1, run.Num("Video.SceneScale"));
        Assert.Equal(1, run.Num("Setting.SceneScale"));
        Assert.True(run.Camera.ResetCount > resets, "the write did not reset the view.");

        run.Do("Profile.Maker = \"Geod\"");
        Assert.Equal("Geod", run.Text("Profile.Maker"));
        run.Do("Profile.Title = \"T\"; Profile.GameDev = \"G\"; Profile.Des = \"D\"");
        Assert.Equal("T", run.Text("Profile.Title"));
        Assert.Equal("G", run.Text("Profile.GameDev"));
        Assert.Equal("D", run.Text("Profile.Des"));
        Assert.Equal("string", run.TypeOf("Profile.Id"));
        Assert.Equal("string", run.TypeOf("Profile.Ver"));
        Assert.Equal("boolean", run.TypeOf("Profile.IsV2Format"));

        run.Do("Profile:RenewId()");
        Assert.Equal(12, run.Num("#Profile.Id"));
        run.Do("RenewId()");
        Assert.Equal(12, run.Num("#Profile.Id"));

        // Setting.VrRuntime is always 0 and SceneRatio is 1 on desktop.
        Assert.Equal(0, run.Num("Setting.VrRuntime"));
        Assert.Equal(1, run.Num("Setting.SceneRatio"));
    }

    /// <summary>
    /// TV-LUA-08 — the <c>Vector3</c> constructors, constants and operators.
    /// </summary>
    [Fact] // REQ-LUA-096, REQ-LUA-102, TV-LUA-08
    public void TV_LUA_08_the_vector_value_type()
    {
        using LuaRun run = Empty();

        Assert.True(run.Flag("Vector3(1, 2) == Vector3(1, 2, 0)"));
        Assert.Equal(0, run.Num("Vector3.Cross(Vector3.back, Vector3(1, 0, 0)).x"), 5);
        Assert.Equal(-1, run.Num("Vector3.Cross(Vector3.back, Vector3(1, 0, 0)).y"), 5);
        Assert.Equal(0, run.Num("Vector3.Cross(Vector3.back, Vector3(1, 0, 0)).z"), 5);
        Assert.Equal(5, run.Num("Vector3(3, 4, 0).magnitude"), 5);
        Assert.Equal(2, run.Num("(Vector3(1, 1, 1) * 2).y"), 5);
        Assert.Equal(2, run.Num("(2 * Vector3(1, 1, 1)).y"), 5);
        Assert.Equal(0.5, run.Num("(Vector3(1, 1, 1) / 2).y"), 5);
        Assert.Equal(-1, run.Num("(-Vector3(1, 1, 1)).x"), 5);
        Assert.Equal(3, run.Num("(Vector3(1, 1, 1) + Vector3(2, 2, 2)).x"), 5);
        Assert.Equal(-1, run.Num("(Vector3(1, 1, 1) - Vector3(2, 2, 2)).x"), 5);
        Assert.Equal(3, run.Num("Vector3(1, 1, 1).sqrMagnitude"), 5);
        Assert.Equal(1, run.Num("Vector3(0, 5, 0).normalized.y"), 5);
        Assert.Equal(0, run.Num("Vector3(0, 0, 0).normalized.y"), 5);

        Assert.Equal(0, run.Num("Vector3.zero.x"));
        Assert.Equal(1, run.Num("Vector3.one.x"));
        Assert.Equal(-1, run.Num("Vector3.back.z"));
        Assert.Equal(1, run.Num("Vector3.forward.z"));
        Assert.Equal(1, run.Num("Vector3.up.y"));
        Assert.Equal(-1, run.Num("Vector3.down.y"));
        Assert.Equal(-1, run.Num("Vector3.left.x"));
        Assert.Equal(1, run.Num("Vector3.right.x"));
        Assert.Equal(3, run.Num("Vector3.Dot(Vector3.one, Vector3.one)"), 5);
        Assert.Equal(5, run.Num("Vector3.Distance(Vector3.zero, Vector3(3, 4, 0))"), 5);
        Assert.Equal(1, run.Num("Vector3.Lerp(Vector3.zero, Vector3(2, 0, 0), 0.5).x"), 5);
        Assert.Equal(2, run.Num("Vector3.Lerp(Vector3.zero, Vector3(2, 0, 0), 5).x"), 5);
    }

    /// <summary>
    /// TV-LUA-09 — the quaternion functions and operators.
    /// </summary>
    [Fact] // REQ-LUA-096, TV-LUA-09
    public void TV_LUA_09_the_quaternion_value_type()
    {
        using LuaRun run = Empty();

        Assert.Equal(0, run.Num("Quaternion.Euler(0, 90, 0).eulerAngles.x"), 3);
        Assert.Equal(90, run.Num("Quaternion.Euler(0, 90, 0).eulerAngles.y"), 3);
        Assert.Equal(0, run.Num("Quaternion.Euler(0, 90, 0).eulerAngles.z"), 3);

        Assert.Equal(
            90,
            run.Num("(Quaternion.Euler(0, 0, 30) * Quaternion.Euler(0, 0, 60)).eulerAngles.z"),
            3);
        Assert.Equal(330, run.Num("Quaternion.Euler(-30, 0, 0).eulerAngles.x"), 3);

        Assert.Equal(0, run.Num("(Quaternion.AngleAxis(90, Vector3(0, 1, 0)) * Vector3(1, 0, 0)).x"), 4);
        Assert.Equal(0, run.Num("(Quaternion.AngleAxis(90, Vector3(0, 1, 0)) * Vector3(1, 0, 0)).y"), 4);
        Assert.Equal(-1, run.Num("(Quaternion.AngleAxis(90, Vector3(0, 1, 0)) * Vector3(1, 0, 0)).z"), 4);

        Assert.Equal(
            90,
            run.Num("Quaternion.Lerp(Quaternion.identity, Quaternion.Euler(0, 90, 0), 2).eulerAngles.y"),
            3);
        Assert.Equal(1, run.Num("Quaternion.identity.w"), 5);
        Assert.True(run.Flag("Quaternion.AngleAxis(90, Vector3.zero) == Quaternion.identity"));
        Assert.Equal(
            45,
            run.Num("Quaternion.Slerp(Quaternion.identity, Quaternion.Euler(0, 90, 0), 0.5).eulerAngles.y"),
            2);
    }

    /// <summary>
    /// REQ-LUA-096, REQ-LUA-102 — the other value types and their operators, plus the rule that
    /// comparing a vector value with a number or nil is false and never an error.
    /// </summary>
    [Fact] // REQ-LUA-096, REQ-LUA-102
    public void The_other_value_types_follow_their_definitions()
    {
        using LuaRun run = Empty();

        Assert.Equal(5, run.Num("Vector2(3, 4).magnitude"), 5);
        Assert.Equal(0, run.Num("Vector2.zero.x"));
        Assert.Equal(1, run.Num("Vector2.one.y"));
        Assert.True(run.Flag("Vector2(1, 2) == Vector2(1, 2)"));

        Assert.Equal(1, run.Num("Color(1, 0, 0).r"));
        Assert.Equal(1, run.Num("Color.white.g"));
        Assert.Equal(0, run.Num("Color.black.g"));
        Assert.True(run.Flag("Color(1, 0, 0, 1) == Color(1, 0, 0)"));
        Assert.Equal(0.5, run.Num("(Color(1, 1, 1) * 0.5).r"), 5);
        Assert.Equal(1, run.Num("(Color(0.5, 0, 0) + Color(0.5, 0, 0)).r"), 5);
        Assert.Equal(0, run.Num("(Color(0.5, 0, 0) - Color(0.5, 0, 0)).r"), 5);

        Assert.Equal(3, run.Num("IntVector2(3, 4).x"));
        Assert.Equal(3, run.Num("IntVector2(3.7, 4).x"));
        Assert.Equal(0, run.Num("IntVector2().x"));
        Assert.Equal(0, run.Num("IntVector2.zero.x"));
        Assert.Equal(1, run.Num("IntVector2.one.x"));
        Assert.Equal(1, run.Num("(IntVector2(3, 4) / 2).x"));
        Assert.Equal(2, run.Num("(IntVector2(3, 4) / 2).y"));
        Assert.Equal(4, run.Num("(IntVector2(3, 4) + IntVector2(1, 1)).x"));
        Assert.True(run.Flag("IntVector2(1, 2) == IntVector2(1, 2)"));
        Assert.Equal(7, run.Num("IntVector3(7, 8, 9).x"));

        Assert.Equal(1, run.Num("Rect(1, 2, 3, 4).x"));
        Assert.Equal(4, run.Num("Rect(1, 2, 3, 4).height"));
        Assert.True(run.Flag("Rect(0, 0, 4, 4):Overlaps(Rect(2, 2, 4, 4))"));
        Assert.False(run.Flag("Rect(0, 0, 4, 4):Overlaps(Rect(4, 0, 4, 4))"));

        // Comparing a value vector with a number or nil is false, not an error.
        Assert.False(run.Flag("Vector3.zero == nil"));
        Assert.False(run.Flag("Vector3.zero == 0"));
        Assert.Null(run.Error("local p = Vector3.one; local b = p ~= Vector3.zero"));
    }

    /// <summary>
    /// REQ-LUA-097 — the texture slot: <c>w</c> and <c>h</c> give the texture size in pixels and
    /// assigning another instance's slot textures the frame instance with it for that frame.
    /// </summary>
    [Fact] // REQ-LUA-097
    public void The_texture_slot_is_readable_and_assignable()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["AAAA", "AAAA"], tags: "bg")
            .Pattern(["B"], tags: "fg")
            .Script("s", string.Empty, tags: "*")
            .Start();

        // A 4 × 2 tile pattern: its slot is 32 pixels wide and 16 high.
        Assert.Equal(32, run.Num("Pattern.GetPatternWithTag(\"bg\").slot.w"));
        Assert.Equal(16, run.Num("Pattern.GetPatternWithTag(\"bg\").slot.h"));

        run.Frame(new SyntheticBatch().Background('B', 0, 80));

        run.Do("""
            target = Frame:GetShape(0)
            source = Pattern.GetPatternWithTag("bg")
            before = target.slot
            target.slot = source.slot
            after = target.slot
            same = (after == source.slot)
            changed = (before ~= after)
            """);

        Assert.True(run.Flag("same"));
        Assert.True(run.Flag("changed"));
        Assert.Equal(32, run.Num("target.slot.w"));
    }

    // ------------------------------------------------------- §3.11 dialect compatibility

    /// <summary>
    /// REQ-LUA-100, REQ-LUA-101 — the runtime is Lua 5.4, accepts the two profiles that use
    /// <c>goto</c>, provides the 5.1 globals REQ-LUA-101 point 4 asks for, and neutralises the
    /// division-by-zero difference.
    /// </summary>
    [Fact] // REQ-LUA-100, REQ-LUA-101
    public void The_dialect_differences_are_neutralised()
    {
        using LuaRun run = Empty();

        Assert.Equal("Lua 5.4", run.Text("_VERSION"));

        // goto and labels compile (REQ-LUA-100).
        Assert.Null(run.Error("do local i = 0; ::top:: i = i + 1; if i < 3 then goto top end end"));

        // The 5.1 globals of point 4.
        Assert.Equal(1, run.Num("(unpack({1, 2}))"));
        Assert.Equal(8, run.Num("math.pow(2, 3)"), 6);
        Assert.Equal(8, run.Num("math.ldexp(1, 3)"), 6);
        Assert.Equal(2, run.Num("math.log10(100)"), 6);
        Assert.Equal(2, run.Num("table.getn({1, 2})"));
        Assert.Equal("function", run.TypeOf("loadstring"));
        Assert.Equal(7, run.Num("loadstring(\"return 7\")()"));

        // math.fmod and % by a host zero give nan, not an error (point 2).
        Assert.True(double.IsNaN(run.Num("math.fmod(5, Frame.shapeCount)")));
        Assert.True(double.IsNaN(run.Num("5 % Frame.shapeCount")));

        // math.atan with one argument (point 3).
        Assert.Equal(Math.Atan(1.0), run.Num("math.atan(1)"), 6);
    }

    /// <summary>
    /// REQ-LUA-100 — the two profiles of the corpus that use <c>goto</c> and labels compile.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-LUA-100
    public void The_two_profiles_that_use_goto_compile()
    {
        int withGoto = 0;

        foreach (string path in ProfileCorpus.Files())
        {
            Zax.Profiles.GameProfile profile = ProfileCorpus.Load(path);
            bool uses = profile.Scripts.Any(static script =>
                script.Source is { } source
                && (source.Contains("goto ", StringComparison.Ordinal)
                    || source.Contains("::", StringComparison.Ordinal)));
            if (!uses)
            {
                continue;
            }

            withGoto++;
            using LuaRun run = LuaRun.Create(profile);
            Assert.All(run.Runtime.Scripts, script => Assert.False(
                script.Failed,
                $"{Path.GetFileNameWithoutExtension(path)}/{script.Name}"));
        }

        Assert.Equal(2, withGoto);
    }

    /// <summary>
    /// TV-LUA-11 — the number rules of the dialect, gathered in one place.
    /// </summary>
    [Fact] // REQ-LUA-040, REQ-LUA-052, REQ-LUA-101, TV-LUA-11
    public void TV_LUA_11_the_number_rules()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script("a", "function Update() printed = frame end", tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 0, 80), counter: 120);

        Assert.Equal("float", run.Text("math.type(frame)"));

        run.Log.Clear();
        run.Do("print(frame)");
        Assert.Equal("120", Assert.Single(run.Log.Prints));

        run.Memory.Write(0x6C, 9);
        Assert.Equal(run.Num("Script:ReadMem(0x6C)"), run.Num("Script:ReadMem(0x6C + 0.5)"));
    }

    /// <summary>
    /// TV-LUA-13 — unknown members: a read gives nil, a write raises the documented message and
    /// an out-of-range frame index raises.
    /// </summary>
    [Fact] // REQ-LUA-042, TV-LUA-13
    public void TV_LUA_13_unknown_members()
    {
        using LuaRun run = Framed(out _);

        Assert.Equal("nil", run.TypeOf("kept.hiden"));
        Assert.EndsWith(
            "cannot set NoSuch, no such field",
            run.MustFail("kept.NoSuch = 1"),
            StringComparison.Ordinal);
        Assert.NotNull(run.Error("local x = Frame:GetShape(Frame.shapeCount)"));
    }

    // --------------------------------------------------------------- helpers

    private static IEnumerable<string> Implementation(string specType) => specType switch
    {
        // The runtime spells three type labels without their accents and names the custom-data
        // type after the member that exposes it. Type names are internal labels: a script never
        // sees them, so the difference is not observable.
        "Lumière" => ["Lumiere"],
        "Découpe" => ["Decoupe"],
        "DonnéesPerso" => ["CData"],
        "Liste<T>" => ["Liste<Instance>", "Liste<Shape>"],
        _ => [specType],
    };

    private static string? Check(ScriptApiSurface surface, string typeName, SpecMemberRow row)
    {
        if (!surface.Types.TryGetValue(typeName, out ScriptApiType? type))
        {
            return $"{typeName}: the type is not declared at all ({row.Priority}).";
        }

        switch (row.Kind)
        {
            case "index":
                if (!type.Indexable)
                {
                    return $"{typeName}[i]: the type is not indexable ({row.Priority}).";
                }

                if (row.Access == "LE" && !type.IndexWritable)
                {
                    return $"{typeName}[i]: the index is not writable ({row.Priority}).";
                }

                return null;

            case "appel":
                return type.CallResult.Length > 0
                    ? null
                    : $"{typeName}(): the type is not callable ({row.Priority}).";

            default:
                if (!type.Members.TryGetValue(row.Member, out ScriptApiMember member))
                {
                    return $"{typeName}.{row.Member}: missing ({row.Priority}).";
                }

                bool shouldCall = row.Kind is "méth" or "fonc";
                if (member.Callable != shouldCall)
                {
                    return $"{typeName}.{row.Member}: genre '{row.Kind}' but "
                        + (member.Callable ? "callable" : "not callable")
                        + $" ({row.Priority}).";
                }

                if (!shouldCall && row.Access == "LE" && !member.Writable)
                {
                    return $"{typeName}.{row.Member}: LE in §4 but read-only ({row.Priority}).";
                }

                if (!shouldCall && row.Access == "L" && member.Writable)
                {
                    return $"{typeName}.{row.Member}: L in §4 but writable ({row.Priority}).";
                }

                return null;
        }
    }
}
