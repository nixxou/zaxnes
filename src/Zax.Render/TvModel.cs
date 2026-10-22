using System.Globalization;
using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render.Assets;

namespace Zax.Render;

/// <summary>
/// The material of one part of the television model (AMD-025): the metallic-roughness model of
/// glTF 2.0 §3.9.2, with the texture names the renderer's image cache resolves.
/// </summary>
/// <param name="BaseColor">Base colour and opacity, multiplied with the base colour texture.</param>
/// <param name="Diffuse">Base colour texture name, or null.</param>
/// <param name="Normal">Normal map name, or null.</param>
/// <param name="NormalScale">Scale of the x and y of the normal map.</param>
/// <param name="MetallicRoughness">
/// Name of the texture holding roughness in green and metalness in blue, or null.
/// </param>
/// <param name="Metallic">Metalness factor, multiplied with the blue channel of the map.</param>
/// <param name="Roughness">Roughness factor, multiplied with the green channel of the map.</param>
/// <param name="Emissive">Emissive texture name, or null.</param>
/// <param name="EmissiveColor">
/// Emissive factor times the strength of <c>KHR_materials_emissive_strength</c>.
/// </param>
/// <param name="AlphaCutout">Whether the base colour's alpha cuts the model out.</param>
/// <param name="AlphaCutoff">The cut-out threshold.</param>
/// <param name="DoubleSided">Whether back faces are drawn.</param>
public readonly record struct TvMaterial(
    Vector4 BaseColor,
    string? Diffuse,
    string? Normal,
    float NormalScale,
    string? MetallicRoughness,
    float Metallic,
    float Roughness,
    string? Emissive,
    Vector3 EmissiveColor,
    bool AlphaCutout,
    float AlphaCutoff,
    bool DoubleSided);

/// <summary>One drawable part of the television model, in the model's own frame.</summary>
/// <param name="Name">Name of the glTF node the part comes from.</param>
/// <param name="Mesh">The geometry, in the glTF world frame (Y up, screen towards +X).</param>
/// <param name="Material">The material.</param>
/// <param name="CastShadow">Whether the part fills the shadow map (AMD-025, AMD-021 R2-014).</param>
/// <param name="ReceiveShadow">Whether the part is darkened by the shadow map.</param>
public sealed record TvModelPart(
    string Name,
    TileMesh Mesh,
    TvMaterial Material,
    bool CastShadow,
    bool ReceiveShadow);

/// <summary>
/// The decorative television of AMD-025: the CC BY 4.0 model « Retro Crt Tv » by meipal, read
/// from its unmodified <c>.glb</c> and adapted at run time.
/// </summary>
/// <remarks>
/// <para>
/// The three adaptations of AMD-025 all happen here, and none of them touches the file:
/// </para>
/// <list type="number">
/// <item>
/// the two screen plates — the coloured glass and the textured picture, the two meshes of
/// <see cref="ScreenPlateVertices"/> vertices — are <b>not drawn</b>, so the NES picture and the
/// tunnel of REQ-SCN-053 appear in their place;
/// </item>
/// <item>
/// scale, orientation and position are computed so that the screen opening of the cabinet
/// frames exactly the visible rectangle of SPEC-06 REQ-SCN-039, whatever the clip margins. The
/// opening is measured on the geometry itself (<see cref="Opening"/>), not assumed;
/// </item>
/// <item>
/// the cabinet neither casts nor receives a shadow and the buttons do both, which is the rule
/// AMD-021 R2-014 set for the original models.
/// </item>
/// </list>
/// <para>
/// The glTF frame has Y up and the screen facing +X; the scene frame of REQ-E3D-410 has Y up,
/// X to the right and Z away from the camera. <see cref="Placement"/> maps model X to scene −Z,
/// model Y to scene Y and model Z to scene X, which is a rotation, and scales x and y
/// independently so that the opening matches the visible rectangle exactly. The resulting
/// aspect difference with the television models of the original is a divergence <b>D</b>
/// (AMD-025).
/// </para>
/// </remarks>
public sealed class TvModel
{
    /// <summary>
    /// Vertex count of each of the two screen plates (AMD-025). The file is shipped unmodified,
    /// so this identifies them exactly.
    /// </summary>
    public const int ScreenPlateVertices = 49;

    /// <summary>Front face of the tunnel of REQ-SCN-053, where the screen opening is placed.</summary>
    public const float TunnelFront = -16f;

    /// <summary>Prefix of the texture names the renderer's image cache resolves.</summary>
    public const string TexturePrefix = "zax:tv-model/";

    private readonly Rect _opening;
    private readonly Rect _silhouette;
    private readonly float _screenPlane;

