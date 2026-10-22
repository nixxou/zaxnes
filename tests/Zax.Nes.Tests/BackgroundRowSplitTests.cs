using Xunit.Abstractions;
using Xunit.Sdk;

namespace Zax.Nes.Tests;

/// <summary>
/// A <see cref="FactAttribute"/> reported as skipped, naming the file, unless the user ROM it
/// lists is present under the folder <see cref="UserRomFactAttribute.Variable"/> names.
/// </summary>
/// <remarks>
/// The [ROM] criteria of SPEC-12 §2 name commercial games, which this repository does not
/// carry, and no machine path is written anywhere in it (lot PUB-1): the folder is named by the
/// environment variable <c>ZAXNES_ROMS</c>, documented in <c>HOWTO-RUN.md</c> §12. The same
/// rule as <see cref="RomFactAttribute"/> applies: the gap must show up in the test report
/// rather than pass silently.
/// </remarks>
[XunitTestCaseDiscoverer("Xunit.Sdk.FactDiscoverer", "xunit.execution.dotnet")]
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
public sealed class UserRomFactAttribute : FactAttribute
{
    /// <summary>Environment variable naming the folder the user keeps their dumps in.</summary>
    public const string Variable = "ZAXNES_ROMS";

    /// <summary>
    /// Where the user ROMs live (REQ-ACC-020), read from <see cref="Variable"/>, or null when
    /// the variable is not set.
    /// </summary>
    public static string? UserRoms { get; } =
        Environment.GetEnvironmentVariable(Variable) is { } value && value.Trim().Length > 0
            ? value.Trim()
            : null;

    /// <summary>Requires <paramref name="fileName"/> under <see cref="UserRoms"/>.</summary>
    public UserRomFactAttribute(string fileName)
    {
        if (UserRoms is null)
        {
            Skip = $"no user ROM folder: set {Variable} to the folder holding your own dumps.";
        }
        else if (!File.Exists(Path(fileName)))
        {
            Skip = $"User ROM not available: {Path(fileName)}.";
        }
    }

    /// <summary>Full path of the user ROM <paramref name="fileName"/>.</summary>
    /// <exception cref="InvalidOperationException"><see cref="Variable"/> is not set.</exception>
    public static string Path(string fileName)
        => UserRoms is null
            ? throw new InvalidOperationException($"{Variable} is not set.")
            : System.IO.Path.Combine(UserRoms, fileName);
}

/// <summary>
/// Criterion 2 of AMD-021 R2-005, measured on a user ROM: a mid-line <c>$2006</c> write must
/// not add a background row.
/// </summary>
/// <remarks>
/// <para>
/// Super Mario Bros. 3 splits its screen from the MMC3 IRQ of line 193, writing $2006 in the
/// horizontal blanking of that line to put the status bar in place. Before R2-005 the row of
/// line 193 was captured from the state at the <em>start</em> of the line, so the frame
/// carried the playfield row of line 193 <em>and</em> the first banner row of line 195 — two
/// rows two lines apart, one more than the thirty lines of fine Y 1 an NTSC frame holds. With
/// the capture moved to the end of the line, line 193 reflects the written address and emits
/// nothing.
/// </para>
/// <para>
/// Measured over 5 000 frames of the title screen and the attract demo: 4 976 frames at 30
/// rows, 10 at 31, 13 empty (rendering still off at power-on) and one at 3. Before R2-005 the
/// same run gave 4 734 frames at 31 rows and only 252 at 30. The ten remaining frames are the
/// ones in which the vertical scroll puts the playfield grid on lines 0, 8 … 192 — the split
/// line 193 is then not a grid line, so suppressing its row removes nothing and the frame
/// legitimately holds 25 playfield rows plus 6 banner rows. See Q-059: the clause "does not
/// exceed that of an image without a split" cannot hold in that phase, whatever the instant
/// of capture.
/// </para>
/// <para>
/// The ROM is a commercial game and is not part of the repository; the test is skipped,
/// naming the file, when it is not on this machine.
/// </para>
/// </remarks>
public sealed class BackgroundRowSplitTests(ITestOutputHelper output)
{
    private const string Smb3 = "Super Mario Bros. 3 (USA).nes";

    /// <summary>
    /// Rows a frame can hold without a split: an NTSC frame has 240 visible lines, of which
    /// 30 have fine Y 1 (R2-005, criterion 2, as made numeric by R2-review-AMD-021 §3).
    /// </summary>
    private const int RowsWithoutSplit = 30;

    /// <summary>Line on which the MMC3 IRQ handler of the title screen writes $2006.</summary>
    private const int SplitLine = 193;

    [Trait("Requires", "UserRoms")]
    [UserRomFact(Smb3)]
    public void AMD_021_R2_005_smb3_emits_no_background_row_at_the_split_line()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(File.ReadAllBytes(UserRomFactAttribute.Path(Smb3)), Smb3).Success);

        var rowsPerFrame = new int[RowsWithoutSplit + 8];
        int worstRows = 0;
        int worstFrame = -1;
        var worstYs = new List<int>();
        int atSplitLine = 0;
        int smallestGap = int.MaxValue;
        int frame = 0;

        core.VideoCallback = (in FrameBatch batch) =>
        {
            var ys = new SortedSet<int>();
            foreach (TileRecord record in batch.Records)
            {
                if (record.Background)
                {
                    ys.Add(record.Y);
                }
            }

            // REQ-CORE-038: the row captured on line L carries y = L - 1.
            if (ys.Contains(SplitLine - 1))
            {
                atSplitLine++;
            }

            int previous = int.MinValue / 2;
            foreach (int y in ys)
            {
                smallestGap = Math.Min(smallestGap, y - previous);
                previous = y;
            }

            rowsPerFrame[Math.Min(ys.Count, rowsPerFrame.Length - 1)]++;
            if (ys.Count > worstRows)
            {
                worstRows = ys.Count;
                worstFrame = frame;
                worstYs = [.. ys];
            }
        };

        for (frame = 0; frame < 5_000; frame++)
        {
            core.RunFrame(0);
        }

        for (int count = 0; count < rowsPerFrame.Length; count++)
        {
            if (rowsPerFrame[count] > 0)
            {
                output.WriteLine($"{count} rows: {rowsPerFrame[count]} frames");
            }
        }

        output.WriteLine($"worst frame {worstFrame}: y = {string.Join(", ", worstYs)}");
        output.WriteLine($"frames with a row at the split line: {atSplitLine}");
        output.WriteLine($"smallest gap between two rows of one frame: {smallestGap}");

        // The criterion itself: the split line never emits a row. This is what the previous
        // instant of capture got wrong, in 4 734 of these 5 000 frames.
        Assert.Equal(0, atSplitLine);

        // The same fact seen from the banner side: the first banner row (line 195) is no
        // longer two lines after a row of the split line.
        Assert.True(smallestGap >= 3, $"two rows only {smallestGap} lines apart");

        // No frame carries more than one row above the thirty of a split-free image, and the
        // frames that do are only the ten of the grid phase described above (Q-059).
        Assert.True(worstRows <= RowsWithoutSplit + 1, $"frame {worstFrame} emitted {worstRows} rows");
        Assert.True(
            rowsPerFrame[RowsWithoutSplit + 1] <= 10,
            $"{rowsPerFrame[RowsWithoutSplit + 1]} frames exceeded {RowsWithoutSplit} rows");
    }
}
