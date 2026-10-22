namespace Zax.Engine3D.Tests;

/// <summary>
/// The pattern registry, its keys and its relations (SPEC-04 §3.4), and the parts of the
/// segmentation that the test vectors do not cover (§3.5, §3.6).
/// </summary>
public sealed class RegistryTests
{
    private static EnginePattern Build(
        ImageRegistry registry,
        int width,
        int height,
        TileImageKey?[] cells,
        bool selfConnected = true,
        bool permanent = true)
    {
        TileImage?[] images = new TileImage?[cells.Length];
        for (int i = 0; i < cells.Length; i++)
        {
            images[i] = cells[i] is { } key ? registry.Intern(key) : null;
        }

        EnginePattern pattern = new(width, height, images, permanent);
        pattern.SetSelfConnected(selfConnected);
        pattern.AddInstance(new PatternInstance());
        return pattern;
    }

    [Fact]
    public void REQ_E3D_061_the_relations_of_a_one_row_pattern()
    {
        ImageRegistry registry = new();
        TileImage a = registry.Intern(Img.A);
        TileImage b = registry.Intern(Img.B);

        EnginePattern open = Build(registry, 2, 1, [Img.A, Img.B], selfConnected: false);
        Assert.Equal(1, open.HorizontalPairCount);
        Assert.True(open.HasHorizontalPair(a, b));
        Assert.Equal(0, open.VerticalPairCount);

        EnginePattern closed = Build(registry, 2, 1, [Img.A, Img.B]);
        Assert.Equal(2, closed.HorizontalPairCount);
        Assert.True(closed.HasHorizontalPair(a, b));
        Assert.True(closed.HasHorizontalPair(b, a));
        Assert.Equal(2, closed.VerticalPairCount);
        Assert.True(closed.HasVerticalPair(a, a));
        Assert.True(closed.HasVerticalPair(b, b));
    }

    [Fact]
    public void REQ_E3D_061_a_single_cell_pattern_connected_to_itself()
    {
        ImageRegistry registry = new();
        TileImage a = registry.Intern(Img.A);
        EnginePattern pattern = Build(registry, 1, 1, [Img.A]);

        Assert.Equal(1, pattern.HorizontalPairCount);
        Assert.Equal(1, pattern.VerticalPairCount);
        Assert.True(pattern.HasHorizontalPair(a, a));
        Assert.True(pattern.HasVerticalPair(a, a));
    }

    [Fact]
    public void REQ_E3D_061_empty_cells_produce_no_pair()
    {
        ImageRegistry registry = new();
        EnginePattern pattern = Build(registry, 2, 1, [Img.A, null], selfConnected: false);

        Assert.Equal(0, pattern.HorizontalPairCount);
        Assert.Equal(0, pattern.VerticalPairCount);
        Assert.Single(pattern.Images);
    }

    [Fact]
    public void REQ_E3D_062_skip_geo_ignores_the_direction()
    {
        ImageRegistry registry = new();
        TileImage a = registry.Intern(Img.A);
        TileImage b = registry.Intern(Img.B);
        EnginePattern pattern = Build(registry, 2, 1, [Img.A, Img.B], selfConnected: false);
        pattern.SkipGeometry = true;

        foreach (GrowDirection direction in Enum.GetValues<GrowDirection>())
        {
            Assert.True(pattern.AllowsDirection(a, b, direction));
            Assert.True(pattern.AllowsDirection(b, a, direction));
        }

        Assert.False(pattern.AllowsDirection(a, registry.Intern(Img.C), GrowDirection.Right));
    }

    [Fact]
    public void REQ_E3D_062_the_size_constraint()
    {
        ImageRegistry registry = new();
        EnginePattern pattern = Build(registry, 1, 1, [Img.A]);
        Assert.True(pattern.FitsConstraint(99, 99));
        Assert.False(pattern.FitsConstraint(100, 1));

        pattern.ConstraintSize = new Cell(2, 99);
        Assert.True(pattern.FitsConstraint(2, 40));
        Assert.False(pattern.FitsConstraint(3, 1));
    }

