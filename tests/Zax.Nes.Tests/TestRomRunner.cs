using System.Globalization;
using System.Text;

namespace Zax.Nes.Tests;

/// <summary>
/// Runs a public test ROM to completion and reads back its documented result.
/// </summary>
/// <remarks>
/// <para>
/// Two result protocols are in use among the ROMs SPEC-02 §3.12 requires.
/// </para>
/// <list type="bullet">
/// <item>
/// The <strong>shell protocol</strong> of the later blargg suites (<c>ppu_vbl_nmi</c>,
/// <c>oam_read</c>, <c>ppu_open_bus</c>, <c>ppu_read_buffer</c>): <c>$6001-$6003</c> hold
/// <c>$DE $B0 $61</c>, <c>$6000</c> holds <c>$80</c> while the test runs, <c>$81</c> when
/// the test wants a reset, and the final code afterwards — 0 meaning "passed". The same
/// text that is printed on screen is also a zero-terminated string at <c>$6004</c>, which
/// is what the readmes mean by "a NES emulator without a working PPU".
/// </item>
/// <item>
/// The <strong>2005-era protocol</strong> (<c>sprite_hit_tests</c>,
/// <c>sprite_overflow_tests</c>, <c>vbl_nmi_timing</c>, <c>branch_timing_tests</c>,
/// <c>blargg_ppu_tests</c>): the result code lives in a low-memory variable, 1 meaning
/// "passed", and the ROM then prints it and jumps to a one-instruction endless loop. The
/// support sources shipped with those ROMs put that variable at <c>$00F8</c>
/// (<c>validation.a</c>: <c>result = $f8</c>).
/// </item>
/// </list>
/// <para>
/// Both are read without any emulator-side hook: the runner only runs frames and looks at
/// CPU memory through the side-effect-free accessor of REQ-CORE-080.
/// </para>
/// </remarks>
internal static class TestRomRunner
{
    /// <summary>Address the later suites report their status at.</summary>
    public const ushort ShellStatus = 0x6000;

    /// <summary>Address the later suites write their output text at.</summary>
    public const ushort ShellText = 0x6004;

    /// <summary>Address the 2005-era suites keep their result code at (<c>validation.a</c>).</summary>
    public const ushort LegacyResult = 0x00F8;

    /// <summary>Outcome of a test ROM run.</summary>
    /// <param name="Code">Result code the ROM reported.</param>
    /// <param name="Passed">Whether the code is the ROM's success value.</param>
    /// <param name="Text">Text the ROM produced, empty for the 2005-era protocol.</param>
    /// <param name="Frames">Frames the run took.</param>
    /// <param name="TimedOut">Whether the frame budget ran out before the ROM finished.</param>
    public sealed record Result(int Code, bool Passed, string Text, long Frames, bool TimedOut)
    {
        /// <summary>A one-line description for an assertion message.</summary>
        public string Describe()
        {
            var text = new StringBuilder();
            text.Append(CultureInfo.InvariantCulture, $"code {Code}");
            if (TimedOut)
            {
                text.Append(" (timed out)");
            }

            text.Append(CultureInfo.InvariantCulture, $" after {Frames} frames");
            if (Text.Length > 0)
            {
                text.Append(", output: ").Append(Text.Replace('\n', '|'));
            }

            return text.ToString();
        }
    }

