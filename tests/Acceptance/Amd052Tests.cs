using Silk.NET.OpenGL;
using Zax.App;
using Zax.App.Persistence;
using Zax.Profiles;
using Zax.Render;
using Zax.Settings;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-052 — frame pacing, GPU frame queue and anti-aliasing default. The acceptance criteria
/// that need an OpenGL 3.3 core context or the persisted settings: the fence criterion and the
/// latency bench of REQ-LOOP-077, and the fresh-installation default of REQ-PERS-089. The
/// cadence criteria of REQ-LOOP-074 and REQ-LOOP-075 are driven headless in
/// <c>Zax.App.Tests</c>, and vector T5 is in <c>LoopTests</c>.
/// </summary>
public sealed unsafe class Amd052Tests
{
    /// <summary><c>GL_SYNC_FLUSH_COMMANDS_BIT</c>, so the probe cannot wait on an unflushed queue.</summary>
    private const uint FlushCommandsBit = 0x0000_0001u;

    // ------------------------------------------------- REQ-LOOP-077, the fence criterion

    /// <summary>
    /// REQ-LOOP-077, second criterion (GL 3.3 core, REQ-SCN-056): a synchronisation object
    /// created after the last command of frame <i>n</i> and queried with
    /// <c>glClientWaitSync</c> at a zero timeout is signalled <b>before</b> the first command of
    /// frame <i>n + 1</i> when <c>video.max_queued_frames</c> = 1; at 2 that is not required,
    /// and the frame is allowed to queue up instead.
    /// </summary>
    /// <remarks>
    /// The probe is placed before the queue's own fence, so that a signalled queue fence implies
    /// a signalled probe: synchronisation objects cover the commands issued before them, and
    /// nothing is issued between the two.
    /// </remarks>
    [GlFact] // REQ-LOOP-077, REQ-SCN-056
    public void The_previous_frame_is_finished_before_the_next_one_starts_at_depth_one()
    {
        using GlTarget target = GlTarget.Create(320, 240);
        using SceneRenderer renderer = new(target.Gl);
        GL gl = target.Gl;

        foreach (int depth in new[] { 1, 2 })
        {
            using GpuFrameQueue queue = new(gl);
            SceneState scene = Edges();

            // Frame n.
            queue.BeginFrame(depth);
            renderer.Render(target.Target, scene);
            nint probe = Fence(gl);
            queue.EndFrame();

            long waits = queue.Waits;

            // Frame n + 1: the bound is held here, before its first command.
            queue.BeginFrame(depth);
            GLEnum state = Poll(gl, probe);

            if (depth == 1)
            {
                Assert.True(queue.Waits > waits, "depth 1 did not wait for the previous frame.");
                Assert.Equal(GLEnum.AlreadySignaled, state);
            }
            else
            {
                Assert.Equal(waits, queue.Waits);
            }

            renderer.Render(target.Target, scene);
            queue.EndFrame();
            queue.Flush();
            gl.DeleteSync(probe);
        }
    }

    /// <summary>
    /// REQ-LOOP-077, third criterion: the latency between the end of the emulation of a frame —
    /// the moment its rendering starts — and its presentation is shorter at 1 than at 2. The
    /// median over the campaign is what is compared; point 52-N4 of the second R2 review notes
    /// that the criterion names no minimum gap, so the assertion is the one the amendment
    /// writes, with the two medians printed.
    /// </summary>
    /// <remarks>
    /// The bench has to be <b>GPU-bound</b> for the comparison to mean anything: a frame queue
    /// only exists when the GPU is behind the submission, and with a scene the driver swallows in
    /// a few microseconds the two depths measure the cost of the wait itself rather than the age
    /// of the image. Hence the 1920 × 1080 target, the full screen of blocks and MSAA 8× of
    /// <see cref="Loaded"/> — the shape of the bench of REQ-ACC-030 the criterion names.
    /// On a light scene the comparison inverts, which is what Q-087 point 2 records.
    /// </remarks>
    // Q-087
    [GlFact] // REQ-LOOP-077, REQ-ACC-030
    public void The_latency_is_shorter_with_a_single_queued_frame()
    {
        using GlTarget target = GlTarget.Create(1920, 1080);
        using SceneRenderer renderer = new(target.Gl);

        double deep = MedianLatency(target, renderer, depth: 2);
        double shallow = MedianLatency(target, renderer, depth: 1);

        Assert.True(
            shallow < deep,
            $"the latency is {shallow * 1000:F2} ms at depth 1 and {deep * 1000:F2} ms at depth 2.");
    }

    /// <summary>
    /// REQ-LOOP-077, last criterion, rendering side: a queue depth of 1 does not keep the
    /// renderer from producing its frames — the cadence part of the criterion is checked
    /// headless in <c>Zax.App.Tests</c>, where a clock can be driven.
    /// </summary>
    [GlFact] // REQ-LOOP-077
    public void Every_frame_is_still_rendered_with_a_single_queued_frame()
    {
        using GlTarget target = GlTarget.Create(320, 240);
        using SceneRenderer renderer = new(target.Gl);
        using GpuFrameQueue queue = new(target.Gl);

        const int frames = 30;
        for (int frame = 0; frame < frames; frame++)
        {
            queue.BeginFrame(1);
            renderer.Render(target.Target, Edges());
            queue.EndFrame();
            Assert.InRange(queue.Outstanding, 0, 1);
        }

        queue.Flush();

        // One wait per frame but the first, which had nothing in front of it.
        Assert.Equal(frames, queue.Waits);
        Assert.Equal(0, queue.Outstanding);
    }

    // --------------------------------------------------------------- REQ-PERS-089

