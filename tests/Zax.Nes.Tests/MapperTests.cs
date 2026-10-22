using Zax.Nes.Cartridges;

namespace Zax.Nes.Tests;

/// <summary>
/// Bank switching, mirroring control, PRG-RAM and savestate round-trip for every mapper of
/// REQ-CORE-100, on the synthetic boards of <see cref="MapperFixture"/>.
/// </summary>
/// <remarks>
/// The register behaviour comes from the nesdev mapper pages. The IRQ counters live in
/// <see cref="MapperIrqTests"/> and the public ROM results in <see cref="MapperRomTests"/>.
/// </remarks>
public sealed class MapperTests
{
    [Fact]
    public void REQ_CORE_100_every_required_mapper_is_registered()
    {
        int[] required =
        [
            0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 13, 15, 16, 18, 21, 22, 23, 25, 27, 28, 30, 32,
            34, 41, 64, 66, 71, 87, 101, 225, 228,
        ];

        Assert.Equal(required, NesCore.SupportedMappers);
    }

    [Theory]
    [InlineData(6)]
    [InlineData(19)]
    [InlineData(69)]
    [InlineData(116)]
    [InlineData(200)]
    public void REQ_CORE_102_any_other_mapper_is_refused(int mapper)
    {
        // 116 is the one REQ-CORE-101 leaves optional: not implemented, so refused with the
        // message of REQ-CORE-011 like any other number outside the table.
        Assert.False(MapperRegistry.IsSupported(mapper));

        var core = new NesCore();
        Rom.RomLoadResult load = core.LoadRom(MapperFixture.Image(mapper), "Other.nes");

        Assert.False(load.Success);
        Assert.Contains(
            mapper.ToString(System.Globalization.CultureInfo.InvariantCulture),
            load.Error!,
            StringComparison.Ordinal);
        Assert.False(core.IsRunning);
    }

    // ---------------------------------------------------------------- MMC1 (1)

    [Fact]
    public void REQ_CORE_100_mmc1_powers_on_with_the_last_bank_at_c000()
    {
        NesCore core = MapperFixture.Load(1, prgBanks8k: 16);
        Assert.Equal<byte[]>([0, 1, 14, 15], core.PrgWindows());
    }

    [Fact]
    public void REQ_CORE_100_mmc1_switches_16_kib_at_8000_in_prg_mode_3()
    {
        NesCore core = MapperFixture.Load(1, prgBanks8k: 16);
        core.Mmc1Write(0x8000, 0x0F); // Control: PRG mode 3, 4 KiB CHR, horizontal.
        core.Mmc1Write(0xE000, 0x03);

        Assert.Equal<byte[]>([6, 7, 14, 15], core.PrgWindows());
        Assert.Equal(NametableMirroring.Horizontal, core.Ppu.Mirroring);
    }

    [Fact]
    public void REQ_CORE_100_mmc1_fixes_the_first_bank_at_8000_in_prg_mode_2()
    {
        NesCore core = MapperFixture.Load(1, prgBanks8k: 16);
        core.Mmc1Write(0x8000, 0x0A); // PRG mode 2, 8 KiB CHR, vertical.
        core.Mmc1Write(0xE000, 0x05);

        Assert.Equal<byte[]>([0, 1, 10, 11], core.PrgWindows());
        Assert.Equal(NametableMirroring.Vertical, core.Ppu.Mirroring);
    }

    [Fact]
    public void REQ_CORE_100_mmc1_switches_32_kib_in_prg_modes_0_and_1()
    {
        NesCore core = MapperFixture.Load(1, prgBanks8k: 16);
        core.Mmc1Write(0x8000, 0x02); // PRG mode 0, 8 KiB CHR, vertical.
        core.Mmc1Write(0xE000, 0x07); // The low bit is ignored: 16 KiB bank 6, not 7.

        Assert.Equal<byte[]>([12, 13, 14, 15], core.PrgWindows());
    }

    [Fact]
    public void REQ_CORE_100_mmc1_switches_chr_in_8_and_4_kib_units()
    {
        NesCore core = MapperFixture.Load(1, prgBanks8k: 8, chrBanks1k: 32);

        core.Mmc1Write(0x8000, 0x0C); // 8 KiB CHR.
        core.Mmc1Write(0xA000, 0x03); // Bank 3 of 4 KiB units: the low bit is ignored.
        Assert.Equal<byte[]>([8, 9, 10, 11, 12, 13, 14, 15], core.ChrWindows());

        core.Mmc1Write(0x8000, 0x1C); // 4 KiB CHR.
        core.Mmc1Write(0xA000, 0x03);
        core.Mmc1Write(0xC000, 0x06);
        Assert.Equal<byte[]>([12, 13, 14, 15, 24, 25, 26, 27], core.ChrWindows());
    }

    [Fact]
    public void REQ_CORE_100_mmc1_takes_the_512_kib_bank_bit_from_the_chr_register()
    {
        // SUROM: the PRG register only reaches 256 KiB and bit 4 of the CHR register is A18,
        // with the fixed window following it into the same half.
        NesCore core = MapperFixture.Load(1, prgBanks8k: 64);

        core.Mmc1Write(0x8000, 0x0C);
        core.Mmc1Write(0xE000, 0x02);
        Assert.Equal<byte[]>([4, 5, 30, 31], core.PrgWindows());

        core.Mmc1Write(0xA000, 0x10);
        Assert.Equal<byte[]>([36, 37, 62, 63], core.PrgWindows());
    }

    [Fact]
    public void REQ_CORE_100_mmc1_ignores_a_write_on_the_cycle_after_another()
    {
        // A read-modify-write instruction writes twice in a row; the chip only takes the
        // first (nesdev, "MMC1").
        NesCore core = MapperFixture.Load(1, prgBanks8k: 16);
        core.Mmc1Write(0x8000, 0x0F);

        // Five accepted writes load the register; interleaving an immediate second write per
        // bit must not change the result.
        for (int bit = 0; bit < 5; bit++)
        {
            core.MapperWrite(0xE000, (byte)((0x03 >> bit) & 0x01));
            core.WriteMemory(0xE000, 0xFF); // Same cycle as the write above: dropped.
        }

        Assert.Equal<byte[]>([6, 7, 14, 15], core.PrgWindows());
    }

