using System.Buffers.Binary;
using System.Globalization;
using System.Numerics;
using System.Text;
using System.Text.Json;
using Zax.Engine3D;

namespace Zax.Render.Assets;

/// <summary>
/// One material of a glTF file, in the metallic-roughness model of the core specification
/// (glTF 2.0 §3.9.2) plus the two extensions the television model declares.
/// </summary>
/// <param name="Name">The material's name, or an empty string.</param>
/// <param name="BaseColorFactor">Base colour and opacity, multiplied with the texture.</param>
/// <param name="BaseColorTexture">Index into <see cref="GltfModel.Images"/>, or −1.</param>
/// <param name="MetallicFactor">Metalness, 1 by default.</param>
/// <param name="RoughnessFactor">Roughness, 1 by default.</param>
/// <param name="MetallicRoughnessTexture">
/// Index into <see cref="GltfModel.Images"/> of the texture holding roughness in green and
/// metalness in blue, or −1.
/// </param>
/// <param name="NormalTexture">Index into <see cref="GltfModel.Images"/>, or −1.</param>
/// <param name="NormalScale">Scale applied to the x and y of the normal map.</param>
/// <param name="EmissiveFactor">Emissive colour, black by default.</param>
/// <param name="EmissiveTexture">Index into <see cref="GltfModel.Images"/>, or −1.</param>
/// <param name="EmissiveStrength">
/// The multiplier of <c>KHR_materials_emissive_strength</c>, 1 when the extension is absent.
/// </param>
/// <param name="AlphaMode"><c>OPAQUE</c>, <c>MASK</c> or <c>BLEND</c>.</param>
/// <param name="AlphaCutoff">The threshold of <c>MASK</c>, 0.5 by default.</param>
/// <param name="DoubleSided">Whether back faces are drawn.</param>
public readonly record struct GltfMaterial(
    string Name,
    Vector4 BaseColorFactor,
    int BaseColorTexture,
    float MetallicFactor,
    float RoughnessFactor,
    int MetallicRoughnessTexture,
    int NormalTexture,
    float NormalScale,
    Vector3 EmissiveFactor,
    int EmissiveTexture,
    float EmissiveStrength,
    string AlphaMode,
    float AlphaCutoff,
    bool DoubleSided)
{
    /// <summary>The default material of glTF 2.0 §3.9.2, used when a primitive names none.</summary>
    public static GltfMaterial Default { get; } = new(
        Name: string.Empty,
        BaseColorFactor: Vector4.One,
        BaseColorTexture: -1,
        MetallicFactor: 1f,
        RoughnessFactor: 1f,
        MetallicRoughnessTexture: -1,
        NormalTexture: -1,
        NormalScale: 1f,
        EmissiveFactor: Vector3.Zero,
        EmissiveTexture: -1,
        EmissiveStrength: 1f,
        AlphaMode: "OPAQUE",
        AlphaCutoff: 0.5f,
        DoubleSided: false);
}

/// <summary>
/// One drawable primitive of a glTF file: a mesh whose vertices are already expressed in the
/// file's own world frame (the node transforms are baked in).
/// </summary>
/// <param name="Name">The node's name, or the mesh's, for identification.</param>
/// <param name="MeshIndex">Index of the glTF mesh the primitive belongs to.</param>
/// <param name="Mesh">The geometry, in the file's world frame, Y up.</param>
/// <param name="Material">The material of the primitive.</param>
/// <param name="VertexCount">
/// Number of vertices the primitive's accessor declares — what AMD-025 identifies the two
/// screen plates of the television by.
/// </param>
public sealed record GltfPart(
    string Name,
    int MeshIndex,
    TileMesh Mesh,
    GltfMaterial Material,
    int VertexCount);

