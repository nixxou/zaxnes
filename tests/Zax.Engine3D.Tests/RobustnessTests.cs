using Zax.Profiles;

namespace Zax.Engine3D.Tests;

/// <summary>
/// AMD-051 §2, geometry half: a texture that cannot be allocated (REQ-E3D-644) and an object
/// that is not ready yet (REQ-E3D-646). Both replace an outright failure by a degradation that
/// no frame and no exception ever notices.
/// </summary>
public sealed class RobustnessTests
{
    /// <summary>A 1 × 1 pattern carries an 8 × 8 texture, so 64 texels (REQ-E3D-426).</summary>
    private const int TexelsPerPattern = 64;

    /// <summary>
    /// REQ-E3D-644, acceptance criterion: with the total texture budget cut to C texels, a
    /// profile whose objects ask for more than C renders the ones that fit, logs one error per
    /// object that does not, and produces the next frame. No exception, and the emulation goes
    /// on.
    /// </summary>
    [Fact] // REQ-E3D-644
    public void REQ_E3D_644_a_texture_that_does_not_fit_leaves_its_tiles_out_and_logs_once()
    {
        List<string> log = [];
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.Diagnostics = log.Add;

        // Two of the four 3D objects fit in the budget; the profile asks for 256 texels.
        geometry.TextureTexelBudget = 2 * TexelsPerPattern;
        engine.LoadProfile(FourPatterns());

        Assert.Equal(4, geometry.ObjectCount);
        Assert.Equal(2, geometry.UnhousedObjects);
        Assert.Equal(2 * TexelsPerPattern, geometry.HousedTexels);

        // One error per object that was refused, and not one more.
        Assert.Equal(2, log.Count);
        Assert.All(log, line => Assert.Contains("REQ-E3D-644", line, StringComparison.Ordinal));

        // The objects that fit are drawn; the others are not, and the frame is produced.
        Batch first = FourTiles();
        Assert.Equal(2, engine.BuildDrawList(first.Convert(engine, 1)).Tiles.Count);

        // The next frame is produced too, and nothing is logged a second time.
        Batch second = FourTiles();
        Assert.Equal(2, engine.BuildDrawList(second.Convert(engine, 2)).Tiles.Count);
        Assert.Equal(2, log.Count);
    }

    /// <summary>
    /// REQ-E3D-644 read with REQ-E3D-603: the texels of a released pattern come back, and the
    /// object that had been refused is drawn at the first frame that follows — never forgotten
    /// (REQ-E3D-646).
    /// </summary>
    [Fact] // REQ-E3D-644, REQ-E3D-646, REQ-E3D-603
    public void REQ_E3D_644_room_freed_by_a_released_pattern_houses_a_refused_object()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.TextureTexelBudget = 3 * TexelsPerPattern;
        engine.LoadProfile(FourPatterns());

        Assert.Equal(1, geometry.UnhousedObjects);

        PatternInstance refused = engine.Patterns.Permanent[3].Instances[0];
        Assert.False(geometry.IsReady(refused));

        // The first pattern goes away: its 64 texels are free again.
        geometry.Release(engine.Patterns.Permanent[0]);
        Assert.Equal(2 * TexelsPerPattern, geometry.HousedTexels);

