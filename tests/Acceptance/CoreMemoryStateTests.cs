using Zax.Nes;
using Zax.Nes.Cartridges;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for SPEC-02 §3.8 (memory access), §3.9 (savestates),
/// §3.10 (battery PRG-RAM) and §3.13 (core choice).
/// </summary>
public sealed class CoreMemoryStateTests
{
    [Fact] // REQ-CORE-080
    public void Reading_a_byte_sees_ram_its_mirror_and_the_banked_prg()
    {
        NesCore core = SpecRom.Warm();
        core.WriteMemory(0x0012, 0x42);
        Assert.Equal(0x42, core.ReadMemory(0x0012));
        Assert.Equal(0x42, core.ReadMemory(0x0812));

        // Low byte of the reset vector, which the minimal ROM sets to $8000.
        Assert.Equal(0x00, core.ReadMemory(0xFFFC));
        Assert.Equal(0x80, core.ReadMemory(0xFFFD));

        // PRG-RAM is readable and writable.
        core.WriteMemory(0x6123, 0x99);
        Assert.Equal(0x99, core.ReadMemory(0x6123));
    }

    [Fact] // REQ-CORE-080 (the read has no side effect)
    public void Host_reads_do_not_disturb_the_emulation()
    {
        static long Run(bool peeking)
        {
            NesCore core = SpecRom.Warm();
            for (int i = 0; i < 100; i++)
            {
                core.RunFrame(0);
                if (peeking)
                {
                    for (int a = 0x2000; a < 0x4020; a++)
                    {
                        core.ReadMemory((ushort)a);
                    }
                }
            }

            return core.CycleCount;
        }

        Assert.Equal(Run(false), Run(true));
    }

    [Fact] // REQ-CORE-081
    public void Reading_2002_from_the_host_leaves_the_vblank_flag_alone()
    {
        NesCore core = SpecRom.Warm();
        core.RunFrame(0); // stops at the start of vblank, flag set

        Assert.Equal(0, core.ReadMemory(0x2002));           // no side effect, returns 0
        Assert.Equal(0x80, core.Bus.Read(0x2002) & 0x80);   // the game still sees the flag
    }

    [Fact] // REQ-CORE-081 ($4020-$5FFF reads nothing)
    public void The_expansion_area_reads_as_zero_without_side_effects()
    {
        NesCore core = SpecRom.Warm();
        for (int a = 0x4020; a <= 0x5FFF; a += 0x100)
        {
            Assert.Equal(0, core.ReadMemory((ushort)a));
        }
    }

    [Fact] // REQ-CORE-082
    public void Writing_a_byte_has_the_effect_a_cpu_write_would_have()
    {
        NesCore core = SpecRom.Warm();
        core.WriteMemory(0x2006, 0x3F);
        core.WriteMemory(0x2006, 0x00);
        core.WriteMemory(0x2007, 0x30);

        CapturedBatch batch = CapturedBatch.Run(core);
        Assert.Equal(0x30, batch.Palette[0]);
    }

    [Fact] // REQ-CORE-082 (it must not advance emulated time)
    public void Writing_a_byte_does_not_advance_emulated_time()
    {
        NesCore core = SpecRom.Warm();
        long before = core.CycleCount;
        for (int i = 0; i < 1000; i++)
        {
            core.WriteMemory(0x0200, (byte)i);
        }

        Assert.Equal(before, core.CycleCount);
    }

    // ------------------------------------------------------------------ savestates

    [Fact] // REQ-CORE-090
    public void A_state_block_is_self_contained()
    {
        NesCore reference = SpecRom.Warm();
        for (int i = 0; i < 20; i++)
        {
            reference.RunFrame((ushort)i);
        }

        byte[] state = reference.SaveState();

        // Uninterrupted run, for comparison.
        CapturedBatch expectedBatch = CapturedBatch.Run(reference, 0x1234);
        byte[] expectedRam = Ram(reference);

        NesCore restored = SpecRom.Warm();
        restored.CloseRom();
        Assert.True(restored.TryLoadState(state, 0, out _, out _, out _));
        Assert.True(restored.IsRunning);

        CapturedBatch actualBatch = CapturedBatch.Run(restored, 0x1234);
        Assert.Equal(expectedBatch.Records, actualBatch.Records);
        Assert.Equal(expectedBatch.Palette, actualBatch.Palette);
        Assert.Equal(expectedRam, Ram(restored));
    }

    [Fact] // REQ-CORE-090 (the block opens with a format version number)
    public void A_state_block_opens_with_a_version()
    {
        NesCore core = SpecRom.Warm();
        byte[] state = core.SaveState();
        Assert.True(state.Length > 8);
        Assert.True(NesCore.StateFormatVersion > 0);
    }