/// <summary>
/// A loaded glTF 2.0 model: its drawable primitives and its decoded images.
/// </summary>
/// <param name="Parts">The primitives of the default scene, in file order.</param>
/// <param name="Images">The decoded images, indexed as the file's <c>images</c> array is.</param>
public sealed record GltfModel(IReadOnlyList<GltfPart> Parts, IReadOnlyList<ImageData> Images)
{
    /// <summary>Bounding box of every part, in the file's world frame.</summary>
    public (Vector3 Min, Vector3 Max) Bounds()
    {
        Vector3 min = new(float.MaxValue);
        Vector3 max = new(float.MinValue);
        foreach (GltfPart part in Parts)
        {
            min = Vector3.Min(min, new Vector3(part.Mesh.BoundsMin.X, part.Mesh.BoundsMin.Y, part.Mesh.BoundsMin.Z));
            max = Vector3.Max(max, new Vector3(part.Mesh.BoundsMax.X, part.Mesh.BoundsMax.Y, part.Mesh.BoundsMax.Z));
        }

        return Parts.Count == 0 ? (Vector3.Zero, Vector3.Zero) : (min, max);
    }
}

/// <summary>
/// A reader for binary glTF 2.0 files (<c>.glb</c>), written against the public Khronos
/// specification: the container of §4.4.1, the JSON structures of §3, and the metallic-roughness
/// material model of §3.9.2.
/// </summary>
/// <remarks>
/// <para>
/// AMD-025 asks for a "generic binary glTF 2.0 loader (meshes, node transforms, PBR
/// metallic-roughness materials including normal and emissive maps)" so that the CC BY
/// television model can be shipped <b>unmodified</b> and adapted at run time. Nothing here is
/// specific to that file.
/// </para>
/// <para>
/// What is read: the default scene's node tree with its <c>matrix</c> or
/// <c>translation</c>/<c>rotation</c>/<c>scale</c>, the triangle primitives of the meshes they
/// point at, the <c>POSITION</c>, <c>NORMAL</c> and <c>TEXCOORD_0</c> attributes, the indices,
/// the materials, and the images, decoded by <see cref="PngImage"/> and
/// <see cref="JpegImage"/>. What is ignored: animations, skins, cameras, morph targets, sparse
/// accessors and every extension but <c>KHR_materials_emissive_strength</c>; a file that
/// <b>requires</b> an unknown extension is rejected.
/// </para>
/// <para>
/// Vertices come out in the file's own world frame — the node transforms are applied to the
/// positions and their inverse transpose to the normals — so a caller places the whole model
/// with a single matrix.
/// </para>
/// </remarks>
public static class GltfBinary
{
    private const uint Magic = 0x46546C67; // "glTF"
    private const uint JsonChunk = 0x4E4F534A; // "JSON"
    private const uint BinaryChunk = 0x004E4942; // "BIN"

    /// <summary>The extensions this reader understands well enough to honour a requirement on.</summary>
    private static readonly string[] Supported = ["KHR_materials_emissive_strength"];

    /// <summary>Loads the binary glTF file <paramref name="path"/>.</summary>
    /// <exception cref="InvalidDataException">The file is not a glTF 2.0 binary this reader supports.</exception>
    public static GltfModel Load(string path) => Decode(File.ReadAllBytes(path));

    /// <summary>Loads the binary glTF file <paramref name="path"/>, or null when it cannot be read.</summary>
    /// <remarks>
    /// AMD-025: the application must still run when the model is missing or broken — the TV
    /// option is simply disabled.
    /// </remarks>
    public static GltfModel? TryLoad(string? path)
    {
        if (string.IsNullOrWhiteSpace(path) || !File.Exists(path))
        {
            return null;
        }

        try
        {
            return Load(path);
        }
        catch (Exception exception)
            when (exception is IOException or InvalidDataException or JsonException
                or UnauthorizedAccessException or NotSupportedException)
        {
            return null;
        }
    }

