using Zax.Engine3D;
using Zax.Scripting;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for SPEC-05 §3.4 and §3.5: the frame hooks, their order and
/// grouping, the <c>shape</c>/<c>shape2D</c> globals, <c>BuildPattern</c>, <c>DetectShape</c>
/// and <c>InputProcess</c>.
/// </summary>
public sealed class LuaFrameTests
{
    /// <summary>The log-collecting prelude every ordering test shares.</summary>
    private const string Prelude = "log = {}\nfunction note(s) log[#log + 1] = s end";

    private static string Hooks(string name) =>
        $$"""
        function Update() note("{{name}}.Update") end
        function UpdateS() note("{{name}}.UpdateS(" .. math.floor(shape2D.tStart.x) .. ")") end
        function LateUpdate() note("{{name}}.LateUpdate") end
        """;

    /// <summary>
    /// REQ-LUA-025 — the five steps run synchronously in order: the globals, every
    /// <c>Update</c>, the per-script instance lists, every <c>UpdateS</c> <b>grouped per
    /// script</b>, then every <c>LateUpdate</c>.
    /// </summary>
    [Fact] // REQ-LUA-025
    public void The_frame_hooks_run_in_the_documented_order_and_UpdateS_is_grouped_per_script()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script("!prelude", Prelude, tags: string.Empty)
            .Script("a", Hooks("a"), tags: "*")
            .Script("b", Hooks("b"), tags: "*")
            .Start();

        run.Frame(new SyntheticBatch()
            .Background('A', 0, 80)
            .Background('A', 32, 80)
            .Background('A', 64, 80));

