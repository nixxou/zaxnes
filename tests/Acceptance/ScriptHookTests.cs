using Zax.Engine3D;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for the Lua hook points of SPEC-04 §3.14 and SPEC-04B
/// REQ-E3D-455. The hooks are exercised through a recording script host, so the checks are
/// about the pipeline's call order and effects, not about the Lua runtime (SPEC-05, part B).
/// </summary>
public sealed class ScriptHookTests
{
    /// <summary>A script host that records what the pipeline asks of it.</summary>
    private sealed class RecordingHost : IScriptHost
    {
        public List<string> Calls { get; } = [];

        public List<PatternInstance> BuiltPatterns { get; } = [];

        public List<FrameInstance> UpdatedInstances { get; } = [];

        public Action<IReadOnlyList<FrameInstance>>? OnUpdateShapes { get; set; }

        public Action? OnUpdate { get; set; }

        public int Frame { get; private set; } = -1;

        public float Time { get; private set; } = -1;

        public bool HasFrameHooks => true;

        public void SetFrameGlobals(int frameCounter, float time)
        {
            Calls.Add("globals");
            Frame = frameCounter;
            Time = time;
        }

        public void InvokeUpdate()
        {
            Calls.Add("Update");
            OnUpdate?.Invoke();
        }

        public void InvokeUpdateShapes(IReadOnlyList<FrameInstance> scriptableInstances)
        {
            Calls.Add("UpdateS");
            UpdatedInstances.AddRange(scriptableInstances);
            OnUpdateShapes?.Invoke(scriptableInstances);
        }

        public void InvokeLateUpdate() => Calls.Add("LateUpdate");

        public void InvokeBuildPattern(PatternInstance patternInstance)
        {
            Calls.Add("BuildPattern");
            BuiltPatterns.Add(patternInstance);
        }
    }

    [Fact] // REQ-E3D-320
    public void The_hooks_run_in_the_documented_order()
    {
        ConversionEngine engine = new();
        RecordingHost host = new();
        engine.ScriptHost = host;
        SyntheticPatterns.Build(engine, ["A"], tags: "t");

        new SyntheticBatch().Background('A', 80, 80).Convert(engine, 120);

        int globals = host.Calls.IndexOf("globals");
        int update = host.Calls.IndexOf("Update");
        int updateS = host.Calls.IndexOf("UpdateS");
        int late = host.Calls.IndexOf("LateUpdate");

        Assert.True(globals >= 0 && update > globals, string.Join(',', host.Calls));
        Assert.True(updateS > update, string.Join(',', host.Calls));
        Assert.True(late > updateS, string.Join(',', host.Calls));

        Assert.Equal(120, host.Frame);
        Assert.Equal(2.0f, host.Time, 4);
    }

    [Fact] // REQ-E3D-320 (UpdateS sees shapes that are already segmented)
    public void Update_shapes_receives_the_segmented_instances()
    {
        ConversionEngine engine = new();
        RecordingHost host = new();
        engine.ScriptHost = host;
        SyntheticPatterns.Build(engine, ["A"], tags: "t");

        ConversionFrame frame = new SyntheticBatch()
            .Background('A', 80, 80)
            .Background('A', 160, 80)
            .Convert(engine);

        Assert.Equal(frame.ScriptableInstances.Count, host.UpdatedInstances.Count);
        Assert.Equal(2, host.UpdatedInstances.Count);
    }

    [Fact] // REQ-E3D-001, REQ-E3D-320 (Pos reflects what UpdateS changed)
    public void Positions_are_computed_after_the_hooks()
    {
        ConversionEngine engine = new();
        RecordingHost host = new()
        {
            OnUpdateShapes = instances =>
            {
                foreach (FrameInstance instance in instances)
                {
                    instance.Offset = new Vec3(100, 0, 0);
                }
            },
        };

        engine.ScriptHost = host;
        SyntheticPatterns.Build(engine, ["A"]);

        ConversionFrame frame = new SyntheticBatch().Background('A', 16, 32).Convert(engine);
        Assert.Equal(120f, frame.Instances[0].Pos.X, 3);
    }

    [Fact] // REQ-E3D-320 (a shape created during Update is drawn with the same frame)
    public void A_shape_created_during_update_joins_the_frame()
    {
        ConversionEngine engine = new();
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);

        RecordingHost host = new();
        host.OnUpdate = () => engine.CreateNewShape(pattern, new Vec3(40, 40, 0), palette: 5);
        engine.ScriptHost = host;

