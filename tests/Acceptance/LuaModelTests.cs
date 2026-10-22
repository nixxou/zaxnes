using Zax.Engine3D;
using Zax.Profiles;
using Zax.Scripting;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for SPEC-05 §3.1 to §3.3: the script model of a profile, the script
/// order, the linking, the single Lua state and the life-cycle hooks.
/// </summary>
public sealed class LuaModelTests
{
    // ------------------------------------------------- §3.1 scripts, order, linking

    /// <summary>
    /// REQ-LUA-001 — the <c>scripts</c> field is read with its name, <c>enable</c> flag, source
    /// and tag string, and the stored <c>error</c> flag is ignored. Corpus criterion:
    /// 579 scripts in 89 of the 101 profiles, 556 enabled and 23 not, and "empty" carrying the
    /// single script <c>Build</c> with the tags <c>*</c>, active.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-LUA-001
    public void The_corpus_holds_579_scripts_in_89_profiles()
    {
        int profiles = 0;
        int withScripts = 0;
        int scripts = 0;
        int enabled = 0;
        int disabled = 0;

        foreach (string path in ProfileCorpus.Files())
        {
            GameProfile profile = ProfileCorpus.Load(path);
            profiles++;
            if (profile.Scripts.Count > 0)
            {
                withScripts++;
            }

            foreach (ProfileScript script in profile.Scripts)
            {
                scripts++;
                if (script.Enabled)
                {
                    enabled++;
                }
                else
                {
                    disabled++;
                }
            }
        }

        Assert.Equal(101, profiles);
        Assert.Equal(89, withScripts);
        Assert.Equal(579, scripts);
        Assert.Equal(556, enabled);
        Assert.Equal(23, disabled);
    }

    /// <summary>REQ-LUA-001 — "empty" carries the single active script <c>Build</c>, tags <c>*</c>.</summary>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-LUA-001
    public void The_two_dimensional_profile_carries_the_single_script_Build()
    {
        GameProfile profile = ProfileCorpus.Named("empty")
            ?? throw new InvalidOperationException("the profile 'empty' is missing.");

        ProfileScript script = Assert.Single(profile.Scripts);
        Assert.Equal("Build", script.Name);
        Assert.Equal("*", script.TagString);
        Assert.True(script.Enabled);

        using LuaRun run = LuaRun.Create(profile);
        LoadedScript loaded = Assert.Single(run.Runtime.Scripts);
        Assert.True(loaded.Universal);
        Assert.True(loaded.Active);
    }

    /// <summary>
    /// REQ-LUA-001 — a script the file marks as being in error is loaded as not in error: the
    /// flag read from the file is ignored.
    /// </summary>
    [Fact] // REQ-LUA-001
    public void The_stored_error_flag_is_ignored()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Scripts.Add(new ProfileScript("a", true, "function Update() end", "*") { StoredErrorFlag = true });