    /// <summary>
    /// REQ-PERS-089, criterion: a blank user root gives <c>video.anti_alias</c> = 1, and the
    /// renderer is granted the four samples AMD-032 §2 maps that value to — the number
    /// <c>Zax.Harness render</c> reports with no option, its default being the same one. A
    /// <c>settings.json</c> carrying <c>"video.anti_alias": 0</c> keeps the 0.
    /// </summary>
    [GlFact] // REQ-PERS-089, REQ-PERS-020, REQ-PERS-023
    public void A_fresh_installation_renders_with_four_samples()
    {
        string root = Path.Combine(Path.GetTempPath(), $"zax-amd052-{Guid.NewGuid():N}");
        Directory.CreateDirectory(root);
        try
        {
            // A blank user root: no settings file at all (REQ-PERS-075).
            UserDataPaths paths = UserDataPaths.Resolve(root);
            AppSettings fresh = new();
            SettingsStore store = new(paths);
            store.Load(fresh, DateTimeOffset.UnixEpoch);

            Assert.False(File.Exists(paths.SettingsFile));
            Assert.Equal(1, fresh.AntiAlias);

            using GlTarget target = GlTarget.Create(320, 240);
            using SceneRenderer renderer = new(target.Gl);

            SceneState scene = Edges();
            scene.Settings.AntiAliasSamples = SceneConstants.AntiAliasSamples(fresh.AntiAlias);
            target.Render(renderer, scene);

            Assert.Equal(4, renderer.Statistics.AntiAliasSamples);

            // An existing file keeps its value (REQ-PERS-020, REQ-PERS-023).
            File.WriteAllText(
                paths.SettingsFile,
                """
                { "schemaVersion": 1, "values": { "video.anti_alias": 0 } }
                """);

            AppSettings kept = new();
            SettingsStore reader = new(paths);
            reader.Load(kept, DateTimeOffset.UnixEpoch);

            Assert.Null(reader.Message);
            Assert.Equal(0, kept.AntiAlias);
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
    }

    /// <summary>
    /// REQ-PERS-089: the divergence is the <b>default</b> only. The domain, the labels, the
    /// profile field 32 and the Lua path of the row are untouched, and so is the libretro entry
    /// of §4.6 apart from its default (point 52-N5 of the second R2 review).
    /// </summary>
    [Fact] // REQ-PERS-089, REQ-PERS-017
    public void Only_the_default_of_the_anti_aliasing_row_moves()
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(SettingKeys.VideoAntiAlias);

        Assert.Equal(1, descriptor.DefaultValue.AsInt);
        Assert.Equal("AntiAlias", descriptor.LuaPath);
        Assert.Equal("32", descriptor.ProfileField);
        Assert.Equal(
            ["None", "FFAA", "SMAA"],
            descriptor.Domain.Options!.Select(static option => option.Label));
        Assert.Equal(
            ["None", "FFAA", "SMAA"],
            descriptor.LibretroValues!.Select(static option => option.Label));
    }

    // ------------------------------------------------------------------- helpers

    /// <summary>
    /// The median latency the queue reports over a campaign of frames at
    /// <paramref name="depth"/>, in seconds.
    /// </summary>
    private static double MedianLatency(GlTarget target, SceneRenderer renderer, int depth)
    {
        using GpuFrameQueue queue = new(target.Gl);
        SceneState scene = Loaded();

        List<double> samples = [];
        long waits = 0;
        for (int frame = 0; frame < 120; frame++)
        {
            queue.BeginFrame(depth);
            if (queue.Waits > waits)
            {
                waits = queue.Waits;
                samples.Add(queue.LastLatencySeconds);
            }

            renderer.Render(target.Target, scene);
            queue.EndFrame();
        }

        queue.Flush();

        Assert.NotEmpty(samples);
        samples.Sort();
        return samples[samples.Count / 2];
    }

    private static nint Fence(GL gl)
    {
        nint sync = (nint)(void*)gl.FenceSync(SyncCondition.SyncGpuCommandsComplete, 0u);
        Assert.NotEqual(0, sync);
        return sync;
    }

    /// <summary>Queries <paramref name="sync"/> at a zero timeout, as the criterion asks.</summary>
    private static GLEnum Poll(GL gl, nint sync) => gl.ClientWaitSync(sync, FlushCommandsBit, 0UL);

    /// <summary>
    /// A frame with the whole screen covered by thick blocks and the anti-aliasing at its
    /// highest level: the GPU, not the submission, is what paces the bench, which is the only
    /// regime in which a queue of rendered frames exists at all.
    /// </summary>
    private static SceneState Loaded()
    {
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x21)
            .Palette(3, 0x30)
            .Pattern(SceneFixture.Solid(3), depth: 24, ProfileLayer.L1)
            .Pattern(SceneFixture.Solid(1), depth: 32, ProfileLayer.L2);

        // A chequerboard of the two patterns over the whole 32 × 30 screen, so the meshing
        // cannot merge it into a handful of quads.
        for (int row = 0; row < 30; row++)
        {
            for (int column = 0; column < 32; column++)
            {
                fixture.Rect(
                    SceneFixture.Solid(((row + column) % 2) == 0 ? 3 : 1),
                    column,
                    row,
                    1,
                    1);
            }
        }

        SceneState scene = fixture.Build();
        scene.Settings.AntiAliasSamples = SceneConstants.MaxAntiAliasSamples;
        return scene;
    }

    /// <summary>
    /// A white block over the background, seen from the default angles: all edge in screen
    /// space, so the anti-aliasing has something to work on.
    /// </summary>
    private static SceneState Edges()
        => new SceneFixture()
            .Background(0x0F)
            .Palette(3, 0x30)
            .Pattern(SceneFixture.Solid(3), depth: 24, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(3), column: 14, row: 12, columns: 4, rows: 4)
            .Build();
}
