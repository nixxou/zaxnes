using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Harness;

/// <summary>
/// One synthetic frame ready to render: the tile list of SPEC-04B REQ-E3D-641 and the scene state
/// SPEC-06 needs, built without any ROM.
/// </summary>
/// <param name="Name">Name of the scene, used for the capture file.</param>
/// <param name="Scene">The scene state, settings and camera included.</param>
/// <param name="Engine">The engine that produced the frame, kept alive for the draw list.</param>
/// <param name="Description">One line saying what the scene checks.</param>
internal sealed record SyntheticScene(
    string Name,
    SceneState Scene,
    ConversionEngine Engine,
    string Description)
{
    /// <summary>
    /// The flat 256 × 240 NES picture of the frame, one colour index per pixel (REQ-CORE-048),
    /// when the scene came from a real ROM. It is written next to the 3D capture so that a
    /// difference between the two can be pinned on the conversion rather than on the renderer.
    /// </summary>
    public byte[]? NesPicture { get; init; }
}

/// <summary>
/// The synthetic frames the renderer is checked against, each built from the acceptance criteria
/// of SPEC-06 so that they can be verified without the user's ROM.
/// </summary>
internal static class SyntheticScenes
{
    /// <summary>Names of every scene, in a stable order.</summary>
    public static IReadOnlyList<string> Names { get; } =
    [
        "framing",
        "sky",
        "shadow",
        "no-shadow",
        "layer-shadow-off",
        "nudge",
        "alpha",
        "hud",
        "clip",
        "backdrop",
        "heavy",
    ];

    /// <summary>Builds the scene called <paramref name="name"/>.</summary>
    /// <exception cref="ArgumentException">The name is not one of <see cref="Names"/>.</exception>
    public static SyntheticScene Build(string name, RenderAlgorithm mesh = RenderAlgorithm.Greedy)
    {
        Algorithm = mesh;
        return BuildScene(name);
    }

    /// <summary>The mesh algorithm the next build uses (REQ-E3D-340).</summary>
    private static RenderAlgorithm Algorithm { get; set; } = RenderAlgorithm.Greedy;

    private static SyntheticScene BuildScene(string name) => name switch
    {
        "framing" => Framing(),
        "sky" => Sky(),
        "shadow" => ShadowPair(shadows: true, layerCasts: true),
        "no-shadow" => ShadowPair(shadows: false, layerCasts: true),
        "layer-shadow-off" => ShadowPair(shadows: true, layerCasts: false),
        "nudge" => Nudge(),
        "alpha" => Alpha(),
        "hud" => Hud(),
        "clip" => Clip(),
        "backdrop" => Backdrop(),
        "heavy" => Heavy(),
        _ => throw new ArgumentException($"unknown synthetic scene '{name}'", nameof(name)),
    };

    /// <summary>
    /// A flat background of one colour over the whole picture, with the universal background
    /// colour 0x22. It pins down the default camera framing (REQ-SCN-012) and the background
    /// colour rule of REQ-SCN-046.
    /// </summary>
    private static SyntheticScene Framing()
    {
        Builder builder = new();
        builder.Background(0x22);
        builder.Palette(1, 0x30);
        for (int row = 1; row < 30; row++)
        {
            for (int column = 0; column < 32; column++)
            {
                builder.Background(Solid(1), column * 8, row * 8);
            }
        }

        return builder.Finish(
            "framing",
            "flat 0x30 background over the whole picture, background colour 0x22, default camera");
    }

    /// <summary>
    /// An empty frame: nothing but the sky. It is what a capture of the procedural skyboxes of
    /// AMD-024 REQ-PUB-003 needs — the gradient, the stars, and the sun, the moon and the clouds
    /// of <c>--sky-bodies</c> — with no tile in the way.
    /// </summary>
    private static SyntheticScene Sky()
    {
        Builder builder = new();
        builder.Background(0x0F);

        // The Fantasy sky recolours itself from colours 1 to 3 of background palette 0
        // (REQ-SCN-051), so the frame carries a palette even though it draws no tile.
        builder.Palette(1, 0x21);
        builder.Palette(2, 0x12);
        builder.Palette(3, 0x30);
        return builder.Finish("sky", "an empty frame with a palette, so that only the skybox shows");
    }