    private TvModel(
        IReadOnlyList<TvModelPart> parts,
        IReadOnlyList<ImageData> images,
        int hiddenParts,
        Rect opening,
        Rect silhouette,
        float screenPlane)
    {
        Parts = parts;
        Images = images;
        HiddenParts = hiddenParts;
        _opening = opening;
        _silhouette = silhouette;
        _screenPlane = screenPlane;
    }

    /// <summary>The parts that are drawn, screen plates excluded.</summary>
    public IReadOnlyList<TvModelPart> Parts { get; }

    /// <summary>The decoded textures, keyed by their index in <see cref="TextureName"/>.</summary>
    public IReadOnlyList<ImageData> Images { get; }

    /// <summary>How many parts the screen rule of AMD-025 hid; two for the shipped model.</summary>
    public int HiddenParts { get; }

    /// <summary>
    /// The screen opening measured on the cabinet, in the model frame: the largest rectangle
    /// around the centre of the screen plates that no geometry in front of them covers.
    /// <c>MinX</c> and <c>MaxX</c> are model <b>z</b> (the width of the screen), <c>MinY</c> and
    /// <c>MaxY</c> model <b>y</b> (its height).
    /// </summary>
    public Rect Opening => _opening;

    /// <summary>The whole model's extent in the same two axes, which bounds its silhouette.</summary>
    public Rect Silhouette => _silhouette;

    /// <summary>The texture name of image <paramref name="index"/> of the model.</summary>
    public static string TextureName(int index)
        => TexturePrefix + index.ToString(CultureInfo.InvariantCulture);

    /// <summary>
    /// Reads the model from a binary glTF file, or returns null when the file is missing or
    /// cannot be read — AMD-025: the application runs without it, the TV option is disabled.
    /// </summary>
    public static TvModel? TryLoad(string? path) => From(GltfBinary.TryLoad(path));

    /// <summary>Builds the television from an already loaded glTF model, or null when it has no usable part.</summary>
    public static TvModel? From(GltfModel? model)
    {
        if (model is null || model.Parts.Count == 0)
        {
            return null;
        }

        List<GltfPart> screens = [.. model.Parts.Where(part => part.VertexCount == ScreenPlateVertices)];
        List<GltfPart> visible = [.. model.Parts.Where(part => part.VertexCount != ScreenPlateVertices)];
        if (visible.Count == 0)
        {
            return null;
        }

        // AMD-025 and AMD-021 R2-014: the cabinet — the largest part — neither casts nor
        // receives; everything else is a button and does both.
        GltfPart cabinet = visible.MaxBy(part => part.VertexCount)!;

        List<TvModelPart> parts = [];
        foreach (GltfPart part in visible)
        {
            bool isCabinet = ReferenceEquals(part, cabinet);
            parts.Add(new TvModelPart(
                part.Name,
                part.Mesh,
                Material(part.Material),
                CastShadow: !isCabinet,
                ReceiveShadow: !isCabinet));
        }

        // The plane the screen sits in, in model x: the front of the plates, or of the whole
        // model when it has none. What hides the screen is everything from the back of the
        // plates forward — the bezel of a cathode-ray set overlaps the glass rather than
        // standing in front of it.
        (Vector3 min, Vector3 max) = screens.Count > 0 ? BoundsOf(screens) : model.Bounds();
        float plane = max.X;
        Rect plates = new(min.Z, min.Y, max.Z, max.Y);
        Rect opening = MeasureOpening(visible, plates, min.X);
        (Vector3 allMin, Vector3 allMax) = BoundsOf(visible);

        return new TvModel(
            parts,
            model.Images,
            screens.Count,
            opening,
            new Rect(allMin.Z, allMin.Y, allMax.Z, allMax.Y),
            plane);
    }

    /// <summary>
    /// The matrix that maps the model frame to scene coordinates for the clip margins
    /// <paramref name="clipping"/>: the screen opening lands exactly on the visible rectangle
    /// of REQ-SCN-039, in the plane of the tunnel's front face.
    /// </summary>
    public Mat4 Placement(Vec4F clipping)
    {
        VisibleRect rect = VisibleRect.From(clipping);
        (float sx, float sy, float sz) = Scale(rect);

        float tx = rect.MaxX + (sx * _opening.MinX);
        float ty = rect.MinY - (sy * _opening.MinY);
        float tz = TunnelFront + (sz * _screenPlane);

        // Columns: model x goes to −z, model y to y, model z to −x. The determinant is
        // −sx·sy·sz, which is what maps the right-handed frame of a glTF file into the
        // left-handed scene frame of REQ-E3D-410 without mirroring what the camera sees —
        // a positive determinant here would show the model's lettering back to front. The
        // renderer reads the sign and flips the winding and the normals accordingly.
        Mat4 m = Mat4.Identity;
        m[0, 0] = 0f;
        m[0, 2] = -sx;
        m[0, 3] = tx;
        m[1, 1] = sy;
        m[1, 3] = ty;
        m[2, 0] = -sz;
        m[2, 2] = 0f;
        m[2, 3] = tz;
        return m;
    }

