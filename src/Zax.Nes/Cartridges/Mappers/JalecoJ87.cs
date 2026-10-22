using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mappers 87 and 101, Jaleco CHR-switching boards (REQ-CORE-100, AMD-021 R2-003).
/// </summary>
/// <remarks>
/// <para>
/// PRG is fixed; the single register is write-only, so neither board has PRG-RAM in the
/// window it sits in.
/// </para>
/// <para>
/// On J87 (mapper 87) the register lives in $6000-$7FFF and the two bits reaching the CHR
/// line are swapped — bank = bit 0 of the value times two plus bit 1 — which is the whole
/// point of the number (nesdev, "INES Mapper 087").
/// </para>
/// <para>
/// On mapper 101 the register answers at every address whose bits 13 and 14 are both set,
/// that is $6000-$7FFF <strong>and</strong> $E000-$FFFF, and selects the 8 KiB CHR bank
/// <c>value &amp; 3</c> — two bits, unswapped (R2-003).
/// </para>
/// </remarks>
internal sealed class JalecoJ87 : BankedCartridge
{
    private readonly bool _swapBits;
    private byte _register;

    internal JalecoJ87(RomImage rom, int mapperNumber)
        : base(rom)
    {
        MapperNumber = mapperNumber;
        _swapBits = mapperNumber == 87;
    }

    public override int MapperNumber { get; }

    protected override bool PrgRamEnabled => false;

    public override void Reset()
    {
        base.Reset();
        _register = 0;
        UpdateBanks();
    }

    public override void CpuWrite(ushort address, byte value)
    {
        // Mapper 87 decodes $6000-$7FFF only; mapper 101 decodes bits 13 and 14 of the
        // address, which adds $E000-$FFFF (R2-003).
        bool selected = _swapBits
            ? address is >= 0x6000 and <= 0x7FFF
            : (address & 0x6000) == 0x6000;

        if (selected)
        {
            _register = value;
            UpdateBanks();
            return;
        }

        base.CpuWrite(address, value);
    }

    private void UpdateBanks()
    {
        MapPrg32k(0);
        MapChr8k(_swapBits
            ? ((_register & 0x01) << 1) | ((_register >> 1) & 0x01)
            : _register & 0x03);
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteByte(_register);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        _register = reader.ReadByte();
    }
}
