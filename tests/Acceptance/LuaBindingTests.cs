using Zax.Scripting;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for SPEC-05 §3.6 to §3.8: the value binding between Lua and the
/// host, the standard libraries of the base environment, and the error rules.
/// </summary>
public sealed class LuaBindingTests
{
    private static LuaRun OneInstance() => new LuaProfile()
        .Pattern(["A"], tags: "t")
        .Script("a", "function UpdateS() kept = shape; kept2D = shape2D end", tags: "*")
        .Start();

    private static LuaRun Framed()
    {
        LuaRun run = OneInstance();
        run.Frame(new SyntheticBatch().Background('A', 16, 32));
        return run;
    }

    // ------------------------------------------------------- §3.6 value binding

    /// <summary>
    /// REQ-LUA-040 — every numeric value the host hands to Lua is a <b>float</b>, and a Lua
    /// number handed to an integer parameter is truncated towards zero then reduced modulo the
    /// target range.
    /// </summary>
    [Fact] // REQ-LUA-040
    public void Host_numbers_are_floats_and_integer_parameters_truncate_towards_zero()
    {
        using LuaRun run = Framed();
        run.Memory.Write(0x70, 0x5A);
        run.Memory.Write(0x6C, 0x33);

        Assert.Equal("float", run.Text("math.type(frame)"));
        Assert.Equal("float", run.Text("math.type(Frame.shapeCount)"));
        Assert.Equal("float", run.Text("math.type(kept2D.tStart.x)"));
        Assert.Equal("float", run.Text("math.type(kept.Palette)"));

        // Truncation towards zero, then modulo the 16-bit address space.
        Assert.Equal(0x5A, run.Num("Script:ReadMem(0x70 + 0.9)"));
        Assert.Equal(0x5A, run.Num("Script:ReadMem(65536 + 0x70)"));
        Assert.Equal(0x33, run.Num("Script:ReadMem(0x6C + 0.5)"));

        // shape.Palette = 2.9 → 2 (REQ-LUA-040 criterion).
        run.Do("kept.Palette = 2.9");
        Assert.Equal(2, run.Num("kept.customPalette"));

        run.Do("kept.Palette = -3.7");
        Assert.Equal(-3, run.Num("kept.customPalette"));

        run.Do("kept.Palette = 2.5");
        Assert.Equal(2, run.Num("kept.customPalette"));
    }

    /// <summary>
    /// REQ-LUA-040 — a byte parameter is reduced modulo 256.
    /// </summary>
    [Fact] // REQ-LUA-040
    public void A_byte_parameter_is_reduced_modulo_256()
    {
        using LuaRun run = Framed();
        run.Do("Script:WriteMem(0x200, 256 + 7)");
        Assert.Equal(7, run.Memory.Read(0x200));
    }

    /// <summary>
    /// REQ-LUA-041 — a numeric parameter refuses a string, even a numeric one; a call whose
    /// arity or types match no signature raises <c>invalid arguments to &lt;member&gt;</c>.
    /// </summary>
    [Fact] // REQ-LUA-041
    public void Argument_checking_refuses_a_string_for_a_number()
    {
        using LuaRun run = Framed();

        Assert.NotNull(run.Error("Script:ReadMem(\"16\")"));
        Assert.NotNull(run.Error("Frame:GetShape()"));

        string message = run.MustFail("Frame:GetShapeWithPos()");
        Assert.Contains("invalid arguments to", message, StringComparison.Ordinal);

        // Optional parameters may be left out (REQ-LUA-087).
        Assert.Null(run.Error("local a = Palette.FrameColorIndex(3)"));
        Assert.Null(run.Error("local b = Palette.FrameColorIndex(3, 1)"));
    }

