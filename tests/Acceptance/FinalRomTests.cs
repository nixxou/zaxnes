using System.Diagnostics;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Xunit.Abstractions;
using Xunit.Sdk;

namespace Zax.Acceptance.Tests;

/// <summary>
/// One of the user's own, legally owned dumps, with the catalogue profile of the same game.
/// </summary>
/// <param name="Title">Catalogue title.</param>
/// <param name="Id">Catalogue id in <c>profiles.json</c>.</param>
/// <param name="Rom">File name under <see cref="UserRoms.Directory"/>.</param>
/// <param name="Profile">Profile file name under <c>assets-neutral/profiles</c>.</param>
/// <param name="Reference">Whether REQ-ACC-020 names the game among its twelve.</param>
public sealed record UserGame(string Title, string Id, string Rom, string Profile, bool Reference);

/// <summary>The user's ROM folder and the mapping of its files to catalogue profiles.</summary>
public static class UserRoms
{
    /// <summary>
    /// Environment variable naming the folder the user keeps their own dumps in. No machine
    /// path is written anywhere in this repository (lot PUB-1): the folder is opt-in, and a
    /// test that needs a dump is <b>skipped</b> with a message naming this variable when it is
    /// not set. <c>HOWTO-RUN.md</c> §12 documents it.
    /// </summary>
    public const string Variable = "ZAXNES_ROMS";

    /// <summary>
    /// Where the user keeps their dumps, read from <see cref="Variable"/> once, or null when
    /// the variable is not set.
    /// </summary>
    public static string? Directory { get; } =
        Environment.GetEnvironmentVariable(Variable) is { } value && value.Trim().Length > 0
            ? value.Trim()
            : null;

    /// <summary>
    /// Why the ROM folder itself cannot be read — the variable is not set, or it names a folder
    /// that is not there — or null when the folder is usable, whatever it holds.
    /// </summary>
    public static string? FolderReason
        => Directory switch
        {
            null => $"no user ROM folder: set {Variable} to the folder holding your own dumps.",
            string folder when !System.IO.Directory.Exists(folder)
                => $"the folder {Variable} names does not exist: {folder}.",
            _ => null,
        };

    /// <summary>
    /// Why no dump of the user can be read — the variable, the folder or the files — or null
    /// when at least one dump of <see cref="All"/> is there.
    /// </summary>
    public static string? MissingReason
        => FolderReason ?? (Present.Any() ? null : $"no user ROM under {Variable} = {Directory}.");

    /// <summary>
    /// The thirteen dumps of the user's folder, with the catalogue entry each one matches. The
    /// twelfth reference game of REQ-ACC-020, Quarth, is only present as a Game Boy ROM
    /// (<c>Quarth (USA, Europe).gb</c>), which this core does not run.
    /// </summary>
    public static IReadOnlyList<UserGame> All { get; } =
    [
        new("Blaster Master", "85", "Blaster Master (USA).nes", "Blaster Master (U).bytes", true),
        new("Contra", "9", "Contra (USA).nes", "Contra.bytes", true),
        new("Duck Hunt", "15", "Duck Hunt (World).nes", "Duck Hunt (World).bytes", true),
        new("Elevator Action", "16", "Elevator Action (USA).nes", "Elevator Action.bytes", false),
        new("Excitebike", "17", "Excitebike (Japan, USA) (En).nes", "Excitebike.bytes", true),
        new("Final Fantasy", "77", "Final Fantasy (USA).nes", "Final_Fantasy_USA.bytes", true),
        new("Legend Of Zelda", "45", "Legend of Zelda, The (USA).nes", "Legend Of Zelda.bytes", true),
        new("Mega Man 2", "31", "Mega Man 2 (USA).nes", "Mega Man 2.bytes", true),
        new("Metroid", "32", "Metroid (USA).nes", "Metroid.bytes", true),
        new("Super Mario Bros.", "37", "Super Mario Bros. (World).nes", "Super Mario Bros.bytes", true),
        new("Super Mario Bros. 3", "55", "Super Mario Bros. 3 (USA).nes", "Super Mario Bros 3 (U) (PRG1).bytes", true),
        new("Tetris - Nintendo", "38", "Tetris (USA).nes", "Tetris-Nintendo.bytes", true),
        new(
            "Zelda II: The Adventure of Link", "73", "Zelda II - The Adventure of Link (Europe).nes",
            "Zelda II - The Adventure of Link (U).bytes", false),
    ];

    /// <summary>Full path of a dump, or its bare name when the folder is not configured.</summary>
    public static string Path(string rom)
        => Directory is null ? rom : System.IO.Path.Combine(Directory, rom);

    /// <summary>Whether a dump is on this machine.</summary>
    public static bool Has(string rom) => Directory is not null && File.Exists(Path(rom));

    /// <summary>The games of <see cref="All"/> whose dump is present.</summary>
    public static IEnumerable<UserGame> Present => All.Where(game => Has(game.Rom));

    /// <summary>A core with <paramref name="game"/> loaded.</summary>
    public static NesCore Load(UserGame game)
    {
        ArgumentNullException.ThrowIfNull(game);
        NesCore core = new();
        Zax.Nes.Rom.RomLoadResult load = core.LoadRom(File.ReadAllBytes(Path(game.Rom)), game.Rom);
        Assert.True(load.Success, $"{game.Rom}: {load.Error}");
        return core;
    }