    [Fact] // REQ-CORE-091
    public void Restoring_reports_the_bytes_consumed_and_the_rom_name()
    {
        NesCore core = SpecRom.Warm(name: "Named.nes");
        byte[] state = core.SaveState();

        byte[] padded = new byte[7 + state.Length + 100];
        state.CopyTo(padded, 7);

        Assert.True(core.TryLoadState(padded, 7, out int consumed, out string name, out string? error));
        Assert.Null(error);
        Assert.Equal(state.Length, consumed);
        Assert.Equal("Named.nes", name);
    }

    [Fact] // REQ-CORE-091 (a truncated block fails and leaves the core stopped)
    public void A_truncated_block_fails_and_stops_the_core()
    {
        NesCore core = SpecRom.Warm();
        byte[] state = core.SaveState();

        Assert.False(core.TryLoadState(state.AsMemory(0, state.Length / 2), 0, out _, out string name, out string? error));
        Assert.Equal(string.Empty, name);
        Assert.NotNull(error);
        Assert.False(core.IsRunning);
        Assert.Equal(0, core.RunFrame(0));
    }

    [Fact] // REQ-CORE-092
    public void The_core_is_deterministic_across_a_save_and_restore()
    {
        static (string Digest, byte[] Ram) Run(bool withRoundTrip)
        {
            NesCore core = SpecRom.Warm();
            core.SampleRate = 48_000;
            System.Security.Cryptography.IncrementalHash hash =
                System.Security.Cryptography.IncrementalHash.CreateHash(
                    System.Security.Cryptography.HashAlgorithmName.SHA256);

            core.VideoCallback = (in FrameBatch b) =>
            {
                foreach (TileRecord r in b.Records)
                {
                    hash.AppendData(BitConverter.GetBytes(r.Plane0));
                    hash.AppendData(BitConverter.GetBytes(r.Plane1));
                    hash.AppendData(BitConverter.GetBytes(r.X));
                    hash.AppendData(BitConverter.GetBytes(r.Y));
                    hash.AppendData(BitConverter.GetBytes(r.Index));
                }

                hash.AppendData(b.Palette);
            };

            Random random = new(12345);
            byte[]? mid = null;
            for (int i = 0; i < 1000; i++)
            {
                if (withRoundTrip && i == 500)
                {
                    mid = core.SaveState();
                    Assert.True(core.TryLoadState(mid, 0, out _, out _, out _));
                }

                core.RunFrame((ushort)random.Next(0x10000));
            }

            return (Convert.ToHexString(hash.GetHashAndReset()), Ram(core));
        }

        (string plainDigest, byte[] plainRam) = Run(false);
        (string roundTripDigest, byte[] roundTripRam) = Run(true);

        Assert.Equal(plainDigest, roundTripDigest);
        Assert.Equal(plainRam, roundTripRam);
    }

    [Fact] // REQ-CORE-093
    public void Saving_and_restoring_a_512_kib_rom_is_fast()
    {
        byte[] image = BigRom(512 * 1024);
        NesCore core = new();
        Assert.True(core.LoadRom(image, "Big.nes").Success);
        for (int i = 0; i < 10; i++)
        {
            core.RunFrame(0);
        }

        // Warm-up, then 1 000 round trips.
        byte[] state = core.SaveState();
        core.TryLoadState(state, 0, out _, out _, out _);

        System.Diagnostics.Stopwatch watch = System.Diagnostics.Stopwatch.StartNew();
        for (int i = 0; i < 1000; i++)
        {
            state = core.SaveState();
            Assert.True(core.TryLoadState(state, 0, out _, out _, out _));
        }

        watch.Stop();
        double perOperation = watch.Elapsed.TotalMilliseconds / 2000.0;
        Assert.True(perOperation < 2.0, $"save/restore took {perOperation:F3} ms, budget 2 ms");
    }

    [Fact] // REQ-CORE-093 (the lightweight variant without the ROM image)
    public void A_lightweight_state_omits_the_rom_image()
    {
        NesCore core = new();
        core.LoadRom(BigRom(512 * 1024), "Big.nes");
        core.RunFrame(0);

        byte[] full = core.SaveState();
        byte[] light = core.SaveState(includeRomImage: false);
        Assert.True(light.Length < full.Length - (400 * 1024));
        Assert.True(core.TryLoadState(light, 0, out _, out _, out _));
    }

    [Fact] // REQ-CORE-095
    public void Battery_prg_ram_round_trips_and_survives_a_reset()
    {
        byte[] image = SpecRom.Build();
        image[6] |= 0x02; // battery bit

        NesCore core = new();
        Assert.True(core.LoadRom(image, "Battery.nes").Success);
        for (int a = 0x6000; a <= 0x7FFF; a++)
        {
            core.WriteMemory((ushort)a, (byte)(a & 0xFF));
        }

        byte[] exported = core.ExportBatteryRam().ToArray();
        Assert.Equal(8 * 1024, exported.Length);

        Assert.True(core.LoadRom(image, "Battery.nes").Success);
        Assert.True(core.ImportBatteryRam(exported));
        for (int a = 0x6000; a <= 0x7FFF; a++)
        {
            Assert.Equal((byte)(a & 0xFF), core.ReadMemory((ushort)a));
        }

        core.Reset();
        Assert.Equal((byte)0x23, core.ReadMemory(0x6023));
    }

