using System.Buffers.Binary;
using System.IO.Compression;
using Zax.Engine3D;
using Zax.Render.Assets;

namespace Zax.Render.Tests;

/// <summary>
/// The readers the extracted assets of SPEC-06 go through: the PNG decoder used by the skybox
/// cubemaps (REQ-SCN-050, REQ-SCN-051) and the OBJ reader of the mesh folder.
/// </summary>
public sealed class AssetTests
{
    [Fact]
    public void REQ_SCN_050_the_png_decoder_reads_an_rgba_image_whatever_the_row_filter()
    {
        // One 4 x 5 image, a different filter on every row, so that all five of PNG section 9.2
        // are exercised at once.
        byte[] pixels = new byte[4 * 5 * 4];
        for (int i = 0; i < 4 * 5; i++)
        {
            pixels[(i * 4) + 0] = (byte)(i * 11);
            pixels[(i * 4) + 1] = (byte)(255 - (i * 7));
            pixels[(i * 4) + 2] = (byte)(i * 3);
            pixels[(i * 4) + 3] = (byte)(i % 2 == 0 ? 255 : 128);
        }

        ImageData decoded = PngImage.Decode(Png.Rgba(4, 5, pixels));

        Assert.Equal(4, decoded.Width);
        Assert.Equal(5, decoded.Height);
        Assert.Equal(pixels, decoded.Pixels);
    }

    [Fact]
    public void REQ_SCN_050_a_grey_png_becomes_an_opaque_grey_rgba_image()
    {
        byte[] grey = [0, 64, 128, 255, 10, 20, 30, 40];
        ImageData decoded = PngImage.Decode(Png.Grey(4, 2, grey));

        Assert.Equal((0, 0, 0, 255), decoded.Pixel(0, 0));
        Assert.Equal((128, 128, 128, 255), decoded.Pixel(2, 0));
        Assert.Equal((40, 40, 40, 255), decoded.Pixel(3, 1));
    }

    [Fact]
    public void The_png_decoder_refuses_a_file_that_is_not_a_png()
    {
        Assert.Throws<InvalidDataException>(() => PngImage.Decode("not a png"u8));
    }

    [Fact]
    public void REQ_SCN_054_the_obj_reader_builds_triangles_with_their_normals_and_uvs()
    {
        TileMesh mesh = ObjMesh.Parse("""
            g cabinet
            v 0 0 0
            v 1 0 0
            v 1 1 0
            v 0 1 0
            vt 0 0
            vt 1 0
            vt 1 1
            vt 0 1
            vn 0 0 -1
            f 1/1/1 2/2/1 3/3/1 4/4/1
            """);

        // The quad is triangulated as a fan: two triangles, four shared vertices.
        Assert.Equal(4, mesh.VertexCount);
        Assert.Equal(2, mesh.TriangleCount);
        Assert.Equal(new Vec3(0f, 0f, -1f), mesh.Vertices[0].Normal);
        Assert.Equal(new Vec2(1f, 1f), mesh.Vertices[2].Uv);
        Assert.Equal(new Vec3(1f, 1f, 0f), mesh.BoundsMax);
    }

    [Fact]
    public void Q_023_only_the_first_group_is_kept_because_one_material_is_listed()
    {
        // Q-023: an exported model may keep a second submesh in its own group; one material per
        // model means one group, so the reader keeps the first and drops the rest.
        const string source = """
            g Object004
            v 0 0 0
            v 1 0 0
            v 1 1 0
            v 0 0 1
            vn 0 0 -1
            g Object004_0
            f 1//1 2//1 3//1
            g Object004_1
            f 1//1 2//1 4//1
            """;

        List<(string Name, TileMesh Mesh)> groups = ObjMesh.ParseGroups(source);
        Assert.Equal(2, groups.Count);
        Assert.Equal("Object004_0", groups[0].Name);
        Assert.Equal("Object004_1", groups[1].Name);
        Assert.Equal(1, ObjMesh.Parse(source).TriangleCount);
    }

    [Fact]
    public void A_negative_obj_index_counts_back_from_the_end()
    {
        TileMesh mesh = ObjMesh.Parse("""
            g g
            v 0 0 0
            v 1 0 0
            v 1 1 0
            f -3 -2 -1
            """);

        Assert.Equal(1, mesh.TriangleCount);
        Assert.Equal(3, mesh.VertexCount);
    }

    /// <summary>Builds the smallest valid PNG files the decoder has to read.</summary>
    private static class Png
    {
        public static byte[] Rgba(int width, int height, byte[] pixels)
            => Build(width, height, 6, Filtered(width, height, 4, pixels));

        public static byte[] Grey(int width, int height, byte[] samples)
            => Build(width, height, 0, Filtered(width, height, 1, samples));

        /// <summary>Applies filter <c>row % 5</c> to each row, filter byte included.</summary>
        private static byte[] Filtered(int width, int height, int bytesPerPixel, byte[] pixels)
        {
            int stride = width * bytesPerPixel;
            byte[] raw = new byte[(stride + 1) * height];
            for (int y = 0; y < height; y++)
            {
                int filter = y % 5;
                raw[y * (stride + 1)] = (byte)filter;
                for (int x = 0; x < stride; x++)
                {
                    int value = pixels[(y * stride) + x];
                    int a = x >= bytesPerPixel ? pixels[(y * stride) + x - bytesPerPixel] : 0;
                    int b = y > 0 ? pixels[((y - 1) * stride) + x] : 0;
                    int c = y > 0 && x >= bytesPerPixel
                        ? pixels[((y - 1) * stride) + x - bytesPerPixel]
                        : 0;

                    raw[(y * (stride + 1)) + 1 + x] = (byte)(filter switch
                    {
                        0 => value,
                        1 => value - a,
                        2 => value - b,
                        3 => value - ((a + b) >> 1),
                        _ => value - Paeth(a, b, c),
                    });
                }
            }

            return raw;
        }

        private static int Paeth(int a, int b, int c)
        {
            int p = a + b - c;
            int pa = Math.Abs(p - a);
            int pb = Math.Abs(p - b);
            int pc = Math.Abs(p - c);
            return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
        }

        private static byte[] Build(int width, int height, int colorType, byte[] raw)
        {
            using MemoryStream file = new();
            file.Write([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

            byte[] header = new byte[13];
            BinaryPrimitives.WriteInt32BigEndian(header, width);
            BinaryPrimitives.WriteInt32BigEndian(header.AsSpan(4), height);
            header[8] = 8;
            header[9] = (byte)colorType;
            Chunk(file, "IHDR"u8, header);

            using MemoryStream compressed = new();
            using (ZLibStream deflate = new(compressed, CompressionLevel.Fastest, leaveOpen: true))
            {
                deflate.Write(raw);
            }

            Chunk(file, "IDAT"u8, compressed.ToArray());
            Chunk(file, "IEND"u8, []);
            return file.ToArray();
        }

        private static void Chunk(Stream file, ReadOnlySpan<byte> type, ReadOnlySpan<byte> data)
        {
            Span<byte> length = stackalloc byte[4];
            BinaryPrimitives.WriteInt32BigEndian(length, data.Length);
            file.Write(length);
            file.Write(type);
            file.Write(data);

            // The decoder does not check the CRC, so four zero bytes stand in for it.
            file.Write([0, 0, 0, 0]);
        }
    }
}