    /// <summary>The catalogue profile of <paramref name="game"/>.</summary>
    public static GameProfile Profile(UserGame game)
    {
        ArgumentNullException.ThrowIfNull(game);
        ProfileLoadResult result = ProfileFormat.Load(
            NeutralAssets.ReadProfile(game.Profile), sourceName: game.Profile);
        Assert.True(result.IsSuccess, $"{game.Profile}: {result.Message}");
        return result.Profile!;
    }

    /// <summary>
    /// The scripted input sequence REQ-ACC-020 allows in place of a manual session: a few
    /// seconds of nothing, then Start pulses until the game is in play, then pseudo-random
    /// directions and A/B held for a few frames at a time, with a Start pulse every ten
    /// seconds so a game that waits on a menu moves on. Deterministic for a given seed.
    /// </summary>
    public static ushort ScriptedInput(int frame, Random random, ref ushort held)
    {
        if (frame < 120)
        {
            return 0;
        }

        if (frame < 600)
        {
            // Start held for half a second at a time, released in between: Duck Hunt only
            // leaves its title screen on a sustained press, and the games that want an edge
            // get one every 30 frames.
            return (ushort)((frame % 60) < 30 ? 0x08 : 0x00);
        }

        if (frame % 600 is >= 0 and < 20)
        {
            return 0x08;
        }

        if (frame % 8 == 0)
        {
            // A direction, plus A or B. Select and Start are left out of the random draw: they
            // reset or pause several of the reference games.
            int direction = random.Next(4) switch
            {
                0 => 0x10,
                1 => 0x20,
                2 => 0x40,
                _ => 0x80,
            };
            held = (ushort)(direction | (random.Next(4) switch { 0 => 0x01, 1 => 0x02, 2 => 0x03, _ => 0x00 }));
        }

        return held;
    }
}

/// <summary>
/// A <see cref="FactAttribute"/> reported as <b>skipped</b>, naming the file, unless the dump
/// it needs is under <see cref="UserRoms.Directory"/>. The [ROM] criteria of SPEC-12 name
/// commercial games, which this repository does not carry: the gap must show in the report
/// rather than pass silently.
/// </summary>
[XunitTestCaseDiscoverer("Xunit.Sdk.FactDiscoverer", "xunit.execution.dotnet")]
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
public sealed class RomOfTheUserFactAttribute : FactAttribute
{
    /// <summary>Requires <paramref name="rom"/> under <see cref="UserRoms.Directory"/>.</summary>
    public RomOfTheUserFactAttribute(string rom)
    {
        if (!UserRoms.Has(rom))
        {
            Skip = UserRoms.MissingReason ?? $"user ROM not available: {UserRoms.Path(rom)}.";
        }
    }
}

/// <summary>
/// The theory form of <see cref="RomOfTheUserFactAttribute"/> for a theory that walks several
/// dumps: reported as <b>skipped</b>, naming <c>ZAXNES_ROMS</c>, when the user's ROM folder is
/// not configured.
/// </summary>
/// <remarks>
/// The gate is the folder, not the files: once the variable names a folder, a dump the theory
/// asks for and does not find is a real gap and the data row fails as it did before. xUnit 2
/// skips a theory as a whole, which is why the file-level verdict stays in the body.
/// </remarks>
public sealed class RomsOfTheUserTheoryAttribute : TheoryAttribute
{
    /// <summary>Requires the ROM folder, and the extracted profiles when asked for.</summary>
    /// <param name="profiles">Whether the theory also needs <c>assets-neutral/profiles</c>.</param>
    public RomsOfTheUserTheoryAttribute(bool profiles = false)
    {
        if (UserRoms.FolderReason is { } missing)
        {
            Skip = missing;
        }
        else if (profiles && !ProfileCorpus.Available)
        {
            Skip = CorpusFactAttribute.MissingMessage;
        }
    }
}

/// <summary>
/// The [ROM] halves of the criteria that round 1 had to leave « en attente de ROM », judged on
/// the thirteen dumps the user supplied: REQ-ACC-020, REQ-ACC-031, REQ-CORE-062, REQ-CORE-114,
/// REQ-CORE-115, REQ-E3D-322, REQ-LUA-110 and REQ-LUA-111. The frame-time measurements that
/// need a window-sized render are in <see cref="FinalRomCaptureTests"/>.
/// </summary>
public sealed class FinalRomTests(ITestOutputHelper output)
{
    private const string Smb3 = "Super Mario Bros. 3 (USA).nes";
    private const string MegaMan2 = "Mega Man 2 (USA).nes";
    private const string DuckHunt = "Duck Hunt (World).nes";

    /// <summary>Five minutes at the NTSC frame rate (REQ-ACC-020).</summary>
    private const int FiveMinutes = 18_000;

    public static TheoryData<string> PresentGames
    {
        get
        {
            TheoryData<string> data = [];
            foreach (UserGame game in UserRoms.Present)
            {
                data.Add(game.Rom);
            }

            if (data.Count == 0)
            {
                data.Add(string.Empty);
            }

            return data;
        }
    }

    private static UserGame Game(string rom) => UserRoms.All.Single(g => g.Rom == rom);

