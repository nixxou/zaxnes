namespace Zax.Engine3D.Tests;

/// <summary>
/// The test vectors of SPEC-04B §5.3: the four cylindrical geometry types (REQ-E3D-444 to
/// 452).
/// </summary>
/// <remarks>
/// Voxels are listed in the plane of the section, <c>(i, k)</c> for the V types (the row
/// <c>j</c> being fixed) and <c>(j, k)</c> for the H types, with the texel assigned to them —
/// the column for V, the row for H.
/// </remarks>
public sealed class CylinderVectorTests
{
    [Fact]
    public void TV_Y1_a_row_of_eight_becomes_a_ring_of_twenty_four_voxels()
    {
        // REQ-E3D-445, 446, 447, 448.
        EnginePattern pattern = Geo.OneTile(Geo.Tile("11111111"));
        PatternInstance instance = Geo.Object3D(depth: 8, geometry: Geo3DType.VCylinder);
        TileVoxels[] fields = Geo.Fields(pattern, instance, out int effectiveDepth);

        Assert.Equal(8, effectiveDepth);
        Assert.Equal(
            Geo.Voxels(
                (0, 3, 0), (0, 4, 7),
                (1, 1, 1), (1, 2, 1), (1, 5, 6), (1, 6, 6),
                (2, 0, 2), (2, 1, 2), (2, 6, 5), (2, 7, 5),
                (3, 0, 3), (3, 7, 4),
                (4, 0, 4), (4, 7, 3),
                (5, 0, 5), (5, 1, 5), (5, 6, 2), (5, 7, 2),
                (6, 1, 6), (6, 2, 6), (6, 5, 1), (6, 6, 1),
                (7, 3, 7), (7, 4, 0)),
            Geo.Section(fields[0], line: 0, vertical: true));

        TileMesh mesh = Geo.Mesh(pattern, instance);
        Assert.Equal(64, Geo.Rectangles(mesh));
    }

    [Fact]
    public void TV_Y2_a_solid_row_of_eight_fills_forty_eight_voxels()
    {
        // REQ-E3D-449.
        EnginePattern pattern = Geo.OneTile(Geo.Tile("11111111"));
        PatternInstance instance = Geo.Object3D(depth: 8, geometry: Geo3DType.VCylinder, solid: true);
        TileVoxels[] fields = Geo.Fields(pattern, instance, out _);
        List<(int Line, int Depth, int Texel)> section = Geo.Section(fields[0], line: 0, vertical: true);

        // Shape (normative): i = 0 and 7 keep k in {3, 4}, i = 1 and 6 keep k in [1, 6],
        // i = 2 to 5 keep the whole depth.
        List<(int, int)> expected = [];
        foreach (int i in new[] { 0, 7 })
        {
            expected.Add((i, 3));
            expected.Add((i, 4));
        }

        foreach (int i in new[] { 1, 6 })
        {
            for (int k = 1; k <= 6; k++)
            {
                expected.Add((i, k));
            }
        }

        foreach (int i in new[] { 2, 3, 4, 5 })
        {
            for (int k = 0; k <= 7; k++)
            {
                expected.Add((i, k));
            }
        }

        expected.Sort();
        List<(int, int)> actual = section.ConvertAll(v => (v.Line, v.Depth));
        actual.Sort();
        Assert.Equal(48, actual.Count);
        Assert.Equal(expected, actual);

        // Control texels of the order of REQ-E3D-449 (P2).
        Assert.Equal(0, TexelAt(section, 1, 3));
        Assert.Equal(7, TexelAt(section, 1, 4));
        Assert.Equal(2, TexelAt(section, 3, 1));
        Assert.Equal(5, TexelAt(section, 4, 1));
        Assert.Equal(1, TexelAt(section, 2, 2));

        Assert.Equal(30, Geo.Rectangles(Geo.Mesh(pattern, instance)));
    }