    [Fact]
    public void REQ_CORE_100_mmc1_disables_prg_ram_with_bit_4_of_the_prg_register()
    {
        NesCore core = MapperFixture.Load(1, prgBanks8k: 16, battery: true);

        core.WriteMemory(0x6000, 0x5A);
        Assert.Equal(0x5A, core.ReadMemory(0x6000));

        core.Mmc1Write(0xE000, 0x10);
        core.WriteMemory(0x6000, 0xA5);
        Assert.Equal(0, core.ReadMemory(0x6000));

        core.Mmc1Write(0xE000, 0x00);
        Assert.Equal(0x5A, core.ReadMemory(0x6000));
    }

    // ---------------------------------------------------------------- UxROM, CNROM, AxROM

    [Fact]
    public void REQ_CORE_100_uxrom_switches_16_kib_and_fixes_the_last_bank()
    {
        NesCore core = MapperFixture.Load(2, prgBanks8k: 16, chrBanks1k: 8);
        Assert.Equal<byte[]>([0, 1, 14, 15], core.PrgWindows());

        core.WriteMemory(0x8000, 0x05);
        Assert.Equal<byte[]>([10, 11, 14, 15], core.PrgWindows());
    }

    [Fact]
    public void REQ_CORE_100_cnrom_switches_8_kib_of_chr()
    {
        NesCore core = MapperFixture.Load(3, prgBanks8k: 4, chrBanks1k: 32);
        Assert.Equal<byte[]>([0, 1, 2, 3], core.PrgWindows());

        core.WriteMemory(0x9234, 0x02);
        Assert.Equal<byte[]>([16, 17, 18, 19, 20, 21, 22, 23], core.ChrWindows());
    }

    [Fact]
    public void REQ_CORE_100_axrom_switches_32_kib_and_selects_the_single_nametable()
    {
        NesCore core = MapperFixture.Load(7, prgBanks8k: 16, chrBanks1k: 0);

        core.WriteMemory(0x8000, 0x02);
        Assert.Equal<byte[]>([8, 9, 10, 11], core.PrgWindows());
        Assert.Equal(NametableMirroring.SingleScreenA, core.Ppu.Mirroring);

        core.WriteMemory(0x8000, 0x12);
        Assert.Equal<byte[]>([8, 9, 10, 11], core.PrgWindows());
        Assert.Equal(NametableMirroring.SingleScreenB, core.Ppu.Mirroring);
        Assert.Equal(core.Ppu.MapNametable(0x2000) + 1024, core.Ppu.MapNametable(0x2400) + 1024);
    }

    // ---------------------------------------------------------------- MMC3 (4)

    [Fact]
    public void REQ_CORE_100_mmc3_maps_two_switchable_prg_banks_and_swaps_them()
    {
        NesCore core = MapperFixture.Load(4, prgBanks8k: 16);

        core.Mmc3Bank(6, 3);
        core.Mmc3Bank(7, 5);
        Assert.Equal<byte[]>([3, 5, 14, 15], core.PrgWindows());

        core.MapperWrite(0x8000, 0x46); // Bit 6: the switchable window moves to $C000.
        Assert.Equal<byte[]>([14, 5, 3, 15], core.PrgWindows());
    }

    [Fact]
    public void REQ_CORE_100_mmc3_maps_two_2_kib_and_four_1_kib_chr_banks_and_inverts_them()
    {
        NesCore core = MapperFixture.Load(4, chrBanks1k: 32);

        core.Mmc3Bank(0, 2); // 2 KiB window: the low bit of the register is ignored.
        core.Mmc3Bank(1, 8);
        core.Mmc3Bank(2, 20);
        core.Mmc3Bank(3, 21);
        core.Mmc3Bank(4, 22);
        core.Mmc3Bank(5, 23);
        Assert.Equal<byte[]>([2, 3, 8, 9, 20, 21, 22, 23], core.ChrWindows());

        core.MapperWrite(0x8000, 0x80); // Bit 7 swaps the halves.
        Assert.Equal<byte[]>([20, 21, 22, 23, 2, 3, 8, 9], core.ChrWindows());
    }

    [Fact]
    public void REQ_CORE_100_mmc3_controls_mirroring_and_prg_ram_protection()
    {
        NesCore core = MapperFixture.Load(4, battery: true);

        core.MapperWrite(0xA000, 0x00);
        Assert.Equal(NametableMirroring.Vertical, core.Ppu.Mirroring);
        core.MapperWrite(0xA000, 0x01);
        Assert.Equal(NametableMirroring.Horizontal, core.Ppu.Mirroring);

        core.WriteMemory(0x6100, 0x11);
        Assert.Equal(0x11, core.ReadMemory(0x6100));

        core.MapperWrite(0xA001, 0xC0); // Enabled but write protected.
        core.WriteMemory(0x6100, 0x22);
        Assert.Equal(0x11, core.ReadMemory(0x6100));

        core.MapperWrite(0xA001, 0x00); // Chip disabled: reads stop too.
        Assert.Equal(0, core.ReadMemory(0x6100));
    }

    [Fact]
    public void REQ_CORE_100_mmc3_leaves_a_four_screen_board_alone()
    {
        NesCore core = MapperFixture.Load(4, mirroring: 0x08);

        core.MapperWrite(0xA000, 0x01);
        Assert.Equal(NametableMirroring.FourScreen, core.Ppu.Mirroring);
    }

    [Fact]
    public void REQ_CORE_100_mmc6_puts_one_kib_of_ram_at_7000()
    {
        // NES 2.0 submapper 1 of mapper 4: the 1 KiB window is enabled by bit 5 of the bank
        // select and each half has its own read and write enable in $A001.
        NesCore core = MapperFixture.Load(4, submapper: 1);

        core.WriteMemory(0x7000, 0x33);
        Assert.Equal(0, core.ReadMemory(0x7000));

        core.MapperWrite(0x8000, 0x20); // RAM enabled.
        core.MapperWrite(0xA001, 0x30); // First half readable and writable.
        core.WriteMemory(0x7000, 0x33);
        Assert.Equal(0x33, core.ReadMemory(0x7000));

        // The second half is still disabled, and the 1 KiB is mirrored to the end of the
        // window.
        core.WriteMemory(0x7200, 0x44);
        Assert.Equal(0, core.ReadMemory(0x7200));
        Assert.Equal(0x33, core.ReadMemory(0x7400));
    }

    // ---------------------------------------------------------------- MMC5 (5)