    [Fact]
    public void REQ_E3D_063_two_patterns_with_the_same_images_are_equal()
    {
        ImageRegistry registry = new();
        EnginePattern wide = Build(registry, 2, 1, [Img.A, Img.B]);
        EnginePattern tall = Build(registry, 1, 3, [Img.B, Img.A, Img.B]);

        Assert.Equal(wide.Key, tall.Key);
        Assert.Equal(wide.Key.GetHashCode(), tall.Key.GetHashCode());

        PatternRegistry patterns = new();
        patterns.Add(wide);
        Assert.Same(wide, patterns.FindPermanent(tall.Key));
    }

    [Fact]
    public void REQ_E3D_064_the_pattern_pixel_box_and_its_corners()
    {
        ImageRegistry registry = new();
        EnginePattern pattern = Build(registry, 1, 1, [Img.A]);

        Assert.Equal(new Cell(0, 0), pattern.PixelStart);
        Assert.Equal(new Cell(7, 7), pattern.PixelEnd);
        Assert.Equal(new Vec2(0f, 0f), pattern.BottomLeft);

        // TopRight uses the grid width on the y axis: the kept quirk of DEC-04-Q04.
        EnginePattern wide = Build(registry, 3, 1, [Img.A, Img.B, Img.C]);
        Assert.Equal(new Vec2(23f, (8 * 3) - 0 - 1), wide.TopRight);
    }

    [Fact]
    public void REQ_E3D_065_a_learned_permanent_pattern_swallows_the_temporary_subsets()
    {
        ImageRegistry registry = new();
        PatternRegistry patterns = new();

        EnginePattern temporary = Build(registry, 1, 1, [Img.A], permanent: false);
        patterns.Add(temporary);
        Assert.Single(patterns.Temporary);

        EnginePattern permanent = Build(registry, 2, 1, [Img.A, Img.B]);
        patterns.Add(permanent);

        Assert.Empty(patterns.Temporary);
        Assert.Single(patterns.Permanent);
    }

    [Fact]
    public void REQ_E3D_065_a_new_temporary_pattern_replaces_the_ones_that_contain_it()
    {
        ImageRegistry registry = new();
        PatternRegistry patterns = new();

        EnginePattern big = Build(registry, 2, 1, [Img.A, Img.B], permanent: false);
        patterns.Add(big);
        EnginePattern small = Build(registry, 1, 1, [Img.A], permanent: false);
        patterns.Add(small);

        Assert.Single(patterns.Temporary);
        Assert.Same(small, patterns.Temporary[0]);
    }

    [Fact]
    public void REQ_E3D_065_a_permanent_pattern_replaces_the_one_of_the_same_key()
    {
        ImageRegistry registry = new();
        PatternRegistry patterns = new();
        EnginePattern first = Build(registry, 1, 1, [Img.A]);
        patterns.Add(first);
        EnginePattern second = Build(registry, 1, 1, [Img.A]);
        patterns.Add(second);

        Assert.Single(patterns.Permanent);
        Assert.Same(second, patterns.Permanent[0]);
        Assert.DoesNotContain(first, registry.Intern(Img.A).PermanentPatterns);
        Assert.Contains(second, registry.Intern(Img.A).PermanentPatterns);
    }

    [Fact]
    public void REQ_E3D_091_candidates_keep_the_registration_order()
    {
        // Two patterns of equal size holding A: the first registered wins.
        ConversionEngine engine = new ProfileBuilder()
            .Row([Img.A, Img.B])
            .Row([Img.A, Img.C])
            .Load();

        Batch batch = new();
        batch.BgCell(Img.A, 10, 10);

        Assert.Same(engine.Patterns.Permanent[0], batch.Convert(engine).Shapes[0].ReferencePattern);
    }

    [Fact]
    public void AMD_021_R2_008_a_newcomer_takes_the_place_of_the_last_pattern_removed()
    {
        // AMD-021 R2-008 criterion: P1 then P2 both hold image A; P1 is removed and P3, which
        // also holds A, is registered — the candidates of A are then P3, P2, so the shape of a
        // lone A refers to P3 and not to P2.
        ImageRegistry registry = new();
        PatternRegistry patterns = new();
        TileImage a = registry.Intern(Img.A);

        EnginePattern p1 = Build(registry, 2, 1, [Img.A, Img.B]);
        EnginePattern p2 = Build(registry, 2, 1, [Img.A, Img.C]);
        patterns.Register(p1);
        patterns.Register(p2);
        Assert.Equal<EnginePattern[]>([p1, p2], [.. a.PermanentPatterns]);

        patterns.Remove(p1);
        EnginePattern p3 = Build(registry, 2, 1, [Img.A, Img.D]);
        patterns.Register(p3);

        Assert.Equal<EnginePattern[]>([p3, p2], [.. a.PermanentPatterns]);

        // The place is used up: a fourth pattern goes to the end.
        EnginePattern p4 = Build(registry, 2, 1, [Img.A, Img.U]);
        patterns.Register(p4);
        Assert.Equal<EnginePattern[]>([p3, p2, p4], [.. a.PermanentPatterns]);
    }

