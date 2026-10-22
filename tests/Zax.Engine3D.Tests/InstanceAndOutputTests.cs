using Zax.Profiles;

namespace Zax.Engine3D.Tests;

/// <summary>
/// Binding and frame instances (SPEC-04 §3.8), order and script queries (§3.9), injected shapes
/// and the pinned frame (§3.10), frame management (§3.12), hooks (§3.14) and the output contract
/// of SPEC-04B §3.8.
/// </summary>
public sealed class InstanceAndOutputTests
{
    private static ConversionEngine OneTileEngine(
        int depth = 3,
        ProfileLayer layer = ProfileLayer.L1,
        string? tags = null,
        int instanceCount = 1)
        => new ProfileBuilder()
            .Pattern(1, 1, [Img.A], depth: depth, layer: layer, tags: tags, instanceCount: instanceCount)
            .Load();

    private static ConversionFrame OneTileFrame(ConversionEngine engine, int x = 16, int y = 32)
    {
        Batch batch = new();
        batch.Bg(Img.A, x, y);
        return batch.Convert(engine);
    }

    [Fact]
    public void REQ_E3D_190_every_parameter_is_copied_and_the_custom_palette_is_reset()
    {
        ConversionEngine engine = OneTileEngine(tags: "t");
        PatternInstance source = engine.Patterns.Permanent[0].Instances[0];
        source.Offset = new Vec3(1f, 2f, 3f);
        source.Scale = new Vec3(2f, 2f, 2f);
        source.Rotation = new Vec3(10f, 20f, 30f);
        source.Alpha = 0.5f;
        source.CustomPalette = 9;
        source.EmptyColor = 4;
        source.DeformAmplitude = new Vec3(0f, 0.1f, 0f);
        source.SetCustomData(5, 7f);

        FrameInstance instance = OneTileFrame(engine).Instances[0];

        Assert.Equal(new Vec3(1f, 2f, 3f), instance.Offset);
        Assert.Equal(new Vec3(2f, 2f, 2f), instance.Scale);
        Assert.Equal(new Vec3(10f, 20f, 30f), instance.Rotation);
        Assert.Equal(0.5f, instance.Alpha);
        Assert.Equal(4, instance.EmptyColor);
        Assert.Equal(7f, instance.CustomData(5));
        Assert.Equal(["t"], instance.Tags);
        Assert.Same(source, instance.Reference);

        // The custom palette comes back to "none" every frame.
        Assert.Equal(-1, instance.CustomPalette);
    }

    [Fact]
    public void REQ_E3D_191_a_hidden_instance_stays_in_the_frame_but_is_not_drawn()
    {
        ConversionEngine engine = OneTileEngine();
        RecordingGeometryBuilder geometry = new();
        engine.Geometry = geometry;

        ConversionFrame frame = OneTileFrame(engine);
        frame.Instances[0].Enable = false;

        Assert.Single(frame.Instances);
        Assert.Equal(RenderStyle.Hidden, frame.Instances[0].Style);
        Assert.Empty(engine.BuildDrawList(frame).Tiles);
    }

    [Fact]
    public void REQ_E3D_192_the_instance_palette_falls_back_to_the_shape_palette()
    {
        ConversionEngine engine = OneTileEngine();
        Batch batch = new();
        batch.Bg(Img.A, 16, 32, palette: 2);
        FrameInstance instance = batch.Convert(engine).Instances[0];

        Assert.Equal(2, instance.PaletteTable);

        instance.CustomPalette = 9;
        Assert.Equal(9, instance.PaletteTable);
    }

    [Fact]
    public void REQ_E3D_205_the_placement_box_and_the_anchor()
    {
        ConversionEngine engine = OneTileEngine();
        FrameInstance instance = OneTileFrame(engine).Instances[0];

        Assert.Equal(new Vec3(16f, 208f, 0f), instance.BoxMin(engine.Settings));
        Assert.Equal(new Vec3(24f, 216f, 3f), instance.BoxMax(engine.Settings));
        Assert.Equal(new Vec3(20f, 212f, 1.5f), instance.Pos);
        Assert.Equal(new Vec3(20f, 212f, 1.5f), instance.Center);
        Assert.Equal(new Vec3(16f, 208f, -1.5f), instance.BL);
        Assert.Equal(new Vec3(23f, 215f, 1.5f), instance.TR);
    }

