using Zax.Nes.Input;

namespace Zax.Nes.Tests;

/// <summary>
/// Controller ports, the input word and the Zapper byte (REQ-CORE-022, REQ-CORE-024,
/// REQ-CORE-060 to REQ-CORE-062).
/// </summary>
public sealed class InputPortTests
{
    /// <summary>
    /// A ROM that strobes $4016 and then reads it eight times, storing the bits at
    /// $0300..$0307. Repeats for $4017 into $0310..$0317, then loops.
    /// </summary>
    private static byte[] ControllerReaderProgram()
    {
        var program = new List<byte>
        {
            // LDA #$01 / STA $4016 / LDA #$00 / STA $4016   — strobe the controllers.
            0xA9, 0x01, 0x8D, 0x16, 0x40, 0xA9, 0x00, 0x8D, 0x16, 0x40,
        };

        for (int i = 0; i < 8; i++)
        {
            // LDA $4016 / AND #$01 / STA $0300+i
            program.AddRange([0xAD, 0x16, 0x40, 0x29, 0x01, 0x8D, (byte)(0x00 + i), 0x03]);
        }

        for (int i = 0; i < 8; i++)
        {
            // LDA $4017 / AND #$01 / STA $0310+i
            program.AddRange([0xAD, 0x17, 0x40, 0x29, 0x01, 0x8D, (byte)(0x10 + i), 0x03]);
        }

        // JMP $8000
        program.AddRange([0x4C, 0x00, 0x80]);
        return [.. program];
    }

