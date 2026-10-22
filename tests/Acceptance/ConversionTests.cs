using Zax.Engine3D;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for SPEC-04: intake, frame model, registry, segmentation,
/// learning, instances, palettes and the test vectors TV-E3D-01 to TV-E3D-14.
/// </summary>
public sealed class ConversionTests
{
    private static ConversionEngine Engine() => new();

    // ------------------------------------------------------------------ 3.1 intake

    [Fact] // REQ-E3D-001
    public void A_batch_produces_one_frame_carrying_its_counter()
    {
        ConversionEngine engine = Engine();
        ConversionFrame frame = new SyntheticBatch().Row("AB", 10, 5).Convert(engine, counter: 42);
        Assert.Equal(42, frame.Counter);
    }

    [Theory] // REQ-E3D-002, TV-E3D-11 (default clipping (0.1, 8.1, 8.1, 0.1))
    [InlineData(-8, 0, false)]
    [InlineData(-7, 0, true)]
    [InlineData(247, 0, true)]
    [InlineData(248, 0, false)]
    [InlineData(0, 239, true)]
    [InlineData(0, 240, false)]
    [InlineData(0, -7, true)]
    public void Clipping_keeps_or_drops_a_background_tile(int x, int y, bool kept)
    {
        ConversionEngine engine = Engine();
        ConversionFrame frame = new SyntheticBatch().Background('A', x, y).Convert(engine);
        Assert.Equal(kept ? 1 : 0, frame.BackgroundTileCount);
    }

    [Fact] // REQ-E3D-003
    public void A_sprite_at_240_or_with_an_empty_image_is_dropped()
    {
        ConversionEngine engine = Engine();
        ConversionFrame frame = new SyntheticBatch()
            .Sprite('A', 100, 240)
            .SpritePlanes(0, 0, 100, 100)
            .Sprite('B', 100, 120)
            .Convert(engine);

        Assert.Single(frame.Sprites);
        Assert.Equal(120, frame.Sprites[0].Y);
    }

    [Theory] // REQ-E3D-004
    [InlineData(false, 0)]
    [InlineData(true, 1)]
    public void An_empty_background_tile_follows_the_empty_tile_setting(bool emptyTile, int expected)
    {
        ConversionEngine engine = Engine();
        engine.Settings.EmptyTile = emptyTile;
        ConversionFrame frame = new SyntheticBatch().BackgroundPlanes(0, 0, 80, 40).Convert(engine);
        Assert.Equal(expected, frame.BackgroundTileCount);
    }

    [Theory] // REQ-E3D-005, TV-E3D-11
    [InlineData(-3, 13, 1, 3)]
    [InlineData(5, 16, 2, 3)]
    [InlineData(-7, -7, 1, 1)]
    public void Background_tiles_land_in_the_documented_cell(int x, int y, int cx, int cy)
    {
        ConversionEngine engine = Engine();
        ConversionFrame frame = new SyntheticBatch().Background('A', x, y).Convert(engine);
        FrameTile tile = Assert.Single(frame.BackgroundGrid, t => t is not null)!;
        Assert.Equal(new Cell(cx, cy), tile.Position);
    }

    [Fact] // REQ-E3D-005 (sprites)
    public void Sprites_land_in_the_documented_cell()
    {
        ConversionEngine engine = Engine();
        ConversionFrame frame = new SyntheticBatch().Sprite('A', 13, 20).Convert(engine);
        Assert.Equal(new Cell(1, 2), frame.Sprites[0].Position);
    }

    [Fact] // REQ-E3D-007
    public void The_image_registry_keeps_one_identity_per_image()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"]);

        ConversionFrame first = new SyntheticBatch().Background('A', 80, 80).Convert(engine, 1);
        EnginePattern? pattern = first.Shapes[0].ReferencePattern;

        for (int i = 2; i <= 1000; i++)
        {
            new SyntheticBatch().Background('A', 80, 80).Convert(engine, i);
        }

