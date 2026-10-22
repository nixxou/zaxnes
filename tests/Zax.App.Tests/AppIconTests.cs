using System.Buffers.Binary;
using Zax.App.Branding;
using Zax.Platform;

namespace Zax.App.Tests;

/// <summary>
/// The application icon and logo of lot ICON-1: the Win32 icon resource of <c>Zaxnes.exe</c>,
/// the embedded PNG sizes the window icon is decoded from, and the rule that a missing or
/// broken resource costs a log line and never a start-up.
/// </summary>
public sealed class AppIconTests
{
    /// <summary>An <see cref="IWindowIconTarget"/> that records what it was handed.</summary>
    private sealed class RecordingWindow : IWindowIconTarget
    {
        public int Calls { get; private set; }

        public IReadOnlyList<WindowIconImage> Images { get; private set; } = [];

        /// <inheritdoc/>
        public void SetIcon(IReadOnlyList<WindowIconImage> images)
        {
            Calls++;
            Images = images;
        }
    }

    /// <summary>A window whose icon call fails, the way a backend without one would.</summary>
    private sealed class FailingWindow : IWindowIconTarget
    {
        public int Calls { get; private set; }

        /// <inheritdoc/>
        public void SetIcon(IReadOnlyList<WindowIconImage> images)
        {
            Calls++;
            throw new PlatformNotSupportedException("this backend has no window icon");
        }
    }

    [Fact] // ICON-1 §3
    public void every_embedded_icon_size_decodes_to_a_square_rgba_image()
    {
        // ICON-1 asks for 16, 32, 48 and 256 at the very least; the build embeds the whole set
        // of branding/zaxnes.ico.
        Assert.Contains(16, AppIcon.Sizes);
        Assert.Contains(32, AppIcon.Sizes);
        Assert.Contains(48, AppIcon.Sizes);
        Assert.Contains(256, AppIcon.Sizes);

        IReadOnlyList<WindowIconImage> images = AppIcon.Load(out IReadOnlyList<string> problems);

        Assert.Empty(problems);
        Assert.Equal(AppIcon.Sizes.Count, images.Count);

        foreach (WindowIconImage image in images)
        {
            Assert.Contains(image.Width, AppIcon.Sizes);
            Assert.Equal(image.Width, image.Height);

            // Four bytes per pixel, R, G, B, A, which is what the window-icon call reads.
            Assert.Equal(image.Width * image.Height * 4, image.Pixels.Length);
            Assert.True(image.IsWellFormed);
        }

        // Smallest first, and every size present exactly once.
        Assert.Equal(
            [.. AppIcon.Sizes.OrderBy(static size => size)],
            images.Select(static image => image.Width));

        // The icon is drawn, not blank: at least one pixel is opaque and at least one is not
        // the same colour as the corner, in every size.
        foreach (WindowIconImage image in images)
        {
            Assert.Contains(255, image.Pixels.Where((_, index) => index % 4 == 3).Select(static a => (int)a));
            Assert.True(
                image.Pixels.Distinct().Count() > 2,
                $"the {image.Width} px icon has no content");
        }
    }

    [Fact] // ICON-1 §3
    public void the_window_icon_setter_is_called_once_with_every_decoded_size()
    {
        RecordingWindow window = new();
        List<string> log = [];

        WindowIconState state = AppIcon.Apply(window, log.Add);

        Assert.True(state.Applied);
        Assert.Null(state.Failure);
        Assert.Equal(0, state.Rejected);
        Assert.Equal(1, window.Calls);

        // The window got exactly the images AppIcon decoded, smallest first.
        Assert.Equal([.. AppIcon.Sizes.OrderBy(static size => size)], state.Sizes);
        Assert.Equal(state.Sizes, window.Images.Select(static image => image.Width));
        Assert.All(window.Images, static image => Assert.True(image.IsWellFormed));

        // Logged once, and the line names the sizes.
        Assert.Single(log);
        Assert.Contains("window icon", log[0], StringComparison.Ordinal);
        Assert.Contains("16", log[0], StringComparison.Ordinal);
        Assert.Contains("256", log[0], StringComparison.Ordinal);
    }

