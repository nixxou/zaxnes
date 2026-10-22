using Zax.Nes.State;

namespace Zax.Nes.Input;

/// <summary>
/// Probes whether the Zapper sees light at <paramref name="position"/> right now
/// (REQ-CORE-062).
/// </summary>
public delegate bool ZapperLightProbe(ZapperPosition position);

/// <summary>
/// The two controller ports, $4016 and $4017 (REQ-CORE-022, REQ-CORE-024, REQ-CORE-061).
/// </summary>
/// <remarks>
/// The buttons come from the 16-bit input word the host passes to "run one frame"; the
/// word stays valid for the whole frame. Bits above $FF of a player's mask are frontend
/// actions (camera, menu, rollback) which the host strips before composing the word, so
/// they never reach here (REQ-CORE-022, SPEC-02 §4.4).
/// </remarks>
public sealed class InputPorts
{
    /// <summary>Number of ports.</summary>
    public const int PortCount = 2;

    private readonly ControllerPortType[] _types = [ControllerPortType.Controller, ControllerPortType.Controller];
    private readonly byte[] _shift = new byte[PortCount];
    private readonly int[] _shiftCount = new int[PortCount];
    private readonly ZapperPosition[] _zapper = [ZapperPosition.OffScreen, ZapperPosition.OffScreen];

    private ushort _inputWord;
    private bool _strobe;

    /// <summary>
    /// Probe used for bit 3 of a Zapper read. The default never sees light.
    /// </summary>
    /// <remarks>
    /// The three conditions of REQ-CORE-062 — aim inside the image, scan already past that
    /// pixel, displayed colour light — are all decided by the PPU, which owns both the scan
    /// position and the tile records the colour is composed from. The ports only turn the
    /// answer into bit 3.
    /// </remarks>
    public ZapperLightProbe LightProbe { get; set; } = static _ => false;

    /// <summary>The 16-bit input word currently in force (SPEC-02 §4.4).</summary>
    public ushort InputWord => _inputWord;

    /// <summary>Returns the device type of <paramref name="port"/> (0 or 1).</summary>
    public ControllerPortType GetPortType(int port)
    {
        ValidatePort(port);
        return _types[port];
    }

    /// <summary>
    /// Sets the device type of <paramref name="port"/>. Allowed at any time between two
    /// frames; the default is a controller on both ports (REQ-CORE-024).
    /// </summary>
    public void SetPortType(int port, ControllerPortType type)
    {
        ValidatePort(port);
        _types[port] = type;
    }

    /// <summary>
    /// Installs the input word for the frame about to run and latches both port bytes
    /// (REQ-CORE-022).
    /// </summary>
    public void BeginFrame(ushort inputWord)
    {
        _inputWord = inputWord;
        if (_strobe)
        {
            // While the strobe is held high the shift registers reload continuously.
            Reload();
        }
    }

    /// <summary>Stores the aimed position of the Zapper on <paramref name="port"/> (REQ-CORE-060).</summary>
    public void SetZapperPosition(int port, ZapperPosition position)
    {
        ValidatePort(port);
        _zapper[port] = position;
    }

    /// <summary>Returns the aimed position latched for <paramref name="port"/>.</summary>
    public ZapperPosition GetZapperPosition(int port)
    {
        ValidatePort(port);
        return _zapper[port];
    }

    /// <summary>The byte of <paramref name="port"/> inside the input word.</summary>
    public byte PortByte(int port)
    {
        ValidatePort(port);
        return (byte)(_inputWord >> (8 * port));
    }

    /// <summary>
    /// Reads $4016 (port 0) or $4017 (port 1).
    /// </summary>
    /// <param name="port">Port number, 0 or 1.</param>
    /// <param name="openBus">Value currently on the data bus, for the unused bits.</param>
    public byte Read(int port, byte openBus)
    {
        ValidatePort(port);

        if (_types[port] == ControllerPortType.Zapper)
        {
            // Bit 4 is the trigger, copied straight from the input word; bit 3 is 0 when
            // light is seen; every other bit is zero (REQ-CORE-061).
            var value = (byte)(PortByte(port) & 0x10);
            if (!LightProbe(_zapper[port]))
            {
                value |= 0x08;
            }

            return value;
        }

        byte bit;
        if (_strobe)
        {
            // Reads taken while the strobe is high always show button A.
            bit = (byte)(_shift[port] & 0x01);
        }
        else if (_shiftCount[port] >= 8)
        {
            // The ninth read and the ones after it return 1 on an official controller
            // (REQ-CORE-022, notes).
            bit = 1;
        }
        else
        {
            bit = (byte)(_shift[port] & 0x01);
            _shift[port] >>= 1;
            _shiftCount[port]++;
        }

        // Bits 5 to 7 of a $4016/$4017 read are the stale bus contents (nesdev).
        return (byte)(bit | (openBus & 0xE0));
    }

    /// <summary>Writes $4016: bit 0 is the strobe (REQ-CORE-022).</summary>
    public void Write(byte value)
    {
        bool strobe = (value & 0x01) != 0;
        if (strobe)
        {
            Reload();
        }

        _strobe = strobe;
    }

    /// <summary>Returns the ports to their power-on state. Port types are kept (REQ-CORE-004).</summary>
    public void Reset()
    {
        _inputWord = 0;
        _strobe = false;
        Array.Clear(_shift);
        Array.Clear(_shiftCount);
        _zapper[0] = ZapperPosition.OffScreen;
        _zapper[1] = ZapperPosition.OffScreen;
    }

    private void Reload()
    {
        for (int port = 0; port < PortCount; port++)
        {
            _shift[port] = PortByte(port);
            _shiftCount[port] = 0;
        }
    }

    private static void ValidatePort(int port)
        => ArgumentOutOfRangeException.ThrowIfGreaterThanOrEqual((uint)port, (uint)PortCount, nameof(port));

    /// <summary>Appends the port state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public void SaveState(StateWriter writer)
    {
        ArgumentNullException.ThrowIfNull(writer);

        writer.WriteUInt16(_inputWord);
        writer.WriteBool(_strobe);
        for (int port = 0; port < PortCount; port++)
        {
            writer.WriteByte((byte)_types[port]);
            writer.WriteByte(_shift[port]);
            writer.WriteInt32(_shiftCount[port]);
            writer.WriteInt32(_zapper[port].X);
            writer.WriteInt32(_zapper[port].Y);
        }
    }

    /// <summary>Restores the port state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        ArgumentNullException.ThrowIfNull(reader);

        _inputWord = reader.ReadUInt16();
        _strobe = reader.ReadBool();
        for (int port = 0; port < PortCount; port++)
        {
            _types[port] = (ControllerPortType)reader.ReadByte();
            _shift[port] = reader.ReadByte();
            _shiftCount[port] = reader.ReadInt32();
            int x = reader.ReadInt32();
            int y = reader.ReadInt32();
            _zapper[port] = new ZapperPosition(x, y);
        }
    }
}