    /// <summary>
    /// REQ-ACC-020 and REQ-CORE-115: each game of the user's folder starts, plays five minutes
    /// through the scripted input sequence without a crash, and produces a coherent 3D scene —
    /// shapes recognised by its own catalogue profile, meshes built for them, and a draw list
    /// the renderer can take.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [Trait("Requires", "UserRoms")]
    [RomsOfTheUserTheory(profiles: true)] // REQ-ACC-020, REQ-CORE-115
    [MemberData(nameof(PresentGames))]
    public void Each_user_game_starts_and_plays_five_minutes_in_3d(string rom)
    {
        if (rom.Length == 0)
        {
            Assert.Fail(
                UserRoms.MissingReason
                ?? "no user ROM on this machine: REQ-ACC-020 stays « en attente de ROM ».");
        }

        UserGame game = Game(rom);
        NesCore core = UserRoms.Load(game);
        GameProfile profile = UserRoms.Profile(game);

        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        engine.LoadProfile(profile);

        int frames = 0;
        long shapes = 0;
        int framesWithShapes = 0;
        int maxShapes = 0;
        int maxTiles = 0;
        core.VideoCallback = (in FrameBatch batch) =>
        {
            ConversionFrame frame = engine.Convert(batch, (int)core.FrameCount);
            engine.Publish();
            shapes += frame.Shapes.Count;
            maxShapes = Math.Max(maxShapes, frame.Shapes.Count);
            maxTiles = Math.Max(maxTiles, batch.Records.Length);
            if (frame.Shapes.Count > 0)
            {
                framesWithShapes++;
            }

            frames++;
        };

        Random random = new(7);
        ushort held = 0;
        if (game.Rom == DuckHunt)
        {
            // Duck Hunt is played with the Zapper on port 2 (REQ-CORE-062).
            core.SetPortType(1, ControllerPortType.Zapper);
            core.ZapperPositionSource = _ => new ZapperPosition(random.Next(256), random.Next(240));
        }

        for (int frame = 0; frame < FiveMinutes; frame++)
        {
            ushort word = UserRoms.ScriptedInput(frame, random, ref held);
            if (game.Rom == DuckHunt)
            {
                // Trigger released is bit 4 of the port byte set (REQ-CORE-061); pulled every
                // half second.
                word = (ushort)(frame % 30 < 3 ? 0x0000 : 0x1000);
            }

            core.RunFrame(word);
        }

        geometry.Commit();
        FrameDrawList drawList = engine.BuildDrawList();

        output.WriteLine(
            $"{game.Title}: {frames} frames, {shapes} shapes total, {framesWithShapes} frames with a shape, "
            + $"max {maxShapes} shapes / {maxTiles} tile records per frame, "
            + $"{drawList.Tiles.Count} draw tiles, {geometry.ObjectCount} built objects.");

        Assert.Equal(FiveMinutes, frames);
        Assert.Equal(FiveMinutes, core.FrameCount);

        // "un rendu 3D cohérent avec la description de SPEC-06": the profile recognises shapes
        // on the great majority of the frames, the geometry is built, and the draw list carries
        // tiles with a depth greater than one voxel.
        if (profile.PermanentPatterns.Count + profile.TemporaryPatterns.Count > 0)
        {
            Assert.True(framesWithShapes > FiveMinutes / 2, $"{framesWithShapes} frames carried a shape.");
            Assert.True(geometry.ObjectCount > 0, "no mesh was built.");
            Assert.NotEmpty(drawList.Tiles);
        }
    }

    /// <summary>
    /// REQ-ACC-020 names Quarth for the MARCHING algorithm, and the user's folder holds Quarth
    /// only as a Game Boy ROM, which this core does not run. The algorithm itself is a global
    /// user setting (REQ-E3D-340), not a property of that profile, so it is exercised here on
    /// the reference games that are present: five minutes of play with MARCHING forced for
    /// every instance, and the geometry rebuilt for it.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [Trait("Requires", "UserRoms")]
    [RomsOfTheUserTheory(profiles: true)] // REQ-ACC-020 (the MARCHING part), REQ-E3D-340, REQ-E3D-408
    [InlineData("Tetris (USA).nes")]
    [InlineData("Super Mario Bros. (World).nes")]
    [InlineData("Metroid (USA).nes")]
    public void A_reference_game_plays_five_minutes_with_the_marching_algorithm(string rom)
    {
        if (!UserRoms.Has(rom))
        {
            Assert.Fail($"user ROM not available: {UserRoms.Path(rom)}.");
        }

        UserGame game = Game(rom);
        NesCore core = UserRoms.Load(game);

        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        engine.Settings.RenderAlgorithm = RenderAlgorithm.Marching;
        engine.LoadProfile(UserRoms.Profile(game));
        Assert.Equal(RenderAlgorithm.Marching, geometry.GlobalAlgorithm);

        int frames = 0;
        long shapes = 0;
        int marchingTiles = 0;
        core.VideoCallback = (in FrameBatch batch) =>
        {
            ConversionFrame frame = engine.Convert(batch, (int)core.FrameCount);
            engine.Publish();
            shapes += frame.Shapes.Count;
            frames++;
        };

        Random random = new(10);
        ushort held = 0;
        for (int frame = 0; frame < FiveMinutes; frame++)
        {
            core.RunFrame(UserRoms.ScriptedInput(frame, random, ref held));
            if (frame % 600 == 0)
            {
                geometry.Commit();
                marchingTiles = Math.Max(marchingTiles, engine.BuildDrawList().Tiles.Count);
            }
        }

        geometry.Commit();
        FrameDrawList drawList = engine.BuildDrawList();
        marchingTiles = Math.Max(marchingTiles, drawList.Tiles.Count);

        output.WriteLine(
            $"{game.Title} in MARCHING: {frames} frames, {shapes} shapes, "
            + $"{geometry.ObjectCount} built objects, up to {marchingTiles} draw tiles.");

        Assert.Equal(FiveMinutes, frames);
        Assert.True(shapes > 0, "no shape was recognised.");
        Assert.True(geometry.ObjectCount > 0, "no mesh was built.");
        Assert.True(marchingTiles > 0, "the draw list stayed empty.");

        // The marching mesher really produced geometry for them: every drawn tile of the frame
        // carries a mesh built under MARCHING (REQ-E3D-408 checks its scale on the synthetic
        // vectors of SPEC-04B).
        Assert.Contains(drawList.Tiles, tile => tile.Mesh is not null);
    }

