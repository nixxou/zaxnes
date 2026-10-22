using System.Globalization;
using System.Text;
using System.Text.RegularExpressions;
using Zax.Nes.Cpu;

namespace Zax.Nes.Tests;

/// <summary>
/// Compares a run of the public <c>nestest.nes</c> in automation mode with the public
/// <c>nestest.log</c>, line by line (REQ-CORE-110).
/// </summary>
/// <remarks>
/// <para>
/// Automation mode means starting at $C000 instead of through the reset vector; the ROM
/// then runs the official opcodes and, after them, the unofficial ones, without touching
/// the PPU. Compared fields: PC, the instruction bytes, A, X, Y, P, SP and the CPU cycle
/// count. The disassembly text is reported when a line differs, to make a failure
/// readable, but it is not part of the comparison: the reference log shows the byte each
/// addressing mode points at, and some of those addresses are PPU registers, which the
/// side-effect-free host accessor of REQ-CORE-081 deliberately reads back as 0.
/// </para>
/// <para>
/// The ROM and its log are public but are not redistributed here; see
/// <c>tests/roms/README.md</c>.
/// </para>
/// </remarks>
public sealed partial class NestestTests
{
    /// <summary>Entry point of nestest's automation mode (public nestest documentation).</summary>
    private const ushort AutomationEntry = 0xC000;

    /// <summary>Addresses nestest writes its two result codes to; 0 means every test passed.</summary>
    private const ushort ResultLow = 0x0002;
    private const ushort ResultHigh = 0x0003;

    // The reference log is fixed-width: the PC occupies columns 1 to 4, the one to three
    // instruction bytes columns 7 to 14, and the mnemonic starts at column 17 — preceded, at
    // column 16, by an asterisk for the unofficial opcodes ("*NOP", "*LAX", "*ISB"…). The
    // separator between the bytes and the mnemonic therefore shrinks to a single space on
    // those lines, so the bytes field is delimited by the mnemonic itself rather than by a
    // run of two or more spaces.
    [GeneratedRegex(
        @"^(?<pc>[0-9A-F]{4})\s+(?<bytes>[0-9A-F]{2}(?: [0-9A-F]{2}){0,2})\s+\*?[A-Z]{3}.*?"
        + @"A:(?<a>[0-9A-F]{2}) X:(?<x>[0-9A-F]{2}) Y:(?<y>[0-9A-F]{2}) "
        + @"P:(?<p>[0-9A-F]{2}) SP:(?<sp>[0-9A-F]{2}).*?CYC:(?<cyc>\d+)\s*$",
        RegexOptions.ExplicitCapture | RegexOptions.CultureInvariant)]
    private static partial Regex LogLine();

    [RomFact("nestest.nes", "nestest.log")]
    public void REQ_CORE_110_nestest_trace_matches_the_public_reference_log()
    {
        string romPath = TestRomLibrary.Find("nestest.nes")!;
        string logPath = TestRomLibrary.Find("nestest.log")!;

        byte[] image = File.ReadAllBytes(romPath);
        string[] logLines = File.ReadAllLines(logPath);

        var core = new NesCore();
        Rom.RomLoadResult load = core.LoadRom(image, "nestest.nes");
        Assert.True(load.Success, load.Error);

        // Automation mode: enter at $C000 instead of the reset vector. The reset itself
        // has already consumed its seven cycles, which is where the log's CYC:7 comes from.
        core.Cpu.PC = AutomationEntry;

        int compared = 0;
        var failures = new StringBuilder();

        foreach (string raw in logLines)
        {
            if (string.IsNullOrWhiteSpace(raw))
            {
                continue;
            }

            Match match = LogLine().Match(raw);
            Assert.True(match.Success, $"Reference log line {compared + 1} could not be parsed: \"{raw}\"");

            CpuTraceLine actual = CpuTracer.Capture(core.Cpu, core.Bus);
            string? difference = Compare(match, actual);

            if (difference is not null)
            {
                failures.AppendLine(CultureInfo.InvariantCulture, $"line {compared + 1}: {difference}");
                failures.AppendLine(CultureInfo.InvariantCulture, $"  expected: {raw.TrimEnd()}");
                failures.AppendLine(CultureInfo.InvariantCulture, $"  actual:   {actual.Format()}");

                if (failures.Length > 4000)
                {
                    break;
                }
            }

            compared++;
            core.Cpu.Step();
        }

        Assert.True(
            failures.Length == 0,
            $"{compared} of {logLines.Length} reference lines compared.{Environment.NewLine}{failures}");

        Assert.True(compared > 8000, $"Only {compared} lines were compared; the log looks truncated.");

        // nestest reports its verdict in $0002 and $0003: zero means everything passed.
        Assert.Equal(0x00, core.ReadMemory(ResultLow));
        Assert.Equal(0x00, core.ReadMemory(ResultHigh));
    }

