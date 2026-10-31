using System.Globalization;
using Zax.Nes;
using Zax.Nes.Rom;

namespace Zax.Harness;

/// <summary>
/// The <c>mixer</c> command: the test point of REQ-CORE-076 (AMD-054), which reads the three
/// integers <c>A</c>, <c>M</c> and <c>S</c> of REQ-CORE-073 at a chosen instant of the
/// emulation, before any conversion to floating point and before the DC blocker.
/// </summary>
/// <remarks>
/// <para>
/// The numeric acceptance criteria of AMD-054 are given in the mixer's integer units, which
/// the output of REQ-CORE-071 does not carry: it only ever offers floats taken after the
/// high-pass and after the band-limited resampling. REQ-CORE-076 therefore makes the three
/// integers a test point the harness can read, and this command is it.
/// </para>
/// <para>
/// No commercial ROM is needed — the category of the criteria is "auto" (SPEC-12 §2). Without
/// <c>--rom</c> the command builds its own cartridge: an iNES image of the mapper asked for,
/// whose PRG is one endless jump, since nothing here runs the game's code. The registers are
/// written straight onto the bus with <c>--poke</c>, exactly as a CPU write would, and the
/// machine is then stepped for as many CPU cycles as asked.
/// </para>
/// </remarks>
internal static class MixerProbe
{
    /// <summary>Usage text of the command.</summary>
    public const string Usage = """
        Zax.Harness mixer — read the APU mixer test point of REQ-CORE-076 (A, M, S)

        Usage:
          Zax.Harness mixer [options]

        Options:
          --rom <file>        ROM to run; a built test cartridge otherwise.
          --mapper <n>        Mapper of the built test cartridge (default 0; 5 for the MMC5).
          --poke <a>=<v>      Write byte <v> at CPU address <a> before stepping, as a CPU
                              write would. Hexadecimal unless prefixed with 'd'. Repeatable,
                              applied in the order given.
          --cycles <n>        CPU cycles to step after the pokes (default 4000).
          --peak              Report the cycle with the largest S instead of the last one.
          --json              Print one JSON object instead of the plain line.

        Output:
          A=<a> M=<m> S=<s>   A: the five APU channels mixed (REQ-CORE-073).
                              M: the cartridge expansion audio (REQ-CORE-075).
                              S: min(A + M, 32767), what the output path is fed.

        Examples:
          mixer --poke 4015=01 --poke 4000=BF --poke 4002=64 --poke 4003=F8 --peak
          mixer --mapper 5 --poke 5011=80 --cycles 1
        """;

    /// <summary>One reading of the test point.</summary>
    /// <param name="Apu"><c>A</c>: the five APU channels mixed (REQ-CORE-073).</param>
    /// <param name="Expansion"><c>M</c>: the cartridge expansion audio (REQ-CORE-075).</param>
    /// <param name="Output"><c>S = min(A + M, 32767)</c>.</param>
    internal readonly record struct Reading(int Apu, int Expansion, int Output)
    {
        /// <summary>The plain one-line form the command prints.</summary>
        public override string ToString() => string.Create(
            CultureInfo.InvariantCulture, $"A={Apu} M={Expansion} S={Output}");

        /// <summary>The JSON form <c>--json</c> prints.</summary>
        public string ToJson() => string.Create(
            CultureInfo.InvariantCulture, $"{{\"a\":{Apu},\"m\":{Expansion},\"s\":{Output}}}");
    }

    /// <summary>Runs the command.</summary>
    /// <param name="args">Arguments after <c>mixer</c>.</param>
    /// <returns>0 on success, 2 on a usage error.</returns>
    public static int Run(string[] args)
    {
        ArgumentNullException.ThrowIfNull(args);

        string? rom = null;
        int mapper = 0;
        int cycles = 4000;
        bool peak = false;
        bool json = false;
        List<(ushort Address, byte Value)> pokes = [];

        for (int i = 0; i < args.Length; i++)
        {
            switch (args[i])
            {
                case "--rom":
                    rom = Next(args, ref i);
                    break;

                case "--mapper":
                    mapper = Number(args, ref i);
                    break;

                case "--cycles":
                    cycles = Number(args, ref i);
                    break;

                case "--poke":
                    if (!TryParsePoke(Next(args, ref i), out (ushort, byte) poke))
                    {
                        Console.Error.WriteLine($"mixer: --poke wants <addr>=<value>, got '{args[i]}'");
                        return 2;
                    }

                    pokes.Add(poke);
                    break;

                case "--peak":
                    peak = true;
                    break;

                case "--json":
                    json = true;
                    break;

                case "--help":
                case "-h":
                    Console.Out.WriteLine(Usage);
                    return 0;

                default:
                    Console.Error.WriteLine($"mixer: unknown option {args[i]}");
                    Console.Error.WriteLine();
                    Console.Error.WriteLine(Usage);
                    return 2;
            }
        }

        if (cycles < 0)
        {
            Console.Error.WriteLine("mixer: --cycles cannot be negative.");
            return 2;
        }

        byte[] image;
        string name;
        if (rom is not null)
        {
            if (!File.Exists(rom))
            {
                Console.Error.WriteLine($"mixer: {rom} was not found.");
                return 2;
            }

            image = File.ReadAllBytes(rom);
            name = Path.GetFileName(rom);
        }
        else
        {
            image = BuildTestCartridge(mapper);
            name = string.Create(CultureInfo.InvariantCulture, $"mixer-mapper-{mapper}.nes");
        }

        NesCore core = new();
        RomLoadResult load = core.LoadRom(image, name);
        if (!load.Success)
        {
            Console.Error.WriteLine($"mixer: {name}: {load.Error}");
            return 2;
        }

        Reading reading = Measure(core, pokes, cycles, peak);
        Console.Out.WriteLine(json ? reading.ToJson() : reading.ToString());
        return 0;
    }

