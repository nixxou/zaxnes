using System.Numerics;
using Silk.NET.Input;

namespace Zax.Platform.Input;

/// <summary>
/// A <see cref="IKeyboardDevice"/> over a Silk.NET keyboard.
/// </summary>
/// <remarks>
/// Silk.NET's <see cref="Key"/> values are the GLFW key codes, which already denote physical
/// positions rather than characters: the key labelled <c>W</c> on an AZERTY keyboard reports
/// <see cref="Key.Z"/>, which is the behaviour DEC-08-5 asks for.
/// </remarks>
public sealed class SilkKeyboard : IKeyboardDevice
{
    private static readonly Key[] AllKeys = Enum.GetValues<Key>();

    private readonly IKeyboard _keyboard;

    /// <summary>Wraps <paramref name="keyboard"/>.</summary>
    public SilkKeyboard(IKeyboard keyboard)
    {
        ArgumentNullException.ThrowIfNull(keyboard);
        _keyboard = keyboard;
    }

    /// <inheritdoc/>
    public bool IsDown(int keyCode)
    {
        if (keyCode <= 0)
        {
            return false;
        }

        try
        {
            return _keyboard.IsKeyPressed((Key)keyCode);
        }
        catch (ArgumentOutOfRangeException)
        {
            // A code that is not part of this backend's enumeration simply is not pressed; a
            // binding file written on another backend must not throw (REQ-INP-052).
            return false;
        }
    }

    /// <inheritdoc/>
    public void CollectHeldKeys(ICollection<int> keys)
    {
        ArgumentNullException.ThrowIfNull(keys);
        foreach (Key key in AllKeys)
        {
            if (key == Key.Unknown)
            {
                continue;
            }

            try
            {
                if (_keyboard.IsKeyPressed(key))
                {
                    keys.Add((int)key);
                }
            }
            catch (ArgumentOutOfRangeException)
            {
                // Not a key this backend knows how to probe.
            }
        }
    }

    /// <summary>The physical key code of a Silk.NET key, for the default tables of §4.2.</summary>
    public static int CodeOf(Key key) => (int)key;
}

/// <summary>A <see cref="IPointerDevice"/> over a Silk.NET mouse.</summary>
public sealed class SilkPointer : IPointerDevice
{
    private readonly IMouse _mouse;
    private float _wheel;

    /// <summary>Wraps <paramref name="mouse"/> and starts listening to its wheel.</summary>
    public SilkPointer(IMouse mouse)
    {
        ArgumentNullException.ThrowIfNull(mouse);
        _mouse = mouse;
        _mouse.Scroll += OnScroll;
        CursorVisible = true;
    }

    /// <inheritdoc/>
    public Vector2 Position => new(_mouse.Position.X, _mouse.Position.Y);

    /// <inheritdoc/>
    public bool LeftButton => _mouse.IsButtonPressed(MouseButton.Left);

    /// <inheritdoc/>
    public bool RightButton => _mouse.IsButtonPressed(MouseButton.Right);

    /// <inheritdoc/>
    public bool CursorVisible
    {
        get => _mouse.Cursor.CursorMode != CursorMode.Hidden;
        set => _mouse.Cursor.CursorMode = value ? CursorMode.Normal : CursorMode.Hidden;
    }

    /// <inheritdoc/>
    public float TakeWheelNotches()
    {
        float notches = _wheel;
        _wheel = 0f;
        return notches;
    }

    /// <summary>Stops listening to the wheel and shows the cursor again (REQ-INP-111).</summary>
    public void Detach()
    {
        _mouse.Scroll -= OnScroll;
        CursorVisible = true;
    }

    private void OnScroll(IMouse _, ScrollWheel wheel) => _wheel += wheel.Y;
}

/// <summary>
/// A <see cref="IGamepadDevice"/> over a Silk.NET gamepad, which is the standard Xbox-style
/// mapping of REQ-INP-020.
/// </summary>
public sealed class SilkGamepad : IGamepadDevice
{
    private readonly IGamepad _gamepad;