    [Theory]
    [InlineData(0, new byte[] { 12, 13, 14, 15 })]
    [InlineData(1, new byte[] { 4, 5, 14, 15 })]
    [InlineData(2, new byte[] { 4, 5, 9, 15 })]
    [InlineData(3, new byte[] { 2, 5, 9, 15 })]
    public void REQ_CORE_100_mmc5_has_four_prg_modes(int mode, byte[] expected)
    {
        NesCore core = MapperFixture.Load(5, prgBanks8k: 16);

        core.WriteMemory(0x5114, 0x82); // $8000: ROM bank 2.
        core.WriteMemory(0x5115, 0x85); // ROM bank 5, or the 16 KiB pair 4-5.
        core.WriteMemory(0x5116, 0x89); // $C000: ROM bank 9.
        core.WriteMemory(0x5117, 0x0F); // $E000 is always ROM: bank 15.
        core.WriteMemory(0x5100, (byte)mode);

        Assert.Equal(expected, core.PrgWindows());
    }

    [Fact]
    public void REQ_CORE_100_mmc5_can_show_prg_ram_in_a_prg_window()
    {
        NesCore core = MapperFixture.Load(5, prgBanks8k: 16);

        core.WriteMemory(0x5100, 0x03); // 8 KiB windows.
        core.WriteMemory(0x5102, 0x02); // Unlock the RAM.
        core.WriteMemory(0x5103, 0x01);
        core.WriteMemory(0x5114, 0x00); // $8000 shows PRG-RAM bank 0 (bit 7 clear).

        core.WriteMemory(0x8100, 0x77);
        Assert.Equal(0x77, core.ReadMemory(0x8100));

        core.WriteMemory(0x5114, 0x80); // Back to ROM bank 0.
        Assert.Equal(0, core.ReadMemory(0x8100));
    }

    [Fact]
    public void REQ_CORE_100_mmc5_banks_the_prg_ram_window_at_6000()
    {
        NesCore core = MapperFixture.Load(5, battery: true);

        core.WriteMemory(0x5102, 0x02);
        core.WriteMemory(0x5103, 0x01);

        core.WriteMemory(0x5113, 0x00);
        core.WriteMemory(0x6100, 0xA1);
        core.WriteMemory(0x5113, 0x03);
        core.WriteMemory(0x6100, 0xB2);

        Assert.Equal(0xB2, core.ReadMemory(0x6100));
        core.WriteMemory(0x5113, 0x00);
        Assert.Equal(0xA1, core.ReadMemory(0x6100));
    }

    [Fact]
    public void REQ_CORE_100_mmc5_write_protects_prg_ram_until_both_values_are_written()
    {
        NesCore core = MapperFixture.Load(5);

        core.WriteMemory(0x6100, 0x5A);
        Assert.Equal(0, core.ReadMemory(0x6100));

        core.WriteMemory(0x5102, 0x02);
        core.WriteMemory(0x6100, 0x5A);
        Assert.Equal(0, core.ReadMemory(0x6100));

        core.WriteMemory(0x5103, 0x01);
        core.WriteMemory(0x6100, 0x5A);
        Assert.Equal(0x5A, core.ReadMemory(0x6100));
    }

    [Theory]
    [InlineData(3, new byte[] { 0, 1, 2, 3, 4, 5, 6, 7 })]
    [InlineData(2, new byte[] { 2, 3, 6, 7, 10, 11, 14, 15 })]
    [InlineData(1, new byte[] { 12, 13, 14, 15, 28, 29, 30, 31 })]
    [InlineData(0, new byte[] { 56, 57, 58, 59, 60, 61, 62, 63 })]
    public void REQ_CORE_100_mmc5_has_four_chr_modes(int mode, byte[] expected)
    {
        NesCore core = MapperFixture.Load(5, chrBanks1k: 64);

        for (int register = 0; register < 8; register++)
        {
            core.WriteMemory((ushort)(0x5120 + register), (byte)register);
        }

        core.WriteMemory(0x5101, (byte)mode);
        Assert.Equal(expected, core.ChrWindows());
    }

    [Fact]
    public void REQ_CORE_100_mmc5_serves_background_fetches_from_the_second_set_with_tall_sprites()
    {
        NesCore core = MapperFixture.Load(5, chrBanks1k: 64);

        core.WriteMemory(0x5101, 0x03); // 1 KiB windows.
        for (int register = 0; register < 8; register++)
        {
            core.WriteMemory((ushort)(0x5120 + register), (byte)register);
        }

        for (int register = 0; register < 4; register++)
        {
            core.WriteMemory((ushort)(0x5128 + register), (byte)(16 + register));
        }

        // 8x8 sprites: the set written last serves everything.
        core.WriteMemory(0x2000, 0x00);
        Assert.Equal(16, core.Cartridge!.PpuPeek(0x0000, ChrFetch.Background));
        Assert.Equal(16, core.Cartridge.PpuPeek(0x0000, ChrFetch.Sprite));

        // 8x16 sprites: background from $5128-$512B, sprites from $5120-$5127.
        core.WriteMemory(0x2000, 0x20);
        Assert.Equal(16, core.Cartridge.PpuPeek(0x0000, ChrFetch.Background));
        Assert.Equal(0, core.Cartridge.PpuPeek(0x0000, ChrFetch.Sprite));
        Assert.Equal(4, core.Cartridge.PpuPeek(0x1000, ChrFetch.Sprite));
    }

    [Fact]
    public void REQ_CORE_100_mmc5_maps_each_nametable_to_vram_exram_or_fill_mode()
    {
        NesCore core = MapperFixture.Load(5);

        core.WriteMemory(0x5104, 0x02); // ExRAM in RAM mode, so the CPU can fill it.
        core.WriteMemory(0x5C00, 0x6B);
        core.WriteMemory(0x5106, 0x5C); // Fill tile.
        core.WriteMemory(0x5107, 0x02); // Fill attribute.

        // $2000 -> VRAM page 0, $2400 -> page 1, $2800 -> ExRAM, $2C00 -> fill mode.
        core.WriteMemory(0x5105, 0xE4);
        core.WriteMemory(0x5104, 0x00); // ExRAM as a nametable.

        Assert.Equal(0, core.Ppu.MapNametable(0x2000));
        Assert.Equal(0x6B, core.Ppu.ReadNametableByte(0x2800));
        Assert.Equal(0x5C, core.Ppu.ReadNametableByte(0x2C00));
        Assert.Equal(0x02 * 0x55, core.Ppu.ReadNametableByte(0x2FC0));
    }

    [Fact]
    public void REQ_CORE_100_mmc5_multiplies_two_bytes()
    {
        NesCore core = MapperFixture.Load(5);

        core.WriteMemory(0x5205, 0x12);
        core.WriteMemory(0x5206, 0x34);

        Assert.Equal(0x12 * 0x34 & 0xFF, core.ReadMemory(0x5205));
        Assert.Equal((0x12 * 0x34) >> 8, core.ReadMemory(0x5206));
    }

