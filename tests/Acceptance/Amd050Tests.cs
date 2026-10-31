using System.Numerics;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-050 — the universal background pixels of the 2D mode are transparent: the opacity mask
/// the core now produces (REQ-CORE-049), the pixel image it travels with (REQ-CORE-048 amended)
/// and the two points AMD-050 §3 adds to the frame batch (REQ-CORE-030). The vector V11 of
/// SPEC-02 §5.3, checked on the minimal ROM of §5.1.
/// </summary>
public sealed class Amd050Tests
{
    private const int Width = 256;
    private const int Height = 240;

    /// <summary>256 × 240 / 8 = 7 680 bytes (REQ-CORE-049).</summary>
    private const int MaskSize = Width * Height / 8;

    // ------------------------------------------------------------ REQ-CORE-030

    /// <summary>
    /// Points 4 and 5 of REQ-CORE-030: the pixel image and the opacity mask are part of the
    /// batch the video callback receives, and are only valid during the call like the rest.
    /// </summary>
    [Fact] // REQ-CORE-030, REQ-CORE-048, REQ-CORE-049
    public void REQ_CORE_030_the_batch_carries_the_picture_and_its_opacity_mask()
    {
        NesCore core = V11Core();
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x0A);

        Assert.Equal(Width * Height, batch.Picture.Length);
        Assert.Equal(MaskSize, batch.OpacityMask.Length);

