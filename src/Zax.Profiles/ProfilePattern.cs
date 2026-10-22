namespace Zax.Profiles;

/// <summary>
/// A pattern: a rectangular assembly of tiles recognised on screen and converted into a 3D
/// object (SPEC-03 §4.3, SPEC-04 for the behaviour).
/// </summary>
/// <remarks>
/// The scalar defaults of REQ-PROF-034 are the ones this type starts with:
/// <c>constraintSize = (99, 99)</c>, <c>selfConnected = true</c>, <c>skipGeo = false</c>. The
/// family of a pattern — permanent or temporary — is given by <see cref="Permanent"/>, not by
/// the list of the file it was read from (REQ-PROF-023).
/// </remarks>
public sealed class ProfilePattern
{
    /// <summary>
    /// The tile grid, or <see langword="null"/> when the pattern has none (the patterns of the
    /// editor's memorised slots, REQ-PROF-044).
    /// </summary>
    public PatternGrid? Grid { get; set; }

    /// <summary>
    /// Size constraint; <c>(99, 99)</c> means unconstrained and <c>(100, 100)</c> is the "V1"
    /// constraint (SPEC-04).
    /// </summary>
    public Vec2I ConstraintSize { get; set; } = Vec2I.Unconstrained;

    /// <summary>
    /// Size in tiles (width, height), or <see langword="null"/> when the field is absent.
    /// </summary>
    public Vec2I? TileSize { get; set; }

    /// <summary>Permanent (true) or temporary (false) pattern.</summary>
    public bool Permanent { get; set; }

    /// <summary>Stored pixel count; a memorised value, never recomputed (SPEC-04).</summary>
    public int PixelCount { get; set; }

    /// <summary>Colours occurring in the pattern.</summary>
    public PatternColorFlags ColorFlags { get; set; }

    /// <summary>Density in tiles. Values outside the enumeration occur and are kept.</summary>
    public PatternDensity TileDensity { get; set; }

    /// <summary>Density in pixels. Values outside the enumeration occur and are kept.</summary>
    public PatternDensity PixelDensity { get; set; }

    /// <summary>
    /// The 3D variants, in file order; the first is the main variant. A pattern read from a
    /// file always has at least one (REQ-PROF-034).
    /// </summary>
    public IList<PatternVariant> Variants { get; } = [];

    /// <summary>Internal connectivity option (<c>selfConnected</c>).</summary>
    public bool SelfConnected { get; set; } = true;

    /// <summary>Skip geometry analysis (<c>skipGeo</c>).</summary>
    public bool SkipGeometry { get; set; }

    /// <summary>The main variant, or <see langword="null"/> when the pattern has none.</summary>
    public PatternVariant? MainVariant => Variants.Count > 0 ? Variants[0] : null;
}

/// <summary>
/// One of the editor's memorised slots (SPEC-03 REQ-PROF-044, <c>RecordedSlot</c>): a name and
/// a pattern.
/// </summary>
/// <remarks>
/// These are shape and deformation presets of the profile editor. They have no effect on a
/// game but must be preserved (SPEC-09 REQ-PERS-016). The pattern of a slot has no grid, a
/// <c>tSize</c> of (0, 0) and one variant, and it is written as a <b>direct message</b>,
/// without a reference envelope.
/// </remarks>
public sealed class RecordedPatternSlot
{
    /// <summary>Creates a slot.</summary>
    public RecordedPatternSlot(string? name, ProfilePattern? pattern)
    {
        Name = name;
        Pattern = pattern;
    }

    /// <summary>Slot name as shown by the editor, or <see langword="null"/> when absent.</summary>
    public string? Name { get; set; }

    /// <summary>The stored pattern, or <see langword="null"/> when the field is absent.</summary>
    public ProfilePattern? Pattern { get; set; }

    /// <summary>
    /// Builds the five default shape slots <c>Config1</c>…<c>Config5</c> used when the list is
    /// missing from a profile (DEC-03-Q03, REQ-PROF-025).
    /// </summary>
    public static IReadOnlyList<RecordedPatternSlot> DefaultShapeSlots() => CreateDefaults("Config");

    /// <summary>
    /// Builds the five default deformation slots <c>Deform1</c>…<c>Deform5</c> used when the
    /// list is missing from a profile (DEC-03-Q03, REQ-PROF-025).
    /// </summary>
    public static IReadOnlyList<RecordedPatternSlot> DefaultDeformSlots() => CreateDefaults("Deform");

    private static RecordedPatternSlot[] CreateDefaults(string prefix)
    {
        RecordedPatternSlot[] slots = new RecordedPatternSlot[5];
        for (int i = 0; i < slots.Length; i++)
        {
            ProfilePattern pattern = new() { TileSize = new Vec2I(0, 0) };
            pattern.Variants.Add(new PatternVariant());
            slots[i] = new RecordedPatternSlot($"{prefix}{i + 1}", pattern);
        }

        return slots;
    }
}
