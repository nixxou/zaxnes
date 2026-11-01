using Zax.Settings;

namespace Zax.Harness.Tests;

/// <summary>
/// AMD-055 — the command-line surface of the style options: what <c>render --help</c> declares
/// and what a refused value does to the exit code. None of these tests needs a GL context: a
/// bad option is reported before the capture directory is even created.
/// </summary>
public sealed class Amd055StyleCommandTests
{
    /// <summary>The three options are declared in the usage text of the command.</summary>
    [Fact] // REQ-SCN-078, REQ-SCN-079
    public void The_three_options_are_documented()
    {
        Assert.Contains("--style <id>", RenderExport.Usage, StringComparison.Ordinal);
        Assert.Contains("--style-param <k>=<v>", RenderExport.Usage, StringComparison.Ordinal);
        Assert.Contains("--style-params \"k=v;...\"", RenderExport.Usage, StringComparison.Ordinal);

        // The identifiers of REQ-SCN-078 and the precedence the command applies.
        Assert.Contains("0 or 'classic', 4 or 'enhanced'", RenderExport.Usage, StringComparison.Ordinal);
        Assert.Contains("Style precedence:", RenderExport.Usage, StringComparison.Ordinal);
    }

    /// <summary>
    /// The twelve parameters of REQ-SCN-079 are all named in the usage text, with their range.
    /// </summary>
    [Fact] // REQ-SCN-079
    public void The_twelve_parameters_are_documented()
    {
        foreach ((string id, string _) in SettingsCatalogue.StyleParameterOrder)
        {
            Assert.Contains(id, RenderExport.Usage, StringComparison.Ordinal);
        }

        Assert.Contains("spl.voxel_grid 0 to 3", RenderExport.Usage, StringComparison.Ordinal);
        Assert.Contains("ssr.quality      0 Low to 3 Ultra", RenderExport.Usage, StringComparison.Ordinal);
    }

    /// <summary>
    /// A refused style or parameter stops the command with a non-zero exit code, before
    /// anything is rendered or written.
    /// </summary>
    [Theory] // REQ-SCN-078, REQ-SCN-079
    [InlineData("--style", "2")]
    [InlineData("--style", "classical")]
    [InlineData("--style-param", "nope=1")]
    [InlineData("--style-param", "spl.ao=5")]
    [InlineData("--style-param", "spl.voxel_grid=4")]
    [InlineData("--style-param", "spl.bend_zx=30")]
    [InlineData("--style-params", "spl.ao=2;nope=1")]
    public void A_refused_value_stops_the_command(string option, string value)
    {
        string output = Path.Combine(
            Path.GetTempPath(),
            $"zax-style-{Guid.NewGuid():N}");

        int code = RenderExport.Run(["--out", output, "--frames", "synthetic:framing", option, value]);

        Assert.Equal(2, code);
        Assert.False(Directory.Exists(output));
    }

    /// <summary>An option left without its value is refused the same way.</summary>
    [Theory] // REQ-SCN-078, REQ-SCN-079
    [InlineData("--style")]
    [InlineData("--style-param")]
    [InlineData("--style-params")]
    public void An_option_without_a_value_is_refused(string option)
        => Assert.Equal(2, RenderExport.Run([option]));
}
