namespace Zax.Nes.Tests;

/// <summary>
/// The Zapper: aimed position, the byte the game reads and the light detection
/// (REQ-CORE-060 to REQ-CORE-062, DEC-02-Q1).
/// </summary>
public sealed class ZapperTests
{
    /// <summary>
    /// Pattern $FF of either table has plane 0 = <c>FF</c>x8 and plane 1 = <c>00</c>x8
    /// (§5.1), so every one of its pixels has colour 1.
    /// </summary>
    private const byte SolidTile = 0xFF;

    [Fact]
    public void REQ_CORE_060_the_position_is_sampled_once_at_the_start_of_each_frame()
    {
        var fixture = new TileFixture();
        NesCore core = fixture.Core;
        core.SetPortType(1, ControllerPortType.Zapper);

        var queried = new List<int>();
        int frame = 0;
        core.ZapperPositionSource = port =>
        {
            queried.Add(port);
            return new ZapperPosition(10 + frame, 20 + frame);
        };

        core.RunFrame(0);
        Assert.Equal<int[]>([1], [.. queried]);
        Assert.Equal(new ZapperPosition(10, 20), core.Input.GetZapperPosition(1));

        // Port 1 is still a controller, so it is not queried; the stored position for the
        // next frame is the one supplied before that frame's call.
        frame = 1;
        queried.Clear();
        core.RunFrame(0);
        Assert.Equal<int[]>([1], [.. queried]);
        Assert.Equal(new ZapperPosition(11, 21), core.Input.GetZapperPosition(1));
    }

    [Fact]
    public void REQ_CORE_061_an_off_screen_pointer_with_the_trigger_released_reads_as_0x18()
    {
        var fixture = new TileFixture();
        NesCore core = fixture.Core;
        core.SetPortType(1, ControllerPortType.Zapper);
        core.ZapperPositionSource = static _ => ZapperPosition.OffScreen;

        // Port 2 byte $10: bit 4 set is "trigger released" (DEC-02-Q1, SPEC-08 §4.1).
        core.RunFrame(0x1000);

        Assert.Equal(0x18, core.Input.Read(1, 0x00));
    }

    [Fact]
    public void REQ_CORE_061_a_scanned_white_pixel_with_the_trigger_pressed_reads_as_0x00()
    {
        // AMD-021 R2-006: a read made during the vblank sees the universal backdrop, so the
        // backdrop has to be the light colour for the white sprite to be read as light. The
        // pixel is probed mid-scan below, in AMD_021_R2_006_*.
        TileFixture fixture = WhiteSpriteOnBlack();
        fixture.WriteVram(0x3F00, 0x30);

        NesCore core = fixture.Core;
        core.SetPortType(1, ControllerPortType.Zapper);
        core.ZapperPositionSource = static _ => new ZapperPosition(103, 103);

        core.RunFrame(0);
        core.RunFrame(0);

        Assert.Equal(0x00, core.Input.Read(1, 0x00));
    }

    [Theory] // AMD-021 R2-006 criterion (a)
    [InlineData(0x0F, false)] // dark backdrop: no light, even over a white sprite
    [InlineData(0x30, true)]  // light backdrop: light
    public void AMD_021_R2_006_a_vblank_read_sees_only_the_universal_backdrop(
        byte backdrop,
        bool expected)
    {
        TileFixture fixture = WhiteSpriteOnBlack();
        fixture.WriteVram(0x3F00, backdrop);

        NesCore core = fixture.Core;
        core.RunFrame(0);
        core.RunFrame(0);

        // RunFrame returns at the start of the vblank, so the records of the frame just drawn
        // are complete — and no longer consulted (R2-006 supersedes Q-005 point 1).
        Assert.True(core.Ppu.Scanline >= Ppu.Ppu2C02.ScreenHeight);
        Assert.Equal(backdrop, core.Ppu.UniversalBackdropColor);
        Assert.Equal(0x30, core.Ppu.ComposeRecordedColor(100, 100));
        Assert.Equal(expected, core.Ppu.SeesLightAt(100, 100));
    }

