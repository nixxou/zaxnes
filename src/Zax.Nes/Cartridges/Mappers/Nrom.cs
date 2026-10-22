using Zax.Nes.Rom;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 0, NROM (REQ-CORE-100).
/// </summary>
/// <remarks>
/// No banking at all: 16 or 32 KiB of PRG-ROM at $8000-$FFFF, 8 KiB of CHR at
/// $0000-$1FFF, nametable layout fixed by the header. An NROM-128 board (16 KiB of PRG)
/// shows its PRG twice, at $8000 and at $C000 (nesdev, "NROM").
/// </remarks>
internal sealed class Nrom : Cartridge
{
    internal Nrom(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 0;

    protected override bool TryReadPrg(ushort address, out byte value)
    {
        if (address < 0x8000)
        {
            value = 0;
            return false;
        }

        ReadOnlySpan<byte> prg = Rom.PrgRom;
        value = prg[(address - 0x8000) % prg.Length];
        return true;
    }

    public override byte PpuRead(ushort address, ChrFetch fetch) => ReadChrDirect(address & 0x1FFF);

    public override void PpuWrite(ushort address, byte value) => WriteChrDirect(address & 0x1FFF, value);
}
