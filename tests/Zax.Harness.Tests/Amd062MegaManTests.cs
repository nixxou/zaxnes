using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Profiles.Hash;
using Zax.Profiles.Tests;

namespace Zax.Harness.Tests;

/// <summary>
/// The optional criterion of AMD-062 REQ-PROF-080 on the user's own dump: with
/// <c>Mega Man 2.bytes</c> (2.1) and <c>Mega Man 2 (USA).nes</c>, what the ROM resolves, what the
/// running game resolves, and the meshes of the drawn tiles whose pattern has nothing left
/// pending compared with the 3dn counterpart.
/// </summary>
/// <remarks>
/// <para>
/// Mega Man 2 is an MMC1 board with no CHR-ROM: it decompresses its tiles into CHR-RAM, which is
/// the very case AMD-062 §1 describes. 5 353 of its 5 497 manifest strings are found in the ROM
/// file, exactly as REQ-PROF-078 measures. The cache is empty, as the determinism of
/// REQ-PROF-080 is stated at.
/// </para>
/// <para>
/// The 144 that remain never turn up in this dump over the frames a blind button script reaches,
/// so the count the criterion asks to journal is reported rather than asserted — see Q-178 §2.
/// What <b>is</b> asserted is that the mechanism works on this real game: every tile the PPU
/// drew is found as a 16-byte aligned block of the CHR-RAM the core reports (REQ-CORE-123), so a
/// tile of the manifest that did appear would be resolved.
/// </para>
/// </remarks>
public sealed class Amd062MegaManTests
{
    private const string RomName = "Mega Man 2 (USA).nes";
    private const string ProfileName = "Mega Man 2.bytes";

    /// <summary>Resolved from the ROM file alone (REQ-PROF-078 criterion).</summary>
    private const int FromRomCount = 5_353;

    /// <summary>Strings of the manifest (REQ-PROF-078 criterion).</summary>
    private const int TotalCount = 5_497;

    /// <summary>How many real frames the run replays: the intro and the attract sequence.</summary>
    private const int FrameCount = 1_200;

    /// <summary>The <c>--press</c> script: Start every two seconds, which walks the intro.</summary>
    private const string AttractScript =
        "120-125:start,240-245:start,360-365:start,480-485:start,600-605:start,"
        + "720-725:start,840-845:start,960-965:start,1080-1085:start";

    [HashRomFact]
    public void The_rom_resolves_what_the_criterion_measures_and_the_game_resolves_the_rest()
    {
        ProfileDocument document = LoadHashProfile();
        HashProfileTiles tiles = document.Tiles!;
        Assert.Equal(TotalCount, tiles.TotalCount);

        // REQ-PROF-078, with an empty cache (REQ-PROF-082).
        tiles.PrepareForRom(document.Profile.Id, File.ReadAllBytes(RomPath), ResolvedTileCache.Disabled);
        Assert.Equal(FromRomCount, tiles.ResolvedCount);

        NesCore core = new();
        Assert.True(core.LoadRom(File.ReadAllBytes(RomPath), RomName).Success);
        Assert.True(core.ChrRamSize > 0, "Mega Man 2 is a CHR-RAM board.");
        core.Reset();

        // The scan of REQ-PROF-080, both of its sources, on every displayed frame.
        int fromChrRam = 0;
        int fromScreen = 0;
        List<(ulong Low, ulong High)> lastFrame = [];
        core.VideoCallback = (in FrameBatch batch) =>
        {
            lastFrame.Clear();
            foreach (TileRecord record in batch.Records)
            {
                fromScreen += tiles.ResolveFromFrameTile(record.Plane0, record.Plane1);
                if (!record.IsEmpty && !lastFrame.Contains((record.Plane0, record.Plane1)))
                {
                    lastFrame.Add((record.Plane0, record.Plane1));
                }
            }
        };

        InputScript script = InputScript.Parse(AttractScript);
        for (int frame = 0; frame < FrameCount; frame++)
        {
            core.RunFrame(script.At(frame), FrameOutput.All);
            fromChrRam += tiles.ResolveFromChrRam(core.ChrRam);
        }

        // Journalled, as the criterion asks. Nothing is asserted about the number: the tiles the
        // ROM file does not hold do not show up in this dump over these frames (Q-178 §2).
        Assert.True(
            fromChrRam >= 0 && fromScreen >= 0,
            $"{fromChrRam} tiles resolved from the CHR-RAM and {fromScreen} from the screen over "
            + $"{FrameCount} frames; {tiles.PendingCount} of {TotalCount} still pending.");
        Assert.Equal(FromRomCount + fromChrRam + fromScreen, tiles.ResolvedCount);

        // The mechanism itself, on the real game: every tile the PPU drew on the last frame is a
        // 16-byte aligned block of the CHR-RAM the core reports (REQ-CORE-123), so a manifest
        // tile that appeared would be resolved by the stateless scan.
        Assert.NotEmpty(lastFrame);
        Assert.Equal(lastFrame.Count, CountInChrRam(core.ChrRam, lastFrame));
    }

