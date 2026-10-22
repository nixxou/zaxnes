using Zax.Profiles;

namespace Zax.Engine3D.Tests;

/// <summary>
/// Intake of a batch (SPEC-04 §3.1), image attributes (§3.2) and the frame model (§3.3).
/// </summary>
public sealed class IntakeAndFrameTests
{
    [Fact]
    public void REQ_E3D_001_a_batch_produces_one_frame_whose_counter_is_the_batch_counter()
    {
        ConversionEngine engine = ProfileBuilder.EmptyEngine();
        Batch batch = new();
        batch.BgCell(Img.A, 10, 10);

        ConversionFrame frame = batch.Convert(engine, 42);

        Assert.Equal(42, frame.Counter);
        Assert.Same(frame, engine.ConvertingFrame);
    }

    [Theory]
    [InlineData(-8, 100, false)]
    [InlineData(-7, 100, true)]
    [InlineData(247, 100, true)]
    [InlineData(248, 100, false)]
    [InlineData(100, 239, true)]
    [InlineData(100, 240, false)]
    [InlineData(100, -7, true)]
    public void REQ_E3D_002_the_clip_area_keeps_a_tile_or_not(int x, int y, bool kept)
    {
        ConversionEngine engine = ProfileBuilder.EmptyEngine();
        Batch batch = new();
        batch.Bg(Img.A, x, y);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Equal(kept ? 1 : 0, frame.BackgroundTileCount);
    }

    [Fact]
    public void REQ_E3D_002_the_clip_margins_come_from_the_settings()
    {
        ConversionEngine engine = ProfileBuilder.EmptyEngine();
        Assert.Equal(ConversionSettings.DefaultClipping, engine.Settings.Clipping);

        // Widening the left margin to 16 drops a tile that the default margins keep.
        engine.Settings.Clipping = new Vec4F(16f, 8.1f, 8.1f, 0.1f);
        Batch batch = new();
        batch.Bg(Img.A, 8, 100);
        Assert.Equal(0, batch.Convert(engine).BackgroundTileCount);
    }

    [Fact]
    public void REQ_E3D_003_a_sprite_at_240_or_empty_is_dropped()
    {
        ConversionEngine engine = ProfileBuilder.EmptyEngine();
        Batch batch = new();
        batch.Sprite(Img.A, 100, 240).Sprite(Img.Empty, 100, 100).Sprite(Img.A, 100, 100);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Single(frame.Sprites);
        Assert.Equal(100, frame.Sprites[0].Y);
    }

    [Fact]
    public void REQ_E3D_004_an_empty_background_tile_depends_on_the_setting()
    {
        ConversionEngine engine = ProfileBuilder.EmptyEngine();
        Batch batch = new();
        batch.BgCell(Img.A, 10, 10).BgCell(Img.Empty, 11, 10).BgCell(Img.A, 12, 10);

        ConversionFrame dropped = batch.Convert(engine);
        Assert.Equal(2, dropped.BackgroundTileCount);

        // REQ-E3D-041: the missing cell separates the two regions.
        Assert.Equal(2, dropped.Shapes.Count);

        engine.Settings.EmptyTile = true;
        Batch second = new();
        second.BgCell(Img.A, 10, 10).BgCell(Img.Empty, 11, 10).BgCell(Img.A, 12, 10);
        ConversionFrame kept = second.Convert(engine, 1);
        Assert.Equal(3, kept.BackgroundTileCount);
    }

    [Fact]
    public void REQ_E3D_004_an_empty_image_can_join_a_shape_when_a_pattern_holds_it()
    {
        ConversionEngine engine = new ProfileBuilder()
            .Row([Img.A, Img.Empty, Img.A])
            .Load();
        engine.Settings.EmptyTile = true;

        Batch batch = new();
        batch.BgRow(10, 10, Img.A, Img.Empty, Img.A);
        ConversionFrame frame = batch.Convert(engine);

        Assert.Single(frame.Shapes);
        Assert.Equal(3, frame.Shapes[0].TileCount);
    }