    [Theory] // AMD-021 R2-006 criterion (b)
    [InlineData(0x0F, false)] // dark backdrop: no light, whatever is displayed there
    [InlineData(0x30, true)]  // light backdrop: light
    public void AMD_021_R2_006_the_right_hand_band_is_always_the_universal_backdrop(
        byte backdrop,
        bool expected)
    {
        TileFixture fixture = WhiteSpriteOnBlack();
        fixture.WriteVram(0x3F00, backdrop);

        // A second white sprite covering (240, 100), so the band is not empty.
        fixture.SetSprite(1, 99, SolidTile, 0x00, 240);

        NesCore core = fixture.Core;
        core.RunFrame(0);

        // Walk into the next frame until the scan is past the capture of the sprite band of
        // line 100, which REQ-CORE-044 places on line 103.
        int guard = 0;
        while (core.Ppu.Scanline is < 104 or >= Ppu.Ppu2C02.ScreenHeight && guard++ < 400_000)
        {
            core.Cpu.Step();
        }

        Assert.True(core.Ppu.Scanline is >= 104 and < Ppu.Ppu2C02.ScreenHeight);

        // Left of the band the records still count: the white sprite at x = 100 is seen.
        Assert.True(core.Ppu.SeesLightAt(103, 100));

        // From x = 239 on, only the backdrop counts, even though a white sprite is there.
        Assert.Equal(0x30, core.Ppu.ComposeRecordedColor(240, 100));
        Assert.Equal(expected, core.Ppu.SeesLightAt(239, 100));
        Assert.Equal(expected, core.Ppu.SeesLightAt(240, 100));
        Assert.Equal(expected, core.Ppu.SeesLightAt(255, 100));
    }

    [Fact]
    public void REQ_CORE_061_a_zapper_read_does_not_advance_a_shift_register()
    {
        var fixture = new TileFixture();
        NesCore core = fixture.Core;
        core.SetPortType(1, ControllerPortType.Zapper);
        core.ZapperPositionSource = static _ => ZapperPosition.OffScreen;
        core.RunFrame(0x1000);

        for (int i = 0; i < 16; i++)
        {
            Assert.Equal(0x18, core.Input.Read(1, 0x00));
        }

        // Back to a controller, the serial protocol works again: Start on port 2 is bit 3,
        // so the fourth read is the one that is set (REQ-CORE-022, REQ-CORE-024).
        core.SetPortType(1, ControllerPortType.Controller);
        core.RunFrame(0x0800);
        core.Input.Write(0x01);
        core.Input.Write(0x00);

        Assert.Equal(0x00, core.Input.Read(1, 0x00) & 0x01);
        Assert.Equal(0x00, core.Input.Read(1, 0x00) & 0x01);
        Assert.Equal(0x00, core.Input.Read(1, 0x00) & 0x01);
        Assert.Equal(0x01, core.Input.Read(1, 0x00) & 0x01);
    }

    [Fact]
    public void REQ_CORE_062_light_is_seen_from_the_line_after_the_aimed_pixel()
    {
        TileFixture fixture = WhiteSpriteOnBlack();
        NesCore core = fixture.Core;

        // One frame so that the programmed state is fully in force, then walk through the
        // next frame instruction by instruction and watch the probe line by line.
        core.RunFrame(0);

        while (core.Ppu.Scanline != 0)
        {
            core.Cpu.Step();
        }

        var light = new bool[Ppu.Ppu2C02.ScreenHeight];
        int guard = 0;
        while (core.Ppu.Scanline < Ppu.Ppu2C02.ScreenHeight && guard++ < 200_000)
        {
            light[core.Ppu.Scanline] |= core.Ppu.SeesLightAt(103, 103);
            core.Cpu.Step();
        }

        for (int line = 0; line <= 103; line++)
        {
            Assert.False(light[line], $"line {line} should not see light yet");
        }

        for (int line = 104; line < Ppu.Ppu2C02.ScreenHeight; line++)
        {
            Assert.True(light[line], $"line {line} should see light");
        }
    }