    /// <summary>Wraps <paramref name="gamepad"/>.</summary>
    public SilkGamepad(IGamepad gamepad)
    {
        ArgumentNullException.ThrowIfNull(gamepad);
        _gamepad = gamepad;
        DisplayName = string.IsNullOrWhiteSpace(gamepad.Name) ? "Gamepad" : gamepad.Name;

        // The identity is the persistence key of the pad's binding set (REQ-INP-025). Silk.NET
        // exposes the readable name but no serial number, so two identical pads share one set,
        // which is what the original does as well ("a model or an instance", SPEC-08 §2).
        Identity = DisplayName;
    }

    /// <inheritdoc/>
    public string Identity { get; }

    /// <inheritdoc/>
    public string DisplayName { get; }

    /// <inheritdoc/>
    public bool IsStandard => true;

    /// <summary>The Silk.NET device behind this pad.</summary>
    public IGamepad Device => _gamepad;

    /// <inheritdoc/>
    public float Value(PhysicalControl control) => control.Kind switch
    {
        PhysicalControlKind.Button => IsButtonDown(control.Code) ? 1f : 0f,
        PhysicalControlKind.Axis => AnalogCalibration.HalfAxis(
            RawAxis(control.Code),
            RawAxis(PairedAxis(control.Code)),
            control.Direction),
        PhysicalControlKind.Trigger => AnalogCalibration.Trigger(RawTrigger(control.Code)),
        _ => 0f,
    };