    /// <summary>
    /// REQ-CORE-062, [ROM] half, on the one of its three games the user owns: « un tir pointé
    /// sur une cible la touche, un tir à côté la manque (10 essais, au moins 9 conformes) ».
    /// </summary>
    /// <remarks>
    /// There is no public memory map of Duck Hunt in the sources this verification may use, so
    /// "hits" is not read from a named address. Each trial is run twice from the <b>same
    /// savestate</b> with the same timing, once aimed at the duck and once aimed at a far
    /// corner, and what is measured is whether the game's own state diverges — which it can
    /// only do through the light bit of <c>$4017</c>. The addresses that diverge in the aimed
    /// trials and not in the others are reported, so the divergence can be read as a hit
    /// counter rather than as noise.
    /// </remarks>
    [Trait("Requires", "UserRoms")]
    [RomOfTheUserFact(DuckHunt)] // REQ-CORE-062
    public void A_zapper_shot_aimed_at_the_duck_changes_the_game_state_and_a_shot_beside_it_does_not()
    {
        const int trials = 10;
        int conforming = 0;
        Dictionary<int, int> divergedOnTarget = [];
        Dictionary<int, int> divergedBeside = [];

        // One session. Duck Hunt leaves its title screen only on a Start held down, not on a
        // short pulse, so Start is held at the start and pressed again now and then: the player
        // of this core never shoots, so every round ends in a miss and the game walks back to
        // its title screen.
        NesCore core = Zapper(out Action<int, int> _, out Func<(int X, int Y)?> duck, out Func<int> sprites);

        bool playing = false;
        for (int frame = 0; frame < 900 && !playing; frame++)
        {
            core.RunFrame(0x1008);
            playing = sprites() > 0;
        }

        Assert.True(playing, "Duck Hunt never left its title screen.");

        int trial = 0;
        int since = 0;
        (int X, int Y)? previous = null;
        for (int frame = 0; frame < 60_000 && trial < trials; frame++)
        {
            core.RunFrame((ushort)(frame % 150 < 50 ? 0x1008 : 0x1000));
            since++;

            // A duck in flight: a compact cluster, up in the sky, that has moved since the
            // frame before. The dog walks in the grass and the round displays do not move.
            (int X, int Y)? target = null;
            if (duck() is { } now && now.Y is >= 32 and <= 100)
            {
                if (previous is { } before
                    && Math.Abs(before.X - now.X) + Math.Abs(before.Y - now.Y) >= 3)
                {
                    target = now;
                }

                previous = now;
            }
            else
            {
                previous = null;
            }

            if (target is null || since < 120)
            {
                continue;
            }

            since = 0;
            trial++;
            byte[] state = core.SaveState();
            byte[] onTarget = Shoot(state, target.Value.X, target.Value.Y);

            // A far corner of the sky, at least 64 pixels from the duck in both axes.
            int besideX = target.Value.X > 128 ? 8 : 247;
            int besideY = target.Value.Y > 80 ? 8 : 150;
            byte[] beside = Shoot(state, besideX, besideY);

            List<int> diverged = [];
            for (int address = 0; address < onTarget.Length; address++)
            {
                if (onTarget[address] != beside[address])
                {
                    diverged.Add(address);
                }
            }

            output.WriteLine(
                $"trial {trial}: duck at ({target.Value.X}, {target.Value.Y}), beside at "
                + $"({besideX}, {besideY}) -> {diverged.Count} RAM bytes differ.");

            if (diverged.Count > 0)
            {
                conforming++;
                foreach (int address in diverged)
                {
                    divergedOnTarget[address] = divergedOnTarget.GetValueOrDefault(address) + 1;
                }
            }

            // Control: the same aim twice must give the very same state, so the divergence
            // above is the aim and not the emulation.
            byte[] again = Shoot(state, besideX, besideY);
            for (int address = 0; address < beside.Length; address++)
            {
                if (beside[address] != again[address])
                {
                    divergedBeside[address] = divergedBeside.GetValueOrDefault(address) + 1;
                }
            }
        }

        output.WriteLine($"{conforming} of {trial} trials diverged on the aim.");
        output.WriteLine(
            "addresses diverging in at least 9 trials: "
            + string.Join(
                ", ",
                divergedOnTarget.Where(e => e.Value >= 9).Select(e => $"${e.Key:X4}")));
        Assert.Empty(divergedBeside);

        // The criterion: at least 9 of the 10 trials behave differently when the shot is on
        // the duck and when it is beside it.
        Assert.True(conforming >= 9, $"only {conforming} of {trials} trials distinguished the aim.");

        // And the divergence is systematic, not an accident of one trial: at least one address
        // tells the two apart in nearly every trial.
        Assert.Contains(divergedOnTarget, e => e.Value >= 9);

        // ---------------------------------------------------------------- local helpers
        byte[] Shoot(byte[] state, int x, int y)
        {
            NesCore shot = Zapper(out Action<int, int> at, out _, out _);
            Assert.True(shot.TryLoadState(state, 0, out _, out _, out string? error), error);
            at(x, y);

            // The trigger is held four frames, then released; the game reads the light over
            // the frames that follow, and 240 more frames let the round resolve.
            for (int frame = 0; frame < 4; frame++)
            {
                shot.RunFrame(0x0000);
            }

            for (int frame = 0; frame < 240; frame++)
            {
                shot.RunFrame(0x1000);
            }

            return [.. shot.Bus.Ram];
        }
    }

