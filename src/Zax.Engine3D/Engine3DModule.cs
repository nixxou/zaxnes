namespace Zax.Engine3D;

/// <summary>
/// Identity of the 2D-to-3D conversion engine.
/// </summary>
/// <remarks>
/// Lot L4a implements the conversion pipeline of SPEC-04: intake of the core's tile records,
/// frame model, pattern recognition, shapes and frame instances, palettes, the tracker, the
/// Lua hook call points and the abstract output the renderer consumes
/// (<see cref="ConversionEngine"/>). The geometry and the meshes of SPEC-04B are lot L4b,
/// behind <see cref="IPatternGeometryBuilder"/>. The assembly never touches a graphics API
/// (REQ-ARCH-002).
/// </remarks>
public static class Engine3DModule
{
    /// <summary>Stable identifier of this module, used by the headless harness and tests.</summary>
    public const string Name = "Zax.Engine3D";

    /// <summary>Short human-readable role of this module.</summary>
    public const string Role = "2D-to-3D conversion (abstract meshes)";
}
