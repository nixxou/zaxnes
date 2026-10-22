namespace Zax.Profiles.Wire;

/// <summary>
/// Which fields each object of a decoded profile actually carried in its file.
/// </summary>
/// <remarks>
/// <para>
/// The format lets a scalar be absent and gives it a default (SPEC-03 REQ-PROF-003): the
/// built-in profiles leave out <c>skipGeo</c> in 27 810 patterns and the trailing settings
/// fields in many profiles, because they were written by several versions of the original
/// program. A model that only kept the resolved values could not be written back byte for byte
/// (REQ-PROF-051), so the decoder records here, per object, the set of field numbers it saw,
/// and the writer leaves out the rest.
/// </para>
/// <para>
/// This keeps presence out of the domain model: a profile built in memory has no layout, and
/// the writer then writes every scalar, which is exactly what REQ-PROF-051 asks for a new
/// profile. Only objects whose field set differs from "everything the schema defines" take
/// room here.
/// </para>
/// <para>
/// Objects are identified by reference. Fields held in struct values —
/// <see cref="Vec3F"/>, <see cref="Vec4F"/>, <see cref="Mat4F"/>, <see cref="Vec2I"/> and
/// <see cref="ArgbColor"/> — cannot be keyed this way; their components are written in full.
/// Every one of them is complete in all 101 built-in profiles, which the byte-identity test
/// confirms.
/// </para>
/// </remarks>
public sealed class ProfileWireLayout
{
    // Q-010: REQ-PROF-051 asks for "scalars always written" and, in the same breath, for a
    // byte-identical rewrite of files that leave scalars out. Recording the field sets of a
    // decoded file satisfies both: a file is rewritten as it was, a new profile is written whole.
    private readonly Dictionary<object, ulong> _presentFields = new(ReferenceEqualityComparer.Instance);

    /// <summary>Number of objects whose field set differs from the full schema.</summary>
    public int RecordedObjectCount => _presentFields.Count;

    /// <summary>
    /// True when <paramref name="node"/> carried field <paramref name="fieldNumber"/>. An
    /// object the layout says nothing about counts as complete, so the writer writes it whole.
    /// </summary>
    public bool IsPresent(object? node, int fieldNumber)
    {
        if (node is null || !_presentFields.TryGetValue(node, out ulong mask))
        {
            return true;
        }

        return (mask & Bit(fieldNumber)) != 0;
    }

    /// <summary>The field numbers recorded for an object, or null when it is not recorded.</summary>
    public IReadOnlyList<int>? PresentFields(object node)
    {
        if (!_presentFields.TryGetValue(node, out ulong mask))
        {
            return null;
        }

        List<int> fields = [];
        for (int field = 1; field < 64; field++)
        {
            if ((mask & Bit(field)) != 0)
            {
                fields.Add(field);
            }
        }

        return fields;
    }

    /// <summary>
    /// Records the field set of a decoded object, but only when it is not the full
    /// <paramref name="schemaMask"/>.
    /// </summary>
    internal void Record(object node, ulong presentMask, ulong schemaMask)
    {
        if (presentMask != schemaMask)
        {
            _presentFields[node] = presentMask;
        }
    }

    internal static ulong Bit(int fieldNumber)
        => (uint)fieldNumber < 64 ? 1UL << fieldNumber : 0UL;

    internal static ulong MaskOf(params int[] fieldNumbers)
    {
        ulong mask = 0;
        foreach (int field in fieldNumbers)
        {
            mask |= Bit(field);
        }

        return mask;
    }
}