    [Fact]
    public void REQ_E3D_205_the_effective_layer_of_an_unidentified_instance()
    {
        ConversionEngine engine = OneTileEngine(layer: ProfileLayer.Unidentified);
        FrameInstance background = OneTileFrame(engine).Instances[0];
        Assert.Equal(ZLayer.L1, background.EffectiveLayer(engine.Settings));

        engine.Settings.DefaultLayer = ZLayer.L3;
        Assert.Equal(ZLayer.L3, background.EffectiveLayer(engine.Settings));

        // A sprite shape always falls back to L1.
        ConversionEngine spriteEngine = new ProfileBuilder()
            .Pattern(1, 1, [Img.A], layer: ProfileLayer.Unidentified)
            .Load();
        spriteEngine.Settings.DefaultLayer = ZLayer.L3;
        Batch batch = new();
        batch.Sprite(Img.A, 16, 32);
        FrameInstance sprite = batch.Convert(spriteEngine).Instances[0];
        Assert.Equal(ZLayer.L1, sprite.EffectiveLayer(spriteEngine.Settings));
    }

    [Fact]
    public void REQ_E3D_205_the_layer_offset_and_alignment_move_the_depth_interval()
    {
        ConversionEngine engine = OneTileEngine(depth: 4, layer: ProfileLayer.L2);
        FrameInstance instance = OneTileFrame(engine).Instances[0];

        (float front, float back) = instance.DepthInterval(engine.Settings);
        Assert.Equal(12f, front);
        Assert.Equal(16f, back);

        engine.Settings.Layer(ZLayer.L2).Offset = 0f;
        engine.Settings.Layer(ZLayer.L2).Alignment = ZAlignment.Center;
        (front, back) = instance.DepthInterval(engine.Settings);
        Assert.Equal(-2f, front);
        Assert.Equal(2f, back);

        engine.Settings.Layer(ZLayer.L2).Alignment = ZAlignment.Back;
        (front, back) = instance.DepthInterval(engine.Settings);
        Assert.Equal(-4f, front);
        Assert.Equal(0f, back);
    }

    [Fact]
    public void REQ_E3D_206_the_exposed_derived_quantities()
    {
        ConversionEngine engine = OneTileEngine();
        ConversionFrame frame = OneTileFrame(engine);
        FrameShape shape = frame.Shapes[0];
        FrameInstance instance = frame.Instances[0];

        Assert.Equal(new Vec2(16f, 208f), shape.BottomLeft);
        Assert.Equal(new Vec2(23f, 215f), shape.TopRight);
        Assert.Equal(new Vec3(16f, 209f, -1.5f), instance.BottomLeft);
        Assert.Equal(new Vec3(23f, 216f, 1.5f), instance.TopRight);
        Assert.Equal(new Vec3(16f, 208f, -1.5f), instance.BL);
        Assert.Equal(new Vec3(8f, 8f, 3f), instance.Size);
        Assert.Equal(new Cell(8, 8), shape.PixelSize);
        Assert.Equal(0, instance.Index);
    }

    [Fact]
    public void REQ_E3D_207_the_age_of_a_shape_is_always_one()
        => Assert.Equal(1, FrameShape.Age);

