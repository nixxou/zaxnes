using Zax.Engine3D;
using Zax.Profiles;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for SPEC-04B: frame of reference, voxel field, geometry types,
/// meshing, texture and colour, and the vectors TV-G, TV-C, TV-Y, TV-M, TV-W and TV-K of §5.
/// </summary>
public sealed class GeometryTests
{
    /// <summary>A face, compared the way REQ-E3D-523 prescribes.</summary>
    private readonly record struct Face(Vec3 Normal, string Corners);

    private static ConversionEngine EngineWith(out GeometryBuilder geometry)
    {
        ConversionEngine engine = new();
        geometry = engine.UseGeometry();
        return engine;
    }

    /// <summary>
    /// Builds a one-tile (or multi-tile) pattern from rows of digits, each digit being a
    /// texel value, and returns its geometry.
    /// </summary>
    private static ObjectGeometry Build(
        ConversionEngine engine,
        GeometryBuilder geometry,
        string[] rows,
        int tilesWide = 1,
        Action<PatternInstance>? configure = null)
    {
        int height = rows.Length;
        int width = rows[0].Length;
        int tilesHigh = height / 8;
        Assert.Equal(8 * tilesWide, width);
        Assert.Equal(8 * tilesHigh, height);

        List<TileImage?> cells = [];
        for (int ty = 0; ty < tilesHigh; ty++)
        {
            for (int tx = 0; tx < tilesWide; tx++)
            {
                ulong low = 0;
                ulong high = 0;
                for (int y = 0; y < 8; y++)
                {
                    for (int x = 0; x < 8; x++)
                    {
                        int value = rows[(8 * ty) + y][(8 * tx) + x] - '0';
                        int bit = (8 * y) + 7 - x;
                        if ((value & 1) != 0)
                        {
                            low |= 1UL << bit;
                        }

                        if ((value & 2) != 0)
                        {
                            high |= 1UL << bit;
                        }
                    }
                }

                cells.Add(engine.Images.Intern(low, high, false, false));
            }
        }

        EnginePattern pattern = new(tilesWide, tilesHigh, cells, permanent: true);
        PatternInstance instance = new()
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
        };

        configure?.Invoke(instance);
        pattern.AddInstance(instance);
        engine.Patterns.Register(pattern);
        geometry.Build(pattern);
        geometry.Commit();

