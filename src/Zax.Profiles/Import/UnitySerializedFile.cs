namespace Zax.Profiles.Import;

/// <summary>One object of a serialized file (AMD-030 §4.3, object table).</summary>
/// <param name="PathId">The object's 64-bit identifier.</param>
/// <param name="ClassId">
/// Its class, taken from the type the object's type index designates: 49 for a
/// <c>TextAsset</c>, 142 for the <c>AssetBundle</c>.
/// </param>
/// <param name="Data">The object's bytes, inside the serialized file's data area.</param>
public sealed record UnityObject(long PathId, int ClassId, ReadOnlyMemory<byte> Data);

/// <summary>
/// Reader of a Unity serialized file — the <c>CAB-…</c> node of a bundle — for versions 17 to 22
/// and both byte orders (AMD-030 §4.3).
/// </summary>
/// <remarks>
/// <para>
/// Only what is needed to reach the objects is decoded: the header, the type list (walked over,
/// type tree included, because its length is what tells where the object table starts) and the
/// object table. The references, the externals and the user information that follow the object
/// table are never read.
/// </para>
/// <para>
/// The header is big-endian; the metadata that follows uses the order the header announces,
/// which is why <see cref="IsBigEndian"/> is carried over to whoever reads an object's fields
/// (AMD-030 §4.4).
/// </para>
/// </remarks>
public sealed class UnitySerializedFile
{
    /// <summary>Lowest accepted version.</summary>
    public const uint MinimumVersion = 17;

    /// <summary>Highest accepted version.</summary>
    public const uint MaximumVersion = 22;

    /// <summary>Class identifier of a <c>MonoBehaviour</c>, whose type carries a script GUID.</summary>
    private const int MonoBehaviourClassId = 114;

    /// <summary>Largest accepted number of types, objects or type-tree nodes.</summary>
    private const int MaximumTableLength = 1 << 20;

    /// <summary>Largest accepted string buffer of a type tree.</summary>
    private const int MaximumStringBuffer = 64 * 1024 * 1024;

    private UnitySerializedFile(
        uint version,
        bool bigEndian,
        string unityVersion,
        int targetPlatform,
        IReadOnlyList<UnityObject> objects)
    {
        Version = version;
        IsBigEndian = bigEndian;
        UnityVersion = unityVersion;
        TargetPlatform = targetPlatform;
        Objects = objects;
    }

    /// <summary>The file's version, 17 to 22 (21 on the measured file).</summary>
    public uint Version { get; }

    /// <summary>Whether the metadata and the object data are big-endian.</summary>
    public bool IsBigEndian { get; }

    /// <summary>The Unity version string the metadata opens with.</summary>
    public string UnityVersion { get; }

    /// <summary>The target platform number.</summary>
    public int TargetPlatform { get; }

    /// <summary>The objects, in the order of the object table.</summary>
    public IReadOnlyList<UnityObject> Objects { get; }

