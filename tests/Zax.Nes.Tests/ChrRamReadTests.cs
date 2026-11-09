namespace Zax.Nes.Tests;

/// <summary>
/// Read-only access to the cartridge's CHR-RAM (AMD-062 REQ-CORE-123), which the tile
/// resolution of a « 3dh » profile needs between two frames.
/// </summary>
public sealed class ChrRamReadTests
{
    /// <summary>
    /// A program that writes the sixteen bytes <c>00 01 … 0f</c> to PPU address $0130 through
    /// $2006 and $2007, rendering off, then loops.
    /// </summary>
    private static byte[] WriteSixteenBytesAt0130()
        =>
        [
            0xAD, 0x02, 0x20,       // $8000  LDA $2002   — clears the $2006 write toggle
            0xA9, 0x00,             // $8003  LDA #$00
            0x8D, 0x00, 0x20,       // $8005  STA $2000   — increment by one, NMI off
            0x8D, 0x01, 0x20,       // $8008  STA $2001   — rendering off
            0xA9, 0x01,             // $800B  LDA #$01
            0x8D, 0x06, 0x20,       // $800D  STA $2006   — high byte of $0130
            0xA9, 0x30,             // $8010  LDA #$30
            0x8D, 0x06, 0x20,       // $8012  STA $2006   — low byte
            0xA2, 0x00,             // $8015  LDX #$00
            0x8A,                   // $8017  TXA         — start of the loop
            0x8D, 0x07, 0x20,       // $8018  STA $2007
            0xE8,                   // $801B  INX
            0xE0, 0x10,             // $801C  CPX #$10
            0xD0, 0xF7,             // $801E  BNE $8017
            0x4C, 0x20, 0x80,       // $8020  JMP $8020
        ];

    // REQ-CORE-123: a mapper 0 ROM with header byte 5 at zero carries 32 KiB of CHR-RAM
    // (REQ-CORE-012), with no banking, so PPU $0000-$1FFF is positions 0 to 0x1FFF.
    [Fact]
    public void The_core_reads_the_chr_ram_the_program_wrote()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(ChrRamRom(WriteSixteenBytesAt0130()), "ChrRam.nes").Success);

        core.RunFrame(0);

        Assert.Equal(32 * 1024, core.ChrRamSize);
        Assert.Equal(32 * 1024, core.ChrRam.Length);
        for (int index = 0; index < 16; index++)
        {
            Assert.Equal((byte)index, core.ChrRam[0x130 + index]);
        }

        Assert.Equal(0, core.ChrRam[0x12F]);
        Assert.Equal(0, core.ChrRam[0x140]);
    }

    // REQ-CORE-123: a ROM with CHR-ROM has no CHR-RAM.
    [Fact]
    public void A_cartridge_with_chr_rom_reports_no_chr_ram()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        core.RunFrame(0);

        Assert.Equal(0, core.ChrRamSize);
        Assert.True(core.ChrRam.IsEmpty);
    }

    [Fact]
    public void Without_a_rom_there_is_no_chr_ram()
    {
        NesCore core = new();
        Assert.Equal(0, core.ChrRamSize);
        Assert.True(core.ChrRam.IsEmpty);
    }

    // REQ-CORE-123: reading the CHR-RAM has no side effect, so two runs that differ only in
    // whether they read it every frame give the same frames.
    [Fact]
    public void Reading_the_chr_ram_changes_nothing_of_the_emulation()
    {
        List<byte[]> withReads = Run(readChrRam: true);
        List<byte[]> withoutReads = Run(readChrRam: false);

        Assert.Equal(withoutReads.Count, withReads.Count);
        for (int frame = 0; frame < withReads.Count; frame++)
        {
            Assert.Equal(withoutReads[frame], withReads[frame]);
        }
    }

    private static List<byte[]> Run(bool readChrRam)
    {
        NesCore core = new();
        Assert.True(core.LoadRom(ChrRamRom(WriteSixteenBytesAt0130()), "ChrRam.nes").Success);

        List<byte[]> frames = [];
        core.VideoCallback = (in FrameBatch batch) =>
        {
            byte[] copy = new byte[batch.Picture.Length + batch.Palette.Length];
            batch.Picture.CopyTo(copy);
            batch.Palette.CopyTo(copy.AsSpan(batch.Picture.Length));
            frames.Add(copy);
        };

        long checksum = 0;
        for (int frame = 0; frame < 8; frame++)
        {
            core.RunFrame((ushort)(frame & 1));
            if (readChrRam)
            {
                ReadOnlySpan<byte> chr = core.ChrRam;
                foreach (byte value in chr)
                {
                    checksum += value;
                }
            }
        }

        Assert.True(!readChrRam || checksum > 0, "the program should have written something.");
        return frames;
    }

    /// <summary>A mapper 0 image with no CHR-ROM, which gives 32 KiB of CHR-RAM.</summary>
    private static byte[] ChrRamRom(byte[] program)
    {
        const int prgSize = 2 * 16 * 1024;
        byte[] image = new byte[16 + prgSize];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 0x02;
        image[5] = 0x00;
        image[6] = 0x01;
        program.CopyTo(image, 16);

        int vectors = 16 + prgSize - 6;
        for (int vector = 0; vector < 3; vector++)
        {
            image[vectors + (2 * vector)] = 0x00;
            image[vectors + (2 * vector) + 1] = 0x80;
        }

        return image;
    }
}
