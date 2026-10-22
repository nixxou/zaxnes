using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Scripting;

namespace Zax.Acceptance.Tests;

/// <summary>
/// The remaining "auto" halves of the [ROM]-tagged criteria of part A, and the part of
/// REQ-CORE-115 the user's own ROMs make reachable.
/// </summary>
public sealed class RomAndRemainderTests
{
    /// <summary>
    /// The user's ROMs that are present on this machine: every <c>.nes</c> file of the folder
    /// <c>ZAXNES_ROMS</c> names (<see cref="UserRoms.Directory"/>), not only the thirteen dumps
    /// <see cref="UserRoms.All"/> maps to a catalogue profile.
    /// </summary>
    public static TheoryData<string> AvailableUserRoms
    {
        get
        {
            TheoryData<string> data = [];
            if (UserRoms.Directory is { } folder && Directory.Exists(folder))
            {
                foreach (string file in Directory.GetFiles(folder, "*.nes").Order(StringComparer.Ordinal))
                {
                    data.Add(Path.GetFileName(file));
                }
            }

            if (data.Count == 0)
            {
                data.Add(string.Empty);
            }

            return data;
        }
    }

    [Trait("Requires", "UserRoms")]
    [RomsOfTheUserTheory] // REQ-CORE-115 (the part the user's own ROMs make reachable)
    [MemberData(nameof(AvailableUserRoms))]
    public void A_user_rom_boots_and_plays_for_two_minutes(string file)
    {
        if (file.Length == 0)
        {
            return; // no user ROM on this machine: reported as "en attente de ROM".
        }

        NesCore core = new();
        Zax.Nes.Rom.RomLoadResult load = core.LoadRom(
            File.ReadAllBytes(UserRoms.Path(file)), file);
        Assert.True(load.Success, $"{file}: {load.Error}");

        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();

        int frames = 0;
        int shapes = 0;
        core.VideoCallback = (in FrameBatch batch) =>
        {
            ConversionFrame frame = engine.Convert(batch, (int)core.FrameCount);
            engine.Publish();
            shapes += frame.Shapes.Count;
            frames++;
        };

        // Two minutes of play, with Start pressed every second to get past title screens.
        Random random = new(5);
        for (int i = 0; i < 7_200; i++)
        {
            ushort input = (ushort)(i % 60 == 0 ? 0x08 : random.Next(0x100) & 0x0F);
            core.RunFrame(input);
        }

        geometry.Commit();

        Assert.Equal(7_200, frames);
        Assert.Equal(7_200, core.FrameCount);
        Assert.True(shapes > 0, $"{file}: no shape was ever recognised");
    }

