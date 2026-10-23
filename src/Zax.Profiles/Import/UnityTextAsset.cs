using System.Text;

namespace Zax.Profiles.Import;

/// <summary>
/// A Unity <c>TextAsset</c> (class 49): its name and its payload (AMD-030 §4.4).
/// </summary>
/// <param name="Name">
/// <c>m_Name</c>, the asset's name — <c>Pinball</c>, <c>Blaster Master (U)</c>…
/// </param>
/// <param name="Script">
/// <c>m_Script</c>. For a 3dSen profile bundle these bytes are exactly a profile in the format
/// of SPEC-03, identical to a <c>.3dn</c> file; they are never decoded as text.
/// </param>
public sealed record UnityTextAsset(string Name, ReadOnlyMemory<byte> Script)
{
    /// <summary>The class identifier of a <c>TextAsset</c>.</summary>
    public const int ClassId = 49;

    /// <summary>The class identifier of an <c>AssetBundle</c>, which AMD-030 §4.4 ignores.</summary>
    public const int AssetBundleClassId = 142;

    /// <summary>
    /// Reads the two first fields of a <c>TextAsset</c> object.
    /// </summary>
    /// <param name="asset">The object, whose class must be <see cref="ClassId"/>.</param>
    /// <param name="bigEndian">
    /// The byte order the serialized file announced
    /// (<see cref="UnitySerializedFile.IsBigEndian"/>).
    /// </param>
    /// <exception cref="UnityAssetFormatException">
    /// The object is not a <c>TextAsset</c>, or one of the two lengths does not fit the object.
    /// </exception>
    public static UnityTextAsset Read(UnityObject asset, bool bigEndian)
    {
        ArgumentNullException.ThrowIfNull(asset);

        if (asset.ClassId != ClassId)
        {
            throw new UnityAssetFormatException(
                $"object {asset.PathId} has class {asset.ClassId} and not the {ClassId} of a TextAsset.");
        }

        AssetReader reader = new(asset.Data.Span, bigEndian);

        int nameLength = reader.ReadInt32();
        if (nameLength < 0 || nameLength > reader.Remaining)
        {
            throw new UnityAssetFormatException(
                $"the TextAsset {asset.PathId} announces a name of {nameLength} bytes in an object of "
                + $"{asset.Data.Length}.");
        }

        string name = Encoding.UTF8.GetString(reader.ReadBytes(nameLength));

        // AMD-030 §4.4: the 4-byte alignment that follows m_Name is counted from the start of
        // the object, which is where this reader starts.
        reader.Align(4);

        int scriptLength = reader.ReadInt32();
        if (scriptLength < 0 || scriptLength > reader.Remaining)
        {
            throw new UnityAssetFormatException(
                $"the TextAsset \"{name}\" announces {scriptLength} bytes of content and only "
                + $"{reader.Remaining} are left in an object of {asset.Data.Length}.");
        }

        // A slice of the object's own bytes: the payload is handed over unmodified, which is
        // what AMD-030 §5 requires of an imported profile.
        return new UnityTextAsset(name, asset.Data.Slice(reader.Position, scriptLength));
    }
}
