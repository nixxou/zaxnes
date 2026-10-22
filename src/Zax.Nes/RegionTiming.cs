namespace Zax.Nes;

/// <summary>
/// Frame geometry and clock rates of one <see cref="NesRegion"/> (REQ-CORE-015).
/// </summary>
/// <remarks>
/// The values come from the public nesdev "Cycle reference chart" and "Clock rate"
/// pages. PPU dots per CPU cycle is expressed as the exact fraction
/// <see cref="DotsPerCycleNumerator"/> / <see cref="DotsPerCycleDenominator"/> so that
/// PAL's 3.2 dots per cycle stays an integer ratio (16 dots every 5 CPU cycles) and the
/// core remains bit-for-bit deterministic (REQ-ARCH-012).
/// </remarks>
public sealed record RegionTiming
{
    /// <summary>Visible scanlines of every region (REQ-CORE-015).</summary>
    public const int VisibleScanlines = 240;

    /// <summary>PPU dots in one scanline on every region.</summary>
    public const int DotsPerScanline = 341;

    private RegionTiming(
        NesRegion region,
        int cpuClockHz,
        int dotsPerCycleNumerator,
        int dotsPerCycleDenominator,
        int scanlinesPerFrame,
        int vblankScanline,
        bool skipsDotOnOddFrames,
        int nominalFramesPerSecond,
        bool usesPalNoiseAndDmcPeriods)
    {
        Region = region;
        CpuClockHz = cpuClockHz;
        DotsPerCycleNumerator = dotsPerCycleNumerator;
        DotsPerCycleDenominator = dotsPerCycleDenominator;
        ScanlinesPerFrame = scanlinesPerFrame;
        VBlankScanline = vblankScanline;
        SkipsDotOnOddFrames = skipsDotOnOddFrames;
        NominalFramesPerSecond = nominalFramesPerSecond;
        UsesPalNoiseAndDmcPeriods = usesPalNoiseAndDmcPeriods;
    }

    /// <summary>The region these timings describe.</summary>
    public NesRegion Region { get; }

    /// <summary>CPU clock in Hz.</summary>
    public int CpuClockHz { get; }

    /// <summary>Numerator of the PPU-dots-per-CPU-cycle ratio.</summary>
    public int DotsPerCycleNumerator { get; }

    /// <summary>Denominator of the PPU-dots-per-CPU-cycle ratio.</summary>
    public int DotsPerCycleDenominator { get; }

    /// <summary>Total scanlines in a frame, visible and non-visible.</summary>
    public int ScanlinesPerFrame { get; }

    /// <summary>
    /// Scanline on which the VBlank flag is raised and NMI fires (dot 1).
    /// 241 for NTSC and PAL, 291 for Dendy.
    /// </summary>
    public int VBlankScanline { get; }

    /// <summary>
    /// Whether the idle dot of the pre-render line is skipped on odd frames while
    /// rendering is enabled. True for NTSC only.
    /// </summary>
    public bool SkipsDotOnOddFrames { get; }

    /// <summary>
    /// Rate at which the reference host chains frames (SPEC-07, DEC-R2-T3). The audio
    /// resampler is calibrated on this value, not on the exact hardware frame rate
    /// (REQ-CORE-070).
    /// </summary>
    public int NominalFramesPerSecond { get; }

    /// <summary>
    /// Whether the APU noise and DMC period tables are the PAL ones. True for
    /// <see cref="NesRegion.Pal"/> only; MULTI and DENDY use the NTSC tables
    /// (REQ-CORE-015).
    /// </summary>
    public bool UsesPalNoiseAndDmcPeriods { get; }

    /// <summary>Mono samples produced per frame at <paramref name="sampleRate"/>, speed 1.</summary>
    public double SamplesPerFrame(int sampleRate) => (double)sampleRate / NominalFramesPerSecond;

    private static readonly RegionTiming NtscTiming = new(
        NesRegion.Ntsc, 1_789_773, 3, 1, 262, 241, true, 60, false);

    private static readonly RegionTiming PalTiming = new(
        NesRegion.Pal, 1_662_607, 16, 5, 312, 241, false, 50, true);

    private static readonly RegionTiming MultiTiming = new(
        NesRegion.Multi, 1_789_773, 3, 1, 262, 241, true, 60, false);

    private static readonly RegionTiming DendyTiming = new(
        NesRegion.Dendy, 1_773_448, 3, 1, 312, 291, false, 50, false);

    /// <summary>Returns the timings of <paramref name="region"/>.</summary>
    /// <exception cref="ArgumentOutOfRangeException">The region is not a known value.</exception>
    public static RegionTiming For(NesRegion region) => region switch
    {
        NesRegion.Ntsc => NtscTiming,
        NesRegion.Pal => PalTiming,
        NesRegion.Multi => MultiTiming,
        NesRegion.Dendy => DendyTiming,
        _ => throw new ArgumentOutOfRangeException(nameof(region), region, "Unknown NES region."),
    };
}