    /// <summary>
    /// Applies <paramref name="pokes"/>, steps <paramref name="core"/> for
    /// <paramref name="cycles"/> CPU cycles and reads the test point.
    /// </summary>
    /// <param name="core">The machine, with a ROM already loaded.</param>
    /// <param name="pokes">Register writes to apply first, in order.</param>
    /// <param name="cycles">CPU cycles to step.</param>
    /// <param name="peak">
    /// When true the reading returned is the one with the largest <c>S</c> over the run, which
    /// is what a criterion on a tone's peak level wants; otherwise it is the last cycle's.
    /// </param>
    internal static Reading Measure(
        NesCore core,
        IReadOnlyList<(ushort Address, byte Value)> pokes,
        int cycles,
        bool peak)
    {
        ArgumentNullException.ThrowIfNull(core);
        ArgumentNullException.ThrowIfNull(pokes);

        foreach ((ushort address, byte value) in pokes)
        {
            core.WriteMemory(address, value);
        }

        Reading best = Read(core);
        for (int i = 0; i < cycles; i++)
        {
            core.Bus.Tick();
            Reading now = Read(core);
            if (!peak || now.Output > best.Output)
            {
                best = now;
            }
        }

        return best;
    }

    private static Reading Read(NesCore core)
        => new(core.Apu.MixerApu, core.Apu.MixerExpansion, core.Apu.MixerOutput);

    /// <summary>
    /// Builds an iNES image of <paramref name="mapper"/> with 32 KiB of PRG holding one
    /// endless jump, 8 KiB of blank CHR and all three vectors on it.
    /// </summary>
    /// <remarks>
    /// Only the low four bits of the mapper number fit in byte 6 of an iNES header and the
    /// next four in byte 7, which covers every mapper the command needs (0 and 5).
    /// </remarks>
    internal static byte[] BuildTestCartridge(int mapper)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(mapper);
        ArgumentOutOfRangeException.ThrowIfGreaterThan(mapper, 255);

        const int prgSize = 2 * 16 * 1024;
        const int chrSize = 8 * 1024;
        var image = new byte[16 + prgSize + chrSize];

        image[0] = 0x4E; // "NES\x1A"
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 0x02; // 2 x 16 KiB of PRG.
        image[5] = 0x01; // 1 x 8 KiB of CHR.
        image[6] = (byte)(0x01 | ((mapper & 0x0F) << 4)); // Vertical mirroring.
        image[7] = (byte)(mapper & 0xF0);

        // JMP $8000 at $8000, and the three vectors pointing at it.
        image[16] = 0x4C;
        image[17] = 0x00;
        image[18] = 0x80;

        int vectors = 16 + prgSize - 6;
        for (int i = 0; i < 3; i++)
        {
            image[vectors + (2 * i)] = 0x00;
            image[vectors + (2 * i) + 1] = 0x80;
        }

        return image;
    }

    /// <summary>
    /// Parses <c>&lt;addr&gt;=&lt;value&gt;</c>, both hexadecimal unless prefixed with
    /// <c>d</c>.
    /// </summary>
    internal static bool TryParsePoke(string text, out (ushort Address, byte Value) poke)
    {
        poke = default;

        int equals = text.IndexOf('=', StringComparison.Ordinal);
        if (equals <= 0 || equals == text.Length - 1)
        {
            return false;
        }

        if (!TryParseNumber(text[..equals], out int address)
            || !TryParseNumber(text[(equals + 1)..], out int value)
            || address is < 0 or > 0xFFFF
            || value is < 0 or > 0xFF)
        {
            return false;
        }

        poke = ((ushort)address, (byte)value);
        return true;
    }

    private static bool TryParseNumber(string text, out int value)
        => text.StartsWith('d') || text.StartsWith('D')
            ? int.TryParse(text[1..], NumberStyles.Integer, CultureInfo.InvariantCulture, out value)
            : int.TryParse(text, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out value);

    private static int Number(string[] args, ref int index)
        => int.Parse(Next(args, ref index), CultureInfo.InvariantCulture);

    private static string Next(string[] args, ref int index)
    {
        if (index + 1 >= args.Length)
        {
            throw new ArgumentException($"{args[index]} wants a value.", nameof(args));
        }

        return args[++index];
    }
}
