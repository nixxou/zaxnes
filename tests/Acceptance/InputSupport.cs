using System.Numerics;
using Silk.NET.Input;
using Zax.App.Input;
using Zax.Platform.Input;

namespace Zax.Acceptance.Tests;

/// <summary>
/// The simulated devices the SPEC-08 verification injects: a keyboard that holds physical key
/// positions, a pointer, a gamepad with raw axis values and a hub whose device list the test
/// changes between two polls (REQ-INP-120, REQ-INP-121).
/// </summary>
/// <remarks>
/// Nothing here comes from the implementation's own test helpers: the devices only implement the
/// three public interfaces of <c>Zax.Platform.Input</c>, which is the seam REQ-INP-121 puts
/// between the host layer and the logic.
/// </remarks>
public sealed class FakeKeyboard : IKeyboardDevice
{
    private readonly HashSet<int> _down = [];

    /// <summary>Presses the key at physical position <paramref name="key"/>.</summary>
    public FakeKeyboard Press(Key key) => Press((int)key);

    /// <summary>Presses the key at physical position <paramref name="code"/>.</summary>
    public FakeKeyboard Press(int code)
    {
        _down.Add(code);
        return this;
    }

    /// <summary>Releases a key.</summary>
    public FakeKeyboard Release(Key key)
    {
        _down.Remove((int)key);
        return this;
    }

    /// <summary>Releases everything.</summary>
    public FakeKeyboard ReleaseAll()
    {
        _down.Clear();
        return this;
    }

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

/// <summary>A mouse the test drives directly.</summary>
public sealed class FakePointer : IPointerDevice
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

    /// <summary>Adds wheel notches, positive forward.</summary>
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
/// A gamepad whose buttons, stick axes and triggers the test sets by hand. The calibration of
/// REQ-INP-031 is the real one: the raw values go through
/// <see cref="AnalogCalibration"/> exactly as a Silk.NET pad's do.
/// </summary>
public sealed class FakeGamepad : IGamepadDevice
{
    private readonly HashSet<int> _buttons = [];
    private readonly float[] _axes = new float[StandardGamepad.AxisCount];
    private readonly float[] _triggers = new float[StandardGamepad.TriggerCount];

    /// <summary>Creates a pad.</summary>
    /// <param name="identity">Its persistence key (REQ-INP-025).</param>
    /// <param name="standard">Whether it answers to the layout of §4.3 (REQ-INP-020).</param>
    public FakeGamepad(string identity, bool standard = true)
    {
        Identity = identity;
        DisplayName = identity;
        IsStandard = standard;
    }

    /// <inheritdoc/>
    public string Identity { get; }

    /// <inheritdoc/>
    public string DisplayName { get; set; }

    /// <inheritdoc/>
    public bool IsStandard { get; }

    /// <summary>Presses a button of <see cref="StandardGamepad"/>.</summary>
    public FakeGamepad Press(int button)
    {
        _buttons.Add(button);
        return this;
    }

    /// <summary>Releases a button.</summary>
    public FakeGamepad Release(int button)
    {
        _buttons.Remove(button);
        return this;
    }

    /// <summary>Sets the raw value of a stick axis, −1 to 1.</summary>
    public FakeGamepad Axis(int axis, float value)
    {
        _axes[axis] = value;
        return this;
    }

    /// <summary>Sets the raw travel of a trigger, 0 to 1.</summary>
    public FakeGamepad Trigger(int index, float value)
    {
        _triggers[index] = value;
        return this;
    }

    /// <summary>Releases every control.</summary>
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
        PhysicalControlKind.Axis when control.Code >= 0 && control.Code < _axes.Length =>
            AnalogCalibration.HalfAxis(_axes[control.Code], _axes[Paired(control.Code)], control.Direction),
        PhysicalControlKind.Trigger when control.Code >= 0 && control.Code < _triggers.Length =>
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

    private static int Paired(int axis) => axis switch
    {
        StandardGamepad.LeftStickX => StandardGamepad.LeftStickY,
        StandardGamepad.LeftStickY => StandardGamepad.LeftStickX,
        StandardGamepad.RightStickX => StandardGamepad.RightStickY,
        _ => StandardGamepad.RightStickX,
    };
}

/// <summary>A hub whose device list the test changes, to exercise plug and unplug.</summary>
public sealed class FakeGamepadHub : IGamepadHub
{
    private readonly List<IGamepadDevice> _devices = [];

    /// <summary>How many times <see cref="Poll"/> ran.</summary>
    public int Polls { get; private set; }

    /// <inheritdoc/>
    public IReadOnlyList<IGamepadDevice> Devices => _devices;

    /// <summary>
    /// Plugs a pad in. VR devices are filtered out exactly as the real hub does, which is what
    /// REQ-INP-022 asks of the host layer.
    /// </summary>
    public FakeGamepadHub Plug(IGamepadDevice device)
    {
        if (!GamepadIdentity.IsVirtualReality(device.Identity))
        {
            _devices.Add(device);
        }

        return this;
    }

    /// <summary>Plugs a device in without the VR filter, to check the filter itself.</summary>
    public FakeGamepadHub PlugUnfiltered(IGamepadDevice device)
    {
        _devices.Add(device);
        return this;
    }

    /// <summary>Unplugs a pad (REQ-INP-024).</summary>
    public FakeGamepadHub Unplug(IGamepadDevice device)
    {
        _devices.Remove(device);
        return this;
    }

    /// <inheritdoc/>
    public void Poll() => Polls++;
}

/// <summary>Shorthands the SPEC-08 tests share.</summary>
public static class InputSupport
{
    /// <summary>A desktop source over the given devices, with the start-up delay already past.</summary>
    public static DesktopInputSource Source(
        InputBindings bindings,
        FakeGamepadHub hub,
        FakeKeyboard? keyboard = null,
        FakePointer? pointer = null)
    {
        DesktopInputSource source = new(bindings, hub)
        {
            Keyboard = keyboard,
            Pointer = pointer,
            RenderTargetSize = new Vector2(640f, 480f),
        };

        source.Pads.SkipStartupDelay();
        return source;
    }

    /// <summary>The mask of <paramref name="player"/> after one sample.</summary>
    public static InputAction Sample(DesktopInputSource source, int player, float seconds = 1f / 60f)
    {
        InputSnapshot snapshot = source.Sample(seconds);
        source.EndSample();
        return snapshot.Player(player);
    }
}
