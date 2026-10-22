using Zax.Nes.Cartridges.Mappers;
using Zax.Nes.Rom;

namespace Zax.Nes.Cartridges;

/// <summary>
/// Maps an iNES mapper number to its implementation (REQ-CORE-100, REQ-CORE-102).
/// </summary>
/// <remarks>
/// <para>
/// The registry only advertises the boards that are actually implemented, so that a ROM
/// is never accepted and then found unplayable. Any other number is refused at load time
/// with the message of REQ-CORE-011 (REQ-CORE-102) — including 116, which REQ-CORE-101
/// leaves optional and which is not implemented here.
/// </para>
/// <para>
/// The table is the one of REQ-CORE-100, one entry per iNES number; several numbers share a
/// class when they are the same silicon differently wired (9 and 10, 21/22/23/25/27, 87 and
/// 101).
/// </para>
/// </remarks>
public static class MapperRegistry
{
    private static readonly Dictionary<int, Func<RomImage, Cartridge>> Factories = new()
    {
        [0] = static rom => new Nrom(rom),
        [1] = static rom => new Mmc1(rom),
        [2] = static rom => new UxRom(rom),
        [3] = static rom => new CnRom(rom),
        [4] = static rom => new Mmc3(rom),
        [5] = static rom => new Mmc5(rom),
        [7] = static rom => new AxRom(rom),
        [9] = static rom => new Mmc2(rom, 9),
        [10] = static rom => new Mmc2(rom, 10),
        [11] = static rom => new ColorDreams(rom),
        [13] = static rom => new CpRom(rom),
        [15] = static rom => new Mapper015(rom),
        [16] = static rom => new BandaiFcg(rom),
        [18] = static rom => new JalecoSs88006(rom),
        [21] = static rom => new Vrc2Vrc4(rom, 21),
        [22] = static rom => new Vrc2Vrc4(rom, 22),
        [23] = static rom => new Vrc2Vrc4(rom, 23),
        [25] = static rom => new Vrc2Vrc4(rom, 25),
        [27] = static rom => new Vrc2Vrc4(rom, 27),
        [28] = static rom => new Action53(rom),
        [30] = static rom => new UnRom512(rom),
        [32] = static rom => new IremG101(rom),
        [34] = static rom => new Mapper034(rom),
        [41] = static rom => new Caltron6In1(rom),
        [64] = static rom => new Rambo1(rom),
        [66] = static rom => new GxRom(rom),
        [71] = static rom => new Codemasters(rom),
        [87] = static rom => new JalecoJ87(rom, 87),
        [101] = static rom => new JalecoJ87(rom, 101),
        [225] = static rom => new Multicart225(rom),
        [228] = static rom => new Action52(rom),
    };

    /// <summary>Mapper numbers this build can run, in ascending order.</summary>
    public static IReadOnlyList<int> SupportedMappers { get; } = [.. Factories.Keys.Order()];

    /// <summary>Whether <paramref name="mapper"/> is implemented.</summary>
    public static bool IsSupported(int mapper) => Factories.ContainsKey(mapper);

    /// <summary>
    /// Builds the cartridge of <paramref name="rom"/>.
    /// </summary>
    /// <returns>
    /// <see langword="false"/> with an REQ-CORE-011 message when the mapper number is not
    /// implemented.
    /// </returns>
    public static bool TryCreate(RomImage rom, out Cartridge? cartridge, out string? error)
        => TryCreate(rom, out cartridge, out _, out error);

    /// <summary>
    /// Builds the cartridge of <paramref name="rom"/>, reporting the typed cause of a
    /// failure as well as its message (AMD-002, answer to Q-001).
    /// </summary>
    /// <remarks>
    /// The cartridge comes back already powered on, so that a caller which only wants to look
    /// at the banks — the header vectors of the test suite, for one — never sees a board whose
    /// registers have not been initialised.
    /// </remarks>
    public static bool TryCreate(
        RomImage rom,
        out Cartridge? cartridge,
        out RomLoadFailure failure,
        out string? error)
    {
        ArgumentNullException.ThrowIfNull(rom);

        if (!Factories.TryGetValue(rom.Mapper, out Func<RomImage, Cartridge>? factory))
        {
            cartridge = null;
            failure = RomLoadFailure.UnsupportedMapper;

            // Q-001: the wording is ours; REQ-CORE-011 only requires the number to appear,
            // and AMD-002 makes the typed cause above the normative part.
            error = $"Unsupported mapper {rom.Mapper}.";
            return false;
        }

        cartridge = factory(rom);
        cartridge.Reset();
        failure = RomLoadFailure.None;
        error = null;
        return true;
    }
}