    [Fact]
    public void TV_Y3_horizontal_cylinders_mirror_the_vertical_ones()
    {
        // REQ-E3D-444, 452: a column of four pixels with sizeZ = 8 ends up with D' = 4.
        EnginePattern pattern = Geo.OneTile(Geo.Tile("1.......", "1.......", "1.......", "1......."));
        PatternInstance instance = Geo.Object3D(depth: 8, geometry: Geo3DType.HCylinder);
        TileVoxels[] fields = Geo.Fields(pattern, instance, out int effectiveDepth);

        Assert.Equal(4, effectiveDepth);
        Assert.Equal(
            Geo.Voxels(
                (4, 3, 3), (4, 4, 0),
                (5, 2, 2), (5, 5, 1),
                (6, 2, 1), (6, 5, 2),
                (7, 3, 0), (7, 4, 3)),
            Geo.Section(fields[0], line: 0, vertical: false));
        Assert.Equal(24, Geo.Rectangles(Geo.Mesh(pattern, instance)));

        // A full column of eight with sizeZ = 8: the 24 voxels of TV-Y1, mirrored.
        EnginePattern full = Geo.OneTile(Geo.Tile("1.......", "1.......", "1.......", "1.......", "1.......", "1.......", "1.......", "1......."));
        TileVoxels[] fullFields = Geo.Fields(full, instance, out int fullDepth);
        Assert.Equal(8, fullDepth);
        Assert.Equal(24, Geo.Section(fullFields[0], line: 0, vertical: false).Count);
        Assert.Equal(64, Geo.Rectangles(Geo.Mesh(full, instance)));

        // The same column with sizeZ = 4.
        PatternInstance shallow = Geo.Object3D(depth: 4, geometry: Geo3DType.HCylinder);
        TileVoxels[] shallowFields = Geo.Fields(full, shallow, out int shallowDepth);
        Assert.Equal(4, shallowDepth);
        Assert.Equal(
            Geo.Voxels(
                (0, 1, 0), (0, 2, 7),
                (1, 1, 6), (1, 2, 1),
                (2, 0, 5), (2, 3, 2),
                (3, 0, 4), (3, 3, 3),
                (4, 0, 3), (4, 3, 4),
                (5, 0, 2), (5, 3, 5),
                (6, 1, 1), (6, 2, 6),
                (7, 1, 7), (7, 2, 0)),
            Geo.Section(shallowFields[0], line: 0, vertical: false));
    }

    [Fact]
    public void TV_Y4_solid_full_warp_fills_each_column_with_its_own_colour()
    {
        // REQ-E3D-450 (AMD-048). Both cases stay exact under the empty-span rule: the region 2
        // points whose bounds are the wrong way round are followed by a point of the same u and
        // the same span — (7, 2) and (7, 3) for D = 8, (3, 2) for D = 4 — which writes the very
        // same voxels. The opposite argument of Q-015 and of Q-082 §4.1 left that point out.
        EnginePattern pattern = Geo.OneTile(Geo.Tile("11111111"));
        PatternInstance instance = Geo.Object3D(
            depth: 8,
            geometry: Geo3DType.VCylinder,
            solid: true,
            fullWarp: true);
        TileVoxels[] fields = Geo.Fields(pattern, instance, out _);
        List<(int Line, int Depth, int Texel)> section = Geo.Section(fields[0], line: 0, vertical: true);

        Assert.Equal(48, section.Count);
        foreach ((int line, int _, int texel) in section)
        {
            // Every voxel of a column carries the texel of that column.
            Assert.Equal(line, texel);
        }

        // Row `1123....`, D = 4.
        EnginePattern four = Geo.OneTile(Geo.Tile("1123...."));
        PatternInstance shallow = Geo.Object3D(
            depth: 4,
            geometry: Geo3DType.VCylinder,
            solid: true,
            fullWarp: true);
        TileVoxels[] fourFields = Geo.Fields(four, shallow, out _);
        Assert.Equal(
            Geo.Voxels(
                (0, 1, 0), (0, 2, 0),
                (1, 0, 1), (1, 1, 1), (1, 2, 1), (1, 3, 1),
                (2, 0, 2), (2, 1, 2), (2, 2, 2), (2, 3, 2),
                (3, 1, 3), (3, 2, 3)),
            Geo.Section(fourFields[0], line: 0, vertical: true));
        Assert.Equal(18, Geo.Rectangles(Geo.Mesh(four, shallow)));
    }

    [Fact]
    public void TV_Y5_a_degenerate_section_is_a_single_voxel()
    {
        // REQ-E3D-451.
        EnginePattern row = Geo.OneTile(Geo.Tile("11111111"));
        PatternInstance flat = Geo.Object3D(depth: 1, geometry: Geo3DType.VCylinder);
        TileVoxels[] fields = Geo.Fields(row, flat, out int depth);

        Assert.Equal(1, depth);
        Assert.Equal(Geo.Voxels((3, 0, 3)), Geo.Section(fields[0], line: 0, vertical: true));

        TileMesh mesh = Geo.Mesh(row, flat);
        Assert.True(Geo.Close(new Vec3(-1f, 3f, -0.5f), mesh.BoundsMin));
        Assert.True(Geo.Close(new Vec3(0f, 4f, 0.5f), mesh.BoundsMax));

        // A single pixel with D = 5: one voxel in the middle slice, D' = 1.
        EnginePattern pixel = Geo.OneTile(Geo.Tile("1......."));
        PatternInstance deep = Geo.Object3D(depth: 5, geometry: Geo3DType.VCylinder);
        TileVoxels[] pixelFields = Geo.Fields(pixel, deep, out int pixelDepth);
        Assert.Equal(1, pixelDepth);
        Assert.Equal(Geo.Voxels((0, 2, 0)), Geo.Section(pixelFields[0], line: 0, vertical: true));
    }

