using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Profiles.Hash;
using Zax.Profiles.Tests;

namespace Zax.Harness.Tests;

/// <summary>
/// The ROM-free half of the V2-34 regression: the same equality as
/// <see cref="Amd062ProfileParityTests"/> — an appariable « 3dh » profile converts a frame
/// exactly as its 3dn counterpart does — read on the synthetic iNES file of AMD-062
/// REQ-PROF-078 instead of a commercial dump, so it runs on any machine.
/// </summary>
/// <remarks>
/// <para>
/// The profile goes through the <b>host path</b>, which is what the defect was on:
/// <see cref="HashProfileTiles.PrepareForRom"/> with an empty cache, then
/// <c>LoadProfile</c>. Nothing is resolved by hand and nothing is rebuilt by hand; whatever a
/// host has to do after a resolution has to happen by itself, or the two conversions differ.
/// </para>
/// <para>
/// Every tile is placed in the ROM <b>mirrored horizontally</b>, so the whole manifest is
/// resolved through the mirrorings of REQ-PROF-079 and every resolved tile receives the
/// un-mirrored bytes its identifier names.
/// </para>
/// <para>
/// The frame is synthetic: the grids of the profile's own patterns, laid out side by side on the
/// background, so that the patterns are recognised and their variants drawn. The profiles chosen
/// are three of the five rows the R5 pass measured, and patterns carrying a
/// <c>textureKind = 2</c> variant come first — that exclusive-or against the default texture
/// (REQ-PROF-083) is what a missed rebuild shows up in.
/// </para>
/// </remarks>
public sealed class Amd062SyntheticParityTests
{
    /// <summary>Patterns laid out on the synthetic frame, at most.</summary>
    private const int PatternsPerFrame = 12;

    [HashAssetsFact] // AMD-062 §1, REQ-PROF-078, REQ-PROF-079, REQ-PROF-083
    public void Super_mario_bros_3_converts_a_synthetic_frame_as_its_counterpart_does()
        => AssertSameConversion("Super Mario Bros 3 (U) (PRG1).bytes");

    [HashAssetsFact] // AMD-062 §1, REQ-PROF-078, REQ-PROF-079, REQ-PROF-083
    public void Contra_converts_a_synthetic_frame_as_its_counterpart_does()
        => AssertSameConversion("Contra.bytes");

    [HashAssetsFact] // AMD-062 §1, REQ-PROF-078, REQ-PROF-079, REQ-PROF-083
    public void Legend_of_zelda_converts_a_synthetic_frame_as_its_counterpart_does()
        => AssertSameConversion("Legend Of Zelda.bytes");

    private static void AssertSameConversion(string profileName)
    {
        ProfileDocument plain = Load(Path.Combine(
            NeutralAssets.ProfilesDirectory!, NeutralAssets.CounterpartName(profileName)));
        ProfileDocument hash = Load(Path.Combine(NeutralAssets.HashProfilesDirectory!, profileName));

        HashProfileTiles tiles = hash.Tiles!;
        Assert.True(tiles.HasHashTiles, $"{profileName} carries no hashed tile.");

        // The host path, with an empty cache: everything the « 3dh » profile needs has to come
        // from the ROM, and everything a resolution implies has to happen on its own.
        tiles.PrepareForRom(
            hash.Profile.Id,
            SyntheticIneFile(RawTiles(plain.Profile), TileFlip.Horizontal),
            ResolvedTileCache.Disabled);
        Assert.Equal(0, tiles.PendingCount);

        // The same layout drives both conversions, and it is built from the 3dn profile so that
        // it owes nothing to the resolution under test.
        List<ProfilePattern> laidOut = [.. Chosen(plain.Profile)];
        Assert.NotEmpty(laidOut);
        FrameBatchPlan plan = Layout(laidOut);

        FrameDrawList expected = ConvertOnce(plain.Profile, tiles: null, plan);
        FrameDrawList actual = ConvertOnce(hash.Profile, tiles, plan);

        Assert.NotEmpty(expected.Tiles);
        Assert.Equal(
            MeshDump.Write(expected, RenderAlgorithm.Greedy, profileName),
            MeshDump.Write(actual, RenderAlgorithm.Greedy, profileName));
    }

    /// <summary>
    /// Converts the synthetic frame with a freshly loaded engine and returns what it would draw.
    /// </summary>
    private static FrameDrawList ConvertOnce(
        GameProfile profile, HashProfileTiles? tiles, FrameBatchPlan plan)
    {
        ConversionEngine engine = new();
        GeometryLimits.Attach(engine);
        engine.LoadProfile(profile, applySettings: false, tiles);

        FrameBatch batch = new(plan.Records, plan.Palette, 0, 0);
        engine.Convert(batch, 1);
        engine.Publish();
        return engine.BuildDrawList();
    }

    /// <summary>
    /// The patterns laid out on the frame: the ones carrying an exclusive-or texture first, then
    /// the others, each small enough to fit twice over in the background grid.
    /// </summary>
    private static IEnumerable<ProfilePattern> Chosen(GameProfile profile)
    {
        List<ProfilePattern> withXor = [];
        List<ProfilePattern> others = [];
        foreach (ProfilePattern pattern in profile.AllPatterns)
        {
            if (pattern.Grid is not { Width: > 0 and <= 8, Height: > 0 and <= 8 })
            {
                continue;
            }

            bool xor = false;
            foreach (PatternVariant variant in pattern.Variants)
            {
                xor |= variant.XorTexture is not null;
            }

            (xor ? withXor : others).Add(pattern);
        }

        return withXor.Concat(others).Take(PatternsPerFrame);
    }

