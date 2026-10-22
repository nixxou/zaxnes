using Zax.Engine3D;
using Zax.Nes;

namespace Zax.Scripting;

/// <summary>
/// Severity of a notification a script raises (SPEC-05 REQ-LUA-093, SPEC-10 REQ-UI-080).
/// </summary>
public enum ScriptNotificationSeverity
{
    /// <summary>Ordinary message.</summary>
    Normal = 0,

    /// <summary>Warning.</summary>
    Warning = 1,

    /// <summary>Error.</summary>
    Error = 2,
}

/// <summary>A notification a script raised through <c>NotiCenter</c> (REQ-LUA-093).</summary>
/// <param name="Message">The text.</param>
/// <param name="Seconds">How long it stays visible; 4 by default.</param>
/// <param name="Severity">Its severity.</param>
public readonly record struct ScriptNotification(
    string Message,
    float Seconds,
    ScriptNotificationSeverity Severity);

/// <summary>
/// Where the runtime writes <c>print</c> output and script errors (REQ-LUA-052, REQ-LUA-061).
/// </summary>
public interface IScriptLog
{
    /// <summary>One <c>print</c> line, channel "Lua", information level (REQ-LUA-052).</summary>
    void Print(string line);

    /// <summary>One diagnostic line: a compilation, chunk or hook error (REQ-LUA-060, 061).</summary>
    void Error(string line);

    /// <summary>One warning: a duplicate script name, for instance (REQ-LUA-003).</summary>
    void Warning(string line);
}

/// <summary>A log that drops everything, the default for a headless runtime.</summary>
public sealed class NullScriptLog : IScriptLog
{
    /// <summary>The single instance.</summary>
    public static NullScriptLog Instance { get; } = new();

    private NullScriptLog()
    {
    }

    /// <inheritdoc/>
    public void Print(string line)
    {
    }

    /// <inheritdoc/>
    public void Error(string line)
    {
    }

    /// <inheritdoc/>
    public void Warning(string line)
    {
    }
}

/// <summary>A log that keeps every line in memory, used by the tests and the harness.</summary>
public sealed class CollectingScriptLog : IScriptLog
{
    private readonly List<string> _prints = [];
    private readonly List<string> _errors = [];
    private readonly List<string> _warnings = [];

    /// <summary>The <c>print</c> lines, in order.</summary>
    public IReadOnlyList<string> Prints => _prints;

    /// <summary>The error lines, in order.</summary>
    public IReadOnlyList<string> Errors => _errors;

    /// <summary>The warning lines, in order.</summary>
    public IReadOnlyList<string> Warnings => _warnings;

    /// <inheritdoc/>
    public void Print(string line) => _prints.Add(line);

    /// <inheritdoc/>
    public void Error(string line) => _errors.Add(line);

    /// <inheritdoc/>
    public void Warning(string line) => _warnings.Add(line);

    /// <summary>Forgets every line.</summary>
    public void Clear()
    {
        _prints.Clear();
        _errors.Clear();
        _warnings.Clear();
    }
}

/// <summary>
/// The camera as scripts see it (SPEC-05 REQ-LUA-088, <c>CameraAPI</c>). The host binds it to
/// the real camera of SPEC-06; <see cref="DetachedScriptCamera"/> is the headless stand-in.
/// </summary>
public abstract class ScriptCamera
{
    /// <summary>Orientation of the current mode in degrees, <c>(pitch, yaw, roll)</c>.</summary>
    public abstract Vec3 Angles { get; set; }

    /// <summary>
    /// Distance of the current mode. The setter bounds it to <c>[0, 512 × SceneScale]</c>
    /// (REQ-LUA-088).
    /// </summary>
    public abstract float Distance { get; set; }

    /// <summary>Mode: 0 = TPV, 1 = FPV.</summary>
    public abstract int Mode { get; set; }

    /// <summary>Aimed point in scene coordinates; <c>(128, 120, 0)</c> by default.</summary>
    public abstract Vec3 LookAt { get; set; }

    /// <summary>The default view of SPEC-06 REQ-SCN-012 (<c>Camera:ResetView()</c>).</summary>
    public abstract void ResetView();

    /// <summary>The upper bound of <see cref="Distance"/>, <c>512 × SceneScale</c>.</summary>
    public virtual float MaxDistance => 512f;
}