    /// <summary>
    /// The acceptance scene of REQ-SCN-030: a solid tile in L1 above a flat tile in L2 that
    /// receives the shadows, light (1, −1, 1).
    /// </summary>
    private static SyntheticScene ShadowPair(bool shadows, bool layerCasts)
    {
        Builder builder = new();
        builder.Background(0x0F);
        builder.Palette(1, 0x30);
        builder.Palette(2, 0x16);

        // A 16 × 10 tile floor in L2, flat (depth 1), which receives the shadows.
        builder.AddPattern(Solid(1), depth: 1, ProfileLayer.L2);
        for (int row = 10; row < 20; row++)
        {
            for (int column = 8; column < 24; column++)
            {
                builder.Background(Solid(1), column * 8, row * 8);
            }
        }

        // A 2 × 2 solid block in L1, deep, which casts on the floor.
        builder.AddPattern(Solid(2), depth: 8, ProfileLayer.L1);
        for (int row = 13; row < 15; row++)
        {
            for (int column = 14; column < 16; column++)
            {
                builder.Background(Solid(2), column * 8, row * 8, palette: 0);
            }
        }

        string name = shadows ? layerCasts ? "shadow" : "layer-shadow-off" : "no-shadow";
        SyntheticScene scene = builder.Finish(
            name,
            "solid L1 block over a flat L2 floor, light (1, -1, 1)",
            settings =>
            {
                settings.Shadow = shadows;
                if (!layerCasts)
                {
                    // REQ-SCN-032: clearing the casting flag of a layer removes its shadows
                    // from the next frame onwards.
                    settings.Layer(ZLayer.L1).CastShadow = false;
                }
            });

        scene.Scene.Settings.Shadows = shadows;
        return scene;
    }

    /// <summary>
    /// The acceptance scene of REQ-SCN-037: a flat sprite laid over a flat background tile at the
    /// same depth. The ±0.1 nudge the renderer applies has to keep the sprite in front.
    /// </summary>
    private static SyntheticScene Nudge()
    {
        Builder builder = new();
        builder.Background(0x0F);
        builder.Palette(1, 0x21);
        builder.Palette(17, 0x16);

        builder.AddPattern(Solid(1), depth: 1, ProfileLayer.L1);
        for (int row = 12; row < 18; row++)
        {
            for (int column = 12; column < 20; column++)
            {
                builder.Background(Solid(1), column * 8, row * 8);
            }
        }

        // The sprite uses palette 0 of the sprite tables, so colour index 1 is entry 17.
        builder.AddPattern(Solid(2), depth: 1, ProfileLayer.L1);
        builder.Palette(18, 0x16);
        for (int row = 13; row < 17; row++)
        {
            for (int column = 13; column < 19; column++)
            {
                builder.Sprite(Solid(2), column * 8, row * 8);
            }
        }

        return builder.Finish(
            "nudge",
            "flat sprite over a flat background tile at the same depth (REQ-SCN-037)");
    }

    /// <summary>A pattern of alpha 0.5 over a red background (REQ-SCN-040).</summary>
    private static SyntheticScene Alpha()
    {
        Builder builder = new();
        builder.Background(0x0F);
        builder.Palette(1, 0x16);
        builder.Palette(5, 0x30);

        builder.AddPattern(Solid(1), depth: 1, ProfileLayer.L2);
        for (int row = 10; row < 20; row++)
        {
            for (int column = 8; column < 24; column++)
            {
                builder.Background(Solid(1), column * 8, row * 8);
            }
        }

        builder.AddPattern(Solid(2), depth: 6, ProfileLayer.L1, alpha: 0.5f);
        builder.Palette(2, 0x30);
        for (int row = 13; row < 17; row++)
        {
            for (int column = 13; column < 19; column++)
            {
                builder.Background(Solid(2), column * 8, row * 8);
            }
        }

        return builder.Finish("alpha", "alpha 0.5 pattern in front of a red wall (REQ-SCN-040)");
    }