    /// <inheritdoc/>
    public void CollectActiveControls(ICollection<PhysicalControl> controls)
    {
        ArgumentNullException.ThrowIfNull(controls);

        for (int button = 0; button < StandardGamepad.ButtonCount; button++)
        {
            if (IsButtonDown(button))
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

    /// <summary>The other axis of the same stick, for the radial dead zone of REQ-INP-031.</summary>
    private static int PairedAxis(int axis) => axis switch
    {
        StandardGamepad.LeftStickX => StandardGamepad.LeftStickY,
        StandardGamepad.LeftStickY => StandardGamepad.LeftStickX,
        StandardGamepad.RightStickX => StandardGamepad.RightStickY,
        StandardGamepad.RightStickY => StandardGamepad.RightStickX,
        _ => -1,
    };

    private bool IsButtonDown(int code)
    {
        ButtonName? name = ButtonNameOf(code);
        if (name is null)
        {
            return false;
        }

        foreach (Button button in _gamepad.Buttons)
        {
            if (button.Name == name)
            {
                return button.Pressed;
            }
        }

        return false;
    }

    private float RawAxis(int index)
    {
        // Silk.NET reports one thumbstick object per stick, with X to the right and Y downwards;
        // the scene frame and SPEC-08 §4.3 both count the vertical axis upwards.
        int stick = index is StandardGamepad.LeftStickX or StandardGamepad.LeftStickY ? 0 : 1;
        bool vertical = index is StandardGamepad.LeftStickY or StandardGamepad.RightStickY;
        if (index is < 0 or >= StandardGamepad.AxisCount || _gamepad.Thumbsticks.Count <= stick)
        {
            return 0f;
        }

        Thumbstick value = _gamepad.Thumbsticks[stick];
        return vertical ? -value.Y : value.X;
    }

    private float RawTrigger(int index)
    {
        if (index < 0 || _gamepad.Triggers.Count <= index)
        {
            return 0f;
        }

        return _gamepad.Triggers[index].Position;
    }

    private static ButtonName? ButtonNameOf(int code) => code switch
    {
        StandardGamepad.South => ButtonName.A,
        StandardGamepad.East => ButtonName.B,
        StandardGamepad.West => ButtonName.X,
        StandardGamepad.North => ButtonName.Y,
        StandardGamepad.LeftBumper => ButtonName.LeftBumper,
        StandardGamepad.RightBumper => ButtonName.RightBumper,
        StandardGamepad.Back => ButtonName.Back,
        StandardGamepad.Start => ButtonName.Start,
        StandardGamepad.Guide => ButtonName.Home,
        StandardGamepad.LeftStick => ButtonName.LeftStick,
        StandardGamepad.RightStick => ButtonName.RightStick,
        StandardGamepad.DPadUp => ButtonName.DPadUp,
        StandardGamepad.DPadRight => ButtonName.DPadRight,
        StandardGamepad.DPadDown => ButtonName.DPadDown,
        StandardGamepad.DPadLeft => ButtonName.DPadLeft,
        _ => null,
    };
}

/// <summary>
/// A <see cref="IGamepadDevice"/> over a Silk.NET joystick: a pad with no standard mapping, whose
/// raw buttons and axes can still be bound by remapping (REQ-INP-021).
/// </summary>
public sealed class SilkJoystick : IGamepadDevice
{
    private readonly IJoystick _joystick;

    /// <summary>Wraps <paramref name="joystick"/>.</summary>
    public SilkJoystick(IJoystick joystick)
    {
        ArgumentNullException.ThrowIfNull(joystick);
        _joystick = joystick;
        DisplayName = string.IsNullOrWhiteSpace(joystick.Name) ? "Joystick" : joystick.Name;
        Identity = DisplayName;
    }

    /// <inheritdoc/>
    public string Identity { get; }

    /// <inheritdoc/>
    public string DisplayName { get; }

    /// <inheritdoc/>
    public bool IsStandard => false;

    /// <inheritdoc/>
    public float Value(PhysicalControl control) => control.Kind switch
    {
        PhysicalControlKind.Button => IsButtonDown(control.Code) ? 1f : 0f,

        // A generic axis has no known pair, so the dead zone is applied to that axis alone.
        PhysicalControlKind.Axis => AnalogCalibration.HalfAxis(RawAxis(control.Code), 0f, control.Direction),
        _ => 0f,
    };

    /// <inheritdoc/>
    public void CollectActiveControls(ICollection<PhysicalControl> controls)
    {
        ArgumentNullException.ThrowIfNull(controls);

        for (int button = 0; button < _joystick.Buttons.Count; button++)
        {
            if (_joystick.Buttons[button].Pressed)
            {
                controls.Add(PhysicalControl.Button(button));
            }
        }

        for (int axis = 0; axis < _joystick.Axes.Count; axis++)
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
    }

    private bool IsButtonDown(int code)
        => code >= 0 && code < _joystick.Buttons.Count && _joystick.Buttons[code].Pressed;

    private float RawAxis(int index)
        => index >= 0 && index < _joystick.Axes.Count ? _joystick.Axes[index].Position : 0f;
}

/// <summary>
/// The gamepad hub over a Silk.NET input context: the connected non-VR pads, refreshed on every
/// <see cref="Poll"/> so that a plug or an unplug is visible at the next frame (REQ-INP-024,
/// REQ-INP-041).
/// </summary>
public sealed class SilkGamepadHub : IGamepadHub
{
    private readonly IInputContext _context;
    private readonly List<IGamepadDevice> _devices = [];

    /// <summary>Creates the hub over <paramref name="context"/>.</summary>
    public SilkGamepadHub(IInputContext context)
    {
        ArgumentNullException.ThrowIfNull(context);
        _context = context;
        Poll();
    }

    /// <inheritdoc/>
    public IReadOnlyList<IGamepadDevice> Devices => _devices;

    /// <inheritdoc/>
    public void Poll()
    {
        _devices.Clear();

        foreach (IGamepad gamepad in _context.Gamepads)
        {
            if (!gamepad.IsConnected || GamepadIdentity.IsVirtualReality(gamepad.Name))
            {
                continue;
            }

            _devices.Add(new SilkGamepad(gamepad));
        }

        foreach (IJoystick joystick in _context.Joysticks)
        {
            if (!joystick.IsConnected || GamepadIdentity.IsVirtualReality(joystick.Name))
            {
                continue;
            }

            // A device that also appears as a gamepad is already mapped; the raw joystick view
            // of it would be a duplicate in the chooser of REQ-INP-041.
            if (IsAlsoGamepad(joystick))
            {
                continue;
            }

            _devices.Add(new SilkJoystick(joystick));
        }
    }

    private bool IsAlsoGamepad(IJoystick joystick)
    {
        foreach (IGamepad gamepad in _context.Gamepads)
        {
            if (gamepad.IsConnected && string.Equals(gamepad.Name, joystick.Name, StringComparison.Ordinal))
            {
                return true;
            }
        }

        return false;
    }
}
