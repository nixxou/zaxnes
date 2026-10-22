namespace Zax.Nes;

/// <summary>
/// Device plugged into one of the two controller ports (REQ-CORE-024).
/// </summary>
/// <remarks>The numeric values are part of the host-visible contract: 0 and 1.</remarks>
public enum ControllerPortType
{
    /// <summary>Standard controller, read as a serial shift register.</summary>
    Controller = 0,

    /// <summary>Zapper light gun (REQ-CORE-061).</summary>
    Zapper = 1,
}