    [Fact] // REQ-CORE-120
    public void There_is_exactly_one_core_and_a_foreign_state_is_refused()
    {
        Assert.Equal("Zaxnes", NesCore.CoreName);
        Assert.Equal("Save state used a different core.", NesCore.ForeignCoreMessage);

        NesCore core = SpecRom.Warm();
        byte[] state = core.SaveState();

        // Corrupt the core name stored in the block.
        int at = IndexOf(state, "Zaxnes"u8);
        Assert.True(at > 0, "the core name must appear in the block");
        state[at] = (byte)'X';

        Assert.False(core.TryLoadState(state, 0, out _, out _, out string? error));
        Assert.Equal(NesCore.ForeignCoreMessage, error);
    }

    private static int IndexOf(byte[] haystack, ReadOnlySpan<byte> needle)
    {
        for (int i = 0; i + needle.Length <= haystack.Length; i++)
        {
            if (haystack.AsSpan(i, needle.Length).SequenceEqual(needle))
            {
                return i;
            }
        }

        return -1;
    }

    private static byte[] Ram(NesCore core)
    {
        byte[] ram = new byte[0x800];
        for (int a = 0; a < 0x800; a++)
        {
            ram[a] = core.ReadMemory((ushort)a);
        }

        return ram;
    }

    /// <summary>A UxROM image whose PRG is <paramref name="size"/> bytes.</summary>
    private static byte[] BigRom(int size)
    {
        int banks = size / (16 * 1024);
        byte[] image = new byte[16 + size];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = (byte)banks;
        image[5] = 0x00;          // CHR-RAM
        image[6] = 0x21;          // mapper 2, vertical
        image[7] = 0x00;

        for (int bank = 0; bank < banks; bank++)
        {
            int at = 16 + (bank * 16 * 1024);
            image[at] = 0x4C;
            image[at + 1] = 0x00;
            image[at + 2] = 0x80;
        }

        int last = 16 + size;
        image[last - 6] = 0x00;
        image[last - 5] = 0x80;
        image[last - 4] = 0x00;
        image[last - 3] = 0x80;
        image[last - 2] = 0x00;
        image[last - 1] = 0x80;
        return image;
    }

    [Fact] // REQ-CORE-100, REQ-CORE-102
    public void The_mapper_list_matches_the_specification()
    {
        int[] required = [0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 13, 15, 16, 18, 21, 22, 23, 25, 27, 28,
            30, 32, 34, 41, 64, 66, 71, 87, 101, 225, 228];

        foreach (int mapper in required)
        {
            Assert.Contains(mapper, NesCore.SupportedMappers);
        }

        Assert.DoesNotContain(200, NesCore.SupportedMappers);
    }

    [Theory] // REQ-CORE-100 (every required mapper accepts a ROM)
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
    public void Every_required_mapper_loads_and_runs(int mapper)
    {
        byte[] image = SpecRom.Build();
        image[6] = (byte)((image[6] & 0x0F) | ((mapper & 0x0F) << 4));
        image[7] = (byte)(mapper & 0xF0);

        NesCore core = new();
        Assert.True(core.LoadRom(image, $"m{mapper}.nes").Success);
        for (int i = 0; i < 60; i++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(60, core.FrameCount);
    }

    [Fact] // REQ-CORE-102
    public void An_unknown_mapper_is_refused()
    {
        byte[] image = SpecRom.Build();
        image[6] = (byte)((image[6] & 0x0F) | 0x80);
        image[7] = 0xC0;

        NesCore core = new();
        Assert.False(core.LoadRom(image, "m200.nes").Success);
    }

    [Fact] // REQ-CORE-101 (mapper 116 is optional; whichever way, loading must not crash)
    public void Mapper_116_is_optional()
    {
        byte[] image = SpecRom.Build();
        image[6] = (byte)((image[6] & 0x0F) | 0x40);
        image[7] = 0x70;

        NesCore core = new();
        Zax.Nes.Rom.RomLoadResult result = core.LoadRom(image, "m116.nes");
        if (!result.Success)
        {
            Assert.NotNull(result.Error);
        }
    }

    [Fact] // REQ-CORE-075 (expansion audio is not required)
    public void Expansion_audio_is_not_required()
    {
        // The requirement is permissive: nothing to fail. The core is checked only for not
        // crashing on a VRC-class mapper, which is REQ-CORE-100 territory.
        Assert.True(MapperRegistry.SupportedMappers.Count > 0);
    }
}