    [Fact]
    public void TV_Y6_anti_shadow_moves_the_texels_of_the_flanks()
    {
        // REQ-E3D-447.
        EnginePattern pattern = Geo.OneTile(Geo.Tile("12333321"));
        PatternInstance plain = Geo.Object3D(depth: 8, geometry: Geo3DType.VCylinder);
        PatternInstance shaded = Geo.Object3D(depth: 8, geometry: Geo3DType.VCylinder, antiShadow: true);

        List<(int Line, int Depth, int Texel)> without = Geo.Section(
            Geo.Fields(pattern, plain, out _)[0],
            line: 0,
            vertical: true);
        List<(int Line, int Depth, int Texel)> with = Geo.Section(
            Geo.Fields(pattern, shaded, out _)[0],
            line: 0,
            vertical: true);

        // Same positions, only the texels of the region 2 voxels move.
        Assert.Equal(
            without.ConvertAll(v => (v.Line, v.Depth)),
            with.ConvertAll(v => (v.Line, v.Depth)));

        Assert.Equal(0, TexelAt(without, 0, 3));
        Assert.Equal(7, TexelAt(without, 0, 4));
        Assert.Equal(7, TexelAt(without, 7, 3));
        Assert.Equal(0, TexelAt(without, 7, 4));

        Assert.Equal(2, TexelAt(with, 0, 3));
        Assert.Equal(5, TexelAt(with, 0, 4));
        Assert.Equal(5, TexelAt(with, 7, 3));
        Assert.Equal(2, TexelAt(with, 7, 4));

        // The other voxels keep their texel.
        foreach ((int line, int depth, int texel) in without)
        {
            if (line is 0 or 7 && depth is 3 or 4)
            {
                continue;
            }

            Assert.Equal(texel, TexelAt(with, line, depth));
        }
    }

    [Fact]
    public void TV_Y7_quadrant_points_of_texel_zero_are_skipped()
    {
        // REQ-E3D-448: the test is on the texel value, not on the transparent colour.
        EnginePattern pattern = Geo.OneTile(Geo.Tile("1101...."));
        PatternInstance instance = Geo.Object3D(
            depth: 4,
            geometry: Geo3DType.VCylinder,
            emptyColor: 4);
        TileVoxels[] fields = Geo.Fields(pattern, instance, out _);

        Assert.Equal(
            Geo.Voxels(
                (0, 2, 0),
                (1, 1, 1),
                (3, 0, 3),
                (4, 3, 3),
                (6, 2, 1),
                (7, 1, 0)),
            Geo.Section(fields[0], line: 0, vertical: true));
    }

    [Fact]
    public void TV_Y8_half_cylinders_leave_their_flat_face_open()
    {
        // REQ-E3D-445, 447: A = 2L, the centre at s or at f depending on flip.
        EnginePattern row = Geo.OneTile(Geo.Tile("1111...."));
        EnginePattern column = Geo.OneTile(Geo.Tile("1.......", "1.......", "1.......", "1......."));

        PatternInstance halfV = Geo.Object3D(depth: 8, geometry: Geo3DType.HalfVCylinder);
        TileVoxels[] halfVFields = Geo.Fields(row, halfV, out int halfVDepth);
        Assert.Equal(7, halfVDepth);
        Assert.Equal(
            Geo.Voxels(
                (0, 1, 0), (0, 6, 0),
                (1, 1, 1), (1, 6, 1),
                (2, 1, 2), (2, 2, 2), (2, 5, 2), (2, 6, 2),
                (3, 3, 3), (3, 4, 3)),
            Geo.Section(halfVFields[0], line: 0, vertical: true));
        Assert.Equal(26, Geo.Rectangles(Geo.Mesh(row, halfV)));

        PatternInstance halfVFlipped = Geo.Object3D(
            depth: 8,
            geometry: Geo3DType.HalfVCylinder,
            flip: true);
        TileVoxels[] flippedFields = Geo.Fields(row, halfVFlipped, out int flippedDepth);
        Assert.Equal(7, flippedDepth);
        Assert.Equal(
            Geo.Voxels(
                (0, 3, 0), (0, 4, 0),
                (1, 1, 1), (1, 2, 1), (1, 5, 1), (1, 6, 1),
                (2, 1, 2), (2, 6, 2),
                (3, 1, 3), (3, 6, 3)),
            Geo.Section(flippedFields[0], line: 0, vertical: true));
        Assert.Equal(26, Geo.Rectangles(Geo.Mesh(row, halfVFlipped)));

        PatternInstance halfH = Geo.Object3D(depth: 8, geometry: Geo3DType.HalfHCylinder);
        TileVoxels[] halfHFields = Geo.Fields(column, halfH, out int halfHDepth);
        Assert.Equal(7, halfHDepth);
        Assert.Equal(
            Geo.Voxels(
                (4, 1, 3), (4, 6, 3),
                (5, 1, 2), (5, 6, 2),
                (6, 1, 1), (6, 2, 1), (6, 5, 1), (6, 6, 1),
                (7, 3, 0), (7, 4, 0)),
            Geo.Section(halfHFields[0], line: 0, vertical: false));
        Assert.Equal(26, Geo.Rectangles(Geo.Mesh(column, halfH)));
    }