    /// <summary>Decodes a binary glTF file held in memory.</summary>
    /// <exception cref="InvalidDataException">The data is not a glTF 2.0 binary this reader supports.</exception>
    public static GltfModel Decode(ReadOnlySpan<byte> file)
    {
        (JsonElement root, byte[] binary) = SplitChunks(file);

        if (root.TryGetProperty("extensionsRequired", out JsonElement required))
        {
            foreach (JsonElement extension in required.EnumerateArray())
            {
                string name = extension.GetString() ?? string.Empty;
                if (Array.IndexOf(Supported, name) < 0)
                {
                    throw new InvalidDataException($"The glTF file requires the extension {name}.");
                }
            }
        }

        byte[][] buffers = ReadBuffers(root, binary);
        BufferView[] views = ReadBufferViews(root);
        Accessor[] accessors = ReadAccessors(root);
        GltfMaterial[] materials = ReadMaterials(root);
        ImageData[] images = ReadImages(root, buffers, views);

        List<GltfPart> parts = [];
        foreach ((int node, Matrix4x4 transform) in WalkScene(root))
        {
            AddNodeParts(root, node, transform, buffers, views, accessors, materials, parts);
        }

        return new GltfModel(parts, images);
    }

    // --------------------------------------------------------------- container

    private static (JsonElement Root, byte[] Binary) SplitChunks(ReadOnlySpan<byte> file)
    {
        if (file.Length < 20)
        {
            throw new InvalidDataException("The file is too short to be a binary glTF.");
        }

        if (BinaryPrimitives.ReadUInt32LittleEndian(file) != Magic)
        {
            throw new InvalidDataException("Not a binary glTF file: the magic does not match.");
        }

        uint version = BinaryPrimitives.ReadUInt32LittleEndian(file[4..]);
        if (version != 2)
        {
            throw new InvalidDataException($"Binary glTF version {version} is not supported.");
        }

        JsonDocument? document = null;
        byte[] binary = [];
        int at = 12;
        while (at + 8 <= file.Length)
        {
            int length = (int)BinaryPrimitives.ReadUInt32LittleEndian(file[at..]);
            uint kind = BinaryPrimitives.ReadUInt32LittleEndian(file[(at + 4)..]);
            at += 8;
            if (length < 0 || at + length > file.Length)
            {
                throw new InvalidDataException("Truncated binary glTF chunk.");
            }

            ReadOnlySpan<byte> data = file.Slice(at, length);
            at += length;

            if (kind == JsonChunk && document is null)
            {
                document = JsonDocument.Parse(data.ToArray());
            }
            else if (kind == BinaryChunk && binary.Length == 0)
            {
                binary = data.ToArray();
            }
        }

        if (document is null)
        {
            throw new InvalidDataException("The binary glTF file has no JSON chunk.");
        }

        // The clone owns its own memory, so the pooled document can go.
        using (document)
        {
            return (document.RootElement.Clone(), binary);
        }
    }

    private static byte[][] ReadBuffers(JsonElement root, byte[] binary)
    {
        if (!root.TryGetProperty("buffers", out JsonElement buffers))
        {
            return [];
        }

        byte[][] data = new byte[buffers.GetArrayLength()][];
        int index = 0;
        foreach (JsonElement buffer in buffers.EnumerateArray())
        {
            string? uri = buffer.TryGetProperty("uri", out JsonElement value) ? value.GetString() : null;
            data[index++] = uri is null
                ? binary
                : uri.StartsWith("data:", StringComparison.Ordinal)
                    ? DecodeDataUri(uri)

                    // External .bin files are part of glTF; nothing reads one here, because the
                    // renderer is handed one self-contained .glb (REQ-SCN-057).
                    : throw new InvalidDataException("External glTF buffers are not supported.");
        }

        return data;
    }

    private static byte[] DecodeDataUri(string uri)
    {
        int comma = uri.IndexOf(',', StringComparison.Ordinal);
        if (comma < 0 || !uri.AsSpan(0, comma).EndsWith(";base64", StringComparison.Ordinal))
        {
            throw new InvalidDataException("Only base64 data URIs are supported in a glTF file.");
        }

        return Convert.FromBase64String(uri[(comma + 1)..]);
    }

    private readonly record struct BufferView(int Buffer, int Offset, int Length, int Stride);

