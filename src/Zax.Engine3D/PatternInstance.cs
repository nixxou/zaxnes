using Zax.Profiles;

namespace Zax.Engine3D;

/// <summary>How an instance is drawn (SPEC-03 <c>renderStyle</c>, REQ-E3D-191).</summary>
public enum RenderStyle
{
    /// <summary>Kept in the frame but not drawn.</summary>
    Hidden = 0,

    /// <summary>Drawn normally.</summary>
    Normal = 1,

    /// <summary>Invisible but casts its shadow.</summary>
    ShadowOnly = 2,
}

/// <summary>
/// The handle the script layer caches on a long-lived engine object so that the same object is
/// always the same Lua value (SPEC-05 REQ-LUA-043) without a per-frame lookup table
/// (REQ-LUA-111).
/// </summary>
/// <remarks>
/// <see cref="Owner"/> identifies the runtime that assigned <see cref="Handle"/>; a runtime
/// that does not recognise the owner allocates its own handle elsewhere, so two engines in one
/// process stay independent (REQ-ARCH-011). It is never copied, never serialised and has no
/// meaning outside the Lua layer.
/// </remarks>
public struct ScriptHandleSlot
{
    /// <summary>Identifier of the runtime that owns <see cref="Handle"/>; 0 when unassigned.</summary>
    public int Owner;

    /// <summary>The handle, meaningful only to the owning runtime.</summary>
    public int Handle;
}

/// <summary>
/// The 3D parameters of one instance: a pattern instance of the registry (the "3D variant" of
/// SPEC-03) or a frame instance, the per-frame copy of REQ-E3D-190.
/// </summary>
/// <remarks>
/// <para>
/// Both kinds carry the same parameters, which is why there is one type for both (SPEC-05
/// §4.2, type <c>Instance</c>). The vector and matrix parameters are public mutable fields so
/// that a script can write one component in place (<c>shape.Offset.x = …</c>, REQ-LUA-044).
/// </para>
/// <para>
/// A frame instance copies <b>every</b> parameter of its pattern instance each frame, with the
/// custom palette reset to "none" (REQ-E3D-190); tags and the link to scripts are
/// <b>shared</b> with the pattern instance, which is what <see cref="Reference"/> gives.
/// </para>
/// </remarks>
public class PatternInstance
{
    private string _tagString = string.Empty;
    private string[] _tags = [];

    /// <summary>Creates an instance with the defaults of SPEC-04 §4.3 and REQ-E3D-453.</summary>
    public PatternInstance()
    {
        Scale = Vec3.One;
        Pivot = Vec3.Half;
        DeformPivot = Vec3.Half;
        TexTilting = Vec2.One;
        Alpha = 1f;
        Style = RenderStyle.Normal;
        Depth = 3;
        Layer = ZLayer.L1;
        FillRate = 1f;
        CustomPalette = -1;
        CastShadow = true;
        ReceiveShadow = true;

        // deltaSpeed zero, deltaOffset pi/2 on rows 0 to 2 and 0 on row 3 (REQ-E3D-453).
        for (int row = 0; row < 3; row++)
        {
            for (int column = 0; column < 4; column++)
            {
                DeformOffset[row, column] = MathF.PI / 2f;
            }
        }
    }

    /// <summary>Depth in voxels (<c>sizeZ</c>, 0 to 96).</summary>
    public int Depth;

    /// <summary>Geometry family (<c>geo</c>).</summary>
    public Geo3DType Geometry;

    /// <summary>Excludes region 2 texels of the cylinders (<c>antiShadow</c>).</summary>
    public bool AntiShadow;

    /// <summary>Side of the half cylinders (<c>flip</c>).</summary>
    public bool Flip;

    /// <summary>Solid cylinder section (<c>solid</c>).</summary>
    public bool Solid;

    /// <summary>Wrapped extrusion (<c>fullWarp</c>).</summary>
    public bool FullWarp;

