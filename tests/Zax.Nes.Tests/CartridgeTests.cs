using Zax.Nes.Cartridges;
using Zax.Nes.Rom;

namespace Zax.Nes.Tests;

/// <summary>
/// NROM and the mapper abstraction L1d extends (REQ-CORE-012, REQ-CORE-100,
/// REQ-CORE-102).
/// </summary>
public sealed class CartridgeTests
{
    [Fact]
    public void REQ_CORE_100_mapper_0_is_implemented()
    {
        Assert.True(MapperRegistry.IsSupported(0));

        byte[] image = TestRoms.Minimal();
        Assert.True(RomImage.TryLoad(image, "Minimal.nes", out RomImage? rom, out string? error), error);
        Assert.True(MapperRegistry.TryCreate(rom!, out Cartridge? cartridge, out string? mapperError), mapperError);

        Assert.Equal(0, cartridge!.MapperNumber);
        Assert.Equal(NametableMirroring.Vertical, cartridge.Mirroring);
    }

    [Theory]
    [InlineData(6)]
    [InlineData(19)]
    [InlineData(69)]
    [InlineData(200)]
    public void REQ_CORE_102_an_unimplemented_mapper_is_refused_by_name(int mapper)
    {
        // Everything outside the REQ-CORE-100 table is refused with the message of
        // REQ-CORE-011.
        Assert.False(MapperRegistry.IsSupported(mapper));

        byte[] image = TestRoms.Minimal();
        image[6] = (byte)((image[6] & 0x0F) | ((mapper & 0x0F) << 4));
        image[7] = (byte)((mapper >> 4) << 4);

        Assert.True(RomImage.TryLoad(image, "X.nes", out RomImage? rom, out string? error), error);
        Assert.Equal(mapper, rom!.Mapper);

        Assert.False(MapperRegistry.TryCreate(rom, out Cartridge? cartridge, out string? mapperError));
        Assert.Null(cartridge);
        Assert.Contains(mapper.ToString(System.Globalization.CultureInfo.InvariantCulture), mapperError);
    }

    [Fact]
    public void REQ_CORE_100_nrom_128_mirrors_its_16_kib_of_prg_at_8000_and_c000()
    {
        // One 16 KiB PRG bank: $8000-$BFFF and $C000-$FFFF show the same bytes.
        var image = new byte[16 + (16 * 1024) + (8 * 1024)];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 0x01; // 1 x 16 KiB PRG.
        image[5] = 0x01; // 1 x 8 KiB CHR.
        image[6] = 0x01;

        image[16] = 0x4C; // A marker at the start of the bank.
        image[16 + (16 * 1024) - 1] = 0x5A; // And one at its end.

        var core = new NesCore();
        Assert.True(core.LoadRom(image, "Nrom128.nes").Success);

        Assert.Equal(0x4C, core.ReadMemory(0x8000));
        Assert.Equal(0x4C, core.ReadMemory(0xC000));
        Assert.Equal(0x5A, core.ReadMemory(0xBFFF));
        Assert.Equal(0x5A, core.ReadMemory(0xFFFF));
    }

    [Fact]
    public void REQ_CORE_100_nrom_256_maps_32_kib_linearly()
    {
        byte[] image = TestRoms.Minimal();
        image[16] = 0x11; // $8000.
        image[16 + (16 * 1024)] = 0x22; // $C000.

        var core = new NesCore();
        Assert.True(core.LoadRom(image, "Nrom256.nes").Success);

        Assert.Equal(0x11, core.ReadMemory(0x8000));
        Assert.Equal(0x22, core.ReadMemory(0xC000));
    }

    [Fact]
    public void REQ_CORE_012_chr_rom_is_readable_and_not_writable()
    {
        byte[] image = TestRoms.Minimal();
        var core = new NesCore();
        Assert.True(core.LoadRom(image, "Chr.nes").Success);

        Cartridge cartridge = core.Cartridge!;

        // Tile 3 of table $0000: plane 0 is eight bytes of 3, plane 1 eight of 252.
        Assert.Equal(0x03, cartridge.PpuPeek(3 * 16));
        Assert.Equal(252, cartridge.PpuPeek((ushort)((3 * 16) + 8)));

        cartridge.PpuWrite(3 * 16, 0xFF);
        Assert.Equal(0x03, cartridge.PpuPeek(3 * 16));
    }

    [Fact]
    public void REQ_CORE_012_chr_ram_is_writable()
    {
        byte[] minimal = TestRoms.Minimal();
        var image = new byte[16 + (32 * 1024)];
        minimal.AsSpan(0, 16 + (32 * 1024)).CopyTo(image);
        image[5] = 0x00; // No CHR-ROM: 32 KiB of CHR-RAM.

        var core = new NesCore();
        Assert.True(core.LoadRom(image, "ChrRam.nes").Success);

        Cartridge cartridge = core.Cartridge!;
        cartridge.PpuWrite(0x0123, 0xA5);

        Assert.Equal(0xA5, cartridge.PpuPeek(0x0123));
    }

    [Fact]
    public void REQ_CORE_033_ppu_peek_never_touches_the_mapper_state()
    {
        // NROM has no latch or IRQ counter, so this only pins the contract the L1d mappers
        // must honour: PpuPeek is what the tile emitter uses and it must be inert.
        byte[] image = TestRoms.Minimal();
        var core = new NesCore();
        Assert.True(core.LoadRom(image, "Chr.nes").Success);

        Cartridge cartridge = core.Cartridge!;
        NametableMirroring before = cartridge.Mirroring;

        for (int i = 0; i < 0x2000; i++)
        {
            cartridge.PpuPeek((ushort)i);
        }

        Assert.Equal(before, cartridge.Mirroring);
        Assert.False(cartridge.IrqAsserted);
    }

    [Theory]
    [InlineData(NametableMirroring.Horizontal, 0x2000, 0x2400)]
    [InlineData(NametableMirroring.Horizontal, 0x2800, 0x2C00)]
    [InlineData(NametableMirroring.Vertical, 0x2000, 0x2800)]
    [InlineData(NametableMirroring.Vertical, 0x2400, 0x2C00)]
    public void REQ_CORE_038_nametable_mirroring_pairs_the_right_tables(
        NametableMirroring mirroring, int first, int second)
    {
        byte headerByte6 = mirroring == NametableMirroring.Vertical ? (byte)0x01 : (byte)0x00;

        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(headerByte6), "Mirror.nes").Success);

        Assert.Equal(mirroring, core.Ppu.Mirroring);
        Assert.Equal(core.Ppu.MapNametable((ushort)first), core.Ppu.MapNametable((ushort)second));

        // And the other pair must land somewhere else.
        int other = mirroring == NametableMirroring.Vertical ? 0x2400 : 0x2800;
        if (first != other)
        {
            Assert.NotEqual(core.Ppu.MapNametable((ushort)first), core.Ppu.MapNametable((ushort)other));
        }
    }

    [Fact]
    public void REQ_CORE_012_four_screen_gives_four_distinct_nametables()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(0x08), "FourScreen.nes").Success);

        Assert.Equal(NametableMirroring.FourScreen, core.Ppu.Mirroring);

        int[] offsets =
        [
            core.Ppu.MapNametable(0x2000),
            core.Ppu.MapNametable(0x2400),
            core.Ppu.MapNametable(0x2800),
            core.Ppu.MapNametable(0x2C00),
        ];

        Assert.Equal(4, offsets.Distinct().Count());
        Assert.Equal<int[]>([0, 1024, 2048, 3072], offsets);
    }
}