    private static BufferView[] ReadBufferViews(JsonElement root)
    {
        if (!root.TryGetProperty("bufferViews", out JsonElement views))
        {
            return [];
        }

        BufferView[] result = new BufferView[views.GetArrayLength()];
        int index = 0;
        foreach (JsonElement view in views.EnumerateArray())
        {
            result[index++] = new BufferView(
                Int(view, "buffer", 0),
                Int(view, "byteOffset", 0),
                Int(view, "byteLength", 0),
                Int(view, "byteStride", 0));
        }

        return result;
    }

    private readonly record struct Accessor(
        int View,
        int Offset,
        int ComponentType,
        int Count,
        string Type,
        bool Normalized);

    private static Accessor[] ReadAccessors(JsonElement root)
    {
        if (!root.TryGetProperty("accessors", out JsonElement accessors))
        {
            return [];
        }

        Accessor[] result = new Accessor[accessors.GetArrayLength()];
        int index = 0;
        foreach (JsonElement accessor in accessors.EnumerateArray())
        {
            result[index++] = new Accessor(
                Int(accessor, "bufferView", -1),
                Int(accessor, "byteOffset", 0),
                Int(accessor, "componentType", 5126),
                Int(accessor, "count", 0),
                accessor.TryGetProperty("type", out JsonElement type) ? type.GetString() ?? "SCALAR" : "SCALAR",
                accessor.TryGetProperty("normalized", out JsonElement normalized) && normalized.GetBoolean());
        }

        return result;
    }

    // --------------------------------------------------------------- materials and images

    private static GltfMaterial[] ReadMaterials(JsonElement root)
    {
        if (!root.TryGetProperty("materials", out JsonElement materials))
        {
            return [];
        }

        int[] textureSources = ReadTextureSources(root);
        GltfMaterial[] result = new GltfMaterial[materials.GetArrayLength()];
        int index = 0;
        foreach (JsonElement material in materials.EnumerateArray())
        {
            GltfMaterial built = GltfMaterial.Default with
            {
                Name = material.TryGetProperty("name", out JsonElement name)
                    ? name.GetString() ?? string.Empty
                    : string.Empty,
                DoubleSided = material.TryGetProperty("doubleSided", out JsonElement sided)
                    && sided.GetBoolean(),
                AlphaMode = material.TryGetProperty("alphaMode", out JsonElement mode)
                    ? mode.GetString() ?? "OPAQUE"
                    : "OPAQUE",
                AlphaCutoff = Float(material, "alphaCutoff", 0.5f),
                EmissiveFactor = Vector(material, "emissiveFactor", Vector3.Zero),
                EmissiveTexture = Source(material, "emissiveTexture", textureSources),
                NormalTexture = Source(material, "normalTexture", textureSources),
            };

            if (material.TryGetProperty("normalTexture", out JsonElement normal))
            {
                built = built with { NormalScale = Float(normal, "scale", 1f) };
            }

            if (material.TryGetProperty("pbrMetallicRoughness", out JsonElement pbr))
            {
                built = built with
                {
                    BaseColorFactor = Vector4Of(pbr, "baseColorFactor", Vector4.One),
                    BaseColorTexture = Source(pbr, "baseColorTexture", textureSources),
                    MetallicFactor = Float(pbr, "metallicFactor", 1f),
                    RoughnessFactor = Float(pbr, "roughnessFactor", 1f),
                    MetallicRoughnessTexture = Source(pbr, "metallicRoughnessTexture", textureSources),
                };
            }

            if (material.TryGetProperty("extensions", out JsonElement extensions)
                && extensions.TryGetProperty("KHR_materials_emissive_strength", out JsonElement strength))
            {
                built = built with { EmissiveStrength = Float(strength, "emissiveStrength", 1f) };
            }

            result[index++] = built;
        }

        return result;
    }

    /// <summary>The image each texture points at, so a material can name an image directly.</summary>
    private static int[] ReadTextureSources(JsonElement root)
    {
        if (!root.TryGetProperty("textures", out JsonElement textures))
        {
            return [];
        }

        int[] sources = new int[textures.GetArrayLength()];
        int index = 0;
        foreach (JsonElement texture in textures.EnumerateArray())
        {
            sources[index++] = Int(texture, "source", -1);
        }

        return sources;
    }