    // ---------------------------------------------------------------- MMC2 and MMC4 (9, 10)

    [Fact]
    public void REQ_CORE_100_mmc2_switches_8_kib_and_fixes_the_last_three_banks()
    {
        NesCore core = MapperFixture.Load(9, prgBanks8k: 8, chrBanks1k: 32);
        core.MapperWrite(0xA000, 0x02);

        Assert.Equal<byte[]>([2, 5, 6, 7], core.PrgWindows());
    }

    [Fact]
    public void REQ_CORE_100_mmc4_switches_16_kib_and_fixes_the_last_bank()
    {
        NesCore core = MapperFixture.Load(10, prgBanks8k: 8, chrBanks1k: 32);
        core.MapperWrite(0xA000, 0x02);

        Assert.Equal<byte[]>([4, 5, 6, 7], core.PrgWindows());
    }

    [Theory]
    [InlineData(9)]
    [InlineData(10)]
    public void REQ_CORE_039_the_fd_and_fe_latches_pick_the_chr_bank(int mapper)
    {
        NesCore core = MapperFixture.Load(mapper, chrBanks1k: 64);

        core.MapperWrite(0xB000, 0x01); // $0000 with latch $FD: 4 KiB bank 1.
        core.MapperWrite(0xC000, 0x02); // $0000 with latch $FE: bank 2.
        core.MapperWrite(0xD000, 0x03); // $1000 with latch $FD: bank 3.
        core.MapperWrite(0xE000, 0x04); // $1000 with latch $FE: bank 4.

        Cartridge cartridge = core.Cartridge!;

        // Power-on selects the $FE registers. Reading the pattern of tile $FD moves the low
        // latch, but the byte served is still the one of the bank that was in force: the 1 KiB
        // fill value identifies the bank, so $0FD8 in 4 KiB bank 2 reads 2 x 4 + 3.
        Assert.Equal(2 * 4, cartridge.PpuRead(0x0000));
        Assert.Equal((2 * 4) + 3, cartridge.PpuRead(0x0FD8));
        Assert.Equal(1 * 4, cartridge.PpuRead(0x0000));

        Assert.Equal((1 * 4) + 3, cartridge.PpuRead(0x0FE8));
        Assert.Equal(2 * 4, cartridge.PpuRead(0x0000));

        // The high window has its own latch and its own pair of registers.
        Assert.Equal(4 * 4, cartridge.PpuRead(0x1000));
        Assert.Equal((4 * 4) + 3, cartridge.PpuRead(0x1FD8));
        Assert.Equal(3 * 4, cartridge.PpuRead(0x1000));
    }

    [Fact]
    public void REQ_CORE_033_a_peek_never_moves_an_mmc2_latch()
    {
        NesCore core = MapperFixture.Load(9, chrBanks1k: 64);
        core.MapperWrite(0xB000, 0x01);
        core.MapperWrite(0xC000, 0x02);

        Cartridge cartridge = core.Cartridge!;
        int before = cartridge.ChrLatchState;

        for (int address = 0; address < 0x2000; address++)
        {
            cartridge.PpuPeek((ushort)address);
        }

        Assert.Equal(before, cartridge.ChrLatchState);
        Assert.Equal(2 * 4, cartridge.PpuPeek(0x0000));
    }

    [Fact]
    public void REQ_CORE_100_mmc4_only_needs_the_row_of_the_tile_to_move_its_latch()
    {
        // MMC2 reacts to the single address $0FD8, MMC4 to the whole row $0FD8-$0FDF.
        NesCore mmc2 = MapperFixture.Load(9, chrBanks1k: 64);
        mmc2.MapperWrite(0xB000, 0x01);
        mmc2.MapperWrite(0xC000, 0x02);
        mmc2.Cartridge!.PpuRead(0x0FD9);
        Assert.Equal(2 * 4, mmc2.Cartridge.PpuPeek(0x0000));

        NesCore mmc4 = MapperFixture.Load(10, chrBanks1k: 64);
        mmc4.MapperWrite(0xB000, 0x01);
        mmc4.MapperWrite(0xC000, 0x02);
        mmc4.Cartridge!.PpuRead(0x0FD9);
        Assert.Equal(1 * 4, mmc4.Cartridge.PpuPeek(0x0000));
    }

    // ---------------------------------------------------------------- the small boards

    [Fact]
    public void REQ_CORE_100_color_dreams_splits_its_register_between_prg_and_chr()
    {
        NesCore core = MapperFixture.Load(11, prgBanks8k: 16, chrBanks1k: 64);

        core.WriteMemory(0x8000, 0x32);
        Assert.Equal<byte[]>([8, 9, 10, 11], core.PrgWindows());
        Assert.Equal<byte[]>([24, 25, 26, 27, 28, 29, 30, 31], core.ChrWindows());
    }

    [Fact]
    public void REQ_CORE_100_cprom_fixes_the_first_4_kib_of_chr_ram()
    {
        NesCore core = MapperFixture.Load(13, prgBanks8k: 4, chrBanks1k: 0);

        core.WriteMemory(0x8000, 0x02);
        Assert.Equal<byte[]>([0, 1, 2, 3], core.PrgWindows());

        // CHR-RAM starts empty, so the banks are told apart by writing through the PPU.
        Cartridge cartridge = core.Cartridge!;
        cartridge.PpuWrite(0x1000, 0xAA);
        core.WriteMemory(0x8000, 0x01);
        cartridge.PpuWrite(0x1000, 0xBB);

        Assert.Equal(0xBB, cartridge.PpuPeek(0x1000));
        core.WriteMemory(0x8000, 0x02);
        Assert.Equal(0xAA, cartridge.PpuPeek(0x1000));
        Assert.Equal(0, cartridge.PpuPeek(0x0000));
    }

    [Theory]
    [InlineData(0, new byte[] { 4, 5, 6, 7 })]
    [InlineData(1, new byte[] { 4, 5, 14, 15 })]
    [InlineData(2, new byte[] { 4, 4, 4, 4 })]
    [InlineData(3, new byte[] { 4, 5, 4, 5 })]
    public void REQ_CORE_100_mapper_15_has_four_prg_layouts(int mode, byte[] expected)
    {
        // AMD-021 R2-002: layout 1 fixes the last 16 KiB bank of the whole image at $C000,
        // not the last bank of a 64 KiB block.
        NesCore core = MapperFixture.Load(15, prgBanks8k: 16, chrBanks1k: 8);

        core.WriteMemory((ushort)(0x8000 + mode), 0x02);
        Assert.Equal(expected, core.PrgWindows());
    }