        return geometry.Of(instance)!;
    }

    private static string[] Blank(int tilesWide = 1, int tilesHigh = 1)
        => [.. Enumerable.Repeat(new string('0', 8 * tilesWide), 8 * tilesHigh)];

    private static string[] WithRow(string row, int tilesHigh = 1)
    {
        string[] rows = Blank(row.Length / 8, tilesHigh);
        rows[0] = row;
        return rows;
    }

    /// <summary>Distinct axis-aligned faces of a mesh, as (normal, sorted corner set).</summary>
    private static List<Face> Faces(TileMesh mesh)
    {
        Dictionary<string, Face> faces = [];
        ReadOnlySpan<int> triangles = mesh.Triangles;
        ReadOnlySpan<MeshVertex> vertices = mesh.Vertices;

        for (int t = 0; t < triangles.Length; t += 3)
        {
            MeshVertex a = vertices[triangles[t]];
            MeshVertex b = vertices[triangles[t + 1]];
            MeshVertex c = vertices[triangles[t + 2]];
            string key = string.Join('|', new[] { Key(a), Key(b), Key(c) }.Order(StringComparer.Ordinal))
                + "#" + Key(a.Normal);
            faces[key] = new Face(a.Normal, key);
        }

        return [.. faces.Values];
    }

    private static string Key(MeshVertex v) => $"{v.Position.X:F3},{v.Position.Y:F3},{v.Position.Z:F3}";

    private static string Key(Vec3 v) => $"{v.X:F3},{v.Y:F3},{v.Z:F3}";

    /// <summary>
    /// Counts the rectangles of a GREEDY mesh: each rectangle is two triangles sharing a
    /// plane and a normal, so the triangle count divided by two.
    /// </summary>
    private static int Rectangles(TileMesh mesh) => mesh.TriangleCount / 2;

    // ------------------------------------------------------------------ 3.1 frame of reference

    [Fact] // REQ-E3D-400, REQ-E3D-401, REQ-E3D-410
    public void The_scene_frame_of_reference()
    {
        Assert.Equal(248, ConversionGrid.SceneHeight);

        // Pixel (0, 0) occupies y in [247, 248]; pixel (255, 239) occupies y in [8, 9].
        Assert.Equal(247, ConversionGrid.SceneHeight - 0 - 1);
        Assert.Equal(8, ConversionGrid.SceneHeight - 239 - 1);
    }

    [Theory] // REQ-E3D-403
    [InlineData(ZLayer.L1, 0f, ZAlignment.Front, 4, 0f, 4f)]
    [InlineData(ZLayer.L2, 12f, ZAlignment.Front, 4, 12f, 16f)]
    [InlineData(ZLayer.L1, 0f, ZAlignment.Center, 4, -2f, 2f)]
    [InlineData(ZLayer.L1, 0f, ZAlignment.Back, 4, -4f, 0f)]
    public void Layer_depth_intervals(
        ZLayer layer, float offset, ZAlignment alignment, int depth, float front, float back)
    {
        ConversionEngine engine = new();
        engine.Settings.Layer(layer).Offset = offset;
        engine.Settings.Layer(layer).Alignment = alignment;

        SyntheticPatterns.Build(engine, ["A"]);
        engine.Patterns.Permanent[0].Instances[0].Layer = layer;
        engine.Patterns.Permanent[0].Instances[0].Depth = depth;

        ConversionFrame frame = new SyntheticBatch().Background('A', 16, 32).Convert(engine);
        (float f, float b) = frame.Instances[0].DepthInterval(engine.Settings);
        Assert.Equal(front, f, 3);
        Assert.Equal(back, b, 3);
    }

    [Fact] // REQ-E3D-403 (the factory values)
    public void Factory_layer_values()
    {
        ConversionSettings settings = new();
        Assert.Equal(0f, settings.Layer(ZLayer.Ui).Offset);
        Assert.Equal(ZAlignment.Center, settings.Layer(ZLayer.Ui).Alignment);
        Assert.Equal(0f, settings.Layer(ZLayer.L1).Offset);
        Assert.Equal(ZAlignment.Front, settings.Layer(ZLayer.L1).Alignment);
        Assert.Equal(12f, settings.Layer(ZLayer.L2).Offset);
        Assert.Equal(24f, settings.Layer(ZLayer.L3).Offset);
        Assert.Equal(36f, settings.Layer(ZLayer.L4).Offset);
        Assert.Equal(ZLayer.L1, settings.DefaultLayer);
    }

    [Fact] // REQ-E3D-402
    public void An_unidentified_layer_resolves_per_shape_kind()
    {
        ConversionEngine engine = new();
        SyntheticPatterns.Build(engine, ["A"]);
        engine.Patterns.Permanent[0].Instances[0].Layer = ZLayer.Unidentified;

        ConversionFrame sprite = new SyntheticBatch().Sprite('A', 80, 80).Convert(engine, 1);
        Assert.Equal(ZLayer.L1, sprite.Instances[0].EffectiveLayer(engine.Settings));

        engine.Settings.DefaultLayer = ZLayer.L3;
        ConversionFrame background = new SyntheticBatch().Background('A', 80, 80).Convert(engine, 2);
        Assert.Equal(ZLayer.L3, background.Instances[0].EffectiveLayer(engine.Settings));
    }

    [Fact] // REQ-E3D-404, TV-W1
    public void Tw1_placement()
    {
        ConversionEngine engine = new();
        SyntheticPatterns.Build(engine, ["A"]);
        engine.Patterns.Permanent[0].Instances[0].Depth = 4;

        ConversionFrame frame = new SyntheticBatch().Background('A', 16, 32).Convert(engine);
        FrameInstance instance = frame.Instances[0];

        Assert.Equal(new Vec3(20, 212, 2), instance.Pos);
        Assert.Equal(new Vec3(16, 208, 0), instance.BoxMin(engine.Settings));
        Assert.Equal(new Vec3(24, 216, 4), instance.BoxMax(engine.Settings));

        Assert.Equal(new Vec3(16, 208, 0), instance.GetPos(engine.Settings, new Vec3(0, 0, 0)));

        instance.Offset = new Vec3(1, -2, 3);
        instance.CalculatePos(engine.Settings);
        Assert.Equal(new Vec3(21, 210, 5), instance.Pos);
    }

    [Theory] // REQ-E3D-407
    [InlineData(0f, 0f, 90f, 8f, 0f, 0f, 0f, 8f, 0f)]
    public void Rotation_of_a_relative_vector(
        float rx, float ry, float rz, float vx, float vy, float vz, float ex, float ey, float ez)
    {
        Mat4 rotation = Mat4.RotationX(rx) * Mat4.RotationY(ry) * Mat4.RotationZ(rz);
        Vec3 rotated = rotation.TransformPoint(new Vec3(vx, vy, vz));
        Assert.Equal(ex, rotated.X, 3);
        Assert.Equal(ey, rotated.Y, 3);
        Assert.Equal(ez, rotated.Z, 3);
    }

    [Fact] // REQ-E3D-407 (the _gaxis order)
    public void The_ground_axis_tag_swaps_the_first_two_rotations()
    {
        Vec3 unitZ = new(0, 0, 1);

        Mat4 x = Mat4.RotationX(90);
        Mat4 y = Mat4.RotationY(90);
        Mat4 z = Mat4.RotationZ(0);
        Vec3 plain = (x * y * z).TransformPoint(unitZ);
        Assert.Equal(1f, plain.X, 3);
        Assert.Equal(0f, plain.Y, 3);
        Assert.Equal(0f, plain.Z, 3);

        Vec3 ground = (y * x * z).TransformPoint(unitZ);
        Assert.Equal(0f, ground.X, 3);
        Assert.Equal(-1f, ground.Y, 3);
        Assert.Equal(0f, ground.Z, 3);
    }

    // ------------------------------------------------------------------ 3.2, 3.3 voxels

    [Fact] // REQ-E3D-420
    public void A_depth_of_zero_produces_no_geometry()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        ObjectGeometry built = Build(engine, geometry, WithRow("10000000"), configure: i => i.Depth = 0);
        Assert.All(built.Meshes.ToArray(), m => Assert.True(m.IsEmpty));
    }

    [Fact] // REQ-E3D-421, TV-G1
    public void Tvg1_single_pixel()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        ObjectGeometry built = Build(engine, geometry, WithRow("10000000"));
        TileMesh mesh = built.Meshes[0];

        Assert.Equal(6, Rectangles(mesh));
        Assert.Equal(new Vec3(-4, 3, -0.5f), mesh.BoundsMin);
        Assert.Equal(new Vec3(-3, 4, 0.5f), mesh.BoundsMax);

        // The six normals, one face each.
        List<Face> faces = Faces(mesh);
        Assert.Equal(12, faces.Count); // two triangles per rectangle
        foreach (Vec3 normal in new[]
        {
            new Vec3(-1, 0, 0), new Vec3(1, 0, 0), new Vec3(0, -1, 0),
            new Vec3(0, 1, 0), new Vec3(0, 0, -1), new Vec3(0, 0, 1),
        })
        {
            Assert.Contains(faces, f => f.Normal.Equals(normal));
        }

        // All texture coordinates are inside the texel (0, 0) square.
        foreach (MeshVertex vertex in mesh.Vertices.ToArray())
        {
            Assert.InRange(vertex.Uv.X, 0f, 1f);
            Assert.InRange(vertex.Uv.Y, 0f, 1f);
        }
    }

    [Fact] // REQ-E3D-440, TV-G1 (depth 4 keeps six rectangles)
    public void Tvg1_single_pixel_at_depth_four()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        ObjectGeometry built = Build(engine, geometry, WithRow("10000000"), configure: i => i.Depth = 4);
        TileMesh mesh = built.Meshes[0];

        Assert.Equal(6, Rectangles(mesh));
        Assert.Equal(new Vec3(-4, 3, -2f), mesh.BoundsMin);
        Assert.Equal(new Vec3(-3, 4, 2f), mesh.BoundsMax);
    }

    [Fact] // REQ-E3D-502, TV-G2
    public void Tvg2_two_pixels_greedy_and_multicolour()
    {
        ConversionEngine greedy = EngineWith(out GeometryBuilder gb);
        ObjectGeometry plain = Build(greedy, gb, WithRow("12000000"), configure: i => i.Depth = 2);
        Assert.Equal(6, Rectangles(plain.Meshes[0]));
        Assert.Equal(new Vec3(-4, 3, -1), plain.Meshes[0].BoundsMin);
        Assert.Equal(new Vec3(-2, 4, 1), plain.Meshes[0].BoundsMax);

        ConversionEngine multi = EngineWith(out GeometryBuilder mb);
        multi.Settings.RenderAlgorithm = RenderAlgorithm.GreedyMulticolor;
        ObjectGeometry coloured = Build(multi, mb, WithRow("12000000"), configure: i => i.Depth = 2);
        Assert.Equal(10, Rectangles(coloured.Meshes[0]));
    }

    [Fact] // REQ-E3D-502, TV-G3
    public void Tvg3_full_tile()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        string[] full = [.. Enumerable.Repeat("11111111", 8)];
        ObjectGeometry built = Build(engine, geometry, full, configure: i => i.Depth = 8);

        Assert.Equal(6, Rectangles(built.Meshes[0]));
        Assert.Equal(new Vec3(-4, -4, -4), built.Meshes[0].BoundsMin);
        Assert.Equal(new Vec3(4, 4, 4), built.Meshes[0].BoundsMax);

        ConversionEngine striped = EngineWith(out GeometryBuilder sb);
        string[] rows = [.. Enumerable.Repeat("12121212", 8)];
        ObjectGeometry stripes = Build(striped, sb, rows, configure: i => i.Depth = 2);
        Assert.Equal(6, Rectangles(stripes.Meshes[0]));
    }

    [Fact] // REQ-E3D-502, TV-G4
    public void Tvg4_l_shape()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        string[] rows = Blank();
        rows[0] = "11000000";
        rows[1] = "10000000";
        ObjectGeometry built = Build(engine, geometry, rows);

        Assert.Equal(10, Rectangles(built.Meshes[0]));
    }

    [Fact] // REQ-E3D-424, TV-G5
    public void Tvg5_empty_colour_four_makes_the_whole_tile_solid()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        ObjectGeometry built = Build(engine, geometry, WithRow("10000000"), configure: i => i.EmptyColor = 4);

        Assert.Equal(6, Rectangles(built.Meshes[0]));
        Assert.Equal(new Vec3(-4, -4, -0.5f), built.Meshes[0].BoundsMin);
        Assert.Equal(new Vec3(4, 4, 0.5f), built.Meshes[0].BoundsMax);
    }

    [Fact] // REQ-E3D-424 (the normalisation rule)
    public void The_transparent_colour_normalisation()
    {
        Assert.Equal(0, ObjectTexture.EmptyColor(0));
        Assert.Equal(4, ObjectTexture.EmptyColor(4));
        Assert.Equal(4, ObjectTexture.EmptyColor(-1));
        Assert.Equal(0, ObjectTexture.EmptyColor(-5));
    }

    [Fact] // REQ-E3D-441, TV-G6
    public void Tvg6_full_warp_extrusion()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        ObjectGeometry built = Build(
            engine, geometry, WithRow("10000000"), configure: i =>
            {
                i.Depth = 2;
                i.FullWarp = true;
            });

        TileMesh mesh = built.Meshes[0];
        Assert.Equal(6, Rectangles(mesh));

        // The +/-Z faces span the whole depth; the side faces only the back half, z in [0, 1].
        List<Face> faces = Faces(mesh);
        Assert.Contains(faces, f => f.Normal.Equals(new Vec3(-1, 0, 0)));
        foreach (MeshVertex vertex in mesh.Vertices.ToArray())
        {
            if (Math.Abs(vertex.Normal.X) > 0.5f || Math.Abs(vertex.Normal.Y) > 0.5f)
            {
                Assert.InRange(vertex.Position.Z, 0f, 1f);
            }
        }
    }

    [Fact] // REQ-E3D-443, TV-C1
    public void Tvc1_cube()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        string[] full = [.. Enumerable.Repeat("11111111", 8)];
        ObjectGeometry built = Build(engine, geometry, full, configure: i =>
        {
            i.Geometry = Geo3DType.Cube;
            i.Depth = 4;
        });

        Assert.Equal(12, Rectangles(built.Meshes[0]));
        Assert.Equal(new Vec3(-4, -4, -2), built.Meshes[0].BoundsMin);
        Assert.Equal(new Vec3(4, 4, 2), built.Meshes[0].BoundsMax);
    }

    [Fact] // REQ-E3D-443, TV-C2
    public void Tvc2_cube_ring()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        string[] rows = new string[8];
        for (int y = 0; y < 8; y++)
        {
            rows[y] = y is 0 or 7 ? "11111111" : "10000001";
        }

        ObjectGeometry built = Build(engine, geometry, rows, configure: i =>
        {
            i.Geometry = Geo3DType.Cube;
            i.Depth = 4;
        });

        Assert.Equal(40, Rectangles(built.Meshes[0]));
    }

    [Fact] // REQ-E3D-425, TV-C3
    public void Tvc3_the_same_tile_twice_shares_one_field()
    {
        ConversionEngine same = EngineWith(out GeometryBuilder sg);
        string[] rows = [.. Enumerable.Repeat("1111111111111111", 8)];
        ObjectGeometry shared = Build(same, sg, rows, tilesWide: 2, configure: i =>
        {
            i.Geometry = Geo3DType.Cube;
            i.Depth = 2;
        });

        Assert.Equal(1, shared.Meshes.Length);
        Assert.Equal(10, Rectangles(shared.Meshes[0]));

        ConversionEngine different = EngineWith(out GeometryBuilder dg);
        string[] distinct = [.. Enumerable.Repeat("1111111122222222", 8)];
        ObjectGeometry pair = Build(different, dg, distinct, tilesWide: 2, configure: i =>
        {
            i.Geometry = Geo3DType.Cube;
            i.Depth = 2;
        });

        Assert.Equal(2, pair.Meshes.Length);
        Assert.All(pair.Meshes.ToArray(), m => Assert.Equal(8, Rectangles(m)));
    }

    [Fact] // REQ-E3D-443, TV-C4
    public void Tvc4_cube_with_one_opaque_row()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        ObjectGeometry built = Build(engine, geometry, WithRow("11111111"), configure: i =>
        {
            i.Geometry = Geo3DType.Cube;
            i.Depth = 2;
        });

        Assert.Equal(10, Rectangles(built.Meshes[0]));
    }

    [Fact] // REQ-E3D-444 to 448, TV-Y1
    public void Tvy1_vertical_cylinder_outline()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        ObjectGeometry built = Build(engine, geometry, WithRow("11111111"), configure: i =>
        {
            i.Geometry = Geo3DType.VCylinder;
            i.Depth = 8;
        });

        Assert.Equal(64, Rectangles(built.Meshes[0]));
        Assert.Equal(8, built.EffectiveDepth);
    }

    [Fact] // REQ-E3D-449, TV-Y2
    public void Tvy2_solid_vertical_cylinder()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        ObjectGeometry built = Build(engine, geometry, WithRow("11111111"), configure: i =>
        {
            i.Geometry = Geo3DType.VCylinder;
            i.Depth = 8;
            i.Solid = true;
        });

        Assert.Equal(30, Rectangles(built.Meshes[0]));
    }

    [Fact] // REQ-E3D-452, TV-Y3
    public void Tvy3_horizontal_cylinder_effective_depth()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        string[] rows = Blank();
        for (int y = 0; y < 4; y++)
        {
            rows[y] = "10000000";
        }

        ObjectGeometry built = Build(engine, geometry, rows, configure: i =>
        {
            i.Geometry = Geo3DType.HCylinder;
            i.Depth = 8;
        });

        Assert.Equal(24, Rectangles(built.Meshes[0]));
        Assert.Equal(4, built.EffectiveDepth);
    }

    [Fact] // REQ-E3D-450, TV-Y4
    public void Tvy4_solid_full_warp_cylinder()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        ObjectGeometry built = Build(engine, geometry, WithRow("11230000"), configure: i =>
        {
            i.Geometry = Geo3DType.VCylinder;
            i.Depth = 4;
            i.Solid = true;
            i.FullWarp = true;
        });

        Assert.Equal(18, Rectangles(built.Meshes[0]));
    }

    [Fact] // REQ-E3D-451, TV-Y5
    public void Tvy5_degenerate_sections()
    {
        ConversionEngine flat = EngineWith(out GeometryBuilder fg);
        ObjectGeometry single = Build(flat, fg, WithRow("11111111"), configure: i =>
        {
            i.Geometry = Geo3DType.VCylinder;
            i.Depth = 1;
        });

        Assert.Equal(6, Rectangles(single.Meshes[0]));
        Assert.Equal(-1f, single.Meshes[0].BoundsMin.X, 3);
        Assert.Equal(0f, single.Meshes[0].BoundsMax.X, 3);

        ConversionEngine dot = EngineWith(out GeometryBuilder dg);
        ObjectGeometry lone = Build(dot, dg, WithRow("10000000"), configure: i =>
        {
            i.Geometry = Geo3DType.VCylinder;
            i.Depth = 5;
        });

        Assert.Equal(6, Rectangles(lone.Meshes[0]));
        Assert.Equal(1, lone.EffectiveDepth);
    }

    [Fact] // REQ-E3D-445, REQ-E3D-447, TV-Y8
    public void Tvy8_half_cylinders()
    {
        foreach ((Geo3DType geo, bool flip) in new[]
        {
            (Geo3DType.HalfVCylinder, false),
            (Geo3DType.HalfVCylinder, true),
            (Geo3DType.HalfHCylinder, false),
        })
        {
            ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
            string[] rows = Blank();
            if (geo == Geo3DType.HalfVCylinder)
            {
                rows[0] = "11110000";
            }
            else
            {
                for (int y = 0; y < 4; y++)
                {
                    rows[y] = "10000000";
                }
            }

            ObjectGeometry built = Build(engine, geometry, rows, configure: i =>
            {
                i.Geometry = geo;
                i.Depth = 8;
                i.Flip = flip;
            });

            Assert.Equal(26, Rectangles(built.Meshes[0]));
            Assert.Equal(7, built.EffectiveDepth);
        }
    }

    [Fact] // REQ-E3D-444 (further control cases of §5.3)
    public void Further_cylinder_control_cases()
    {
        ConversionEngine three = EngineWith(out GeometryBuilder tg);
        ObjectGeometry built = Build(three, tg, WithRow("11100000"), configure: i =>
        {
            i.Geometry = Geo3DType.VCylinder;
            i.Depth = 8;
        });
        Assert.Equal(3, built.EffectiveDepth);

        ConversionEngine shallow = EngineWith(out GeometryBuilder sg);
        ObjectGeometry flat = Build(shallow, sg, WithRow("11111111"), configure: i =>
        {
            i.Geometry = Geo3DType.VCylinder;
            i.Depth = 2;
        });
        Assert.Equal(6, Rectangles(flat.Meshes[0]));
    }

    [Fact] // REQ-E3D-503, TV-G8
    public void Tvg8_multicolour_stripes()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        engine.Settings.RenderAlgorithm = RenderAlgorithm.GreedyMulticolor;
        string[] rows = [.. Enumerable.Repeat("12121212", 8)];
        ObjectGeometry built = Build(engine, geometry, rows, configure: i => i.Depth = 2);

        Assert.Equal(34, Rectangles(built.Meshes[0]));
    }

    [Fact] // REQ-E3D-504
    public void Local_positions_are_centred_on_the_tile()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        ObjectGeometry built = Build(engine, geometry, WithRow("10000000"));

        Assert.Equal(new Vec3(-4, 3, -0.5f), built.Meshes[0].BoundsMin);
        Assert.Equal(new Vec3(-3, 4, 0.5f), built.Meshes[0].BoundsMax);
    }

    [Fact] // REQ-E3D-505
    public void Triangle_winding_follows_the_normal()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        string[] rows = Blank();
        rows[0] = "11000000";
        rows[1] = "10000000";
        ObjectGeometry built = Build(engine, geometry, rows);
        TileMesh mesh = built.Meshes[0];

        ReadOnlySpan<int> triangles = mesh.Triangles;
        ReadOnlySpan<MeshVertex> vertices = mesh.Vertices;
        for (int t = 0; t < triangles.Length; t += 3)
        {
            Vec3 a = vertices[triangles[t]].Position;
            Vec3 b = vertices[triangles[t + 1]].Position;
            Vec3 c = vertices[triangles[t + 2]].Position;
            Vec3 normal = vertices[triangles[t]].Normal;

            Vec3 ab = new(b.X - a.X, b.Y - a.Y, b.Z - a.Z);
            Vec3 ac = new(c.X - a.X, c.Y - a.Y, c.Z - a.Z);
            Vec3 cross = new(
                (ab.Y * ac.Z) - (ab.Z * ac.Y),
                (ab.Z * ac.X) - (ab.X * ac.Z),
                (ab.X * ac.Y) - (ab.Y * ac.X));

            float dot = (cross.X * normal.X) + (cross.Y * normal.Y) + (cross.Z * normal.Z);
            Assert.True(dot > 0, $"triangle {t / 3} is wound against its normal");
        }
    }

    [Fact] // REQ-E3D-501
    public void Neighbouring_tiles_do_not_remove_faces_in_greedy()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        string[] rows = [.. Enumerable.Repeat("1111111122222222", 8)];
        ObjectGeometry built = Build(engine, geometry, rows, tilesWide: 2);

        Assert.Equal(2, built.Meshes.Length);
        Assert.All(built.Meshes.ToArray(), m => Assert.Equal(6, Rectangles(m)));
    }

    // ------------------------------------------------------------------ MARCHING

    [Fact] // REQ-E3D-521, REQ-E3D-522, TV-M1
    public void Tvm1_marching_octahedron()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        engine.Settings.RenderAlgorithm = RenderAlgorithm.Marching;
        ObjectGeometry built = Build(engine, geometry, WithRow("10000000"));
        TileMesh mesh = built.Meshes[0];

        int solid = 0;
        ReadOnlySpan<int> triangles = mesh.Triangles;
        ReadOnlySpan<MeshVertex> vertices = mesh.Vertices;
        for (int t = 0; t < triangles.Length; t += 3)
        {
            Vec3 a = vertices[triangles[t]].Position;
            Vec3 b = vertices[triangles[t + 1]].Position;
            Vec3 c = vertices[triangles[t + 2]].Position;
            Vec3 ab = new(b.X - a.X, b.Y - a.Y, b.Z - a.Z);
            Vec3 ac = new(c.X - a.X, c.Y - a.Y, c.Z - a.Z);
            Vec3 cross = new(
                (ab.Y * ac.Z) - (ab.Z * ac.Y),
                (ab.Z * ac.X) - (ab.X * ac.Z),
                (ab.X * ac.Y) - (ab.Y * ac.X));
            if (Math.Abs(cross.X) + Math.Abs(cross.Y) + Math.Abs(cross.Z) > 1e-4)
            {
                solid++;
            }
        }

        Assert.Equal(8, solid);
        Assert.Equal(new Vec3(-4.5f, 2.5f, -1), mesh.BoundsMin);
        Assert.Equal(new Vec3(-2.5f, 4.5f, 1), mesh.BoundsMax);

        // Every vertex carries the texel centre (0.5, 0.5).
        foreach (MeshVertex vertex in mesh.Vertices.ToArray())
        {
            Assert.Equal(0.5f, vertex.Uv.X, 3);
            Assert.Equal(0.5f, vertex.Uv.Y, 3);
        }
    }

    [Fact] // REQ-E3D-521, TV-M2
    public void Tvm2_two_adjacent_pixels()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        engine.Settings.RenderAlgorithm = RenderAlgorithm.Marching;
        ObjectGeometry built = Build(engine, geometry, WithRow("11000000"));
        TileMesh mesh = built.Meshes[0];

        Assert.Equal(new Vec3(-4.5f, 2.5f, -1), mesh.BoundsMin);
        Assert.Equal(new Vec3(-1.5f, 4.5f, 1), mesh.BoundsMax);
    }

    [Fact] // REQ-E3D-520, TV-M3
    public void Tvm3_marching_tiles_join_without_a_wall()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        engine.Settings.RenderAlgorithm = RenderAlgorithm.Marching;
        string[] rows = [.. Enumerable.Repeat("1111111111111111", 8)];
        ObjectGeometry built = Build(engine, geometry, rows, tilesWide: 2, configure: i => i.Depth = 2);

        // One shared mesh (the same tile twice) with no face in the junction plane x = 4.
        TileMesh mesh = built.Meshes[0];
        Assert.DoesNotContain(
            mesh.Vertices.ToArray(),
            v => Math.Abs(v.Position.X - 4f) < 1e-3 && Math.Abs(v.Normal.X) > 0.9f);
    }

    [Fact] // REQ-E3D-524
    public void A_mesh_beyond_65535_vertices_is_accepted()
    {
        // The criterion injects a 3D chequerboard of 8 x 8 x 96, one voxel in two solid.
        TileVoxels field = new(96, perAxisSlots: true);
        for (int k = 0; k < 96; k++)
        {
            for (int j = 0; j < 8; j++)
            {
                for (int i = 0; i < 8; i++)
                {
                    if ((i + j + k) % 2 == 0)
                    {
                        field.Write(i, j, k, TileVoxels.AllAxes, 0, 0, transparent: false);
                    }
                }
            }
        }

        ConversionEngine engine = new();
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);
        ObjectTexture texture = ObjectTexture.FromPattern(pattern);
        TileMesh mesh = GreedyMesher.Build(field, texture, multicolor: false);

        Assert.Equal(18_432, Rectangles(mesh));
        Assert.Equal(73_728, mesh.VertexCount);
        Assert.True(mesh.VertexCount > 65_535);
    }

    // ------------------------------------------------------------------ 3.5, 3.6

    [Fact] // REQ-E3D-541
    public void A_pattern_with_two_objects_builds_two_geometries()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);
        PatternInstance second = new()
        {
            Depth = 5,
            Geometry = Geo3DType.Cube,
            Layer = ZLayer.L2,
            Scale = new Vec3(1, 1, 1),
            Pivot = new Vec3(0.5f, 0.5f, 0.5f),
            DeformPivot = new Vec3(0.5f, 0.5f, 0.5f),
            Alpha = 1,
            Style = RenderStyle.Normal,
            CustomPalette = -1,
            TexTilting = new Vec2(1, 1),
            FillRate = 1,
        };

        pattern.AddInstance(second);
        geometry.Build(pattern);
        geometry.Commit();

        Assert.NotNull(geometry.Of(pattern.Instances[0]));
        Assert.NotNull(geometry.Of(second));
        Assert.NotSame(geometry.Of(pattern.Instances[0]), geometry.Of(second));

        ConversionFrame frame = new SyntheticBatch().Background('A', 80, 80).Convert(engine);
        Assert.Equal(2, frame.Instances.Count);
    }

    [Fact] // REQ-E3D-542
    public void Changing_an_instance_parameter_does_not_rebuild_the_mesh()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);
        geometry.Build(pattern);
        geometry.Commit();

        int builds = geometry.BuildCount;
        ConversionFrame frame = new SyntheticBatch().Background('A', 80, 80).Convert(engine);
        FrameInstance instance = frame.Instances[0];
        TileMesh? before = geometry.MeshOf(instance, instance.Shape.Tiles[0].Image);

        instance.Depth = 32;
        instance.Alpha = 0.5f;
        instance.CustomPalette = 7;
        engine.CalculatePositions(frame);

        Assert.Equal(builds, geometry.BuildCount);
        Assert.Same(before, geometry.MeshOf(instance, instance.Shape.Tiles[0].Image));
    }

    [Fact] // REQ-E3D-560, REQ-E3D-561, TV-K1
    public void Tvk1_indexed_colour_resolution()
    {
        ConversionEngine engine = new();
        SyntheticBatch batch = new();
        byte[] core = new byte[32];
        core[0] = 0x0F;
        for (int i = 1; i < 32; i++)
        {
            core[i] = (byte)i;
            batch.Palette(i, (byte)i);
        }

        batch.Palette(0, 0x0F);
        ConversionFrame frame = batch.Convert(engine);

        Assert.Equal(core[6], frame.Palette[(4 * 1) + 2]);
        Assert.Equal(core[22], frame.Palette[(4 * 5) + 2]);
        Assert.Equal(core[0], frame.Palette[4 * 1]);
        Assert.Equal(core[6], frame.Palette[(4 * 9) + 2]);
    }

    [Fact] // REQ-E3D-561
    public void The_frame_palette_mirrors_rows_8_to_15()
    {
        ConversionEngine engine = new();
        SyntheticBatch batch = new();
        for (int i = 0; i < 32; i++)
        {
            batch.Palette(i, (byte)(0x10 + i));
        }

        ConversionFrame frame = batch.Convert(engine);
        Assert.Equal(frame.Palette[0], frame.Palette[36]);
        Assert.Equal(frame.Palette[6], frame.Palette[38]);
        Assert.Equal(frame.Palette[22], frame.Palette[54]);
    }

    [Fact] // REQ-E3D-563
    public void Texture_sampling_defaults()
    {
        PatternInstance instance = new();
        Assert.Equal(new Vec2(1, 1), instance.TexTilting);
        Assert.Equal(new Vec2(0, 0), instance.TexOffset);
    }

    [Fact] // REQ-E3D-564
    public void Every_embedded_profile_loads_without_a_texture_capacity_error()
    {
        foreach (string path in NeutralAssets.ProfilePaths)
        {
            GameProfile profile = ProfileFormat.Load(File.ReadAllBytes(path), sourceName: path).Profile!;
            foreach (ProfilePattern pattern in profile.PermanentPatterns.Concat(profile.TemporaryPatterns))
            {
                foreach (PatternVariant variant in pattern.Variants)
                {
                    if (variant.Texture is { IsNullMarker: false } texture)
                    {
                        Assert.InRange(texture.Width, 0, 512);
                        Assert.InRange(texture.Height, 0, 512);
                    }
                }
            }
        }
    }

    [Theory] // REQ-E3D-566
    [InlineData(1f, RenderStyle.Normal, MaterialClass.Opaque)]
    [InlineData(0.5f, RenderStyle.Normal, MaterialClass.Transparent)]
    [InlineData(0.5f, RenderStyle.ShadowOnly, MaterialClass.ShadowOnly)]
    [InlineData(1f, RenderStyle.ShadowOnly, MaterialClass.ShadowOnly)]
    public void Material_classes(float alpha, RenderStyle style, MaterialClass expected)
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);
        pattern.Instances[0].Alpha = alpha;
        pattern.Instances[0].Style = style;
        geometry.Build(pattern);
        geometry.Commit();

        ConversionFrame frame = new SyntheticBatch().Background('A', 80, 80).Convert(engine);
        engine.Publish();
        FrameDrawList list = engine.BuildDrawList(frame);

        DrawTile tile = Assert.Single(list.Tiles);
        Assert.Equal(expected, tile.Material);
        Assert.Equal(alpha, tile.Alpha);
        if (style == RenderStyle.ShadowOnly)
        {
            Assert.True(tile.CastShadow);
        }
    }

    [Fact] // REQ-E3D-543
    public void A_disabled_object_is_not_drawn()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);
        pattern.Instances[0].Style = RenderStyle.Hidden;
        geometry.Build(pattern);
        geometry.Commit();

        ConversionFrame frame = new SyntheticBatch().Background('A', 80, 80).Convert(engine);
        engine.Publish();
        Assert.Empty(engine.BuildDrawList(frame).Tiles);
    }

    [Fact] // REQ-E3D-567
    public void Shadow_flags()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        EnginePattern l1 = SyntheticPatterns.Build(engine, ["A"]);
        EnginePattern l2 = SyntheticPatterns.Build(engine, ["B"]);
        l2.Instances[0].Layer = ZLayer.L2;

        EnginePattern noCast = SyntheticPatterns.Build(engine, ["C"], tags: "!cast");
        EnginePattern ui = SyntheticPatterns.Build(engine, ["D"]);
        ui.Instances[0].Layer = ZLayer.Ui;

        geometry.Build(l1);
        geometry.Build(l2);
        geometry.Build(noCast);
        geometry.Build(ui);
        geometry.Commit();

        ConversionFrame frame = new SyntheticBatch()
            .Background('A', 16, 16)
            .Background('B', 48, 16)
            .Background('C', 80, 16)
            .Background('D', 112, 16)
            .Convert(engine);
        engine.Publish();

        FrameDrawList list = engine.BuildDrawList(frame);
        DrawTile Find(ZLayer layer, string tag)
            => list.Tiles.First(t => t.Instance.Layer == layer && t.Instance.TagString == tag);

        DrawTile first = Find(ZLayer.L1, string.Empty);
        Assert.True(first.CastShadow);
        Assert.True(first.ReceiveShadow);

        DrawTile second = Find(ZLayer.L2, string.Empty);
        Assert.False(second.CastShadow);
        Assert.True(second.ReceiveShadow);

        DrawTile third = Find(ZLayer.L1, "!cast");
        Assert.False(third.CastShadow);

        DrawTile fourth = Find(ZLayer.Ui, string.Empty);
        Assert.False(fourth.CastShadow);
        Assert.False(fourth.ReceiveShadow);
        Assert.Equal(SceneContainer.Interface, fourth.Container);
        Assert.Equal(MaterialClass.Interface, fourth.Material);
    }

    [Fact] // REQ-E3D-569
    public void The_fourth_matrix_row_does_not_move_a_vertex()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);
        PatternInstance instance = pattern.Instances[0];
        instance.DeformAmplitude = new Vec3(0, 0.1f, 0);
        geometry.Build(pattern);
        geometry.Commit();

        ConversionFrame frame = new SyntheticBatch().Background('A', 16, 16).Convert(engine);
        engine.Publish();
        DeformParameters? before = engine.BuildDrawList(frame).Tiles[0].Deform;

        for (int i = 0; i < 8; i++)
        {
            instance.SetCustomData(i, 1.5f + i);
        }

        ConversionFrame after = new SyntheticBatch().Background('A', 16, 16).Convert(engine, 2);
        engine.Publish();
        DeformParameters? later = engine.BuildDrawList(after).Tiles[0].Deform;

        Assert.NotNull(before);
        Assert.NotNull(later);
        Vec3 v = new(1, 2, 3);
        Assert.Equal(before!.Value.Displacement(v, 0f).X, later!.Value.Displacement(v, 0f).X, 5);
        Assert.Equal(before.Value.Displacement(v, 0f).Y, later.Value.Displacement(v, 0f).Y, 5);
        Assert.Equal(before.Value.Displacement(v, 0f).Z, later.Value.Displacement(v, 0f).Z, 5);
    }

    // ------------------------------------------------------------------ 3.7, 3.8

    [Fact] // REQ-E3D-600, REQ-E3D-362
    public void Loading_progress_follows_the_documented_formula()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        geometry.DeferTransfer = true;
        geometry.BeginMassBuild(100);
        Assert.Equal(0f, geometry.Progress.Value, 4);

        List<EnginePattern> patterns = [];
        for (int i = 0; i < 100; i++)
        {
            patterns.Add(SyntheticPatterns.Build(engine, [((char)('a' + (i % 26))).ToString() + i]));
        }

        for (int i = 0; i < 50; i++)
        {
            geometry.Build(patterns[i]);
        }

        Assert.Equal(0.25f, geometry.Progress.Value, 4);

        for (int i = 50; i < 100; i++)
        {
            geometry.Build(patterns[i]);
        }

        geometry.Commit();
        Assert.Equal(1f, geometry.Progress.Value, 4);
    }

    [Fact] // REQ-E3D-362 (1 outside loading)
    public void Progress_is_one_when_nothing_is_being_built()
        => Assert.Equal(1f, new GeometryProgress(0, 0, 0).Value);

    [Fact] // REQ-E3D-601, REQ-E3D-543
    public void An_object_whose_meshes_are_not_ready_is_not_drawn()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        geometry.DeferTransfer = true;
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);
        geometry.Build(pattern);

        ConversionFrame frame = new SyntheticBatch().Background('A', 80, 80).Convert(engine);
        engine.Publish();
        Assert.Single(frame.Shapes);
        Assert.Empty(engine.BuildDrawList(frame).Tiles);

        geometry.Commit();
        ConversionFrame later = new SyntheticBatch().Background('A', 80, 80).Convert(engine, 2);
        engine.Publish();
        Assert.Single(engine.BuildDrawList(later).Tiles);
    }

    [Fact] // REQ-E3D-603
    public void Releasing_a_pattern_frees_its_geometry()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);
        geometry.Build(pattern);
        geometry.Commit();
        Assert.NotNull(geometry.Of(pattern.Instances[0]));

        geometry.Release(pattern);
        Assert.Null(geometry.Of(pattern.Instances[0]));
    }

    [Fact] // REQ-E3D-640
    public void A_mesh_exposes_a_platform_independent_description()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        ObjectGeometry built = Build(engine, geometry, WithRow("10000000"));
        TileMesh mesh = built.Meshes[0];

        Assert.Equal(1, TileMesh.SubmeshCount);
        Assert.Equal(mesh.TriangleCount * 3, mesh.Triangles.Length);
        Assert.True(mesh.VertexCount > 0);
        Assert.True(mesh.BoundsMin.X <= mesh.BoundsMax.X);
    }

    [Fact] // REQ-E3D-641, REQ-E3D-642, TV-W2
    public void Tvw2_the_draw_list_of_a_sprite_tile()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);
        pattern.Instances[0].Depth = 4;
        geometry.Build(pattern);
        geometry.Commit();

        ConversionFrame background = new SyntheticBatch().Background('A', 16, 32).Convert(engine, 1);
        engine.Publish();
        DrawTile backgroundTile = Assert.Single(engine.BuildDrawList(background).Tiles);

        ConversionFrame sprite = new SyntheticBatch().Sprite('A', 16, 32).Convert(engine, 2);
        engine.Publish();
        DrawTile spriteTile = Assert.Single(engine.BuildDrawList(sprite).Tiles);

        // The transform handed over is the same; the renderer applies the +/-0.1 offset.
        Assert.Equal(backgroundTile.Transform, spriteTile.Transform);
        Assert.False(backgroundTile.Sprite);
        Assert.True(spriteTile.Sprite);
        Assert.False(spriteTile.BehindBackground);

        Assert.Equal(SceneContainer.Game, backgroundTile.Container);
        Assert.NotNull(backgroundTile.Texture);
        Assert.InRange(backgroundTile.PaletteTable, 0, 15);
    }

    [Fact] // REQ-E3D-641 (the sprite priority flag)
    public void A_sprite_behind_the_background_is_flagged()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);
        geometry.Build(pattern);
        geometry.Commit();

        ConversionFrame frame = new SyntheticBatch().Sprite('A', 16, 32, hidden: true).Convert(engine);
        engine.Publish();
        DrawTile tile = Assert.Single(engine.BuildDrawList(frame).Tiles);

        Assert.True(tile.Sprite);
        Assert.True(tile.BehindBackground);
    }

    [Fact] // REQ-E3D-641 (the frame palette and the time come once per frame)
    public void The_draw_list_carries_the_palette_and_the_time()
    {
        ConversionEngine engine = EngineWith(out GeometryBuilder geometry);
        SyntheticPatterns.Build(engine, ["A"]);
        ConversionFrame frame = new SyntheticBatch().Palette(1, 0x16).Background('A', 16, 32).Convert(engine, 120);
        engine.Publish();

        FrameDrawList list = engine.BuildDrawList(frame);
        Assert.Equal(64, list.Palette.Length);
        Assert.Equal(120, list.FrameCounter);
        Assert.Equal(2.0f, list.Time, 4);
    }
}
