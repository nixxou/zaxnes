using Zax.Nes;

namespace Zax.App.Tests;

/// <summary>
/// The test vector ROM of AMD-041 REQ-LOOP-065: it shows, on the image of a frame, the value a
/// per-frame counter had at the <b>beginning</b> of that frame and the state of button A read
/// during that same frame.
/// </summary>
/// <remarks>
/// <para>
/// Built here from the specification text, like every other test ROM of this solution. It is an
/// NROM image (32 KiB of PRG, 8 KiB of CHR) whose CHR is the one of SPEC-02 §5.1: tile <c>n</c>
/// has plane 0 = eight bytes equal to <c>n</c> and plane 1 = eight bytes equal to
/// <c>255 − n</c>. Writing the counter as the tile index of the top-left cell therefore prints
/// the counter's <b>bits</b> on the first eight pixels of the picture, and
/// <see cref="Displayed"/> reads them back.
/// </para>
/// <para>
/// One frame of the core runs from the start of a vblank to the start of the next one, so the NMI
/// handler of a frame runs first and the visible lines that follow render what it wrote: the
/// picture a frame produces is the one its own handler prepared, from the counter value it had on
/// entry and from the controller word that frame was given.
/// </para>
/// </remarks>
internal static class RunAheadRom
{
    /// <summary>Zero-page address holding the value the current picture shows.</summary>
    public const ushort DisplayedAddress = 0x0010;

    /// <summary>Zero-page address of the counter, incremented once per emulated frame.</summary>
    public const ushort CounterAddress = 0x0011;

    /// <summary>Zero-page address holding the A bit read during the current frame.</summary>
    public const ushort ButtonAddress = 0x0012;

    /// <summary>
    /// Where the light the Zapper saw is recorded, one byte per emulated frame: <c>$08</c> when
    /// port 2 reported light at least once during the frame, 0 when it never did
    /// (REQ-LOOP-068).
    /// </summary>
    public const ushort ProbeLog = 0x0200;

    /// <summary>Zero-page index of the next entry of <see cref="ProbeLog"/>.</summary>
    public const ushort ProbeIndexAddress = 0x0013;

    /// <summary>Zero-page accumulator of the light bit for the frame under way.</summary>
    public const ushort ProbeAccumulatorAddress = 0x0014;

    /// <summary>NES colour the pattern bits set to 1 are drawn in (palette entry 1).</summary>
    private const byte OneColour = 0x21;

    /// <summary>Builds the image.</summary>
    public static byte[] Build()
    {
        byte[] image = new byte[16 + (32 * 1024) + (8 * 1024)];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 2; // 2 × 16 KiB PRG
        image[5] = 1; // 1 × 8 KiB CHR
        image[6] = 0x01; // vertical mirroring, mapper 0

        Reset.CopyTo(image, 16);
        Nmi.CopyTo(image, 16 + 0x1000);

        // Vectors: NMI at $9000, RESET and IRQ at $8000.
        int vectors = 16 + (32 * 1024) - 6;
        image[vectors] = 0x00;
        image[vectors + 1] = 0x90;
        image[vectors + 2] = 0x00;
        image[vectors + 3] = 0x80;
        image[vectors + 4] = 0x00;
        image[vectors + 5] = 0x80;

        // CHR of SPEC-02 §5.1.
        int chr = 16 + (32 * 1024);
        for (int table = 0; table < 2; table++)
        {
            for (int tile = 0; tile < 256; tile++)
            {
                int offset = chr + (table * 4096) + (tile * 16);
                if (table == 0 && tile == 0)
                {
                    continue;
                }

                for (int row = 0; row < 8; row++)
                {
                    image[offset + row] = (byte)tile;
                    image[offset + 8 + row] = (byte)(255 - tile);
                }
            }
        }

        return image;
    }

    /// <summary>
    /// The value the picture of <paramref name="core"/> shows: the eight bits of the top-left
    /// tile, most significant first.
    /// </summary>
    /// <remarks>
    /// The pixel image is <b>not</b> part of the in-memory state of REQ-CORE-121, so what it
    /// holds after a real frame of run-ahead is what the displayed frame drew — which is exactly
    /// what the criterion of REQ-LOOP-065 asks to be read.
    /// </remarks>
    public static int Displayed(NesCore core)
    {
        ArgumentNullException.ThrowIfNull(core);

        ReadOnlySpan<byte> pixels = core.PixelFrame;
        int value = 0;
        for (int bit = 0; bit < 8; bit++)
        {
            if (pixels[bit] == OneColour)
            {
                value |= 0x80 >> bit;
            }
        }

        return value;
    }

    /// <summary>Whether the picture of <paramref name="core"/> shows button A held.</summary>
    /// <remarks>
    /// The second cell holds tile <c>$80</c> or <c>$81</c>, so its last pixel carries bit 0 of
    /// the tile index, which is the A bit the frame read.
    /// </remarks>
    public static bool ShowsButtonA(NesCore core)
    {
        ArgumentNullException.ThrowIfNull(core);
        return core.PixelFrame[15] == OneColour;
    }