    [Fact]
    public void REQ_CORE_100_mapper_15_carries_mirroring_in_bit_6()
    {
        // AMD-021 R2-002: bit 6 is the layout and bit 7 the half-bank bit of layout 2.
        NesCore core = MapperFixture.Load(15, prgBanks8k: 16, chrBanks1k: 8);

        core.WriteMemory(0x8000, 0x00);
        Assert.Equal(NametableMirroring.Vertical, core.Ppu.Mirroring);
        core.WriteMemory(0x8000, 0x40);
        Assert.Equal(NametableMirroring.Horizontal, core.Ppu.Mirroring);
        core.WriteMemory(0x8000, 0x80);
        Assert.Equal(NametableMirroring.Vertical, core.Ppu.Mirroring);
    }

    [Fact]
    public void AMD_021_R2_002_mapper_15_acceptance_vectors()
    {
        // AMD-021 R2-001/R2-002 criterion: a numbered 1 MiB image, 64 banks of 16 KiB, so
        // 16 KiB bank n is the pair of 8 KiB banks (2n, 2n + 1).
        NesCore core = MapperFixture.Load(15, prgBanks8k: 128, chrBanks1k: 8);

        // Power-on shows the first 32 KiB.
        Assert.Equal<byte[]>([0, 1, 2, 3], core.PrgWindows());

        // $8001 <- $05: layout 1, B = 5 -> 5 / 63 (the last 16 KiB bank of the image).
        core.WriteMemory(0x8001, 0x05);
        Assert.Equal<byte[]>([10, 11, 126, 127], core.PrgWindows());

        // $8002 <- $85: layout 2, B = 5, h = 1 -> 8 KiB bank 11 in all four windows.
        core.WriteMemory(0x8002, 0x85);
        Assert.Equal<byte[]>([11, 11, 11, 11], core.PrgWindows());

        // $8000 <- $40: bit 6 set -> horizontal.
        core.WriteMemory(0x8000, 0x40);
        Assert.Equal(NametableMirroring.Horizontal, core.Ppu.Mirroring);
    }

    [Fact]
    public void REQ_CORE_100_bandai_fcg_banks_prg_chr_and_mirroring()
    {
        NesCore core = MapperFixture.Load(16, prgBanks8k: 16, chrBanks1k: 32);

        for (int register = 0; register < 8; register++)
        {
            core.WriteMemory((ushort)(0x8000 + register), (byte)(16 + register));
        }

        core.WriteMemory(0x8008, 0x03);
        core.WriteMemory(0x8009, 0x02);

        Assert.Equal<byte[]>([6, 7, 14, 15], core.PrgWindows());
        Assert.Equal<byte[]>([16, 17, 18, 19, 20, 21, 22, 23], core.ChrWindows());
        Assert.Equal(NametableMirroring.SingleScreenA, core.Ppu.Mirroring);

        // The FCG-1 and FCG-2 wiring puts the same registers at $6000-$7FFF.
        core.WriteMemory(0x6009, 0x01);
        Assert.Equal(NametableMirroring.Horizontal, core.Ppu.Mirroring);
    }

    [Fact]
    public void REQ_CORE_100_jaleco_ss88006_loads_its_banks_one_nibble_at_a_time()
    {
        NesCore core = MapperFixture.Load(18, prgBanks8k: 32, chrBanks1k: 32);

        core.WriteMemory(0x8000, 0x05); // $8000 bank low nibble.
        core.WriteMemory(0x8001, 0x01); // high nibble: bank $15.
        core.WriteMemory(0x8002, 0x03); // $A000 bank 3.
        core.WriteMemory(0x8003, 0x00);
        core.WriteMemory(0x9000, 0x07); // $C000 bank 7.
        core.WriteMemory(0x9001, 0x00);

        Assert.Equal<byte[]>([0x15, 3, 7, 31], core.PrgWindows());

        core.WriteMemory(0xA000, 0x09); // CHR 0 low nibble.
        core.WriteMemory(0xA001, 0x01); // high nibble: bank $19.
        Assert.Equal(0x19, core.ChrBank(0x0000));

        core.WriteMemory(0xF002, 0x01);
        Assert.Equal(NametableMirroring.Vertical, core.Ppu.Mirroring);
    }

    [Theory]
    [InlineData(21, 0x8000, 0xA000)]
    [InlineData(22, 0x8000, 0xA000)]
    [InlineData(23, 0x8000, 0xA000)]
    [InlineData(25, 0x8000, 0xA000)]
    [InlineData(27, 0x8000, 0xA000)]
    public void REQ_CORE_100_vrc_switches_two_prg_banks_and_fixes_the_last_two(
        int mapper, int first, int second)
    {
        NesCore core = MapperFixture.Load(mapper, prgBanks8k: 16, chrBanks1k: 32);

        core.WriteMemory((ushort)first, 0x03);
        core.WriteMemory((ushort)second, 0x05);
        Assert.Equal<byte[]>([3, 5, 14, 15], core.PrgWindows());

        // $9002 bit 1 moves the first switchable bank to $C000.
        core.WriteMemory((ushort)(0x9000 | VrcRegister(mapper, 2)), 0x02);
        Assert.Equal<byte[]>([14, 5, 3, 15], core.PrgWindows());
    }

    [Fact]
    public void REQ_CORE_100_vrc2a_addresses_chr_in_2_kib_units()
    {
        // Mapper 22 swaps the two register-select lines and shifts the CHR bank right by one,
        // which is what the public m22chrbankingtest ROM displays.
        NesCore core = MapperFixture.Load(22, chrBanks1k: 64);

        core.WriteMemory(0xB002, 0x04); // Register index 1 on this wiring: CHR 0 high.
        core.WriteMemory(0xB000, 0x06); // CHR 0 low.
        Assert.Equal((0x46 >> 1) & 0x3F, core.ChrBank(0x0000));
    }

    [Fact]
    public void REQ_CORE_100_vrc4_builds_each_chr_bank_from_two_nibbles()
    {
        NesCore core = MapperFixture.Load(23, chrBanks1k: 64);

        core.WriteMemory(0xB000, 0x05); // CHR 0 low nibble.
        core.WriteMemory(0xB001, 0x02); // CHR 0 high nibble: bank $25.
        Assert.Equal(0x25, core.ChrBank(0x0000));

        core.WriteMemory(0xE002, 0x0F); // CHR 7 low nibble.
        core.WriteMemory(0xE003, 0x01); // bank $1F.
        Assert.Equal(0x1F, core.ChrBank(0x1C00));
    }