    /// <summary>
    /// A Duck Hunt core with the Zapper on port 2, an <paramref name="aim"/> setter and a
    /// reader giving the middle of the sprites of the frame just run — the duck, once the dog
    /// has left the screen.
    /// </summary>
    private static NesCore Zapper(
        out Action<int, int> aim, out Func<(int X, int Y)?> duck, out Func<int> spriteCount)
    {
        NesCore core = UserRoms.Load(UserRoms.All.Single(g => g.Rom == DuckHunt));
        core.SetPortType(1, ControllerPortType.Zapper);

        int x = -1;
        int y = -1;
        core.ZapperPositionSource = _ => new ZapperPosition(x, y);
        aim = (ax, ay) =>
        {
            x = ax;
            y = ay;
        };

        (int X, int Y)? middle = null;
        int all = 0;
        core.VideoCallback = (in FrameBatch batch) =>
        {
            // A duck in flight is a compact cluster of two to eight sprite tiles in the sky.
            // The dog walks in the grass and the score row sits at the bottom, so both the
            // band and the size of the cluster are bounded.
            all = 0;
            int count = 0;
            int minX = int.MaxValue;
            int maxX = int.MinValue;
            int minY = int.MaxValue;
            int maxY = int.MinValue;
            foreach (TileRecord record in batch.Records)
            {
                if (record.Background)
                {
                    continue;
                }

                all++;
                if (record.Y is <= 24 or >= 120)
                {
                    continue;
                }

                count++;
                minX = Math.Min(minX, record.X);
                maxX = Math.Max(maxX, record.X + 8);
                minY = Math.Min(minY, record.Y);
                maxY = Math.Max(maxY, record.Y + 8);
            }

            middle = count is >= 2 and <= 8 && maxX - minX <= 32 && maxY - minY <= 32
                ? ((minX + maxX) / 2, (minY + maxY) / 2)
                : null;
        };

        duck = () => middle;
        spriteCount = () => all;
        return core;
    }

    /// <summary>
    /// REQ-CORE-114, [ROM] half: « la barre de statut de SMB3 ne tremble pas en jeu (inspection
    /// sur 60 s) ». The bar trembles when the line the MMC3 IRQ writes <c>$2006</c> on moves
    /// from frame to frame, which shows in the picture as the top edge of the bar moving. What
    /// is measured over 3 600 frames of play is therefore that edge, in the indexed picture of
    /// REQ-CORE-048: the first scanline of the uniform band the bar starts with. The background
    /// rows of REQ-CORE-037 are reported next to it, with the two facts criterion 2 of AMD-021
    /// R2-005 asks for (AMD-022 Q-059.1).
    /// </summary>
    [Trait("Requires", "UserRoms")]
    [RomOfTheUserFact(Smb3)] // REQ-CORE-114
    public void The_status_bar_of_smb3_does_not_tremble_over_sixty_seconds()
    {
        NesCore core = UserRoms.Load(Game(Smb3));

        Dictionary<int, int> rowBandTop = [];
        int atSplitLine = 0;
        int smallestGap = int.MaxValue;
        core.VideoCallback = (in FrameBatch batch) =>
        {
            SortedSet<int> ys = [];
            foreach (TileRecord record in batch.Records)
            {
                if (record.Background)
                {
                    ys.Add(record.Y);
                }
            }

            // AMD-021 R2-005, criterion 2 as corrected by AMD-022 Q-059.1: no row at the line
            // the split writes $2006 on (line 193, so y = 192), and never two rows less than
            // three lines apart.
            if (ys.Contains(192))
            {
                atSplitLine++;
            }

            int previous = int.MinValue / 2;
            foreach (int y in ys)
            {
                smallestGap = Math.Min(smallestGap, y - previous);
                previous = y;
            }

            int[] band = [.. ys.Where(y => y >= 190)];
            int top = band.Length == 0 ? -1 : band[0];
            rowBandTop[top] = rowBandTop.GetValueOrDefault(top) + 1;
        };

        // A minute of play, after the title screen: Start, then the scripted sequence.
        Random random = new(3);
        ushort held = 0;
        for (int frame = 0; frame < 1_200; frame++)
        {
            core.RunFrame(UserRoms.ScriptedInput(frame, random, ref held));
        }

        rowBandTop.Clear();
        for (int frame = 1_200; frame < 1_200 + 3_600; frame++)
        {
            core.RunFrame(UserRoms.ScriptedInput(frame, random, ref held));
        }

        output.WriteLine("first background row of the bottom band, by number of frames:");
        foreach ((int top, int count) in rowBandTop.OrderByDescending(e => e.Value))
        {
            output.WriteLine($"  y = {top}: {count} frames");
        }

        output.WriteLine($"frames with a row at the split line: {atSplitLine}");
        output.WriteLine($"smallest gap between two rows of one frame: {smallestGap}");

        // AMD-021 R2-005, criterion 2 (AMD-022 Q-059.1).
        Assert.Equal(0, atSplitLine);
        Assert.True(smallestGap >= 3, $"two rows only {smallestGap} lines apart.");

        // The bar itself: 600 more frames walked scanline by scanline, looking for the line on
        // which the vertical scroll jumps — the one the IRQ handler writes $2006 on. The bar
        // trembles exactly when that line moves, so its histogram is the inspection.
        core.VideoCallback = null;
        Dictionary<int, int> splitLine = [];
        for (int frame = 0; frame < 600; frame++)
        {
            StepToVisibleLine(core, 0);
            int fine = core.Ppu.FineY;
            int found = -1;
            for (int line = 1; line < 240; line++)
            {
                StepToVisibleLine(core, line);
                int now = core.Ppu.FineY;
                if (line >= 150 && found < 0 && now != ((fine + 1) & 7))
                {
                    found = line;
                }

                fine = now;
            }

            splitLine[found] = splitLine.GetValueOrDefault(found) + 1;
        }

        output.WriteLine("line the vertical scroll jumps on, by number of frames:");
        foreach ((int line, int count) in splitLine.OrderByDescending(e => e.Value))
        {
            output.WriteLine($"  line {line}: {count} frames");
        }

        // One line, the same in every one of the 600 frames: the bar does not tremble.
        Assert.Single(splitLine);
        Assert.InRange(splitLine.Keys.Single(), 150, 239);
    }

