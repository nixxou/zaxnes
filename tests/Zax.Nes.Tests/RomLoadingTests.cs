using Zax.Nes.Rom;

namespace Zax.Nes.Tests;

/// <summary>
/// iNES and NES 2.0 header parsing, region detection and the rejection rules
/// (REQ-CORE-010 to REQ-CORE-014, REQ-CORE-102; vectors of SPEC-02 §5.2).
/// </summary>
public sealed class RomLoadingTests
{
    [Fact]
    public void REQ_CORE_010_minimal_test_rom_loads()
    {
        var core = new NesCore();
        RomLoadResult result = core.LoadRom(TestRoms.Minimal(), "Minimal.nes");

        Assert.True(result.Success, result.Error);
        Assert.Null(result.Error);
        Assert.True(core.IsRunning);
    }

    [Fact]
    public void REQ_CORE_010_equivalent_nes20_header_loads()
    {
        byte[] image = TestRoms.Minimal();

        // Same cartridge announced as NES 2.0: byte 7 bits 2 and 3 become %10, and the
        // size MSB nibbles of byte 9 stay zero.
        image[7] = 0x08;

        var core = new NesCore();
        RomLoadResult result = core.LoadRom(image, "Minimal20.nes");

        Assert.True(result.Success, result.Error);
        Assert.Equal(NesRegion.Ntsc, result.Region);
    }

    [Fact]
    public void REQ_CORE_011_missing_signature_is_rejected()
    {
        var core = new NesCore();
        RomLoadResult result = core.LoadRom(new byte[16], "Zeroes.nes");

        Assert.False(result.Success);
        Assert.Contains("signature", result.Error, StringComparison.OrdinalIgnoreCase);
        Assert.False(core.IsRunning);
    }

    [Fact]
    public void REQ_CORE_011_truncated_image_is_rejected()
    {
        byte[] image = TestRoms.Minimal();
        byte[] truncated = image[..^1];

        var core = new NesCore();
        RomLoadResult result = core.LoadRom(truncated, "Truncated.nes");

        Assert.False(result.Success);
        Assert.Contains("Truncated", result.Error, StringComparison.OrdinalIgnoreCase);
        Assert.False(core.IsRunning);
    }

    [Fact]
    public void REQ_CORE_011_unsupported_mapper_is_rejected_with_its_number()
    {
        // Mapper 200 = low nibble 8 in byte 6, high nibble C in byte 7.
        byte[] image = TestRoms.Minimal();
        image[6] = 0x80;
        image[7] = 0xC0;

        var core = new NesCore();
        RomLoadResult result = core.LoadRom(image, "Mapper200.nes");

        Assert.False(result.Success);
        Assert.Contains("200", result.Error, StringComparison.Ordinal);
        Assert.False(core.IsRunning);
    }

    [Fact]
    public void REQ_CORE_102_only_implemented_mappers_are_accepted()
    {
        // The table of REQ-CORE-100, and nothing else: the list itself is checked in
        // MapperTests, here only that it is the gate the loader uses.
        Assert.DoesNotContain(116, NesCore.SupportedMappers);
        Assert.DoesNotContain(200, NesCore.SupportedMappers);
        Assert.Contains(0, NesCore.SupportedMappers);
        Assert.Contains(228, NesCore.SupportedMappers);
    }

    [Fact]
    public void REQ_CORE_011_header_shorter_than_16_bytes_is_rejected()
    {
        var core = new NesCore();
        RomLoadResult result = core.LoadRom([0x4E, 0x45, 0x53, 0x1A], "Short.nes");

        Assert.False(result.Success);
        Assert.NotNull(result.Error);
    }

    [Fact]
    public void REQ_CORE_041_random_bytes_never_throw()
    {
        // A corrupt file must never crash the application (REQ-ACC-041).
        var random = new Random(1234);
        var core = new NesCore();

        for (int attempt = 0; attempt < 200; attempt++)
        {
            var image = new byte[random.Next(0, 300)];
            random.NextBytes(image);
            RomLoadResult result = core.LoadRom(image, "Fuzz.nes");
            Assert.True(result.Success || result.Error is not null);
        }
    }

    // ------------------------------------------------------- SPEC-02 §5.2 vectors

    [Fact]
    public void REQ_CORE_012_vector1_mapper0_vertical_chr_rom_ntsc()
    {
        InesHeader header = Parse(0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00);

        Assert.Equal(0, header.Mapper);
        Assert.Equal(NametableMirroring.Vertical, header.Mirroring);
        Assert.Equal(8 * 1024, header.ChrRomSize);
        Assert.Equal(0, header.ChrRamSize);
        Assert.Equal(NesRegion.Ntsc, header.Region);
    }

    [Fact]
    public void REQ_CORE_012_vector2_mapper1_horizontal_battery_chr_ram()
    {
        InesHeader header = Parse(0x08, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00);

        Assert.Equal(1, header.Mapper);
        Assert.Equal(NametableMirroring.Horizontal, header.Mirroring);
        Assert.True(header.HasBattery);
        Assert.Equal(0, header.ChrRomSize);
        Assert.Equal(32 * 1024, header.ChrRamSize);
        Assert.Equal(128 * 1024, header.PrgRomSize);
    }