    [Fact]
    public void REQ_E3D_220_scriptable_instances_are_the_segmentation_ones_in_order()
    {
        ConversionEngine engine = OneTileEngine(instanceCount: 2);
        EnginePattern pattern = engine.Patterns.Permanent[0];

        RecordingScriptHost host = new();
        engine.ScriptHost = host;
        host.OnUpdate = () => engine.CreateNewShape(pattern, new Vec3(100f, 100f, 0f), 5);

        Batch batch = new();
        batch.Bg(Img.A, 16, 32).Bg(Img.A, 100, 100);
        ConversionFrame frame = batch.Convert(engine, 1);

        // Two segmented shapes of two instances each, plus the injected shape.
        Assert.Equal(3, frame.Shapes.Count);
        Assert.Equal(6, frame.Instances.Count);
        Assert.Equal(4, frame.ScriptableInstances.Count);
        Assert.Same(frame.Shapes[0].Instances[0], engine.GetShape(0));
        Assert.Same(frame.Shapes[0].Instances[1], engine.GetShape(1));
        Assert.Same(frame.Shapes[1].Instances[0], engine.GetShape(2));

        // A shape created by a script is not returned by the tag queries.
        List<FrameInstance> all = [];
        engine.CollectShapesWithTag("*", all);
        Assert.Equal(4, all.Count);
        Assert.DoesNotContain(frame.Shapes[2].Instances[0], all);
    }

    [Fact]
    public void REQ_E3D_231_add_new_shape_places_the_tiles_at_the_grid_origin()
    {
        ConversionEngine engine = OneTileEngine();
        EnginePattern pattern = engine.Patterns.Permanent[0];

        RecordingScriptHost host = new();
        engine.ScriptHost = host;
        host.OnUpdate = () => engine.AddNewShape(pattern, new Vec3(40f, 100f, 0f), 5);

        ConversionFrame frame = new Batch().Convert(engine, 1);

        FrameShape shape = frame.Shapes[0];
        Assert.Equal(0, shape.Tiles[0].X);
        Assert.Equal(0, shape.Tiles[0].Y);
        Assert.Equal(new Vec3(44f, 104f, 1.5f), shape.Instances[0].Pos);
    }

    [Fact]
    public void REQ_E3D_232_insert_shape_equals_create_new_shape_shifted_by_the_bottom_left()
    {
        ConversionEngine engine = OneTileEngine();
        EnginePattern pattern = engine.Patterns.Permanent[0];
        Assert.Equal(new Vec2(0f, 0f), pattern.BottomLeft);

        RecordingScriptHost host = new();
        engine.ScriptHost = host;
        host.OnUpdate = () =>
        {
            engine.InsertShape(pattern, new Vec3(10f, 20f, 0f), 5);
            engine.CreateNewShape(pattern, new Vec3(10f, 20f, 0f), 5);
        };

        ConversionFrame frame = new Batch().Convert(engine, 1);

        Assert.Equal(2, frame.Shapes.Count);
        Assert.Equal(frame.Shapes[1].Tiles[0].X, frame.Shapes[0].Tiles[0].X);
        Assert.Equal(frame.Shapes[1].Tiles[0].Y, frame.Shapes[0].Tiles[0].Y);
    }

    [Fact]
    public void REQ_E3D_230_with_an_instance_only_that_instance_is_enabled()
    {
        ConversionEngine engine = OneTileEngine(instanceCount: 3);
        PatternInstance second = engine.Patterns.Permanent[0].Instances[1];

        RecordingScriptHost host = new();
        engine.ScriptHost = host;
        FrameInstance? returned = null;
        host.OnUpdate = () => returned = engine.CreateNewShape(second, new Vec3(0f, 0f, 0f), 5);

        ConversionFrame frame = new Batch().Convert(engine, 1);

        FrameShape shape = frame.Shapes[0];
        Assert.Same(shape.Instances[1], returned);
        Assert.False(shape.Instances[0].Enable);
        Assert.True(shape.Instances[1].Enable);
        Assert.False(shape.Instances[2].Enable);
    }

