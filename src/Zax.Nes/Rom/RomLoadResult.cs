namespace Zax.Nes.Rom;

/// <summary>
/// Outcome of "load a ROM" (REQ-CORE-010, REQ-CORE-011, REQ-CORE-013).
/// </summary>
/// <remarks>
/// On failure the instance is left stopped, as if no ROM were loaded (REQ-CORE-005);
/// <see cref="Failure"/> is the typed cause and <see cref="Error"/> a technical message in
/// English, the host choosing the text it displays (AMD-002, answer to Q-001). On success
/// <see cref="Region"/> carries the detected region, which the core does <em>not</em> apply
/// by itself: the host calls "choose the region" according to its "automatic region"
/// setting (REQ-CORE-013).
/// </remarks>
public readonly record struct RomLoadResult
{
    private RomLoadResult(bool success, NesRegion region, RomLoadFailure failure, string? error)
    {
        Success = success;
        Region = region;
        Failure = failure;
        Error = error;
    }

    /// <summary>Whether the ROM was accepted.</summary>
    public bool Success { get; }

    /// <summary>Region detected from the header, meaningful only on success.</summary>
    public NesRegion Region { get; }

    /// <summary>
    /// Typed cause of the failure, <see cref="RomLoadFailure.None"/> on success
    /// (AMD-002, answer to Q-001).
    /// </summary>
    public RomLoadFailure Failure { get; }

    /// <summary>Technical message naming the cause, or <see langword="null"/> on success.</summary>
    public string? Error { get; }

    /// <summary>Builds a successful result carrying the detected region.</summary>
    public static RomLoadResult Ok(NesRegion region) => new(true, region, RomLoadFailure.None, null);

    /// <summary>Builds a failed result carrying the typed cause and its message.</summary>
    public static RomLoadResult Fail(RomLoadFailure failure, string error)
        => new(false, NesRegion.Ntsc, failure, error);
}
