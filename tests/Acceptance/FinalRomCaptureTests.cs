using System.Diagnostics;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;
using Xunit.Abstractions;

namespace Zax.Acceptance.Tests;

/// <summary>
/// The [ROM] halves that need a real render: the archived captures of REQ-ACC-020 and the
/// frame-time measurements of REQ-ACC-030 and REQ-SCN-064, at 1920 × 1080 with shadows on and
/// the default view. Captures go to <c>verify/round1/final/</c>.
/// </summary>
/// <remarks>
/// One hidden GL 3.3 core context at a time, like <see cref="SceneCaptureTests"/>.
/// </remarks>
[Collection("opengl-acceptance")]
public sealed class FinalRomCaptureTests(ITestOutputHelper output)
{
    /// <summary>Budget of REQ-ACC-030 and REQ-SCN-064: 60 images per second.</summary>
    private const double FrameBudget = 16.7;

    /// <summary>What one measured run of a game gave.</summary>
    private sealed record Timing(string Title, int Frames, double Mean, double P99, double Over);

    /// <summary>
    /// A game, its profile and its engine, driven frame by frame with the scripted input of
    /// REQ-ACC-020 and rendered into <paramref name="target"/>.
    /// </summary>
    private sealed class Session : IDisposable
    {
        private readonly NesCore _core;
        private readonly ConversionEngine _engine;
        private readonly GeometryBuilder _geometry;
        private readonly SceneState _scene;
        private readonly Random _random = new(9);
        private ushort _held;
        private int _frame;

        public Session(UserGame game)
        {
            _core = UserRoms.Load(game);
            _engine = new ConversionEngine();
            _geometry = _engine.UseGeometry();
            _engine.LoadProfile(UserRoms.Profile(game));
            _core.VideoCallback = (in FrameBatch batch) =>
            {
                _engine.Convert(batch, (int)_core.FrameCount);
                _engine.Publish();
            };

            _scene = new SceneState();
            _scene.Camera.SnapToSetpoints();
            _scene.GameRunning = true;
            _scene.Settings.Shadows = true;
            Title = game.Title;
        }

        public string Title { get; }

        public SceneState Scene => _scene;

        public int Shapes => _engine.PublishedFrame?.Shapes.Count ?? 0;

        public int DrawTiles => _scene.DrawList?.Tiles.Count ?? 0;

        /// <summary>One emulated frame, converted, its geometry committed and its scene ready.</summary>
        public void Advance()
        {
            _core.RunFrame(UserRoms.ScriptedInput(_frame++, _random, ref _held));
            _geometry.Commit();
            _scene.Clipping = _engine.Settings.Clipping;
            _scene.DrawList = _engine.BuildDrawList();
            _scene.NesBackgroundColor = _engine.PublishedFrame!.Palette.Color(0);
            _scene.EmulatedTime = _engine.PublishedFrame.Time;
        }

        public void Dispose() => _geometry.Clear();
    }

    /// <summary>
    /// REQ-ACC-020, the archived captures: each reference game the user owns is played into its
    /// game proper and rendered in the default view at 1920 × 1080, and the capture is kept in
    /// <c>verify/round1/final/</c>. A blank or one-colour image would not be « un rendu 3D
    /// cohérent avec la description de SPEC-06 ».
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [Trait("Requires", "UserRoms")]
    [CorpusAnyRomGlFact] // REQ-ACC-020
    public void Each_reference_game_is_captured_in_the_default_view()
    {
        UserGame[] games = [.. UserRoms.Present];
        Assert.NotEmpty(games);

        using GlTarget target = GlTarget.Create(1920, 1080);
        using SceneRenderer renderer = new(target.Gl);

        foreach (UserGame game in games)
        {
            using Session session = new(game);
            for (int frame = 0; frame < 1_500; frame++)
            {
                session.Advance();
            }

            // Tetris sits on its three menus for a while; the capture is taken once the game
            // proper is on screen, which is what « démarrer, se jouer » means.
            for (int frame = 0; frame < 6_000 && session.DrawTiles < 50; frame++)
            {
                session.Advance();
            }

            Shot shot = target.Render(renderer, session.Scene);
            int colours = shot.DistinctColors().Count;

            output.WriteLine(
                $"{game.Title}: {session.Shapes} shapes, {session.DrawTiles} draw tiles, "
                + $"{colours} distinct colours in the capture.");

            string file = game.Title.Replace('.', ' ').Replace(':', ' ').Trim().Replace(' ', '-');
            shot.SavePng(Path.Combine(FinalCaptures.Ready(), $"acc-020-{file.ToLowerInvariant()}.png"));

            Assert.True(colours > 8, $"{game.Title}: only {colours} colours in the capture.");
            Assert.True(session.DrawTiles > 0, $"{game.Title}: the draw list is empty.");
        }
    }