    [Fact] // ICON-1 §3
    public void a_broken_resource_is_refused_rather_than_decoded()
    {
        // Not a PNG at all.
        Assert.False(AppIcon.TryDecode("not a png"u8, 32, out _, out string? reason));
        Assert.NotNull(reason);

        // A PNG header followed by nothing: a truncated resource.
        byte[] truncated = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        Assert.False(AppIcon.TryDecode(truncated, 16, out _, out reason));
        Assert.NotNull(reason);

        // Empty.
        Assert.False(AppIcon.TryDecode([], 16, out _, out _));

        // And a real icon offered under the wrong size is refused too, so a resource that was
        // replaced by another image cannot reach the native call with a lying size.
        byte[] real = AppIcon.ReadResource(32)!;
        Assert.True(AppIcon.TryDecode(real, 32, out WindowIconImage good, out _));
        Assert.Equal(32, good.Width);
        Assert.False(AppIcon.TryDecode(real, 48, out _, out reason));
        Assert.Contains("32", reason!, StringComparison.Ordinal);
    }

    [Fact] // ICON-1 §3
    public void nothing_usable_leaves_the_window_alone_without_throwing()
    {
        RecordingWindow window = new();

        // No image at all: the platform keeps its default icon and the call is never made.
        WindowIconState none = WindowIcon.Apply(window, []);
        Assert.False(none.Applied);
        Assert.Equal(0, window.Calls);
        Assert.NotNull(none.Failure);
        Assert.Contains("window icon not set", none.Describe(), StringComparison.Ordinal);

        Assert.False(WindowIcon.Apply(window, null).Applied);
        Assert.Equal(0, window.Calls);

        // Malformed images — a buffer that is not Width × Height RGBA — are dropped, not passed
        // to a native call that would read past their end.
        WindowIconState malformed = WindowIcon.Apply(window, [new WindowIconImage(16, 16, [1, 2, 3])]);
        Assert.False(malformed.Applied);
        Assert.Equal(1, malformed.Rejected);
        Assert.Equal(0, window.Calls);

        // A good size beside a malformed one still goes through, with the bad one counted.
        IReadOnlyList<WindowIconImage> images = AppIcon.Load(out _);
        WindowIconState mixed = WindowIcon.Apply(
            window,
            [new WindowIconImage(8, 8, []), .. images]);
        Assert.True(mixed.Applied);
        Assert.Equal(1, mixed.Rejected);
        Assert.Equal(1, window.Calls);
        Assert.Equal(images.Count, window.Images.Count);

        // And there is no window: still no throw.
        Assert.False(WindowIcon.Apply(null, images).Applied);
    }

    [Fact] // ICON-1 §3
    public void a_window_that_refuses_the_icon_does_not_stop_the_start_up()
    {
        FailingWindow window = new();
        List<string> log = [];

        WindowIconState state = AppIcon.Apply(window, log.Add);

        Assert.Equal(1, window.Calls);
        Assert.False(state.Applied);
        Assert.Equal("this backend has no window icon", state.Failure);

        // One line, and it says why.
        Assert.Single(log);
        Assert.Contains("no window icon", log[0], StringComparison.Ordinal);
    }

    [Fact] // ICON-1 §3
    public void an_assembly_without_the_resources_decodes_nothing_and_still_reports()
    {
        // An assembly that embeds no icon — a trimmed or repackaged build — yields no image, and
        // Load says so for each size instead of throwing.
        Assert.Null(AppIcon.ReadResource(32, typeof(WindowIcon).Assembly));
        Assert.Null(AppIcon.ReadResource(-1));
        Assert.Null(AppIcon.ReadResource(999));

        RecordingWindow window = new();
        List<string> log = [];
        Assert.False(WindowIcon.Apply(window, []).Applied);
        Assert.Empty(log);
        Assert.Equal(0, window.Calls);
    }