    [Fact]
    public void REQ_E3D_233_a_pinned_frame_restores_its_shapes_where_a_rectangle_overlaps()
    {
        ConversionEngine engine = OneTileEngine();

        Batch first = new();
        first.Bg(Img.A, 16, 32).Bg(Img.A, 160, 160);
        ConversionFrame pinned = first.ConvertAndPublish(engine, 1);
        Assert.Equal(2, pinned.Shapes.Count);

        engine.PinNewFrame();
        Assert.Same(pinned, engine.PinnedFrame);

        RecordingScriptHost host = new();
        engine.ScriptHost = host;
        host.OnUpdate = () => engine.RestoreFromPinnedFrame(new Rect(14f, 30f, 12f, 12f));

        ConversionFrame second = new Batch().Convert(engine, 2);

        Assert.Single(second.Shapes);
        FrameShape restored = second.Shapes[0];
        Assert.Same(pinned.Shapes[0], restored);
        Assert.True(restored.RestoredFromPinnedFrame);
        Assert.False(restored.Scriptable);
        Assert.Empty(second.ScriptableInstances);

        // Only the tiles whose 8 x 8 cell overlaps a rectangle are drawn.
        Assert.True(second.TileOverlapsPinnedRects(restored.Tiles[0]));
        Assert.Single(engine.BuildDrawList(second).Tiles);

        // The pinned frame itself is untouched by the new conversion.
        Assert.Equal(2, pinned.Shapes.Count);
        Assert.Equal(1, pinned.Counter);
    }

    [Fact]
    public void REQ_E3D_233_without_a_pinned_frame_the_calls_do_nothing()
    {
        ConversionEngine engine = OneTileEngine();
        ConversionFrame frame = new Batch().Convert(engine, 1);

        engine.AddRectFromPinnedFrame(0f, 0f, 256f, 248f);
        Assert.Empty(frame.PinnedRects);
        Assert.Equal(0, engine.RestoreFromPinnedFrame());

        engine.ReleasePinnedFrame();
        Assert.Null(engine.PinnedFrame);
    }

    [Fact]
    public void REQ_E3D_280_a_new_conversion_never_touches_the_published_or_pinned_frame()
    {
        ConversionEngine engine = OneTileEngine();

        Batch first = new();
        first.Bg(Img.A, 16, 32);
        ConversionFrame published = first.ConvertAndPublish(engine, 1);
        engine.PinNewFrame();

        for (int counter = 2; counter < 8; counter++)
        {
            Batch batch = new();
            batch.Bg(Img.A, 24, 32);
            ConversionFrame frame = batch.Convert(engine, counter);
            Assert.NotSame(published, frame);
            Assert.NotSame(engine.PinnedFrame, frame);
            engine.Publish();
        }

        Assert.Single(published.Shapes);
        Assert.Equal(1, published.Counter);
        Assert.Equal(new Cell(16, 32), published.Shapes[0].PixelStart);
    }

    [Fact]
    public void REQ_E3D_281_clearing_the_frames_empties_everything()
    {
        ConversionEngine engine = OneTileEngine();
        Batch batch = new();
        batch.Bg(Img.A, 16, 32);
        batch.ConvertAndPublish(engine, 5);
        engine.PinNewFrame();

        engine.ClearFrames();

        Assert.Null(engine.PublishedFrame);
        Assert.Null(engine.PinnedFrame);
        Assert.Empty(engine.ConvertingFrame.Shapes);
        Assert.Equal(0, engine.ConvertingFrame.Counter);
    }

    [Fact]
    public void REQ_E3D_320_the_hooks_run_in_order_after_the_segmentation()
    {
        ConversionEngine engine = OneTileEngine();
        RecordingScriptHost host = new();
        engine.ScriptHost = host;

        int shapesSeenByUpdate = -1;
        host.OnUpdate = () => shapesSeenByUpdate = engine.ConvertingFrame.Shapes.Count;

        Batch batch = new();
        batch.Bg(Img.A, 16, 32);
        batch.Convert(engine, 7);

        Assert.Equal(["globals(7)", "Update", "UpdateS[1]", "LateUpdate"], host.Calls);
        Assert.Equal(1, shapesSeenByUpdate);
        Assert.Equal(7, host.LastFrame);
        Assert.Equal(7f / 60f, host.LastTime, 5);
    }

    [Fact]
    public void REQ_E3D_001_the_position_takes_the_offset_a_hook_changed()
    {
        ConversionEngine engine = OneTileEngine();
        RecordingScriptHost host = new()
        {
            OnUpdateShape = static instance => instance.Offset.Y += 10f,
        };
        engine.ScriptHost = host;

        ConversionFrame frame = OneTileFrame(engine);

        Assert.Equal(222f, frame.Instances[0].Pos.Y);
    }