        // The same data the core exposes after the frame, which is what the 2D mode reads.
        Assert.Equal(core.PixelFrame.ToArray(), batch.Picture);
        Assert.Equal(core.OpacityMask.ToArray(), batch.OpacityMask);
    }

    /// <summary>
    /// A frame that produces no image (<see cref="FrameOutput.Pixels"/> clear, a hidden
    /// run-ahead frame of REQ-LOOP-068) hands out no image and no mask rather than those of the
    /// frame before it.
    /// </summary>
    [Fact] // REQ-CORE-030, REQ-CORE-048
    public void REQ_CORE_030_a_frame_without_an_image_carries_neither_image_nor_mask()
    {
        NesCore core = V11Core();
        SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x0A);

        int picture = -1;
        int mask = -1;
        core.VideoCallback = (in FrameBatch batch) =>
        {
            picture = batch.Picture.Length;
            mask = batch.OpacityMask.Length;
        };
        core.RunFrame(0, FrameOutput.Video | FrameOutput.Audio);

        Assert.Equal(0, picture);
        Assert.Equal(0, mask);
    }

    // ------------------------------------------------------------ V11

    /// <summary>
    /// V11, first bullet: nametable <c>$2000</c> entirely filled with tile 1, OAM at
    /// <c>$FF</c>, PPUMASK = <c>$0A</c> — the mask is 7 680 bytes at <c>0xFF</c> and the image
    /// carries the indices of palette entries 1 and 2.
    /// </summary>
    [Fact] // REQ-CORE-048, REQ-CORE-049 (V11)
    public void V11_a_frame_covered_by_tile_1_is_entirely_opaque()
    {
        CapturedBatch batch = SpecRom.Capture(V11Core(), ppuCtrl: 0x00, ppuMask: 0x0A);

        Assert.All(batch.OpacityMask, static value => Assert.Equal(0xFF, value));

        // Tile 1 of table $0000 is plane 0 = $0101…01, plane 1 = $FEFE…FE: with the bit order
        // of REQ-CORE-034 its pixels are of value 2 in columns 0 to 6 and 1 in column 7.
        for (int y = 0; y < Height; y++)
        {
            for (int x = 0; x < Width; x++)
            {
                Assert.Equal(batch.Palette[(x % 8) == 7 ? 1 : 2], batch.PixelAt(x, y));
            }
        }
    }

    /// <summary>
    /// V11, second bullet — the case the image alone cannot tell apart: the three palette
    /// colours set to the universal background colour leave a uniform image and a mask still
    /// entirely at 1.
    /// </summary>
    [Fact] // REQ-CORE-048, REQ-CORE-049 (V11)
    public void V11_three_palette_colours_equal_to_the_background_leave_the_mask_at_one()
    {
        NesCore core = V11Core(c => SpecRom.Vram(c, 0x3F00, 0x0F, 0x0F, 0x0F));
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x0A);

        Assert.All(batch.Picture, static index => Assert.Equal(0x0F, index));
        Assert.All(batch.OpacityMask, static value => Assert.Equal(0xFF, value));
    }

    /// <summary>
    /// V11, third bullet: PPUMASK = <c>$08</c> masks the leftmost 8 background pixels, so the
    /// first 8 bits of every row of the mask are 0 and the other 248 are 1.
    /// </summary>
    [Fact] // REQ-CORE-049 (V11)
    public void V11_the_masked_left_column_is_transparent()
    {
        CapturedBatch batch = SpecRom.Capture(V11Core(), ppuCtrl: 0x00, ppuMask: 0x08);

        for (int y = 0; y < Height; y++)
        {
            for (int x = 0; x < Width; x++)
            {
                Assert.Equal(x >= 8, batch.OpaqueAt(x, y));
            }
        }
    }

    /// <summary>
    /// V11, fourth bullet: PPUMASK = <c>$00</c> — forced blanking — gives a mask entirely at 0
    /// and an image uniformly equal to the index of palette entry 0.
    /// </summary>
    [Fact] // REQ-CORE-048, REQ-CORE-049 (V11)
    public void V11_forced_blanking_is_entirely_transparent()
    {
        CapturedBatch batch = SpecRom.Capture(V11Core(), ppuCtrl: 0x00, ppuMask: 0x00);

        Assert.All(batch.OpacityMask, static value => Assert.Equal(0x00, value));
        Assert.All(batch.Picture, index => Assert.Equal(batch.Palette[0], index));
    }

    /// <summary>
    /// V11, fifth bullet: a null background and one 8 × 8 sprite at (Y <c>$20</c>, T
    /// <c>$01</c>, A <c>$00</c>, X <c>$10</c>) mark the rectangle <c>x ∈ [16, 23]</c>,
    /// <c>y ∈ [33, 40]</c> and nothing else.
    /// </summary>
    [Fact] // REQ-CORE-049, REQ-CORE-042 (V11)
    public void V11_a_lone_sprite_marks_its_own_rectangle_only()
    {
        NesCore core = SpecRom.Warm();
        SpecRom.FillOam(core, 0xFF);
        SpecRom.Vram(core, 0x2000, new byte[960]);
        SpecRom.Oam(core, 0, y: 0x20, tile: 0x01, attribute: 0x00, x: 0x10);

        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x1E);

        for (int y = 0; y < Height; y++)
        {
            for (int x = 0; x < Width; x++)
            {
                // y = Y + 1 (REQ-CORE-042).
                bool wanted = x is >= 16 and <= 23 && y is >= 33 and <= 40;
                Assert.Equal(wanted, batch.OpaqueAt(x, y));
            }
        }
    }

    /// <summary>
    /// The invariant the mask exists for: wherever its bit is 1 the PPU drew something, so the
    /// image index differs from entry 0 of the palette — unless the palette is degenerate, the
    /// second bullet of V11.
    /// </summary>
    [Fact] // REQ-CORE-048, REQ-CORE-049
    public void REQ_CORE_049_an_opaque_bit_implies_a_pixel_other_than_entry_0()
    {
        NesCore core = V11Core(c => SpecRom.Vram(c, 0x3F00, 0x0F, 0x16, 0x21));
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x0A);

        int opaque = 0;
        for (int y = 0; y < Height; y++)
        {
            for (int x = 0; x < Width; x++)
            {
                if (!batch.OpaqueAt(x, y))
                {
                    continue;
                }

                opaque++;
                Assert.NotEqual(batch.Palette[0], batch.PixelAt(x, y));
            }
        }

        Assert.Equal(Width * Height, opaque);
    }

    /// <summary>
    /// A core with the minimal ROM of §5.1 warmed up and the set-up of V11 applied: nametable
    /// <c>$2000</c> filled with tile 1, its attribute table cleared so palette table 0 is
    /// selected, and the OAM at <c>$FF</c>.
    /// </summary>
    /// <param name="writes">
    /// Extra PPU writes, made before the scroll and PPUCTRL writes of
    /// <see cref="SpecRom.Capture"/>: a <c>$2006</c> write leaves the nametable bits of
    /// <c>t</c> set and only the <c>$2000</c> write puts them back.
    /// </param>
    private static NesCore V11Core(Action<NesCore>? writes = null)
    {
        NesCore core = SpecRom.Warm();
        SpecRom.FillOam(core, 0xFF);

        byte[] nametable = new byte[960];
        Array.Fill(nametable, (byte)0x01);
        SpecRom.Vram(core, 0x2000, nametable);
        SpecRom.Vram(core, 0x23C0, new byte[64]);
        writes?.Invoke(core);
        return core;
    }
}