    [Fact]
    public void REQ_CORE_022_input_word_0x0081_reads_a_then_right_on_port_one()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(0x01, ControllerReaderProgram()), "Pads.nes").Success);

        core.RunFrame(0x0081);

        // Serial order: A, B, Select, Start, Up, Down, Left, Right.
        byte[] expected = [1, 0, 0, 0, 0, 0, 0, 1];
        for (int i = 0; i < 8; i++)
        {
            Assert.Equal(expected[i], core.ReadMemory((ushort)(0x0300 + i)));
        }
    }

    [Fact]
    public void REQ_CORE_022_input_word_0x0800_presses_start_on_port_two()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(0x01, ControllerReaderProgram()), "Pads.nes").Success);

        core.RunFrame(0x0800);

        // Bit 11 is bit 3 of port 2's byte: Start, the fourth bit read on $4017.
        byte[] expected = [0, 0, 0, 1, 0, 0, 0, 0];
        for (int i = 0; i < 8; i++)
        {
            Assert.Equal(expected[i], core.ReadMemory((ushort)(0x0310 + i)));
        }

        // Port 1 saw nothing.
        for (int i = 0; i < 8; i++)
        {
            Assert.Equal(0, core.ReadMemory((ushort)(0x0300 + i)));
        }
    }

    [Fact]
    public void REQ_CORE_022_the_ninth_read_and_the_ones_after_it_return_one()
    {
        var ports = new InputPorts();
        ports.BeginFrame(0x0000);
        ports.Write(0x01);
        ports.Write(0x00);

        for (int i = 0; i < 8; i++)
        {
            Assert.Equal(0, ports.Read(0, 0) & 0x01);
        }

        for (int i = 0; i < 4; i++)
        {
            Assert.Equal(1, ports.Read(0, 0) & 0x01);
        }
    }

    [Fact]
    public void REQ_CORE_022_frontend_action_bits_never_reach_the_core()
    {
        // The host strips the bits above $FF of a player's mask before composing the word
        // (SPEC-02 §4.4); the core only ever sees 16 bits, 8 per port.
        var ports = new InputPorts();
        ports.BeginFrame(0xFFFF);

        Assert.Equal(0xFF, ports.PortByte(0));
        Assert.Equal(0xFF, ports.PortByte(1));
    }

    [Fact]
    public void REQ_CORE_024_ports_default_to_controllers_and_can_be_switched()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        Assert.Equal(ControllerPortType.Controller, core.GetPortType(0));
        Assert.Equal(ControllerPortType.Controller, core.GetPortType(1));

        core.SetPortType(1, ControllerPortType.Zapper);
        Assert.Equal(ControllerPortType.Zapper, core.GetPortType(1));

        core.SetPortType(1, ControllerPortType.Controller);
        Assert.Equal(ControllerPortType.Controller, core.GetPortType(1));
    }

    [Fact]
    public void REQ_CORE_024_an_out_of_range_port_is_rejected()
    {
        var core = new NesCore();

        Assert.Throws<ArgumentOutOfRangeException>(() => core.GetPortType(2));
        Assert.Throws<ArgumentOutOfRangeException>(
            () => core.SetPortType(2, ControllerPortType.Zapper));
    }

    [Fact]
    public void REQ_CORE_061_zapper_byte_is_trigger_in_bit_four_and_light_in_bit_three()
    {
        var ports = new InputPorts();
        ports.SetPortType(1, ControllerPortType.Zapper);

        // Trigger released (bit 4 = 1), aim off screen: no light, so bit 3 is 1 → $18.
        ports.BeginFrame(0x1000);
        ports.SetZapperPosition(1, ZapperPosition.OffScreen);
        Assert.Equal(0x18, ports.Read(1, 0xFF));

        // Trigger pressed (bit 4 = 0), light seen: both bits clear → $00.
        ports.BeginFrame(0x0000);
        ports.LightProbe = static _ => true;
        Assert.Equal(0x00, ports.Read(1, 0xFF));

        // Trigger released and light seen → $10.
        ports.BeginFrame(0x1000);
        Assert.Equal(0x10, ports.Read(1, 0xFF));
    }

    [Fact]
    public void REQ_CORE_061_a_zapper_read_does_not_advance_a_shift_register()
    {
        var ports = new InputPorts();
        ports.SetPortType(1, ControllerPortType.Zapper);
        ports.BeginFrame(0x1000);

        for (int i = 0; i < 20; i++)
        {
            Assert.Equal(0x18, ports.Read(1, 0x00));
        }
    }

    [Fact]
    public void REQ_CORE_024_switching_back_to_a_controller_restores_serial_reads()
    {
        var ports = new InputPorts();
        ports.SetPortType(1, ControllerPortType.Zapper);
        ports.BeginFrame(0x0100); // Port 2 byte = $01: button A pressed, trigger pressed.

        // Bit 4 of $01 is clear, so the Zapper byte is just "no light".
        Assert.Equal(0x08, ports.Read(1, 0x00));

        ports.SetPortType(1, ControllerPortType.Controller);
        ports.Write(0x01);
        ports.Write(0x00);

        Assert.Equal(1, ports.Read(1, 0x00) & 0x01); // A.
        Assert.Equal(0, ports.Read(1, 0x00) & 0x01); // B.
    }

    [Fact]
    public void REQ_CORE_060_the_position_is_sampled_once_at_the_start_of_the_frame()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);
        core.SetPortType(1, ControllerPortType.Zapper);

        int frame = 0;
        int queries = 0;
        core.ZapperPositionSource = port =>
        {
            Assert.Equal(1, port); // Only the Zapper port is queried.
            queries++;
            return new ZapperPosition(frame, frame + 1);
        };

        for (frame = 0; frame < 5; frame++)
        {
            core.RunFrame(0);
            Assert.Equal(new ZapperPosition(frame, frame + 1), core.Input.GetZapperPosition(1));
        }

        Assert.Equal(5, queries);
    }

    [Fact]
    public void REQ_CORE_062_exactly_nine_of_the_sixty_four_colours_are_dark()
    {
        // SPEC-02 §5.4 asks for this to be checked exhaustively.
        var dark = new List<byte>();
        for (int color = 0; color < NesColors.Count; color++)
        {
            if (NesColors.IsDark((byte)color))
            {
                dark.Add((byte)color);
            }

            Assert.NotEqual(NesColors.IsDark((byte)color), NesColors.IsLight((byte)color));
        }

        Assert.Equal(9, dark.Count);
        Assert.Equal<byte[]>([0x0D, 0x0E, 0x0F, 0x1E, 0x1F, 0x2E, 0x2F, 0x3E, 0x3F], [.. dark]);
        Assert.Equal<byte[]>([.. NesColors.DarkColors], [.. dark]);
    }

    [Fact]
    public void REQ_CORE_062_the_aim_must_be_inside_the_image()
    {
        Assert.True(new ZapperPosition(0, 0).IsOnScreen);
        Assert.True(new ZapperPosition(255, 239).IsOnScreen);
        Assert.False(new ZapperPosition(256, 0).IsOnScreen);
        Assert.False(new ZapperPosition(0, 240).IsOnScreen);
        Assert.False(new ZapperPosition(-1, 0).IsOnScreen);
        Assert.False(ZapperPosition.OffScreen.IsOnScreen);
    }
}
