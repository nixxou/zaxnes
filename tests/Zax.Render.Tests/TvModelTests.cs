using System.Buffers.Binary;
using System.Globalization;
using System.Numerics;
using System.Text;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;
using Zax.Render.Assets;

namespace Zax.Render.Tests;

/// <summary>
/// The television of AMD-025: the binary glTF reader, the two adaptations the amendment asks
/// for — the screen plates are not drawn and the opening frames the visible rectangle — the
/// shadow rule of AMD-021 R2-014, and the behaviour when the model file is missing.
/// </summary>
/// <remarks>
/// Everything here but the three tests guarded by <see cref="TestTelevision.Available"/> runs on
/// a <c>.glb</c> built in memory by <see cref="Glb"/>: a cabinet pierced by a rectangular hole,
/// a small second part and two screen plates of <see cref="TvModel.ScreenPlateVertices"/>
/// vertices, which is the shape AMD-025 describes.
/// </remarks>
[Collection(GlCollection.Name)]
public sealed class TvModelTests
{
    private static readonly Vec4F Default = new(0.1f, 8.1f, 8.1f, 0.1f);

    /// <summary>The hole in the synthetic cabinet, in model (z, y).</summary>
    private static readonly Rect Hole = new(-0.6f, -0.45f, 0.6f, 0.45f);

    [Fact] // AMD-025
    public void AMD_025_the_gltf_reader_reads_the_meshes_the_transforms_and_the_materials()
    {
        GltfModel model = GltfBinary.Decode(Glb.Television());

        // Four primitives: the cabinet, the small part and the two screen plates.
        Assert.Equal(4, model.Parts.Count);
        Assert.Equal(2, model.Parts.Count(part => part.VertexCount == TvModel.ScreenPlateVertices));

        // The node transform is baked in: the cabinet node is translated by +0.25 in y, so its
        // geometry comes out there and not at the origin.
        GltfPart cabinet = model.Parts[0];
        Assert.Equal(Glb.CabinetShift + Hole.MinY - Glb.BorderY, cabinet.Mesh.BoundsMin.Y, 4);
        Assert.Equal(Glb.CabinetShift + Hole.MaxY + Glb.BorderY, cabinet.Mesh.BoundsMax.Y, 4);

        // Normals survive the transform, and the texture coordinates come through unchanged.
        Assert.Equal(-1f, cabinet.Mesh.Vertices[0].Normal.X, 4);
        Assert.Equal(2, Assert.Single(model.Images).Width);

        // The metallic-roughness material of glTF 2.0 §3.9.2, with the two extensions the
        // television file declares.
        GltfMaterial material = cabinet.Material;
        Assert.Equal(0, material.BaseColorTexture);
        Assert.Equal(0.25f, material.MetallicFactor, 4);
        Assert.Equal(0.6f, material.RoughnessFactor, 4);
        Assert.Equal(0, material.NormalTexture);
        Assert.True(material.DoubleSided);

        // The screen plate material carries the emissive strength extension.
        GltfMaterial screen = model.Parts[2].Material;
        Assert.Equal(new Vector3(1f, 1f, 1f), screen.EmissiveFactor);
        Assert.Equal(2.5f, screen.EmissiveStrength, 4);
    }

    [Fact] // AMD-025
    public void AMD_025_the_two_screen_plates_are_not_drawn()
    {
        TvModel television = Load();

        Assert.Equal(2, television.HiddenParts);
        Assert.Equal(2, television.Parts.Count);
        Assert.All(television.Parts, part =>
            Assert.NotEqual(TvModel.ScreenPlateVertices, part.Mesh.Vertices.Length));
    }

    [Fact] // AMD-025, AMD-021 R2-014
    public void AMD_025_the_cabinet_casts_and_receives_nothing_and_the_buttons_do_both()
    {
        TvModel television = Load();

        TvModelPart cabinet = television.Parts.MaxBy(part => part.Mesh.Vertices.Length)!;
        Assert.False(cabinet.CastShadow);
        Assert.False(cabinet.ReceiveShadow);

        Assert.All(television.Parts.Where(part => !ReferenceEquals(part, cabinet)), part =>
        {
            Assert.True(part.CastShadow);
            Assert.True(part.ReceiveShadow);
        });
    }