/// <summary>
/// AMD-050 REQ-SCN-076 on the GPU: the three automatic criteria of the requirement, each in the
/// conditions the amendment prescribes — <c>scene.use_nes_bg_color</c> false and
/// <c>Setting.CustomBgColor</c> magenta, without which the transparency is not observable.
/// </summary>
/// <remarks>Its own collection, so that two GL contexts are never created at once.</remarks>
[Collection("opengl-acceptance")]
public sealed class Amd050CaptureTests
{
    private const int Width = 256;
    private const int Height = 240;
    private const int MaskSize = Width * Height / 8;

    /// <summary>The magenta <c>#FF00FFFF</c> of the criteria.</summary>
    private static Rgb24 Magenta => new(255, 0, 255);

    /// <summary>The capture size: wide enough for one NES pixel to cover several pixels.</summary>
    private const int CaptureWidth = 1024;

    /// <summary>The capture height.</summary>
    private const int CaptureHeight = 768;

    /// <summary>
    /// First criterion: a synthetic image whose background tile carries two pixels of value 0
    /// and two of value 3, palette entry 3 being equal to entry 0. The two pairs hold the
    /// <b>same</b> colour index, so only the mask tells them apart: the pixels of value 0 show
    /// the scene background (magenta, ±2 sRGB levels) and the pixels of value 3 are drawn with
    /// the universal background colour.
    /// </summary>
    /// <remarks>
    /// The universal background colour is <c>$0F</c>, NES black: the rectangle is lit like a
    /// tile (REQ-SCN-028) and a black albedo comes out black whatever the light, so the
    /// criterion can be read literally. The point 50-N3 of the second R2 review — a non-black
    /// colour would come out <em>lit</em>, not raw — is covered by the value-1 pixels below,
    /// which are compared against <see cref="SceneColor.Shade"/>.
    /// </remarks>
    [GlFact] // REQ-SCN-076, REQ-CORE-049
    public void REQ_SCN_076_a_pixel_of_value_0_shows_the_scene_and_a_pixel_of_value_3_is_drawn()
    {
        byte[] palette = Palette(background: 0x0F, one: 0x21, three: 0x0F);
        byte[] values = new byte[Width * Height];
        Array.Fill(values, (byte)1);

        // The background tile at column 16, row 15 of the picture.
        const int x0 = 16 * 8;
        const int y0 = 15 * 8;
        values[(y0 * Width) + x0 + 0] = 0;
        values[(y0 * Width) + x0 + 1] = 0;
        values[(y0 * Width) + x0 + 2] = 3;
        values[(y0 * Width) + x0 + 3] = 3;

        Compose(values, palette, out byte[] picture, out byte[] mask);

        using GlTarget target = GlTarget.Create(CaptureWidth, CaptureHeight);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = Scene(picture, mask);
        Shot shot = target.Render(renderer, scene);
        CameraView view = renderer.LastView;

        // The two pixels of value 0: the scene behind the rectangle.
        Near(shot, At(shot, view, x0 + 0, y0), Magenta, "the first pixel of value 0");
        Near(shot, At(shot, view, x0 + 1, y0), Magenta, "the second pixel of value 0");

        // The two pixels of value 3: drawn, with the colour of the universal background.
        Rgb24 universal = NesPalette.Color(palette[0]);
        Near(shot, At(shot, view, x0 + 2, y0), universal, "the first pixel of value 3");
        Near(shot, At(shot, view, x0 + 3, y0), universal, "the second pixel of value 3");

        // And the rest of the picture is drawn with its own colour, lit as a tile.
        Rgb24 lit = Lit(NesPalette.Color(palette[1]), scene);
        Near(shot, At(shot, view, x0 + 4, y0), lit, "a pixel of value 1");
        Near(shot, At(shot, view, 128, 128), lit, "a pixel of value 1 away from the tile");
    }