    /// <summary>
    /// Runs a ROM that uses the shell protocol: $6000 holds $80 while running and the final
    /// code afterwards, 0 meaning "passed".
    /// </summary>
    /// <param name="path">ROM to run.</param>
    /// <param name="maxFrames">Frame budget before the run is declared timed out.</param>
    /// <param name="pressReset">
    /// Whether to press reset when the ROM asks for it ($6000 = $81). Passing
    /// <see langword="false"/> stops the run there and reports $81, which is how the
    /// power-on half of a reset test is read on a core whose reset is a power cycle
    /// (REQ-CORE-004).
    /// </param>
    public static Result RunShellRom(string path, int maxFrames = 4000, bool pressReset = true)
    {
        NesCore core = Load(path);

        long frames = 0;
        bool started = false;
        int resetDelay = 0;

        while (frames < maxFrames)
        {
            core.RunFrame(0);
            frames++;

            if (resetDelay > 0 && --resetDelay == 0)
            {
                core.Reset();
                continue;
            }

            byte status = core.ReadMemory(ShellStatus);

            if (!started)
            {
                if (core.ReadMemory(0x6001) == 0xDE
                    && core.ReadMemory(0x6002) == 0xB0
                    && core.ReadMemory(0x6003) == 0x61
                    && status == 0x80)
                {
                    started = true;
                }

                continue;
            }

            if (status == 0x81)
            {
                if (!pressReset)
                {
                    return new Result(status, Passed: false, ReadText(core), frames, TimedOut: false);
                }

                // "Emulator should wait a couple of frames after seeing $81."
                resetDelay = 3;
                continue;
            }

            if (status != 0x80)
            {
                return new Result(status, status == 0, ReadText(core), frames, TimedOut: false);
            }
        }

        return new Result(core.ReadMemory(ShellStatus), Passed: false, ReadText(core), frames, TimedOut: true);
    }

    /// <summary>
    /// Frames the CPU has to stay parked at the same address before the run is called over.
    /// </summary>
    /// <remarks>
    /// Being parked in a one-instruction loop is not on its own a terminal state: the
    /// runtimes of the 2005-era suites also park that way while waiting for an interrupt to
    /// move them on, and <c>vbl_clear_time</c> does so for two frames in the middle of the
    /// test, with an intermediate code still in the result variable. The final loop never
    /// ends, so a confirmation window tells the two apart.
    /// </remarks>
    private const int HaltConfirmationFrames = 20;

    /// <summary>
    /// Runs a ROM of the 2005-era suites: it stops in a one-instruction endless loop once
    /// it has reported, and the result code, 1 meaning "passed", stays in
    /// <paramref name="resultAddress"/>.
    /// </summary>
    public static Result RunLegacyRom(string path, int maxFrames = 2000, ushort resultAddress = LegacyResult)
    {
        NesCore core = Load(path);

        long frames = 0;
        ushort parkedAt = 0;
        int parkedFrames = 0;

        while (frames < maxFrames)
        {
            core.RunFrame(0);
            frames++;

            if (IsHalted(core) && (parkedFrames == 0 || core.Cpu.PC == parkedAt))
            {
                parkedAt = core.Cpu.PC;
                parkedFrames++;

                if (parkedFrames >= HaltConfirmationFrames)
                {
                    byte code = core.ReadMemory(resultAddress);
                    return new Result(code, code == 1, string.Empty, frames, TimedOut: false);
                }
            }
            else
            {
                parkedFrames = 0;
            }
        }

        return new Result(
            core.ReadMemory(resultAddress),
            Passed: false,
            string.Empty,
            frames,
            TimedOut: true);
    }

    /// <summary>
    /// Whether the CPU sits in a <c>jmp</c> to itself, which is how every blargg runtime
    /// parks once it has reported its result (<c>runtime_rom.a</c>: <c>exit: jmp exit</c>).
    /// </summary>
    private static bool IsHalted(NesCore core)
    {
        ushort pc = core.Cpu.PC;
        if (core.ReadMemory(pc) != 0x4C)
        {
            return false;
        }

        var target = (ushort)(core.ReadMemory((ushort)(pc + 1)) | (core.ReadMemory((ushort)(pc + 2)) << 8));
        return target == pc;
    }

    private static NesCore Load(string path)
    {
        var core = new NesCore();
        Rom.RomLoadResult load = core.LoadRom(File.ReadAllBytes(path), Path.GetFileName(path));
        Assert.True(load.Success, $"{Path.GetFileName(path)}: {load.Error}");
        return core;
    }

    private static string ReadText(NesCore core)
    {
        var text = new StringBuilder();
        for (ushort address = ShellText; address < 0x7FFF; address++)
        {
            byte value = core.ReadMemory(address);
            if (value == 0)
            {
                break;
            }

            text.Append((char)value);
        }

        return text.ToString().Trim();
    }
}