    private static int Source(JsonElement owner, string property, int[] textureSources)
    {
        if (!owner.TryGetProperty(property, out JsonElement reference))
        {
            return -1;
        }

        int texture = Int(reference, "index", -1);
        return texture >= 0 && texture < textureSources.Length ? textureSources[texture] : -1;
    }

    private static ImageData[] ReadImages(JsonElement root, byte[][] buffers, BufferView[] views)
    {
        if (!root.TryGetProperty("images", out JsonElement images))
        {
            return [];
        }

        ImageData[] result = new ImageData[images.GetArrayLength()];
        int index = 0;
        foreach (JsonElement image in images.EnumerateArray())
        {
            byte[] bytes;
            if (image.TryGetProperty("bufferView", out JsonElement view))
            {
                BufferView slice = views[view.GetInt32()];
                bytes = buffers[slice.Buffer].AsSpan(slice.Offset, slice.Length).ToArray();
            }
            else if (image.TryGetProperty("uri", out JsonElement uri)
                && uri.GetString() is { } text
                && text.StartsWith("data:", StringComparison.Ordinal))
            {
                bytes = DecodeDataUri(text);
            }
            else
            {
                // An external image file; the model this reader exists for embeds all of its.
                result[index++] = ImageData.Single(255, 255, 255);
                continue;
            }

            result[index++] = JpegImage.IsJpeg(bytes) ? JpegImage.Decode(bytes) : PngImage.Decode(bytes);
        }

        return result;
    }

    // --------------------------------------------------------------- nodes and meshes

    /// <summary>
    /// Walks the default scene depth first and yields every node with its world transform
    /// (glTF 2.0 §3.5.2: a child's transform is its parent's times its own).
    /// </summary>
    private static List<(int Node, Matrix4x4 Transform)> WalkScene(JsonElement root)
    {
        List<(int, Matrix4x4)> flat = [];
        if (!root.TryGetProperty("nodes", out JsonElement nodes))
        {
            return flat;
        }

        List<int> roots = [];
        int scene = Int(root, "scene", 0);
        if (root.TryGetProperty("scenes", out JsonElement scenes)
            && scene >= 0
            && scene < scenes.GetArrayLength()
            && scenes[scene].TryGetProperty("nodes", out JsonElement sceneNodes))
        {
            foreach (JsonElement node in sceneNodes.EnumerateArray())
            {
                roots.Add(node.GetInt32());
            }
        }
        else
        {
            for (int i = 0; i < nodes.GetArrayLength(); i++)
            {
                roots.Add(i);
            }
        }

        Stack<(int Node, Matrix4x4 Parent)> pending = new();
        for (int i = roots.Count - 1; i >= 0; i--)
        {
            pending.Push((roots[i], Matrix4x4.Identity));
        }

        // A malformed file could name a cycle; the visited set keeps the walk finite.
        HashSet<int> visited = [];
        while (pending.Count > 0)
        {
            (int index, Matrix4x4 parent) = pending.Pop();
            if (index < 0 || index >= nodes.GetArrayLength() || !visited.Add(index))
            {
                continue;
            }

            JsonElement node = nodes[index];
            Matrix4x4 world = Multiply(parent, LocalTransform(node));
            flat.Add((index, world));

            if (node.TryGetProperty("children", out JsonElement children))
            {
                // Pushed back to front, so the walk visits them in the order the file lists
                // them: the caller identifies parts by their position.
                for (int at = children.GetArrayLength() - 1; at >= 0; at--)
                {
                    pending.Push((children[at].GetInt32(), world));
                }
            }
        }

        return flat;
    }