    /// <summary>
    /// Reads a serialized file.
    /// </summary>
    /// <param name="bytes">
    /// The whole serialized file. The objects keep slices of it, so the array must not be
    /// changed afterwards.
    /// </param>
    /// <exception cref="UnityAssetFormatException">
    /// The version is outside 17 to 22, or a size, an offset or a type index does not fit the
    /// bytes at hand.
    /// </exception>
    public static UnitySerializedFile Read(byte[] bytes)
    {
        ArgumentNullException.ThrowIfNull(bytes);

        // AMD-030 §4.3: the header is big-endian, whatever order it then announces.
        AssetReader reader = new(bytes, bigEndian: true);

        long metadataSize = reader.ReadUInt32();
        long fileSize = reader.ReadUInt32();
        uint version = reader.ReadUInt32();
        long dataOffset = reader.ReadUInt32();
        bool bigEndian = reader.ReadByte() != 0;
        reader.Skip(3);

        if (version is < MinimumVersion or > MaximumVersion)
        {
            throw new UnityAssetFormatException(
                $"the serialized file version {version} is not supported; "
                + $"{MinimumVersion} to {MaximumVersion} are.");
        }

        if (version >= 22)
        {
            // Q-071: from version 22 the sizes and offsets are given again, the file size and
            // the data offset as 64-bit fields and the metadata size as a 32-bit one, followed
            // by one reserved 64-bit field. No measured file uses this branch (the installation
            // carries version 21), so the public layout is the one implemented.
            metadataSize = reader.ReadUInt32();
            fileSize = reader.ReadInt64();
            dataOffset = reader.ReadInt64();
            reader.Skip(8);
        }

        if (fileSize > bytes.Length)
        {
            throw new UnityAssetFormatException(
                $"the serialized file announces {fileSize} bytes and only {bytes.Length} are there: "
                + "it is truncated.");
        }

        if (dataOffset < 0 || dataOffset > bytes.Length)
        {
            throw new UnityAssetFormatException(
                $"the serialized file puts its data at offset {dataOffset} of {bytes.Length} bytes.");
        }

        if (metadataSize < 0)
        {
            throw new UnityAssetFormatException(
                $"the serialized file announces a metadata block of {metadataSize} bytes.");
        }

        reader.BigEndian = bigEndian;

        string unityVersion = reader.ReadNullTerminatedString();
        int targetPlatform = reader.ReadInt32();
        bool enableTypeTree = reader.ReadByte() != 0;

        int typeCount = reader.ReadInt32();
        CheckTableLength(typeCount, "types");

        int[] classIds = new int[typeCount];
        for (int index = 0; index < typeCount; index++)
        {
            classIds[index] = ReadType(ref reader, version, enableTypeTree);
        }

        int objectCount = reader.ReadInt32();
        CheckTableLength(objectCount, "objects");

        List<UnityObject> objects = new(objectCount);
        for (int index = 0; index < objectCount; index++)
        {
            // AMD-030 §4.3: the object table is aligned on 4 bytes from the start of the file.
            // Each record is a multiple of 4 bytes long, so this only ever moves on the first
            // one, but it is what the format says.
            reader.Align(4);

            long pathId = reader.ReadInt64();
            long byteStart = version >= 22 ? reader.ReadInt64() : reader.ReadUInt32();
            long byteSize = reader.ReadUInt32();
            int typeIndex = reader.ReadInt32();

            if ((uint)typeIndex >= (uint)typeCount)
            {
                throw new UnityAssetFormatException(
                    $"object {index} of the serialized file points at type {typeIndex} of {typeCount}.");
            }

            long start = dataOffset + byteStart;
            if (byteStart < 0 || byteSize < 0 || start + byteSize > bytes.Length)
            {
                throw new UnityAssetFormatException(
                    $"object {index} of the serialized file claims {byteSize} bytes at offset {start} "
                    + $"of {bytes.Length}: the object table is out of range.");
            }

            objects.Add(new UnityObject(
                pathId,
                classIds[typeIndex],
                new ReadOnlyMemory<byte>(bytes, (int)start, (int)byteSize)));
        }

        return new UnitySerializedFile(version, bigEndian, unityVersion, targetPlatform, objects);
    }

    /// <summary>
    /// Walks over one entry of the type list and returns its class identifier (AMD-030 §4.3,
    /// metadata).
    /// </summary>
    private static int ReadType(ref AssetReader reader, uint version, bool enableTypeTree)
    {
        int classId = reader.ReadInt32();
        reader.Skip(1); // « allégé »: the stripped flag.
        reader.Skip(2); // The script type index.

        if (classId == MonoBehaviourClassId)
        {
            reader.Skip(16); // The script GUID.
        }

        reader.Skip(16); // The old type hash.

        // Q-071: the type dependencies of version 21 and later belong to the type-tree block in
        // the public format, so nothing more is read when the tree is absent.
        if (!enableTypeTree)
        {
            return classId;
        }

        int nodeCount = reader.ReadInt32();
        CheckTableLength(nodeCount, "type-tree nodes");

        int stringBufferSize = reader.ReadInt32();
        if (stringBufferSize < 0 || stringBufferSize > MaximumStringBuffer)
        {
            throw new UnityAssetFormatException(
                $"a type tree announces a string buffer of {stringBufferSize} bytes.");
        }

        int nodeSize = version >= 19 ? 32 : 24;
        reader.Skip(nodeCount * nodeSize);
        reader.Skip(stringBufferSize);

        if (version >= 21)
        {
            int dependencyCount = reader.ReadInt32();
            CheckTableLength(dependencyCount, "type dependencies");
            reader.Skip(dependencyCount * 4);
        }

        return classId;
    }

    private static void CheckTableLength(int count, string what)
    {
        if (count < 0 || count > MaximumTableLength)
        {
            throw new UnityAssetFormatException(
                $"the serialized file announces {count} {what}, which is not a usable count.");
        }
    }
}
