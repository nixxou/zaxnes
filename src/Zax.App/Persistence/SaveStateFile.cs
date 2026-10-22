using System.Buffers.Binary;
using System.Globalization;
using System.Numerics;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Zax.Engine3D;
using Zax.Render;
using Zax.Scripting;

namespace Zax.App.Persistence;

/// <summary>Metadata header of a savestate file (SPEC-09 REQ-PERS-056 point 2).</summary>
/// <param name="CoreId">Identifier of the core that wrote the state.</param>
/// <param name="CoreStateVersion">Format version of the core's state block.</param>
/// <param name="GameId">Profile identifier of the game.</param>
/// <param name="RomSha1">SHA-1 of the ROM content (PRG + CHR), lower-case hexadecimal.</param>
/// <param name="SavedAt">Save time, Unix seconds.</param>
/// <param name="SlotId">Slot the state was written to (<c>1</c>…<c>36</c> or <c>quick</c>).</param>
public sealed record SaveStateHeader(
    string CoreId,
    int CoreStateVersion,
    string GameId,
    string RomSha1,
    long SavedAt,
    string SlotId);

/// <summary>The four sections of a savestate and its header (SPEC-07 REQ-LOOP-040).</summary>
/// <param name="Header">Metadata.</param>
/// <param name="Core">Core state, self-contained (ROM image included).</param>
/// <param name="Lua">The 64 <c>PData</c> entries as the block of SPEC-05 §4.6.</param>
/// <param name="Camera">Camera state of SPEC-06 REQ-SCN-024.</param>
/// <param name="Conversion">Learned conversion state (DEC-R2-T5).</param>
public sealed record SaveStateContent(
    SaveStateHeader Header,
    byte[] Core,
    byte[] Lua,
    byte[] Camera,
    byte[] Conversion);

/// <summary>
/// The savestate file of SPEC-09 §4.5.4: the signature <c>ZAXNESAV</c>, a format version, a
/// length-prefixed JSON metadata header and the four length-prefixed sections core / Lua /
/// camera / conversion.
/// </summary>
/// <remarks>
/// <para>
/// Little-endian throughout. Any inconsistent length, a wrong signature or an unknown version
/// makes the file invalid, which the loop reports as <c>Corrupted save state.</c>
/// (REQ-LOOP-046, REQ-PERS-056).
/// </para>
/// <para>
/// AMD-029 §2: writing uses the product's own signature and reading accepts it as well as the
/// one of the « Sen » working name, <c>SEN3DSAV</c>, so that states written before the rename
/// keep loading. Both are eight bytes, which is what <see cref="MaxFileSize"/> counts (Q-070).
/// </para>
/// <para>
/// The thumbnail is stored beside the file as <c>&lt;slot&gt;.png</c>, which SPEC-09 §4.5.3
/// allows ("la vignette PEUT être stockée dans le fichier ou à côté").
/// </para>
/// </remarks>
public static class SaveStateFile
{
    /// <summary>The 8-byte signature every state is written with (AMD-029 §2, Q-070).</summary>
    public static ReadOnlySpan<byte> Signature => "ZAXNESAV"u8;

    /// <summary>
    /// The 8-byte signature of the « Sen » working name, still accepted on read (AMD-029 §2).
    /// Never written.
    /// </summary>
    public static ReadOnlySpan<byte> LegacySignature => "SEN3DSAV"u8;

    /// <summary>Current format version.</summary>
    public const int FormatVersion = 1;

    /// <summary>Message shown for a file that cannot be read (REQ-LOOP-046).</summary>
    public const string CorruptedMessage = "Corrupted save state.";

    /// <summary>Message shown when the slot's game is gone (REQ-LOOP-044 step 2).</summary>
    public const string GameMissingMessage = "The game profile doesn't exist anymore.";

    /// <summary>Highest metadata header size accepted, which bounds a hostile file.</summary>
    public const int MaxHeaderBytes = 4096;

    /// <summary>Size of the camera section: the mode, two sets of setpoints, the transition.</summary>
    public const int CameraSectionBytes = 4 + (2 * 7 * 4) + 4;