    /// <summary>
    /// Second criterion, first half: bits 3 and 4 of PPUMASK at 0 — forced blanking — leave the
    /// whole rectangle transparent, so the capture is exactly the scene background colour.
    /// </summary>
    [GlFact] // REQ-SCN-076, REQ-CORE-049
    public void REQ_SCN_076_forced_blanking_leaves_nothing_of_the_rectangle()
    {
        CapturedBatch batch = SpecRom.Capture(TiledCore(), ppuCtrl: 0x00, ppuMask: 0x00);

        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        Shot shot = target.Render(renderer, Scene(batch.Picture, batch.OpacityMask));

        for (int y = 0; y < shot.Height; y++)
        {
            for (int x = 0; x < shot.Width; x++)
            {
                Near(shot, new Vector2(x, y), Magenta, $"({x}, {y})");
            }
        }
    }

    /// <summary>
    /// Second criterion, second half: bit 1 of PPUMASK at 0 with the background rendered and a
    /// non-zero tile pattern leaves the 8 <b>left</b> columns transparent and the rest of the
    /// rectangle unchanged; with <c>maskX</c> = 8 (REQ-CORE-046, horizontal mirroring) the 8
    /// right columns and the top and bottom edges stay drawn — the unreliable bands of
    /// <c>maskX</c> and <c>maskY</c> make no pixel transparent.
    /// </summary>
    /// <remarks>
    /// Q-085: with the default clip margins the 8 right columns and the NES rows 0 to 7 fall
    /// outside the visible rectangle of REQ-SCN-036 — the rows always do, whatever the margins
    /// (REQ-E3D-410) — so the criterion cannot be read literally. The conditions below are the
    /// provisional answer: the right margin opened, and the top and bottom edges taken as the
    /// first and last rows R keeps.
    /// </remarks>
    // Q-085
    [GlFact] // REQ-SCN-076, REQ-CORE-046, REQ-CORE-049
    public void REQ_SCN_076_only_the_eight_left_columns_go_with_the_ppumask_bit()
    {
        // Horizontal mirroring, so that maskX is 8 — and maskY 0, which is what REQ-CORE-046
        // gives for that mirroring (point 50-N4 of the second R2 review).
        NesCore core = TiledCore(SpecRom.Mirroring.Horizontal);
        CapturedBatch full = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x0A);
        CapturedBatch cut = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x08);

        Assert.Equal(8, cut.MaskX);
        Assert.Equal(0, cut.MaskY);

        using GlTarget target = GlTarget.Create(CaptureWidth, CaptureHeight);
        using SceneRenderer renderer = new(target.Gl);

        // The right margin is opened so that the 8 right columns fall inside the visible
        // rectangle of REQ-SCN-036; the top band of the picture (NES rows 0 to 7, scene
        // y ∈ [240, 248]) lies above R whatever the margins (REQ-E3D-410), so the top edge the
        // criterion names is the first row R keeps.
        Vec4F clipping = new(0.1f, 8.1f, 0.1f, 0.1f);
        const int top = 8;
        const int bottom = Height - 1;

        SceneState scene = Scene(full.Picture, full.OpacityMask, clipping);
        Shot drawn = target.Render(renderer, scene);
        CameraView view = renderer.LastView;
        Shot shot = target.Render(renderer, Scene(cut.Picture, cut.OpacityMask, clipping));

        // Column 7 of every tile is the pixel of value 1, the others are of value 2; both
        // entries hold the same colour here.
        Rgb24 tile = Lit(NesPalette.Color(full.Palette[2]), scene);
        for (int y = top; y <= bottom; y += 17)
        {
            // The 8 left columns: transparent.
            for (int x = 0; x < 8; x++)
            {
                Near(shot, At(shot, view, x, y), Magenta, $"left column {x}, row {y}");
            }

            // The 8 right columns: still drawn, and unchanged.
            for (int x = Width - 8; x < Width; x++)
            {
                Near(shot, At(shot, view, x, y), tile, $"right column {x}, row {y}");
                Near(drawn, At(drawn, view, x, y), tile, $"right column {x}, row {y}, unmasked");
            }
        }

        // The top and bottom edges: drawn too.
        for (int x = 8; x < Width; x += 19)
        {
            Near(shot, At(shot, view, x, top), tile, $"top edge at {x}");
            Near(shot, At(shot, view, x, bottom), tile, $"bottom edge at {x}");
        }
    }

    /// <summary>
    /// Third criterion: the same scene rendered with <c>video.anti_alias</c> = 0, 1 and 2 draws
    /// exactly the same set of pixels, and no edge of the opaque block comes out partly
    /// transparent — the fragment rejection of REQ-SCN-076 is binary, where an alpha-to-coverage
    /// would have blended the samples.
    /// </summary>
    [GlFact] // REQ-SCN-076, AMD-032 §2
    public void REQ_SCN_076_the_drawn_pixels_do_not_depend_on_the_anti_aliasing()
    {
        byte[] palette = Palette(background: 0x0F, one: 0x21, three: 0x0F);

        // An 8 × 8 tile block of opaque pixels in the middle of a transparent picture: every
        // edge of the drawn set is a transparency edge, not a silhouette of the rectangle.
        byte[] values = new byte[Width * Height];
        for (int y = 11 * 8; y < 19 * 8; y++)
        {
            for (int x = 12 * 8; x < 20 * 8; x++)
            {
                values[(y * Width) + x] = 1;
            }
        }

        Compose(values, palette, out byte[] picture, out byte[] mask);

        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = Scene(picture, mask);
        Rgb24 lit = Lit(NesPalette.Color(palette[1]), scene);

        List<bool[]> drawn = [];
        HashSet<int> granted = [];
        foreach (int option in new[] { 0, 1, 2 })
        {
            scene.Settings.AntiAliasSamples = SceneConstants.AntiAliasSamples(option);
            Shot shot = target.Render(renderer, scene);
            granted.Add(renderer.Statistics.AntiAliasSamples);

            bool[] opaque = new bool[shot.Width * shot.Height];
            for (int y = 0; y < shot.Height; y++)
            {
                for (int x = 0; x < shot.Width; x++)
                {
                    (int r, int g, int b) = shot.At(x, y);
                    bool background = Close(r, Magenta.R) && Close(g, Magenta.G) && Close(b, Magenta.B);
                    bool tile = Close(r, lit.R) && Close(g, lit.G) && Close(b, lit.B);

                    // Neither the one nor the other would be a partly transparent pixel.
                    Assert.True(
                        background ^ tile,
                        $"({x}, {y}) is neither the scene background nor the drawn colour at "
                            + $"{renderer.Statistics.AntiAliasSamples} sample(s): ({r}, {g}, {b}) "
                            + $"instead of ({Magenta.R}, {Magenta.G}, {Magenta.B}) or "
                            + $"({lit.R}, {lit.G}, {lit.B})");
                    opaque[(y * shot.Width) + x] = tile;
                }
            }

            Assert.Contains(true, opaque);
            drawn.Add(opaque);
        }

        Assert.Equal(drawn[0], drawn[1]);
        Assert.Equal(drawn[0], drawn[2]);

        // The comparison would be vacuous if the driver had refused every multisampled target.
        Assert.True(
            granted.Any(samples => samples > 1),
            "no multisampled target was granted, so the criterion checked nothing.");
    }

    /// <summary>
    /// A host that produces no mask at all keeps every pixel of the rectangle: the transparency
    /// is driven by REQ-CORE-049 and by nothing else, never by the colour.
    /// </summary>
    [GlFact] // REQ-SCN-076
    public void REQ_SCN_076_without_a_mask_the_whole_rectangle_is_drawn()
    {
        byte[] picture = new byte[Width * Height];
        Array.Fill(picture, (byte)0x21);

        using GlTarget target = GlTarget.Create(320, 240);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = Scene(picture, []);
        Shot shot = target.Render(renderer, scene);

        Near(shot, new Vector2(shot.Width / 2, shot.Height / 2), Lit(NesPalette.Color(0x21), scene), "the centre");
    }

    // ------------------------------------------------------------ helpers

    /// <summary>The 32 palette bytes of the criteria: entry 3 equal to entry 0.</summary>
    private static byte[] Palette(byte background, byte one, byte three)
    {
        byte[] palette = new byte[32];
        palette[0] = background;
        palette[1] = one;
        palette[2] = one;
        palette[3] = three;
        return palette;
    }

    /// <summary>
    /// Turns a grid of pixel values into the image of REQ-CORE-048 and the mask of
    /// REQ-CORE-049, as the core composes them: the index is that of the palette entry, entry 0
    /// where the value is 0, and the bit is 1 wherever the value is not 0.
    /// </summary>
    private static void Compose(byte[] values, byte[] palette, out byte[] picture, out byte[] mask)
    {
        picture = new byte[values.Length];
        mask = new byte[MaskSize];
        for (int i = 0; i < values.Length; i++)
        {
            byte value = values[i];
            picture[i] = palette[value];
            if (value != 0)
            {
                mask[i >> 3] |= (byte)(1 << (i & 7));
            }
        }
    }

    /// <summary>
    /// The 2D-mode scene of the criteria: a flat rectangle, no shadow, and the conditions the
    /// amendment prescribes — <c>scene.use_nes_bg_color</c> false, a magenta
    /// <c>Setting.CustomBgColor</c>.
    /// </summary>
    private static SceneState Scene(byte[] picture, byte[] mask, Vec4F? clipping = null)
    {
        SceneState scene = new()
        {
            GameRunning = true,
            TwoDimensionalMode = true,
            NesPicture = picture,
            NesOpacityMask = mask,
            UseNesBackgroundColor = false,
        };
        scene.Settings.CustomBackgroundColor = Magenta;
        scene.Settings.Shadows = false;
        if (clipping is { } margins)
        {
            scene.Clipping = margins;
        }

        scene.Camera.SnapToSetpoints();
        return scene;
    }

    /// <summary>The colour a pixel of the flat rectangle comes out at (REQ-SCN-028).</summary>
    private static Rgb24 Lit(Rgb24 albedo, SceneState scene) => SceneColor.Shade(
        albedo,
        new Vector3(0f, 0f, -1f),
        scene.Light.Direction,
        scene.Light.Color,
        scene.Settings.LightIntensity,
        shadow: 1f);

    /// <summary>
    /// Where NES pixel (<paramref name="px"/>, <paramref name="py"/>) lands in the capture. The
    /// quad of REQ-E3D-410 maps it on <c>[px, px + 1] × [247 − py, 248 − py]</c> at
    /// <c>z = 0</c>, and the scene frame is left-handed (REQ-SCN-001).
    /// </summary>
    private static Vector2 At(Shot shot, in CameraView view, int px, int py)
    {
        Vector3 point = new(px + 0.5f, 247.5f - py, 0f);
        Vector4 clip = Vector4.Transform(new Vector3(point.X, point.Y, -point.Z), view.ViewProjection);
        return new Vector2(
            ((clip.X / clip.W) + 1f) * 0.5f * shot.Width,
            (1f - (clip.Y / clip.W)) * 0.5f * shot.Height);
    }

    /// <summary>Asserts that the capture carries <paramref name="wanted"/> ±2 sRGB levels.</summary>
    private static void Near(Shot shot, Vector2 at, Rgb24 wanted, string what)
    {
        int x = Math.Clamp((int)MathF.Round(at.X), 0, shot.Width - 1);
        int y = Math.Clamp((int)MathF.Round(at.Y), 0, shot.Height - 1);
        (int r, int g, int b) = shot.At(x, y);

        Assert.True(
            Close(r, wanted.R) && Close(g, wanted.G) && Close(b, wanted.B),
            $"{what} at ({x}, {y}) is ({r}, {g}, {b}) and not ({wanted.R}, {wanted.G}, {wanted.B}).");
    }

    private static bool Close(int read, int wanted) => Math.Abs(read - wanted) <= 2;

    /// <summary>
    /// A core with the minimal ROM of §5.1 and the nametable filled with tile 1, the set-up of
    /// V11: every pixel of the background is of a value other than 0.
    /// </summary>
    private static NesCore TiledCore(SpecRom.Mirroring mirroring = SpecRom.Mirroring.Vertical)
    {
        NesCore core = SpecRom.Warm(mirroring);
        SpecRom.FillOam(core, 0xFF);

        byte[] nametable = new byte[960];
        Array.Fill(nametable, (byte)0x01);
        SpecRom.Vram(core, 0x2000, nametable);
        SpecRom.Vram(core, 0x23C0, new byte[64]);

        // A palette whose entries 1 and 2 differ from the universal background colour, so that
        // a drawn pixel can be told from a transparent one.
        SpecRom.Vram(core, 0x3F00, 0x0F, 0x21, 0x21);
        return core;
    }
}