    [Fact]
    public void REQ_CORE_100_action53_combines_an_outer_and_an_inner_prg_bank()
    {
        // AMD-021 R2-001: the outer register counts 32 KiB banks (E = 2 x outer) and
        // power-on is outer = 63, inner = 15, mode = 0, which on a 256 KiB image wraps to the
        // last 32 KiB — the last 16 KiB bank at $C000, what the board's own test ROM checks
        // first.
        NesCore core = MapperFixture.Load(28, prgBanks8k: 32, chrBanks1k: 0);
        Assert.Equal<byte[]>([28, 29, 30, 31], core.PrgWindows());

        // Mode $1B: horizontal (bits 0-1 = 3), UNROM layout (bits 2-3 = 2), 64 KiB game
        // (bits 4-5 = 1, so m = 3 and the inner bank picks one of four 16 KiB banks).
        Select(core, 0x80, 0x1B);
        Select(core, 0x81, 0x04); // Outer bank 4, in 32 KiB units: E = 8.
        Select(core, 0x01, 0x02); // Inner bank 2 inside the game.

        Assert.Equal(NametableMirroring.Horizontal, core.Ppu.Mirroring);

        // Layout 2: $8000 shows E = 8 and $C000 shows (8 & ~3) | 2 = 10.
        Assert.Equal<byte[]>([16, 17, 20, 21], core.PrgWindows());

        // Layout 0 shows a 32 KiB game: (8 & ~3) | (2I & 3) = 8, then 9.
        Select(core, 0x80, 0x10);
        Assert.Equal<byte[]>([16, 17, 18, 19], core.PrgWindows());
        Assert.Equal(NametableMirroring.SingleScreenA, core.Ppu.Mirroring);

        static void Select(NesCore core, byte register, byte value)
        {
            core.WriteMemory(0x5000, register);
            core.WriteMemory(0x8000, value);
        }
    }

    [Fact]
    public void AMD_021_R2_001_action53_acceptance_vectors()
    {
        // AMD-021 R2-001 criterion, on a numbered 1 MiB image: 16 KiB bank n is the pair of
        // 8 KiB banks (2n, 2n + 1).
        NesCore core = MapperFixture.Load(28, prgBanks8k: 128, chrBanks1k: 0);

        // Power-on: outer = 63 -> E = 126, mode 0, inner = 15 -> banks 126 and 127, which a
        // 64-bank image wraps to 62 and 63.
        Assert.Equal<byte[]>([124, 125, 126, 127], core.PrgWindows());

        // (ext = 5, mode = 0) -> 10 / 11.
        Select(core, 0x81, 0x05);
        Assert.Equal<byte[]>([20, 21, 22, 23], core.PrgWindows());

        // (ext = 2, mode = $18, int = 3) -> 4 / 7.
        Select(core, 0x81, 0x02);
        Select(core, 0x80, 0x18);
        Select(core, 0x01, 0x03);
        Assert.Equal<byte[]>([8, 9, 14, 15], core.PrgWindows());

        // (ext = 2, mode = $1C, int = 3) -> 7 / 5.
        Select(core, 0x80, 0x1C);
        Assert.Equal<byte[]>([14, 15, 10, 11], core.PrgWindows());

        static void Select(NesCore core, byte register, byte value)
        {
            core.WriteMemory(0x5000, register);
            core.WriteMemory(0x8000, value);
        }
    }

    [Fact]
    public void AMD_021_R2_001_action53_takes_the_single_screen_bit_from_a_chr_write()
    {
        // AMD-021 R2-001: while bit 1 of the mode is clear, a CHR or inner-PRG write replaces
        // bit 0 of the mode with bit 4 of the value — a CHR write of $10 with mode 0 selects
        // screen B.
        NesCore core = MapperFixture.Load(28, prgBanks8k: 128, chrBanks1k: 0);
        Assert.Equal(NametableMirroring.SingleScreenA, core.Ppu.Mirroring);

        core.WriteMemory(0x5000, 0x00);
        core.WriteMemory(0x8000, 0x10);
        Assert.Equal(NametableMirroring.SingleScreenB, core.Ppu.Mirroring);

        // Back to screen A, and a write to the outer register never touches the mode.
        core.WriteMemory(0x5000, 0x01);
        core.WriteMemory(0x8000, 0x00);
        Assert.Equal(NametableMirroring.SingleScreenA, core.Ppu.Mirroring);

        core.WriteMemory(0x5000, 0x81);
        core.WriteMemory(0x8000, 0x1F);
        Assert.Equal(NametableMirroring.SingleScreenA, core.Ppu.Mirroring);

        // With bit 1 of the mode set — the two-screen layouts — the bit is left alone.
        core.WriteMemory(0x5000, 0x80);
        core.WriteMemory(0x8000, 0x02); // Vertical.
        core.WriteMemory(0x5000, 0x00);
        core.WriteMemory(0x8000, 0x10);
        Assert.Equal(NametableMirroring.Vertical, core.Ppu.Mirroring);
    }

    [Fact]
    public void REQ_CORE_100_unrom512_switches_prg_chr_ram_and_mirroring()
    {
        NesCore core = MapperFixture.Load(30, prgBanks8k: 32, chrBanks1k: 0, mirroring: 0x08);

        core.WriteMemory(0x8000, 0x45); // PRG bank 5, CHR-RAM bank 2, screen A.
        Assert.Equal<byte[]>([10, 11, 30, 31], core.PrgWindows());
        Assert.Equal(NametableMirroring.SingleScreenA, core.Ppu.Mirroring);

        core.WriteMemory(0x8000, 0xC5);
        Assert.Equal(NametableMirroring.SingleScreenB, core.Ppu.Mirroring);
    }

    [Fact]
    public void REQ_CORE_100_irem_g101_switches_two_prg_banks_eight_chr_banks_and_mirroring()
    {
        NesCore core = MapperFixture.Load(32, prgBanks8k: 16, chrBanks1k: 32);

        core.WriteMemory(0x8000, 0x03);
        core.WriteMemory(0xA000, 0x05);
        Assert.Equal<byte[]>([3, 5, 14, 15], core.PrgWindows());

        core.WriteMemory(0x9000, 0x03); // Horizontal, PRG mode 1.
        Assert.Equal(NametableMirroring.Horizontal, core.Ppu.Mirroring);
        Assert.Equal<byte[]>([14, 5, 3, 15], core.PrgWindows());

        for (int register = 0; register < 8; register++)
        {
            core.WriteMemory((ushort)(0xB000 + register), (byte)(24 + register));
        }

        Assert.Equal<byte[]>([24, 25, 26, 27, 28, 29, 30, 31], core.ChrWindows());
    }

