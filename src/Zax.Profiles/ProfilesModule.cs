namespace Zax.Profiles;

/// <summary>
/// Identifies the profile format and catalogue library to the headless harness and the tests.
/// </summary>
/// <remarks>
/// The library covers SPEC-03: the binary profile format (<c>.bytes</c> and <c>.3dn</c>) with
/// its reader and its canonical writer, the domain model the conversion engine and the Lua
/// runtime work with, the catalogue of the built-in profiles, the user profiles and the game
/// library with its ROM associations. It depends on no windowing, rendering, audio or input
/// API (REQ-ARCH-002).
/// </remarks>
public static class ProfilesModule
{
    /// <summary>Stable identifier of this module, used by the headless harness and tests.</summary>
    public const string Name = "Zax.Profiles";

    /// <summary>Short human-readable role of this module.</summary>
    public const string Role = "Profile format and catalogue";
}