    /// <summary>
    /// Lays the grids of <paramref name="patterns"/> out on the background, one next to the
    /// other with an empty column and row between them so that no two of them touch.
    /// </summary>
    private static FrameBatchPlan Layout(List<ProfilePattern> patterns)
    {
        List<TileRecord> records = [];
        int column = 2;
        int row = 2;
        int rowHeight = 0;
        foreach (ProfilePattern pattern in patterns)
        {
            PatternGrid grid = pattern.Grid!;
            if (column + grid.Width > 30)
            {
                column = 2;
                row += rowHeight + 1;
                rowHeight = 0;
            }

            if (row + grid.Height > 28)
            {
                break;
            }

            for (int j = 0; j < grid.Height; j++)
            {
                for (int i = 0; i < grid.Width; i++)
                {
                    if (grid[i, j] is not { } tile)
                    {
                        continue;
                    }

                    records.Add(new TileRecord(
                        tile.LowPlane,
                        tile.HighPlane,
                        (short)(8 * (column + i)),
                        (short)(8 * (row + j)),
                        records.Count,
                        tile.FlipX,
                        tile.FlipY,
                        hidden: false,
                        palette: 0,
                        background: true));
                }
            }

            column += grid.Width + 1;
            rowHeight = Math.Max(rowHeight, grid.Height);
        }

        // A palette that is not all zeros, so an indexed texel reaches a real colour.
        byte[] palette = new byte[32];
        for (int index = 0; index < palette.Length; index++)
        {
            palette[index] = (byte)(0x0F + (index * 3));
        }

        return new FrameBatchPlan([.. records], palette);
    }

    /// <summary>The raw sixteen bytes of every tile the 3dn profile carries, slots included.</summary>
    private static List<byte[]> RawTiles(GameProfile profile)
    {
        HashSet<(ulong Low, ulong High)> seen = [];
        List<byte[]> raw = [];
        foreach (ProfileTile tile in Tiles(profile))
        {
            if (seen.Add((tile.LowPlane, tile.HighPlane)))
            {
                raw.Add(RawTile.FromPlanes(tile.LowPlane, tile.HighPlane));
            }
        }

        return raw;
    }

    /// <summary>
    /// Every tile object of the profile: the grids of its patterns, and those of the editor's
    /// memorised slots, which the manifest of the « 3dh » counterpart also names.
    /// </summary>
    private static IEnumerable<ProfileTile> Tiles(GameProfile profile)
    {
        foreach (ProfileTile tile in profile.DistinctTileObjects)
        {
            yield return tile;
        }

        if (profile.Settings is not { } settings)
        {
            yield break;
        }

        foreach (RecordedPatternSlot slot in settings.SavedPatterns.Concat(settings.SavedAnimPatterns))
        {
            foreach (ProfileTile? cell in slot.Pattern?.Grid?.Cells ?? [])
            {
                if (cell is not null)
                {
                    yield return cell;
                }
            }
        }
    }

    /// <summary>
    /// An iNES file (SPEC-02 §5.1) whose CHR zone holds <paramref name="tiles"/>, each sixteen
    /// bytes at a position that is <b>not</b> a multiple of sixteen and each mirrored by
    /// <paramref name="flip"/> — the shape of the synthetic file REQ-PROF-078 asks for. Nothing
    /// here depends on the space between two tiles resolving nothing, as the criterion of
    /// REQ-PROF-078 does, so the padding is left at zero.
    /// </summary>
    private static byte[] SyntheticIneFile(List<byte[]> tiles, TileFlip flip)
    {
        const int prgSize = 16 * 1024;
        const int chrBankSize = 8 * 1024;
        const int firstOffset = 8;
        const int stride = 32;

        int needed = firstOffset + (Math.Max(1, tiles.Count) * stride);
        int chrBanks = Math.Min(255, ((needed + chrBankSize - 1) / chrBankSize) + 1);
        int chrStart = 16 + prgSize;
        byte[] file = new byte[chrStart + (chrBanks * chrBankSize)];
        file[0] = 0x4E;
        file[1] = 0x45;
        file[2] = 0x53;
        file[3] = 0x1A;
        file[4] = 1;
        file[5] = (byte)chrBanks;

        for (int index = 0; index < tiles.Count; index++)
        {
            RawTile.Flip(
                tiles[index], flip, file.AsSpan(chrStart + firstOffset + (index * stride), 16));
        }

        return file;
    }

    private static ProfileDocument Load(string path)
    {
        ProfileLoadResult result = ProfileFormat.Load(File.ReadAllBytes(path), sourceName: path);
        Assert.True(result.IsSuccess, $"{path}: {result.Message}");
        return result.Document!;
    }

    /// <summary>The records and the palette of the synthetic frame both conversions are given.</summary>
    private sealed record FrameBatchPlan(TileRecord[] Records, byte[] Palette);
}

/// <summary>
/// A fact that needs <c>assets-neutral/profiles</c> and <c>assets-neutral/profiles-2.1</c>, and
/// no dump at all.
/// </summary>
public sealed class HashAssetsFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public HashAssetsFactAttribute()
    {
        if (!NeutralAssets.Available || !NeutralAssets.HashProfilesAvailable)
        {
            Skip = NeutralAssets.MissingMessage;
        }
    }
}