    /// <summary>
    /// REQ-LUA-042 — reading an unknown member gives nil without an error; writing one raises
    /// <c>cannot set &lt;name&gt;, no such field</c>; writing a read-only member raises too.
    /// </summary>
    [Fact] // REQ-LUA-042
    public void Unknown_and_read_only_members_follow_the_documented_rules()
    {
        using LuaRun run = Framed();

        Assert.Equal("nil", run.TypeOf("kept.hiden"));
        Assert.Equal("nil", run.TypeOf("kept2D.Hiden"));

        // Lua prefixes an error() message with its own position information; the message the
        // requirement fixes is what follows it.
        Assert.EndsWith(
            "cannot set hiden, no such field",
            run.MustFail("kept.hiden = true"),
            StringComparison.Ordinal);
        Assert.EndsWith(
            "cannot set NoSuch, no such field",
            run.MustFail("kept.NoSuch = 1"),
            StringComparison.Ordinal);

        // Pos is read-only on an instance (L in §4.2).
        Assert.NotNull(run.Error("kept.Pos = Vector3.zero"));
    }

    /// <summary>
    /// REQ-LUA-043 — one host object is always represented by a Lua value equal to itself, and
    /// two distinct objects differ; host objects can be table keys.
    /// </summary>
    [Fact] // REQ-LUA-043
    public void Host_objects_have_a_stable_identity()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Pattern(["B"], tags: "u")
            .Script("a", string.Empty, tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 0, 80).Background('B', 32, 80));

        Assert.True(run.Flag("Frame:GetShape(0).Shape2D == Frame:GetShape(0).Shape2D"));
        Assert.True(run.Flag("Frame:GetShape(0) == Frame:GetShape(0)"));
        Assert.True(run.Flag("Frame:GetShape(0).Shape2D ~= Frame:GetShape(1).Shape2D"));
        Assert.True(run.Flag("Light == Setting.Light"));

        run.Do("t = {}; t[Frame:GetShape(0)] = 7");
        Assert.Equal(7, run.Num("t[Frame:GetShape(0)]"));
    }

    /// <summary>
    /// REQ-LUA-044 — bound vectors are live views: writing a component changes the object, and a
    /// reference kept in a global keeps changing the same object on later frames. Value types
    /// are copies: changing a field of a copy changes nothing.
    /// </summary>
    [Fact] // REQ-LUA-044
    public void Bound_vectors_are_live_and_value_vectors_are_copies()
    {
        using LuaRun run = Framed();

        // Reference: shape.Offset.z = 4 changes the instance.
        run.Do("kept.Offset.z = 4");
        Assert.Equal(4, run.Num("kept.Offset.z"));

        // A reference kept in a global ("camRot = Camera.Rot" in Start) keeps working.
        run.Do("camRot = Camera.Rot");
        run.Do("camRot.x, camRot.y = -60, 0");
        Assert.Equal(-60f, run.Camera.Angles.X, 4);
        Assert.Equal(0f, run.Camera.Angles.Y, 4);
        Assert.Equal(-60, run.Num("Camera.Rot.x"));

        // Value: a local copy of BottomLeft may be changed without touching the instance.
        run.Do("local bl = kept.BottomLeft; bl.x = 0; blx = kept.BottomLeft.x");
        Assert.NotEqual(0, run.Num("blx"));

        // Value: shape2D.tStart is a copy too.
        run.Do("local before = kept2D.tStart.x; kept2D.tStart.x = 0; after = kept2D.tStart.x; same = (before == after)");
        Assert.True(run.Flag("same"));

        // A whole-value assignment is how a value member is written.
        run.Do("kept.Offset.content = Vector3(0, 16, -4)");
        Assert.Equal(16, run.Num("kept.Offset.y"));
        Assert.Equal(-4, run.Num("kept.Offset.z"));
    }

    /// <summary>
    /// REQ-LUA-045 — host objects are indexed from 0 with a truncated numeric index; an index
    /// out of range on a list, a shape, a pattern or <c>Setting.Layer</c> raises a Lua error;
    /// Lua tables keep their own convention.
    /// </summary>
    [Fact] // REQ-LUA-045
    public void Host_objects_are_indexed_from_zero_and_bound_checked()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "pf", instanceCount: 2)
            .Script("a", string.Empty, tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 0, 80).Background('A', 32, 80));

        run.Do("""
            l = Frame:GetShapesWithTag("pf")
            n = 0
            for i = 0, l.Count - 1 do local s = l[i]; n = n + 1 end
            """);
        // Two shapes of two instances each: the list holds instances (REQ-LUA-084).
        Assert.Equal(4, run.Num("n"));
        Assert.Equal(4, run.Num("l.Count"));
        Assert.NotNull(run.Error("local x = l[l.Count]"));

        // A shape indexes its frame instances from 0.
        Assert.Equal(2, run.Num("Frame:GetShape(0).Shape2D.Count"));
        Assert.Null(run.Error("local a = Frame:GetShape(0).Shape2D[0]"));
        Assert.NotNull(run.Error("local a = Frame:GetShape(0).Shape2D[2]"));

        // Setting.Layer[3] is the L2 layer; 6 is out of range.
        Assert.Null(run.Error("Setting.Layer[3].CastShadow = true"));
        Assert.True(run.Settings.Layer(3).CastShadow);
        Assert.NotNull(run.Error("local x = Setting.Layer[6]"));

        // Lua tables stay 1-based.
        Assert.Equal(64, run.Num("#PData"));
    }

    /// <summary>
    /// REQ-LUA-046 — enumerations are plain integers in Lua and a member of an enumeration type
    /// accepts either an integer or a constant.
    /// </summary>
    [Fact] // REQ-LUA-046
    public void Enumerations_are_integers
        ()
    {
        using LuaRun run = Framed();

        Assert.Equal(0, run.Num("TLayer.UNIDENTIFIED"));
        Assert.Equal(1, run.Num("TLayer.UI"));
        Assert.Equal(2, run.Num("TLayer.L1"));
        Assert.Equal(3, run.Num("TLayer.L2"));
        Assert.Equal(4, run.Num("TLayer.L3"));
        Assert.Equal(5, run.Num("TLayer.L4"));
        Assert.Equal(6, run.Num("TLayer.COUNT"));
        Assert.Equal(0, run.Num("Align.FRONT"));
        Assert.Equal(1, run.Num("Align.CENTER"));
        Assert.Equal(2, run.Num("Align.BACK"));
        Assert.Equal(0, run.Num("CamMode.TPV"));
        Assert.Equal(1, run.Num("CamMode.FPV"));

        run.Do("kept.Layer = 3");
        Assert.Equal(3, run.Num("kept.Layer"));
        run.Do("kept.Layer = TLayer.L1");
        Assert.Equal(2, run.Num("kept.Layer"));
        Assert.True(run.Flag("kept.Layer == TLayer.L1"));

        run.Do("kept.geo = 0");
        Assert.Equal(0, run.Num("kept.geo"));
    }

    /// <summary>
    /// REQ-LUA-047 — a query that finds nothing returns nil, and passing nil where an object is
    /// required raises a Lua error rather than crashing the program.
    /// </summary>
    [Fact] // REQ-LUA-047
    public void A_query_that_finds_nothing_returns_nil_and_nil_arguments_raise
        ()
    {
        using LuaRun run = Framed();

        Assert.Equal("nil", run.TypeOf("Frame:GetShapeWithTag(\"nothing\")"));
        Assert.Equal("nil", run.TypeOf("Pattern.GetPatternWithTag(\"nothing\")"));
        Assert.NotNull(run.Error("Frame:AddNewShape(nil, Vector3(0, 0, 0), 0)"));
        Assert.NotNull(run.Error("Frame:CreateNewShape(nil, Vector3(0, 0, 0), 0)"));
    }

    /// <summary>
    /// REQ-LUA-048 — <c>Frame:GetShapesWithTag</c> hands back the <b>same</b> list every time,
    /// emptied and refilled; <c>Pattern.GetPatternsWithTag</c> has its own such list.
    /// </summary>
    [Fact] // REQ-LUA-048, TV-LUA-14
    public void The_tag_queries_share_one_list()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "a")
            .Pattern(["B"], tags: "b")
            .Script("s", string.Empty, tags: "*")
            .Start();

        run.Frame(new SyntheticBatch()
            .Background('A', 0, 80)
            .Background('A', 32, 80)
            .Background('A', 64, 80)
            .Background('B', 96, 80));

        run.Do("""
            x = Frame:GetShapesWithTag("a")
            n = x.Count
            y = Frame:GetShapesWithTag("b")
            same = (x == y)
            after = x.Count
            """);

        Assert.Equal(3, run.Num("n"));
        Assert.True(run.Flag("same"));
        Assert.Equal(1, run.Num("after"));
        Assert.Equal(1, run.Num("y.Count"));

        run.Do("""
            p = Pattern.GetPatternsWithTag("a")
            q = Pattern.GetPatternsWithTag("b")
            samePatterns = (p == q)
            different = (p ~= x)
            """);
        Assert.True(run.Flag("samePatterns"));
        Assert.True(run.Flag("different"));
    }

    // ------------------------------------- §3.7 standard libraries and base environment

    /// <summary>
    /// REQ-LUA-050 — the base library, <c>math</c>, <c>string</c> and <c>table</c> are present,
    /// <c>coroutine</c> and <c>utf8</c> should be, and <c>io</c>, <c>os</c> (apart from
    /// <c>clock</c>, <c>time</c> and <c>date</c>), <c>debug</c>, <c>package</c>,
    /// <c>require</c>, <c>dofile</c> and <c>loadfile</c> must not be.
    /// </summary>
    [Fact] // REQ-LUA-050
    public void The_base_environment_offers_exactly_the_allowed_libraries()
    {
        using LuaRun run = new LuaProfile().Script("a", string.Empty).Start();

        foreach (string name in new[]
        {
            "assert", "error", "ipairs", "pairs", "next", "pcall", "xpcall", "select",
            "tonumber", "tostring", "type", "rawget", "rawset", "rawequal", "setmetatable",
            "getmetatable", "unpack", "print", "clear",
        })
        {
            Assert.Equal("function", run.TypeOf(name));
        }

        Assert.Equal("function", run.TypeOf("math.fmod"));
        Assert.Equal("function", run.TypeOf("table.sort"));
        Assert.Equal("function", run.TypeOf("table.remove"));
        Assert.Equal("function", run.TypeOf("string.format"));
        Assert.Equal("table", run.TypeOf("coroutine"));
        Assert.Equal("table", run.TypeOf("utf8"));

        Assert.Equal("nil", run.TypeOf("io"));
        Assert.Equal("nil", run.TypeOf("debug"));
        Assert.Equal("nil", run.TypeOf("package"));
        Assert.Equal("nil", run.TypeOf("require"));
        Assert.Equal("nil", run.TypeOf("dofile"));
        Assert.Equal("nil", run.TypeOf("loadfile"));

        Assert.Equal("function", run.TypeOf("os.clock"));
        Assert.Equal("function", run.TypeOf("os.time"));
        Assert.Equal("function", run.TypeOf("os.date"));
        Assert.Equal("nil", run.TypeOf("os.execute"));
        Assert.Equal("nil", run.TypeOf("os.remove"));
        Assert.Equal("nil", run.TypeOf("os.getenv"));
        Assert.Equal("nil", run.TypeOf("os.exit"));
    }

    /// <summary>
    /// REQ-LUA-050, last criterion — every one of the 579 scripts of the corpus loads in the
    /// base environment the runtime installs.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-LUA-050, REQ-LUA-100
    public void Every_script_of_the_corpus_compiles_and_runs_its_chunk()
    {
        List<string> failures = [];
        int scripts = 0;

        foreach (string path in ProfileCorpus.Files())
        {
            Zax.Profiles.GameProfile profile = ProfileCorpus.Load(path);
            if (profile.Scripts.Count == 0)
            {
                continue;
            }

            scripts += profile.Scripts.Count;
            using LuaRun run = LuaRun.Create(profile);
            foreach (LoadedScript script in run.Runtime.Scripts)
            {
                if (script.Failed)
                {
                    failures.Add($"{Path.GetFileNameWithoutExtension(path)}/{script.Name}");
                }
            }

            failures.AddRange(run.Log.Errors);
        }

        Assert.Equal(579, scripts);
        Assert.Empty(failures);
    }

    /// <summary>
    /// REQ-LUA-052 — <c>print</c> joins its arguments with a tab and writes one line to the
    /// log; an integral float is written without a decimal part, the rest with <c>%.14g</c>.
    /// </summary>
    [Fact] // REQ-LUA-052
    public void Print_writes_one_tab_separated_line_with_the_dialect_number_format()
    {
        using LuaRun run = new LuaProfile().Script("a", string.Empty).Start();

        run.Do("print(\"frame\", 12.0, 0.4, true, nil)");
        Assert.Equal("frame\t12\t0.4\ttrue\tnil", Assert.Single(run.Log.Prints));

        run.Log.Clear();
        run.Do("print(3)");
        run.Do("print(-0.5)");
        run.Do("print(1/3)");
        Assert.Equal(["3", "-0.5", "0.33333333333333"], run.Log.Prints);
    }

    /// <summary>REQ-LUA-053 — <c>clear()</c> exists and does nothing.</summary>
    [Fact] // REQ-LUA-053
    public void Clear_exists_and_does_nothing()
    {
        using LuaRun run = new LuaProfile().Script("a", string.Empty).Start();
        Assert.Null(run.Error("clear()"));
        Assert.Empty(run.Log.Errors);
        Assert.Empty(run.Log.Prints);
    }

    // ------------------------------------------------------------------ §3.8 errors

    /// <summary>
    /// REQ-LUA-060 — a compilation error marks the script in error, registers none of its hooks,
    /// leaves the globals its chunk had set, logs the message once and carries on with the other
    /// scripts.
    /// </summary>
    [Fact] // REQ-LUA-060
    public void A_compilation_error_stops_that_script_alone()
    {
        using LuaRun run = new LuaProfile()
            .Script("a", "this is not lua !!")
            .Script("b", "bWorks = true\nfunction Update() bRan = true end")
            .Start();

        LoadedScript a = run.Runtime.Scripts.First(static s => s.Name == "a");
        LoadedScript b = run.Runtime.Scripts.First(static s => s.Name == "b");

        Assert.True(a.Failed);
        Assert.False(b.Failed);
        Assert.True(run.Flag("bWorks"));

        run.Runtime.InvokeUpdate();
        Assert.True(run.Flag("bRan"));

        string error = Assert.Single(run.Log.Errors);
        Assert.Contains("'a'", error);
    }

    /// <summary>
    /// REQ-LUA-060 — an error raised while the chunk runs leaves in place the globals it had
    /// already defined.
    /// </summary>
    [Fact] // REQ-LUA-060
    public void A_chunk_error_keeps_the_globals_defined_before_it()
    {
        using LuaRun run = new LuaProfile()
            .Script("a", "before = 1\nerror('stop')\nafter = 1")
            .Start();

        Assert.True(run.Runtime.Scripts[0].Failed);
        Assert.Equal(1, run.Num("before"));
        Assert.Equal("nil", run.TypeOf("after"));
        Assert.Single(run.Log.Errors);
    }

    /// <summary>
    /// REQ-LUA-061 — an error in a hook stops that hook, marks the script in error, logs once
    /// with the script name, the hook name and the Lua message, and the sequence carries on. For
    /// <c>UpdateS</c>, the instances left in that script's list are skipped for the frame.
    /// </summary>
    [Fact] // REQ-LUA-061
    public void An_error_in_UpdateS_drops_the_rest_of_that_script_list_only()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script("!prelude", "log = {}\nfunction note(s) log[#log + 1] = s end", tags: string.Empty)
            .Script(
                "e",
                """
                seen = 0
                function UpdateS()
                  seen = seen + 1
                  if seen == 2 then error("x") end
                  note("e.UpdateS")
                end
                function LateUpdate() note("e.LateUpdate") end
                """,
                tags: "*")
            .Script(
                "f",
                """
                function UpdateS() note("f.UpdateS") end
                function LateUpdate() note("f.LateUpdate") end
                """,
                tags: "*")
            .Start();

        run.Frame(new SyntheticBatch()
            .Background('A', 0, 80)
            .Background('A', 32, 80)
            .Background('A', 64, 80));

        // Only the first instance of "e" was handled; "f" ran all three and its LateUpdate ran;
        // "e" is in error so its own LateUpdate did not.
        Assert.Equal(
            "e.UpdateS|f.UpdateS|f.UpdateS|f.UpdateS|f.LateUpdate",
            run.Text("table.concat(log, '|')"));
        Assert.True(run.Runtime.Scripts.First(static s => s.Name == "e").Failed);

        string error = Assert.Single(run.Log.Errors);
        Assert.Contains("'e'", error);
        Assert.Contains("UpdateS", error, StringComparison.Ordinal);
    }

    /// <summary>
    /// TV-LUA-12 — the message of a hook error carries the script name, the hook name and the
    /// Lua message with its line, it is logged once, and the script is never called again.
    /// </summary>
    [Fact] // REQ-LUA-061, REQ-LUA-062, TV-LUA-12
    public void TV_LUA_12_a_failing_Update_is_logged_once_and_never_called_again()
    {
        using LuaRun run = new LuaProfile()
            .Script("e", "function Update() local x = nil; return x.y end\n"
                + "function Start() starts = (starts or 0) + 1 end\n"
                + "function End() ends = (ends or 0) + 1 end\n"
                + "function LateUpdate() late = true end")
            .Start();

        run.Runtime.InvokeUpdate();
        run.Runtime.InvokeLateUpdate();

        string error = Assert.Single(run.Log.Errors);
        Assert.Contains("'e'", error);
        Assert.Contains("Update", error, StringComparison.Ordinal);
        Assert.Contains("attempt to index", error, StringComparison.Ordinal);

        Assert.Equal("nil", run.TypeOf("late"));

        run.Runtime.InvokeUpdate();
        Assert.Single(run.Log.Errors);

        // REQ-LUA-062: a relaunch of the same game calls neither End nor Start of that script.
        run.Runtime.InvokeEnd();
        run.Runtime.InvokeStart();
        Assert.Equal(1, run.Num("starts"));
        Assert.Equal("nil", run.TypeOf("ends"));
    }

    /// <summary>
    /// REQ-LUA-063 — an internal error an API call would raise becomes an ordinary Lua error:
    /// the game carries on and nothing escapes as a managed exception.
    /// </summary>
    [Fact] // REQ-LUA-063
    public void Api_misuse_becomes_an_ordinary_Lua_error()
    {
        using LuaRun run = Framed();

        Assert.NotNull(run.Error("local x = Setting.Layer[6]"));
        Assert.NotNull(run.Error("local x = Frame:GetShape(9999)"));
        Assert.NotNull(run.Error("local x = kept.CData[-1]"));

        // And the frame after keeps converting.
        Assert.Single(run.Frame(new SyntheticBatch().Background('A', 16, 32), counter: 2)
            .ScriptableInstances);
    }

    /// <summary>
    /// REQ-LUA-063 — the same, reached through a hook: the error is logged and the conversion of
    /// the frame finishes.
    /// </summary>
    [Fact] // REQ-LUA-061, REQ-LUA-063
    public void A_hook_that_misuses_the_api_does_not_stop_the_frame()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script("a", "function Update() local x = Frame:GetShape(9999) end", tags: "*")
            .Start();

        Zax.Engine3D.ConversionFrame frame = run.Frame(new SyntheticBatch().Background('A', 0, 80));

        Assert.Single(run.Log.Errors);
        Assert.Single(frame.ScriptableInstances);
    }

    /// <summary>
    /// REQ-LUA-064 — a hook that never gives control back is cut off after its execution budget,
    /// as if it had raised an error, and the emulation resumes.
    /// </summary>
    [Fact] // REQ-LUA-064
    public void An_endless_hook_is_cut_off_after_its_budget()
    {
        using LuaRun run = new LuaProfile()
            .Script("a", "function Update() while true do end end")
            .Start();

        run.Runtime.HookBudget = TimeSpan.FromMilliseconds(200);

        System.Diagnostics.Stopwatch watch = System.Diagnostics.Stopwatch.StartNew();
        run.Runtime.InvokeUpdate();
        watch.Stop();

        Assert.True(run.Runtime.Scripts[0].Failed);
        Assert.Single(run.Log.Errors);
        Assert.True(watch.Elapsed < TimeSpan.FromSeconds(10), $"the hook ran for {watch.Elapsed}.");
    }
}