        using LuaRun run = LuaRun.Create(profile);
        LoadedScript script = Assert.Single(run.Runtime.Scripts);
        Assert.False(script.Failed);
        Assert.True(script.Active);
    }

    /// <summary>
    /// REQ-LUA-002 — scripts are global to the profile: a script with no tag in common with any
    /// instance still receives <c>Start</c>, <c>Update</c> and <c>LateUpdate</c>, and never
    /// receives <c>UpdateS</c>.
    /// </summary>
    [Fact] // REQ-LUA-002
    public void A_script_linked_to_nothing_still_receives_the_global_hooks()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "other")
            .Script(
                "a",
                """
                log = {}
                function Start() log[#log + 1] = "Start" end
                function Update() log[#log + 1] = "Update" end
                function UpdateS() log[#log + 1] = "UpdateS" end
                function LateUpdate() log[#log + 1] = "LateUpdate" end
                """,
                tags: "nothing")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 80, 80));

        Assert.Equal("Start", run.Text("log[1]"));
        Assert.Equal("Update", run.Text("log[2]"));
        Assert.Equal("LateUpdate", run.Text("log[3]"));
        Assert.Equal(3, run.Num("#log"));
    }

    /// <summary>
    /// REQ-LUA-003 — two scripts of the same name: the last one read is kept and a warning is
    /// written to the log.
    /// </summary>
    [Fact] // REQ-LUA-003
    public void A_duplicate_script_name_keeps_the_last_one_and_warns()
    {
        using LuaRun run = new LuaProfile()
            .Script("a", "which = 'first'")
            .Script("a", "which = 'second'")
            .Load();

        LoadedScript script = Assert.Single(run.Runtime.Scripts);
        Assert.Equal("a", script.Name);
        Assert.Equal("second", run.Text("which"));

        string warning = Assert.Single(run.Log.Warnings);
        Assert.Contains("'a'", warning);
    }

    /// <summary>
    /// REQ-LUA-004 — the script order is the linguistic collation of the names: punctuation,
    /// space and symbols &lt; digits &lt; letters, then the lower-cased character, then the
    /// shorter name, then lower case before upper case.
    /// </summary>
    [Theory] // REQ-LUA-004
    [InlineData(new[] { "scene", "ramp7", "ground", "ramp_rotation", "camera", "bike" },
        "bike,camera,ground,ramp_rotation,ramp7,scene")]
    [InlineData(new[] { "B", "a", "b" }, "a,b,B")]
    [InlineData(new[] { "bg1", "bg'", "bg" }, "bg,bg',bg1")]
    public void The_script_order_is_the_documented_collation(string[] names, string expected)
    {
        LuaProfile profile = new();
        foreach (string name in names)
        {
            profile.Script(name, string.Empty);
        }

        using LuaRun run = profile.Load();
        Assert.Equal(expected, string.Join(',', run.Runtime.Scripts.Select(static s => s.Name)));
    }

    /// <summary>
    /// REQ-LUA-004, second criterion — for each of the 89 built-in profiles that carry scripts,
    /// sorting the names gives back exactly the order they appear in the file.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-LUA-004
    public void The_collation_reproduces_the_file_order_of_every_built_in_profile()
    {
        List<string> wrong = [];
        int checked_ = 0;

        foreach (string path in ProfileCorpus.Files())
        {
            GameProfile profile = ProfileCorpus.Load(path);
            if (profile.Scripts.Count == 0)
            {
                continue;
            }

            checked_++;
            using LuaRun run = LuaRun.Create(profile);
            string fileOrder = string.Join(',', profile.Scripts.Select(static s => s.Name));
            string sorted = string.Join(',', run.Runtime.Scripts.Select(static s => s.Name));
            if (!string.Equals(fileOrder, sorted, StringComparison.Ordinal))
            {
                wrong.Add($"{Path.GetFileNameWithoutExtension(path)}: file [{fileOrder}] sorted [{sorted}]");
            }
        }

        Assert.Equal(89, checked_);
        Assert.Empty(wrong);
    }

    /// <summary>
    /// REQ-LUA-005 — the tag string is split on commas, the elements trimmed; a string holding
    /// <c>*</c> anywhere links to every instance; empty elements have no effect.
    /// </summary>
    [Theory] // REQ-LUA-005
    [InlineData("*", true, "")]
    [InlineData("ui,*", true, "")]
    [InlineData("a*b", true, "")]
    [InlineData("gear, gear1", false, "gear|gear1")]
    [InlineData("a,,b", false, "a|b")]
    [InlineData("  spaced  ", false, "spaced")]
    public void Tag_strings_split_on_commas_and_a_star_links_to_everything(
        string tagString, bool universal, string tags)
    {
        using LuaRun run = new LuaProfile().Script("a", string.Empty, tags: tagString).Load();
        LoadedScript script = Assert.Single(run.Runtime.Scripts);

        Assert.Equal(universal, script.Universal);
        if (!universal)
        {
            Assert.Equal(tags, string.Join('|', script.Tags));
        }
    }

    /// <summary>
    /// REQ-LUA-006 — a script is linked to a pattern instance if and only if its tag string
    /// holds <c>*</c> or one of its tags equals, exactly and case-sensitively, one of the
    /// instance's tags.
    /// </summary>
    [Fact] // REQ-LUA-006
    public void Linking_is_an_exact_case_sensitive_tag_match()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "gear,gear1")
            .Pattern(["B"], tags: "Gear")
            .Pattern(["C"], tags: "spring")
            .Script("universal", string.Empty, tags: "*")
            .Script("gear", string.Empty, tags: "gear,gear1")
            .Script("none", string.Empty, tags: "nothing")
            .Load();

        PatternInstance gears = run.Engine.Patterns.Permanent[0].Instances[0];
        PatternInstance capital = run.Engine.Patterns.Permanent[1].Instances[0];
        PatternInstance spring = run.Engine.Patterns.Permanent[2].Instances[0];

        LoadedScript universal = run.Runtime.Scripts.First(static s => s.Name == "universal");
        LoadedScript gear = run.Runtime.Scripts.First(static s => s.Name == "gear");
        LoadedScript none = run.Runtime.Scripts.First(static s => s.Name == "none");

        Assert.True(universal.IsLinkedTo(gears));
        Assert.True(universal.IsLinkedTo(capital));
        Assert.True(universal.IsLinkedTo(spring));

        Assert.True(gear.IsLinkedTo(gears));
        Assert.False(gear.IsLinkedTo(capital)); // case-sensitive
        Assert.False(gear.IsLinkedTo(spring));

        Assert.False(none.IsLinkedTo(gears));
    }

    /// <summary>
    /// REQ-LUA-006 — a frame instance is linked to the same scripts as the pattern instance it
    /// is a copy of (SPEC-04 REQ-E3D-190).
    /// </summary>
    [Fact] // REQ-LUA-006
    public void A_frame_instance_inherits_the_links_of_its_pattern_instance()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Pattern(["B"], tags: "u")
            .Script(
                "t",
                """
                seen = {}
                function UpdateS() seen[#seen + 1] = shape2D.palIndex end
                """,
                tags: "t")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 80, 80, palette: 1).Background('B', 120, 80, palette: 2));

        Assert.Equal(1, run.Num("#seen"));
        Assert.Equal(1, run.Num("seen[1]"));
    }

    /// <summary>
    /// REQ-LUA-007 — a disabled script receives <c>Start</c> and <c>End</c> but none of the
    /// other hooks, and <c>Script:Get(name).Enable</c> changes that from the next hook call on.
    /// </summary>
    [Fact] // REQ-LUA-007
    public void A_disabled_script_receives_only_Start_and_End()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script(
                "hud",
                """
                log = {}
                function Start() log[#log + 1] = "Start" end
                function End() log[#log + 1] = "End" end
                function Update() log[#log + 1] = "Update" end
                function UpdateS() log[#log + 1] = "UpdateS" end
                function LateUpdate() log[#log + 1] = "LateUpdate" end
                function Save() log[#log + 1] = "Save" end
                function Load() log[#log + 1] = "Load" end
                """,
                tags: "*",
                enabled: false)
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 80, 80));
        run.Runtime.InvokeSave();
        run.Runtime.InvokeLoad();
        run.Runtime.InvokeEnd();

        Assert.Equal("Start|End", run.Text("table.concat(log, '|')"));
    }

    /// <summary>
    /// REQ-LUA-007 — one script enabling another: the newly active script receives its frame
    /// hooks from the next frame on.
    /// </summary>
    [Fact] // REQ-LUA-007
    public void A_script_can_enable_another_one_through_the_script_object()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script(
                "ov",
                """
                ovRuns = 0
                function Update() ovRuns = ovRuns + 1 end
                """,
                tags: "*",
                enabled: false)
            .Script(
                "scene",
                """
                function Start() sOv = Script:Get("ov") end
                function Update() sOv.Enable = (turnOn == true) end
                """,
                tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 80, 80), counter: 1);
        Assert.Equal(0, run.Num("ovRuns"));

        run.Do("turnOn = true");
        // "scene" sorts after "ov", so "ov" is still disabled when its own Update is reached
        // this frame; the change shows from the following frame (REQ-LUA-007).
        run.Frame(new SyntheticBatch().Background('A', 80, 80), counter: 2);
        Assert.Equal(0, run.Num("ovRuns"));

        run.Frame(new SyntheticBatch().Background('A', 80, 80), counter: 3);
        Assert.Equal(1, run.Num("ovRuns"));
    }

    // ------------------------------------------------------- §3.2 the profile's Lua state

    /// <summary>
    /// REQ-LUA-010 — every script of a profile runs in a single Lua state whose globals are
    /// shared: a global one script sets in <c>UpdateS</c> is read by another in
    /// <c>LateUpdate</c>, within the same frame.
    /// </summary>
    [Fact] // REQ-LUA-010
    public void The_globals_are_shared_between_the_scripts_of_a_profile()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script(
                "bg",
                """
                function UpdateS() bgSlot = shape2D.palIndex; bgCount = (bgCount or 0) + 1 end
                """,
                tags: "t")
            .Script(
                "bg1",
                """
                function LateUpdate() observed = bgSlot; observedCount = bgCount end
                """,
                tags: "t")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 80, 80, palette: 3));

        Assert.Equal(3, run.Num("observed"));
        Assert.Equal(1, run.Num("observedCount"));
    }

    /// <summary>
    /// REQ-LUA-011 — the ten hook globals are set to nil before each chunk runs, so a script
    /// never inherits another script's hook even when it is loaded afterwards.
    /// </summary>
    [Fact] // REQ-LUA-011
    public void A_script_never_inherits_the_hooks_of_another()
    {
        using LuaRun run = new LuaProfile()
            .Script("A", "aRuns = 0\nfunction Update() aRuns = aRuns + 1 end")
            .Script("B", "bLoaded = true")
            .Start();

        LoadedScript a = run.Runtime.Scripts.First(static s => s.Name == "A");
        LoadedScript b = run.Runtime.Scripts.First(static s => s.Name == "B");

        Assert.True(a.Defines(ScriptHook.Update));
        Assert.False(b.Defines(ScriptHook.Update));

        run.Runtime.InvokeUpdate();
        Assert.Equal(1, run.Num("aRuns"));
    }

    /// <summary>
    /// REQ-LUA-012 — a global function whose name is <b>not</b> reserved follows Lua's ordinary
    /// rule: the last definition executed replaces the earlier ones for every script, hooks of
    /// scripts loaded earlier included.
    /// </summary>
    [Fact] // REQ-LUA-012
    public void A_non_reserved_global_function_is_replaced_by_the_last_definition()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script(
                "bg",
                """
                function Connect(a) return "bg" end
                function UpdateS() seenByBg = Connect(1) end
                """,
                tags: "t")
            .Script("foot_body", "function Connect(foot, body) return \"foot_body\" end", tags: "t")
            .Start();

        Assert.Equal("foot_body", run.Text("Connect(1, 2)"));

        run.Frame(new SyntheticBatch().Background('A', 80, 80));
        Assert.Equal("foot_body", run.Text("seenByBg"));
    }

    /// <summary>
    /// REQ-LUA-013 — the globals live as long as the Lua state does, and relaunching the same
    /// game does not re-run the chunks: only <c>End</c> then <c>Start</c> are called.
    /// </summary>
    [Fact] // REQ-LUA-013
    public void Relaunching_the_same_game_keeps_the_state_and_does_not_re_run_the_chunks()
    {
        using LuaRun run = new LuaProfile()
            .Script(
                "a",
                """
                n = (n or 0) + 1
                starts = 0
                function Start() starts = starts + 1; printed = n end
                """)
            .Start();

        Assert.Equal(1, run.Num("n"));
        Assert.Equal(1, run.Num("printed"));

        // A relaunch of the same game: End then Start, the Lua state kept.
        run.Runtime.InvokeEnd();
        run.Runtime.InvokeStart();

        Assert.Equal(1, run.Num("n"));
        Assert.Equal(2, run.Num("starts"));
    }

    /// <summary>
    /// REQ-LUA-013 — <c>enable</c> and the error flag survive a relaunch of the same game
    /// (also REQ-LUA-062).
    /// </summary>
    [Fact] // REQ-LUA-013
    public void Enable_and_the_error_flag_survive_a_relaunch()
    {
        using LuaRun run = new LuaProfile()
            .Script("a", "function Update() error('boom') end\nfunction Start() end\nfunction End() end")
            .Script("b", "bEnable = true\nfunction Start() end")
            .Start();

        LoadedScript a = run.Runtime.Scripts.First(static s => s.Name == "a");
        LoadedScript b = run.Runtime.Scripts.First(static s => s.Name == "b");
        b.Enabled = false;

        run.Runtime.InvokeUpdate();
        Assert.True(a.Failed);

        run.Runtime.InvokeEnd();
        run.Runtime.InvokeStart();

        Assert.True(a.Failed);
        Assert.False(b.Enabled);
    }

    /// <summary>
    /// REQ-LUA-014 — a change of game destroys the previous profile's Lua state: no global and
    /// no persistent datum survives from one profile to the next.
    /// </summary>
    [Fact] // REQ-LUA-014
    public void Changing_profile_leaves_nothing_behind()
    {
        using (LuaRun first = new LuaProfile().Script("a", "leak = 1\nPData[6] = 42").Start())
        {
            Assert.Equal(1, first.Num("leak"));
            Assert.Equal(42, first.Num("PData[6]"));
        }

        using LuaRun second = new LuaProfile().Script("a", string.Empty).Start();
        Assert.Equal("nil", second.TypeOf("leak"));
        Assert.Equal(0, second.Num("PData[6]"));
    }

    /// <summary>
    /// REQ-LUA-015 — <c>BuildPattern</c> runs at step 3, that is before any <c>Start</c>.
    /// </summary>
    [Fact] // REQ-LUA-015, REQ-LUA-030
    public void BuildPattern_runs_before_every_Start()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script(
                "a",
                """
                order = {}
                function BuildPattern() order[#order + 1] = "BuildPattern" end
                function Start() order[#order + 1] = "Start" end
                """,
                tags: "*")
            .Start();

        Assert.Equal("BuildPattern|Start", run.Text("table.concat(order, '|')"));
    }

    // -------------------------------------------------------- §3.3 life-cycle hooks

    /// <summary>
    /// REQ-LUA-020, REQ-LUA-021 — <c>Start</c> and <c>End</c> are called for every script that
    /// is not in error, active or not, in script order.
    /// </summary>
    [Fact] // REQ-LUA-020, REQ-LUA-021
    public void Start_and_End_run_in_script_order_for_active_and_inactive_scripts()
    {
        using LuaRun run = new LuaProfile()
            .Script("b", "function Start() log[#log + 1] = 'b.Start' end\n"
                + "function End() log[#log + 1] = 'b.End' end")
            .Script("!prelude", "log = {}")
            .Script("a", "function Start() log[#log + 1] = 'a.Start' end\n"
                + "function End() log[#log + 1] = 'a.End' end", enabled: false)
            .Start();

        run.Runtime.InvokeEnd();

        Assert.Equal("a.Start|b.Start|a.End|b.End", run.Text("table.concat(log, '|')"));
    }

    /// <summary>REQ-LUA-022 — a hook the chunk does not define is simply never called.</summary>
    [Fact] // REQ-LUA-022
    public void An_absent_hook_is_not_an_error()
    {
        using LuaRun run = new LuaProfile()
            .Script("empty", string.Empty)
            .Script("notafunction", "Update = 3")
            .Start();

        run.Runtime.InvokeUpdate();
        run.Runtime.InvokeLateUpdate();
        run.Runtime.InvokeEnd();

        Assert.Empty(run.Log.Errors);
        Assert.All(run.Runtime.Scripts, static script => Assert.False(script.Failed));
        Assert.False(run.Runtime.Scripts.First(static s => s.Name == "notafunction")
            .Defines(ScriptHook.Update));
    }

    /// <summary>
    /// REQ-LUA-023 — during <c>Start</c>, <c>End</c>, <c>Load</c> and <c>Save</c> the globals
    /// <c>frame</c> and <c>time</c> hold the value of the last converted frame, and 0 before the
    /// first frame of the Lua state.
    /// </summary>
    [Fact] // REQ-LUA-023
    public void Frame_and_time_are_zero_before_the_first_frame_and_then_hold_the_last_one()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "t")
            .Script(
                "a",
                """
                function Start() atStart = frame; timeAtStart = time end
                function Save() atSave = frame end
                function End() atEnd = frame end
                """)
            .Start();

        Assert.Equal(0, run.Num("atStart"));
        Assert.Equal(0, run.Num("timeAtStart"));

        run.Frame(new SyntheticBatch().Background('A', 80, 80), counter: 120);
        run.Runtime.InvokeSave();
        run.Runtime.InvokeEnd();

        Assert.Equal(120, run.Num("atSave"));
        Assert.Equal(120, run.Num("atEnd"));
    }
}