    /// <summary>
    /// The scene-space rectangle the model covers for <paramref name="clipping"/>, in x and y:
    /// what the cabinet hides of the tunnel behind it.
    /// </summary>
    public VisibleRect Footprint(Vec4F clipping)
    {
        VisibleRect rect = VisibleRect.From(clipping);
        (float sx, float sy, _) = Scale(rect);
        float tx = rect.MaxX + (sx * _opening.MinX);
        float ty = rect.MinY - (sy * _opening.MinY);

        // x is mirrored, so the model's lowest z is the scene's highest x.
        float right = tx - (sx * _silhouette.MinX);
        float left = tx - (sx * _silhouette.MaxX);
        return new VisibleRect(
            left,
            (sy * _silhouette.MinY) + ty,
            right,
            (sy * _silhouette.MaxY) + ty);
    }

    /// <summary>A short description, for the harness and the start-up log.</summary>
    public string Describe() => string.Create(
        CultureInfo.InvariantCulture,
        $"{Parts.Count} parts drawn, {HiddenParts} screen plates hidden, {Images.Count} textures; "
            + $"opening {_opening.MaxX - _opening.MinX:F3} x {_opening.MaxY - _opening.MinY:F3} "
            + $"of a {_silhouette.MaxX - _silhouette.MinX:F3} x {_silhouette.MaxY - _silhouette.MinY:F3} front");

    /// <summary>
    /// The three scale factors: x and y stretch the opening onto the visible rectangle, and the
    /// depth takes their mean so the cabinet keeps a plausible thickness.
    /// </summary>
    private (float X, float Y, float Z) Scale(VisibleRect rect)
    {
        float width = MathF.Max(_opening.MaxX - _opening.MinX, 1e-4f);
        float height = MathF.Max(_opening.MaxY - _opening.MinY, 1e-4f);
        float sx = rect.Width / width;
        float sy = rect.Height / height;
        return (sx, sy, (sx + sy) * 0.5f);
    }

    private static TvMaterial Material(GltfMaterial material) => new(
        BaseColor: material.BaseColorFactor,
        Diffuse: Texture(material.BaseColorTexture),
        Normal: Texture(material.NormalTexture),
        NormalScale: material.NormalScale,
        MetallicRoughness: Texture(material.MetallicRoughnessTexture),
        Metallic: material.MetallicFactor,
        Roughness: material.RoughnessFactor,
        Emissive: Texture(material.EmissiveTexture),
        EmissiveColor: material.EmissiveFactor * material.EmissiveStrength,
        AlphaCutout: material.AlphaMode == "MASK",
        AlphaCutoff: material.AlphaCutoff,
        DoubleSided: material.DoubleSided);

    private static string? Texture(int image) => image >= 0 ? TextureName(image) : null;

    private static (Vector3 Min, Vector3 Max) BoundsOf(IEnumerable<GltfPart> parts)
    {
        Vector3 min = new(float.MaxValue);
        Vector3 max = new(float.MinValue);
        foreach (GltfPart part in parts)
        {
            min = Vector3.Min(min, new Vector3(part.Mesh.BoundsMin.X, part.Mesh.BoundsMin.Y, part.Mesh.BoundsMin.Z));
            max = Vector3.Max(max, new Vector3(part.Mesh.BoundsMax.X, part.Mesh.BoundsMax.Y, part.Mesh.BoundsMax.Z));
        }

        return (min, max);
    }