    private static string? Compare(Match expected, CpuTraceLine actual)
    {
        ushort pc = Hex16(expected.Groups["pc"].Value);
        if (pc != actual.Pc)
        {
            return $"PC ${pc:X4} vs ${actual.Pc:X4}";
        }

        byte[] bytes = expected.Groups["bytes"].Value
            .Split(' ', StringSplitOptions.RemoveEmptyEntries)
            .Select(Hex8)
            .ToArray();

        if (!bytes.AsSpan().SequenceEqual(actual.Bytes))
        {
            return $"instruction bytes {Join(bytes)} vs {Join(actual.Bytes)}";
        }

        if (Hex8(expected.Groups["a"].Value) != actual.A)
        {
            return $"A ${expected.Groups["a"].Value} vs ${actual.A:X2}";
        }

        if (Hex8(expected.Groups["x"].Value) != actual.X)
        {
            return $"X ${expected.Groups["x"].Value} vs ${actual.X:X2}";
        }

        if (Hex8(expected.Groups["y"].Value) != actual.Y)
        {
            return $"Y ${expected.Groups["y"].Value} vs ${actual.Y:X2}";
        }

        if (Hex8(expected.Groups["p"].Value) != actual.P)
        {
            return $"P ${expected.Groups["p"].Value} vs ${actual.P:X2}";
        }

        if (Hex8(expected.Groups["sp"].Value) != actual.S)
        {
            return $"SP ${expected.Groups["sp"].Value} vs ${actual.S:X2}";
        }

        long cycles = long.Parse(expected.Groups["cyc"].Value, CultureInfo.InvariantCulture);
        return cycles != actual.Cycles ? $"CYC {cycles} vs {actual.Cycles}" : null;
    }

    private static byte Hex8(string text) => byte.Parse(text, NumberStyles.HexNumber, CultureInfo.InvariantCulture);

    private static ushort Hex16(string text) => ushort.Parse(text, NumberStyles.HexNumber, CultureInfo.InvariantCulture);

    private static string Join(ReadOnlySpan<byte> bytes)
    {
        var text = new StringBuilder();
        foreach (byte value in bytes)
        {
            text.Append(value.ToString("X2", CultureInfo.InvariantCulture)).Append(' ');
        }

        return text.ToString().TrimEnd();
    }

    [Fact]
    public void REQ_CORE_110_the_tracer_renders_the_nestest_line_format()
    {
        // Pinned without the ROM so that the format itself stays covered: the first line of
        // the public log is
        // "C000  4C F5 C5  JMP $C5F5   ... A:00 X:00 Y:00 P:24 SP:FD ... CYC:7".
        var bus = new TestBus();
        bus.Load(0xC000, 0x4C, 0xF5, 0xC5);
        bus[0xFFFC] = 0x00;
        bus[0xFFFD] = 0xC0;

        var cpu = new Cpu6502(bus);
        cpu.Reset();

        CpuTraceLine line = CpuTracer.Capture(cpu, bus);

        Assert.Equal(0xC000, line.Pc);
        Assert.Equal<byte[]>([0x4C, 0xF5, 0xC5], line.Bytes);
        Assert.Equal(0x00, line.A);
        Assert.Equal(0x00, line.X);
        Assert.Equal(0x00, line.Y);
        Assert.Equal(0x24, line.P);
        Assert.Equal(0xFD, line.S);
        Assert.Equal(7, line.Cycles);

        string formatted = line.Format();
        Assert.StartsWith("C000  4C F5 C5", formatted, StringComparison.Ordinal);
        Assert.Contains("JMP $C5F5", formatted, StringComparison.Ordinal);
        Assert.Contains("A:00 X:00 Y:00 P:24 SP:FD CYC:7", formatted, StringComparison.Ordinal);

        // And the parser of the comparison accepts the real reference layout.
        const string reference =
            "C000  4C F5 C5  JMP $C5F5                       A:00 X:00 Y:00 P:24 SP:FD PPU:  0, 21 CYC:7";
        Match match = LogLine().Match(reference);
        Assert.True(match.Success);
        Assert.Null(Compare(match, line));
    }

    [Fact]
    public void REQ_CORE_110_the_tracer_has_no_effect_on_the_emulation()
    {
        // Tracing must not change a single cycle or byte, otherwise the comparison above
        // would be meaningless (REQ-CORE-081).
        byte[] rom = TestRoms.Minimal(0x01, [0xE6, 0x20, 0xA5, 0x20, 0x8D, 0x00, 0x03, 0x4C, 0x00, 0x80]);

        var untraced = new NesCore();
        var traced = new NesCore();
        Assert.True(untraced.LoadRom(rom, "T.nes").Success);
        Assert.True(traced.LoadRom(rom, "T.nes").Success);

        for (int i = 0; i < 20_000; i++)
        {
            CpuTracer.Capture(traced.Cpu, traced.Bus);
            traced.Cpu.Step();
            untraced.Cpu.Step();
        }

        Assert.Equal(untraced.CycleCount, traced.CycleCount);
        Assert.Equal(untraced.Cpu.PC, traced.Cpu.PC);

        for (int address = 0; address < 0x800; address++)
        {
            Assert.Equal(untraced.ReadMemory((ushort)address), traced.ReadMemory((ushort)address));
        }
    }
}