    [Theory]
    [InlineData(-3, 13, 1, 3)]
    [InlineData(5, 16, 2, 3)]
    public void REQ_E3D_005_a_background_tile_lands_in_its_cell(int x, int y, int cx, int cy)
    {
        Assert.Equal(new Cell(cx, cy), ConversionGrid.BackgroundCell(x, y));

        ConversionEngine engine = ProfileBuilder.EmptyEngine();
        Batch batch = new();
        batch.Bg(Img.A, x, y);
        ConversionFrame frame = batch.Convert(engine);

        FrameTile? tile = frame.BackgroundAt(new Cell(cx, cy));
        Assert.NotNull(tile);
        Assert.Equal(x, tile.X);
        Assert.Equal(y, tile.Y);
    }

    [Fact]
    public void REQ_E3D_005_a_sprite_lands_in_its_cell_without_margin()
    {
        Assert.Equal(new Cell(1, 2), ConversionGrid.SpriteCell(13, 20));

        ConversionEngine engine = ProfileBuilder.EmptyEngine();
        Batch batch = new();
        batch.Sprite(Img.A, 13, 20, palette: 2);
        ConversionFrame frame = batch.Convert(engine);

        Assert.Equal(new Cell(1, 2), frame.Sprites[0].Position);
        Assert.Equal(2, frame.Sprites[0].Palette);
    }

    [Fact]
    public void REQ_E3D_006_the_germination_order_follows_the_scan_direction()
    {
        // Two separate one-tile shapes: the one that germinates first is added first.
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Row([Img.B]).Load();

        Batch batch = new();
        batch.BgCell(Img.A, 20, 2).BgCell(Img.B, 2, 20);

        ConversionFrame byRow = batch.Convert(engine);
        Assert.Equal(2, byRow.Shapes.Count);
        Assert.Equal(Img.A, byRow.Shapes[0].Images[0].Key);

        engine.Settings.ScanDirection = 1;
        Batch again = new();
        again.BgCell(Img.A, 20, 2).BgCell(Img.B, 2, 20);
        ConversionFrame byColumn = again.Convert(engine, 1);
        Assert.Equal(Img.B, byColumn.Shapes[0].Images[0].Key);
    }