    [Fact]
    public void REQ_CORE_062_a_pointer_outside_the_image_never_sees_light()
    {
        // Condition 1 of REQ-CORE-062 (0 <= x < 256, 0 <= y < 240) is unchanged by R2-006,
        // which only redefines the colour considered inside the image. The backdrop is light
        // here, so the only reason to see no light is the aim being out of the image.
        TileFixture fixture = WhiteSpriteOnBlack();
        fixture.WriteVram(0x3F00, 0x30);

        NesCore core = fixture.Core;
        core.RunFrame(0);
        core.RunFrame(0);

        Assert.False(core.Ppu.SeesLightAt(-1, 103));
        Assert.False(core.Ppu.SeesLightAt(103, -1));
        Assert.False(core.Ppu.SeesLightAt(256, 103));
        Assert.False(core.Ppu.SeesLightAt(103, 240));
        Assert.True(core.Ppu.SeesLightAt(103, 103));
    }

    [Theory]
    [InlineData(true, true, 0x03)]   // the front sprite wins
    [InlineData(false, true, 0x01)]  // no front sprite: the background shows
    [InlineData(false, false, 0x02)] // transparent background: the hidden sprite shows
    public void REQ_CORE_062_the_displayed_colour_follows_the_nes_composition_order(
        bool withFrontSprite,
        bool withSolidBackground,
        byte expected)
    {
        var fixture = new TileFixture();

        fixture.WriteVram(0x3F00, 0x04); // universal backdrop
        fixture.WriteVram(0x3F01, 0x01); // background sub-palette 0, colour 1
        fixture.WriteVram(0x3F15, 0x02); // sprite sub-palette 1, colour 1
        fixture.WriteVram(0x3F19, 0x03); // sprite sub-palette 2, colour 1

        if (withSolidBackground)
        {
            fixture.FillVram(0x2000, 960, SolidTile);
        }

        // Sprite 0 is behind the background, sprite 1 in front of it; both cover (103, 103).
        fixture.SetSprite(0, 99, SolidTile, 0x21, 100);
        if (withFrontSprite)
        {
            fixture.SetSprite(1, 99, SolidTile, 0x02, 100);
        }

        fixture.SetScroll(0, 0);
        fixture.SetControl(0x00);
        fixture.SetMask(0x1E);

        fixture.NextBatch();

        Assert.Equal(expected, fixture.Core.Ppu.ComposeRecordedColor(103, 103));
    }

    [Fact]
    public void REQ_CORE_062_an_empty_frame_composes_to_the_universal_backdrop()
    {
        var fixture = new TileFixture();
        fixture.WriteVram(0x3F00, 0x04);
        fixture.SetMask(0x00);

        fixture.NextBatch();

        Assert.Equal(0x04, fixture.Core.Ppu.ComposeRecordedColor(103, 103));
    }

    [Fact]
    public void REQ_CORE_062_exactly_nine_of_the_64_colours_are_dark()
    {
        int dark = 0;
        for (int color = 0; color < NesColors.Count; color++)
        {
            if (NesColors.IsDark((byte)color))
            {
                dark++;
            }

            Assert.NotEqual(NesColors.IsDark((byte)color), NesColors.IsLight((byte)color));
        }

        Assert.Equal(9, dark);
        Assert.Equal<byte[]>(
            [0x0D, 0x0E, 0x0F, 0x1E, 0x1F, 0x2E, 0x2F, 0x3E, 0x3F],
            [.. NesColors.DarkColors]);

        // The six unused bits of a colour byte are ignored.
        Assert.True(NesColors.IsDark(0x4F));
        Assert.False(NesColors.IsDark(0x70));
    }

    /// <summary>
    /// The unit-test scene of REQ-CORE-062: a black backdrop ($0F) and a white-palette
    /// sprite ($30) whose top-left pixel is at (100, 100).
    /// </summary>
    private static TileFixture WhiteSpriteOnBlack()
    {
        var fixture = new TileFixture();

        fixture.WriteVram(0x3F00, 0x0F); // backdrop: black
        fixture.WriteVram(0x3F11, 0x30); // sprite sub-palette 0, colour 1: white

        // OAM Y is one line above the top of the sprite, so Y = 99 puts it at y = 100.
        fixture.SetSprite(0, 99, SolidTile, 0x00, 100);

        fixture.SetScroll(0, 0);
        fixture.SetControl(0x00);
        fixture.SetMask(0x10);

        return fixture;
    }
}