    /// <summary>Depth layer (<c>layer</c>).</summary>
    public ZLayer Layer;

    /// <summary>Scale factors (<c>scale</c>).</summary>
    public Vec3 Scale;

    /// <summary>Rotation in degrees (<c>rot</c>).</summary>
    public Vec3 Rotation;

    /// <summary>Offset of the anchor point (<c>offset</c>).</summary>
    public Vec3 Offset;

    /// <summary>Pivot, a fraction of the placement box per axis (<c>pivot</c>).</summary>
    public Vec3 Pivot;

    /// <summary>Opacity 0 to 1 (<c>alpha</c>).</summary>
    public float Alpha;

    /// <summary>
    /// Per-instance mesh algorithm (<c>rm</c>): 0 = use the global setting, 1 = GREEDY,
    /// 2 = MARCHING, 3 = GREEDY_MULTICOLOR (REQ-E3D-340).
    /// </summary>
    public int MeshAlgorithm;

    /// <summary>Deformation amplitude (<c>deltaAmp</c>).</summary>
    public Vec3 DeformAmplitude;

    /// <summary>Deformation pivot (<c>deltaPivot</c>).</summary>
    public Vec3 DeformPivot;

    /// <summary>Deformation speeds (<c>deltaSpeed</c>); row 3 holds <c>CData[0..3]</c>.</summary>
    public Mat4 DeformSpeed;

    /// <summary>Deformation phases (<c>deltaOffset</c>); row 3 holds <c>CData[4..7]</c>.</summary>
    public Mat4 DeformOffset;

    /// <summary>Render style (<c>renderStyle</c>).</summary>
    public RenderStyle Style;

    /// <summary>Transparent colour after normalisation (<c>emptyColor</c>, REQ-E3D-424).</summary>
    public int EmptyColor;

    /// <summary>Custom palette table, or −1 for "none" (<c>customPalette</c>, REQ-E3D-192).</summary>
    public int CustomPalette;

    /// <summary>Texture offset, a fraction of the texture size (<c>texOffset</c>).</summary>
    public Vec2 TexOffset;

    /// <summary>Texture repetition (<c>texTilting</c>).</summary>
    public Vec2 TexTilting;

    /// <summary>Fill rate of the cylindrical geometries (<c>fillrate</c>); runtime only.</summary>
    public float FillRate;

    /// <summary>Whether the instance casts a shadow; false when the tags carry <c>!cast</c>.</summary>
    public bool CastShadow;

    /// <summary>
    /// Whether the instance receives shadows; false when the tags carry <c>!receiv</c> or
    /// <c>!recv</c>.
    /// </summary>
    public bool ReceiveShadow;

    /// <summary>
    /// The custom texture of the profile, or <see langword="null"/> when the instance uses the
    /// pattern image (SPEC-04B REQ-E3D-426). Shared with the pattern instance.
    /// </summary>
    public VariantTexture? Texture { get; set; }

    /// <summary>
    /// The pattern instance whose texture this one is drawn with, or <see langword="null"/> to
    /// use its own (<see cref="Reference"/>). A script sets it through
    /// <c>shape.slot = other.slot</c> (SPEC-05 REQ-LUA-097).
    /// </summary>
    public PatternInstance? TextureSource { get; set; }

    /// <summary>
    /// The pattern instance that supplies the texture: <see cref="TextureSource"/> when a script
    /// redirected it, otherwise <see cref="Reference"/> (SPEC-05 REQ-LUA-097).
    /// </summary>
    public PatternInstance TextureOwner => TextureSource ?? Reference ?? this;

    /// <summary>Index of the instance in the instance list of its owner (0-based).</summary>
    public int Index { get; internal set; }

    /// <summary>The script layer's handle cache; not part of the instance parameters.</summary>
    public ScriptHandleSlot ScriptHandle;

    /// <summary>
    /// The pattern instance this one copies, or itself for a pattern instance (SPEC-05
    /// <c>refPattern3D</c>).
    /// </summary>
    public PatternInstance Reference { get; internal set; } = null!;

