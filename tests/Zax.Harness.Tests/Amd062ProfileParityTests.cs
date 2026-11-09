using Zax.Engine3D;
using Zax.Profiles;
using Zax.Profiles.Hash;
using Zax.Profiles.Tests;

namespace Zax.Harness.Tests;

/// <summary>
/// AMD-062 §1, REQ-PROF-078, REQ-PROF-081 and REQ-PROF-083 — the appariable « 3dh » profile of
/// 3dSen 2.1 and its 3dn counterpart draw the <b>same thing</b>: same ROM, same inputs, same
/// number of frames, and the conversion ends up with the same meshes and the same texels, tile
/// by drawn tile.
/// </summary>
/// <remarks>
/// <para>
/// This is the regression of lot V2-34, written from the five rows the R5 pass measured. The
/// comparison is read where the amendment says to read it: on the <c>--dump-meshes</c> export of
/// <c>render</c> (Q-081), whose entries carry the tile position, the vertex and triangle counts
/// and the local and world bounds. Nothing in that export depends on the format the profile came
/// in, so the two dumps must be equal character for character.
/// </para>
/// <para>
/// The « capture pixels » of the row are read without a GL context, as every test of this
/// project does: a drawn tile's colours come from its <see cref="ObjectTexture"/>, its material,
/// its alpha and its palette table, so the digest of those is what a differing pixel would show
/// up in. A GPU capture would add nothing but a window — and the machine this runs on does not
/// take kindly to sustained GL load.
/// </para>
/// <para>
/// The run uses an <b>empty</b> tile cache (<c>tileCache: null</c>), which is the initial
/// resolved set the determinism of REQ-PROF-080 is stated at.
/// </para>
/// </remarks>
public sealed class Amd062ProfileParityTests
{
    private const RenderAlgorithm Mesh = RenderAlgorithm.Greedy;

    [ProfileParityFact(Rom)] // AMD-062 §1, REQ-PROF-078, REQ-PROF-083
    public void Super_mario_bros_3_draws_the_same_tiles_in_both_formats()
        => AssertSameRender(
            Rom,
            "Super Mario Bros 3 (U) (PRG1).bytes",
            runFrames: 601,
            press: "201-206:start,301-306:start,701-706:right,801-806:a");

    [ProfileParityFact(ContraRom)] // AMD-062 §1, REQ-PROF-078, REQ-PROF-083
    public void Contra_draws_the_same_tiles_in_both_formats()
        => AssertSameRender(
            ContraRom,
            "Contra.bytes",
            runFrames: 601,
            press: "201-206:start,601-881:right");

    [ProfileParityFact(ZeldaRom)] // AMD-062 §1, REQ-PROF-078, REQ-PROF-083
    public void Legend_of_zelda_draws_the_same_tiles_in_both_formats()
        => AssertSameRender(ZeldaRom, "Legend Of Zelda.bytes", runFrames: 201, press: null);

    [ProfileParityFact(MarioRom)] // AMD-062 §1, REQ-PROF-078, REQ-PROF-083
    public void Super_mario_bros_draws_the_same_tiles_in_both_formats()
        => AssertSameRender(
            MarioRom,
            "Super Mario Bros.bytes",
            runFrames: 421,
            press: "201-206:start,301-420:right");

    [ProfileParityFact(MegaManRom)] // AMD-062 §1, REQ-PROF-078, REQ-PROF-083
    public void Mega_man_2_draws_the_same_tiles_in_both_formats()
        => AssertSameRender(MegaManRom, "Mega Man 2.bytes", runFrames: 601, press: null);

    private const string Rom = "Super Mario Bros. 3 (USA).nes";
    private const string ContraRom = "Contra (USA).nes";
    private const string ZeldaRom = "Legend of Zelda, The (USA).nes";
    private const string MarioRom = "Super Mario Bros. (World).nes";
    private const string MegaManRom = "Mega Man 2 (USA).nes";

    /// <summary>
    /// Captures the same run twice — once with the 3dn profile, once with the « 3dh » one — and
    /// compares what the conversion drew.
    /// </summary>
    /// <param name="romName">The dump to replay, under <c>ZAXNES_ROMS</c>.</param>
    /// <param name="profileName">The profile file name, the same in both folders.</param>
    /// <param name="runFrames">The <c>--run</c> of the row.</param>
    /// <param name="press">The <c>--press</c> of the row, or null for idle controllers.</param>
    private static void AssertSameRender(
        string romName, string profileName, int runFrames, string? press)
    {
        string romPath = UserRoms.Path(romName)!;

        // The 3dn counterpart first: it needs no resolution, and it is the reference.
        FrameDrawList expected = Capture(PlainProfile(profileName), romPath, runFrames, press);
        FrameDrawList actual = Capture(HashProfile(profileName), romPath, runFrames, press);

        Assert.NotEmpty(expected.Tiles);
        Assert.Equal(expected.Tiles.Count, actual.Tiles.Count);

        // The per-tile report the row is phrased in: how many drawn tiles differ, and the first
        // few, named by their tile position as the dump names them.
        List<string> differences = [];
        for (int index = 0; index < expected.Tiles.Count; index++)
        {
            string left = Signature(expected.Tiles[index]);
            string right = Signature(actual.Tiles[index]);
            if (!string.Equals(left, right, StringComparison.Ordinal))
            {
                differences.Add($"\n    3dn {left}\n    3dh {right}");
            }
        }

        Assert.True(
            differences.Count == 0,
            $"{romName} with {profileName}: {differences.Count} of {expected.Tiles.Count} drawn "
            + $"tiles differ between the 3dh profile and its 3dn counterpart."
            + string.Concat(differences.Take(5)));

        // And the export itself, as the row reads it: the two dumps are the same text.
        Assert.Equal(
            MeshDump.Write(expected, Mesh, romName),
            MeshDump.Write(actual, Mesh, romName));
    }