    /// <summary>
    /// REQ-ACC-030 and REQ-SCN-064: on the reference machine the whole per-frame work — core,
    /// conversion, geometry transfer and the 1920 × 1080 render with shadows — holds in 16.7 ms.
    /// Each reference game is measured over 30 seconds, and the heaviest of them over the five
    /// minutes REQ-ACC-030 asks for.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [Trait("Requires", "UserRoms")]
    [CorpusAnyRomGlFact] // REQ-ACC-030, REQ-SCN-064
    public void The_reference_games_hold_sixty_frames_per_second_in_1080p()
    {
        UserGame[] games = [.. UserRoms.Present];
        Assert.NotEmpty(games);

        using GlTarget target = GlTarget.Create(1920, 1080);
        using SceneRenderer renderer = new(target.Gl);

        List<Timing> timings = [];
        foreach (UserGame game in games)
        {
            timings.Add(Measure(target, renderer, game, 1_800));
        }

        foreach (Timing timing in timings.OrderByDescending(t => t.P99))
        {
            output.WriteLine(
                $"{timing.Title}: {timing.Frames} frames, mean {timing.Mean:F2} ms, "
                + $"p99 {timing.P99:F2} ms, {timing.Over:P3} over {FrameBudget} ms.");
        }

        // REQ-ACC-030: less than 1 % of the frames over 16.7 ms, on every reference game.
        foreach (Timing timing in timings)
        {
            Assert.True(timing.Over < 0.01, $"{timing.Title}: {timing.Over:P2} of frames over budget.");
        }

        // REQ-SCN-064 and the five-minute sequence of REQ-ACC-030, on the heaviest of them.
        UserGame heaviest = games.Single(g => g.Title == timings.OrderByDescending(t => t.P99).First().Title);
        Timing full = Measure(target, renderer, heaviest, 18_000);
        output.WriteLine(
            $"five minutes of {full.Title}: mean {full.Mean:F2} ms, p99 {full.P99:F2} ms, "
            + $"{full.Over:P3} over {FrameBudget} ms.");

        Assert.True(full.Over < 0.01, $"{full.Title}: {full.Over:P2} of frames over budget.");
        Assert.True(full.P99 <= FrameBudget, $"{full.Title}: 99th centile {full.P99:F2} ms.");
    }

    private Timing Measure(GlTarget target, SceneRenderer renderer, UserGame game, int frames)
    {
        using Session session = new(game);

        // The loading of the profile's geometry is not part of the per-frame budget
        // (REQ-E3D-360, « hors construction initiale des meshes »), and neither is the first
        // render, which uploads the meshes and compiles nothing else.
        for (int frame = 0; frame < 600; frame++)
        {
            session.Advance();
        }

        target.Draw(renderer, session.Scene);

        double[] samples = new double[frames];
        for (int frame = 0; frame < frames; frame++)
        {
            long start = Stopwatch.GetTimestamp();
            session.Advance();
            target.Draw(renderer, session.Scene);
            samples[frame] = Stopwatch.GetElapsedTime(start).TotalMilliseconds;
        }

        double over = samples.Count(sample => sample > FrameBudget) / (double)frames;
        double mean = samples.Average();
        Array.Sort(samples);
        return new Timing(game.Title, frames, mean, samples[(int)(frames * 0.99)], over);
    }
}
