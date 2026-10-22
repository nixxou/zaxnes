using Zax.Profiles.Wire;

namespace Zax.Profiles.Tests;

/// <summary>
/// The two round-trip requirements of SPEC-03 on the 101 built-in profiles: decoding keeps
/// every field (REQ-PROF-004, REQ-PROF-050) and the canonical writer reproduces each file byte
/// for byte (REQ-PROF-051, REQ-ACC-010).
/// </summary>
public sealed class ProfileRoundTripTests
{
    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_001_every_built_in_profile_decodes_with_no_unknown_field()
    {
        List<string> failures = [];
        int unknownFields = 0;
        int count = 0;
        foreach (string path in NeutralAssets.ProfileFiles())
        {
            count++;
            ProfileLoadResult result = ProfileFormat.Load(File.ReadAllBytes(path), sourceName: path);
            if (!result.IsSuccess)
            {
                failures.Add($"{Path.GetFileName(path)}: {result.Message}");
                continue;
            }

            unknownFields += result.Document!.UnknownFieldCount;
        }

        Assert.Empty(failures);
        Assert.Equal(101, count);
        Assert.Equal(0, unknownFields);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_051_the_canonical_writer_reproduces_every_built_in_profile_byte_for_byte()
    {
        List<string> failures = [];
        int identical = 0;
        foreach (string path in NeutralAssets.ProfileFiles())
        {
            byte[] original = File.ReadAllBytes(path);
            ProfileLoadResult result = ProfileFormat.Load(original, sourceName: path);
            if (!result.IsSuccess)
            {
                failures.Add($"{Path.GetFileName(path)}: {result.Message}");
                continue;
            }

            byte[] rewritten = result.Document!.Save();
            if (rewritten.AsSpan().SequenceEqual(original))
            {
                identical++;
                continue;
            }

            failures.Add(
                $"{Path.GetFileName(path)}: {DescribeFirstDifference(original, rewritten)}");
        }

        Assert.Empty(failures);
        Assert.Equal(101, identical);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_050_decoding_a_rewritten_profile_gives_the_same_model()
    {
        foreach (string path in NeutralAssets.ProfileFiles())
        {
            ProfileDocument first = NeutralAssets.LoadProfile(Path.GetFileName(path));
            byte[] rewritten = first.Save();
            ProfileLoadResult second = ProfileFormat.Load(rewritten, sourceName: path);
            Assert.True(second.IsSuccess, $"{path}: {second.Message}");
            ProfileModelComparer.AssertEqual(first.Profile, second.Document!.Profile, Path.GetFileName(path));
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_011_shared_tiles_are_resolved_to_the_same_object()
    {
        ProfileDocument document = NeutralAssets.LoadProfile("BalloonFight.bytes");
        int filledCells = 0;
        foreach (ProfilePattern pattern in document.Profile.AllPatterns)
        {
            filledCells += pattern.Grid?.FilledCellCount ?? 0;
        }

        int distinctObjects = document.Profile.DistinctTileObjects.Count();

        // 516 tile objects carry 1 085 filled cells: the rest are shared references
        // (REQ-PROF-011, §5.3).
        Assert.Equal(516, distinctObjects);
        Assert.True(filledCells > distinctObjects, "tiles are expected to be shared");
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_004_a_profile_built_in_memory_writes_every_scalar()
    {
        // Without a layout the writer follows REQ-PROF-051 literally: every scalar written,
        // nulls included. Decoding it back gives a model with the same values.
        GameProfile profile = GameProfile.CreateEmpty();
        ProfilePattern pattern = new()
        {
            TileSize = new Vec2I(1, 1),
            Permanent = true,
            SkipGeometry = false,
        };
        pattern.Variants.Add(new PatternVariant { Depth = 3, Alpha = 1f });
        pattern.Grid = new PatternGrid(1, 1, [new ProfileTile(0x0020201800000000, 0x103A3C1800000000, TileFlip.None)]);
        profile.PermanentPatterns.Add(pattern);

        byte[] bytes = ProfileFormat.Save(profile);
        ProfileLoadResult reloaded = ProfileFormat.Load(bytes);

        Assert.True(reloaded.IsSuccess, reloaded.Message);
        ProfilePattern restored = Assert.Single(reloaded.Profile!.PermanentPatterns);
        Assert.Equal(new Vec2I(1, 1), restored.TileSize);
        Assert.True(restored.Permanent);
        Assert.Equal(Vec2I.Unconstrained, restored.ConstraintSize);
        Assert.Single(restored.Variants);
        Assert.Equal(3, restored.Variants[0].Depth);
        Assert.Equal(0x103A3C1800000000UL, restored.Grid!.Cells[0]!.HighPlane);

        // All twelve pattern fields were written, so the layout records no deviation.
        IReadOnlyList<int>? recorded = reloaded.Document!.Layout.PresentFields(restored);
        Assert.Null(recorded);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_051_the_recorded_field_sets_are_only_the_ones_the_files_leave_out()
    {
        ProfileDocument empty = NeutralAssets.LoadProfile("empty.bytes");
        IReadOnlyList<int>? settingsFields = empty.Layout.PresentFields(empty.Profile.Settings!);

        // empty.bytes stores settings fields 1 to 23 only (§5.4).
        Assert.NotNull(settingsFields);
        Assert.Equal(Enumerable.Range(1, 23), settingsFields!);
    }

    private static string DescribeFirstDifference(byte[] expected, byte[] actual)
    {
        int common = Math.Min(expected.Length, actual.Length);
        for (int i = 0; i < common; i++)
        {
            if (expected[i] != actual[i])
            {
                return $"byte {i} is 0x{actual[i]:X2}, expected 0x{expected[i]:X2} "
                    + $"(lengths {actual.Length} and {expected.Length})";
            }
        }

        return $"length {actual.Length}, expected {expected.Length}";
    }
}