    [Fact]
    public void TV_Y9_a_wrapped_solid_segment_of_two_pixels_at_depth_three_keeps_one_slice()
    {
        // REQ-E3D-450 (AMD-048), 452, 504: the ring ropes of the `punchout` profile, an
        // HCYLINDER of D = 3, solid = fullWarp = true, whose rope is two pixels tall on the
        // image rows r = 5 and r + 1 = 6. HCYLINDER cuts its segments column by column, so
        // L = 1, A = B = 1 and Cz = 2. The only contour point whose bounds differ is the first
        // of region 2, with z+ = 0 and z- = 2: its span is empty and it writes nothing. Two
        // voxels are left, on the middle slice, each carrying the texel of its own row, at
        // (7 - r, 1) and (6 - r, 1) in the (j, k) plane of §5.3. D' = B + 1 = 2.
        EnginePattern pattern = Geo.OneTile(Geo.Tile(
            "........",
            "........",
            "........",
            "........",
            "........",
            "1.......",
            "2......."));
        PatternInstance instance = Geo.Object3D(
            depth: 3,
            geometry: Geo3DType.HCylinder,
            solid: true,
            fullWarp: true);
        TileVoxels[] fields = Geo.Fields(pattern, instance, out int effectiveDepth);

        Assert.Equal(2, effectiveDepth);
        Assert.Equal(
            Geo.Voxels((2, 1, 5), (1, 1, 6)),
            Geo.Section(fields[0], line: 0, vertical: false));

        // A whole tile of eight identical columns gives a box of 8 × 2 × 1 units: six
        // rectangles, and one unit of depth in the local frame of REQ-E3D-504, which keeps the
        // construction depth D = 3 rather than D'.
        EnginePattern rope = Geo.OneTile(Geo.Tile(
            "........",
            "........",
            "........",
            "........",
            "........",
            "11111111",
            "22222222"));
        TileMesh mesh = Geo.Mesh(rope, instance);
        Assert.Equal(6, Geo.Rectangles(mesh));
        Assert.True(Geo.Close(new Vec3(-4f, -3f, -0.5f), mesh.BoundsMin));
        Assert.True(Geo.Close(new Vec3(4f, -1f, 0.5f), mesh.BoundsMax));

        // REQ-E3D-448 (contour) and REQ-E3D-449 (solid, not wrapped) are unchanged: there the
        // point of inverted bounds lays its voxels as before, and the same segment still
        // occupies the three slices. The asymmetry between the four flag pairs is intended
        // (AMD-048).
        foreach ((bool solid, bool fullWarp) in new[] { (false, false), (false, true), (true, false) })
        {
            PatternInstance other = Geo.Object3D(
                depth: 3,
                geometry: Geo3DType.HCylinder,
                solid: solid,
                fullWarp: fullWarp);
            TileVoxels[] others = Geo.Fields(pattern, other, out int otherDepth);
            List<(int Line, int Depth, int Texel)> section =
                Geo.Section(others[0], line: 0, vertical: false);

            Assert.Equal(2, otherDepth);
            Assert.Equal(
                [(1, 0), (1, 1), (1, 2), (2, 0), (2, 1), (2, 2)],
                section.ConvertAll(v => (v.Line, v.Depth)));
        }
    }