        // REQ-E3D-646: the object is drawn at the first frame that follows, with no rebuild.
        int builds = geometry.BuildCount;
        Batch batch = new();
        batch.Bg(Distinct(3), 0, 0);
        Assert.Single(engine.BuildDrawList(batch.Convert(engine, 1)).Tiles);
        Assert.True(geometry.IsReady(refused));
        Assert.Equal(0, geometry.UnhousedObjects);
        Assert.Equal(builds, geometry.BuildCount);
    }

    /// <summary>
    /// REQ-E3D-644: at most 100 lines per game, then a summary line at most once per minute of
    /// play giving the cumulative counts. Patterns are created during play too (REQ-E3D-008,
    /// REQ-E3D-453), so without the cap a pathological ROM would fill the log.
    /// </summary>
    [Fact] // REQ-E3D-644
    public void REQ_E3D_644_the_log_is_capped_at_a_hundred_lines_and_then_summarised()
    {
        List<string> log = [];
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.Diagnostics = log.Add;

        // Nothing fits at all: 120 objects, 120 refusals, 100 lines.
        geometry.TextureTexelBudget = 0;
        engine.LoadProfile(ManyPatterns(120));

        Assert.Equal(120, geometry.UnhousedObjects);
        Assert.Equal(GeometryBuilder.DiagnosticLineBudget, log.Count);

        // A minute of play has to pass before the summary appears, and only one line comes out.
        Batch batch = new();
        for (int frame = 0; frame < GeometryBuilder.SummaryFrames - 1; frame++)
        {
            batch.Convert(engine, frame + 1);
        }

        Assert.Equal(GeometryBuilder.DiagnosticLineBudget, log.Count);

        batch.Convert(engine, GeometryBuilder.SummaryFrames);
        Assert.Equal(GeometryBuilder.DiagnosticLineBudget + 1, log.Count);
        Assert.Contains("have been degraded since this game started", log[^1], StringComparison.Ordinal);

        // And not a second one before the next minute.
        for (int frame = 0; frame < GeometryBuilder.SummaryFrames - 1; frame++)
        {
            batch.Convert(engine, GeometryBuilder.SummaryFrames + frame + 1);
        }

        Assert.Equal(GeometryBuilder.DiagnosticLineBudget + 1, log.Count);
        batch.Convert(engine, (2 * GeometryBuilder.SummaryFrames) + 1);
        Assert.Equal(GeometryBuilder.DiagnosticLineBudget + 2, log.Count);
    }

    /// <summary>
    /// REQ-E3D-646, first half of the acceptance criterion: an object whose construction is
    /// held back by 10 frames is drawn at the first rendered frame that follows the end of that
    /// construction, and no diagnostic is reported. The delay is a test case, not a behaviour to
    /// aim for — REQ-E3D-601 recommends meshes ready at most 2 emulated frames after a pattern
    /// is created.
    /// </summary>
    [Fact] // REQ-E3D-646, REQ-E3D-601
    public void REQ_E3D_646_an_object_delayed_by_ten_frames_is_drawn_and_never_reported()
    {
        List<string> log = [];
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.Diagnostics = log.Add;
        geometry.BuildDelayFrames = 10;
        engine.LoadProfile(OnePattern());

        PatternInstance reference = engine.Patterns.Permanent[0].Instances[0];
        Assert.False(geometry.IsReady(reference));

        // REQ-E3D-601: the shape is recognised and seen by scripts all along, just not drawn.
        for (int frame = 1; frame <= 10; frame++)
        {
            Batch batch = new();
            batch.Bg(Img.A, 0, 0);
            ConversionFrame converted = batch.Convert(engine, frame);
            Assert.Single(converted.Instances);

            // The construction is due on the tenth frame, so that is the first frame drawn.
            Assert.Equal(frame == 10 ? 1 : 0, engine.BuildDrawList(converted).Tiles.Count);
        }

        Assert.True(geometry.IsReady(reference));
        Assert.Empty(log);
    }

    /// <summary>
    /// REQ-E3D-646, second half: an object whose construction is held back by 700 frames
    /// produces exactly one diagnostic line, at frame 600 ± 1.
    /// </summary>
    [Fact] // REQ-E3D-646
    public void REQ_E3D_646_an_object_waiting_six_hundred_frames_is_reported_exactly_once()
    {
        List<string> log = [];
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.Diagnostics = log.Add;
        geometry.BuildDelayFrames = 700;
        engine.LoadProfile(OnePattern());

        int reported = 0;
        Batch batch = new();
        for (int frame = 1; frame <= 700; frame++)
        {
            batch.Convert(engine, frame);
            if (log.Count == 1 && reported == 0)
            {
                reported = frame;
            }
        }

        Assert.Single(log);
        Assert.InRange(reported, GeometryBuilder.WaitingDiagnosticFrames - 1, GeometryBuilder.WaitingDiagnosticFrames + 1);
        Assert.Contains("REQ-E3D-646", log[0], StringComparison.Ordinal);
        Assert.Equal(1, geometry.WaitingDiagnosticCount);

        // The object is drawable once its construction has run, and is not reported again.
        PatternInstance reference = engine.Patterns.Permanent[0].Instances[0];
        Assert.True(geometry.IsReady(reference));
        batch.Convert(engine, 701);
        Assert.Single(log);
    }

    /// <summary>
    /// REQ-E3D-644 does not reopen REQ-E3D-603: with the budget left alone nothing is refused,
    /// nothing is logged and the frame of 1 500 shapes over 3 000 tiles is drawn whole.
    /// </summary>
    [Fact] // REQ-E3D-644, REQ-E3D-603, REQ-E3D-564
    public void REQ_E3D_603_the_default_budget_caps_nothing()
    {
        List<string> log = [];
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.Diagnostics = log.Add;

        // REQ-E3D-564 asks for at least 64 × 512 × 512 texels; the default is unbounded.
        Assert.True(geometry.TextureTexelBudget >= 64L * 512 * 512);

        engine.LoadProfile(new ProfileBuilder().Pattern(2, 1, [Img.A, Img.B], depth: 2).Build());
        EnginePattern pattern = engine.Patterns.Permanent[0];

        Batch empty = new();
        empty.Convert(engine);
        for (int n = 0; n < 1500; n++)
        {
            engine.AddNewShape(pattern, new Vec3(n % 200, n / 200, 0f), palette: 4);
        }

        engine.CalculatePositions(engine.ConvertingFrame);
        Assert.Equal(3000, engine.BuildDrawList(engine.ConvertingFrame).Tiles.Count);
        Assert.Equal(0, geometry.UnhousedObjects);
        Assert.Empty(log);
    }

    /// <summary>
    /// A rebuild of the same object does not count twice against the budget: the texels of the
    /// previous build come back before the new texture asks for room.
    /// </summary>
    [Fact] // REQ-E3D-644, REQ-E3D-564
    public void REQ_E3D_644_a_rebuild_does_not_count_its_texels_twice()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.TextureTexelBudget = TexelsPerPattern;
        engine.LoadProfile(OnePattern());

        Assert.Equal(TexelsPerPattern, geometry.HousedTexels);
        Assert.Equal(0, geometry.UnhousedObjects);

        engine.SetRenderAlgorithm(RenderAlgorithm.Marching);
        Assert.Equal(TexelsPerPattern, geometry.HousedTexels);
        Assert.Equal(0, geometry.UnhousedObjects);
    }

    /// <summary>
    /// A full tile image whose identity is <paramref name="n"/>: the low plane is all ones and
    /// the high plane carries <c>n + 1</c>, so every value gives another image (SPEC-04 §2) and
    /// the count is not limited to the 64 shorthands of <see cref="Img"/>.
    /// </summary>
    private static TileImageKey Distinct(int n) => new(ulong.MaxValue, (ulong)(n + 1), false, false);

    private static GameProfile OnePattern()
        => new ProfileBuilder().Pattern(1, 1, [Img.A], depth: 2).Build();

    private static GameProfile FourPatterns() => ManyPatterns(4);

    private static GameProfile ManyPatterns(int count)
    {
        ProfileBuilder builder = new();
        for (int n = 0; n < count; n++)
        {
            builder.Pattern(1, 1, [Distinct(n)], depth: 2);
        }

        return builder.Build();
    }

    private static Batch FourTiles()
    {
        Batch batch = new();
        for (int n = 0; n < 4; n++)
        {
            batch.Bg(Distinct(n), 8 * n, 0);
        }

        return batch;
    }
}