    /// <summary>
    /// The raw tag string. Writing it recomputes <see cref="Tags"/> and the two shadow flags
    /// (SPEC-04B REQ-E3D-567).
    /// </summary>
    public string TagString
    {
        get => _tagString;

        set
        {
            _tagString = value ?? string.Empty;
            _tags = [.. ProfileTags.Parse(_tagString)];
            CastShadow = !HasTag("!cast");
            ReceiveShadow = !HasTag("!receiv") && !HasTag("!recv");
        }
    }

    /// <summary>Tags of the instance: non-empty comma-separated elements, trimmed.</summary>
    public IReadOnlyList<string> Tags => _tags;

    /// <summary>True when the instance carries <paramref name="tag"/> (exact, case-sensitive).</summary>
    public bool HasTag(string tag)
    {
        foreach (string own in _tags)
        {
            if (string.Equals(own, tag, StringComparison.Ordinal))
            {
                return true;
            }
        }

        return false;
    }

    /// <summary>
    /// True when <paramref name="tag"/> is <c>"*"</c> or one of the instance's tags
    /// (SPEC-05 REQ-LUA-083 point 5).
    /// </summary>
    public bool ContainsTag(string tag)
        => string.Equals(tag, "*", StringComparison.Ordinal) || HasTag(tag);

    /// <summary>True when the instance is drawn at all (<c>Enable</c>, REQ-E3D-191).</summary>
    public bool Enable
    {
        get => Style != RenderStyle.Hidden;
        set => Style = value ? RenderStyle.Normal : RenderStyle.Hidden;
    }

    /// <summary>True when a deformation is active (SPEC-04B REQ-E3D-568).</summary>
    public bool DeformEnabled
        => DeformAmplitude.X != 0f || DeformAmplitude.Y != 0f || DeformAmplitude.Z != 0f;

