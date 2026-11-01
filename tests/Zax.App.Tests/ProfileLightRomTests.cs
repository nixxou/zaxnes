using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;
using Xunit.Sdk;

namespace Zax.App.Tests;

/// <summary>
/// The user's own dumps and the extracted profiles, both optional. A test that needs either is
/// reported as <b>skipped</b>, naming what is missing, so a checkout without them stays green
/// while the gap stays visible (AMD-024 REQ-PUB-001: nothing in the build depends on an asset).
/// </summary>
internal static class ProfileRoms
{
    /// <summary>
    /// Environment variable naming the folder the user keeps their own dumps in. No machine path
    /// is written anywhere in this repository (lot PUB-1); <c>HOWTO-RUN.md</c> §12 documents it.
    /// </summary>
    public const string Variable = "ZAXNES_ROMS";

    /// <summary>Where the user keeps their dumps, or null when the variable is not set.</summary>
    public static string? RomFolder { get; } =
        Environment.GetEnvironmentVariable(Variable) is { } value && value.Trim().Length > 0
            ? value.Trim()
            : null;

    /// <summary>
    /// The extracted <c>assets-neutral</c> folder, or null. A <b>test</b> may walk the tree
    /// looking for it; the application may not, which is why this does not use
    /// <c>Zax.App.NeutralAssets</c> (REQ-PUB-001).
    /// </summary>
    public static string? AssetFolder { get; } = LocateAssets();

    /// <summary>Full path of a dump, or null when it is not on this machine.</summary>
    public static string? Rom(string name)
    {
        if (RomFolder is null)
        {
            return null;
        }

        string candidate = Path.Combine(RomFolder, name);
        return File.Exists(candidate) ? candidate : null;
    }

    /// <summary>Full path of an extracted profile, or null when it is not on this machine.</summary>
    public static string? ProfilePath(string file)
    {
        if (AssetFolder is null)
        {
            return null;
        }

        string candidate = Path.Combine(AssetFolder, "profiles", file);
        return File.Exists(candidate) ? candidate : null;
    }

    /// <summary>The profile <paramref name="file"/>, which must be present.</summary>
    public static GameProfile Profile(string file)
    {
        ProfileLoadResult result = ProfileFormat.Load(
            File.ReadAllBytes(ProfilePath(file)!), sourceName: file);
        Assert.True(result.IsSuccess, $"{file}: {result.Message}");
        return result.Profile!;
    }

    /// <summary>Why <paramref name="rom"/> and <paramref name="profile"/> cannot be used, or null.</summary>
    public static string? MissingReason(string rom, string profile)
    {
        if (RomFolder is null)
        {
            return $"no user ROM folder: set {Variable} to the folder holding your own dumps.";
        }

        if (Rom(rom) is null)
        {
            return $"user ROM not available: {Path.Combine(RomFolder, rom)}.";
        }

        return ProfilePath(profile) is null
            ? $"extracted profile not available: assets-neutral/profiles/{profile}."
            : null;
    }

    private static string? LocateAssets()
    {
        DirectoryInfo? at = new(AppContext.BaseDirectory);
        while (at is not null)
        {
            string candidate = Path.Combine(at.FullName, "assets-neutral");
            if (Directory.Exists(candidate))
            {
                return candidate;
            }

            at = at.Parent;
        }

        return null;
    }
}

/// <summary>
/// A fact skipped, naming what is missing, unless both the user's dump and the extracted profile
/// it needs are on this machine.
/// </summary>
[XunitTestCaseDiscoverer("Xunit.Sdk.FactDiscoverer", "xunit.execution.dotnet")]
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
public sealed class ProfileRomFactAttribute : FactAttribute
{
    /// <summary>Requires <paramref name="rom"/> and <paramref name="profile"/>.</summary>
    public ProfileRomFactAttribute(string rom, string profile)
    {
        if (ProfileRoms.MissingReason(rom, profile) is { } reason)
        {
            Skip = reason;
        }
    }
}