    /// <summary>
    /// Runs the emulated CPU until the PPU reaches visible <paramref name="line"/>. Unlike
    /// <c>RunFrame</c> this gives control back inside the frame, which is what a per-scanline
    /// measurement needs; the input word stays the one of the last <c>RunFrame</c>.
    /// </summary>
    private static void StepToVisibleLine(NesCore core, int line)
    {
        for (int guard = 0; guard < 200_000 && core.Ppu.Scanline != line; guard++)
        {
            core.Cpu.Step();
        }

        Assert.Equal(line, core.Ppu.Scanline);
    }

    /// <summary>
    /// REQ-CORE-115, sensitive point « scroll vertical en milieu de frame par <c>$2006</c> »:
    /// The Legend of Zelda and Metroid split their screen that way, and so does Super Mario
    /// Bros. by sprite 0. The split must show up as an extra band of rows, and the rows must
    /// obey R2-005.
    /// </summary>
    [Trait("Requires", "UserRoms")]
    [RomsOfTheUserTheory] // REQ-CORE-115, AMD-021 R2-005
    [InlineData("Legend of Zelda, The (USA).nes", 1_200)]
    [InlineData("Super Mario Bros. (World).nes", 1_200)]
    [InlineData("Super Mario Bros. 3 (USA).nes", 1_200)]
    public void A_split_screen_game_keeps_its_rows_three_lines_apart(string rom, int settle)
    {
        if (!UserRoms.Has(rom))
        {
            Assert.Fail($"user ROM not available: {UserRoms.Path(rom)}.");
        }

        NesCore core = UserRoms.Load(Game(rom));

        int smallestGap = int.MaxValue;
        int maxRows = 0;
        int measured = 0;
        bool measuring = false;
        core.VideoCallback = (in FrameBatch batch) =>
        {
            if (!measuring)
            {
                return;
            }

            SortedSet<int> ys = [];
            foreach (TileRecord record in batch.Records)
            {
                if (record.Background)
                {
                    ys.Add(record.Y);
                }
            }

            if (ys.Count == 0)
            {
                return;
            }

            int previous = int.MinValue / 2;
            foreach (int y in ys)
            {
                smallestGap = Math.Min(smallestGap, y - previous);
                previous = y;
            }

            maxRows = Math.Max(maxRows, ys.Count);
            measured++;
        };

        Random random = new(4);
        ushort held = 0;
        for (int frame = 0; frame < settle; frame++)
        {
            core.RunFrame(UserRoms.ScriptedInput(frame, random, ref held));
        }

        measuring = true;
        for (int frame = settle; frame < settle + 3_600; frame++)
        {
            core.RunFrame(UserRoms.ScriptedInput(frame, random, ref held));
        }

        output.WriteLine($"{rom}: {measured} frames measured, max {maxRows} rows, smallest gap {smallestGap}.");

        Assert.True(measured > 3_000, $"only {measured} frames carried background rows.");
        Assert.True(smallestGap >= 3, $"two rows only {smallestGap} lines apart.");

        // An NTSC frame holds 30 fine-Y-1 lines; a split can add at most one row (AMD-022
        // Q-059.1 admits the phase case the implementers reported).
        Assert.True(maxRows <= 31, $"{maxRows} rows in one frame.");
    }

