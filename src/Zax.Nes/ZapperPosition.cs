namespace Zax.Nes;

/// <summary>
/// Point the Zapper is aimed at, in NES pixels (REQ-CORE-060).
/// </summary>
/// <remarks>
/// The coordinates may fall outside [0, 256) x [0, 240), in which case no light is ever
/// detected (REQ-CORE-062, SPEC-08 REQ-INP-063/064).
/// </remarks>
/// <param name="X">Aimed x, in NES pixels.</param>
/// <param name="Y">Aimed y, in NES pixels.</param>
public readonly record struct ZapperPosition(int X, int Y)
{
    /// <summary>A position outside the image, used when the host supplies no source.</summary>
    public static ZapperPosition OffScreen => new(-1, -1);

    /// <summary>Whether the point is inside the 256 x 240 image.</summary>
    public bool IsOnScreen => X is >= 0 and < 256 && Y is >= 0 and < RegionTiming.VisibleScanlines;
}

/// <summary>
/// Supplies the aimed position of the Zapper plugged into <paramref name="port"/> (0 or 1).
/// </summary>
/// <remarks>
/// The core queries it once per frame, at the start of "run one frame", and the position
/// then stays fixed for the whole frame (REQ-CORE-060).
/// </remarks>
public delegate ZapperPosition ZapperPositionSource(int port);