    [Fact] // ICON-1 §2
    public void the_executable_icon_is_the_multi_size_ico_of_the_branding_folder()
    {
        string root = RepositoryRoot();

        // §1: the branding folder is in the repository, generator scripts included.
        foreach (string name in (string[])
            ["zaxnes.ico", "zaxnes-icon.svg", "zaxnes-icon-small.svg", "zaxnes-logo.svg",
             "make-icon.py", "make-logo.py"])
        {
            Assert.True(
                File.Exists(Path.Combine(root, "branding", name)),
                $"branding/{name} is missing from the repository");
        }

        // §2: the application project points its Win32 icon resource at that .ico.
        string csproj = File.ReadAllText(Path.Combine(root, "src", "Zax.App", "Zax.App.csproj"));
        Assert.Contains(
            @"<ApplicationIcon>..\..\branding\zaxnes.ico</ApplicationIcon>",
            csproj,
            StringComparison.Ordinal);

        // And the file really is an icon directory carrying the sizes Explorer and the taskbar
        // ask for. The ICONDIR of the public ICO format: reserved 0, type 1, count, then one
        // 16-byte entry per image whose width and height are a single byte, 0 meaning 256.
        byte[] ico = File.ReadAllBytes(Path.Combine(root, "branding", "zaxnes.ico"));
        Assert.Equal(0, BinaryPrimitives.ReadUInt16LittleEndian(ico));
        Assert.Equal(1, BinaryPrimitives.ReadUInt16LittleEndian(ico.AsSpan(2)));

        int count = BinaryPrimitives.ReadUInt16LittleEndian(ico.AsSpan(4));
        List<int> sizes = [];
        for (int entry = 0; entry < count; entry++)
        {
            int at = 6 + (entry * 16);
            int width = ico[at] == 0 ? 256 : ico[at];
            Assert.Equal(width, ico[at + 1] == 0 ? 256 : ico[at + 1]);

            // The image data the entry points at must lie inside the file.
            int length = BinaryPrimitives.ReadInt32LittleEndian(ico.AsSpan(at + 8));
            int offset = BinaryPrimitives.ReadInt32LittleEndian(ico.AsSpan(at + 12));
            Assert.InRange(offset, 6 + (count * 16), ico.Length);
            Assert.InRange(length, 1, ico.Length - offset);

            sizes.Add(width);
        }

        Assert.Equal([16, 24, 32, 48, 64, 128, 256], [.. sizes.Order()]);
    }

    [Fact] // ICON-1 §5, §6
    public void the_readme_shows_the_logo_and_the_notices_carry_the_branding_entry()
    {
        string root = RepositoryRoot();

        // §5: the wordmark at the top of the README, above the description.
        string readme = File.ReadAllText(Path.Combine(root, "README.md"));
        Assert.Contains("branding/zaxnes-logo.svg", readme, StringComparison.Ordinal);
        Assert.True(
            readme.IndexOf("branding/zaxnes-logo.svg", StringComparison.Ordinal)
                < readme.IndexOf("NES emulator that renders games in 3D", StringComparison.Ordinal),
            "the logo must sit above the project description");

        // §6: the licence entry. The icon and the logo were made for the project and travel
        // under its own licence, which is the exception REQ-PUB-002 makes.
        string notices = File.ReadAllText(Path.Combine(root, "ASSETS-LICENSES.md"));
        Assert.Contains("branding/", notices, StringComparison.Ordinal);
        Assert.Contains("zaxnes.ico", notices, StringComparison.Ordinal);
        Assert.Contains("zaxnes-logo.svg", notices, StringComparison.Ordinal);
    }

    [Fact] // ICON-1 §6
    public void the_publish_script_ships_no_new_file_for_the_icon()
    {
        string script = File.ReadAllText(
            Path.Combine(RepositoryRoot(), "build", "publish.ps1"));

        // The icon travels inside Zaxnes.exe — the Win32 resource and the embedded PNGs — so the
        // script copies nothing for it. Only its comments may name the branding folder.
        string[] code = [.. script
            .Split('\n')
            .Select(static line => line.TrimEnd('\r').Trim())
            .Where(static line => line.Length > 0 && !line.StartsWith('#'))];

        Assert.DoesNotContain(
            code,
            static line => line.Contains("branding", StringComparison.OrdinalIgnoreCase));

        // And its REQ-PUB-002 sweep still refuses a stray .png or .ico in the output.
        Assert.Contains("'.png'", script, StringComparison.Ordinal);
        Assert.Contains("'.ico'", script, StringComparison.Ordinal);
    }

    private static string RepositoryRoot()
    {
        DirectoryInfo? at = new(AppContext.BaseDirectory);
        while (at is not null)
        {
            if (File.Exists(Path.Combine(at.FullName, "Zax.sln")))
            {
                return at.FullName;
            }

            at = at.Parent;
        }

        throw new InvalidOperationException("the repository root was not found above the test binaries.");
    }
}