    /// <summary>
    /// Measures the hole the camera looks through: the triangles that stand in front of the
    /// screen plane are projected on the (z, y) plane into a coverage grid, and the opening is
    /// the rectangle grown from the centre of the screen plates while the next row or column is
    /// clear.
    /// </summary>
    /// <remarks>
    /// A bezel is usually rounded at the corners, so a row is accepted while at most a small
    /// fraction of it is covered: a few corner cells must not stop the growth and shrink the
    /// opening to the inscribed rectangle of the rounding. The result is never larger than the
    /// plates grown by a quarter, which is what the grid covers.
    /// </remarks>
    private static Rect MeasureOpening(IReadOnlyList<GltfPart> parts, Rect plates, float blockFrom)
    {
        const int size = 192;
        const float margin = 0.25f;
        const float CornerTolerance = 0.08f;

        float width = plates.MaxX - plates.MinX;
        float height = plates.MaxY - plates.MinY;
        if (width <= 0f || height <= 0f)
        {
            return plates;
        }

        Rect area = new(
            plates.MinX - (width * margin),
            plates.MinY - (height * margin),
            plates.MaxX + (width * margin),
            plates.MaxY + (height * margin));
        float cellX = (area.MaxX - area.MinX) / size;
        float cellY = (area.MaxY - area.MinY) / size;

        bool[] covered = new bool[size * size];
        foreach (GltfPart part in parts)
        {
            ReadOnlySpan<MeshVertex> vertices = part.Mesh.Vertices;
            ReadOnlySpan<int> triangles = part.Mesh.Triangles;
            for (int at = 0; at + 2 < triangles.Length; at += 3)
            {
                Vec3 a = vertices[triangles[at]].Position;
                Vec3 b = vertices[triangles[at + 1]].Position;
                Vec3 c = vertices[triangles[at + 2]].Position;

                // Only what reaches the depth of the screen or stands in front of it hides it.
                if (MathF.Max(a.X, MathF.Max(b.X, c.X)) <= blockFrom)
                {
                    continue;
                }

                Cover(covered, size, area, cellX, cellY, a, b, c);
            }
        }

        int centreX = Clamp((int)((((plates.MinX + plates.MaxX) * 0.5f) - area.MinX) / cellX), size);
        int centreY = Clamp((int)((((plates.MinY + plates.MaxY) * 0.5f) - area.MinY) / cellY), size);

        int left = centreX;
        int right = centreX;
        int bottom = centreY;
        int top = centreY;

        bool grew = true;
        while (grew)
        {
            grew = false;
            if (left > 0 && ColumnClear(covered, size, left - 1, bottom, top, CornerTolerance))
            {
                left--;
                grew = true;
            }

            if (right < size - 1 && ColumnClear(covered, size, right + 1, bottom, top, CornerTolerance))
            {
                right++;
                grew = true;
            }

            if (bottom > 0 && RowClear(covered, size, bottom - 1, left, right, CornerTolerance))
            {
                bottom--;
                grew = true;
            }

            if (top < size - 1 && RowClear(covered, size, top + 1, left, right, CornerTolerance))
            {
                top++;
                grew = true;
            }
        }

        return new Rect(
            area.MinX + (left * cellX),
            area.MinY + (bottom * cellY),
            area.MinX + ((right + 1) * cellX),
            area.MinY + ((top + 1) * cellY));
    }

    private static bool ColumnClear(bool[] covered, int size, int column, int bottom, int top, float tolerance)
    {
        int blocked = 0;
        for (int y = bottom; y <= top; y++)
        {
            if (covered[(y * size) + column])
            {
                blocked++;
            }
        }

        return blocked <= (top - bottom + 1) * tolerance;
    }

    private static bool RowClear(bool[] covered, int size, int row, int left, int right, float tolerance)
    {
        int blocked = 0;
        for (int x = left; x <= right; x++)
        {
            if (covered[(row * size) + x])
            {
                blocked++;
            }
        }

        return blocked <= (right - left + 1) * tolerance;
    }

    /// <summary>Marks every cell a triangle's (z, y) projection touches.</summary>
    private static void Cover(
        bool[] covered,
        int size,
        Rect area,
        float cellX,
        float cellY,
        Vec3 a,
        Vec3 b,
        Vec3 c)
    {
        float minX = MathF.Min(a.Z, MathF.Min(b.Z, c.Z));
        float maxX = MathF.Max(a.Z, MathF.Max(b.Z, c.Z));
        float minY = MathF.Min(a.Y, MathF.Min(b.Y, c.Y));
        float maxY = MathF.Max(a.Y, MathF.Max(b.Y, c.Y));
        if (maxX < area.MinX || minX > area.MaxX || maxY < area.MinY || minY > area.MaxY)
        {
            return;
        }

        int x0 = Clamp((int)((minX - area.MinX) / cellX), size);
        int x1 = Clamp((int)((maxX - area.MinX) / cellX), size);
        int y0 = Clamp((int)((minY - area.MinY) / cellY), size);
        int y1 = Clamp((int)((maxY - area.MinY) / cellY), size);

        // The triangles of a cabinet are far smaller than the grid cells, so the bounding box
        // of each one is already its coverage; a point-in-triangle test would change nothing
        // but the cost.
        for (int y = y0; y <= y1; y++)
        {
            for (int x = x0; x <= x1; x++)
            {
                covered[(y * size) + x] = true;
            }
        }
    }

    private static int Clamp(int value, int size) => Math.Clamp(value, 0, size - 1);
}

/// <summary>A rectangle in the model frame, in the same two axes as <see cref="TvModel.Opening"/>.</summary>
/// <param name="MinX">Lowest model z.</param>
/// <param name="MinY">Lowest model y.</param>
/// <param name="MaxX">Highest model z.</param>
/// <param name="MaxY">Highest model y.</param>
public readonly record struct Rect(float MinX, float MinY, float MaxX, float MaxY);
