using System.Globalization;
using Silk.NET.Core;
using Silk.NET.Windowing;

namespace Zax.Platform;

/// <summary>
/// One size of the application icon, decoded: 8-bit RGBA, rows top-down, as
/// <see cref="Silk.NET.Core.RawImage"/> and the platform's window-icon call want it.
/// </summary>
/// <param name="Width">Width in pixels.</param>
/// <param name="Height">Height in pixels.</param>
/// <param name="Pixels">
/// <c>Width × Height × 4</c> bytes, four per pixel in R, G, B, A order, the first row first.
/// </param>
public readonly record struct WindowIconImage(int Width, int Height, byte[] Pixels)
{
    /// <summary>
    /// Whether the buffer really holds <c>Width × Height</c> RGBA pixels. The window-icon call
    /// reads the buffer natively, so a short one would be read past its end.
    /// </summary>
    public bool IsWellFormed =>
        Width > 0 && Height > 0 && Pixels is not null && Pixels.Length == Width * Height * 4;
}

/// <summary>
/// What setting the window icon needs of a window.
/// </summary>
/// <remarks>
/// It exists for the same reason as <see cref="IDisplaySurface"/>: so that the rule — which
/// images are offered, in which order, and what happens when none of them is usable — is
/// testable without a window and without a GL context. The real implementation over Silk.NET is
/// <see cref="HostWindow.Icon"/>; the tests pass a recording double.
/// </remarks>
public interface IWindowIconTarget
{
    /// <summary>
    /// Hands the window the icon, in every size available; the window manager picks the one it
    /// wants. An empty list asks for the platform's default icon back.
    /// </summary>
    /// <param name="images">The sizes, smallest first.</param>
    void SetIcon(IReadOnlyList<WindowIconImage> images);
}

/// <summary>What <see cref="WindowIcon.Apply"/> actually put in force.</summary>
/// <param name="Applied">Whether the window was given an icon at all.</param>
/// <param name="Sizes">The sizes that were handed over, smallest first.</param>
/// <param name="Rejected">How many images were dropped as malformed.</param>
/// <param name="Failure">Why nothing was applied, or <see langword="null"/> on success.</param>
public readonly record struct WindowIconState(
    bool Applied,
    IReadOnlyList<int> Sizes,
    int Rejected,
    string? Failure)
{
    /// <summary>The one log line of the start-up sequence.</summary>
    public string Describe()
    {
        if (!Applied)
        {
            return $"window icon not set: {Failure ?? "no image was usable"}";
        }

        return string.Create(
            CultureInfo.InvariantCulture,
            $"window icon: {Sizes.Count} size(s) {string.Join(", ", Sizes)} px"
                + $"{(Rejected > 0 ? $", {Rejected} unusable image(s) dropped" : string.Empty)}");
    }
}

/// <summary>
/// Puts the application icon on the window (lot ICON-1).
/// </summary>
/// <remarks>
/// The icon is cosmetic: a resource that is missing, truncated or not a PNG must never keep the
/// program from starting. Everything here therefore reports rather than throws, and the caller
/// logs the single line <see cref="WindowIconState.Describe"/> returns and carries on.
/// </remarks>
public static class WindowIcon
{
    /// <summary>
    /// Offers <paramref name="images"/> to <paramref name="target"/>, smallest first, keeping
    /// only the well-formed ones.
    /// </summary>
    /// <param name="target">The window, or <see langword="null"/> when there is none.</param>
    /// <param name="images">The decoded sizes, in any order.</param>
    /// <returns>What was put in force, for the log and the tests.</returns>
    public static WindowIconState Apply(
        IWindowIconTarget? target,
        IReadOnlyList<WindowIconImage>? images)
    {
        if (target is null)
        {
            return new WindowIconState(false, [], 0, "there is no window");
        }

        if (images is null || images.Count == 0)
        {
            return new WindowIconState(false, [], 0, "no icon resource was decoded");
        }

        List<WindowIconImage> usable = [];
        int rejected = 0;
        foreach (WindowIconImage image in images)
        {
            if (image.IsWellFormed)
            {
                usable.Add(image);
            }
            else
            {
                rejected++;
            }
        }

        if (usable.Count == 0)
        {
            return new WindowIconState(false, [], rejected, "every icon resource was malformed");
        }

        // Smallest first: the taskbar and the title bar ask for small sizes, and a window
        // manager that takes the first image rather than the closest match gets a sane one.
        usable.Sort(static (left, right) => (left.Width * left.Height).CompareTo(right.Width * right.Height));

        try
        {
            target.SetIcon(usable);
        }
        catch (Exception exception) when (exception is not OutOfMemoryException)
        {
            // Whatever the backend does with an icon it dislikes, the window is already open and
            // the program keeps running without one.
            return new WindowIconState(false, [], rejected, exception.Message);
        }

        return new WindowIconState(true, [.. usable.Select(static image => image.Width)], rejected, null);
    }

    /// <summary>
    /// The <see cref="IWindowIconTarget"/> of a Silk.NET window: <c>IWindow.SetWindowIcon</c>,
    /// which is <c>glfwSetWindowIcon</c> on the GLFW backend this host asks for.
    /// </summary>
    internal sealed class SilkWindowIcon(IWindow window) : IWindowIconTarget
    {
        /// <inheritdoc/>
        public void SetIcon(IReadOnlyList<WindowIconImage> images)
        {
            ArgumentNullException.ThrowIfNull(images);

            RawImage[] raw = new RawImage[images.Count];
            for (int index = 0; index < images.Count; index++)
            {
                WindowIconImage image = images[index];
                raw[index] = new RawImage(image.Width, image.Height, image.Pixels);
            }

            window.SetWindowIcon(raw);
        }
    }
}