    /// <summary>
    /// A UI pattern placed outside the visible rectangle, which has to stay whole
    /// (REQ-SCN-044).
    /// </summary>
    private static SyntheticScene Hud()
    {
        Builder builder = new();
        builder.Background(0x0F);
        builder.Palette(1, 0x30);

        builder.AddPattern(Solid(1), depth: 2, ProfileLayer.Ui);

        // Screen row 0 is scene y in [240, 248], above the top edge of R (REQ-E3D-410).
        for (int column = 4; column < 12; column++)
        {
            builder.Background(Solid(1), column * 8, 0);
        }

        return builder.Finish("hud", "UI pattern above the visible rectangle (REQ-SCN-044)");
    }

    /// <summary>An object straddling the left edge of R, which has to be cut (REQ-SCN-036).</summary>
    private static SyntheticScene Clip()
    {
        Builder builder = new();
        builder.Background(0x0F);
        builder.Palette(1, 0x30);

        builder.AddPattern(Solid(1), depth: 8, ProfileLayer.L1);
        for (int row = 8; row < 22; row++)
        {
            for (int column = 0; column < 6; column++)
            {
                builder.Background(Solid(1), column * 8, row * 8);
            }
        }

        SyntheticScene scene = builder.Finish("clip", "block straddling the left edge of R (REQ-SCN-036)");
        scene.Scene.Clipping = new Vec4F(24.1f, 8.1f, 8.1f, 0.1f);
        return scene;
    }

    /// <summary>The backdrop of REQ-SCN-048, 26 units behind the scene.</summary>
    private static SyntheticScene Backdrop()
    {
        SyntheticScene scene = ShadowPair(shadows: true, layerCasts: true) with { Name = "backdrop" };
        scene.Scene.Backdrop.Enabled = true;
        scene.Scene.Backdrop.Color = new Rgb24(0, 0, 0);
        scene.Scene.Backdrop.Offset = new System.Numerics.Vector3(0f, 0f, 26f);
        return scene with { Description = "black backdrop at z = 25.5 receiving the front shadows" };
    }

    /// <summary>
    /// The heavy frame of REQ-SCN-064: a full background grid plus 64 sprites, every tile its own
    /// shape, which is what the performance target is measured on.
    /// </summary>
    private static SyntheticScene Heavy()
    {
        Builder builder = new();
        builder.Background(0x22);
        for (int entry = 1; entry < 64; entry++)
        {
            builder.Palette(entry, (byte)(0x01 + (entry % 0x3F)));
        }

        // 32 distinct images, each its own non-self-connected pattern carrying two 3D objects
        // (REQ-E3D-541), so that one tile is one shape and two drawn tiles: 960 background
        // shapes plus 64 sprite shapes make 1024 shapes and 2048 tiles, the load REQ-SCN-064
        // names.
        for (int image = 0; image < 32; image++)
        {
            builder.AddPattern(
                Solid(1, image),
                depth: 6,
                ProfileLayer.L1,
                selfConnected: false,
                objects: 2);
        }

        int n = 0;
        for (int row = 1; row < 31; row++)
        {
            for (int column = 0; column < 32; column++)
            {
                builder.Background(Solid(1, n++ % 32), column * 8, row * 8, palette: (row + column) & 3);
            }
        }

        for (int sprite = 0; sprite < 64; sprite++)
        {
            builder.Sprite(Solid(1, sprite % 32), (sprite * 3) % 248, 8 + ((sprite * 5) % 224), sprite & 3);
        }

        return builder.Finish("heavy", "full background grid and 64 sprites, one shape per tile");
    }