        ConversionFrame frame = new SyntheticBatch().Convert(engine);
        FrameShape injected = Assert.Single(frame.Shapes);
        Assert.False(injected.Scriptable);
    }

    [Fact] // REQ-E3D-320 (disabling a pattern only changes the next frame)
    public void Disabling_a_pattern_during_update_only_bites_next_frame()
    {
        ConversionEngine engine = new();
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);

        RecordingHost host = new();
        host.OnUpdate = () => pattern.Disabled = true;
        engine.ScriptHost = host;

        ConversionFrame first = new SyntheticBatch().Background('A', 80, 80).Convert(engine, 1);
        Assert.Single(first.Shapes);

        ConversionFrame second = new SyntheticBatch().Background('A', 80, 80).Convert(engine, 2);
        Assert.Empty(second.Shapes);
    }

    [Fact] // REQ-E3D-321
    public void Tag_binding_is_exact_and_case_sensitive()
    {
        ConversionEngine engine = new();
        EnginePattern boss = SyntheticPatterns.Build(engine, ["A"], tags: "boss");
        EnginePattern capital = SyntheticPatterns.Build(engine, ["B"], tags: "Boss");

        Assert.True(boss.Instances[0].HasTag("boss"));
        Assert.False(boss.Instances[0].HasTag("Boss"));
        Assert.True(capital.Instances[0].HasTag("Boss"));
        Assert.False(capital.Instances[0].HasTag("boss"));

        Assert.True(ScriptLinking.IsLinked("enemy, boss", boss.Instances[0]));
        Assert.False(ScriptLinking.IsLinked("enemy, boss", capital.Instances[0]));
        Assert.True(ScriptLinking.IsLinked("*", capital.Instances[0]));
    }

    [Fact] // REQ-E3D-322, REQ-E3D-340, REQ-E3D-455
    public void Build_pattern_runs_again_on_every_instance_when_the_algorithm_changes()
    {
        ConversionEngine engine = new();
        engine.UseGeometry();

        EnginePattern first = SyntheticPatterns.Build(engine, ["A"]);
        EnginePattern second = SyntheticPatterns.Build(engine, ["B"]);

        RecordingHost host = new();
        engine.ScriptHost = host;

        engine.SetRenderAlgorithm(RenderAlgorithm.Marching);

        Assert.Contains(first.Instances[0], host.BuiltPatterns);
        Assert.Contains(second.Instances[0], host.BuiltPatterns);
        Assert.Equal(RenderAlgorithm.Marching, engine.Settings.RenderAlgorithm);
    }

    [Fact] // REQ-E3D-322, REQ-E3D-455 (what the hook sets is what gets built)
    public void What_the_build_hook_changes_is_what_is_built()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();

        RecordingHost host = new();
        engine.ScriptHost = host;
        engine.Geometry = geometry;

        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);

        // Emulate a BuildPattern hook that sets sizeZ = 8 before the field is built.
        pattern.Instances[0].Depth = 8;
        geometry.Build(pattern);
        geometry.Commit();

        ObjectGeometry built = geometry.Of(pattern.Instances[0])!;
        Assert.Equal(8, built.BuildDepth);
    }

    [Fact] // REQ-E3D-322 (a learned pattern is built too)
    public void A_learned_pattern_goes_through_the_build_hook()
    {
        ConversionEngine engine = new();
        engine.UseGeometry();
        RecordingHost host = new();
        engine.ScriptHost = host;

        new SyntheticBatch().Sprite('Z', 120, 120).Convert(engine, 1);

        EnginePattern learned = Assert.Single(engine.Patterns.Permanent);
        Assert.Contains(learned.Instances[0], host.BuiltPatterns);
    }

    [Fact] // REQ-E3D-323
    public void Detect_shape_is_never_invoked_by_the_pipeline()
    {
        // The pipeline's host interface has no DetectShape entry point at all, so the hook
        // cannot be called from the conversion: the requirement holds by construction.
        Assert.DoesNotContain(
            typeof(IScriptHost).GetMethods(),
            m => m.Name.Contains("DetectShape", StringComparison.Ordinal));
    }

    [Fact] // REQ-E3D-324
    public void A_lua_error_in_a_hook_does_not_stop_the_conversion()
    {
        ConversionEngine engine = new();
        SyntheticPatterns.Build(engine, ["A"], tags: "t");

        ThrowingHost host = new();
        engine.ScriptHost = host;

        ConversionFrame frame = new SyntheticBatch().Background('A', 80, 80).Convert(engine, 1);
        Assert.Single(frame.Shapes);
        Assert.True(frame.ScriptErrorCount > 0);

        host.Throw = false;
        ConversionFrame next = new SyntheticBatch().Background('A', 80, 80).Convert(engine, 2);
        Assert.Single(next.Shapes);
        Assert.Equal(0, next.ScriptErrorCount);
    }

    private sealed class ThrowingHost : IScriptHost
    {
        public bool Throw { get; set; } = true;

        public bool HasFrameHooks => true;

        public void SetFrameGlobals(int frameCounter, float time)
        {
        }

        public void InvokeUpdate()
        {
        }

        public void InvokeUpdateShapes(IReadOnlyList<FrameInstance> scriptableInstances)
        {
            if (Throw)
            {
                throw new InvalidOperationException("Lua error");
            }
        }

        public void InvokeLateUpdate()
        {
        }

        public void InvokeBuildPattern(PatternInstance patternInstance)
        {
        }
    }
}