    [Fact]
    public void REQ_E3D_006_sprites_germinate_in_emission_order()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Row([Img.B]).Load();
        Batch batch = new();
        batch.Sprite(Img.B, 200, 100).Sprite(Img.A, 10, 100);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Equal(2, frame.Shapes.Count);
        Assert.Equal(Img.B, frame.Shapes[0].Images[0].Key);
    }

    [Fact]
    public void REQ_E3D_007_an_image_keeps_its_identity_for_the_session()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Load();

        Batch first = new();
        first.BgCell(Img.A, 10, 10);
        ConversionFrame frame1 = first.Convert(engine, 1);
        TileImage image = frame1.Shapes[0].Images[0];

        for (int counter = 2; counter <= 1000; counter++)
        {
            Batch batch = new();
            batch.BgCell(Img.A, 10, 10);
            ConversionFrame frame = batch.Convert(engine, counter);
            Assert.Same(image, frame.Shapes[0].Images[0]);
            Assert.Same(engine.Patterns.Permanent[0], frame.Shapes[0].ReferencePattern);
        }

        Assert.Equal(1, engine.Images.Count);
    }

    [Fact]
    public void REQ_E3D_008_the_learned_state_survives_a_relaunch_and_dies_with_the_game()
    {
        GameProfile profileA = new ProfileBuilder().Row([Img.A]).Build();
        ConversionEngine engine = new();
        engine.LoadProfile(profileA);

        Batch learning = new();
        learning.BgCell(Img.U, 10, 10);
        learning.Convert(engine, 1);
        Assert.Equal(2, engine.Patterns.Permanent.Count);

        // Relaunching the same game without reloading the profile keeps what was learned, and
        // the learned pattern is active from the very first frame.
        engine.Restart();
        Assert.Equal(2, engine.Patterns.Permanent.Count);
        Batch again = new();
        again.BgCell(Img.U, 10, 10);
        ConversionFrame first = again.Convert(engine, 1);
        Assert.Single(first.Shapes);
        Assert.Same(engine.Patterns.Permanent[1], first.Shapes[0].ReferencePattern);

        // Loading another game clears everything.
        engine.LoadProfile(new ProfileBuilder().Row([Img.B]).Build());
        Assert.Single(engine.Patterns.Permanent);
        Assert.Equal(1, engine.Images.Count);
    }

    [Fact]
    public void REQ_E3D_020_image_attributes()
    {
        ImageRegistry registry = new();

        TileImage two = registry.Intern(
            Img.Pixel(2, 3).Low | Img.Pixel(5, 6).Low,
            0UL,
            false,
            false);
        Assert.Equal(2, two.PixelCount);
        Assert.Equal(ImageColors.Color1, two.Colors);
        Assert.Equal(new Cell(2, 3), two.PixelStart);
        Assert.Equal(new Cell(5, 6), two.PixelEnd);

        TileImage empty = registry.Intern(Img.Empty);
        Assert.Equal(0, empty.PixelCount);
        Assert.Equal(new Cell(7, 7), empty.PixelStart);
        Assert.Equal(new Cell(0, 0), empty.PixelEnd);
    }

    [Fact]
    public void REQ_E3D_021_density_and_2D_shape_classes()
    {
        ImageRegistry registry = new();

        // A full tile of one colour: SOLID and FULL_RECT.
        TileImage solid = registry.Intern(ulong.MaxValue, 0UL, false, false);
        Assert.Equal(ImageDensity.Solid, solid.Density);
        Assert.Equal(ImageShape2D.FullRect, solid.Shape2D);

        // A monochrome glyph of 20 pixels: AVERAGE and CHAR.
        ulong plane = 0UL;
        for (int i = 0; i < 20; i++)
        {
            plane |= 1UL << i;
        }

        TileImage glyph = registry.Intern(plane, 0UL, false, false);
        Assert.Equal(20, glyph.PixelCount);
        Assert.Equal(ImageDensity.Average, glyph.Density);
        Assert.Equal(ImageShape2D.Char, glyph.Shape2D);
    }

    [Fact]
    public void REQ_E3D_040_the_frame_lists_background_shapes_first()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Row([Img.B]).Load();
        Batch batch = new();
        batch.BgCell(Img.A, 10, 10).Sprite(Img.B, 100, 100);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Equal(2, frame.Shapes.Count);
        Assert.Equal(1, frame.BackgroundShapeCount);
        for (int i = 0; i < frame.BackgroundShapeCount; i++)
        {
            Assert.True(frame.Shapes[i].Background);
        }

        Assert.False(frame.Shapes[frame.BackgroundShapeCount].Background);
    }

    [Fact]
    public void REQ_E3D_042_every_tile_belongs_to_at_most_one_shape()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A, Img.B]).Load();
        Batch batch = Batch.FullFrame(4);

        ConversionFrame frame = batch.Convert(engine);

        int tilesInShapes = frame.Shapes.Sum(static s => s.TileCount);
        Assert.True(tilesInShapes <= frame.BackgroundTileCount + frame.Sprites.Count);

        HashSet<FrameTile> seen = [];
        foreach (FrameShape shape in frame.Shapes)
        {
            foreach (FrameTile tile in shape.Tiles)
            {
                Assert.True(seen.Add(tile));
            }
        }
    }

    [Fact]
    public void REQ_E3D_043_the_frame_time_is_the_counter_over_60_whatever_the_region()
    {
        ConversionEngine engine = ProfileBuilder.EmptyEngine();
        ConversionFrame frame = new Batch().Convert(engine, 600);

        Assert.Equal(600, frame.Counter);
        Assert.Equal(10.0f, frame.Time, 5);
    }

    [Fact]
    public void REQ_E3D_044_shape_boxes_and_centre()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Load();
        Batch batch = new();
        batch.Bg(Img.A, 16, 32).Bg(Img.A, 24, 32);

        ConversionFrame frame = batch.Convert(engine);
        FrameShape shape = frame.Shapes[0];

        Assert.Equal(2, shape.TileCount);
        Assert.Equal(new Cell(16, 32), shape.PixelStart);
        Assert.Equal(new Cell(31, 39), shape.PixelEnd);
        Assert.Equal(new Vec3(24f, 36f, 0f), shape.Center);
    }

    [Fact]
    public void REQ_E3D_045_inside_at_border_big_and_finished()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Load();

        Batch border = new();
        border.Mask(8, 0).Bg(Img.A, 8, 100);
        FrameShape atBorder = border.Convert(engine).Shapes[0];
        Assert.Equal(8, atBorder.PixelStart.X);
        Assert.True(atBorder.AtBorder);
        Assert.False(atBorder.Inside);

        Batch inside = new();
        inside.Mask(8, 0);
        for (int x = 9; x <= 193; x += 8)
        {
            inside.Bg(Img.A, x, 1);
        }

        for (int y = 9; y <= 225; y += 8)
        {
            inside.Bg(Img.A, 9, y);
        }

        FrameShape shape = inside.Convert(engine, 1).Shapes[0];
        Assert.Equal(9, shape.PixelStart.X);
        Assert.Equal(200, shape.PixelEnd.X);
        Assert.Equal(1, shape.PixelStart.Y);
        Assert.Equal(232, shape.PixelEnd.Y);
        Assert.True(shape.Inside);
        Assert.True(shape.IsFinished);
    }

    [Fact]
    public void REQ_E3D_045_a_shape_28_cells_wide_is_big()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Load();
        Batch batch = new();
        for (int column = 0; column < 28; column++)
        {
            batch.BgCell(Img.A, column, 10);
        }

        FrameShape shape = batch.Convert(engine).Shapes[0];
        Assert.Equal(28, shape.TileSize.X);
        Assert.True(shape.IsBig);
        Assert.True(shape.IsFinished);
    }

    [Fact]
    public void REQ_E3D_046_completed_REQ_E3D_063_shape_key()
    {
        ConversionEngine engine = new ProfileBuilder()
            .Row([Img.A, Img.B, Img.C], selfConnected: false)
            .Load();
        Batch batch = new();
        batch.BgRow(10, 10, Img.A, Img.B);

        FrameShape shape = batch.Convert(engine).Shapes[0];

        Assert.Equal(2, shape.Images.Count);
        Assert.Equal(3, shape.ReferencePattern!.Images.Count);
        Assert.False(shape.Completed);
    }

    [Fact]
    public void REQ_E3D_047_the_shape_palette_is_the_germ_attribute_plus_four_for_a_sprite()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Load();

        Batch sprite = new();
        sprite.Sprite(Img.A, 100, 100, palette: 2);
        Assert.Equal(6, sprite.Convert(engine).Shapes[0].Palette);

        Batch background = new();
        background.BgCell(Img.A, 10, 10, palette: 2);
        Assert.Equal(2, background.Convert(engine, 1).Shapes[0].Palette);
    }

    [Fact]
    public void REQ_E3D_048_hidden_is_the_or_of_the_sprite_flags()
    {
        ConversionEngine engine = ProfileBuilder.EmptyEngine();
        Batch batch = new();
        batch.Sprite(Img.A, 100, 100)
            .Sprite(Img.B, 108, 100)
            .Sprite(Img.C, 116, 100, hidden: true)
            .Sprite(Img.D, 124, 100);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Single(frame.Shapes);
        Assert.Equal(4, frame.Shapes[0].TileCount);
        Assert.True(frame.Shapes[0].Hidden);
    }

    [Fact]
    public void REQ_E3D_048_a_background_shape_is_never_hidden()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Load();
        Batch batch = new();
        batch.BgCell(Img.A, 10, 10);

        Assert.False(batch.Convert(engine).Shapes[0].Hidden);
    }
}