    [Fact]
    public void AMD_021_R2_008_without_a_removal_a_newcomer_goes_last()
    {
        // Second half of the criterion: with no removal beforehand, P3 comes after P2.
        ImageRegistry registry = new();
        PatternRegistry patterns = new();
        TileImage a = registry.Intern(Img.A);

        EnginePattern p1 = Build(registry, 2, 1, [Img.A, Img.B]);
        EnginePattern p2 = Build(registry, 2, 1, [Img.A, Img.C]);
        EnginePattern p3 = Build(registry, 2, 1, [Img.A, Img.D]);
        patterns.Register(p1);
        patterns.Register(p2);
        patterns.Register(p3);

        Assert.Equal<EnginePattern[]>([p1, p2, p3], [.. a.PermanentPatterns]);
    }

    [Fact]
    public void AMD_021_R2_008_the_place_is_kept_per_image()
    {
        // Removing a pattern frees a place in the candidate list of each of its images, at the
        // index it occupied there — which need not be the same index in both.
        ImageRegistry registry = new();
        PatternRegistry patterns = new();
        TileImage a = registry.Intern(Img.A);
        TileImage b = registry.Intern(Img.B);

        EnginePattern onlyB = Build(registry, 1, 1, [Img.B]);
        EnginePattern both = Build(registry, 2, 1, [Img.A, Img.B]);
        EnginePattern onlyA = Build(registry, 1, 1, [Img.A]);
        patterns.Register(onlyB);
        patterns.Register(both);
        patterns.Register(onlyA);

        Assert.Equal<EnginePattern[]>([both, onlyA], [.. a.PermanentPatterns]);
        Assert.Equal<EnginePattern[]>([onlyB, both], [.. b.PermanentPatterns]);

        // "both" sits at index 0 of A and at index 1 of B.
        patterns.Remove(both);
        EnginePattern newcomer = Build(registry, 2, 1, [Img.A, Img.B]);
        patterns.Register(newcomer);

        Assert.Equal<EnginePattern[]>([newcomer, onlyA], [.. a.PermanentPatterns]);
        Assert.Equal<EnginePattern[]>([onlyB, newcomer], [.. b.PermanentPatterns]);
    }

    [Fact]
    public void AMD_021_R2_008_the_mesh_of_a_tile_follows_the_new_candidate_order()
    {
        // The observable the review asked for: which pattern the shape of a lone A refers to.
        ConversionEngine engine = new ProfileBuilder()
            .Row([Img.A, Img.B])
            .Row([Img.A, Img.C])
            .Load();

        EnginePattern p1 = engine.Patterns.Permanent[0];
        EnginePattern p2 = engine.Patterns.Permanent[1];

        Batch before = new();
        before.BgCell(Img.A, 10, 10);
        Assert.Same(p1, before.Convert(engine).Shapes[0].ReferencePattern);

        engine.Patterns.Remove(p1);

        TileImage[] images =
        [
            engine.Images.Intern(Img.A),
            engine.Images.Intern(Img.D),
        ];
        EnginePattern p3 = new(2, 1, images, permanent: true);
        p3.SetSelfConnected(true);
        p3.AddInstance(new PatternInstance());
        engine.Patterns.Register(p3);

        Batch after = new();
        after.BgCell(Img.A, 10, 10);
        Assert.Same(p3, after.Convert(engine).Shapes[0].ReferencePattern);
        Assert.NotSame(p2, after.Convert(engine).Shapes[0].ReferencePattern);
    }

    [Fact]
    public void REQ_E3D_093_a_tile_whose_patterns_are_all_disabled_is_not_displayed()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Load();
        EnginePattern pattern = engine.Patterns.Permanent[0];

        Batch batch = new();
        batch.BgCell(Img.A, 10, 10);
        Assert.Single(batch.Convert(engine, 1).Shapes);