    /// <summary>
    /// Reset handler, at <c>$8000</c>: the two-vblank warm-up of SPEC-02 §5.1, the palette, the
    /// picture and the NMI on, then the main loop, which keeps reading port 2 so that the light
    /// the Zapper sees is sampled on the visible lines and not only during the vblank
    /// (AMD-021 R2-006).
    /// </summary>
    private static readonly byte[] Reset =
    [
        0x78,                   // $8000 SEI
        0xA2, 0xFF,             //       LDX #$FF
        0x9A,                   //       TXS
        0xA9, 0x00,             //       LDA #$00
        0x8D, 0x00, 0x20,       //       STA $2000      NMI off while the PPU warms up
        0x8D, 0x01, 0x20,       //       STA $2001      picture off
        0x85, 0x10,             //       STA $10        displayed value
        0x85, 0x11,             //       STA $11        counter
        0x85, 0x12,             //       STA $12        A bit
        0x85, 0x13,             //       STA $13        probe index
        0x85, 0x14,             //       STA $14        probe accumulator
        0xAD, 0x02, 0x20,       // L1:   LDA $2002
        0x10, 0xFB,             //       BPL L1         first vblank
        0xAD, 0x02, 0x20,       // L2:   LDA $2002
        0x10, 0xFB,             //       BPL L2         second vblank: writes now stick
        0xA9, 0x3F,             //       LDA #$3F
        0x8D, 0x06, 0x20,       //       STA $2006
        0xA9, 0x00,             //       LDA #$00
        0x8D, 0x06, 0x20,       //       STA $2006      PPUADDR = $3F00
        0xA9, 0x0F,             //       LDA #$0F
        0x8D, 0x07, 0x20,       //       STA $2007      universal backdrop
        0xA9, OneColour,        //       LDA #$21
        0x8D, 0x07, 0x20,       //       STA $2007      pattern bits at 1
        0xA9, 0x11,             //       LDA #$11
        0x8D, 0x07, 0x20,       //       STA $2007      pattern bits at 0
        0xA9, 0x30,             //       LDA #$30
        0x8D, 0x07, 0x20,       //       STA $2007
        0xA9, 0x80,             //       LDA #$80
        0x8D, 0x00, 0x20,       //       STA $2000      NMI on, patterns at $0000
        0xA9, 0x0A,             //       LDA #$0A
        0x8D, 0x01, 0x20,       //       STA $2001      background on, leftmost column shown
        0xAD, 0x17, 0x40,       // $8048 LDA $4017      port 2, the Zapper's byte
        0x29, 0x08,             //       AND #$08       the light bit
        0x05, 0x14,             //       ORA $14
        0x85, 0x14,             //       STA $14        light seen at any point this frame
        0x4C, 0x48, 0x80,       //       JMP $8048
    ];

    /// <summary>
    /// NMI handler, at <c>$9000</c>: print the counter, print the A bit the controller reports
    /// for this frame, increment the counter, put the scroll back.
    /// </summary>
    private static readonly byte[] Nmi =
    [
        0xA9, 0x20,             // LDA #$20
        0x8D, 0x06, 0x20,       // STA $2006
        0xA9, 0x00,             // LDA #$00
        0x8D, 0x06, 0x20,       // STA $2006      PPUADDR = $2000
        0xA5, 0x11,             // LDA $11        the counter, as it is on entry
        0x85, 0x10,             // STA $10        that is the value this picture shows
        0x8D, 0x07, 0x20,       // STA $2007      tile index of the top-left cell
        0xE6, 0x11,             // INC $11        one increment per emulated frame
        0xA9, 0x01,             // LDA #$01
        0x8D, 0x16, 0x40,       // STA $4016      strobe
        0xA9, 0x00,             // LDA #$00
        0x8D, 0x16, 0x40,       // STA $4016
        0xAD, 0x16, 0x40,       // LDA $4016      first read: button A
        0x29, 0x01,             // AND #$01
        0x85, 0x12,             // STA $12
        0xA9, 0x20,             // LDA #$20
        0x8D, 0x06, 0x20,       // STA $2006
        0xA9, 0x01,             // LDA #$01
        0x8D, 0x06, 0x20,       // STA $2006      PPUADDR = $2001
        0xA5, 0x12,             // LDA $12
        0x18,                   // CLC
        0x69, 0x80,             // ADC #$80       tile $80 released, $81 held
        0x8D, 0x07, 0x20,       // STA $2007
        0xA9, 0x20,             // LDA #$20
        0x8D, 0x06, 0x20,       // STA $2006
        0xA9, 0x00,             // LDA #$00
        0x8D, 0x06, 0x20,       // STA $2006      scroll back to the top left
        0xA5, 0x14,             // LDA $14        the light of the frame that has just ended
        0xA6, 0x13,             // LDX $13
        0x9D, 0x00, 0x02,       // STA $0200,X    one record per emulated frame
        0xE6, 0x13,             // INC $13
        0xA9, 0x00,             // LDA #$00
        0x85, 0x14,             // STA $14        the next frame starts over
        0x40,                   // RTI
    ];
}