        Assert.Equal(
            "a.Update|b.Update"
                + "|a.UpdateS(1)|a.UpdateS(5)|a.UpdateS(9)"
                + "|b.UpdateS(1)|b.UpdateS(5)|b.UpdateS(9)"
                + "|a.LateUpdate|b.LateUpdate",
            run.Text("table.concat(log, '|')"));
    }

    /// <summary>
    /// TV-LUA-01 — scripts <c>b</c> (tags <c>t</c>) and <c>a</c> (tags <c>*</c>) in that file
    /// order, a frame with two scriptable instances <c>i1</c> (tag <c>t</c>) and <c>i2</c>
    /// (tag <c>u</c>): the chunks run in file order, <c>Start</c> in script order, and
    /// <c>b</c> only sees <c>i1</c>.
    /// </summary>
    [Fact] // REQ-LUA-025, TV-LUA-01
    public void TV_LUA_01_the_hook_log_of_the_test_vector()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Pattern(["B"], tags: "u")
            .Script("!prelude", Prelude, tags: string.Empty)
            .Script("b", "chunks = (chunks or '') .. 'b'\n" + Hooks("b")
                + "\nfunction Start() note('b.Start') end", tags: "t")
            .Script("a", "chunks = (chunks or '') .. 'a'\n" + Hooks("a")
                + "\nfunction Start() note('a.Start') end", tags: "*")
            .Start();

        Assert.Equal("ba", run.Text("chunks"));

        run.Frame(new SyntheticBatch().Background('A', 0, 80).Background('B', 32, 80));

        Assert.Equal(
            "a.Start|b.Start"
                + "|a.Update|b.Update"
                + "|a.UpdateS(1)|a.UpdateS(5)"
                + "|b.UpdateS(1)"
                + "|a.LateUpdate|b.LateUpdate",
            run.Text("table.concat(log, '|')"));
    }

    /// <summary>
    /// REQ-LUA-025 — <c>frame</c> is the frame counter pushed as a float and <c>time</c> is
    /// <c>frame / 60</c> in seconds.
    /// </summary>
    [Fact] // REQ-LUA-025, REQ-LUA-040
    public void Frame_and_time_are_set_before_the_hooks()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script("a", "function Update() f = frame; t = time; k = math.type(frame) end")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 0, 80), counter: 120);

        Assert.Equal(120, run.Num("f"));
        Assert.Equal(2.0, run.Num("t"), 6);
        Assert.Equal("float", run.Text("k"));
    }

    /// <summary>
    /// REQ-LUA-025, notes — a script turned off during its own <c>Update</c> (step 2) gets no
    /// <c>UpdateS</c> at all, because the lists are built at step 3.
    /// </summary>
    [Fact] // REQ-LUA-025
    public void A_script_turned_off_during_step_two_receives_no_UpdateS()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script("!prelude", Prelude, tags: string.Empty)
            .Script(
                "a",
                """
                function Update() Script:Get("b").Enable = false end
                """,
                tags: "*")
            .Script("b", "function UpdateS() note('b.UpdateS') end", tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 0, 80).Background('A', 32, 80));

        Assert.Equal(0, run.Num("#log"));
    }

    /// <summary>
    /// REQ-LUA-025, notes — a script turned off <b>during</b> step 4 keeps the calls left in
    /// its own list for that frame.
    /// </summary>
    [Fact] // REQ-LUA-025
    public void A_script_turned_off_during_step_four_keeps_the_rest_of_its_list()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script("!prelude", Prelude, tags: string.Empty)
            .Script(
                "a",
                """
                function UpdateS()
                  note("a.UpdateS")
                  Script:Get("a").Enable = false
                end
                """,
                tags: "*")
            .Start();

        run.Frame(new SyntheticBatch()
            .Background('A', 0, 80)
            .Background('A', 32, 80)
            .Background('A', 64, 80));

        Assert.Equal(3, run.Num("#log"));
    }

    /// <summary>
    /// REQ-LUA-026 — <c>shape</c> and <c>shape2D</c> are ordinary globals: after step 4 they
    /// hold the last instance handled, and a script may reassign them.
    /// </summary>
    [Fact] // REQ-LUA-026
    public void Shape_and_shape2D_keep_the_last_instance_and_can_be_reassigned()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script(
                "a",
                """
                function UpdateS() end
                function LateUpdate()
                  lateX = shape2D.tStart.x
                  shape = Frame:GetShape(0)
                  reassigned = (shape == Frame:GetShape(0))
                end
                """,
                tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 0, 80).Background('A', 64, 80));

        Assert.Equal(9, run.Num("lateX"));
        Assert.True(run.Flag("reassigned"));
    }

    /// <summary>
    /// REQ-LUA-026 — only <b>scriptable</b> instances reach <c>UpdateS</c>: a shape created by
    /// <c>Frame:CreateNewShape</c> during <c>Update</c> appears in no <c>UpdateS</c> call.
    /// </summary>
    [Fact] // REQ-LUA-026
    public void A_script_created_shape_never_reaches_UpdateS()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script(
                "a",
                """
                calls = 0
                function Update()
                  local p = Pattern.GetPatternWithTag("t")
                  if p ~= nil then
                    created = Frame:CreateNewShape(p, Vector3(100, 100, 0), 4)
                  end
                end
                function UpdateS() calls = calls + 1 end
                """,
                tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 0, 80));

        Assert.Equal(1, run.Num("calls"));
        Assert.Equal("userdata", run.TypeOf("created"));
    }

    /// <summary>
    /// REQ-LUA-027 — a change made to a <b>pattern instance</b> is durable: every frame
    /// instance copied from it from the next frame on starts with the new value, before any
    /// <c>UpdateS</c>.
    /// </summary>
    [Fact] // REQ-LUA-027
    public void A_change_to_a_pattern_instance_is_durable
        ()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script(
                "a",
                """
                function Update()
                  if frame == 1 then Pattern.GetPatternWithTag("t").Offset.x = 5 end
                end
                function UpdateS() seen = shape.Offset.x end
                """,
                tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 0, 80), counter: 1);
        Assert.Equal(0, run.Num("seen"));

        run.Frame(new SyntheticBatch().Background('A', 0, 80), counter: 2);
        Assert.Equal(5, run.Num("seen"));
    }

    /// <summary>
    /// REQ-LUA-027 — a change made to a <b>frame instance</b> lasts for that frame only.
    /// </summary>
    [Fact] // REQ-LUA-027
    public void A_change_to_a_frame_instance_lasts_one_frame()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script(
                "a",
                """
                function UpdateS()
                  seen = shape.Offset.y
                  shape.Offset.y = 9
                end
                """,
                tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 0, 80), counter: 1);
        Assert.Equal(0, run.Num("seen"));

        run.Frame(new SyntheticBatch().Background('A', 0, 80), counter: 2);
        Assert.Equal(0, run.Num("seen"));
    }

    /// <summary>
    /// REQ-LUA-028 — an instance or shape reference a script keeps beyond its frame stays
    /// readable without a Lua error, and writing through it has no visible effect.
    /// </summary>
    [Fact] // REQ-LUA-028
    public void A_reference_kept_beyond_its_frame_stays_readable()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script(
                "a",
                """
                function UpdateS() if old == nil then old = shape end end
                """,
                tags: "*")
            .Start();

        for (int frame = 1; frame <= 11; frame++)
        {
            run.Frame(new SyntheticBatch().Background('A', 0, 80), counter: frame);
        }

        Assert.Null(run.Error("x = old.Offset.x; y = old.Shape2D.tStart.x; old.Alpha = 0.5"));
        Assert.Equal("number", run.TypeOf("x"));
        Assert.Equal("number", run.TypeOf("y"));
    }

    // ------------------------------------------------------------- §3.5 special hooks

    /// <summary>
    /// REQ-LUA-030 — <c>BuildPattern</c> is called for every active script linked to a pattern
    /// instance, with <c>pattern</c> set to it, and <b>before</b> the geometry is built; the
    /// changes it makes are permanent.
    /// </summary>
    [Fact] // REQ-LUA-030
    public void BuildPattern_runs_once_per_linked_pattern_instance_and_its_changes_are_permanent()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "gear", instanceCount: 2)
            .Pattern(["B"], tags: "other")
            .Script(
                "gear",
                """
                built = 0
                function BuildPattern()
                  built = built + 1
                  pattern.sizeZ = 1
                  pattern.geo = 0
                  pattern.Scale.z = 0.1
                  pattern.fillrate = 0.05
                end
                """,
                tags: "gear,gear1")
            .Start();

        Assert.Equal(2, run.Num("built"));

        foreach (PatternInstance instance in run.Engine.Patterns.Permanent[0].Instances)
        {
            Assert.Equal(1, instance.Depth);
            Assert.Equal(Geo3DType.Default, instance.Geometry);
            Assert.Equal(0.1f, instance.Scale.Z, 5);
            Assert.Equal(0.05f, instance.FillRate, 5);
        }

        // The unlinked pattern is untouched.
        Assert.Equal(3, run.Engine.Patterns.Permanent[1].Instances[0].Depth);
    }

    /// <summary>
    /// REQ-LUA-030 — a <c>BuildPattern</c> of a <b>disabled</b> script is never called.
    /// </summary>
    [Fact] // REQ-LUA-030
    public void BuildPattern_of_a_disabled_script_is_never_called()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script("a", "built = 0\nfunction BuildPattern() built = built + 1 end", enabled: false)
            .Start();

        Assert.Equal(0, run.Num("built"));
    }

    /// <summary>
    /// REQ-LUA-031 — <c>DetectShape</c> is never called, even when the chunk defines it, and
    /// its presence changes nothing about the segmentation.
    /// </summary>
    [Fact] // REQ-LUA-031
    public void DetectShape_is_never_called()
    {
        using LuaRun withHook = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script("detect_shape", "detected = 0\nfunction DetectShape() detected = detected + 1 end")
            .Start();

        ConversionFrame withIt = withHook.Frame(
            new SyntheticBatch().Background('A', 0, 80).Background('A', 8, 80));

        using LuaRun without = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script("detect_shape", string.Empty)
            .Start();

        ConversionFrame withoutIt = without.Frame(
            new SyntheticBatch().Background('A', 0, 80).Background('A', 8, 80));

        Assert.Equal(0, withHook.Num("detected"));
        Assert.Equal(withoutIt.Shapes.Count, withIt.Shapes.Count);
        Assert.Equal(withoutIt.ScriptableInstances.Count, withIt.ScriptableInstances.Count);
    }

    /// <summary>
    /// REQ-LUA-032 — <c>InputProcess</c> comes from the <b>last script loaded</b> that defines
    /// it, is called once per emulated frame with the pad's player index while the script pad is
    /// on, and its return value is truncated towards zero; anything non-numeric contributes 0.
    /// </summary>
    [Fact] // REQ-LUA-032
    public void InputProcess_comes_from_the_last_script_that_defines_it()
    {
        using LuaRun run = new LuaProfile()
            .Script("first", "function InputProcess(i) return 1 end")
            .Script("second", """
                calls = 0
                function InputProcess(i)
                  calls = calls + 1
                  index = i
                  return answer
                end
                function Start() ScriptPad:Enable(true) end
                """)
            .Start();

        run.Do("answer = 8");
        Assert.Equal(8, run.Runtime.InvokeInputProcess(1, 1 / 60f));
        Assert.Equal(1, run.Num("calls"));
        Assert.Equal(0, run.Num("index"));
        Assert.Equal(8, run.Runtime.ScriptPadState);

        run.Do("answer = 8.9");
        Assert.Equal(8, run.Runtime.InvokeInputProcess(2, 2 / 60f));

        run.Do("answer = -3.7");
        Assert.Equal(-3, run.Runtime.InvokeInputProcess(3, 3 / 60f));

        run.Do("answer = nil");
        Assert.Equal(0, run.Runtime.InvokeInputProcess(4, 4 / 60f));

        run.Do("answer = true");
        Assert.Equal(0, run.Runtime.InvokeInputProcess(5, 5 / 60f));

        run.Do("answer = 'x'");
        Assert.Equal(0, run.Runtime.InvokeInputProcess(6, 6 / 60f));
    }

    /// <summary>
    /// REQ-LUA-032 — with the script pad off, or the owning script inactive, the contribution
    /// is 0 and the hook is not called. <c>ScriptPad.PadIndex</c> chooses the player passed in.
    /// </summary>
    [Fact] // REQ-LUA-032, REQ-LUA-091
    public void InputProcess_contributes_nothing_when_the_pad_is_off()
    {
        using LuaRun run = new LuaProfile()
            .Script("pad", """
                calls = 0
                function InputProcess(i) calls = calls + 1; index = i; return 8 end
                """)
            .Start();

        Assert.Equal(0, run.Runtime.InvokeInputProcess(1, 0f));
        Assert.Equal(0, run.Num("calls"));

        run.Do("ScriptPad:Enable(true); ScriptPad.PadIndex = 1");
        Assert.Equal(8, run.Runtime.InvokeInputProcess(2, 0f));
        Assert.Equal(1, run.Num("index"));

        run.Runtime.Scripts.First(static s => s.Name == "pad").Enabled = false;
        Assert.Equal(0, run.Runtime.InvokeInputProcess(3, 0f));
        Assert.Equal(0, run.Runtime.ScriptPadState);
    }

    /// <summary>
    /// REQ-LUA-033 — hooks take no argument apart from <c>InputProcess</c>, and their return
    /// value is ignored apart from <c>InputProcess</c>'s.
    /// </summary>
    [Fact] // REQ-LUA-033
    public void Hook_arguments_and_return_values_are_ignored()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script(
                "a",
                """
                args = -1
                function Update(...) args = select('#', ...); return false end
                function UpdateS() return {1, 2} end
                function LateUpdate() ran = true; return 7 end
                """,
                tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 0, 80));

        Assert.Equal(0, run.Num("args"));
        Assert.True(run.Flag("ran"));
        Assert.Empty(run.Log.Errors);
    }
}