    /// <summary>
    /// The local transform of a node: its <c>matrix</c>, or the composition
    /// <c>T · R · S</c> of §3.5.1.
    /// </summary>
    private static Matrix4x4 LocalTransform(JsonElement node)
    {
        if (node.TryGetProperty("matrix", out JsonElement matrix) && matrix.GetArrayLength() == 16)
        {
            Span<float> cells = stackalloc float[16];
            int at = 0;
            foreach (JsonElement cell in matrix.EnumerateArray())
            {
                cells[at++] = (float)cell.GetDouble();
            }

            // glTF stores column-major; System.Numerics.Matrix4x4 is row-major with row
            // vectors, which is the same memory layout for the same transform.
            return new Matrix4x4(
                cells[0], cells[1], cells[2], cells[3],
                cells[4], cells[5], cells[6], cells[7],
                cells[8], cells[9], cells[10], cells[11],
                cells[12], cells[13], cells[14], cells[15]);
        }

        Vector3 translation = Vector(node, "translation", Vector3.Zero);
        Vector3 scale = Vector(node, "scale", Vector3.One);
        Quaternion rotation = Quaternion.Identity;
        if (node.TryGetProperty("rotation", out JsonElement quaternion) && quaternion.GetArrayLength() == 4)
        {
            Span<float> cells = stackalloc float[4];
            int at = 0;
            foreach (JsonElement cell in quaternion.EnumerateArray())
            {
                cells[at++] = (float)cell.GetDouble();
            }

            rotation = new Quaternion(cells[0], cells[1], cells[2], cells[3]);
        }

        return Matrix4x4.CreateScale(scale)
            * Matrix4x4.CreateFromQuaternion(rotation)
            * Matrix4x4.CreateTranslation(translation);
    }

    /// <summary>
    /// The product <c>parent · child</c> for row-vector matrices, which is what
    /// <see cref="Matrix4x4.Multiply(Matrix4x4, Matrix4x4)"/> computes as <c>child × parent</c>.
    /// </summary>
    private static Matrix4x4 Multiply(Matrix4x4 parent, Matrix4x4 child) => child * parent;

    private static void AddNodeParts(
        JsonElement root,
        int index,
        Matrix4x4 transform,
        byte[][] buffers,
        BufferView[] views,
        Accessor[] accessors,
        GltfMaterial[] materials,
        List<GltfPart> parts)
    {
        JsonElement node = root.GetProperty("nodes")[index];
        if (!node.TryGetProperty("mesh", out JsonElement meshIndex)
            || !root.TryGetProperty("meshes", out JsonElement meshes))
        {
            return;
        }

        int mesh = meshIndex.GetInt32();
        if (mesh < 0 || mesh >= meshes.GetArrayLength())
        {
            return;
        }

        string name = node.TryGetProperty("name", out JsonElement nodeName)
            ? nodeName.GetString() ?? string.Empty
            : string.Empty;

        JsonElement primitives = meshes[mesh].GetProperty("primitives");

        // The normals need the inverse transpose of the upper-left block; a singular transform
        // leaves them as they are rather than producing NaNs.
        Matrix4x4 normalMatrix = Matrix4x4.Invert(transform, out Matrix4x4 inverse)
            ? Matrix4x4.Transpose(inverse)
            : Matrix4x4.Identity;

        foreach (JsonElement primitive in primitives.EnumerateArray())
        {
            // Mode 4 is TRIANGLES, the default; strips, fans and the point and line modes are
            // not geometry this renderer draws.
            if (Int(primitive, "mode", 4) != 4 || !primitive.TryGetProperty("attributes", out JsonElement attributes))
            {
                continue;
            }

            if (!attributes.TryGetProperty("POSITION", out JsonElement positionIndex))
            {
                continue;
            }

            Vector3[] positions = ReadVector3(accessors, buffers, views, positionIndex.GetInt32());
            Vector3[] normals = attributes.TryGetProperty("NORMAL", out JsonElement normalIndex)
                ? ReadVector3(accessors, buffers, views, normalIndex.GetInt32())
                : [];
            Vector2[] uvs = attributes.TryGetProperty("TEXCOORD_0", out JsonElement uvIndex)
                ? ReadVector2(accessors, buffers, views, uvIndex.GetInt32())
                : [];

            int[] indices = primitive.TryGetProperty("indices", out JsonElement indicesIndex)
                ? ReadIndices(accessors, buffers, views, indicesIndex.GetInt32())
                : [.. Enumerable.Range(0, positions.Length)];

            MeshVertex[] vertices = new MeshVertex[positions.Length];
            for (int i = 0; i < positions.Length; i++)
            {
                Vector3 position = Vector3.Transform(positions[i], transform);
                Vector3 normal = i < normals.Length
                    ? Vector3.TransformNormal(normals[i], normalMatrix)
                    : Vector3.UnitY;
                if (normal.LengthSquared() > 1e-12f)
                {
                    normal = Vector3.Normalize(normal);
                }

                Vector2 uv = i < uvs.Length ? uvs[i] : Vector2.Zero;
                vertices[i] = new MeshVertex(
                    new Vec3(position.X, position.Y, position.Z),
                    new Vec3(normal.X, normal.Y, normal.Z),
                    new Vec2(uv.X, uv.Y));
            }

            int material = Int(primitive, "material", -1);
            parts.Add(new GltfPart(
                string.IsNullOrEmpty(name) ? FormattableString.Invariant($"mesh{mesh}") : name,
                mesh,
                new TileMesh(vertices, indices),
                material >= 0 && material < materials.Length ? materials[material] : GltfMaterial.Default,
                positions.Length));
        }
    }

