namespace Zax.Nes.Rom;

/// <summary>
/// A parsed 16-byte iNES or NES 2.0 header (REQ-CORE-010, REQ-CORE-012, REQ-CORE-013).
/// </summary>
/// <remarks>
/// Parsing is pure: it never consults the mapper registry and never looks at the ROM
/// payload. Rejection of unsupported mappers and of truncated files happens in
/// <see cref="RomImage"/> (REQ-CORE-011), so that the header vectors of SPEC-02 §5.2 can
/// be checked on their own.
/// </remarks>
// Q-001: the failure phrases of REQ-CORE-011 are read as descriptions of the cause, not as
// literal strings, and the messages are written in English like the one literal message
// SPEC-02 does impose (REQ-CORE-120).
public sealed record InesHeader
{
    /// <summary>Size in bytes of the header itself.</summary>
    public const int Size = 16;

    /// <summary>Size in bytes of the optional trainer (REQ-CORE-012).</summary>
    public const int TrainerSize = 512;

    /// <summary>CPU address the trainer is loaded at (REQ-CORE-012).</summary>
    public const ushort TrainerLoadAddress = 0x7000;

    /// <summary>Default CHR-RAM size when the header announces no CHR-ROM (REQ-CORE-012).</summary>
    public const int DefaultChrRamSize = 32 * 1024;

    /// <summary>PRG-RAM always mapped at $6000-$7FFF unless the mapper disables it (REQ-CORE-012).</summary>
    public const int DefaultPrgRamSize = 8 * 1024;

    private static readonly byte[] Signature = [0x4E, 0x45, 0x53, 0x1A];

    private InesHeader()
    {
    }

    /// <summary>True when the header carries the NES 2.0 extensions.</summary>
    public bool IsNes20 { get; private init; }

    /// <summary>
    /// True for a "dirty" iNES header: not NES 2.0 and bytes 12 to 15 are not all zero
    /// (for instance the "DiskDude!" signature). The high nibble of byte 7 is then
    /// ignored, and the region is NTSC without reading bytes 7 to 15 (REQ-CORE-012,
    /// REQ-CORE-013).
    /// </summary>
    public bool IsDirty { get; private init; }

    /// <summary>iNES mapper number.</summary>
    public int Mapper { get; private init; }

    /// <summary>NES 2.0 submapper number, 0 when absent.</summary>
    public int Submapper { get; private init; }

    /// <summary>PRG-ROM size in bytes.</summary>
    public int PrgRomSize { get; private init; }

    /// <summary>CHR-ROM size in bytes; 0 when the cartridge uses CHR-RAM.</summary>
    public int ChrRomSize { get; private init; }

    /// <summary>CHR-RAM size in bytes; 0 when the cartridge has CHR-ROM.</summary>
    public int ChrRamSize { get; private init; }

    /// <summary>PRG-RAM size in bytes at $6000-$7FFF.</summary>
    public int PrgRamSize { get; private init; }

    /// <summary>Initial nametable layout (REQ-CORE-012).</summary>
    public NametableMirroring Mirroring { get; private init; }

    /// <summary>Bit 1 of byte 6: the PRG-RAM is battery backed (REQ-CORE-095).</summary>
    public bool HasBattery { get; private init; }

    /// <summary>Bit 2 of byte 6: a 512-byte trainer follows the header.</summary>
    public bool HasTrainer { get; private init; }

    /// <summary>Region announced by the header (REQ-CORE-013).</summary>
    public NesRegion Region { get; private init; }

    /// <summary>Offset of the PRG-ROM inside the ROM image, trainer accounted for.</summary>
    public int PrgRomOffset => Size + (HasTrainer ? TrainerSize : 0);

    /// <summary>Offset of the CHR-ROM inside the ROM image.</summary>
    public int ChrRomOffset => PrgRomOffset + PrgRomSize;

    /// <summary>Total number of bytes the header announces, header and trainer included.</summary>
    public int AnnouncedSize => ChrRomOffset + ChrRomSize;

