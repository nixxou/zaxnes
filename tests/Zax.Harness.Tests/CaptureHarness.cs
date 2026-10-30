using Zax.Engine3D;
using Zax.Profiles;

namespace Zax.Harness.Tests;

/// <summary>
/// Runs the harness's own <c>render --rom</c> path on a blank NROM image, which is what every
/// test of this project captures: the headless equivalent of "load a ROM and a profile, replay,
/// export an offscreen capture" (SPEC-01 REQ-ARCH-031).
/// </summary>
internal static class CaptureHarness
{
    /// <summary>
    /// Captures <paramref name="profile"/> after <paramref name="runFrames"/> real frames.
    /// </summary>
    /// <param name="profile">The profile to replay; its scripts and settings both act.</param>
    /// <param name="runFrames">Real frames to run before the capture.</param>
    /// <param name="title">How the capture names the profile (REQ-PROF-022).</param>
    /// <param name="mesh">The mesh algorithm <c>--mesh</c> stands for (REQ-E3D-340).</param>
    public static SyntheticScene Capture(
        GameProfile profile,
        int runFrames = 4,
        string? title = null,
        RenderAlgorithm mesh = RenderAlgorithm.Greedy)
    {
        string rom = Path.Combine(Path.GetTempPath(), $"zax-harness-{Guid.NewGuid():N}.nes");
        File.WriteAllBytes(rom, BlankRom());
        try
        {
            return RenderExport.FromRom(
                rom,
                runFrames,
                profile,
                title ?? "2D Mode",
                mesh,
                InputScript.Parse(null));
        }
        finally
        {
            File.Delete(rom);
        }
    }

    /// <summary>
    /// The default profile of <see cref="GameProfile.CreateEmpty"/> carrying
    /// <paramref name="source"/> as its only script; an empty source leaves it without one.
    /// </summary>
    public static GameProfile WithScript(string source)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        if (source.Length > 0)
        {
            profile.Scripts.Add(new ProfileScript("a.lua", true, source, "*"));
        }

        return profile;
    }

    /// <summary>A 32 KiB PRG + 8 KiB CHR NROM image whose program is an endless loop.</summary>
    public static byte[] BlankRom()
    {
        byte[] image = new byte[16 + (32 * 1024) + (8 * 1024)];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 2;
        image[5] = 1;
        image[6] = 0x01;

        // JMP $8000 at the reset vector, which the three vectors all point to.
        image[16] = 0x4C;
        image[17] = 0x00;
        image[18] = 0x80;
        for (int vector = 0; vector < 3; vector++)
        {
            image[16 + (32 * 1024) - 6 + (vector * 2)] = 0x00;
            image[16 + (32 * 1024) - 5 + (vector * 2)] = 0x80;
        }

        return image;
    }
}