/// <summary>
/// What a built-in profile's scripts light the real game with, on the real dump: the acceptance
/// level of SPEC-05 §4.1.3 <c>Light</c> and SPEC-06 REQ-SCN-025 (REQ-PERS-007, DEC-06-Q5).
/// </summary>
public sealed class ProfileLightRomTests
{
    private const string MegaMan2Rom = "Mega Man 2 (USA).nes";
    private const string MegaMan2Profile = "Mega Man 2.bytes";
    private const string CastlevaniaRom = "Castlevania [USA][Rev 1][!!].nes";
    private const string CastlevaniaProfile = "Castlevania (U) (PRG 1).bytes";

    /// <summary>
    /// REQ-SCN-025 with REQ-PERS-007: the Mega Man 2 profile stores a green light
    /// (FF33CC00, direction (−3, −3, 1)) and its <c>stage</c> script replaces it every frame
    /// with white and (1, −1, 1) while <c>$2A</c> is not 6 — which it never is during the
    /// intro. The scene the window renders at frame 601 must be lit by the script's white, not
    /// by the profile's green.
    /// </summary>
    [ProfileRomFact(MegaMan2Rom, MegaMan2Profile)] // REQ-SCN-025, REQ-PERS-007, REQ-LUA-025
    public void The_stage_script_of_mega_man_2_lights_the_intro_white()
    {
        GameProfile profile = ProfileRoms.Profile(MegaMan2Profile);

        // The green really is what the profile carries, so the assertion below is a measure of
        // the script's write and not of an unread setting.
        ArgbColor stored = profile.Settings!.Light!.Color!.Value;
        Assert.Equal(unchecked((int)0xFF33CC00), stored.Packed);

        SceneState scene = new();
        using GameSession game = GameSession.Load(
            ProfileRoms.Rom(MegaMan2Rom)!, profile, "Mega Man 2", scene);
        game.ApplyProfileSettings(scene, profile.Settings);

        for (int frame = 0; frame < 601; frame++)
        {
            game.RunFrame(0);
        }

        // $2A never reaches 6 during the intro, so the script's « else » branch is the one in
        // force: white, coming from above, from the left and from the camera side.
        Assert.Equal(new Rgb24(255, 255, 255), scene.Light.Color);
        Assert.Equal(
            Vector3.Normalize(new Vector3(1f, -1f, 1f)),
            Vector3.Normalize(scene.Light.Direction),
            Approximately);
    }

    /// <summary>
    /// REQ-SCN-025 with REQ-LUA-020: Castlevania's <c>ui</c> script dims the light to
    /// <c>Color(0.8, 0.8, 0.8)</c> in <c>Start</c> and puts white back in <c>End</c>. While the
    /// game runs it is the dimmed one that lights the scene.
    /// </summary>
    [ProfileRomFact(CastlevaniaRom, CastlevaniaProfile)] // REQ-SCN-025, REQ-LUA-020, REQ-LUA-021
    public void The_ui_script_of_castlevania_dims_the_light_while_the_game_runs()
    {
        GameProfile profile = ProfileRoms.Profile(CastlevaniaProfile);

        SceneState scene = new();
        using GameSession game = GameSession.Load(
            ProfileRoms.Rom(CastlevaniaRom)!, profile, "Castlevania", scene);
        game.ApplyProfileSettings(scene, profile.Settings);

        for (int frame = 0; frame < 120; frame++)
        {
            game.RunFrame(0);
        }

        Assert.Equal(new Rgb24(204, 204, 204), scene.Light.Color);

        // And the white the End hook puts back only arrives once the game is over.
        game.End();
        Assert.Equal(new Rgb24(255, 255, 255), scene.Light.Color);
    }

    private const float Tolerance = 1e-4f;

    private static readonly IEqualityComparer<Vector3> Approximately = new VectorComparer();

    private sealed class VectorComparer : IEqualityComparer<Vector3>
    {
        public bool Equals(Vector3 x, Vector3 y) => (x - y).Length() <= Tolerance;

        public int GetHashCode(Vector3 value) => 0;
    }
}