/// <summary>
/// A camera that holds its state and nothing else: what the headless runtime, the acceptance
/// harness and the unit tests use.
/// </summary>
public sealed class DetachedScriptCamera : ScriptCamera
{
    private float _distance = 260f;

    /// <inheritdoc/>
    public override Vec3 Angles { get; set; } = new(0.81f, 0.81f, 0f);

    /// <inheritdoc/>
    public override float Distance
    {
        get => _distance;
        set => _distance = Math.Clamp(value, 0f, MaxDistance);
    }

    /// <inheritdoc/>
    public override int Mode { get; set; }

    /// <inheritdoc/>
    public override Vec3 LookAt { get; set; } = new(128f, 120f, 0f);

    /// <summary>How many times <see cref="ResetView"/> was called.</summary>
    public int ResetCount { get; private set; }

    /// <inheritdoc/>
    public override void ResetView()
    {
        ResetCount++;
        Angles = new Vec3(0.81f, 0.81f, 0f);
        LookAt = new Vec3(128f, 120f, 0f);
    }
}

/// <summary>
/// The directional light as scripts see it (SPEC-05 type <c>Lumière</c>, SPEC-09 §4.1.3).
/// </summary>
public abstract class ScriptLight
{
    /// <summary>Propagation direction; <c>(1, −1, 1)</c> by default.</summary>
    public abstract Vec3 Direction { get; set; }

    /// <summary>Colour, white by default; components 0 to 1.</summary>
    public abstract LuaColor Color { get; set; }

    /// <summary>Intensity, 1 by default; a user preference (REQ-PERS-005 point 2).</summary>
    public abstract float Intensity { get; set; }
}

/// <summary>A light that holds its own state (headless default).</summary>
public sealed class DetachedScriptLight : ScriptLight
{
    /// <inheritdoc/>
    public override Vec3 Direction { get; set; } = new(1f, -1f, 1f);

    /// <inheritdoc/>
    public override LuaColor Color { get; set; } = LuaColor.White;

    /// <inheritdoc/>
    public override float Intensity { get; set; } = 1f;
}

/// <summary>
/// The backdrop as scripts see it (SPEC-05 REQ-LUA-090, SPEC-06 REQ-SCN-048).
/// </summary>
public abstract class ScriptBackdrop
{
    /// <summary>Whether the panel is displayed; false by default.</summary>
    public abstract bool Enabled { get; set; }

    /// <summary>Colour of the panel, opaque black by default.</summary>
    public abstract LuaColor Color { get; set; }

    /// <summary>Whether the panel receives shadows; true by default.</summary>
    public abstract bool ReceiveShadow { get; set; }

    /// <summary>Size of the panel, <c>(256, 240, 1)</c> by default.</summary>
    public abstract Vec3 Size { get; set; }

    /// <summary>Offset of the panel centre, zero by default.</summary>
    public abstract Vec3 Offset { get; set; }

    /// <summary>The end-of-game reset of REQ-LUA-090.</summary>
    public abstract void Reset();
}

/// <summary>A backdrop that holds its own state (headless default).</summary>
public sealed class DetachedScriptBackdrop : ScriptBackdrop
{
    /// <inheritdoc/>
    public override bool Enabled { get; set; }

    /// <inheritdoc/>
    public override LuaColor Color { get; set; } = LuaColor.Black;

    /// <inheritdoc/>
    public override bool ReceiveShadow { get; set; } = true;

    /// <inheritdoc/>
    public override Vec3 Size { get; set; } = new(256f, 240f, 1f);

    /// <inheritdoc/>
    public override Vec3 Offset { get; set; }

    /// <inheritdoc/>
    public override void Reset()
    {
        Enabled = false;
        Color = LuaColor.Black;
        ReceiveShadow = true;
        Size = new Vec3(256f, 240f, 1f);
        Offset = default;
    }
}

/// <summary>
/// The inputs as scripts see them: the two players' action masks, the port bytes, the port
/// types and the Zapper aim offset (SPEC-05 REQ-LUA-091, 092, SPEC-08 §4.1).
/// </summary>
public abstract class ScriptInput
{
    /// <summary>Action mask of player <paramref name="index"/> (0 or 1), SPEC-08 REQ-INP-002.</summary>
    public abstract int PlayerState(int index);