    // --------------------------------------------------------------- accessors

    private static ReadOnlySpan<byte> Slice(Accessor accessor, byte[][] buffers, BufferView[] views)
    {
        if (accessor.View < 0 || accessor.View >= views.Length)
        {
            return [];
        }

        BufferView view = views[accessor.View];
        if (view.Buffer < 0 || view.Buffer >= buffers.Length)
        {
            return [];
        }

        return buffers[view.Buffer].AsSpan(view.Offset, view.Length);
    }

    private static int ComponentSize(int componentType) => componentType switch
    {
        5120 or 5121 => 1,
        5122 or 5123 => 2,
        5125 or 5126 => 4,
        _ => throw new InvalidDataException($"Unknown glTF component type {componentType}."),
    };

    private static int Components(string type) => type switch
    {
        "SCALAR" => 1,
        "VEC2" => 2,
        "VEC3" => 3,
        "VEC4" => 4,
        "MAT2" => 4,
        "MAT3" => 9,
        "MAT4" => 16,
        _ => throw new InvalidDataException($"Unknown glTF accessor type {type}."),
    };

    private static float[] ReadFloats(Accessor[] accessors, byte[][] buffers, BufferView[] views, int index)
    {
        if (index < 0 || index >= accessors.Length)
        {
            return [];
        }

        Accessor accessor = accessors[index];
        int components = Components(accessor.Type);
        if (accessor.View < 0 || accessor.View >= views.Length)
        {
            // glTF 2.0 §3.6.2.1: an accessor without a buffer view reads as zeros.
            return new float[accessor.Count * components];
        }

        ReadOnlySpan<byte> data = Slice(accessor, buffers, views);
        int size = ComponentSize(accessor.ComponentType);
        int stride = views[accessor.View].Stride > 0 ? views[accessor.View].Stride : components * size;

        float[] result = new float[accessor.Count * components];
        for (int element = 0; element < accessor.Count; element++)
        {
            int at = accessor.Offset + (element * stride);
            for (int component = 0; component < components; component++)
            {
                int offset = at + (component * size);
                if (offset + size > data.Length)
                {
                    throw new InvalidDataException("A glTF accessor runs past its buffer view.");
                }

                result[(element * components) + component] =
                    ReadComponent(data[offset..], accessor.ComponentType, accessor.Normalized);
            }
        }

        return result;
    }