    [Fact]
    public void REQ_CORE_100_bnrom_switches_32_kib_from_8000()
    {
        NesCore core = MapperFixture.Load(34, prgBanks8k: 16, chrBanks1k: 0);

        core.WriteMemory(0x8000, 0x02);
        Assert.Equal<byte[]>([8, 9, 10, 11], core.PrgWindows());
    }

    [Fact]
    public void REQ_CORE_100_nina001_has_its_registers_at_the_top_of_the_prg_ram_window()
    {
        NesCore core = MapperFixture.Load(34, prgBanks8k: 16, chrBanks1k: 32);

        core.WriteMemory(0x7FFD, 0x01);
        core.WriteMemory(0x7FFE, 0x03);
        core.WriteMemory(0x7FFF, 0x06);

        Assert.Equal<byte[]>([4, 5, 6, 7], core.PrgWindows());
        Assert.Equal<byte[]>([12, 13, 14, 15, 24, 25, 26, 27], core.ChrWindows());
    }

    [Fact]
    public void REQ_CORE_100_caltron_takes_the_prg_bank_from_the_address_and_the_chr_from_the_data()
    {
        NesCore core = MapperFixture.Load(41, prgBanks8k: 16, chrBanks1k: 64);

        core.WriteMemory((ushort)(0x6002 | 0x04 | 0x08), 0x00); // PRG 32 KiB bank 2, horizontal, CHR high 1.
        core.WriteMemory(0x8000, 0x01);

        Assert.Equal<byte[]>([8, 9, 10, 11], core.PrgWindows());
        Assert.Equal(((1 << 2) | 1) * 8, core.ChrBank(0x0000));
        Assert.Equal(NametableMirroring.Horizontal, core.Ppu.Mirroring);
    }

    [Fact]
    public void REQ_CORE_100_rambo1_maps_three_prg_banks_and_a_1_kib_chr_mode()
    {
        NesCore core = MapperFixture.Load(64, prgBanks8k: 16, chrBanks1k: 32);

        core.Mmc3Bank(6, 3);
        core.Mmc3Bank(7, 5);
        core.Mmc3Bank(15, 9);
        Assert.Equal<byte[]>([3, 5, 9, 15], core.PrgWindows());

        core.MapperWrite(0x8000, 0x40); // PRG mode 1 rotates the three windows.
        Assert.Equal<byte[]>([9, 3, 5, 15], core.PrgWindows());

        // In the 1 KiB CHR mode, registers 8 and 9 fill the odd kilobytes of the first two
        // windows. The bank registers go in first, since selecting one clears the mode bits.
        core.Mmc3Bank(0, 16);
        core.Mmc3Bank(8, 17);
        core.Mmc3Bank(1, 18);
        core.Mmc3Bank(9, 19);
        core.Mmc3Bank(2, 20);
        core.Mmc3Bank(3, 21);
        core.Mmc3Bank(4, 22);
        core.Mmc3Bank(5, 23);
        core.MapperWrite(0x8000, 0x20);

        Assert.Equal<byte[]>([16, 17, 18, 19, 20, 21, 22, 23], core.ChrWindows());
    }

    [Fact]
    public void REQ_CORE_100_gxrom_splits_its_register_between_prg_and_chr()
    {
        NesCore core = MapperFixture.Load(66, prgBanks8k: 16, chrBanks1k: 32);

        core.WriteMemory(0x8000, 0x23);
        Assert.Equal<byte[]>([8, 9, 10, 11], core.PrgWindows());
        Assert.Equal<byte[]>([24, 25, 26, 27, 28, 29, 30, 31], core.ChrWindows());
    }

    [Fact]
    public void REQ_CORE_100_codemasters_switches_16_kib_and_can_pick_one_screen()
    {
        NesCore core = MapperFixture.Load(71, prgBanks8k: 16, chrBanks1k: 8);

        core.WriteMemory(0xC000, 0x05);
        Assert.Equal<byte[]>([10, 11, 14, 15], core.PrgWindows());

        core.WriteMemory(0x9000, 0x10);
        Assert.Equal(NametableMirroring.SingleScreenB, core.Ppu.Mirroring);
    }

    [Fact]
    public void REQ_CORE_100_jaleco_j87_swaps_the_two_bits_of_its_chr_bank()
    {
        NesCore core = MapperFixture.Load(87, prgBanks8k: 4, chrBanks1k: 32);

        core.WriteMemory(0x6000, 0x01); // Bank 2, not 1.
        Assert.Equal(2 * 8, core.ChrBank(0x0000));

        core.WriteMemory(0x6000, 0x02); // Bank 1.
        Assert.Equal(1 * 8, core.ChrBank(0x0000));
    }

    [Fact]
    public void AMD_021_R2_003_mapper_101_decodes_address_bits_13_and_14()
    {
        // AMD-021 R2-003 criterion, on a numbered 32 KiB CHR image: 8 KiB CHR bank n is the
        // run of 1 KiB banks starting at 8n.
        NesCore core = MapperFixture.Load(101, prgBanks8k: 4, chrBanks1k: 32);

        // $6000 <- $06 -> bank 6 & 3 = 2.
        core.WriteMemory(0x6000, 0x06);
        Assert.Equal(2 * 8, core.ChrBank(0x0000));

        // $E000 <- $03 -> bank 3: bits 13 and 14 of $E000 are both set.
        core.WriteMemory(0xE000, 0x03);
        Assert.Equal(3 * 8, core.ChrBank(0x0000));

        // $8000 <- $01: bits 13 and 14 clear, no change.
        core.WriteMemory(0x8000, 0x01);
        Assert.Equal(3 * 8, core.ChrBank(0x0000));

        // $A000 and $C000 carry only one of the two bits, so they are not decoded either.
        core.WriteMemory(0xA000, 0x01);
        core.WriteMemory(0xC000, 0x02);
        Assert.Equal(3 * 8, core.ChrBank(0x0000));
    }

