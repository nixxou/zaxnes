namespace Zax.Profiles;

/// <summary>
/// One 3D variant of a pattern (SPEC-03 §4.4, <c>ShapeParams</c>): the parameters that turn a
/// recognised tile assembly into a 3D object.
/// </summary>
/// <remarks>
/// <para>
/// A pattern has at least one variant; the first is its main variant (REQ-PROF-034). On the
/// script side a variant is the <c>pattern</c> object (SPEC-05), which is why every field is
/// settable: scripts change these values during a game.
/// </para>
/// <para>
/// Fields whose wire type is a message are nullable here, because "absent" and "present and
/// zero" are different in the file and both must survive a rewrite (REQ-PROF-004).
/// </para>
/// </remarks>
public sealed class PatternVariant
{
    /// <summary>Depth in voxels (<c>sizeZ</c>, at most 96).</summary>
    public int Depth { get; set; }

    /// <summary>Geometry family.</summary>
    public VariantGeometry Geometry { get; set; }

    /// <summary>Excludes the object from shadow casting (<c>antiShadow</c>).</summary>
    public bool AntiShadow { get; set; }

    /// <summary>Flips the generated volume (<c>flip</c>).</summary>
    public bool Flip { get; set; }

    /// <summary>Builds a solid volume (<c>solid</c>).</summary>
    public bool Solid { get; set; }

    /// <summary>Warps the whole object rather than its surface (<c>fullWarp</c>).</summary>
    public bool FullWarp { get; set; }

    /// <summary>Depth layer the object belongs to.</summary>
    public ProfileLayer Layer { get; set; }

    /// <summary>Scale, or <see langword="null"/> when the field is absent.</summary>
    public Vec3F? Scale { get; set; }

    /// <summary>Rotation in degrees, or <see langword="null"/> when absent.</summary>
    public Vec3F? Rotation { get; set; }

    /// <summary>Offset, or <see langword="null"/> when absent.</summary>
    public Vec3F? Offset { get; set; }

    /// <summary>Pivot as a fraction of the size, or <see langword="null"/> when absent.</summary>
    public Vec3F? Pivot { get; set; }

    /// <summary>Opacity (<c>alpha</c>).</summary>
    public float Alpha { get; set; }

    /// <summary>
    /// Mesh algorithm for this variant; <see cref="MeshAlgorithm.FromSettings"/> means "use the
    /// global setting" (<c>rm</c>).
    /// </summary>
    public MeshAlgorithm MeshAlgorithm { get; set; }

    /// <summary>Deformation amplitude (<c>deltaAmp</c>), or <see langword="null"/> when absent.</summary>
    public Vec3F? DeformAmplitude { get; set; }

    /// <summary>Deformation pivot (<c>deltaPivot</c>), or <see langword="null"/> when absent.</summary>
    public Vec3F? DeformPivot { get; set; }

    /// <summary>
    /// Deformation speeds (<c>deltaSpeed</c>), or <see langword="null"/> when absent. Row 3
    /// holds the free script data <c>CData[0..3]</c>.
    /// </summary>
    public Mat4F? DeformSpeed { get; set; }

    /// <summary>
    /// Deformation phases (<c>deltaOffset</c>), or <see langword="null"/> when absent. Row 3
    /// holds the free script data <c>CData[4..7]</c>.
    /// </summary>
    public Mat4F? DeformOffset { get; set; }

    /// <summary>How the object is drawn (<c>renderStyle</c>).</summary>
    public VariantRenderStyle RenderStyle { get; set; }

    /// <summary>
    /// Texture, or <see langword="null"/> when the field is absent (no texture at all). The
    /// null marker — a texture with <see cref="VariantTexture.IsNullMarker"/> — means "use the
    /// default texture built from the tiles" (REQ-PROF-035).
    /// </summary>
    public VariantTexture? Texture { get; set; }

    /// <summary>Raw tag string, or <see langword="null"/> when the field is absent.</summary>
    public string? TagString { get; set; }

    /// <summary>
    /// Colour of the empty pixels <b>as stored</b> (<c>emptyColor</c>, a signed byte). Use
    /// <see cref="EmptyColor"/> for the value the engine sees.
    /// </summary>
    public int StoredEmptyColor { get; set; }

    /// <summary>
    /// Colour of the empty pixels after the normalisation of REQ-PROF-037: a negative stored
    /// value is brought back into 0..4 by adding 5.
    /// </summary>
    public int EmptyColor => StoredEmptyColor < 0 ? StoredEmptyColor + 5 : StoredEmptyColor;

    /// <summary>Tags of this variant, split and trimmed (REQ-PROF-036).</summary>
    public IReadOnlyList<string> Tags => ProfileTags.Parse(TagString);

    /// <summary>True when this variant carries <paramref name="tag"/> (exact, case-sensitive).</summary>
    public bool HasTag(string tag)
    {
        foreach (string own in Tags)
        {
            if (string.Equals(own, tag, StringComparison.Ordinal))
            {
                return true;
            }
        }

        return false;
    }

    /// <summary>
    /// One of the eight free script values (<c>CData</c>): indices 0 to 3 come from row 3 of
    /// <see cref="DeformSpeed"/>, indices 4 to 7 from row 3 of <see cref="DeformOffset"/>
    /// (SPEC-03 §4.4). Zero when the matrix is absent.
    /// </summary>
    /// <exception cref="ArgumentOutOfRangeException">Index outside 0..7.</exception>
    public float CustomData(int index)
    {
        if ((uint)index > 7)
        {
            throw new ArgumentOutOfRangeException(nameof(index));
        }

        Mat4F? matrix = index < 4 ? DeformSpeed : DeformOffset;
        return matrix is null ? 0f : matrix.Value[3, index & 3];
    }
}