    private static FrameDrawList Capture(
        ProfileDocument document, string romPath, int runFrames, string? press)
    {
        SyntheticScene scene = RenderExport.FromRom(
            romPath,
            runFrames,
            document.Profile,
            document.Profile.Title ?? "2D Mode",
            Mesh,
            InputScript.Parse(press),
            tiles: document.Tiles);
        return scene.Engine.BuildDrawList();
    }

    /// <summary>
    /// Everything of a drawn tile that a differing mesh or a differing pixel would show in: what
    /// <c>--dump-meshes</c> writes, and what the renderer samples.
    /// </summary>
    private static string Signature(DrawTile tile)
    {
        TileMesh mesh = tile.Mesh ?? TileMesh.Empty;
        return $"({tile.Tile.X}, {tile.Tile.Y}) {(tile.Sprite ? "sprite" : "bg")}"
            + $" instance {tile.Instance.Index} {tile.Material} alpha {tile.Alpha:0.######}"
            + $" vertices {mesh.VertexCount} triangles {mesh.TriangleCount}"
            + $" localMin ({Vector(mesh.BoundsMin)}) localMax ({Vector(mesh.BoundsMax)})"
            + $" depth {tile.Instance.Depth} palette {tile.PaletteTable}"
            + $" texOffset ({tile.TexOffset.X:0.######}, {tile.TexOffset.Y:0.######})"
            + $" texTilting ({tile.TexTilting.X:0.######}, {tile.TexTilting.Y:0.######})"
            + $" texture {Texels(tile.Texture)}";
    }

    private static string Vector(Vec3 value)
        => $"{value.X:0.######}, {value.Y:0.######}, {value.Z:0.######}";

    /// <summary>
    /// The size, the format and an FNV-1a 64 of every texel of a tile's texture — the pixels the
    /// renderer would put on screen. FNV rather than <c>HashCode</c>, whose seed changes from one
    /// process to the next.
    /// </summary>
    private static string Texels(ObjectTexture? texture)
    {
        if (texture is null)
        {
            return "none";
        }

        ulong hash = 0xCBF2_9CE4_8422_2325UL;
        void Mix(int value)
        {
            for (int shift = 0; shift < 32; shift += 8)
            {
                hash = (hash ^ (byte)(value >> shift)) * 0x0000_0100_0000_01B3UL;
            }
        }

        for (int y = 0; y < texture.Height; y++)
        {
            for (int x = 0; x < texture.Width; x++)
            {
                Mix(texture.Texel(x, y));
                if (texture.IsDirect)
                {
                    Rgb24 color = texture.Color(x, y);
                    Mix(color.R | (color.G << 8) | (color.B << 16));
                }
            }
        }

        return $"{texture.Width}x{texture.Height} {texture.Format} {hash:x16}";
    }

    private static ProfileDocument HashProfile(string fileName)
        => Load(Path.Combine(NeutralAssets.HashProfilesDirectory!, fileName));

    private static ProfileDocument PlainProfile(string fileName)
        => Load(Path.Combine(
            NeutralAssets.ProfilesDirectory!, NeutralAssets.CounterpartName(fileName)));

    private static ProfileDocument Load(string path)
    {
        ProfileLoadResult result = ProfileFormat.Load(File.ReadAllBytes(path), sourceName: path);
        Assert.True(result.IsSuccess, $"{path}: {result.Message}");
        return result.Document!;
    }
}

/// <summary>
/// A fact of <see cref="Amd062ProfileParityTests"/>, skipped when <c>assets-neutral</c> or the
/// one dump the row replays is not on this machine. No machine path is committed
/// (AMD-024 REQ-PUB-001): <c>HOWTO-RUN.md</c> documents <c>ZAXNES_ROMS</c>.
/// </summary>
public sealed class ProfileParityFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    /// <param name="rom">File name of the dump the row needs.</param>
    public ProfileParityFactAttribute(string rom)
    {
        if (!NeutralAssets.HashProfilesAvailable || !NeutralAssets.Available)
        {
            Skip = NeutralAssets.MissingMessage;
        }
        else if (UserRoms.Path(rom) is null)
        {
            Skip = UserRoms.MissingMessage(rom);
        }
    }
}