    /// <summary>The global action mask: the or of both players' (SPEC-08).</summary>
    public abstract int GlobalState { get; }

    /// <summary>The NES controller byte of <paramref name="port"/> (0 or 1).</summary>
    public abstract int PortByte(int port);

    /// <summary>Device type of <paramref name="port"/>: 0 controller, 1 Zapper.</summary>
    public abstract int GetPortType(int port);

    /// <summary>Sets the device type of both ports (REQ-INP-060).</summary>
    public abstract void SetPortTypes(int first, int second);

    /// <summary>The Zapper aim offset in scene units (SPEC-08 REQ-INP-063).</summary>
    public abstract Vec3 ZapperOffset { get; set; }
}

/// <summary>Inputs that hold their own state (headless default): nothing is ever pressed.</summary>
public sealed class DetachedScriptInput : ScriptInput
{
    private readonly int[] _players = new int[2];
    private readonly int[] _types = new int[2];

    /// <summary>Sets the action mask a player reports.</summary>
    public void SetPlayerState(int index, int mask) => _players[index & 1] = mask;

    /// <inheritdoc/>
    public override int PlayerState(int index) => (uint)index < 2u ? _players[index] : 0;

    /// <inheritdoc/>
    public override int GlobalState => _players[0] | _players[1];

    /// <inheritdoc/>
    public override int PortByte(int port) => (uint)port < 2u ? _players[port] & 0xFF : 0;

    /// <inheritdoc/>
    public override int GetPortType(int port) => (uint)port < 2u ? _types[port] : 0;

    /// <inheritdoc/>
    public override void SetPortTypes(int first, int second)
    {
        _types[0] = first;
        _types[1] = second;
    }

    /// <inheritdoc/>
    public override Vec3 ZapperOffset { get; set; }
}

/// <summary>Inputs backed by a NES core: port types and port bytes come from it.</summary>
public sealed class CoreScriptInput : ScriptInput
{
    private readonly NesCore _core;
    private int _playerOne;
    private int _playerTwo;

    /// <summary>Binds the inputs to <paramref name="core"/>.</summary>
    public CoreScriptInput(NesCore core)
    {
        ArgumentNullException.ThrowIfNull(core);
        _core = core;
    }

    /// <summary>
    /// Publishes the action masks of the frame about to run, the ones the host composed from
    /// the keyboard, the pads and the script pad (SPEC-08 REQ-INP-002).
    /// </summary>
    public void SetPlayerStates(int playerOne, int playerTwo)
    {
        _playerOne = playerOne;
        _playerTwo = playerTwo;
    }

    /// <inheritdoc/>
    public override int PlayerState(int index) => index switch
    {
        0 => _playerOne,
        1 => _playerTwo,
        _ => 0,
    };

    /// <inheritdoc/>
    public override int GlobalState => _playerOne | _playerTwo;

    /// <inheritdoc/>
    public override int PortByte(int port)
        => (uint)port < 2u ? _core.Input.PortByte(port) : 0;

    /// <inheritdoc/>
    public override int GetPortType(int port)
        => (uint)port < 2u ? (int)_core.GetPortType(port) : 0;

    /// <inheritdoc/>
    public override void SetPortTypes(int first, int second)
    {
        _core.SetPortType(0, (ControllerPortType)Math.Clamp(first, 0, 1));
        _core.SetPortType(1, (ControllerPortType)Math.Clamp(second, 0, 1));
    }

    /// <inheritdoc/>
    public override Vec3 ZapperOffset { get; set; }
}

/// <summary>
/// Side-effect-free access to the core's CPU address space (SPEC-05 REQ-LUA-094, SPEC-02
/// REQ-CORE-080).
/// </summary>
public abstract class ScriptMemory
{
    /// <summary>Reads the byte at the 16-bit CPU address <paramref name="address"/>.</summary>
    public abstract byte Read(ushort address);

    /// <summary>Writes a byte at the 16-bit CPU address <paramref name="address"/>.</summary>
    public abstract void Write(ushort address, byte value);
}

/// <summary>A flat 64 KiB scratch space: the headless default.</summary>
public sealed class DetachedScriptMemory : ScriptMemory
{
    private readonly byte[] _bytes = new byte[0x10000];

    /// <inheritdoc/>
    public override byte Read(ushort address) => _bytes[address];