    [Fact] // AMD-025
    public void AMD_025_the_opening_is_measured_on_the_cabinet_and_not_on_the_plates()
    {
        TvModel television = Load();

        // The hole of the synthetic cabinet, to within a cell of the measuring grid. The
        // whole model hangs under a node translated by CabinetShift in y, which the reader
        // has already applied, so the opening is the hole moved by as much.
        // The measuring grid has a cell of about a hundredth of the model, which is the
        // accuracy of an edge.
        const float cell = 0.02f;
        Assert.Equal(Hole.MinX, television.Opening.MinX, cell);
        Assert.Equal(Hole.MaxX, television.Opening.MaxX, cell);
        Assert.Equal(Hole.MinY + Glb.CabinetShift, television.Opening.MinY, cell);
        Assert.Equal(Hole.MaxY + Glb.CabinetShift, television.Opening.MaxY, cell);

        // The plates are wider than the hole; the opening is the hole, not the plates.
        Assert.True(television.Opening.MaxX - television.Opening.MinX < Glb.PlateWidth);
    }

    [Theory] // AMD-025
    [InlineData(0.1f, 8.1f, 8.1f, 0.1f)]
    [InlineData(0f, 0f, 0f, 0f)]
    [InlineData(16.1f, 7.9f, 24.1f, 0.5f)]
    public void AMD_025_the_opening_frames_exactly_the_visible_rectangle(float c0, float c1, float c2, float c3)
    {
        TvModel television = Load();
        Vec4F clipping = new(c0, c1, c2, c3);
        VisibleRect rect = VisibleRect.From(clipping);
        Mat4 placement = television.Placement(clipping);
        Rect opening = television.Opening;

        // The four corners of the opening land on the four corners of the visible rectangle,
        // whatever the clip margins — including the asymmetric ones the TV frame of
        // REQ-SCN-054 could only approximate.
        Vec3 low = placement.TransformPoint(new Vec3(ScreenPlane(television), opening.MinY, opening.MinX));
        Vec3 high = placement.TransformPoint(new Vec3(ScreenPlane(television), opening.MaxY, opening.MaxX));

        Assert.Equal(rect.MinX, MathF.Min(low.X, high.X), 2);
        Assert.Equal(rect.MaxX, MathF.Max(low.X, high.X), 2);
        Assert.Equal(rect.MinY, MathF.Min(low.Y, high.Y), 2);
        Assert.Equal(rect.MaxY, MathF.Max(low.Y, high.Y), 2);

        // The opening sits in the plane of the tunnel's front face, so the tunnel of
        // REQ-SCN-053 stands right behind it.
        Assert.Equal(TvModel.TunnelFront, low.Z, 2);

        // The cabinet covers the opening and reaches past it on all four sides.
        VisibleRect footprint = television.Footprint(clipping);
        Assert.True(footprint.MinX < rect.MinX);
        Assert.True(footprint.MaxX > rect.MaxX);
        Assert.True(footprint.MinY < rect.MinY);
        Assert.True(footprint.MaxY > rect.MaxY);
    }

    [Fact] // AMD-025
    public void AMD_025_the_placement_does_not_mirror_what_the_camera_sees()
    {
        TvModel television = Load();
        Mat4 placement = television.Placement(Default);

        // The scene frame of REQ-E3D-410 is left-handed and a glTF file is right-handed, so
        // the placement has to reverse the orientation — otherwise every piece of lettering
        // on the cabinet would read back to front. The renderer flips the winding and the
        // normals on that sign.
        float determinant =
            (placement[0, 0] * ((placement[1, 1] * placement[2, 2]) - (placement[1, 2] * placement[2, 1])))
            - (placement[0, 1] * ((placement[1, 0] * placement[2, 2]) - (placement[1, 2] * placement[2, 0])))
            + (placement[0, 2] * ((placement[1, 0] * placement[2, 1]) - (placement[1, 1] * placement[2, 0])));
        Assert.True(determinant < 0f);

        // Model +Y is scene +Y: the set is never upside down.
        Vec3 bottom = placement.TransformPoint(new Vec3(0f, 0f, 0f));
        Vec3 top = placement.TransformPoint(new Vec3(0f, 1f, 0f));
        Assert.True(top.Y > bottom.Y);
    }

