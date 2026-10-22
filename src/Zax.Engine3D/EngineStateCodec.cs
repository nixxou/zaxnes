using System.Text;

namespace Zax.Engine3D;

/// <summary>
/// Reads and writes the <b>learned state</b> of the conversion: the images met while playing,
/// the learned patterns, the temporary patterns and the tracker. A savestate carries it, which
/// is what makes a reload replay identically (DEC-R2-T5, REQ-E3D-008, SPEC-07 REQ-LOOP-040).
/// </summary>
/// <remarks>
/// <para>
/// The profile's own patterns and images are <b>not</b> written: they come back from the
/// profile, with the same identities, because loading a profile is deterministic. A state is
/// therefore only valid for the profile that was loaded when it was written, which
/// <see cref="Write"/> records and <see cref="Read"/> checks.
/// </para>
/// <para>
/// The format is little-endian and fully determined by the state, so writing the same state
/// twice gives the same bytes (REQ-ARCH-012).
/// </para>
/// </remarks>
public static class EngineStateCodec
{
    /// <summary>Magic of the block: <c>SE3D</c>.</summary>
    public const uint Magic = 0x44334553;

    /// <summary>Version of the format.</summary>
    public const ushort Version = 1;

    /// <summary>
    /// The capacity a host may reserve for this block so that it can announce an upper bound
    /// on the savestate size before serialising (REQ-ARCH-013).
    /// </summary>
    public const int ReservedCapacity = 8 * 1024 * 1024;

    /// <summary>Whether a written block fits the reserved capacity (REQ-ARCH-013).</summary>
    public static bool FitsReservedCapacity(int length) => length <= ReservedCapacity;

    /// <summary>Writes the learned state of <paramref name="engine"/>.</summary>
    public static byte[] Write(ConversionEngine engine)
    {
        ArgumentNullException.ThrowIfNull(engine);

        using MemoryStream stream = new();
        using BinaryWriter writer = new(stream, Encoding.UTF8, leaveOpen: true);

        writer.Write(Magic);
        writer.Write(Version);
        writer.Write(engine.ProfileImageCount);
        writer.Write(engine.Patterns.ProfilePatternCount);

        // Images met while playing, in registry order, so that reinterning them in that order
        // gives back the same identities (REQ-E3D-007).
        IReadOnlyList<TileImage> images = engine.Images.Images;
        int learnedImageCount = Math.Max(0, images.Count - engine.ProfileImageCount);
        writer.Write(learnedImageCount);
        for (int id = engine.ProfileImageCount; id < images.Count; id++)
        {
            TileImageKey key = images[id].Key;
            writer.Write(key.Low);
            writer.Write(key.High);
            writer.Write((byte)key.Flip);
        }

        // Learned permanent patterns, then every temporary pattern. Their order in the block is
        // their registration order, which the read puts back.
        List<EnginePattern> learned = [];
        foreach (EnginePattern pattern in engine.Patterns.Permanent)
        {
            if (pattern.RegistrationIndex >= engine.Patterns.ProfilePatternCount)
            {
                learned.Add(pattern);
            }
        }

        List<EnginePattern> temporary = [.. engine.Patterns.Temporary];

        writer.Write(learned.Count);
        foreach (EnginePattern pattern in learned)
        {
            WritePattern(writer, pattern);
        }

        writer.Write(temporary.Count);
        foreach (EnginePattern pattern in temporary)
        {
            WritePattern(writer, pattern);
        }

        WriteTracker(writer, engine, learned, temporary);

        writer.Flush();
        return stream.ToArray();
    }