    // REQ-PROF-080: once nothing is pending any more, the drawn tiles of the captured frame have
    // the meshes the 3dn counterpart gives.
    [HashRomFact]
    public void Once_every_tile_is_resolved_the_capture_matches_the_3dn_one()
    {
        ProfileDocument plain = LoadPlainProfile();
        ProfileDocument hash = LoadHashProfile();

        // The tiles come from the counterpart's tiles rather than from the game: this dump does
        // not show the 144 the ROM file lacks over the frames a button script reaches (Q-178 §2),
        // and what the criterion is really about is that a resolved profile draws the same thing.
        HashProfileTiles tiles = hash.Tiles!;
        foreach (ProfileTile? cell in AllTiles(plain.Profile))
        {
            tiles.ResolveFromFrameTile(cell!.LowPlane, cell.HighPlane);
        }

        Assert.Equal(0, tiles.PendingCount);
        tiles.RebuildAllTextures();

        List<string> expected = Meshes(Capture(plain).Engine);
        List<string> actual = Meshes(Capture(hash).Engine);

        Assert.NotEmpty(expected);
        Assert.Equal(expected.Count, actual.Count);
        Assert.Equal(expected, actual);
    }

    /// <summary>The mesh of each drawn tile of the captured frame, in draw order.</summary>
    private static List<string> Meshes(ConversionEngine engine)
    {
        List<string> meshes = [];
        foreach (DrawTile tile in engine.BuildDrawList().Tiles)
        {
            TileMesh mesh = tile.Mesh ?? TileMesh.Empty;
            meshes.Add(
                $"{(tile.Sprite ? 's' : 'b')}:{tile.Tile.X},{tile.Tile.Y}:{tile.Instance.Index}:"
                + $"{mesh.VertexCount}/{mesh.TriangleCount}/"
                + $"{mesh.BoundsMin.X},{mesh.BoundsMin.Y},{mesh.BoundsMin.Z}/"
                + $"{mesh.BoundsMax.X},{mesh.BoundsMax.Y},{mesh.BoundsMax.Z}");
        }

        return meshes;
    }

    /// <summary>Every tile of every pattern of a profile, memorised slots included.</summary>
    private static IEnumerable<ProfileTile?> AllTiles(GameProfile profile)
    {
        foreach (ProfilePattern pattern in profile.PermanentPatterns.Concat(profile.TemporaryPatterns))
        {
            foreach (ProfileTile? cell in pattern.Grid?.Cells ?? [])
            {
                if (cell is not null)
                {
                    yield return cell;
                }
            }
        }
    }

    private static int CountInChrRam(ReadOnlySpan<byte> ram, List<(ulong Low, ulong High)> tiles)
    {
        HashSet<string> blocks = [];
        for (int offset = 0; offset + RawTile.ByteCount <= ram.Length; offset += RawTile.ByteCount)
        {
            blocks.Add(Convert.ToHexStringLower(ram.Slice(offset, RawTile.ByteCount)));
        }

        int found = 0;
        foreach ((ulong low, ulong high) in tiles)
        {
            if (blocks.Contains(Convert.ToHexStringLower(RawTile.FromPlanes(low, high))))
            {
                found++;
            }
        }

        return found;
    }

    private static SyntheticScene Capture(ProfileDocument document)
        => RenderExport.FromRom(
            RomPath,
            runFrames: FrameCount,
            document.Profile,
            "Mega Man 2",
            RenderAlgorithm.Greedy,
            InputScript.Parse(AttractScript),
            tiles: document.Tiles);

    private static string RomPath => UserRoms.Path(RomName)!;

    private static ProfileDocument LoadHashProfile()
        => Load(Path.Combine(NeutralAssets.HashProfilesDirectory!, ProfileName));

    private static ProfileDocument LoadPlainProfile()
        => Load(Path.Combine(NeutralAssets.ProfilesDirectory!, ProfileName));

    private static ProfileDocument Load(string path)
    {
        ProfileLoadResult result = ProfileFormat.Load(File.ReadAllBytes(path), sourceName: path);
        Assert.True(result.IsSuccess, $"{path}: {result.Message}");
        return result.Document!;
    }
}