    [Fact]
    public void REQ_CORE_012_vector3_dirty_header_ignores_high_nibble_of_byte7()
    {
        // "DiskDude!" in bytes 7 to 15.
        InesHeader header = Parse(0x02, 0x01, 0x41, 0x44, 0x69, 0x73, 0x6B, 0x44, 0x75, 0x64, 0x65, 0x21);

        Assert.True(header.IsDirty);
        Assert.Equal(4, header.Mapper);
        Assert.Equal(NametableMirroring.Vertical, header.Mirroring);

        // Byte 9 is 's' (0x73), whose bit 0 is 1; the dirty rule still gives NTSC because
        // bytes 7 to 15 are not read for the region (REQ-CORE-013).
        Assert.Equal(NesRegion.Ntsc, header.Region);
    }

    [Fact]
    public void REQ_CORE_012_vector4_byte6_23_byte7_64_is_mapper2()
    {
        InesHeader header = Parse(0x02, 0x00, 0x23, 0x64, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00);

        Assert.Equal(2, header.Mapper);
    }

    [Fact]
    public void REQ_CORE_012_vector5_four_screen()
    {
        InesHeader header = Parse(0x02, 0x01, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00);

        Assert.Equal(NametableMirroring.FourScreen, header.Mirroring);
    }

    [Fact]
    public void REQ_CORE_013_vector6_nes20_byte12_is_dendy()
    {
        InesHeader header = Parse(0x02, 0x01, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00);

        Assert.True(header.IsNes20);
        Assert.Equal(NesRegion.Dendy, header.Region);
    }

    [Fact]
    public void REQ_CORE_013_vector7_ines1_byte9_bit0_is_pal()
    {
        InesHeader header = Parse(0x02, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00);

        Assert.False(header.IsNes20);
        Assert.Equal(NesRegion.Pal, header.Region);
    }

    [Theory]
    [InlineData(0x00, NesRegion.Ntsc)]
    [InlineData(0x01, NesRegion.Pal)]
    [InlineData(0x02, NesRegion.Multi)]
    [InlineData(0x03, NesRegion.Dendy)]
    public void REQ_CORE_013_nes20_region_bits(byte byte12, NesRegion expected)
    {
        InesHeader header = Parse(0x02, 0x01, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, byte12, 0x00, 0x00, 0x00);

        Assert.Equal(expected, header.Region);
    }

    [Fact]
    public void REQ_CORE_013_detected_region_is_not_applied_by_the_core()
    {
        byte[] image = TestRoms.Minimal();
        image[9] = 0x01; // iNES 1.0 PAL flag.

        var core = new NesCore();
        RomLoadResult result = core.LoadRom(image, "Pal.nes");

        Assert.True(result.Success, result.Error);
        Assert.Equal(NesRegion.Pal, result.Region);

        // The host decides; the core stays on its current region until told otherwise.
        Assert.Equal(NesRegion.Ntsc, core.Region);
    }

    [Fact]
    public void REQ_CORE_012_trainer_is_skipped_and_loaded_at_7000()
    {
        const int prgSize = 2 * 16 * 1024;
        byte[] minimal = TestRoms.Minimal();
        var image = new byte[minimal.Length + 512];

        minimal.AsSpan(0, 16).CopyTo(image);
        image[6] |= 0x04; // Trainer present.
        for (int i = 0; i < 512; i++)
        {
            image[16 + i] = (byte)(i & 0xFF);
        }

        minimal.AsSpan(16).CopyTo(image.AsSpan(16 + 512));

        var core = new NesCore();
        RomLoadResult result = core.LoadRom(image, "Trainer.nes");

        Assert.True(result.Success, result.Error);

        // The PRG must start after the trainer: $8000 still holds the JMP of §5.1.
        Assert.Equal(0x4C, core.ReadMemory(0x8000));

        // And the trainer must have landed at $7000.
        Assert.Equal(0x00, core.ReadMemory(0x7000));
        Assert.Equal(0x7F, core.ReadMemory(0x707F));
        Assert.Equal(prgSize, 32768);
    }

    [Fact]
    public void REQ_CORE_014_rom_name_is_kept()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Test.nes").Success);

        Assert.Equal("Test.nes", core.RomName);
    }

    [Fact]
    public void REQ_CORE_012_trailing_bytes_beyond_the_announced_size_are_ignored()
    {
        byte[] image = [.. TestRoms.Minimal(), .. new byte[128]];

        var core = new NesCore();
        Assert.True(core.LoadRom(image, "WithTitle.nes").Success);
    }

    private static InesHeader Parse(params byte[] bytes4To15)
    {
        byte[] header = TestRoms.FromHeaderVector(bytes4To15);
        Assert.True(InesHeader.TryParse(header, out InesHeader parsed, out string? error), error);
        return parsed;
    }
}