    /// <summary>
    /// Puts a learned state back: drops what was learned since the profile loaded, restores the
    /// images, the patterns and the tracker, and rebuilds the geometry of the restored patterns
    /// (REQ-E3D-008, REQ-E3D-322).
    /// </summary>
    /// <exception cref="InvalidDataException">
    /// The block is not a learned state, its version is unknown, or it was written with another
    /// profile loaded.
    /// </exception>
    public static void Read(ConversionEngine engine, ReadOnlySpan<byte> block)
    {
        ArgumentNullException.ThrowIfNull(engine);

        byte[] bytes = block.ToArray();
        using MemoryStream stream = new(bytes, writable: false);
        using BinaryReader reader = new(stream, Encoding.UTF8, leaveOpen: true);

        if (reader.ReadUInt32() != Magic)
        {
            throw new InvalidDataException("Not a conversion learned-state block.");
        }

        ushort version = reader.ReadUInt16();
        if (version != Version)
        {
            throw new InvalidDataException($"Unsupported learned-state version {version}.");
        }

        int profileImageCount = reader.ReadInt32();
        int profilePatternCount = reader.ReadInt32();
        if (profileImageCount != engine.ProfileImageCount
            || profilePatternCount != engine.Patterns.ProfilePatternCount)
        {
            throw new InvalidDataException(
                "The learned state was written with another profile loaded.");
        }

        engine.Patterns.ClearLearned();
        engine.Images.TruncateTo(profileImageCount);

        int learnedImageCount = reader.ReadInt32();
        for (int i = 0; i < learnedImageCount; i++)
        {
            ulong low = reader.ReadUInt64();
            ulong high = reader.ReadUInt64();
            byte flip = reader.ReadByte();
            engine.Images.Intern(low, high, (flip & 1) != 0, (flip & 2) != 0);
        }

        List<EnginePattern> restored = [];
        int learnedCount = reader.ReadInt32();
        for (int i = 0; i < learnedCount; i++)
        {
            restored.Add(ReadPattern(reader, engine, permanent: true));
        }

        int learnedPermanentCount = restored.Count;
        int temporaryCount = reader.ReadInt32();
        for (int i = 0; i < temporaryCount; i++)
        {
            restored.Add(ReadPattern(reader, engine, permanent: false));
        }

        ReadTracker(reader, engine, restored, learnedPermanentCount);

        foreach (EnginePattern pattern in restored)
        {
            engine.RebuildGeometryOf(pattern);
        }
    }

    private static void WritePattern(BinaryWriter writer, EnginePattern pattern)
    {
        writer.Write((ushort)pattern.Width);
        writer.Write((ushort)pattern.Height);
        writer.Write(pattern.SelfConnected);
        writer.Write(pattern.SkipGeometry);
        writer.Write(pattern.ConstraintSize.X);
        writer.Write(pattern.ConstraintSize.Y);
        writer.Write(pattern.StoredPixelCount);
        writer.Write(pattern.StoredColorFlags);
        writer.Write(pattern.StoredTileDensity);
        writer.Write(pattern.StoredPixelDensity);

        for (int j = 0; j < pattern.Height; j++)
        {
            for (int i = 0; i < pattern.Width; i++)
            {
                writer.Write(pattern.ImageAt(i, j)?.Id ?? -1);
            }
        }

        writer.Write((ushort)pattern.Instances.Count);
        foreach (PatternInstance instance in pattern.Instances)
        {
            WriteInstance(writer, instance);
        }
    }

    private static EnginePattern ReadPattern(BinaryReader reader, ConversionEngine engine, bool permanent)
    {
        int width = reader.ReadUInt16();
        int height = reader.ReadUInt16();
        bool selfConnected = reader.ReadBoolean();
        bool skipGeometry = reader.ReadBoolean();
        int constraintX = reader.ReadInt32();
        int constraintY = reader.ReadInt32();
        int storedPixelCount = reader.ReadInt32();
        int storedColorFlags = reader.ReadInt32();
        int storedTileDensity = reader.ReadInt32();
        int storedPixelDensity = reader.ReadInt32();

        TileImage?[] cells = new TileImage?[width * height];
        for (int n = 0; n < cells.Length; n++)
        {
            int id = reader.ReadInt32();
            cells[n] = id < 0 ? null : engine.Images.ById(id);
        }

        EnginePattern pattern = new(width, height, cells, permanent)
        {
            SkipGeometry = skipGeometry,
            ConstraintSize = new Cell(constraintX, constraintY),
            StoredPixelCount = storedPixelCount,
            StoredColorFlags = storedColorFlags,
            StoredTileDensity = storedTileDensity,
            StoredPixelDensity = storedPixelDensity,
        };
        pattern.SetSelfConnected(selfConnected);

        int instanceCount = reader.ReadUInt16();
        for (int k = 0; k < instanceCount; k++)
        {
            pattern.AddInstance(ReadInstance(reader));
        }

        // Registered as recorded: the replacement rules of REQ-E3D-065 already ran when these
        // patterns were first added, and a restore must give back exactly what was saved.
        engine.Patterns.Register(pattern);
        return pattern;
    }