    [Fact]
    public void REQ_E3D_321_a_script_is_linked_by_star_or_by_an_equal_tag()
    {
        ConversionEngine engine = OneTileEngine(tags: "boss");
        PatternInstance instance = engine.Patterns.Permanent[0].Instances[0];

        Assert.True(ScriptLinking.IsLinked("enemy, boss", instance));
        Assert.False(ScriptLinking.IsLinked("enemy, Boss", instance));
        Assert.True(ScriptLinking.IsLinked("*", instance));
        Assert.True(ScriptLinking.IsLinked("ui,*", instance));
        Assert.True(ScriptLinking.IsLinked("a*b", instance));
        Assert.False(ScriptLinking.IsLinked(string.Empty, instance));
    }

    [Fact]
    public void REQ_E3D_322_build_pattern_runs_before_every_construction()
    {
        ProfileBuilder builder = new ProfileBuilder().Pattern(1, 1, [Img.A], tags: "*");
        ConversionEngine engine = new();
        RecordingGeometryBuilder geometry = new();
        List<string> order = [];
        RecordingScriptHost host = new()
        {
            OnBuildPattern = instance =>
            {
                instance.Depth = 8;
                order.Add("hook");
            },
        };
        engine.ScriptHost = host;
        engine.Geometry = geometry;

        engine.LoadProfile(builder.Build());

        Assert.Single(host.BuiltPatterns);
        Assert.Equal(8, engine.Patterns.Permanent[0].Instances[0].Depth);
        Assert.Single(geometry.Built);

        // Learning a pattern runs the hook for it too, before its construction.
        Batch batch = new();
        batch.BgCell(Img.U, 10, 10);
        batch.Convert(engine, 1);
        Assert.Equal(2, host.BuiltPatterns.Count);
        Assert.Equal(2, geometry.Built.Count);
        Assert.Equal(8, engine.Patterns.Permanent[1].Instances[0].Depth);
    }

    [Fact]
    public void REQ_E3D_323_the_pipeline_has_no_detect_shape_call_point()
    {
        // The hook is recognised by the runtime but never invoked, so the interface the
        // pipeline drives carries no such member (DEC-04-Q01).
        string[] members = typeof(IScriptHost).GetMembers().Select(static m => m.Name).ToArray();
        Assert.DoesNotContain("DetectShape", members);
        Assert.DoesNotContain("InvokeDetectShape", members);
    }

    [Fact]
    public void REQ_E3D_324_a_hook_error_does_not_interrupt_the_conversion()
    {
        ConversionEngine engine = OneTileEngine();
        RecordingScriptHost host = new()
        {
            OnUpdateShape = static _ => throw new InvalidOperationException("lua error"),
        };
        engine.ScriptHost = host;

        Batch batch = new();
        batch.Bg(Img.A, 16, 32);
        ConversionFrame frame = batch.Convert(engine, 1);

        Assert.Equal(1, frame.ScriptErrorCount);
        Assert.Single(frame.Shapes);
        Assert.Equal(new Vec3(20f, 212f, 1.5f), frame.Instances[0].Pos);

        host.OnUpdateShape = null;
        Batch next = new();
        next.Bg(Img.A, 16, 32);
        ConversionFrame clean = next.Convert(engine, 2);
        Assert.Equal(0, clean.ScriptErrorCount);
    }

    [Fact]
    public void REQ_E3D_340_changing_the_global_algorithm_rebuilds_every_pattern()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Row([Img.B]).Load();
        RecordingGeometryBuilder geometry = new();
        engine.Geometry = geometry;

        engine.SetRenderAlgorithm(RenderAlgorithm.Marching);
        Assert.Equal(RenderAlgorithm.Marching, engine.Settings.RenderAlgorithm);
        Assert.Equal(2, geometry.Built.Count);