    [Fact]
    public void TV_Y10_a_wrapped_solid_row_at_depth_four_loses_its_extreme_columns()
    {
        // REQ-E3D-450 (AMD-048): at an even depth the first two points of region 2 both have
        // their bounds the wrong way round, and here nothing else reaches the extreme columns —
        // region 1 stops at u+ = 6 and region 2 does not retain (A, 2), since 2A > B². The row
        // therefore keeps 20 voxels out of 24 and is six units wide instead of eight.
        EnginePattern pattern = Geo.OneTile(Geo.Tile("11111111"));
        PatternInstance instance = Geo.Object3D(
            depth: 4,
            geometry: Geo3DType.VCylinder,
            solid: true,
            fullWarp: true);
        TileVoxels[] fields = Geo.Fields(pattern, instance, out int effectiveDepth);
        List<(int Line, int Depth, int Texel)> section = Geo.Section(
            fields[0],
            line: 0,
            vertical: true);

        // i = 2 to 5 keep the whole depth, i = 1 and 6 keep k in {1, 2}, i = 0 and 7 are empty;
        // every voxel (i, k) carries the texel i.
        List<(int Line, int Depth, int Texel)> expected = [];
        foreach (int i in new[] { 1, 6 })
        {
            expected.Add((i, 1, i));
            expected.Add((i, 2, i));
        }

        foreach (int i in new[] { 2, 3, 4, 5 })
        {
            for (int k = 0; k <= 3; k++)
            {
                expected.Add((i, k, i));
            }
        }

        expected.Sort();
        Assert.Equal(4, effectiveDepth);
        Assert.Equal(20, section.Count);
        Assert.Equal(expected, section);
        Assert.DoesNotContain(section, v => v.Line is 0 or 7);

        // The rectangle count does not discriminate the two readings (18 either way): the
        // voxels and the local box do.
        TileMesh mesh = Geo.Mesh(pattern, instance);
        Assert.Equal(18, Geo.Rectangles(mesh));
        Assert.True(Geo.Close(new Vec3(-3f, 3f, -2f), mesh.BoundsMin));
        Assert.True(Geo.Close(new Vec3(3f, 4f, 2f), mesh.BoundsMax));
    }

    [Fact]
    public void Section_control_cases_of_paragraph_5_3()
    {
        // Row `111.....`, VCYLINDER, D = 8 -> six voxels, D' = 3.
        EnginePattern three = Geo.OneTile(Geo.Tile("111....."));
        PatternInstance instance = Geo.Object3D(depth: 8, geometry: Geo3DType.VCylinder);
        TileVoxels[] fields = Geo.Fields(three, instance, out int depth);

        Assert.Equal(3, depth);
        Assert.Equal(
            Geo.Voxels(
                (0, 3, 0), (0, 4, 2),
                (1, 3, 1), (1, 4, 1),
                (2, 3, 2), (2, 4, 0)),
            Geo.Section(fields[0], line: 0, vertical: true));

        // Row `11111111`, D = 2 -> sixteen voxels and six rectangles. The texels of the two
        // extreme columns are those of REQ-E3D-446 and 447, not those of the parenthetical of
        // paragraph 5.3, which the region 2 point overwrites (Q-014).
        EnginePattern row = Geo.OneTile(Geo.Tile("11111111"));
        PatternInstance shallow = Geo.Object3D(depth: 2, geometry: Geo3DType.VCylinder);
        TileVoxels[] shallowFields = Geo.Fields(row, shallow, out int shallowDepth);
        List<(int Line, int Depth, int Texel)> section = Geo.Section(
            shallowFields[0],
            line: 0,
            vertical: true);

        Assert.Equal(2, shallowDepth);
        Assert.Equal(16, section.Count);
        Assert.Equal(6, Geo.Rectangles(Geo.Mesh(row, shallow)));

        for (int i = 1; i <= 6; i++)
        {
            Assert.Equal(i, TexelAt(section, i, 0));
            Assert.Equal(7 - i, TexelAt(section, i, 1));
        }
    }

    [Fact]
    public void REQ_E3D_452_a_cylinder_without_a_segment_has_no_depth()
    {
        EnginePattern blank = Geo.OneTile(Geo.Blank);
        PatternInstance instance = Geo.Object3D(depth: 8, geometry: Geo3DType.VCylinder);
        TileVoxels[] fields = Geo.Fields(blank, instance, out int depth);

        Assert.Equal(0, depth);
        Assert.Empty(Geo.Section(fields[0], line: 0, vertical: true));
    }

    private static int TexelAt(List<(int Line, int Depth, int Texel)> section, int line, int depth)
    {
        foreach ((int l, int d, int texel) in section)
        {
            if (l == line && d == depth)
            {
                return texel;
            }
        }

        Assert.Fail($"no voxel at ({line}, {depth})");
        return -1;
    }
}