    [Fact] // AMD-025
    public void AMD_025_a_missing_or_broken_model_file_gives_no_television()
    {
        Assert.Null(TvModel.TryLoad(null));
        Assert.Null(TvModel.TryLoad(string.Empty));
        Assert.Null(TvModel.TryLoad(Path.Combine(Path.GetTempPath(), "zax-no-such-model.glb")));

        string broken = Path.Combine(Path.GetTempPath(), $"zax-broken-{Guid.NewGuid():N}.glb");
        try
        {
            File.WriteAllBytes(broken, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
            Assert.Null(TvModel.TryLoad(broken));
            Assert.Null(GltfBinary.TryLoad(broken));
            Assert.Throws<InvalidDataException>(() => GltfBinary.Decode(File.ReadAllBytes(broken)));
        }
        finally
        {
            File.Delete(broken);
        }
    }

    [GlFact] // AMD-025
    public void AMD_025_the_model_renders_and_leaves_the_whole_picture_visible()
    {
        using GlFixture gl = GlFixture.Create(240, 240);
        using SceneRenderer renderer = new(gl.Gl, null, Load());

        // A white NES plane filling the whole frame: what the opening shows must be that
        // plane, not a cabinet.
        SceneState scene = new SceneFixture()
            .Background(0x22)
            .Palette(1, 0x30)
            .Rect(SceneFixture.Solid(1), 0, 0, 32, 30)
            .Build();
        scene.Settings.Shadows = false;
        scene.Settings.TvBox = TvBoxKind.Tv;

        Capture capture = gl.Render(renderer, scene);

        Assert.Equal(2, renderer.Statistics.TvParts);
        Assert.Equal(TvBox.TunnelBoxCount, renderer.Statistics.TunnelBoxes);

        // The lit NES plane is NES 0x30, sRGB (223, 219, 223): the centre of the image is the
        // picture, so the visible rectangle is not masked by the cabinet or by a screen plate.
        (int r, int g, int b) = capture.At(capture.Width / 2, capture.Height / 2);
        Assert.InRange(r, 215, 231);
        Assert.InRange(g, 211, 227);
        Assert.InRange(b, 215, 231);
    }

    [GlFact] // AMD-025, AMD-021 R2-014
    public void AMD_025_only_the_buttons_fill_the_shadow_map()
    {
        using GlFixture gl = GlFixture.Create(160, 160);
        using SceneRenderer renderer = new(gl.Gl, null, Load());

        SceneState scene = new SceneFixture().Background(0x22).Build();
        scene.Settings.TvBox = TvBoxKind.Tv;
        scene.Settings.Shadows = true;
        gl.Render(renderer, scene);

        // One caster, the small part: the cabinet is not one of them, and the frame holds no
        // tile at all.
        Assert.True(renderer.Statistics.ShadowPass);
        Assert.Equal(1, renderer.Statistics.ShadowCasters);

        // With the shadows off nothing casts.
        scene.Settings.Shadows = false;
        gl.Render(renderer, scene);
        Assert.Equal(0, renderer.Statistics.ShadowCasters);
    }

    // ------------------------------------------------------- the shipped model, when present

    [Fact] // AMD-025
    public void AMD_025_the_shipped_model_holds_four_meshes_and_six_textures()
    {
        if (TestTelevision.Gltf is not { } model)
        {
            return;
        }

        Assert.Equal(4, model.Parts.Count);
        Assert.Equal(6, model.Images.Count);

        // AMD-025 names the two screen plates by their vertex count.
        Assert.Equal(2, model.Parts.Count(part => part.VertexCount == TvModel.ScreenPlateVertices));

        // Five PNG and one JPEG, all decoded to usable images.
        Assert.All(model.Images, image =>
        {
            Assert.True(image.Width > 1);
            Assert.True(image.Height > 1);
            Assert.Equal(image.Width * image.Height * 4, image.Pixels.Length);
        });
    }

    [Fact] // AMD-025
    public void AMD_025_the_shipped_model_hides_its_screen_and_opens_on_the_visible_rectangle()
    {
        if (TestTelevision.Model is not { } television)
        {
            return;
        }

        Assert.Equal(2, television.HiddenParts);
        Assert.Equal(2, television.Parts.Count);

        // The opening is a slice of the front of the set, not the whole of it.
        float openWidth = television.Opening.MaxX - television.Opening.MinX;
        float frontWidth = television.Silhouette.MaxX - television.Silhouette.MinX;
        Assert.InRange(openWidth / frontWidth, 0.5f, 0.95f);

        // Fitted on the visible rectangle, the cabinet hides the whole of the tunnel of
        // REQ-SCN-053, whose outer bounds are x in [-8, 264] and y in [-4, 244].
        VisibleRect footprint = television.Footprint(Default);
        Assert.True(footprint.MinX <= -8f);
        Assert.True(footprint.MaxX >= 264f);
        Assert.True(footprint.MinY <= -4f);
        Assert.True(footprint.MaxY >= 244f);
    }

    [GlFact] // AMD-025
    public void AMD_025_the_shipped_model_renders_and_does_not_mask_the_picture()
    {
        if (TestTelevision.Model is not { } television)
        {
            return;
        }

        using GlFixture gl = GlFixture.Create(256, 256);
        using SceneRenderer renderer = new(gl.Gl, null, television);

        SceneState scene = new SceneFixture()
            .Background(0x22)
            .Palette(1, 0x30)
            .Rect(SceneFixture.Solid(1), 0, 0, 32, 30)
            .Build();
        scene.Settings.Shadows = false;
        scene.Settings.TvBox = TvBoxKind.Tv;

        Capture capture = gl.Render(renderer, scene);

        Assert.Equal(television.Parts.Count, renderer.Statistics.TvParts);
        (int r, int g, int b) = capture.At(capture.Width / 2, capture.Height / 2);
        Assert.InRange(r, 215, 231);
        Assert.InRange(g, 211, 227);
        Assert.InRange(b, 215, 231);
    }

    private static TvModel Load()
        => TvModel.From(GltfBinary.Decode(Glb.Television()))
            ?? throw new InvalidOperationException("The synthetic model should load.");

    /// <summary>The model x the opening is measured in: the front of the screen plates.</summary>
    private static float ScreenPlane(TvModel television)
    {
        _ = television;
        return Glb.PlateFront;
    }

    /// <summary>
    /// A binary glTF 2.0 file built in memory, in the shape AMD-025 describes: a cabinet with a
    /// rectangular hole, a small second part, and two superposed screen plates of 49 vertices.
    /// </summary>
    private static class Glb
    {
        /// <summary>Translation of the cabinet node, which the reader has to apply.</summary>
        public const float CabinetShift = 0.25f;

        /// <summary>Width of the cabinet around the hole.</summary>
        public const float BorderX = 0.4f;

        /// <summary>Height of the cabinet around the hole.</summary>
        public const float BorderY = 0.35f;

        /// <summary>Model x of the front plate, the plane the opening is measured in.</summary>
        public const float PlateFront = 0.06f;

        /// <summary>Width of a screen plate, wider than the hole it sits behind.</summary>
        public const float PlateWidth = 1.24f;

        public static byte[] Television()
        {
            Builder builder = new();

            // The cabinet: four quads making a ring in the (z, y) plane, facing −x.
            List<(Vector3 A, Vector3 B, Vector3 C, Vector3 D)> ring = [];
            float outerZ = Hole.MaxX + BorderX;
            float outerY = Hole.MaxY + BorderY;
            Quad(ring, -outerZ, -outerY, Hole.MinX, outerY);
            Quad(ring, Hole.MaxX, -outerY, outerZ, outerY);
            Quad(ring, Hole.MinX, -outerY, Hole.MaxX, Hole.MinY);
            Quad(ring, Hole.MinX, Hole.MaxY, Hole.MaxX, outerY);
            int cabinet = builder.AddMesh(Quads(ring, 0.12f), material: 0);

            // The small second part, somewhere behind the front: the "buttons" of AMD-025.
            List<(Vector3, Vector3, Vector3, Vector3)> knob = [];
            Quad(knob, -0.1f, -0.7f, 0.1f, -0.55f);
            int buttons = builder.AddMesh(Quads(knob, -0.3f), material: 0);

            // The two screen plates, 49 vertices each, wider than the hole.
            int glass = builder.AddMesh(Plate(0.05f), material: 1);
            int picture = builder.AddMesh(Plate(PlateFront), material: 1);

            return builder.Build(cabinet, buttons, glass, picture);
        }

        private static void Quad(
            List<(Vector3, Vector3, Vector3, Vector3)> quads,
            float minZ,
            float minY,
            float maxZ,
            float maxY)
            => quads.Add((
                new Vector3(0f, minY, minZ),
                new Vector3(0f, minY, maxZ),
                new Vector3(0f, maxY, maxZ),
                new Vector3(0f, maxY, minZ)));

        private static Mesh Quads(List<(Vector3 A, Vector3 B, Vector3 C, Vector3 D)> quads, float x)
        {
            List<Vector3> positions = [];
            List<Vector3> normals = [];
            List<Vector2> uvs = [];
            List<int> indices = [];

            foreach ((Vector3 a, Vector3 b, Vector3 c, Vector3 d) in quads)
            {
                int first = positions.Count;
                foreach (Vector3 corner in new[] { a, b, c, d })
                {
                    positions.Add(corner with { X = x });
                    normals.Add(new Vector3(-1f, 0f, 0f));
                    uvs.Add(new Vector2(corner.Z, corner.Y));
                }

                indices.AddRange([first, first + 1, first + 2, first, first + 2, first + 3]);
            }

            return new Mesh([.. positions], [.. normals], [.. uvs], [.. indices]);
        }

        /// <summary>A 7 × 7 grid of vertices: exactly 49, the count AMD-025 names.</summary>
        private static Mesh Plate(float x)
        {
            const int side = 7;
            List<Vector3> positions = [];
            List<Vector3> normals = [];
            List<Vector2> uvs = [];
            List<int> indices = [];

            for (int row = 0; row < side; row++)
            {
                for (int column = 0; column < side; column++)
                {
                    float u = column / (float)(side - 1);
                    float v = row / (float)(side - 1);
                    positions.Add(new Vector3(
                        x,
                        -0.47f + (0.94f * v),
                        (-PlateWidth / 2f) + (PlateWidth * u)));
                    normals.Add(new Vector3(-1f, 0f, 0f));
                    uvs.Add(new Vector2(u, v));
                }
            }

            for (int row = 0; row + 1 < side; row++)
            {
                for (int column = 0; column + 1 < side; column++)
                {
                    int at = (row * side) + column;
                    indices.AddRange([at, at + 1, at + side + 1, at, at + side + 1, at + side]);
                }
            }

            return new Mesh([.. positions], [.. normals], [.. uvs], [.. indices]);
        }

        private readonly record struct Mesh(
            Vector3[] Positions,
            Vector3[] Normals,
            Vector2[] Uvs,
            int[] Indices);

        private sealed class Builder
        {
            private readonly List<byte> _bin = [];
            private readonly List<string> _views = [];
            private readonly List<string> _accessors = [];
            private readonly List<string> _meshes = [];

            public int AddMesh(Mesh mesh, int material)
            {
                int position = AddFloats(
                    [.. mesh.Positions.SelectMany(v => new[] { v.X, v.Y, v.Z })], 3, "VEC3", mesh.Positions.Length);
                int normal = AddFloats(
                    [.. mesh.Normals.SelectMany(v => new[] { v.X, v.Y, v.Z })], 3, "VEC3", mesh.Normals.Length);
                int uv = AddFloats(
                    [.. mesh.Uvs.SelectMany(v => new[] { v.X, v.Y })], 2, "VEC2", mesh.Uvs.Length);
                int index = AddIndices(mesh.Indices);

                _meshes.Add(string.Create(
                    CultureInfo.InvariantCulture,
                    $$"""{"primitives":[{"attributes":{"POSITION":{{position}},"NORMAL":{{normal}},"TEXCOORD_0":{{uv}}},"indices":{{index}},"material":{{material}},"mode":4}]}"""));
                return _meshes.Count - 1;
            }

            public byte[] Build(int cabinet, int buttons, int glass, int picture)
            {
                byte[] png = PngImage.EncodeRgb(2, 2, [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120]);
                int image = AddBytes(png);

                string json = string.Create(
                    CultureInfo.InvariantCulture,
                    $$"""
                    {
                      "asset": {"version": "2.0", "generator": "Zaxnes tests"},
                      "extensionsUsed": ["KHR_materials_emissive_strength"],
                      "scene": 0,
                      "scenes": [{"nodes": [0]}],
                      "nodes": [
                        {"name": "root", "children": [1, 2, 3, 4], "translation": [0, {{CabinetShift.ToString(CultureInfo.InvariantCulture)}}, 0]},
                        {"name": "cabinet", "mesh": {{cabinet}}},
                        {"name": "buttons", "mesh": {{buttons}}},
                        {"name": "glass", "mesh": {{glass}}},
                        {"name": "picture", "mesh": {{picture}}}
                      ],
                      "meshes": [{{string.Join(",", _meshes)}}],
                      "materials": [
                        {
                          "name": "cabinet",
                          "doubleSided": true,
                          "normalTexture": {"index": 0},
                          "pbrMetallicRoughness": {
                            "baseColorTexture": {"index": 0},
                            "metallicFactor": 0.25,
                            "roughnessFactor": 0.6
                          }
                        },
                        {
                          "name": "screen",
                          "emissiveFactor": [1, 1, 1],
                          "extensions": {"KHR_materials_emissive_strength": {"emissiveStrength": 2.5} },
                          "pbrMetallicRoughness": {"baseColorFactor": [0, 0, 0, 1]}
                        }
                      ],
                      "textures": [{"source": 0}],
                      "images": [{"bufferView": {{image}}, "mimeType": "image/png"}],
                      "accessors": [{{string.Join(",", _accessors)}}],
                      "bufferViews": [{{string.Join(",", _views)}}],
                      "buffers": [{"byteLength": {{_bin.Count}} }]
                    }
                    """);

                return Pack(json, [.. _bin]);
            }

            private int AddFloats(float[] data, int components, string type, int count)
            {
                byte[] bytes = new byte[data.Length * 4];
                for (int i = 0; i < data.Length; i++)
                {
                    BinaryPrimitives.WriteSingleLittleEndian(bytes.AsSpan(i * 4), data[i]);
                }

                int view = AddBytes(bytes);
                _accessors.Add(string.Create(
                    CultureInfo.InvariantCulture,
                    $$"""{"bufferView":{{view}},"componentType":5126,"count":{{count}},"type":"{{type}}"}"""));
                _ = components;
                return _accessors.Count - 1;
            }

            private int AddIndices(int[] indices)
            {
                byte[] bytes = new byte[indices.Length * 4];
                for (int i = 0; i < indices.Length; i++)
                {
                    BinaryPrimitives.WriteUInt32LittleEndian(bytes.AsSpan(i * 4), (uint)indices[i]);
                }

                int view = AddBytes(bytes);
                _accessors.Add(string.Create(
                    CultureInfo.InvariantCulture,
                    $$"""{"bufferView":{{view}},"componentType":5125,"count":{{indices.Length}},"type":"SCALAR"}"""));
                return _accessors.Count - 1;
            }

            private int AddBytes(byte[] bytes)
            {
                while (_bin.Count % 4 != 0)
                {
                    _bin.Add(0);
                }

                int offset = _bin.Count;
                _bin.AddRange(bytes);
                _views.Add(string.Create(
                    CultureInfo.InvariantCulture,
                    $$"""{"buffer":0,"byteOffset":{{offset}},"byteLength":{{bytes.Length}}}"""));
                return _views.Count - 1;
            }

            /// <summary>The container of glTF 2.0 §4.4.1: a header and two padded chunks.</summary>
            private static byte[] Pack(string json, byte[] bin)
            {
                byte[] text = Encoding.UTF8.GetBytes(json);
                byte[] jsonChunk = Pad(text, 0x20);
                byte[] binChunk = Pad(bin, 0x00);

                using MemoryStream file = new();

                void Write(uint value)
                {
                    byte[] word = new byte[4];
                    BinaryPrimitives.WriteUInt32LittleEndian(word, value);
                    file.Write(word);
                }

                Write(0x46546C67);
                Write(2);
                Write((uint)(12 + 8 + jsonChunk.Length + 8 + binChunk.Length));
                Write((uint)jsonChunk.Length);
                Write(0x4E4F534A);
                file.Write(jsonChunk);
                Write((uint)binChunk.Length);
                Write(0x004E4942);
                file.Write(binChunk);
                return file.ToArray();
            }

            private static byte[] Pad(byte[] data, byte filler)
            {
                int padded = (data.Length + 3) / 4 * 4;
                byte[] result = new byte[padded];
                data.CopyTo(result, 0);
                for (int i = data.Length; i < padded; i++)
                {
                    result[i] = filler;
                }

                return result;
            }
        }
    }
}
