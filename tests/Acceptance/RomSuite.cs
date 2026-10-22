using System.Globalization;
using System.Text;
using Zax.Nes;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Runs the public test ROMs of SPEC-02 §3.12 and reads back their documented verdict.
/// Written by the verifier from the readmes and the assembly sources shipped with the ROMs.
/// </summary>
internal static class RomSuite
{
    /// <summary>Root of the public test ROM tree (<c>impl/tests/roms</c>).</summary>
    public static string Root { get; } = Locate();

    private static string Locate()
    {
        DirectoryInfo? at = new(AppContext.BaseDirectory);
        while (at is not null)
        {
            string candidate = Path.Combine(at.FullName, "tests", "roms");
            if (Directory.Exists(candidate))
            {
                return candidate;
            }

            at = at.Parent;
        }

        throw new DirectoryNotFoundException("tests/roms was not found above the test binary.");
    }

    /// <summary>Full path of a ROM under <see cref="Root"/>, or null when it is absent.</summary>
    public static string? Find(string relative)
    {
        string path = Path.Combine(Root, relative.Replace('/', Path.DirectorySeparatorChar));
        return File.Exists(path) ? path : null;
    }

    /// <summary>Outcome of a run.</summary>
    /// <param name="Code">Result code the ROM reported.</param>
    /// <param name="Text">Text output, empty for the 2005-era protocol.</param>
    /// <param name="Frames">Frames the run took.</param>
    /// <param name="TimedOut">Whether the budget ran out.</param>
    public sealed record Outcome(int Code, string Text, long Frames, bool TimedOut)
    {
        /// <summary>A one-line description for an assertion message.</summary>
        public string Describe()
            => $"code {Code}{(TimedOut ? " (timed out)" : string.Empty)} after {Frames} frames"
             + (Text.Length > 0 ? $", output: {Text.Replace('\n', '|')}" : string.Empty);
    }

    private static NesCore Load(string path)
    {
        NesCore core = new();
        Zax.Nes.Rom.RomLoadResult result = core.LoadRom(File.ReadAllBytes(path), Path.GetFileName(path));
        Assert.True(result.Success, $"{Path.GetFileName(path)}: {result.Error}");
        return core;
    }

    /// <summary>
    /// Runs a ROM that uses the later blargg "shell" protocol: <c>$6001-$6003</c> hold
    /// <c>DE B0 61</c>, <c>$6000</c> holds <c>$80</c> while running, <c>$81</c> when the ROM
    /// asks for a reset, and the final code afterwards; 0 means "Passed". The text printed on
    /// screen is a zero-terminated string at <c>$6004</c>.
    /// </summary>
    public static Outcome RunShell(string path, int maxFrames = 20_000)
    {
        NesCore core = Load(path);
        bool started = false;
        long resetAt = -1;

        for (long frame = 0; frame < maxFrames; frame++)
        {
            core.RunFrame(0);

            if (core.ReadMemory(0x6001) != 0xDE || core.ReadMemory(0x6002) != 0xB0
                || core.ReadMemory(0x6003) != 0x61)
            {
                continue;
            }

            byte status = core.ReadMemory(0x6000);
            if (status == 0x80)
            {
                started = true;
                continue;
            }

            if (status == 0x81)
            {
                // The ROM asks for a reset, no sooner than 100 ms later.
                if (resetAt < 0)
                {
                    resetAt = frame + 10;
                }
                else if (frame >= resetAt)
                {
                    core.Reset();
                    resetAt = -1;
                }

                continue;
            }

            if (started)
            {
                return new Outcome(status, ReadText(core, 0x6004), frame + 1, false);
            }
        }

        return new Outcome(core.ReadMemory(0x6000), ReadText(core, 0x6004), maxFrames, true);
    }

    /// <summary>
    /// Address the <c>validation.a</c> shipped with <c>sprite_hit_tests</c>,
    /// <c>sprite_overflow_tests</c>, <c>vbl_nmi_timing</c> and <c>branch_timing_tests</c>
    /// declares for its result code (<c>result = $f8</c>).
    /// </summary>
    public const ushort ValidationResult = 0x00F8;

    /// <summary>
    /// Result address of the two 2005 suites whose support sources are not shipped
    /// (<c>blargg_apu_2005.07.30</c>, <c>blargg_ppu_tests_2005.09.15b</c>). It was located by
    /// running the suites, and the control of
    /// <c>Power_up_palette_is_the_one_blargg_ppu_test_that_fails</c> shows that the reader
    /// discriminates pass from failure at that address.
    /// </summary>
    public const ushort BlarggResult = 0x00F0;

    /// <summary>
    /// Runs a ROM of the 2005-era suites. Such a ROM reports its code and then parks in a
    /// one-instruction endless loop; 1 means "passed". Parking is confirmed over several
    /// frames, because the runtimes also park while waiting for an interrupt.
    /// </summary>
    public static Outcome RunLegacy(
        string path,
        ushort resultAddress = ValidationResult,
        int maxFrames = 6_000,
        int confirmFrames = 30)
    {
        NesCore core = Load(path);
        ushort parkedAt = 0;
        int parked = 0;

        for (long frame = 0; frame < maxFrames; frame++)
        {
            core.RunFrame(0);

            ushort pc = core.Cpu.PC;
            bool loops = core.ReadMemory(pc) == 0x4C
                && (ushort)(core.ReadMemory((ushort)(pc + 1)) | (core.ReadMemory((ushort)(pc + 2)) << 8)) == pc;

            if (loops && (parked == 0 || pc == parkedAt))
            {
                parkedAt = pc;
                if (++parked >= confirmFrames)
                {
                    return new Outcome(core.ReadMemory(resultAddress), string.Empty, frame + 1, false);
                }
            }
            else
            {
                parked = 0;
            }
        }

        return new Outcome(core.ReadMemory(resultAddress), string.Empty, maxFrames, true);
    }

    private static string ReadText(NesCore core, ushort at)
    {
        StringBuilder text = new();
        for (int i = 0; i < 512; i++)
        {
            byte b = core.ReadMemory((ushort)(at + i));
            if (b == 0)
            {
                break;
            }

            text.Append((char)b);
        }

        return text.ToString().Trim();
    }

    /// <summary>Parses a <c>nestest.log</c> line into the fields the comparison checks.</summary>
    public static (ushort Pc, byte A, byte X, byte Y, byte P, byte S, long Cyc) ParseLogLine(string line)
    {
        ushort pc = ushort.Parse(line.AsSpan(0, 4), NumberStyles.HexNumber, CultureInfo.InvariantCulture);
        byte Field(string key)
        {
            int at = line.IndexOf(key, StringComparison.Ordinal);
            return byte.Parse(line.AsSpan(at + key.Length, 2), NumberStyles.HexNumber, CultureInfo.InvariantCulture);
        }

        int cycAt = line.IndexOf("CYC:", StringComparison.Ordinal);
        long cyc = long.Parse(line.AsSpan(cycAt + 4), CultureInfo.InvariantCulture);
        return (pc, Field("A:"), Field("X:"), Field("Y:"), Field("P:"), Field("SP:"), cyc);
    }
}