        // Setting the same value again rebuilds nothing.
        engine.SetRenderAlgorithm(RenderAlgorithm.Marching);
        Assert.Equal(2, geometry.Built.Count);
    }

    [Fact]
    public void REQ_E3D_340_the_per_instance_algorithm_wins_over_the_setting()
    {
        PatternInstance instance = new() { MeshAlgorithm = 2 };
        Assert.Equal(RenderAlgorithm.Marching, instance.EffectiveAlgorithm(RenderAlgorithm.Greedy));
        Assert.Equal(RenderAlgorithm.Marching, instance.EffectiveAlgorithm(RenderAlgorithm.GreedyMulticolor));

        instance.MeshAlgorithm = 0;
        Assert.Equal(RenderAlgorithm.Greedy, instance.EffectiveAlgorithm(RenderAlgorithm.Greedy));
        Assert.Equal(RenderAlgorithm.Marching, instance.EffectiveAlgorithm(RenderAlgorithm.Marching));
    }

    [Fact]
    public void REQ_E3D_361_a_shape_without_geometry_is_kept_but_not_drawn()
    {
        ConversionEngine engine = OneTileEngine();
        RecordingGeometryBuilder geometry = new();
        engine.Geometry = geometry;
        geometry.NotReady.Add(engine.Patterns.Permanent[0].Instances[0]);

        ConversionFrame frame = OneTileFrame(engine);

        Assert.Single(frame.Shapes);
        Assert.Single(frame.ScriptableInstances);
        Assert.Empty(engine.BuildDrawList(frame).Tiles);
    }

    [Theory]
    [InlineData(0, 0, 0, 1f)]
    [InlineData(50, 0, 100, 0.25f)]
    [InlineData(100, 100, 100, 1f)]
    [InlineData(100, 0, 100, 0.5f)]
    public void REQ_E3D_362_the_loading_progress(int built, int transferred, int toBuild, float expected)
    {
        ConversionEngine engine = OneTileEngine();
        RecordingGeometryBuilder geometry = new()
        {
            ReportedProgress = new GeometryProgress(built, transferred, toBuild),
        };
        engine.Geometry = geometry;

        Assert.Equal(expected, engine.LoadProgress, 5);
    }

    [Fact]
    public void REQ_E3D_250_254_the_scene_background_colour()
    {
        ConversionEngine engine = OneTileEngine();
        Batch batch = new();
        batch.Palette(0, 0x21).Bg(Img.A, 16, 32);
        batch.ConvertAndPublish(engine, 1);

        Assert.Equal(new Rgb24(0x38, 0xC0, 0xFC), engine.BackgroundColor(skyboxActive: false));

        // A skybox, or the setting turned off, gives the custom colour.
        Assert.Equal(engine.Settings.CustomBackgroundColor, engine.BackgroundColor(skyboxActive: true));
        engine.Settings.UseNesBackgroundColor = false;
        engine.Settings.CustomBackgroundColor = new Rgb24(1, 2, 3);
        Assert.Equal(new Rgb24(1, 2, 3), engine.BackgroundColor(skyboxActive: false));
    }

    [Fact]
    public void REQ_E3D_252_a_palette_write_only_affects_the_frame_it_targets()
    {
        ConversionEngine engine = OneTileEngine();
        RecordingScriptHost host = new();
        engine.ScriptHost = host;

        Batch first = new();
        first.Palette(5, 0x16).Bg(Img.A, 16, 32);
        first.ConvertAndPublish(engine, 1);

        host.OnUpdate = () => engine.SetColorIndex(5, 0x30, current: true);
        Batch second = new();
        second.Palette(5, 0x16).Bg(Img.A, 16, 32);
        ConversionFrame current = second.Convert(engine, 2);

        Assert.Equal(0x30, current.Palette[5]);
        Assert.Equal(0x30, engine.FrameColorIndex(5, current: true));

        // The published frame keeps the core's value; the next frame rebuilds from the core.
        Assert.Equal(0x16, engine.FrameColorIndex(5));
        engine.Publish();
        host.OnUpdate = null;
        Batch third = new();
        third.Palette(5, 0x16).Bg(Img.A, 16, 32);
        Assert.Equal(0x16, third.Convert(engine, 3).Palette[5]);
    }

    [Fact]
    public void REQ_E3D_641_the_draw_list_carries_the_placement_and_the_flags()
    {
        ConversionEngine engine = OneTileEngine();
        RecordingGeometryBuilder geometry = new();
        engine.Geometry = geometry;

        Batch batch = new();
        batch.Palette(1, 0x16).Bg(Img.A, 16, 32, palette: 1).Sprite(Img.A, 40, 40, hidden: true);
        ConversionFrame frame = batch.Convert(engine, 3);

        FrameDrawList list = engine.BuildDrawList(frame);

        Assert.Equal(2, list.Tiles.Count);
        Assert.Equal(3, list.FrameCounter);
        Assert.Equal(3f / 60f, list.Time, 5);
        Assert.Equal(new Rgb24(0xFC, 0x38, 0x00), list.Palette[1]);

        DrawTile background = list.Tiles[0];
        Assert.True(background.Instance.Background);
        Assert.False(background.Sprite);
        Assert.Equal(SceneContainer.Game, background.Container);
        Assert.Equal(MaterialClass.Opaque, background.Material);
        Assert.Equal(1, background.PaletteTable);
        Assert.Equal(TextureFormat.Indexed, background.TextureFormat);
        Assert.True(background.CastShadow);
        Assert.True(background.ReceiveShadow);
        Assert.Null(background.Deform);
        Assert.NotNull(background.Mesh);

        // The tile centre of SPEC-04B REQ-E3D-405 and the transform of REQ-E3D-406.
        Assert.Equal(new Vec3(20f, 212f, 1.5f), background.TileCenter);
        Assert.Equal(background.TileCenter, background.Transform.TransformPoint(Vec3.Zero));

        DrawTile sprite = list.Tiles[1];
        Assert.True(sprite.Sprite);
        Assert.True(sprite.BehindBackground);
    }

    [Fact]
    public void REQ_E3D_642_material_classes_and_shadow_rules()
    {
        ConversionEngine engine = OneTileEngine();
        ConversionFrame frame = OneTileFrame(engine);
        FrameInstance instance = frame.Instances[0];

        instance.Alpha = 0.5f;
        Assert.Equal(MaterialClass.Transparent, engine.BuildDrawList(frame).Tiles[0].Material);

        instance.Style = RenderStyle.ShadowOnly;
        DrawTile shadowOnly = engine.BuildDrawList(frame).Tiles[0];
        Assert.Equal(MaterialClass.ShadowOnly, shadowOnly.Material);
        Assert.True(shadowOnly.CastShadow);

        instance.Style = RenderStyle.Normal;
        instance.Alpha = 1f;
        instance.Layer = ZLayer.Ui;
        DrawTile ui = engine.BuildDrawList(frame).Tiles[0];
        Assert.Equal(MaterialClass.Interface, ui.Material);
        Assert.Equal(SceneContainer.Interface, ui.Container);
        Assert.False(ui.CastShadow);
        Assert.False(ui.ReceiveShadow);

        // A transparent instance casts no shadow (REQ-E3D-567).
        instance.Layer = ZLayer.L1;
        instance.Alpha = 0.25f;
        Assert.False(engine.BuildDrawList(frame).Tiles[0].CastShadow);

        // !cast takes the shadow away, !recv the reception.
        instance.Alpha = 1f;
        instance.TagString = "!cast";
        Assert.False(engine.BuildDrawList(frame).Tiles[0].CastShadow);
        instance.TagString = "!recv";
        Assert.False(engine.BuildDrawList(frame).Tiles[0].ReceiveShadow);
    }

    [Fact]
    public void REQ_E3D_567_layer_permissions_decide_the_shadows()
    {
        ConversionEngine engine = OneTileEngine(layer: ProfileLayer.L2);
        ConversionFrame frame = OneTileFrame(engine);

        DrawTile tile = engine.BuildDrawList(frame).Tiles[0];
        Assert.False(tile.CastShadow);
        Assert.True(tile.ReceiveShadow);

        engine.Settings.Shadow = false;
        DrawTile none = engine.BuildDrawList(frame).Tiles[0];
        Assert.False(none.CastShadow);
        Assert.False(none.ReceiveShadow);
    }
}
