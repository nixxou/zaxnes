using System.Numerics;
using Zax.Platform.Input;

namespace Zax.App.Tests;

/// <summary>
/// A keyboard driven by the test: the set of physical key positions currently held
/// (SPEC-08 DEC-08-5).
/// </summary>
internal sealed class FakeKeyboard : IKeyboardDevice
{
    private readonly HashSet<int> _down = [];

    /// <summary>Holds the key at position <paramref name="code"/>.</summary>
    public void Press(int code) => _down.Add(code);

    /// <summary>Holds the key at the position of <paramref name="key"/>.</summary>
    public void Press(Silk.NET.Input.Key key) => Press((int)key);

    /// <summary>Releases the key at position <paramref name="code"/>.</summary>
    public void Release(int code) => _down.Remove(code);

    /// <summary>Releases the key at the position of <paramref name="key"/>.</summary>
    public void Release(Silk.NET.Input.Key key) => Release((int)key);

    /// <summary>Releases every key.</summary>
    public void ReleaseAll() => _down.Clear();

    /// <inheritdoc/>
    public bool IsDown(int keyCode) => _down.Contains(keyCode);

    /// <inheritdoc/>
    public void CollectHeldKeys(ICollection<int> keys)
    {
        foreach (int code in _down)
        {
            keys.Add(code);
        }
    }
}

/// <summary>A mouse driven by the test (REQ-INP-120).</summary>
internal sealed class FakePointer : IPointerDevice
{
    private float _wheel;

    /// <inheritdoc/>
    public Vector2 Position { get; set; }

    /// <inheritdoc/>
    public bool LeftButton { get; set; }

    /// <inheritdoc/>
    public bool RightButton { get; set; }

    /// <inheritdoc/>
    public bool CursorVisible { get; set; } = true;

    /// <summary>Adds <paramref name="notches"/> of wheel travel, positive forward.</summary>
    public void Scroll(float notches) => _wheel += notches;

    /// <inheritdoc/>
    public float TakeWheelNotches()
    {
        float notches = _wheel;
        _wheel = 0f;
        return notches;
    }
}

/// <summary>
/// A simulated gamepad: raw button states, raw stick axes and raw triggers, so the dead zones
/// and the 0.34 threshold of SPEC-08 §3.4 are exercised exactly as on a real pad.
/// </summary>
internal sealed class FakeGamepad : IGamepadDevice
{
    private readonly HashSet<int> _buttons = [];
    private readonly float[] _axes = new float[StandardGamepad.AxisCount];
    private readonly float[] _triggers = new float[StandardGamepad.TriggerCount];

    /// <summary>Creates a pad.</summary>
    /// <param name="identity">Its identity, the persistence key of its set (REQ-INP-025).</param>
    /// <param name="standard">Whether it answers to the standard layout (REQ-INP-020).</param>
    public FakeGamepad(string identity, bool standard = true)
    {
        Identity = identity;
        DisplayName = identity;
        IsStandard = standard;
    }

    /// <inheritdoc/>
    public string Identity { get; }

    /// <inheritdoc/>
    public string DisplayName { get; }

    /// <inheritdoc/>
    public bool IsStandard { get; }

    /// <summary>Holds button <paramref name="code"/>.</summary>
    public FakeGamepad Press(int code)
    {
        _buttons.Add(code);
        return this;
    }

    /// <summary>Releases button <paramref name="code"/>.</summary>
    public FakeGamepad Release(int code)
    {
        _buttons.Remove(code);
        return this;
    }

    /// <summary>Sets the raw value of stick axis <paramref name="index"/>, −1 to 1.</summary>
    public FakeGamepad Axis(int index, float value)
    {
        _axes[index] = value;
        return this;
    }

    /// <summary>Sets the raw travel of trigger <paramref name="index"/>, 0 to 1.</summary>
    public FakeGamepad Trigger(int index, float value)
    {
        _triggers[index] = value;
        return this;
    }