    /// <summary>
    /// One of the eight free script values (<c>CData</c>, SPEC-05 REQ-LUA-082): indices 0 to 3
    /// are row 3 of <see cref="DeformSpeed"/>, 4 and up row 3 of <see cref="DeformOffset"/>.
    /// </summary>
    /// <exception cref="ArgumentOutOfRangeException">The index is negative.</exception>
    public float CustomData(int index)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(index);
        return index < 4 ? DeformSpeed[3, index] : DeformOffset[3, index & 3];
    }

    /// <summary>Writes one of the eight free script values.</summary>
    /// <exception cref="ArgumentOutOfRangeException">The index is negative.</exception>
    public void SetCustomData(int index, float value)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(index);
        if (index < 4)
        {
            DeformSpeed[3, index] = value;
        }
        else
        {
            DeformOffset[3, index & 3] = value;
        }
    }

    /// <summary>
    /// The effective mesh algorithm (SPEC-04B REQ-E3D-500): <c>rm</c> 1, 2 or 3 wins,
    /// anything else (0 included) takes the global setting.
    /// </summary>
    public RenderAlgorithm EffectiveAlgorithm(RenderAlgorithm global) => MeshAlgorithm switch
    {
        1 => RenderAlgorithm.Greedy,
        2 => RenderAlgorithm.Marching,
        3 => RenderAlgorithm.GreedyMulticolor,
        _ => global,
    };

    /// <summary>
    /// Copies every 3D parameter from <paramref name="source"/>, resets the custom palette to
    /// −1 and shares the tags and the texture with it (REQ-E3D-190).
    /// </summary>
    public void CopyFrom(PatternInstance source)
    {
        ArgumentNullException.ThrowIfNull(source);

        Depth = source.Depth;
        Geometry = source.Geometry;
        AntiShadow = source.AntiShadow;
        Flip = source.Flip;
        Solid = source.Solid;
        FullWarp = source.FullWarp;
        Layer = source.Layer;
        Scale = source.Scale;
        Rotation = source.Rotation;
        Offset = source.Offset;
        Pivot = source.Pivot;
        Alpha = source.Alpha;
        MeshAlgorithm = source.MeshAlgorithm;
        DeformAmplitude = source.DeformAmplitude;
        DeformPivot = source.DeformPivot;
        DeformSpeed = source.DeformSpeed;
        DeformOffset = source.DeformOffset;
        Style = source.Style;
        EmptyColor = source.EmptyColor;
        TexOffset = source.TexOffset;
        TexTilting = source.TexTilting;
        FillRate = source.FillRate;
        CastShadow = source.CastShadow;
        ReceiveShadow = source.ReceiveShadow;
        Texture = source.Texture;
        TextureSource = source.TextureSource;

        CustomPalette = -1;

        // Shared, not copied: the tag list and, through it, the script links.
        _tagString = source._tagString;
        _tags = source._tags;

        Reference = source.Reference;
        Index = source.Index;
    }

    /// <summary>Builds an independent copy, used when a pattern is learned or cloned.</summary>
    public PatternInstance Clone()
    {
        PatternInstance copy = new();
        copy.CopyFrom(this);
        copy.CustomPalette = CustomPalette;
        copy.Reference = copy;
        return copy;
    }

    /// <summary>
    /// Reads the 3D parameters of a profile's 3D variant (SPEC-03 §4.4), applying the defaults
    /// of REQ-PROF-034 for the sub-messages a file leaves out.
    /// </summary>
    public static PatternInstance FromVariant(PatternVariant variant)
    {
        ArgumentNullException.ThrowIfNull(variant);

        PatternInstance instance = new()
        {
            Depth = variant.Depth,
            Geometry = (Geo3DType)(int)variant.Geometry,
            AntiShadow = variant.AntiShadow,
            Flip = variant.Flip,
            Solid = variant.Solid,
            FullWarp = variant.FullWarp,
            Layer = (ZLayer)(int)variant.Layer,
            Scale = ToVec3(variant.Scale, Vec3.One),
            Rotation = ToVec3(variant.Rotation, Vec3.Zero),
            Offset = ToVec3(variant.Offset, Vec3.Zero),
            Pivot = ToVec3(variant.Pivot, Vec3.Half),
            Alpha = variant.Alpha,
            MeshAlgorithm = (int)variant.MeshAlgorithm,
            DeformAmplitude = ToVec3(variant.DeformAmplitude, Vec3.Zero),
            DeformPivot = ToVec3(variant.DeformPivot, Vec3.Half),
            Style = (RenderStyle)(int)variant.RenderStyle,
            EmptyColor = variant.EmptyColor,
            Texture = variant.Texture is { IsNullMarker: false } texture ? texture : null,
        };

        CopyMatrix(variant.DeformSpeed, ref instance.DeformSpeed, 0f);
        CopyMatrix(variant.DeformOffset, ref instance.DeformOffset, MathF.PI / 2f);
        instance.TagString = variant.TagString ?? string.Empty;
        instance.Reference = instance;
        return instance;
    }

    private static Vec3 ToVec3(Vec3F? value, Vec3 fallback)
        => value is { } v ? new Vec3(v.X, v.Y, v.Z) : fallback;

    private static void CopyMatrix(Mat4F? source, ref Mat4 destination, float absentRows012)
    {
        if (source is not { } matrix)
        {
            // An absent matrix keeps the default of REQ-E3D-453: zero speeds, pi/2 phases on
            // rows 0 to 2 and 0 on row 3.
            for (int row = 0; row < 3; row++)
            {
                for (int column = 0; column < 4; column++)
                {
                    destination[row, column] = absentRows012;
                }
            }

            for (int column = 0; column < 4; column++)
            {
                destination[3, column] = 0f;
            }

            return;
        }

        for (int row = 0; row < 4; row++)
        {
            for (int column = 0; column < 4; column++)
            {
                destination[row, column] = matrix[row, column];
            }
        }
    }
}