    /// <summary>
    /// One component, with the normalised integer conversions of glTF 2.0 §3.6.2.2.
    /// </summary>
    private static float ReadComponent(ReadOnlySpan<byte> data, int componentType, bool normalized) => componentType switch
    {
        5120 => normalized ? MathF.Max((sbyte)data[0] / 127f, -1f) : (sbyte)data[0],
        5121 => normalized ? data[0] / 255f : data[0],
        5122 => normalized
            ? MathF.Max(BinaryPrimitives.ReadInt16LittleEndian(data) / 32767f, -1f)
            : BinaryPrimitives.ReadInt16LittleEndian(data),
        5123 => normalized
            ? BinaryPrimitives.ReadUInt16LittleEndian(data) / 65535f
            : BinaryPrimitives.ReadUInt16LittleEndian(data),
        5125 => BinaryPrimitives.ReadUInt32LittleEndian(data),
        _ => BitConverter.ToSingle(data),
    };

    private static Vector3[] ReadVector3(Accessor[] accessors, byte[][] buffers, BufferView[] views, int index)
    {
        float[] raw = ReadFloats(accessors, buffers, views, index);
        Vector3[] result = new Vector3[raw.Length / 3];
        for (int i = 0; i < result.Length; i++)
        {
            result[i] = new Vector3(raw[i * 3], raw[(i * 3) + 1], raw[(i * 3) + 2]);
        }

        return result;
    }

    private static Vector2[] ReadVector2(Accessor[] accessors, byte[][] buffers, BufferView[] views, int index)
    {
        float[] raw = ReadFloats(accessors, buffers, views, index);
        Vector2[] result = new Vector2[raw.Length / 2];
        for (int i = 0; i < result.Length; i++)
        {
            result[i] = new Vector2(raw[i * 2], raw[(i * 2) + 1]);
        }

        return result;
    }

    private static int[] ReadIndices(Accessor[] accessors, byte[][] buffers, BufferView[] views, int index)
    {
        float[] raw = ReadFloats(accessors, buffers, views, index);
        int[] result = new int[raw.Length];
        for (int i = 0; i < raw.Length; i++)
        {
            result[i] = (int)raw[i];
        }

        return result;
    }

    // --------------------------------------------------------------- JSON helpers

    private static int Int(JsonElement owner, string property, int fallback)
        => owner.ValueKind == JsonValueKind.Object
            && owner.TryGetProperty(property, out JsonElement value)
            && value.ValueKind == JsonValueKind.Number
                ? value.GetInt32()
                : fallback;

    private static float Float(JsonElement owner, string property, float fallback)
        => owner.ValueKind == JsonValueKind.Object
            && owner.TryGetProperty(property, out JsonElement value)
            && value.ValueKind == JsonValueKind.Number
                ? (float)value.GetDouble()
                : fallback;

    private static Vector3 Vector(JsonElement owner, string property, Vector3 fallback)
    {
        if (!owner.TryGetProperty(property, out JsonElement value) || value.GetArrayLength() < 3)
        {
            return fallback;
        }

        return new Vector3(
            (float)value[0].GetDouble(),
            (float)value[1].GetDouble(),
            (float)value[2].GetDouble());
    }

    private static Vector4 Vector4Of(JsonElement owner, string property, Vector4 fallback)
    {
        if (!owner.TryGetProperty(property, out JsonElement value) || value.GetArrayLength() < 4)
        {
            return fallback;
        }

        return new Vector4(
            (float)value[0].GetDouble(),
            (float)value[1].GetDouble(),
            (float)value[2].GetDouble(),
            (float)value[3].GetDouble());
    }

    /// <summary>A short description of a model, for the harness and the logs.</summary>
    public static string Describe(GltfModel model)
    {
        ArgumentNullException.ThrowIfNull(model);
        StringBuilder text = new();
        text.Append(CultureInfo.InvariantCulture, $"{model.Parts.Count} parts, {model.Images.Count} images");
        foreach (GltfPart part in model.Parts)
        {
            text.Append(CultureInfo.InvariantCulture,
                $"\n  {part.Name}: {part.VertexCount} vertices, material '{part.Material.Name}'");
        }

        return text.ToString();
    }
}