    /// <inheritdoc/>
    public override void Write(ushort address, byte value) => _bytes[address] = value;
}

/// <summary>Memory backed by a NES core (REQ-LUA-094).</summary>
public sealed class CoreScriptMemory : ScriptMemory
{
    private readonly NesCore _core;

    /// <summary>Binds to <paramref name="core"/>.</summary>
    public CoreScriptMemory(NesCore core)
    {
        ArgumentNullException.ThrowIfNull(core);
        _core = core;
    }

    /// <inheritdoc/>
    public override byte Read(ushort address) => _core.ReadMemory(address);

    /// <inheritdoc/>
    public override void Write(ushort address, byte value) => _core.WriteMemory(address, value);
}

/// <summary>
/// The profile metadata a script may read and write for the session (SPEC-05 REQ-LUA-095,
/// SPEC-03 REQ-PROF-022). Writing has no effect on the library display.
/// </summary>
public sealed class ScriptProfileInfo
{
    /// <summary>Title, field 8.</summary>
    public string Title { get; set; } = string.Empty;

    /// <summary>Profile author, field 9.</summary>
    public string Maker { get; set; } = string.Empty;

    /// <summary>Game publisher, field 10.</summary>
    public string GameDev { get; set; } = string.Empty;

    /// <summary>Description, field 11.</summary>
    public string Description { get; set; } = string.Empty;

    /// <summary>Profile identifier, field 7; read only.</summary>
    public string Id { get; set; } = string.Empty;

    /// <summary>Format version, field 2; read only.</summary>
    public string Version { get; set; } = string.Empty;

    /// <summary>True when <see cref="Version"/> is "2" (SPEC-03).</summary>
    public bool IsV2Format => string.Equals(Version, "2", StringComparison.Ordinal);

    /// <summary>
    /// Replaces <see cref="Id"/> with a fresh 12-character identifier (SPEC-03 REQ-PROF-052),
    /// without any other effect.
    /// </summary>
    public void RenewId()
    {
        const string alphabet = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        char[] buffer = new char[12];
        for (int i = 0; i < buffer.Length; i++)
        {
            buffer[i] = alphabet[System.Security.Cryptography.RandomNumberGenerator.GetInt32(alphabet.Length)];
        }

        Id = new string(buffer);
    }
}

/// <summary>
/// Everything the Lua runtime needs from the host besides the conversion engine: the camera,
/// the settings, the backdrop, the inputs, the core memory, the notifications, the profile
/// metadata and the log.
/// </summary>
/// <remarks>
/// Every member has a working default, so a runtime can be created for the acceptance test of
/// REQ-ACC-011 and for the unit tests without a window, a renderer or a ROM.
/// </remarks>
public sealed class ScriptEnvironment
{
    /// <summary>The conversion engine the frame queries act on; required.</summary>
    public required ConversionEngine Engine { get; init; }

    /// <summary>The settings model exposed as <c>Setting</c> (REQ-LUA-089).</summary>
    public ScriptSettings? Settings { get; init; }

    /// <summary>The camera exposed as <c>Camera</c> (REQ-LUA-088).</summary>
    public ScriptCamera Camera { get; init; } = new DetachedScriptCamera();

    /// <summary>The backdrop exposed as <c>Backdrop</c> (REQ-LUA-090).</summary>
    public ScriptBackdrop Backdrop { get; init; } = new DetachedScriptBackdrop();

    /// <summary>The inputs exposed as <c>Gamepad</c> and <c>ScriptPad</c> (REQ-LUA-091, 092).</summary>
    public ScriptInput Input { get; init; } = new DetachedScriptInput();

    /// <summary>The core memory exposed through <c>Script:ReadMem</c> (REQ-LUA-094).</summary>
    public ScriptMemory Memory { get; init; } = new DetachedScriptMemory();

    /// <summary>The profile metadata exposed as <c>Profile</c> (REQ-LUA-095).</summary>
    public ScriptProfileInfo Profile { get; init; } = new();

    /// <summary>Where <c>print</c> and the error messages go (REQ-LUA-052, 061).</summary>
    public IScriptLog Log { get; init; } = NullScriptLog.Instance;

    /// <summary>What <c>NotiCenter.Notify</c> raises (REQ-LUA-093); ignored by default.</summary>
    public Action<ScriptNotification>? Notify { get; init; }
}