    private static void WriteInstance(BinaryWriter writer, PatternInstance instance)
    {
        writer.Write(instance.Depth);
        writer.Write((byte)instance.Geometry);
        writer.Write((byte)instance.Layer);
        writer.Write((byte)instance.Style);
        writer.Write(instance.AntiShadow);
        writer.Write(instance.Flip);
        writer.Write(instance.Solid);
        writer.Write(instance.FullWarp);
        writer.Write(instance.CastShadow);
        writer.Write(instance.ReceiveShadow);
        WriteVec3(writer, instance.Scale);
        WriteVec3(writer, instance.Rotation);
        WriteVec3(writer, instance.Offset);
        WriteVec3(writer, instance.Pivot);
        writer.Write(instance.Alpha);
        writer.Write(instance.MeshAlgorithm);
        WriteVec3(writer, instance.DeformAmplitude);
        WriteVec3(writer, instance.DeformPivot);
        WriteMat4(writer, instance.DeformSpeed);
        WriteMat4(writer, instance.DeformOffset);
        writer.Write(instance.EmptyColor);
        writer.Write(instance.CustomPalette);
        writer.Write(instance.TexOffset.X);
        writer.Write(instance.TexOffset.Y);
        writer.Write(instance.TexTilting.X);
        writer.Write(instance.TexTilting.Y);
        writer.Write(instance.FillRate);
        writer.Write(instance.TagString);
    }

    private static PatternInstance ReadInstance(BinaryReader reader)
    {
        PatternInstance instance = new()
        {
            Depth = reader.ReadInt32(),
            Geometry = (Geo3DType)reader.ReadByte(),
            Layer = (ZLayer)reader.ReadByte(),
            Style = (RenderStyle)reader.ReadByte(),
            AntiShadow = reader.ReadBoolean(),
            Flip = reader.ReadBoolean(),
            Solid = reader.ReadBoolean(),
            FullWarp = reader.ReadBoolean(),
        };

        bool castShadow = reader.ReadBoolean();
        bool receiveShadow = reader.ReadBoolean();
        instance.Scale = ReadVec3(reader);
        instance.Rotation = ReadVec3(reader);
        instance.Offset = ReadVec3(reader);
        instance.Pivot = ReadVec3(reader);
        instance.Alpha = reader.ReadSingle();
        instance.MeshAlgorithm = reader.ReadInt32();
        instance.DeformAmplitude = ReadVec3(reader);
        instance.DeformPivot = ReadVec3(reader);
        instance.DeformSpeed = ReadMat4(reader);
        instance.DeformOffset = ReadMat4(reader);
        instance.EmptyColor = reader.ReadInt32();
        instance.CustomPalette = reader.ReadInt32();
        instance.TexOffset = new Vec2(reader.ReadSingle(), reader.ReadSingle());
        instance.TexTilting = new Vec2(reader.ReadSingle(), reader.ReadSingle());
        instance.FillRate = reader.ReadSingle();

        // Writing the tag string recomputes the shadow flags, so they are restored after it.
        instance.TagString = reader.ReadString();
        instance.CastShadow = castShadow;
        instance.ReceiveShadow = receiveShadow;
        return instance;
    }