    /// <summary>Releases everything.</summary>
    public FakeGamepad ReleaseAll()
    {
        _buttons.Clear();
        Array.Clear(_axes);
        Array.Clear(_triggers);
        return this;
    }

    /// <inheritdoc/>
    public float Value(PhysicalControl control) => control.Kind switch
    {
        PhysicalControlKind.Button => _buttons.Contains(control.Code) ? 1f : 0f,
        PhysicalControlKind.Axis when control.Code < _axes.Length => AnalogCalibration.HalfAxis(
            _axes[control.Code],
            Paired(control.Code),
            control.Direction),
        PhysicalControlKind.Trigger when control.Code < _triggers.Length =>
            AnalogCalibration.Trigger(_triggers[control.Code]),
        _ => 0f,
    };

    /// <inheritdoc/>
    public void CollectActiveControls(ICollection<PhysicalControl> controls)
    {
        for (int button = 0; button < StandardGamepad.ButtonCount; button++)
        {
            if (_buttons.Contains(button))
            {
                controls.Add(PhysicalControl.Button(button));
            }
        }

        for (int axis = 0; axis < StandardGamepad.AxisCount; axis++)
        {
            foreach (int direction in (int[])[1, -1])
            {
                PhysicalControl half = PhysicalControl.Axis(axis, direction);
                if (AnalogCalibration.IsPressed(Value(half)))
                {
                    controls.Add(half);
                }
            }
        }

        for (int trigger = 0; trigger < StandardGamepad.TriggerCount; trigger++)
        {
            PhysicalControl control = PhysicalControl.Trigger(trigger);
            if (AnalogCalibration.IsPressed(Value(control)))
            {
                controls.Add(control);
            }
        }
    }

    private float Paired(int axis) => axis switch
    {
        StandardGamepad.LeftStickX => _axes[StandardGamepad.LeftStickY],
        StandardGamepad.LeftStickY => _axes[StandardGamepad.LeftStickX],
        StandardGamepad.RightStickX => _axes[StandardGamepad.RightStickY],
        StandardGamepad.RightStickY => _axes[StandardGamepad.RightStickX],
        _ => 0f,
    };
}

/// <summary>A temporary user root, deleted when the test is over.</summary>
internal sealed class TemporaryRoot : IDisposable
{
    /// <summary>Creates the folder.</summary>
    public TemporaryRoot()
    {
        Path = System.IO.Path.Combine(
            System.IO.Path.GetTempPath(),
            "zax-input-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(Path);
    }

    /// <summary>The folder.</summary>
    public string Path { get; }

    /// <inheritdoc/>
    public void Dispose()
    {
        try
        {
            Directory.Delete(Path, recursive: true);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            // A locked temporary folder must not fail a test run.
        }
    }
}

/// <summary>A hub whose device list the test plugs and unplugs (REQ-INP-024).</summary>
internal sealed class FakeGamepadHub : IGamepadHub
{
    private readonly List<IGamepadDevice> _devices = [];

    /// <inheritdoc/>
    public IReadOnlyList<IGamepadDevice> Devices => _devices;

    /// <summary>How many times <see cref="Poll"/> ran.</summary>
    public int Polls { get; private set; }

    /// <summary>Plugs <paramref name="device"/> in, unless its identity is a VR one.</summary>
    public FakeGamepadHub Plug(IGamepadDevice device)
    {
        // REQ-INP-022: the hub is what filters VR devices out, so they are never offered.
        if (!GamepadIdentity.IsVirtualReality(device.Identity))
        {
            _devices.Add(device);
        }

        return this;
    }

    /// <summary>Unplugs the device of that identity (REQ-INP-024).</summary>
    public FakeGamepadHub Unplug(string identity)
    {
        _devices.RemoveAll(device =>
            string.Equals(device.Identity, identity, StringComparison.OrdinalIgnoreCase));
        return this;
    }

    /// <inheritdoc/>
    public void Poll() => Polls++;
}
