using Silk.NET.Input;
using Silk.NET.Maths;
using Silk.NET.Windowing;

namespace Zax.Platform;

/// <summary>
/// Owns the application window and its OpenGL 3.3 core context.
/// </summary>
/// <remarks>
/// The GLFW backend is requested explicitly (PLAN.md section 1) rather than letting
/// Silk.NET pick between GLFW and SDL, so that the context and input behaviour are the
/// same on every machine.
/// </remarks>
public sealed class HostWindow : IDisposable
{
    private IInputContext? _input;
    private IDisplaySurface? _display;
    private IWindowIconTarget? _icon;
    private bool _disposed;

    private HostWindow(IWindow window)
    {
        Window = window;
    }

    /// <summary>The underlying Silk.NET window.</summary>
    public IWindow Window { get; }

    /// <summary>
    /// Creates, but does not yet run, a window with an OpenGL 3.3 core forward-compatible context.
    /// </summary>
    /// <param name="settings">Title, size and vsync preference.</param>
    public static HostWindow Create(HostWindowSettings settings)
    {
        if (settings.Width <= 0 || settings.Height <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(settings), "Window size must be positive.");
        }

        Silk.NET.Windowing.Window.PrioritizeGlfw();

        WindowOptions options = WindowOptions.Default with
        {
            Title = settings.Title,
            Size = new Vector2D<int>(settings.Width, settings.Height),
            API = new GraphicsAPI(
                ContextAPI.OpenGL,
                ContextProfile.Core,
                ContextFlags.ForwardCompatible,
                new APIVersion(3, 3)),
            VSync = settings.IsVSyncEnabled,
            ShouldSwapAutomatically = true,
            IsEventDriven = false,

            // SPEC-06 REQ-SCN-061: the display rate is a user setting (60, 90 or 120). With
            // vsync on the driver paces the swaps, so the cap is left off.
            FramesPerSecond = settings.IsVSyncEnabled || settings.TargetFrameRate <= 0
                ? 0
                : settings.TargetFrameRate,
            UpdatesPerSecond = 0,
        };

        return new HostWindow(Silk.NET.Windowing.Window.Create(options));
    }

    /// <summary>
    /// Returns the input context for this window, creating it on first use.
    /// </summary>
    /// <remarks>Must be called after the window's Load event has fired.</remarks>
    public IInputContext Input
    {
        get
        {
            ObjectDisposedException.ThrowIf(_disposed, this);
            return _input ??= Window.CreateInput();
        }
    }

    /// <summary>
    /// The pacing surface of this window: the swap synchronisation and the frame cap of
    /// REQ-SCN-061.
    /// </summary>
    public IDisplaySurface Display
    {
        get
        {
            ObjectDisposedException.ThrowIf(_disposed, this);
            return _display ??= new SilkDisplaySurface(Window);
        }
    }

    /// <summary>
    /// The window-icon surface of lot ICON-1. The native window must exist, so the earliest
    /// moment it can be driven is the window's Load event.
    /// </summary>
    public IWindowIconTarget Icon
    {
        get
        {
            ObjectDisposedException.ThrowIf(_disposed, this);
            return _icon ??= new WindowIcon.SilkWindowIcon(Window);
        }
    }

    /// <summary>
    /// Puts <c>video.vsync</c> and <c>video.frame_rate</c> in force on this window. The GL
    /// context must exist, so the earliest moment is the window's Load event.
    /// </summary>
    /// <param name="vsync">0 off, 1 every refresh, 2 every other refresh (SPEC-09).</param>
    /// <param name="targetFrameRate">The cap to use when vsync is off; 0 for uncapped.</param>
    public DisplayPacingState ApplyPacing(int vsync, int targetFrameRate)
        => DisplayPacing.Apply(Display, vsync, targetFrameRate);

    /// <summary>Runs the window loop until the window is closed.</summary>
    public void Run()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        Window.Run();
    }

    /// <inheritdoc />
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _input?.Dispose();
        Window.Dispose();
    }

    /// <summary>
    /// The <see cref="IDisplaySurface"/> of a Silk.NET window: <c>IWindow.VSync</c> for the
    /// intervals 0 and 1, <c>IWindow.FramesPerSecond</c> for the cap, and the GL context's own
    /// swap-interval call for the exact interval <c>video.vsync</c> = 2 needs.
    /// </summary>
    private sealed class SilkDisplaySurface(IWindow window) : IDisplaySurface
    {
        /// <inheritdoc/>
        public void SetVSync(bool enabled) => window.VSync = enabled;

        /// <inheritdoc/>
        public void SetFrameCap(int framesPerSecond)
            => window.FramesPerSecond = framesPerSecond > 0 ? framesPerSecond : 0d;

        /// <inheritdoc/>
        public bool TrySetSwapInterval(int interval)
        {
            if (window.GLContext is not { } context)
            {
                return false;
            }

            context.SwapInterval(interval);
            return true;
        }
    }
}