    /// <summary>
    /// Parses the first 16 bytes of <paramref name="image"/>.
    /// </summary>
    /// <returns>
    /// <see langword="true"/> on success. <see langword="false"/> when the image is
    /// shorter than a header or when the <c>NES&lt;EOF&gt;</c> signature is absent
    /// (REQ-CORE-011).
    /// </returns>
    public static bool TryParse(ReadOnlySpan<byte> image, out InesHeader header, out string? error)
    {
        header = null!;

        if (image.Length < Size)
        {
            error = $"Corrupt header: a ROM image needs at least {Size} bytes, got {image.Length}.";
            return false;
        }

        if (!image[..4].SequenceEqual(Signature))
        {
            error = "Corrupt header: missing iNES signature \"NES\\x1A\".";
            return false;
        }

        byte byte4 = image[4];
        byte byte5 = image[5];
        byte byte6 = image[6];
        byte byte7 = image[7];
        byte byte8 = image[8];
        byte byte9 = image[9];
        byte byte11 = image[11];
        byte byte12 = image[12];

        bool isNes20 = (byte7 & 0x0C) == 0x08;

        // A header is "dirty" when it is not NES 2.0 and bytes 12 to 15 carry something
        // else than zero, which old tools used as a free-text comment (REQ-CORE-012).
        bool isDirty = !isNes20 && (image[12] | image[13] | image[14] | image[15]) != 0;

        int mapper;
        if (byte6 == 0x23 && byte7 == 0x64)
        {
            // Compatibility fix of the reference for one mislabelled dump (REQ-CORE-012).
            mapper = 2;
        }
        else
        {
            mapper = byte6 >> 4;
            if (!isDirty)
            {
                mapper |= (byte7 >> 4) << 4;
                if (isNes20)
                {
                    mapper |= (byte8 & 0x0F) << 8;
                }
            }
        }

        int submapper = isNes20 ? byte8 >> 4 : 0;

        int prgRomSize;
        int chrRomSize;
        if (isNes20)
        {
            prgRomSize = Nes20RomSize(byte4, byte9 & 0x0F, 16 * 1024);
            chrRomSize = Nes20RomSize(byte5, byte9 >> 4, 8 * 1024);
        }
        else
        {
            prgRomSize = byte4 * 16 * 1024;
            chrRomSize = byte5 * 8 * 1024;
        }

        int chrRamSize;
        if (isNes20)
        {
            int shift = byte11 & 0x0F;
            chrRamSize = shift == 0 ? 0 : 64 << shift;
        }
        else
        {
            // No CHR-ROM means the cartridge carries 32 KiB of CHR-RAM (REQ-CORE-012).
            chrRamSize = chrRomSize == 0 ? DefaultChrRamSize : 0;
        }

        if (chrRomSize == 0 && chrRamSize == 0)
        {
            chrRamSize = DefaultChrRamSize;
        }

        int prgRamSize = DefaultPrgRamSize;
        if (isNes20)
        {
            int shift = image[10] & 0x0F;
            int eepromShift = image[10] >> 4;
            int volatile_ = shift == 0 ? 0 : 64 << shift;
            int nonVolatile = eepromShift == 0 ? 0 : 64 << eepromShift;
            prgRamSize = Math.Max(volatile_ + nonVolatile, DefaultPrgRamSize);
        }

        NametableMirroring mirroring = (byte6 & 0x08) != 0
            ? NametableMirroring.FourScreen
            : (byte6 & 0x01) != 0
                ? NametableMirroring.Vertical
                : NametableMirroring.Horizontal;

        // REQ-CORE-013. A dirty header is NTSC, bytes 7 to 15 unread.
        NesRegion region;
        if (isDirty)
        {
            region = NesRegion.Ntsc;
        }
        else if (isNes20)
        {
            region = (NesRegion)(byte12 & 0x03);
        }
        else
        {
            region = (byte9 & 0x01) != 0 ? NesRegion.Pal : NesRegion.Ntsc;
        }

        header = new InesHeader
        {
            IsNes20 = isNes20,
            IsDirty = isDirty,
            Mapper = mapper,
            Submapper = submapper,
            PrgRomSize = prgRomSize,
            ChrRomSize = chrRomSize,
            ChrRamSize = chrRamSize,
            PrgRamSize = prgRamSize,
            Mirroring = mirroring,
            HasBattery = (byte6 & 0x02) != 0,
            HasTrainer = (byte6 & 0x04) != 0,
            Region = region,
        };

        error = null;
        return true;
    }

    /// <summary>
    /// NES 2.0 ROM size: the 12-bit value <paramref name="msbNibble"/>:<paramref name="low"/>
    /// in units of <paramref name="unit"/>, or the exponent-multiplier form when the MSB
    /// nibble is 0xF (nesdev, "NES 2.0").
    /// </summary>
    private static int Nes20RomSize(byte low, int msbNibble, int unit)
    {
        if (msbNibble == 0x0F)
        {
            int exponent = low >> 2;
            int multiplier = (low & 0x03) * 2 + 1;
            return multiplier << exponent;
        }

        return ((msbNibble << 8) | low) * unit;
    }
}