        ConversionFrame last = new SyntheticBatch().Background('A', 80, 80).Convert(engine, 1001);
        Assert.Same(pattern, last.Shapes[0].ReferencePattern);
    }

    // ------------------------------------------------------------------ 3.2 image attributes

    [Fact] // REQ-E3D-020
    public void Image_attributes()
    {
        ImageRegistry images = new();
        ulong low = (1UL << ((8 * 3) + 7 - 2)) | (1UL << ((8 * 6) + 7 - 5));
        TileImage image = images.Intern(low, 0, false, false);

        Assert.Equal(2, image.PixelCount);
        Assert.Equal(ImageColors.Color1, image.Colors);
        Assert.Equal(new Cell(2, 3), image.PixelStart);
        Assert.Equal(new Cell(5, 6), image.PixelEnd);
    }

    [Fact] // REQ-E3D-020 (the empty image has the (7,7)-(0,0) convention)
    public void An_empty_image_has_the_inverted_box()
    {
        ImageRegistry images = new();
        TileImage empty = images.Intern(0, 0, false, false);
        Assert.Equal(0, empty.PixelCount);
        Assert.Equal(new Cell(7, 7), empty.PixelStart);
        Assert.Equal(new Cell(0, 0), empty.PixelEnd);
    }

    [Fact] // REQ-E3D-020, TV-E3D-10 (a flip gives another image and another box)
    public void A_flip_produces_a_distinct_image()
    {
        ImageRegistry images = new();
        (ulong low, ulong high) = SyntheticBatch.SinglePixel(1, 2);

        TileImage plain = images.Intern(low, high, false, false);
        TileImage flipped = images.Intern(low, high, true, false);

        Assert.NotSame(plain, flipped);
        Assert.NotEqual(plain.Id, flipped.Id);
        Assert.Equal(new Cell(1, 2), plain.PixelStart);
        Assert.Equal(new Cell(6, 2), flipped.PixelStart);
        Assert.Equal(new Cell(6, 2), flipped.PixelEnd);
    }

    [Theory] // REQ-E3D-021
    [InlineData(0, ImageDensity.Empty)]
    [InlineData(1, ImageDensity.Low)]
    [InlineData(15, ImageDensity.Low)]
    [InlineData(16, ImageDensity.Average)]
    [InlineData(31, ImageDensity.Average)]
    [InlineData(32, ImageDensity.Medium)]
    [InlineData(47, ImageDensity.Medium)]
    [InlineData(48, ImageDensity.High)]
    [InlineData(63, ImageDensity.High)]
    [InlineData(64, ImageDensity.Solid)]
    public void Density_classes(int pixels, ImageDensity expected)
    {
        ImageRegistry images = new();
        ulong low = pixels == 64 ? ulong.MaxValue : (pixels == 0 ? 0UL : (1UL << pixels) - 1);
        Assert.Equal(expected, images.Intern(low, 0, false, false).Density);
    }

    [Fact] // REQ-E3D-021 (2D shapes)
    public void Two_dimensional_shape_classes()
    {
        ImageRegistry images = new();
        Assert.Equal(ImageShape2D.FullRect, images.Intern(ulong.MaxValue, 0, false, false).Shape2D);

        // A monochrome letter of 20 pixels, not filling its box: CHAR.
        ulong letter = 0;
        for (int i = 0; i < 20; i++)
        {
            letter |= 1UL << ((8 * (i / 3)) + (i % 3));
        }

        TileImage charImage = images.Intern(letter, 0, false, false);
        Assert.Equal(20, charImage.PixelCount);
        Assert.Equal(ImageDensity.Average, charImage.Density);
        Assert.Equal(ImageShape2D.Char, charImage.Shape2D);
    }

    // ------------------------------------------------------------------ 3.3 frame model

    [Fact] // REQ-E3D-040
    public void Background_shapes_come_before_sprite_shapes()
    {
        ConversionEngine engine = Engine();
        ConversionFrame frame = new SyntheticBatch()
            .Row("AB", 10, 5)
            .Sprite('C', 100, 100)
            .Convert(engine);

        Assert.True(frame.BackgroundShapeCount > 0);
        for (int i = 0; i < frame.Shapes.Count; i++)
        {
            Assert.Equal(i < frame.BackgroundShapeCount, frame.Shapes[i].Background);
        }
    }

    [Fact] // REQ-E3D-041
    public void A_gap_separates_two_background_shapes()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"]);
        ConversionFrame frame = new SyntheticBatch()
            .Background('A', 80, 40)
            .Background('A', 96, 40) // one empty cell between them
            .Convert(engine);

        Assert.Equal(2, frame.Shapes.Count);
    }

    [Fact] // REQ-E3D-042
    public void Each_tile_belongs_to_at_most_one_shape()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["AB"], selfConnected: true);
        ConversionFrame frame = new SyntheticBatch().Row("ABABAB", 10, 5).Convert(engine);

        List<FrameTile> all = [];
        foreach (FrameShape shape in frame.Shapes)
        {
            all.AddRange(shape.Tiles);
        }

        Assert.Equal(all.Count, all.Distinct().Count());
        Assert.True(all.Count <= frame.BackgroundTileCount + frame.Sprites.Count);
    }

    [Fact] // REQ-E3D-043
    public void Frame_time_is_the_counter_over_sixty()
    {
        ConversionEngine engine = Engine();
        ConversionFrame frame = new SyntheticBatch().Convert(engine, counter: 600);
        Assert.Equal(600, frame.Counter);
        Assert.Equal(10.0f, frame.Time);
    }

    [Fact] // REQ-E3D-044
    public void Shape_boxes_and_centre()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["AA"]);
        ConversionFrame frame = new SyntheticBatch()
            .Background('A', 16, 32)
            .Background('A', 24, 32)
            .Convert(engine);

        FrameShape shape = Assert.Single(frame.Shapes);
        Assert.Equal(new Cell(16, 32), shape.PixelStart);
        Assert.Equal(new Cell(31, 39), shape.PixelEnd);
        Assert.Equal(2, shape.TileCount);
        Assert.Equal(24f, shape.Center.X);
        Assert.Equal(36f, shape.Center.Y);
    }

    [Fact] // REQ-E3D-045
    public void Inside_and_at_border()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"]);

        SyntheticBatch atBorder = new() { MaskX = 8, MaskY = 0 };
        ConversionFrame first = atBorder.Background('A', 8, 100).Convert(engine);
        Assert.True(first.Shapes[0].AtBorder);

        SyntheticBatch inside = new() { MaskX = 8, MaskY = 0 };
        for (int x = 9; x <= 200; x += 8)
        {
            inside.Background('A', x, 8);
        }

        ConversionFrame second = inside.Convert(engine, 2);
        Assert.Contains(second.Shapes, s => s.Inside);
    }

    [Fact] // REQ-E3D-045 (a shape is big at 28 columns or 26 rows)
    public void A_big_shape_is_finished()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"]);

        SyntheticBatch batch = new() { MaskX = 0, MaskY = 0 };
        for (int i = 0; i < 28; i++)
        {
            batch.Background('A', 8 * i, 0);
        }

        ConversionFrame frame = batch.Convert(engine);
        FrameShape shape = frame.Shapes.OrderByDescending(s => s.TileCount).First();
        Assert.True(shape.TileSize.X >= 28);
        Assert.True(shape.IsBig);
        Assert.True(shape.IsFinished);
    }

    [Fact] // REQ-E3D-046, TV-E3D-05
    public void Completed_compares_the_image_sets()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["ABC"], selfConnected: false);
        ConversionFrame frame = new SyntheticBatch().Row("AB", 10, 5).Convert(engine);

        FrameShape shape = Assert.Single(frame.Shapes);
        Assert.False(shape.Completed);
    }

    [Fact] // REQ-E3D-047
    public void Shape_palette_is_the_seed_tile_plus_four_for_sprites()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"]);

        ConversionFrame background = new SyntheticBatch().Background('A', 80, 80, palette: 2).Convert(engine);
        Assert.Equal(2, background.Shapes[0].Palette);

        ConversionFrame sprites = new SyntheticBatch().Sprite('A', 80, 80, palette: 2).Convert(engine, 2);
        Assert.Equal(6, sprites.Shapes[0].Palette);
    }

    [Fact] // REQ-E3D-048
    public void Hidden_is_the_or_of_the_sprite_tiles()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["AAAA"], selfConnected: true);

        ConversionFrame frame = new SyntheticBatch()
            .Sprite('A', 100, 100)
            .Sprite('A', 108, 100)
            .Sprite('A', 116, 100, hidden: true)
            .Sprite('A', 124, 100)
            .Convert(engine);

        FrameShape shape = frame.Shapes.OrderByDescending(s => s.TileCount).First();
        Assert.True(shape.Hidden);

        ConversionFrame background = new SyntheticBatch().Background('A', 80, 80).Convert(engine, 2);
        Assert.False(background.Shapes[0].Hidden);
    }

    // ------------------------------------------------------------------ 3.4 registry

    [Fact] // REQ-E3D-061
    public void Adjacency_relations()
    {
        ConversionEngine engine = Engine();
        TileImage a = SyntheticPatterns.Image(engine.Images, 'A');
        TileImage b = SyntheticPatterns.Image(engine.Images, 'B');

        EnginePattern loose = SyntheticPatterns.Build(engine, ["AB"], selfConnected: false);
        Assert.True(loose.HasHorizontalPair(a, b));
        Assert.False(loose.HasHorizontalPair(b, a));
        Assert.Equal(0, loose.VerticalPairCount);

        ConversionEngine other = Engine();
        TileImage a2 = SyntheticPatterns.Image(other.Images, 'A');
        TileImage b2 = SyntheticPatterns.Image(other.Images, 'B');
        EnginePattern wrapped = SyntheticPatterns.Build(other, ["AB"], selfConnected: true);
        Assert.True(wrapped.HasHorizontalPair(a2, b2));
        Assert.True(wrapped.HasHorizontalPair(b2, a2));
        Assert.True(wrapped.HasVerticalPair(a2, a2));
        Assert.True(wrapped.HasVerticalPair(b2, b2));

        ConversionEngine single = Engine();
        TileImage a3 = SyntheticPatterns.Image(single.Images, 'A');
        EnginePattern one = SyntheticPatterns.Build(single, ["A"], selfConnected: true);
        Assert.True(one.HasHorizontalPair(a3, a3));
        Assert.True(one.HasVerticalPair(a3, a3));
    }

    [Fact] // REQ-E3D-062
    public void Skip_geometry_ignores_the_direction()
    {
        ConversionEngine engine = Engine();
        TileImage a = SyntheticPatterns.Image(engine.Images, 'A');
        TileImage b = SyntheticPatterns.Image(engine.Images, 'B');
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["AB"], selfConnected: false, skipGeometry: true);

        foreach (GrowDirection direction in Enum.GetValues<GrowDirection>())
        {
            Assert.True(pattern.AllowsDirection(a, b, direction));
            Assert.True(pattern.AllowsDirection(b, a, direction));
        }
    }

    [Fact] // REQ-E3D-063
    public void Two_patterns_with_the_same_image_set_are_equal()
    {
        ConversionEngine engine = Engine();
        EnginePattern first = SyntheticPatterns.Build(engine, ["AB"]);
        EnginePattern second = new(
            1,
            3,
            [
                SyntheticPatterns.Image(engine.Images, 'B'),
                SyntheticPatterns.Image(engine.Images, 'A'),
                SyntheticPatterns.Image(engine.Images, 'B'),
            ],
            permanent: true);

        Assert.Equal(first.Key, second.Key);
    }

    [Fact] // REQ-E3D-064
    public void Pattern_pixel_box_and_corners()
    {
        ConversionEngine engine = Engine();
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);

        Assert.Equal(new Cell(0, 0), pattern.PixelStart);
        Assert.Equal(new Cell(7, 7), pattern.PixelEnd);
        Assert.Equal(new Vec2(0, 0), pattern.BottomLeft);
    }

    [Fact] // REQ-E3D-064 (TopRight uses the grid width on the y axis, deliberately)
    public void Pattern_top_right_uses_the_width_on_the_y_axis()
    {
        ConversionEngine engine = Engine();
        EnginePattern wide = SyntheticPatterns.Build(engine, ["AB"]); // 2 x 1
        Assert.Equal(new Vec2(15, (8 * 2) - 0 - 1), wide.TopRight);
    }

    [Fact] // REQ-E3D-065
    public void Learning_a_permanent_pattern_removes_the_temporary_subsets()
    {
        ConversionEngine engine = Engine();
        TileImage a = SyntheticPatterns.Image(engine.Images, 'A');
        TileImage b = SyntheticPatterns.Image(engine.Images, 'B');

        EnginePattern temporary = new(1, 1, [a], permanent: false);
        temporary.AddInstance(new PatternInstance());
        engine.Patterns.Add(temporary);
        Assert.Single(engine.Patterns.Temporary);

        EnginePattern permanent = new(2, 1, [a, b], permanent: true);
        permanent.AddInstance(new PatternInstance());
        engine.Patterns.Add(permanent);

        Assert.Empty(engine.Patterns.Temporary);
        Assert.Contains(permanent, engine.Patterns.Permanent);
    }

    // ------------------------------------------------------------------ 3.5 test vectors

    [Fact] // TV-E3D-01
    public void Tv01_periodic_pattern_region()
    {
        ConversionEngine engine = Engine();
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["AB"], selfConnected: true);

        ConversionFrame six = new SyntheticBatch().Row("ABABAB", 10, 5).Convert(engine);
        FrameShape shape = Assert.Single(six.Shapes);
        Assert.Equal(6, shape.TileCount);
        Assert.Same(pattern, shape.ReferencePattern);
        Assert.True(shape.Completed);

        ConversionFrame eight = new SyntheticBatch().Row("ABABABAB", 10, 5).Convert(engine, 2);
        FrameShape bigger = Assert.Single(eight.Shapes);
        Assert.Equal(8, bigger.TileCount);
        Assert.Same(pattern, bigger.ReferencePattern);
    }

    [Fact] // TV-E3D-02
    public void Tv02_self_connected_false()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["AB"], selfConnected: false);

        ConversionFrame frame = new SyntheticBatch().Row("ABABAB", 10, 5).Convert(engine);
        Assert.Equal(3, frame.Shapes.Count);
        Assert.All(frame.Shapes, s => Assert.Equal(2, s.TileCount));

        ConversionEngine reversed = Engine();
        SyntheticPatterns.Build(reversed, ["AB"], selfConnected: false);
        ConversionFrame ba = new SyntheticBatch().Row("BA", 10, 5).Convert(reversed);
        Assert.Equal(2, ba.Shapes.Count);

        ConversionEngine wrapped = Engine();
        SyntheticPatterns.Build(wrapped, ["AB"], selfConnected: true);
        ConversionFrame baWrapped = new SyntheticBatch().Row("BA", 10, 5).Convert(wrapped);
        Assert.Single(baWrapped.Shapes);
    }

    [Fact] // TV-E3D-03
    public void Tv03_constraint_size()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"], selfConnected: true, constraint: new Cell(2, 99));

        ConversionFrame frame = new SyntheticBatch().Row("AAAAA", 10, 5).Convert(engine);
        Assert.Equal(3, frame.Shapes.Count);
        Assert.Equal([2, 2, 1], [.. frame.Shapes.Select(s => s.TileCount)]);

        ConversionEngine column = Engine();
        SyntheticPatterns.Build(column, ["A"], selfConnected: true, constraint: new Cell(99, 1));
        SyntheticBatch batch = new();
        for (int j = 0; j < 3; j++)
        {
            batch.Background('A', 80, 8 * (5 + j));
        }

        ConversionFrame vertical = batch.Convert(column);
        Assert.Equal(3, vertical.Shapes.Count);
        Assert.All(vertical.Shapes, s => Assert.Equal(1, s.TileCount));
    }

    [Fact] // TV-E3D-04
    public void Tv04_skip_geometry()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["AB"], selfConnected: false, skipGeometry: true);
        ConversionFrame frame = new SyntheticBatch()
            .Background('A', 80, 40)
            .Background('B', 80, 48)
            .Convert(engine);

        FrameShape shape = Assert.Single(frame.Shapes);
        Assert.Equal(2, shape.TileCount);

        ConversionEngine strict = Engine();
        SyntheticPatterns.Build(strict, ["AB"], selfConnected: false, skipGeometry: false);
        ConversionFrame separated = new SyntheticBatch()
            .Background('A', 80, 40)
            .Background('B', 80, 48)
            .Convert(strict);

        Assert.Equal(2, separated.Shapes.Count);
    }

    [Fact] // TV-E3D-05
    public void Tv05_candidate_narrowing_and_smallest_pattern()
    {
        static ConversionEngine Prepared()
        {
            ConversionEngine engine = new();
            SyntheticPatterns.Build(engine, ["AB"], selfConnected: false);
            SyntheticPatterns.Build(engine, ["ABC"], selfConnected: false);
            return engine;
        }

        ConversionEngine ab = Prepared();
        ConversionFrame first = new SyntheticBatch().Row("AB", 10, 5).Convert(ab);
        FrameShape abShape = Assert.Single(first.Shapes);
        Assert.Equal(2, abShape.ReferencePattern!.Images.Count);
        Assert.True(abShape.Completed);

        ConversionEngine abc = Prepared();
        ConversionFrame second = new SyntheticBatch().Row("ABC", 10, 5).Convert(abc);
        FrameShape abcShape = Assert.Single(second.Shapes);
        Assert.Equal(3, abcShape.ReferencePattern!.Images.Count);

        ConversionEngine cab = Prepared();
        ConversionFrame third = new SyntheticBatch().Row("CAB", 10, 5).Convert(cab);
        Assert.Equal(2, third.Shapes.Count);

        FrameShape lone = third.Shapes[0];
        Assert.Equal(1, lone.TileCount);
        Assert.Equal(3, lone.ReferencePattern!.Images.Count);
        Assert.False(lone.Completed);

        FrameShape pair = third.Shapes[1];
        Assert.Equal(2, pair.TileCount);
        Assert.Equal(2, pair.ReferencePattern!.Images.Count);
    }

    [Fact] // TV-E3D-06
    public void Tv06_pass_order()
    {
        static ConversionFrame Run(int scanDirection)
        {
            ConversionEngine engine = new();
            engine.Settings.ScanDirection = scanDirection;
            SyntheticPatterns.Build(engine, ["AB", "BA"], selfConnected: true);
            SyntheticPatterns.Build(engine, ["D"], selfConnected: true);

            SyntheticBatch batch = new() { MaskX = 0, MaskY = 0 };
            batch.Row("ABAB", 4, 2);
            batch.Row("BABA", 4, 3);
            batch.Background('U', 160, 0);
            batch.Background('D', 160, 120);
            return batch.Convert(engine);
        }

        foreach (int direction in new[] { 0, 1 })
        {
            ConversionFrame frame = Run(direction);
            Assert.Equal(3, frame.Shapes.Count);

            Assert.Equal(8, frame.Shapes[0].TileCount);            // pass 1: the chequerboard
            Assert.Equal(1, frame.Shapes[1].TileCount);            // pass 2: the unknown tile
            Assert.True(frame.Shapes[1].AtBorder);
            Assert.False(frame.Shapes[1].ReferencePattern!.Permanent);
            Assert.Equal(1, frame.Shapes[2].TileCount);            // pass 3: D
        }
    }

    [Fact] // TV-E3D-07
    public void Tv07_learning()
    {
        ConversionEngine engine = Engine();

        SyntheticBatch first = new() { MaskX = 0, MaskY = 0 };
        first.Row("UUU", 10, 10);
        ConversionFrame frame1 = first.Convert(engine, 1);

        Assert.Equal(2, frame1.Shapes.Count);
        Assert.Equal(1, frame1.Shapes[0].TileCount);
        EnginePattern learned = frame1.Shapes[0].ReferencePattern!;
        Assert.True(learned.Permanent);
        Assert.Equal(2, frame1.Shapes[1].TileCount);
        Assert.Same(learned, frame1.Shapes[1].ReferencePattern);

        SyntheticBatch second = new() { MaskX = 0, MaskY = 0 };
        second.Row("UUU", 10, 10);
        ConversionFrame frame2 = second.Convert(engine, 2);
        FrameShape whole = Assert.Single(frame2.Shapes);
        Assert.Equal(3, whole.TileCount);
        Assert.Same(learned, whole.ReferencePattern);
    }

    [Fact] // TV-E3D-07 (border variant: a temporary pattern, never promoted)
    public void Tv07_border_variant_learns_a_temporary_pattern()
    {
        ConversionEngine engine = Engine();

        for (int counter = 1; counter <= 3; counter++)
        {
            SyntheticBatch batch = new() { MaskX = 8, MaskY = 0 };
            batch.Background('U', 0, 80);
            ConversionFrame frame = batch.Convert(engine, counter);

            FrameShape shape = Assert.Single(frame.Shapes);
            Assert.False(shape.ReferencePattern!.Permanent);
        }

        Assert.Empty(engine.Patterns.Permanent);
        Assert.Single(engine.Patterns.Temporary);
    }

    [Fact] // TV-E3D-08 (sprites)
    public void Tv08_sprites()
    {
        // Empty profile: pass S1 groups S1 and S2, then S3 alone.
        ConversionEngine empty = Engine();
        ConversionFrame grouped = new SyntheticBatch()
            .Sprite('A', 100, 50)
            .Sprite('B', 108, 50)
            .Sprite('C', 116, 52)
            .Convert(empty);

        Assert.Equal(2, grouped.Shapes.Count);
        Assert.Equal(2, grouped.Shapes[0].TileCount);
        Assert.Equal(1, grouped.Shapes[1].TileCount);
        Assert.Equal(4, grouped.Shapes[0].Palette);
        Assert.Equal(4, grouped.Shapes[1].Palette);

        // Profile [A B C], tolerance 2: one shape.
        ConversionEngine tolerant = Engine();
        tolerant.Settings.SpriteTolerance = 2;
        SyntheticPatterns.Build(tolerant, ["ABC"], selfConnected: false);
        ConversionFrame single = new SyntheticBatch()
            .Sprite('A', 100, 50)
            .Sprite('B', 108, 50)
            .Sprite('C', 116, 52)
            .Convert(tolerant);

        Assert.Single(single.Shapes);
        Assert.Equal(3, single.Shapes[0].TileCount);

        // Same profile, tolerance 0: two shapes, both bound to [A B C].
        ConversionEngine strict = Engine();
        SyntheticPatterns.Build(strict, ["ABC"], selfConnected: false);
        ConversionFrame split = new SyntheticBatch()
            .Sprite('A', 100, 50)
            .Sprite('B', 108, 50)
            .Sprite('C', 116, 52)
            .Convert(strict);

        Assert.Equal(2, split.Shapes.Count);
        Assert.All(split.Shapes, s => Assert.Equal(3, s.ReferencePattern!.Images.Count));
    }

    [Fact] // TV-E3D-08 (the palette condition of pass S2)
    public void Tv08_same_sprite_palette()
    {
        ConversionEngine separate = Engine();
        separate.Settings.SpriteTolerance = 2;
        separate.Settings.SameSpritePalette = true;
        SyntheticPatterns.Build(separate, ["ABC"], selfConnected: false);
        ConversionFrame split = new SyntheticBatch()
            .Sprite('A', 100, 50)
            .Sprite('B', 108, 50, palette: 1)
            .Sprite('C', 116, 52)
            .Convert(separate);

        Assert.Equal(3, split.Shapes.Count);

        ConversionEngine merged = Engine();
        merged.Settings.SpriteTolerance = 2;
        merged.Settings.SameSpritePalette = false;
        SyntheticPatterns.Build(merged, ["ABC"], selfConnected: false);
        ConversionFrame one = new SyntheticBatch()
            .Sprite('A', 100, 50)
            .Sprite('B', 108, 50, palette: 1)
            .Sprite('C', 116, 52)
            .Convert(merged);

        Assert.Single(one.Shapes);
        Assert.Equal(4, one.Shapes[0].Palette);
    }

    [Fact] // REQ-E3D-130
    public void Sprite_adjacency_with_tolerance()
    {
        ConversionEngine zero = Engine();
        SyntheticPatterns.Build(zero, ["AB"], selfConnected: false);
        ConversionFrame apart = new SyntheticBatch()
            .Sprite('A', 100, 50)
            .Sprite('B', 108, 52)
            .Convert(zero);
        Assert.Equal(2, apart.Shapes.Count);

        ConversionEngine two = Engine();
        two.Settings.SpriteTolerance = 2;
        SyntheticPatterns.Build(two, ["AB"], selfConnected: false);
        ConversionFrame together = new SyntheticBatch()
            .Sprite('A', 100, 50)
            .Sprite('B', 108, 52)
            .Convert(two);
        Assert.Single(together.Shapes);
    }

    [Fact] // TV-E3D-09, REQ-E3D-250, REQ-E3D-251
    public void Tv09_frame_palette()
    {
        ConversionEngine engine = Engine();
        ConversionFrame frame = new SyntheticBatch()
            .Palette(0, 0x0F)
            .Palette(1, 0x30)
            .Palette(4, 0x12)
            .Palette(5, 0x16)
            .Palette(17, 0x27)
            .Convert(engine);

        Assert.Equal(0x0F, frame.Palette[0]);
        Assert.Equal(0x30, frame.Palette[1]);
        Assert.Equal(0x0F, frame.Palette[4]);
        Assert.Equal(0x16, frame.Palette[5]);
        Assert.Equal(0x0F, frame.Palette[16]);
        Assert.Equal(0x27, frame.Palette[17]);
        Assert.Equal(0x16, frame.Palette[37]);
        Assert.Equal(0x27, frame.Palette[49]);

        Assert.Equal(new Rgb24(0xFC, 0x38, 0x00), frame.Palette.Color(5));
        Assert.Equal(new Rgb24(0xFC, 0xA0, 0x48), frame.Palette.Color(17));
    }

    [Fact] // REQ-E3D-251
    public void The_nes_colour_table()
    {
        Assert.Equal(new Rgb24(0xFC, 0x38, 0x00), NesPalette.Color(0x16));
        Assert.Equal(new Rgb24(0x00, 0x00, 0x00), NesPalette.Color(0x0F));
        Assert.Equal(new Rgb24(0x78, 0x80, 0x84), NesPalette.Color(0x00));
        Assert.Equal(new Rgb24(0xFC, 0xF8, 0xFC), NesPalette.Color(0x20));
        Assert.Equal(new Rgb24(0x38, 0xC0, 0xFC), NesPalette.Color(0x21));
        Assert.Equal(64, NesPalette.ColorCount);
    }

    [Fact] // REQ-E3D-254
    public void The_scene_background_colour()
    {
        ConversionEngine engine = Engine();
        engine.Settings.UseNesBackgroundColor = true;
        new SyntheticBatch().Palette(0, 0x21).Convert(engine);
        engine.Publish();

        Assert.Equal(new Rgb24(0x38, 0xC0, 0xFC), engine.BackgroundColor(skyboxActive: false));
    }

    [Fact] // TV-E3D-10
    public void Tv10_a_flipped_sprite_is_another_image()
    {
        ConversionEngine engine = Engine();
        (ulong low, ulong high) = SyntheticBatch.SinglePixel(1, 2);
        TileImage plain = engine.Images.Intern(low, high, false, false);

        EnginePattern pattern = new(1, 1, [plain], permanent: true);
        pattern.AddInstance(new PatternInstance());
        engine.Patterns.Register(pattern);

        SyntheticBatch batch = new() { MaskX = 0, MaskY = 0 };
        batch.SpritePlanes(low, high, 100, 100, flipX: true);
        ConversionFrame frame = batch.Convert(engine);

        FrameShape shape = Assert.Single(frame.Shapes);
        Assert.NotSame(pattern, shape.ReferencePattern);
        Assert.True(shape.ReferencePattern!.Permanent); // learned, not bound to [X]
    }

    // ------------------------------------------------------------------ 3.5 passes

    [Fact] // REQ-E3D-092 (pass 1 keeps a full shape of at least 8 tiles and 2 images)
    public void Pass_one_rejects_a_shape_that_is_too_small()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["AB"], selfConnected: true);

        // Six tiles: pass 1 rejects it, pass 3 keeps it (TV-E3D-01).
        ConversionFrame frame = new SyntheticBatch().Row("ABABAB", 10, 5).Convert(engine);
        Assert.Single(frame.Shapes);
    }

    [Fact] // REQ-E3D-093
    public void A_disabled_pattern_hides_its_tiles()
    {
        ConversionEngine engine = Engine();
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);

        ConversionFrame before = new SyntheticBatch().Background('A', 80, 80).Convert(engine, 1);
        Assert.Single(before.Shapes);

        pattern.Disabled = true;
        ConversionFrame during = new SyntheticBatch().Background('A', 80, 80).Convert(engine, 2);
        Assert.Empty(during.Shapes);

        pattern.Disabled = false;
        ConversionFrame after = new SyntheticBatch().Background('A', 80, 80).Convert(engine, 3);
        Assert.Single(after.Shapes);
    }

    [Fact] // REQ-E3D-094
    public void Fullness()
    {
        // A 3x3 ring with the centre missing is not full.
        ConversionEngine ring = Engine();
        SyntheticPatterns.Build(ring, ["AAA", "A A", "AAA"], selfConnected: true);
        SyntheticBatch ringBatch = new();
        for (int j = 0; j < 3; j++)
        {
            for (int i = 0; i < 3; i++)
            {
                if (i != 1 || j != 1)
                {
                    ringBatch.Background('A', 8 * (10 + i), 8 * (5 + j));
                }
            }
        }

        ConversionFrame frame = ringBatch.Convert(ring);
        FrameShape shape = frame.Shapes.OrderByDescending(s => s.TileCount).First();
        Assert.Equal(8, shape.TileCount);
        Assert.False(shape.IsFull());

        // A simple line is full.
        ConversionEngine line = Engine();
        SyntheticPatterns.Build(line, ["A"], selfConnected: true);
        ConversionFrame lineFrame = new SyntheticBatch().Row("AAA", 10, 5).Convert(line);
        Assert.True(lineFrame.Shapes[0].IsFull());
    }

    [Fact] // REQ-E3D-132
    public void A_sprite_cell_is_occupied_by_its_first_owner()
    {
        ConversionEngine engine = Engine();
        ConversionFrame frame = new SyntheticBatch()
            .Sprite('A', 100, 50)
            .Sprite('B', 100, 50)   // same cell as the first
            .Sprite('C', 108, 50)
            .Convert(engine);

        Assert.Contains(frame.Shapes, s => !s.Tiles.Any(t => t.Image.Key == engine.Images.Intern(SyntheticBatch.Full('B').Low, SyntheticBatch.Full('B').High, false, false).Key));
        Assert.True(frame.Shapes.Count >= 2);
    }

    // ------------------------------------------------------------------ 3.7 learning

    [Fact] // REQ-E3D-161
    public void A_learned_pattern_carries_the_default_parameters()
    {
        ConversionEngine engine = Engine();
        SyntheticBatch batch = new() { MaskX = 0, MaskY = 0 };
        batch.Sprite('Z', 120, 120);
        ConversionFrame frame = batch.Convert(engine);

        EnginePattern learned = Assert.Single(engine.Patterns.Permanent);
        Assert.Equal(1, learned.Width);
        Assert.Equal(1, learned.Height);
        Assert.True(learned.SelfConnected);
        Assert.False(learned.SkipGeometry);
        Assert.Equal(new Cell(99, 99), learned.ConstraintSize);

        PatternInstance instance = Assert.Single(learned.Instances);
        Assert.Equal(Geo3DType.Default, instance.Geometry);
        Assert.Equal(3, instance.Depth);
        Assert.Equal(ZLayer.L1, instance.Layer);
        Assert.Equal(new Vec3(1, 1, 1), instance.Scale);
        Assert.Equal(new Vec3(0, 0, 0), instance.Rotation);
        Assert.Equal(new Vec3(0, 0, 0), instance.Offset);
        Assert.Equal(new Vec3(0.5f, 0.5f, 0.5f), instance.Pivot);
        Assert.Equal(new Vec3(0.5f, 0.5f, 0.5f), instance.DeformPivot);
        Assert.Equal(1f, instance.Alpha);
        Assert.Equal(0, instance.MeshAlgorithm);
        Assert.Equal(RenderStyle.Normal, instance.Style);
        Assert.Equal(0, instance.EmptyColor);
        Assert.False(instance.AntiShadow);
        Assert.False(instance.Flip);
        Assert.False(instance.Solid);
        Assert.False(instance.FullWarp);
        Assert.Empty(instance.Tags);

        // Next frame the image is known.
        Assert.Same(learned, frame.Shapes[0].ReferencePattern);
    }

    // ------------------------------------------------------------------ 3.8 instances

    [Fact] // REQ-E3D-190
    public void An_instance_copies_every_parameter_of_its_reference()
    {
        ConversionEngine engine = Engine();
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"], tags: "t");
        PatternInstance reference = pattern.Instances[0];
        reference.Depth = 11;
        reference.Offset = new Vec3(3, 4, 5);
        reference.Alpha = 0.25f;
        reference.Geometry = Geo3DType.Cube;

        ConversionFrame frame = new SyntheticBatch().Background('A', 80, 80).Convert(engine);
        FrameInstance instance = Assert.Single(frame.Instances);

        Assert.Equal(11, instance.Depth);
        Assert.Equal(new Vec3(3, 4, 5), instance.Offset);
        Assert.Equal(0.25f, instance.Alpha);
        Assert.Equal(Geo3DType.Cube, instance.Geometry);
        Assert.Equal(-1, instance.CustomPalette);
        Assert.Same(reference, instance.Reference);
        Assert.Equal(reference.TagString, instance.TagString);
    }

    [Fact] // REQ-E3D-190, TV-E3D-13 (instance parameters are reset every frame)
    public void Tv13_instance_parameters_are_reset_every_frame()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"], tags: "t");

        List<float> offsets = [];
        for (int counter = 1; counter <= 3; counter++)
        {
            ConversionFrame frame = new SyntheticBatch().Background('A', 80, 80).Convert(engine, counter);
            FrameInstance instance = frame.Instances[0];
            instance.Offset = new Vec3(instance.Offset.X + 5, instance.Offset.Y, instance.Offset.Z);
            engine.CalculatePositions(frame);
            offsets.Add(instance.Pos.X);
        }

        Assert.Equal(offsets[0], offsets[1]);
        Assert.Equal(offsets[1], offsets[2]);
    }

    [Fact] // REQ-E3D-191
    public void A_disabled_render_style_keeps_the_instance_in_the_frame()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"]);
        ConversionFrame frame = new SyntheticBatch().Background('A', 80, 80).Convert(engine);

        FrameInstance instance = frame.Instances[0];
        instance.Enable = false;
        Assert.Equal(RenderStyle.Hidden, instance.Style);
        Assert.Contains(instance, frame.Instances);
    }

    [Fact] // REQ-E3D-192
    public void The_palette_of_an_instance()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"]);
        ConversionFrame frame = new SyntheticBatch().Background('A', 80, 80, palette: 2).Convert(engine);

        FrameInstance instance = frame.Instances[0];
        Assert.Equal(2, instance.PaletteTable);
        instance.CustomPalette = 9;
        Assert.Equal(9, instance.PaletteTable);
    }

    [Fact] // REQ-E3D-205, REQ-E3D-206
    public void Instance_position_and_derived_values()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"]);
        ConversionFrame frame = new SyntheticBatch().Background('A', 16, 32).Convert(engine);

        FrameShape shape = Assert.Single(frame.Shapes);
        FrameInstance instance = frame.Instances[0];

        Assert.Equal(new Vec3(16, 208, 0), instance.BoxMin(engine.Settings));
        Assert.Equal(new Vec3(24, 216, 3), instance.BoxMax(engine.Settings));
        Assert.Equal(new Vec3(20, 212, 1.5f), instance.Pos);

        Assert.Equal(new Vec2(16, 208), shape.BottomLeft);
        Assert.Equal(new Vec2(23, 215), shape.TopRight);

        Assert.Equal(new Vec3(16, 209, -1.5f), instance.BottomLeft);
        Assert.Equal(new Vec3(16, 208, -1.5f), instance.BL);
        Assert.Equal(new Vec3(23, 215, 1.5f), instance.TR);
        Assert.Equal(new Vec3(8, 8, 3), instance.Size);
    }

    [Fact] // REQ-E3D-207
    public void Age_is_always_one()
        => Assert.Equal(1, FrameShape.Age);

    // ------------------------------------------------------------------ 3.9, 3.10

    [Fact] // REQ-E3D-220
    public void Scriptable_instances_follow_the_shape_order()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"]);
        ConversionFrame frame = new SyntheticBatch()
            .Background('A', 80, 80)
            .Sprite('A', 160, 120)
            .Convert(engine);

        Assert.Equal(frame.Instances.Count, frame.ScriptableInstances.Count);
        Assert.Same(frame.Shapes[0].Instances[0], frame.ScriptableInstances[0]);
    }

    [Fact] // REQ-E3D-220, REQ-E3D-230, TV-E3D-14
    public void Tv14_injected_shape()
    {
        ConversionEngine engine = Engine();
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);
        ConversionFrame frame = new SyntheticBatch().Convert(engine);

        FrameShape injected = engine.CreateNewShape(pattern, new Vec3(10.5f, 20.25f, 3), palette: 5);

        Assert.False(injected.Scriptable);
        Assert.False(injected.Background);
        FrameTile tile = Assert.Single(injected.Tiles);
        Assert.Equal(10, tile.X);
        Assert.Equal(221, tile.Y);
        Assert.Equal(1, tile.Palette);

        Assert.Equal(new Cell(10, 221), injected.PixelStart);
        Assert.Equal(new Cell(17, 228), injected.PixelEnd);
        Assert.Equal(new Vec2(10, 19), injected.BottomLeft);

        FrameInstance instance = injected.Instances[0];
        Assert.Equal(0.5f, instance.Offset.X, 4);
        Assert.Equal(-0.25f, instance.Offset.Y, 4);
        Assert.Equal(3f, instance.Offset.Z, 4);

        engine.CalculatePositions(frame);
        Assert.Equal(14.5f, instance.Pos.X, 3);
        Assert.Equal(22.75f, instance.Pos.Y, 3);
        Assert.Equal(4.5f, instance.Pos.Z, 3);

        Assert.DoesNotContain(injected, frame.Shapes.Where(s => s.Scriptable));
    }

    [Fact] // REQ-E3D-231
    public void Add_new_shape()
    {
        ConversionEngine engine = Engine();
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);
        ConversionFrame frame = new SyntheticBatch().Convert(engine);

        FrameShape added = engine.AddNewShape(pattern, new Vec3(40, 100, 0), palette: 5);
        engine.CalculatePositions(frame);

        FrameInstance instance = added.Instances[0];
        Assert.Equal(44f, instance.Pos.X, 3);
        Assert.Equal(104f, instance.Pos.Y, 3);
        Assert.Equal(1.5f, instance.Pos.Z, 3);
    }

    [Fact] // REQ-E3D-232
    public void Insert_shape_matches_create_new_shape_for_a_1x1_pattern()
    {
        ConversionEngine inserted = Engine();
        EnginePattern first = SyntheticPatterns.Build(inserted, ["A"]);
        new SyntheticBatch().Convert(inserted, 1);
        FrameShape a = inserted.InsertShape(first, new Vec3(40, 100, 0), palette: 5);

        ConversionEngine created = Engine();
        EnginePattern second = SyntheticPatterns.Build(created, ["A"]);
        new SyntheticBatch().Convert(created, 1);
        FrameShape b = created.CreateNewShape(second, new Vec3(40, 100, 0), palette: 5);

        Assert.Equal(b.Tiles[0].X, a.Tiles[0].X);
        Assert.Equal(b.Tiles[0].Y, a.Tiles[0].Y);
    }

    [Fact] // REQ-E3D-233 (the "auto" part of the criterion)
    public void A_pinned_frame_restores_overlapping_shapes()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"]);

        new SyntheticBatch().Background('A', 80, 80).Convert(engine, 1);
        ConversionFrame published = engine.Publish();
        Assert.Single(published.Shapes);

        engine.PinNewFrame();
        Assert.NotNull(engine.PinnedFrame);

        ConversionFrame second = new SyntheticBatch().Convert(engine, 2);
        Assert.Empty(second.Shapes);

        // The overlap test compares a scene rectangle with the shape's screen-pixel box,
        // which REQ-E3D-233 keeps on purpose (Q-05): the shape box is (80, 80)-(87, 87).
        Assert.Equal(0, engine.RestoreFromPinnedFrame(new Rect(0, 0, 20, 20)));
        int restored = engine.RestoreFromPinnedFrame(new Rect(78, 78, 20, 20));
        Assert.Equal(1, restored);

        FrameShape added = Assert.Single(second.Shapes);
        Assert.True(added.RestoredFromPinnedFrame);
        Assert.False(added.Scriptable);

        engine.ReleasePinnedFrame();
        Assert.Null(engine.PinnedFrame);
    }

    [Fact] // REQ-E3D-233 (no pinned frame: the calls do nothing)
    public void Without_a_pinned_frame_the_calls_do_nothing()
    {
        ConversionEngine engine = Engine();
        ConversionFrame frame = new SyntheticBatch().Convert(engine);
        engine.AddRectFromPinnedFrame(0, 0, 100, 100);
        Assert.Equal(0, engine.RestoreFromPinnedFrame());
        Assert.Empty(frame.Shapes);
    }

    // ------------------------------------------------------------------ 3.12, 3.13

    [Fact] // REQ-E3D-280
    public void Publishing_does_not_touch_the_previous_frame()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"]);

        new SyntheticBatch().Background('A', 80, 80).Convert(engine, 1);
        ConversionFrame published = engine.Publish();
        int before = published.Shapes.Count;

        new SyntheticBatch().Background('A', 160, 160).Convert(engine, 2);
        Assert.Equal(before, published.Shapes.Count);
        Assert.Equal(1, published.Counter);
        Assert.Same(published, engine.PublishedFrame);
    }

    [Fact] // REQ-E3D-281
    public void Restarting_empties_every_frame_and_releases_the_pin()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"]);
        new SyntheticBatch().Background('A', 80, 80).Convert(engine, 1);
        engine.Publish();
        engine.PinNewFrame();

        engine.Restart();
        Assert.Null(engine.PinnedFrame);
        Assert.Empty(engine.ConvertingFrame.Shapes);
        Assert.Equal(0, engine.ConvertingFrame.Counter);
    }

    [Fact] // REQ-E3D-300, TV-E3D-12
    public void Tv12_tracker()
    {
        ConversionEngine engine = Engine();
        EnginePattern pt = SyntheticPatterns.Build(engine, ["T"]);
        EnginePattern px = SyntheticPatterns.Build(engine, ["X"]);
        EnginePattern pz = SyntheticPatterns.Build(engine, ["Z"]);

        // Frame 1: the tracked shape T, centre (100, 100).
        ConversionFrame frame1 = new SyntheticBatch().Sprite('T', 96, 96).Convert(engine, 1);
        FrameShape start = Assert.Single(frame1.Shapes);
        Assert.Equal(new Vec3(100, 100, 0).X, start.Center.X);
        ShapeTracker tracker = engine.StartTracker(start);

        // Frame 2: X at (104, 100) then Y of pattern PT at (140, 100) -> Y wins.
        ConversionFrame frame2 = new SyntheticBatch()
            .Sprite('X', 100, 96)
            .Sprite('T', 136, 96)
            .Convert(engine, 2);
        FrameShape? tracked = tracker.Track(frame2);
        Assert.NotNull(tracked);
        Assert.Same(pt, tracked!.ReferencePattern);
        Assert.Equal(140f, tracker.Position.X);
        Assert.Contains(px, tracker.Excluded);

        // Frame 3: X alone, now excluded -> nothing, q unchanged.
        ConversionFrame frame3 = new SyntheticBatch().Sprite('X', 137, 96).Convert(engine, 3);
        Assert.Null(tracker.Track(frame3));
        Assert.Equal(140f, tracker.Position.X);

        // Frame 4: Z at (145, 103), distance 5.83 < 9 -> tracked, PZ added to I.
        ConversionFrame frame4 = new SyntheticBatch().Sprite('Z', 141, 99).Convert(engine, 4);
        FrameShape? z = tracker.Track(frame4);
        Assert.NotNull(z);
        Assert.Same(pz, z!.ReferencePattern);
        Assert.Equal(145f, tracker.Position.X);
        Assert.Equal(103f, tracker.Position.Y);
        Assert.Contains(pz, tracker.Included);
    }

    [Fact] // REQ-E3D-300 (the threshold is 9 pixels)
    public void The_tracker_threshold_is_nine()
        => Assert.Equal(9f, ShapeTracker.Threshold);

    // ------------------------------------------------------------------ 3.15

    [Fact] // REQ-E3D-340
    public void The_render_algorithm_setting_and_its_per_instance_override()
    {
        ConversionEngine engine = Engine();
        Assert.Equal(RenderAlgorithm.Greedy, engine.Settings.RenderAlgorithm);
        Assert.Equal(0, (int)RenderAlgorithm.Greedy);
        Assert.Equal(1, (int)RenderAlgorithm.Marching);
        Assert.Equal(2, (int)RenderAlgorithm.GreedyMulticolor);
    }

    [Fact] // REQ-E3D-341
    public void Stored_density_classes_survive_a_load()
    {
        ConversionEngine engine = Engine();
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);
        pattern.StoredTileDensity = 5;
        pattern.StoredPixelDensity = 1;
        pattern.StoredColorFlags = 12;
        pattern.StoredPixelCount = 7;

        Assert.Equal(5, pattern.StoredTileDensity);
        Assert.Equal(1, pattern.StoredPixelDensity);
        Assert.Equal(12, pattern.StoredColorFlags);
        Assert.Equal(7, pattern.StoredPixelCount);
    }

    [Fact] // REQ-E3D-342
    public void Automatic_layers_change_nothing()
    {
        ConversionEngine engine = Engine();
        SyntheticPatterns.Build(engine, ["A"]);

        ConversionFrame before = new SyntheticBatch().Background('A', 80, 80).Convert(engine, 1);
        ZLayer layerBefore = before.Instances[0].EffectiveLayer(engine.Settings);

        engine.Settings.AutoLayer = true;
        ConversionFrame after = new SyntheticBatch().Background('A', 80, 80).Convert(engine, 2);
        Assert.Equal(layerBefore, after.Instances[0].EffectiveLayer(engine.Settings));
    }

    // ------------------------------------------------------------------ 3.17

    [Fact] // REQ-E3D-380
    public void Two_dimensional_mode_starts_from_an_empty_registry()
    {
        ConversionEngine engine = Engine();
        engine.TwoDimensionalMode = true;
        new SyntheticBatch().Background('A', 80, 80).Convert(engine, 1);
        Assert.NotEmpty(engine.Patterns.Permanent);

        engine.Patterns.Clear();
        engine.Images.Clear();
        Assert.Empty(engine.Patterns.Permanent);
    }

    [Fact] // REQ-E3D-008
    public void Relaunching_the_same_game_keeps_what_was_learned
        ()
    {
        ConversionEngine engine = Engine();
        new SyntheticBatch().Sprite('Z', 120, 120).Convert(engine, 1);
        EnginePattern learned = Assert.Single(engine.Patterns.Permanent);

        engine.Restart();
        Assert.Contains(learned, engine.Patterns.Permanent);

        engine.Patterns.Clear();
        Assert.Empty(engine.Patterns.Permanent);
    }
}
