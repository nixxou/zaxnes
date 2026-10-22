using Zax.Nes.Rom;

namespace Zax.Nes.Tests;

/// <summary>
/// Builds synthetic cartridges for the mappers of REQ-CORE-100 and reads their bank windows
/// back.
/// </summary>
/// <remarks>
/// <para>
/// Every 8 KiB PRG bank is filled with its own bank number and every 1 KiB CHR bank with
/// its own, so one read anywhere in a window says which bank the mapper put there: that is
/// what a "bank switching vector" means in the tests below. The last six bytes of the image
/// still carry the three vectors, pointing at an endless loop at the start of the last bank,
/// so the machine can be powered on and frames can be run.
/// </para>
/// <para>
/// The public test ROMs cover ten of the required mappers (REQ-CORE-100 acceptance
/// criterion); for the others, these synthetic boards are what pins the documented register
/// behaviour down.
/// </para>
/// </remarks>
internal static class MapperFixture
{
    /// <summary>
    /// Builds a cartridge image for <paramref name="mapper"/>.
    /// </summary>
    /// <param name="mapper">iNES mapper number.</param>
    /// <param name="prgBanks8k">Number of 8 KiB PRG banks, 2 to 128.</param>
    /// <param name="chrBanks1k">Number of 1 KiB CHR banks; 0 asks for CHR-RAM.</param>
    /// <param name="mirroring">
    /// Byte 6 mirroring bits: 0 horizontal, 1 vertical, 8 four-screen.
    /// </param>
    /// <param name="battery">Sets bit 1 of byte 6, the battery-backed PRG-RAM flag.</param>
    /// <param name="submapper">NES 2.0 submapper, or -1 for a plain iNES header.</param>
    public static byte[] Image(
        int mapper,
        int prgBanks8k = 8,
        int chrBanks1k = 16,
        int mirroring = 0,
        bool battery = false,
        int submapper = -1)
    {
        int prgSize = prgBanks8k * 8 * 1024;
        int chrSize = chrBanks1k * 1024;
        var image = new byte[16 + prgSize + chrSize];

        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = (byte)(prgSize / (16 * 1024));
        image[5] = (byte)(chrSize / (8 * 1024));
        image[6] = (byte)(((mapper & 0x0F) << 4) | mirroring | (battery ? 0x02 : 0x00));
        image[7] = (byte)(mapper & 0xF0);

        if (submapper >= 0)
        {
            image[7] |= 0x08; // NES 2.0.
            image[8] = (byte)((submapper << 4) | ((mapper >> 8) & 0x0F));
        }
        else
        {
            image[7] |= (byte)((mapper >> 8) != 0 ? 0x08 : 0x00);
            if ((mapper >> 8) != 0)
            {
                image[8] = (byte)((mapper >> 8) & 0x0F);
            }
        }

        for (int bank = 0; bank < prgBanks8k; bank++)
        {
            image.AsSpan(16 + (bank * 8 * 1024), 8 * 1024).Fill((byte)bank);
        }

        for (int bank = 0; bank < chrBanks1k; bank++)
        {
            image.AsSpan(16 + prgSize + (bank * 1024), 1024).Fill((byte)bank);
        }

        // An endless loop at the start of the last PRG bank, and the three vectors pointing
        // at it, so that the CPU has somewhere to go.
        int last = 16 + prgSize - (8 * 1024);
        image[last] = 0x4C;
        image[last + 1] = 0x00;
        image[last + 2] = 0xE0;

        int vectors = 16 + prgSize - 6;
        for (int i = 0; i < 3; i++)
        {
            image[vectors + (2 * i)] = 0x00;
            image[vectors + (2 * i) + 1] = 0xE0;
        }

        return image;
    }

    /// <summary>Loads a synthetic cartridge into a fresh core, asserting the load succeeds.</summary>
    public static NesCore Load(
        int mapper,
        int prgBanks8k = 8,
        int chrBanks1k = 16,
        int mirroring = 0,
        bool battery = false,
        int submapper = -1)
    {
        byte[] image = Image(mapper, prgBanks8k, chrBanks1k, mirroring, battery, submapper);
        var core = new NesCore();
        RomLoadResult load = core.LoadRom(image, $"Mapper{mapper}.nes");
        Assert.True(load.Success, load.Error);
        Assert.Equal(mapper, core.Cartridge!.MapperNumber);
        return core;
    }

    /// <summary>The PRG bank number the mapper currently shows at <paramref name="address"/>.</summary>
    public static byte PrgBank(this NesCore core, int address) => core.ReadMemory((ushort)address);

    /// <summary>The CHR bank number the mapper currently shows at <paramref name="address"/>.</summary>
    public static byte ChrBank(this NesCore core, int address)
        => core.Cartridge!.PpuPeek((ushort)address);

    /// <summary>
    /// The four PRG bank numbers at $8000, $A000, $C000 and $E000, read 256 bytes into each
    /// window so that the endless loop and the vectors planted in the last bank cannot be
    /// mistaken for a bank number.
    /// </summary>
    public static byte[] PrgWindows(this NesCore core)
        => [core.PrgBank(0x8100), core.PrgBank(0xA100), core.PrgBank(0xC100), core.PrgBank(0xE100)];

    /// <summary>The eight CHR bank numbers at $0000, $0400 … $1C00.</summary>
    public static byte[] ChrWindows(this NesCore core)
    {
        var banks = new byte[8];
        for (int slot = 0; slot < 8; slot++)
        {
            banks[slot] = core.ChrBank(slot * 0x0400);
        }

        return banks;
    }

    /// <summary>
    /// Writes a mapper register, leaving two CPU cycles of room before the next write.
    /// </summary>
    /// <remarks>
    /// MMC1 drops a write made on the cycle right after another one, so the serial register
    /// can only be loaded by writes that are spaced out; the bus clock has to move between
    /// them, which the host memory write of REQ-CORE-082 does not do by itself.
    /// </remarks>
    public static void MapperWrite(this NesCore core, int address, byte value)
    {
        core.Bus.Tick();
        core.Bus.Tick();
        core.WriteMemory((ushort)address, value);
    }

    /// <summary>Loads an MMC1 register through its five-bit serial port.</summary>
    public static void Mmc1Write(this NesCore core, int address, byte value)
    {
        for (int bit = 0; bit < 5; bit++)
        {
            core.MapperWrite(address, (byte)((value >> bit) & 0x01));
        }
    }

    /// <summary>Writes an MMC3-style bank register pair ($8000 selects, $8001 loads).</summary>
    public static void Mmc3Bank(this NesCore core, int register, byte value)
    {
        core.MapperWrite(0x8000, (byte)register);
        core.MapperWrite(0x8001, value);
    }

    /// <summary>
    /// Saves the state, runs <paramref name="frames"/> frames, restores and returns whether
    /// the state is byte-identical to a second save (REQ-CORE-090, REQ-CORE-092).
    /// </summary>
    public static void AssertStateRoundTrips(this NesCore core, int frames = 3)
    {
        byte[] before = core.SaveState();

        for (int frame = 0; frame < frames; frame++)
        {
            core.RunFrame(0);
        }

        Assert.True(
            core.TryLoadState(before, 0, out int consumed, out string name, out string? error),
            error);
        Assert.Equal(before.Length, consumed);
        Assert.False(string.IsNullOrEmpty(name));
        Assert.Equal(before, core.SaveState());
    }
}