    [Fact]
    public void REQ_CORE_100_multicart_225_takes_everything_from_the_address()
    {
        NesCore core = MapperFixture.Load(225, prgBanks8k: 32, chrBanks1k: 64);

        // 32 KiB mode: PRG bank field 2 (16 KiB units, low bit dropped) and CHR bank 3.
        core.WriteMemory((ushort)(0x8000 | (2 << 6) | 3), 0x00);
        Assert.Equal<byte[]>([4, 5, 6, 7], core.PrgWindows());
        Assert.Equal(3 * 8, core.ChrBank(0x0000));
        Assert.Equal(NametableMirroring.Vertical, core.Ppu.Mirroring);

        // 16 KiB mode, mirrored over both windows, horizontal layout.
        core.WriteMemory((ushort)(0x8000 | 0x2000 | 0x1000 | (5 << 6)), 0x00);
        Assert.Equal<byte[]>([10, 11, 10, 11], core.PrgWindows());
        Assert.Equal(NametableMirroring.Horizontal, core.Ppu.Mirroring);
    }

    [Fact]
    public void REQ_CORE_100_action52_splits_the_chr_bank_between_address_and_data()
    {
        NesCore core = MapperFixture.Load(228, prgBanks8k: 32, chrBanks1k: 64);

        core.WriteMemory((ushort)(0x8000 | (2 << 6) | 0x01), 0x02);
        Assert.Equal<byte[]>([4, 5, 6, 7], core.PrgWindows());
        Assert.Equal((((1 << 2) | 2) * 8) % 64, core.ChrBank(0x0000));

        core.WriteMemory((ushort)(0x8000 | 0x20 | (3 << 6)), 0x00);
        Assert.Equal<byte[]>([6, 7, 6, 7], core.PrgWindows());
    }

    // ---------------------------------------------------------------- savestates

    [Theory]
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(2)]
    [InlineData(3)]
    [InlineData(4)]
    [InlineData(5)]
    [InlineData(7)]
    [InlineData(9)]
    [InlineData(10)]
    [InlineData(11)]
    [InlineData(13)]
    [InlineData(15)]
    [InlineData(16)]
    [InlineData(18)]
    [InlineData(21)]
    [InlineData(22)]
    [InlineData(23)]
    [InlineData(25)]
    [InlineData(27)]
    [InlineData(28)]
    [InlineData(30)]
    [InlineData(32)]
    [InlineData(34)]
    [InlineData(41)]
    [InlineData(64)]
    [InlineData(66)]
    [InlineData(71)]
    [InlineData(87)]
    [InlineData(101)]
    [InlineData(225)]
    [InlineData(228)]
    public void REQ_CORE_090_the_mapper_state_survives_a_save_and_restore(int mapper)
    {
        NesCore core = MapperFixture.Load(mapper, prgBanks8k: 16, chrBanks1k: 32, battery: true);

        // Move whatever the board has: a few register writes, then some frames so that any
        // IRQ counter and latch has run.
        core.WriteMemory(0x8000, 0x01);
        core.WriteMemory(0x8001, 0x02);
        core.WriteMemory(0xA000, 0x03);
        core.WriteMemory(0xB000, 0x04);
        core.WriteMemory(0xC000, 0x05);
        core.WriteMemory(0xC001, 0x06);
        core.WriteMemory(0xE001, 0x07);
        core.WriteMemory(0x6000, 0x42);

        for (int frame = 0; frame < 2; frame++)
        {
            core.RunFrame(0);
        }

        byte[] state = core.SaveState();
        byte[] windows = core.PrgWindows();
        byte[] chr = core.ChrWindows();

        for (int frame = 0; frame < 5; frame++)
        {
            core.RunFrame(0);
        }

        Assert.True(core.TryLoadState(state, 0, out int consumed, out _, out string? error), error);
        Assert.Equal(state.Length, consumed);
        Assert.Equal(windows, core.PrgWindows());
        Assert.Equal(chr, core.ChrWindows());
        Assert.Equal(state, core.SaveState());
    }

    [Theory]
    [InlineData(1)]
    [InlineData(4)]
    [InlineData(5)]
    [InlineData(10)]
    public void REQ_CORE_095_battery_backed_prg_ram_is_exposed_and_survives_a_reload(int mapper)
    {
        NesCore core = MapperFixture.Load(mapper, battery: true);

        if (mapper == 5)
        {
            core.WriteMemory(0x5102, 0x02);
            core.WriteMemory(0x5103, 0x01);
        }

        core.WriteMemory(0x6123, 0x7E);
        Assert.Equal(0x7E, core.ReadMemory(0x6123));

        byte[] saved = core.ExportBatteryRam().ToArray();
        Assert.NotEmpty(saved);

        NesCore reloaded = MapperFixture.Load(mapper, battery: true);
        Assert.True(reloaded.ImportBatteryRam(saved));

        if (mapper == 5)
        {
            reloaded.WriteMemory(0x5102, 0x02);
            reloaded.WriteMemory(0x5103, 0x01);
        }

        Assert.Equal(0x7E, reloaded.ReadMemory(0x6123));

        // A reset no longer clears it, since the host has taken charge of it.
        reloaded.Reset();
        if (mapper == 5)
        {
            reloaded.WriteMemory(0x5102, 0x02);
            reloaded.WriteMemory(0x5103, 0x01);
        }

        Assert.Equal(0x7E, reloaded.ReadMemory(0x6123));
    }

    [Fact]
    public void REQ_CORE_095_a_cartridge_without_a_battery_exposes_nothing()
    {
        NesCore core = MapperFixture.Load(4);

        Assert.True(core.ExportBatteryRam().IsEmpty);
        Assert.False(core.ImportBatteryRam([1, 2, 3]));
    }

    [Theory]
    [InlineData(1)]
    [InlineData(4)]
    [InlineData(16)]
    [InlineData(64)]
    public void REQ_CORE_004_a_reset_puts_the_mapper_back_to_its_power_on_banks(int mapper)
    {
        NesCore core = MapperFixture.Load(mapper, prgBanks8k: 16, chrBanks1k: 32);
        byte[] powerOn = core.PrgWindows();

        core.Mmc1Write(0xE000, 0x05);
        core.Mmc3Bank(6, 7);
        core.WriteMemory(0x8008, 0x03);
        core.Reset();

        Assert.Equal(powerOn, core.PrgWindows());
        Assert.False(core.Cartridge!.IrqAsserted);
    }

    /// <summary>
    /// Index of a VRC register inside its $x000 group, for the wiring of
    /// <paramref name="mapper"/>.
    /// </summary>
    private static int VrcRegister(int mapper, int index) => mapper switch
    {
        21 => ((index & 1) << 1) | ((index & 2) << 1),
        22 or 25 => ((index & 1) << 1) | ((index & 2) >> 1),
        _ => index,
    };
}