    private static void WriteTracker(
        BinaryWriter writer,
        ConversionEngine engine,
        List<EnginePattern> learned,
        List<EnginePattern> temporary)
    {
        ShapeTracker tracker = engine.Tracker;
        writer.Write(tracker.Running);
        writer.Write(tracker.Sprites);
        WriteVec3(writer, tracker.Position);

        writer.Write(tracker.Included.Count);
        foreach (EnginePattern pattern in tracker.Included)
        {
            writer.Write(Reference(engine, pattern, learned, temporary));
        }

        writer.Write(tracker.Excluded.Count);
        foreach (EnginePattern pattern in tracker.Excluded)
        {
            writer.Write(Reference(engine, pattern, learned, temporary));
        }
    }

    private static void ReadTracker(
        BinaryReader reader,
        ConversionEngine engine,
        List<EnginePattern> restored,
        int learnedPermanentCount)
    {
        bool running = reader.ReadBoolean();
        bool sprites = reader.ReadBoolean();
        Vec3 position = ReadVec3(reader);

        List<EnginePattern> included = ReadReferences(reader, engine, restored, learnedPermanentCount);
        List<EnginePattern> excluded = ReadReferences(reader, engine, restored, learnedPermanentCount);
        engine.Tracker.Restore(running, sprites, position, included, excluded);
    }

    private static List<EnginePattern> ReadReferences(
        BinaryReader reader,
        ConversionEngine engine,
        List<EnginePattern> restored,
        int learnedPermanentCount)
    {
        int count = reader.ReadInt32();
        List<EnginePattern> result = new(count);
        for (int i = 0; i < count; i++)
        {
            int reference = reader.ReadInt32();
            EnginePattern? pattern = Resolve(engine, reference, restored, learnedPermanentCount);
            if (pattern is not null)
            {
                result.Add(pattern);
            }
        }

        return result;
    }

    /// <summary>
    /// A stable reference to a registry pattern: its registration index when the profile
    /// supplied it, and <c>−1 − k</c> for the <c>k</c>-th pattern of this block otherwise.
    /// </summary>
    private static int Reference(
        ConversionEngine engine,
        EnginePattern pattern,
        List<EnginePattern> learned,
        List<EnginePattern> temporary)
    {
        if (pattern.RegistrationIndex < engine.Patterns.ProfilePatternCount
            && pattern.Permanent)
        {
            return pattern.RegistrationIndex;
        }

        int index = learned.IndexOf(pattern);
        if (index >= 0)
        {
            return -1 - index;
        }

        index = temporary.IndexOf(pattern);
        return index >= 0 ? -1 - learned.Count - index : int.MinValue;
    }

    private static EnginePattern? Resolve(
        ConversionEngine engine,
        int reference,
        List<EnginePattern> restored,
        int learnedPermanentCount)
    {
        if (reference == int.MinValue)
        {
            return null;
        }

        if (reference >= 0)
        {
            foreach (EnginePattern pattern in engine.Patterns.Permanent)
            {
                if (pattern.RegistrationIndex == reference)
                {
                    return pattern;
                }
            }

            return null;
        }

        int index = -1 - reference;
        _ = learnedPermanentCount;
        return (uint)index < (uint)restored.Count ? restored[index] : null;
    }

    private static void WriteVec3(BinaryWriter writer, Vec3 value)
    {
        writer.Write(value.X);
        writer.Write(value.Y);
        writer.Write(value.Z);
    }

    private static Vec3 ReadVec3(BinaryReader reader)
        => new(reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle());

    private static void WriteMat4(BinaryWriter writer, Mat4 value)
    {
        Span<float> cells = stackalloc float[16];
        value.CopyTo(cells);
        foreach (float cell in cells)
        {
            writer.Write(cell);
        }
    }

    private static Mat4 ReadMat4(BinaryReader reader)
    {
        Mat4 matrix = default;
        for (int row = 0; row < 4; row++)
        {
            for (int column = 0; column < 4; column++)
            {
                matrix[row, column] = reader.ReadSingle();
            }
        }

        return matrix;
    }
}