        pattern.Disabled = true;
        Batch hidden = new();
        hidden.BgCell(Img.A, 10, 10);
        ConversionFrame frame = hidden.Convert(engine, 2);
        Assert.Empty(frame.Shapes);

        // The image stays known, so pass 2 does not learn a pattern for it.
        Assert.Single(engine.Patterns.Permanent);
        Assert.Empty(engine.Patterns.Temporary);

        pattern.Disabled = false;
        Batch back = new();
        back.BgCell(Img.A, 10, 10);
        Assert.Single(back.Convert(engine, 3).Shapes);
    }

    [Fact]
    public void REQ_E3D_093_a_disabled_pattern_does_not_stop_another_germ_from_taking_the_tile()
    {
        ConversionEngine engine = new ProfileBuilder()
            .Row([Img.A])
            .Row([Img.B, Img.A], selfConnected: false)
            .Load();
        engine.Patterns.Permanent[0].Disabled = true;

        Batch batch = new();
        batch.BgRow(10, 10, Img.B, Img.A);
        ConversionFrame frame = batch.Convert(engine);

        Assert.Single(frame.Shapes);
        Assert.Equal(2, frame.Shapes[0].TileCount);
        Assert.Same(engine.Patterns.Permanent[1], frame.Shapes[0].ReferencePattern);
    }

    [Fact]
    public void REQ_E3D_094_a_ring_is_not_full_a_U_and_a_line_are()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Load();

        // A 3x3 ring of 8 tiles, centre missing: the centre is surrounded, so not full.
        Batch ring = new();
        for (int dy = 0; dy < 3; dy++)
        {
            for (int dx = 0; dx < 3; dx++)
            {
                if (dx != 1 || dy != 1)
                {
                    ring.BgCell(Img.A, 10 + dx, 10 + dy);
                }
            }
        }

        Assert.False(ring.Convert(engine, 1).Shapes[0].IsFull());

        // A U shape: nothing strictly inside the box, so full.
        Batch u = new();
        u.BgCell(Img.A, 10, 10).BgCell(Img.A, 10, 11).BgCell(Img.A, 11, 11)
            .BgCell(Img.A, 12, 11).BgCell(Img.A, 12, 10);
        Assert.True(u.Convert(engine, 2).Shapes[0].IsFull());

        // A single line.
        Batch line = new();
        line.BgRow(10, 10, Img.A, Img.A, Img.A);
        Assert.True(line.Convert(engine, 3).Shapes[0].IsFull());
    }

    [Fact]
    public void REQ_E3D_090_pass_1_keeps_a_full_region_of_eight_tiles_and_two_images()
    {
        ConversionEngine engine = new ProfileBuilder().Pattern(2, 2, [Img.A, Img.B, Img.B, Img.A]).Load();

        // A 4x2 chequerboard: full, 8 tiles, 2 images, so pass 1 keeps it.
        Batch batch = new();
        batch.BgRow(10, 10, Img.A, Img.B, Img.A, Img.B);
        batch.BgRow(11, 10, Img.B, Img.A, Img.B, Img.A);

        ConversionFrame frame = batch.Convert(engine);
        Assert.Single(frame.Shapes);
        Assert.Equal(8, frame.Shapes[0].TileCount);
    }

    [Fact]
    public void REQ_E3D_092_a_shape_rejected_by_pass_1_keeps_its_tiles_until_the_pass_ends()
    {
        // [A] alone would grow one tile at a time; a 7-tile region of one image is rejected by
        // pass 1 (fewer than 8 tiles, one image) and recovered whole by pass 3.
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Load();
        Batch batch = new();
        for (int column = 0; column < 7; column++)
        {
            batch.BgCell(Img.A, 10 + column, 10);
        }

        ConversionFrame frame = batch.Convert(engine);

        Assert.Single(frame.Shapes);
        Assert.Equal(7, frame.Shapes[0].TileCount);
    }

    [Fact]
    public void REQ_E3D_132_a_sprite_cell_is_occupied_for_the_rest_of_the_frame()
    {
        ConversionEngine engine = ProfileBuilder.EmptyEngine();

        // Two unknown sprites of the same palette in the same cell, and a third adjacent to the
        // first: pass S1 cannot attach the second through the shared cell.
        Batch batch = new();
        batch.Sprite(Img.A, 100, 50).Sprite(Img.B, 100, 50).Sprite(Img.C, 108, 50);

        ConversionFrame frame = batch.Convert(engine);

        Assert.Equal(2, frame.Shapes.Count);
        Assert.Equal(2, frame.Shapes[0].TileCount);
        Assert.Equal(1, frame.Shapes[1].TileCount);
        Assert.Equal(Img.B, frame.Shapes[1].Images[0].Key);
    }

    [Fact]
    public void REQ_E3D_131_pass_S1_absorbs_a_known_neighbour()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.B]).Load();

        Batch batch = new();
        batch.Sprite(Img.U, 100, 50).Sprite(Img.B, 108, 50);
        ConversionFrame frame = batch.Convert(engine);

        Assert.Single(frame.Shapes);
        Assert.Equal(2, frame.Shapes[0].TileCount);
    }

    [Fact]
    public void REQ_E3D_131_a_germ_without_a_candidate_stays_free_in_pass_S2()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Load();
        engine.Patterns.Permanent[0].Disabled = true;

        Batch batch = new();
        batch.Sprite(Img.A, 100, 50);
        ConversionFrame frame = batch.Convert(engine);

        Assert.Empty(frame.Shapes);
        Assert.Single(frame.Sprites);
        Assert.True(frame.Sprites[0].IsFree);
    }

    [Fact]
    public void REQ_E3D_090_same_background_palette_stops_the_growth()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Load();
        engine.Settings.SameBackgroundPalette = true;

        Batch batch = new();
        batch.BgCell(Img.A, 10, 10).BgCell(Img.A, 11, 10, palette: 1);
        ConversionFrame frame = batch.Convert(engine);

        Assert.Equal(2, frame.Shapes.Count);

        engine.Settings.SameBackgroundPalette = false;
        Batch merged = new();
        merged.BgCell(Img.A, 10, 10).BgCell(Img.A, 11, 10, palette: 1);
        Assert.Single(merged.Convert(engine, 1).Shapes);
    }

    [Fact]
    public void REQ_E3D_253_each_tile_keeps_its_own_palette_attribute()
    {
        ConversionEngine engine = new ProfileBuilder().Row([Img.A]).Load();
        Batch batch = new();
        batch.BgCell(Img.A, 10, 10, palette: 1).BgCell(Img.A, 11, 10, palette: 2);

        FrameShape shape = batch.Convert(engine).Shapes[0];

        Assert.Equal(2, shape.TileCount);
        Assert.Equal(1, shape.Palette);
        Assert.Equal(1, shape.Tiles[0].Palette);
        Assert.Equal(2, shape.Tiles[1].Palette);
    }

    [Fact]
    public void REQ_E3D_341_the_stored_densities_are_kept_as_read()
    {
        ProfileBuilder builder = new();
        builder.Row([Img.A]);
        Zax.Profiles.ProfilePattern pattern = builder.Build().PermanentPatterns[0];
        pattern.PixelCount = 123;
        pattern.TileDensity = (Zax.Profiles.PatternDensity)6;
        pattern.PixelDensity = Zax.Profiles.PatternDensity.High;
        pattern.ColorFlags = Zax.Profiles.PatternColorFlags.Color2;

        ConversionEngine engine = new();
        engine.LoadProfile(builder.Build());

        EnginePattern registered = engine.Patterns.Permanent[0];
        Assert.Equal(123, registered.StoredPixelCount);
        Assert.Equal(6, registered.StoredTileDensity);
        Assert.Equal(4, registered.StoredPixelDensity);
        Assert.Equal(4, registered.StoredColorFlags);
    }

    [Fact]
    public void REQ_E3D_342_the_automatic_layer_setting_has_no_effect()
    {
        ConversionEngine engine = new ProfileBuilder()
            .Pattern(1, 1, [Img.A], layer: Zax.Profiles.ProfileLayer.Unidentified)
            .Load();

        Batch batch = new();
        batch.BgCell(Img.A, 10, 10);
        ZLayer before = batch.Convert(engine, 1).Instances[0].Layer;

        engine.Settings.AutoLayer = true;
        Batch after = new();
        after.BgCell(Img.A, 10, 10);
        Assert.Equal(before, after.Convert(engine, 2).Instances[0].Layer);
        Assert.Equal(ZLayer.Unidentified, before);
    }
}