    private static readonly JsonSerializerOptions HeaderJson = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = false,
    };

    /// <summary>Serialises a complete savestate.</summary>
    public static byte[] Write(SaveStateContent content)
    {
        ArgumentNullException.ThrowIfNull(content);

        byte[] header = JsonSerializer.SerializeToUtf8Bytes(content.Header, HeaderJson);

        int total = Signature.Length + 4 + 4 + header.Length
            + 4 + content.Core.Length
            + 4 + content.Lua.Length
            + 4 + content.Camera.Length
            + 4 + content.Conversion.Length;

        byte[] file = new byte[total];
        Span<byte> cursor = file;

        Signature.CopyTo(cursor);
        cursor = cursor[Signature.Length..];
        cursor = WriteInt32(cursor, FormatVersion);
        cursor = WriteSection(cursor, header);
        cursor = WriteSection(cursor, content.Core);
        cursor = WriteSection(cursor, content.Lua);
        cursor = WriteSection(cursor, content.Camera);
        WriteSection(cursor, content.Conversion);

        return file;
    }

    /// <summary>
    /// Reads a savestate. Returns false — without throwing — for anything the format rejects,
    /// which is what REQ-LOOP-046 turns into <c>Corrupted save state.</c>
    /// </summary>
    public static bool TryRead(ReadOnlySpan<byte> file, out SaveStateContent? content)
    {
        content = null;

        if (file.Length < Signature.Length + 4 || !HasKnownSignature(file[..Signature.Length]))
        {
            return false;
        }

        ReadOnlySpan<byte> cursor = file[Signature.Length..];
        if (!TryReadInt32(ref cursor, out int version) || version != FormatVersion)
        {
            return false;
        }

        if (!TryReadSection(ref cursor, out ReadOnlySpan<byte> headerBytes)
            || headerBytes.Length > MaxHeaderBytes
            || !TryReadSection(ref cursor, out ReadOnlySpan<byte> core)
            || !TryReadSection(ref cursor, out ReadOnlySpan<byte> lua)
            || !TryReadSection(ref cursor, out ReadOnlySpan<byte> camera)
            || !TryReadSection(ref cursor, out ReadOnlySpan<byte> conversion))
        {
            return false;
        }

        // Trailing bytes mean the file does not match the format it claims.
        if (!cursor.IsEmpty)
        {
            return false;
        }

        if (core.Length == 0 || lua.Length != PersistentData.BlockSize || camera.Length != CameraSectionBytes)
        {
            return false;
        }

        SaveStateHeader? header;
        try
        {
            header = JsonSerializer.Deserialize<SaveStateHeader>(headerBytes, HeaderJson);
        }
        catch (JsonException)
        {
            return false;
        }

        if (header is null)
        {
            return false;
        }

        content = new SaveStateContent(
            header,
            core.ToArray(),
            lua.ToArray(),
            camera.ToArray(),
            conversion.ToArray());
        return true;
    }

    /// <summary>
    /// Upper bound on the file size, announced before serialisation and valid for the whole
    /// session of the loaded ROM and profile (SPEC-01 REQ-ARCH-013).
    /// </summary>
    /// <param name="coreMaxStateSize">What <c>NesCore.MaxStateSize()</c> returns.</param>
    public static int MaxFileSize(int coreMaxStateSize)
        => Signature.Length
            + 4
            + 4 + MaxHeaderBytes
            + 4 + coreMaxStateSize
            + 4 + PersistentData.BlockSize
            + 4 + CameraSectionBytes

            // The learned conversion state has a reserved capacity precisely so that this bound
            // exists (REQ-ARCH-013, last sentence).
            + 4 + EngineStateCodec.ReservedCapacity;

    /// <summary>
    /// Whether <paramref name="head"/> opens a savestate this build can read: the current signature
    /// or the legacy one of AMD-029 §2. The two have the same length, so neither the layout
    /// that follows nor <see cref="MaxFileSize"/> changes.
    /// </summary>
    public static bool HasKnownSignature(ReadOnlySpan<byte> head)
        => head.SequenceEqual(Signature) || head.SequenceEqual(LegacySignature);

    /// <summary>Encodes the camera state as the fixed-size camera section.</summary>
    public static byte[] WriteCamera(in CameraStateData state)
    {
        byte[] block = new byte[CameraSectionBytes];
        Span<byte> cursor = block;
        cursor = WriteInt32(cursor, (int)state.Mode);
        cursor = WriteMode(cursor, state.Tpv);
        cursor = WriteMode(cursor, state.Fpv);
        WriteSingle(cursor, state.TransitionRemaining);
        return block;
    }

    /// <summary>Decodes a camera section.</summary>
    public static CameraStateData ReadCamera(ReadOnlySpan<byte> block)
    {
        if (block.Length != CameraSectionBytes)
        {
            throw new ArgumentException("The camera section has the wrong size.", nameof(block));
        }

        int mode = BinaryPrimitives.ReadInt32LittleEndian(block);
        CameraModeState tpv = ReadMode(block.Slice(4, 7 * 4));
        CameraModeState fpv = ReadMode(block.Slice(4 + (7 * 4), 7 * 4));
        float transition = BinaryPrimitives.ReadSingleLittleEndian(block[(4 + (2 * 7 * 4))..]);
        return new CameraStateData((CameraMode)mode, tpv, fpv, transition);
    }

    /// <summary>
    /// SHA-1 of the ROM content, PRG then CHR (REQ-PERS-056 point 2). It is only logged at load
    /// time: DEC-09-Q4 forbids using it to refuse a savestate.
    /// </summary>
    public static string RomFingerprint(ReadOnlySpan<byte> prgRom, ReadOnlySpan<byte> chrRom)
    {
        byte[] buffer = new byte[prgRom.Length + chrRom.Length];
        prgRom.CopyTo(buffer);
        chrRom.CopyTo(buffer.AsSpan(prgRom.Length));
        return Convert.ToHexString(SHA1.HashData(buffer)).ToLowerInvariant();
    }

    private static Span<byte> WriteMode(Span<byte> cursor, CameraModeState mode)
    {
        cursor = WriteSingle(cursor, mode.Distance);
        cursor = WriteVector(cursor, mode.Angles);
        return WriteVector(cursor, mode.Target);
    }

    private static CameraModeState ReadMode(ReadOnlySpan<byte> block) => new(
        BinaryPrimitives.ReadSingleLittleEndian(block),
        new Vector3(
            BinaryPrimitives.ReadSingleLittleEndian(block[4..]),
            BinaryPrimitives.ReadSingleLittleEndian(block[8..]),
            BinaryPrimitives.ReadSingleLittleEndian(block[12..])),
        new Vector3(
            BinaryPrimitives.ReadSingleLittleEndian(block[16..]),
            BinaryPrimitives.ReadSingleLittleEndian(block[20..]),
            BinaryPrimitives.ReadSingleLittleEndian(block[24..])));

    private static Span<byte> WriteVector(Span<byte> cursor, Vector3 value)
    {
        cursor = WriteSingle(cursor, value.X);
        cursor = WriteSingle(cursor, value.Y);
        return WriteSingle(cursor, value.Z);
    }

    private static Span<byte> WriteSingle(Span<byte> cursor, float value)
    {
        BinaryPrimitives.WriteSingleLittleEndian(cursor, value);
        return cursor[4..];
    }

    private static Span<byte> WriteInt32(Span<byte> cursor, int value)
    {
        BinaryPrimitives.WriteInt32LittleEndian(cursor, value);
        return cursor[4..];
    }

    private static Span<byte> WriteSection(Span<byte> cursor, ReadOnlySpan<byte> section)
    {
        cursor = WriteInt32(cursor, section.Length);
        section.CopyTo(cursor);
        return cursor[section.Length..];
    }

    private static bool TryReadInt32(ref ReadOnlySpan<byte> cursor, out int value)
    {
        if (cursor.Length < 4)
        {
            value = 0;
            return false;
        }

        value = BinaryPrimitives.ReadInt32LittleEndian(cursor);
        cursor = cursor[4..];
        return true;
    }

    private static bool TryReadSection(ref ReadOnlySpan<byte> cursor, out ReadOnlySpan<byte> section)
    {
        section = default;
        if (!TryReadInt32(ref cursor, out int length) || length < 0 || length > cursor.Length)
        {
            return false;
        }

        section = cursor[..length];
        cursor = cursor[length..];
        return true;
    }

    /// <summary>Unix seconds formatted the way the slot lists show a date.</summary>
    public static string FormatTimestamp(long unixSeconds) => unixSeconds <= 0
        ? "empty"
        : DateTimeOffset.FromUnixTimeSeconds(unixSeconds).ToLocalTime()
            .ToString("yyyy-MM-dd HH:mm", CultureInfo.InvariantCulture);

    /// <summary>UTF-8 without a BOM, as SPEC-09 §4.5 requires for the JSON files.</summary>
    internal static readonly UTF8Encoding Utf8NoBom = new(encoderShouldEmitUTF8Identifier: false);
}
