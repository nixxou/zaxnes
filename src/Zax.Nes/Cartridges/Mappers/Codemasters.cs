using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 71, Camerica and Codemasters (REQ-CORE-100).
/// </summary>
/// <remarks>
/// A UxROM-like board: a write to $C000-$FFFF selects the 16 KiB bank at $8000 and the last
/// bank stays at $C000. One title (Fire Hawk) wires a second latch at $9000-$9FFF whose
/// bit 4 picks the single nametable the whole screen shows; the other games leave the
/// header's layout alone (nesdev, "Camerica/Codemasters").
/// </remarks>
internal sealed class Codemasters : BankedCartridge
{
    private byte _bank;
    private bool _singleScreen;
    private bool _singleScreenB;

    internal Codemasters(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 71;

    public override void Reset()
    {
        base.Reset();
        _bank = 0;
        _singleScreen = false;
        _singleScreenB = false;
        UpdateBanks();
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        if ((address & 0xF000) == 0x9000)
        {
            _singleScreen = true;
            _singleScreenB = (value & 0x10) != 0;
        }
        else if (address >= 0xC000)
        {
            _bank = value;
        }
        else
        {
            return;
        }

        UpdateBanks();
    }

    private void UpdateBanks()
    {
        MapPrg16k(0, _bank);
        MapPrg16k(2, -1);
        MapChr8k(0);

        if (_singleScreen)
        {
            Mirroring = _singleScreenB
                ? NametableMirroring.SingleScreenB
                : NametableMirroring.SingleScreenA;
        }
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteByte(_bank);
        writer.WriteBool(_singleScreen);
        writer.WriteBool(_singleScreenB);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        _bank = reader.ReadByte();
        _singleScreen = reader.ReadBool();
        _singleScreenB = reader.ReadBool();
    }
}