    /// <summary>A full tile of colour <paramref name="color"/> whose identity is unique.</summary>
    private static (ulong Low, ulong High) Solid(int color, int id = 0)
    {
        ulong low = (color & 1) != 0 ? ulong.MaxValue : 0UL;
        ulong high = (color & 2) != 0 ? ulong.MaxValue : 0UL;

        // The identity has to be unique per image: one pixel carries the extra colour bit.
        if ((color & 2) == 0)
        {
            high = 1UL << (id & 63);
        }
        else
        {
            low = (color & 1) != 0 ? ulong.MaxValue : 1UL << (id & 63);
        }

        return (low, high);
    }

    /// <summary>
    /// Assembles a profile and a batch of tile records, then converts them with a real engine and
    /// a real geometry builder.
    /// </summary>
    private sealed class Builder
    {
        private readonly GameProfile _profile = GameProfile.CreateEmpty();
        private readonly List<TileRecord> _records = [];
        private readonly byte[] _palette = new byte[32];
        private int _spriteOrder;

        public void Background(byte nesColor) => _palette[0] = nesColor;

        public void Palette(int entry, byte nesColor)
        {
            if (entry is > 0 and < 32)
            {
                _palette[entry] = nesColor;
            }
        }

        public void AddPattern(
            (ulong Low, ulong High) tile,
            int depth,
            ProfileLayer layer,
            float alpha = 1f,
            bool selfConnected = true,
            int objects = 1)
        {
            ProfilePattern pattern = new()
            {
                Grid = new PatternGrid(1, 1, [new ProfileTile(tile.Low, tile.High, TileFlip.None)]),
                TileSize = new Vec2I(1, 1),
                Permanent = true,
                SelfConnected = selfConnected,
                ConstraintSize = Vec2I.Unconstrained,
            };
            for (int variant = 0; variant < objects; variant++)
            {
                pattern.Variants.Add(new PatternVariant
                {
                    Depth = depth,
                    Layer = layer,
                    Alpha = alpha,
                    RenderStyle = VariantRenderStyle.Normal,
                });
            }

            _profile.PermanentPatterns.Add(pattern);
        }

        public void Background((ulong Low, ulong High) tile, int x, int y, int palette = 0)
            => _records.Add(new TileRecord(
                tile.Low, tile.High, (short)x, (short)y, _records.Count,
                mirrorH: false, mirrorV: false, hidden: false, (byte)palette, background: true));

        public void Sprite((ulong Low, ulong High) tile, int x, int y, int palette = 0)
            => _records.Add(new TileRecord(
                tile.Low, tile.High, (short)x, (short)y, _spriteOrder++,
                mirrorH: false, mirrorV: false, hidden: false, (byte)palette, background: false));

        public SyntheticScene Finish(
            string name,
            string description,
            Action<ConversionSettings>? configure = null)
        {
            ConversionEngine engine = new();
            engine.UseGeometry();
            engine.Settings.RenderAlgorithm = Algorithm;
            engine.LoadProfile(_profile);

            // After the load, because loading a profile resets the layer shadow permissions
            // (REQ-SCN-033, REQ-PROF-042).
            configure?.Invoke(engine.Settings);

            FrameBatch batch = new(_records.ToArray(), _palette, 0, 0);
            engine.Convert(batch, frameCounter: 0);
            engine.Publish();

            SceneState scene = new();
            scene.Camera.SnapToSetpoints();
            scene.GameRunning = true;
            scene.Clipping = engine.Settings.Clipping;
            scene.DrawList = engine.BuildDrawList();
            scene.NesBackgroundColor = engine.PublishedFrame!.Palette.Color(0);
            scene.EmulatedTime = engine.PublishedFrame.Time;

            return new SyntheticScene(name, scene, engine, description);
        }
    }
}