    [Fact] // REQ-E3D-252 (the "auto" half of the criterion)
    public void A_script_can_read_and_write_the_palette_of_either_frame()
    {
        ConversionEngine engine = new();

        new SyntheticBatch().Palette(5, 0x16).Convert(engine, 1);
        engine.Publish();
        Assert.Equal(0x16, engine.FrameColorIndex(5));

        new SyntheticBatch().Palette(5, 0x16).Convert(engine, 2);
        engine.SetColorIndex(5, 0x30, current: true);
        Assert.Equal(0x30, engine.FrameColorIndex(5, current: true));

        // The published frame is untouched by a write aimed at the converting one.
        Assert.Equal(0x16, engine.FrameColorIndex(5));

        // Next batch: the entry is rebuilt from the core's palette.
        engine.Publish();
        new SyntheticBatch().Palette(5, 0x16).Convert(engine, 3);
        Assert.Equal(0x16, engine.FrameColorIndex(5, current: true));

        // Without the second argument the write aims at the published frame.
        engine.SetColorIndex(5, 0x21);
        Assert.Equal(0x21, engine.FrameColorIndex(5));
        Assert.Equal(new Rgb24(0x38, 0xC0, 0xFC), engine.FrameColor(5));
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-E3D-426 (the "auto" half: a custom texture smaller than the pattern)
    public void A_custom_texture_is_read_modulo_its_size()
    {
        GameProfile profile = ProfileFormat.Load(
            NeutralAssets.ReadProfile("Mega Man 1.bytes"), sourceName: "Mega Man 1").Profile!;

        // The profile is documented as carrying a texture whose size differs from its
        // pattern's; find one and check the reader honours it.
        (ProfilePattern Pattern, PatternVariant Variant)? mismatch = null;
        foreach (ProfilePattern pattern in profile.PermanentPatterns.Concat(profile.TemporaryPatterns))
        {
            foreach (PatternVariant variant in pattern.Variants)
            {
                if (variant.Texture is { IsNullMarker: false } texture
                    && pattern.TileSize is { } size
                    && (texture.Width != 8 * size.X || texture.Height != 8 * size.Y))
                {
                    mismatch = (pattern, variant);
                    break;
                }
            }

            if (mismatch is not null)
            {
                break;
            }
        }

        Assert.True(mismatch is not null, "Mega Man 1 carries no texture of a different size");

        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        engine.LoadProfile(profile);
        geometry.Commit();

        // The same check on a synthetic equivalent: an 8 x 8 texture on a 1 x 24 pattern.
        ConversionEngine synthetic = new();
        List<TileImage?> cells = [];
        for (int row = 0; row < 24; row++)
        {
            ulong low = 0x0101010101010101UL * (ulong)(row + 1);
            cells.Add(synthetic.Images.Intern(low, 0, false, false));
        }

        EnginePattern tall = new(1, 24, cells, permanent: true);
        VariantTexture small = new(8, 8, new TexturePixels(
            TextureFormat.Indexed, Enumerable.Repeat(0x5555, (8 * 8) / 16)));

        tall.AddInstance(new PatternInstance
        {
            Depth = 1,
            Layer = ZLayer.L1,
            Scale = new Vec3(1, 1, 1),
            Pivot = new Vec3(0.5f, 0.5f, 0.5f),
            DeformPivot = new Vec3(0.5f, 0.5f, 0.5f),
            Alpha = 1,
            Style = RenderStyle.Normal,
            CustomPalette = -1,
            TexTilting = new Vec2(1, 1),
            FillRate = 1,
            Texture = small,
        });

        synthetic.Patterns.Register(tall);
        GeometryBuilder builder = synthetic.UseGeometry();
        builder.Build(tall);
        builder.Commit();

        ObjectGeometry built = builder.Of(tall.Instances[0])!;
        Assert.Equal(8, built.Texture.Width);
        Assert.Equal(8, built.Texture.Height);

        // Every block of eight rows reads the same texels back, modulo 8.
        for (int y = 0; y < 24; y++)
        {
            Assert.Equal(built.Texture.Texel(3, y % 8), built.Texture.Texel(3, y));
        }
    }

    [Fact] // REQ-E3D-322 (the "auto" half, with a real Lua BuildPattern hook)
    public void A_lua_build_pattern_hook_runs_before_the_geometry_and_before_start()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Scripts.Add(new ProfileScript(
            "Build",
            enabled: true,
            source: """
                    local built = 0
                    local startedAt = -1

                    function BuildPattern()
                        pattern.sizeZ = 8
                        built = built + 1
                    end

                    function Start()
                        startedAt = built
                    end

                    function BuiltCount() return built end
                    function StartedAt() return startedAt end
                    """,
            tagString: "*"));

        ProfilePattern first = new()
        {
            TileSize = new Vec2I(1, 1),
            Permanent = true,
            Grid = new PatternGrid(1, 1, [new ProfileTile(0xFFFFFFFFFFFFFFFFUL, 0, TileFlip.None)]),
        };

        first.Variants.Add(new PatternVariant
        {
            Depth = 3,
            Alpha = 1,
            Scale = new Vec3F(1, 1, 1),
            Pivot = new Vec3F(0.5f, 0.5f, 0.5f),
            DeformPivot = new Vec3F(0.5f, 0.5f, 0.5f),
            RenderStyle = VariantRenderStyle.Normal,
        });

        profile.PermanentPatterns.Add(first);

        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();

        CollectingScriptLog log = new();
        using ProfileScriptRuntime runtime = ProfileScriptRuntime.Create(
            profile, new ScriptEnvironment { Engine = engine, Log = log });

        engine.ScriptHost = runtime;
        engine.LoadProfile(profile, applySettings: false);
        geometry.Commit();
        runtime.InvokeStart();

        Assert.Empty(log.Errors);
        Assert.Equal(1, runtime.EvaluateNumber("return BuiltCount()"));
        Assert.Equal(1, runtime.EvaluateNumber("return StartedAt()"));

        EnginePattern loaded = Assert.Single(engine.Patterns.Permanent);
        ObjectGeometry built = geometry.Of(loaded.Instances[0])!;
        Assert.Equal(8, built.BuildDepth);
    }
}