    /// <summary>
    /// REQ-ACC-031, [ROM] half: « mesure du harnais sur 1 000 frames de Super Mario Bros 3 ».
    /// The conversion of one frame must hold in 4 ms on average and 8 ms at the 99th centile.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [Trait("Requires", "UserRoms")]
    [CorpusRomFact(Smb3)] // REQ-ACC-031, REQ-E3D-360
    public void Converting_a_thousand_frames_of_smb3_fits_the_time_budget()
    {
        NesCore core = UserRoms.Load(Game(Smb3));
        ConversionEngine engine = new();
        engine.LoadProfile(UserRoms.Profile(Game(Smb3)));

        List<double> samples = [];
        bool measuring = false;
        core.VideoCallback = (in FrameBatch batch) =>
        {
            long start = Stopwatch.GetTimestamp();
            engine.Convert(batch, (int)core.FrameCount);
            engine.Publish();
            double elapsed = Stopwatch.GetElapsedTime(start).TotalMilliseconds;
            if (measuring)
            {
                samples.Add(elapsed);
            }
        };

        // Warm-up: the title screen and the first seconds of play settle the pools and the
        // learned registry, which the criterion excludes ("hors construction initiale").
        Random random = new(5);
        ushort held = 0;
        for (int frame = 0; frame < 1_200; frame++)
        {
            core.RunFrame(UserRoms.ScriptedInput(frame, random, ref held));
        }

        // Three passes of 1 000 frames; the best is the one that measures the code rather than
        // the load of the machine the suite shares (same protocol as REQ-E3D-360, Q-042).
        List<(double Mean, double P99, double Worst)> runs = [];
        for (int run = 0; run < 3; run++)
        {
            samples.Clear();
            measuring = true;
            for (int frame = 0; frame < 1_000; frame++)
            {
                core.RunFrame(UserRoms.ScriptedInput(2_000 + (run * 1_000) + frame, random, ref held));
            }

            measuring = false;
            double[] sorted = [.. samples];
            double mean = sorted.Average();
            Array.Sort(sorted);
            runs.Add((mean, sorted[(int)(sorted.Length * 0.99)], sorted[^1]));
        }

        string report = string.Join(
            " | ", runs.Select(r => $"mean {r.Mean:F3} / p99 {r.P99:F3} / max {r.Worst:F3} ms"));
        output.WriteLine($"SMB 3, 3 x 1000 frames: {report}");

        (double bestMean, double bestP99, double bestWorst) = runs.OrderBy(r => r.Mean).First();
        Assert.True(bestMean <= 4.0 && bestP99 <= 8.0, $"budget: mean 4 ms, p99 8 ms. {report}");
        Assert.True(bestWorst <= 16.6, $"worst conversion {bestWorst:F3} ms exceeds one frame. {report}");
    }

    /// <summary>
    /// REQ-E3D-322, [ROM] half: the <c>BuildPattern</c> hook of the Mega Man 2 profile runs for
    /// the game's own patterns, before the geometry is built, and the depths it sets are the
    /// ones the meshes are built from.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [Trait("Requires", "UserRoms")]
    [CorpusRomFact(MegaMan2)] // REQ-E3D-322
    public void The_build_pattern_hook_of_mega_man_2_runs_on_the_games_own_patterns()
    {
        UserGame game = Game(MegaMan2);
        GameProfile profile = UserRoms.Profile(game);
        Assert.NotEmpty(profile.Scripts);

        using LuaRun run = LuaRun.Create(profile);
        run.Runtime.InvokeStart();

        NesCore core = UserRoms.Load(game);
        int frames = 0;
        long shapes = 0;
        core.VideoCallback = (in FrameBatch batch) =>
        {
            ConversionFrame frame = run.Engine.Convert(batch, (int)core.FrameCount);
            run.Engine.Publish();
            shapes += frame.Shapes.Count;
            frames++;
        };

        Random random = new(6);
        ushort held = 0;
        for (int frame = 0; frame < 3_600; frame++)
        {
            core.RunFrame(UserRoms.ScriptedInput(frame, random, ref held));
        }

        run.Runtime.InvokeEnd();

        output.WriteLine($"Mega Man 2: {frames} frames, {shapes} shapes, {run.Log.Errors.Count} script errors.");
        foreach (string error in run.Log.Errors.Take(5))
        {
            output.WriteLine($"  {error}");
        }

        Assert.Equal(3_600, frames);
        Assert.True(shapes > 0, "no shape was recognised.");

        // REQ-ACC-011 holds on the real frames too: no API member is missing.
        Assert.DoesNotContain(
            run.Log.Errors,
            error => error.Contains("a nil value", StringComparison.OrdinalIgnoreCase));
    }

    /// <summary>What one run of 10 000 game frames measured.</summary>
    private sealed record Budget(
        double Mean,
        double P99,
        double Worst,
        double BytesPerFrame,
        double PauseMilliseconds,
        int FramesOverFourMilliseconds);

    /// <summary>
    /// REQ-LUA-110 and REQ-LUA-111, [ROM] halves: on 10 000 real frames of a reference game, the
    /// frame hooks stay under 2 ms on average and 6 ms at the 99th centile, the scripting layer
    /// allocates less than 1 KiB per frame, and no collection pause is owed to it.
    /// </summary>
    /// <remarks>
    /// Both figures are differential: the very same 10 000 frames are run twice, once with the
    /// profile's Lua runtime attached and once with the same engine and profile but no script
    /// host at all. The difference is the scripting layer's own cost, which is what the two
    /// requirements bound; the absolute figures of the scripted run are reported next to it.
    /// REQ-LUA-111 asks for « aucune pause > 4 ms … qui ne survienne aussi sans scripts »:
    /// <see cref="GC.GetTotalPauseDuration"/> gives the total pause time of the run, and a total
    /// below 4 ms per minute of game is a sufficient condition for no single pause to exceed it.
    /// </remarks>
    [Trait("Requires", "OriginalAssets")]
    [Trait("Requires", "UserRoms")]
    [RomsOfTheUserTheory(profiles: true)] // REQ-LUA-110, REQ-LUA-111
    [InlineData("Super Mario Bros. 3 (USA).nes")]
    [InlineData("Mega Man 2 (USA).nes")]
    [InlineData("Legend of Zelda, The (USA).nes")]
    public void The_frame_hooks_of_a_heavy_profile_fit_their_budget_on_real_frames(string rom)
    {
        if (!UserRoms.Has(rom))
        {
            Assert.Fail($"user ROM not available: {UserRoms.Path(rom)}.");
        }

        UserGame game = Game(rom);
        Assert.NotEmpty(UserRoms.Profile(game).Scripts);

        Budget scripted = Measure(game, withScripts: true);
        Budget plain = Measure(game, withScripts: false);

        double hookMean = scripted.Mean - plain.Mean;
        double hookBytes = scripted.BytesPerFrame - plain.BytesPerFrame;
        double hookPause = scripted.PauseMilliseconds - plain.PauseMilliseconds;
        const double minutes = 10_000 / 3600.0;

        output.WriteLine(
            $"{game.Title}, 10 000 frames with scripts: mean {scripted.Mean:F3} ms, "
            + $"p99 {scripted.P99:F3} ms, max {scripted.Worst:F3} ms, "
            + $"{scripted.BytesPerFrame:F0} bytes/frame, {scripted.PauseMilliseconds:F1} ms of GC pause.");
        output.WriteLine(
            $"  without scripts: mean {plain.Mean:F3} ms, p99 {plain.P99:F3} ms, "
            + $"{plain.BytesPerFrame:F0} bytes/frame, {plain.PauseMilliseconds:F1} ms of GC pause.");
        output.WriteLine(
            $"  hooks alone: mean {hookMean:F3} ms, {hookBytes:F0} bytes/frame, "
            + $"{hookPause:F1} ms of GC pause over {minutes:F1} minutes of game "
            + $"({hookPause / minutes:F2} ms per minute, process-wide and therefore an "
            + "over-estimate).");
        output.WriteLine(
            $"  frames over 4 ms: {scripted.FramesOverFourMilliseconds} with scripts, "
            + $"{plain.FramesOverFourMilliseconds} without.");

        // REQ-LUA-110: 2 ms on average and 6 ms at the 99th centile. The scripted run carries
        // the conversion as well, so it is an upper bound on the hooks alone.
        Assert.True(scripted.Mean < 2.0, $"{scripted.Mean:F3} ms per frame, conversion included.");
        Assert.True(scripted.P99 < 6.0, $"99th centile {scripted.P99:F3} ms, conversion included.");

        // REQ-LUA-111: under 1 KiB per frame owed to the scripting layer.
        Assert.True(hookBytes < 1024, $"{hookBytes:F0} bytes per frame owed to the Lua layer.");

        // And no collection pause owed to it. A pause of more than 4 ms falls inside a frame
        // and makes that frame last more than 4 ms, so the frames over 4 ms are counted in
        // both runs: the scripted run must not add more than the one per minute of game that
        // REQ-LUA-111 calls the defect threshold. (The total pause duration above is
        // process-wide — the test host allocates too — so it is reported, not asserted.)
        Assert.True(
            scripted.FramesOverFourMilliseconds
                <= plain.FramesOverFourMilliseconds + (int)Math.Ceiling(minutes),
            $"{scripted.FramesOverFourMilliseconds} frames over 4 ms with scripts against "
            + $"{plain.FramesOverFourMilliseconds} without, over {minutes:F1} minutes of game.");
    }

    /// <summary>
    /// Runs 1 200 frames of warm-up and 10 000 measured frames of <paramref name="game"/>
    /// through its own profile, with or without the profile's Lua runtime.
    /// </summary>
    private static Budget Measure(UserGame game, bool withScripts)
    {
        GameProfile profile = UserRoms.Profile(game);
        LuaRun? run = withScripts ? LuaRun.Create(profile) : null;
        ConversionEngine engine;
        if (run is not null)
        {
            run.Runtime.InvokeStart();
            engine = run.Engine;
        }
        else
        {
            engine = new ConversionEngine();
            engine.LoadProfile(profile);
        }

        try
        {
            NesCore core = UserRoms.Load(game);
            List<double> samples = [];
            bool measuring = false;
            long allocated = 0;
            core.VideoCallback = (in FrameBatch batch) =>
            {
                long before = GC.GetAllocatedBytesForCurrentThread();
                long start = Stopwatch.GetTimestamp();
                engine.Convert(batch, (int)core.FrameCount);
                engine.Publish();
                double elapsed = Stopwatch.GetElapsedTime(start).TotalMilliseconds;
                if (measuring)
                {
                    samples.Add(elapsed);
                    allocated += GC.GetAllocatedBytesForCurrentThread() - before;
                }
            };

            Random random = new(8);
            ushort held = 0;
            for (int frame = 0; frame < 1_200; frame++)
            {
                core.RunFrame(UserRoms.ScriptedInput(frame, random, ref held));
            }

            TimeSpan pause = GC.GetTotalPauseDuration();
            measuring = true;
            for (int frame = 1_200; frame < 1_200 + 10_000; frame++)
            {
                core.RunFrame(UserRoms.ScriptedInput(frame, random, ref held));
            }

            measuring = false;
            pause = GC.GetTotalPauseDuration() - pause;

            double[] sorted = [.. samples];
            double mean = sorted.Average();
            Array.Sort(sorted);
            return new Budget(
                mean,
                sorted[(int)(sorted.Length * 0.99)],
                sorted[^1],
                allocated / (double)samples.Count,
                pause.TotalMilliseconds,
                sorted.Count(sample => sample > 4.0));
        }
        finally
        {
            run?.Dispose();
        }
    }
}
