namespace Zax.Profiles.Tests;

/// <summary>
/// The counts SPEC-03 §5.5 and §5.6 state for the 101 built-in profiles. They are the cheapest
/// way to show the reader agrees with the spec on every convention at once: a mistake in the
/// reference envelopes, the nullable cells, the texture lengths or the settings would move one
/// of these numbers.
/// </summary>
public sealed class ProfileCorpusTests
{
    private sealed record Totals(
        int Files,
        int FilesWithScripts,
        int Scripts,
        int EnabledScripts,
        int DisabledScripts,
        int PatternsField4,
        int PatternsField5,
        int SlotPatterns,
        int PatternsTotal,
        int PatternsWithGrid,
        int RectangularGrids,
        int Variants,
        int NewTiles,
        int TileReferences,
        int FilledCells,
        int EmptyCells,
        int NullMarkerTextures,
        int IndexedTextures,
        int DirectTextures,
        int AbsentTextures,
        int VariantsWithOneVariant,
        int StoredLayers,
        int StoredCastShadow,
        int Version2,
        int Version3,
        int ProfilesWithId,
        int ProfilesWithTitle,
        int ProfilesWithDescription);

    private static Totals Measure()
    {
        int files = 0;
        int filesWithScripts = 0;
        int scripts = 0;
        int enabled = 0;
        int disabled = 0;
        int patterns4 = 0;
        int patterns5 = 0;
        int slotPatterns = 0;
        int patternsTotal = 0;
        int withGrid = 0;
        int rectangular = 0;
        int variants = 0;
        int newTiles = 0;
        int tileReferences = 0;
        int filled = 0;
        int empty = 0;
        int nullMarkers = 0;
        int indexed = 0;
        int direct = 0;
        int absent = 0;
        int storedLayers = 0;
        int storedCast = 0;
        int version2 = 0;
        int version3 = 0;
        int withId = 0;
        int withTitle = 0;
        int withDescription = 0;
        Dictionary<int, int> variantHistogram = [];

        foreach (string path in NeutralAssets.ProfileFiles())
        {
            files++;
            GameProfile profile = NeutralAssets.LoadProfile(Path.GetFileName(path)).Profile;

            if (profile.Scripts.Count > 0)
            {
                filesWithScripts++;
            }

            scripts += profile.Scripts.Count;
            foreach (ProfileScript script in profile.Scripts)
            {
                if (script.Enabled)
                {
                    enabled++;
                }
                else
                {
                    disabled++;
                }
            }

            patterns4 += profile.PermanentPatterns.Count;
            patterns5 += profile.TemporaryPatterns.Count;

            if (profile.VersionText == "2")
            {
                version2++;
            }
            else if (profile.VersionText == "3")
            {
                version3++;
            }

            if (profile.Id is not null)
            {
                withId++;
            }

            if (profile.Title is not null)
            {
                withTitle++;
            }

            if (profile.Description is not null)
            {
                withDescription++;
            }

            HashSet<ProfileTile> seenTiles = new(ReferenceEqualityComparer.Instance);
            List<ProfilePattern> all = [.. profile.AllPatterns];
            foreach (ProfilePattern pattern in all)
            {
                patternsTotal++;
                variants += pattern.Variants.Count;
                variantHistogram[pattern.Variants.Count] =
                    variantHistogram.GetValueOrDefault(pattern.Variants.Count) + 1;

                if (pattern.Grid is not { } grid)
                {
                    continue;
                }

                withGrid++;
                if (grid.IsRectangular)
                {
                    rectangular++;
                }

                foreach (ProfileTile? cell in grid.Cells)
                {
                    if (cell is null)
                    {
                        empty++;
                        continue;
                    }

                    filled++;
                    if (seenTiles.Add(cell))
                    {
                        newTiles++;
                    }
                    else
                    {
                        tileReferences++;
                    }
                }
            }

            ProfileSettings settings = profile.Settings!;
            storedLayers += settings.Layers!.Layers.Count;
            foreach (LayerSettings layer in settings.Layers.Layers)
            {
                if (layer.StoredCastShadow)
                {
                    storedCast++;
                }
            }

            List<ProfilePattern> slots = [];
            foreach (RecordedPatternSlot slot in settings.SavedPatterns)
            {
                slotPatterns++;
                if (slot.Pattern is not null)
                {
                    slots.Add(slot.Pattern);
                }
            }

            foreach (RecordedPatternSlot slot in settings.SavedAnimPatterns)
            {
                slotPatterns++;
                if (slot.Pattern is not null)
                {
                    slots.Add(slot.Pattern);
                }
            }

            patternsTotal += slots.Count;

            foreach (ProfilePattern pattern in all)
            {
                CountTextures(pattern, ref nullMarkers, ref indexed, ref direct, ref absent);
            }

            foreach (ProfilePattern pattern in slots)
            {
                CountTextures(pattern, ref nullMarkers, ref indexed, ref direct, ref absent);
            }
        }

        return new Totals(
            files, filesWithScripts, scripts, enabled, disabled, patterns4, patterns5, slotPatterns,
            patternsTotal, withGrid, rectangular, variants, newTiles, tileReferences, filled, empty,
            nullMarkers, indexed, direct, absent, variantHistogram.GetValueOrDefault(1), storedLayers,
            storedCast, version2, version3, withId, withTitle, withDescription);
    }

    private static void CountTextures(
        ProfilePattern pattern,
        ref int nullMarkers,
        ref int indexed,
        ref int direct,
        ref int absent)
    {
        foreach (PatternVariant variant in pattern.Variants)
        {
            if (variant.Texture is not { } texture)
            {
                absent++;
                continue;
            }

            Assert.True(texture.HasConsistentLength(), "texture payload length");
            if (texture.IsNullMarker)
            {
                nullMarkers++;
            }
            else if (texture.Pixels?.Format == TextureFormat.Direct)
            {
                direct++;
            }
            else
            {
                indexed++;
            }
        }
    }

    private static readonly Lazy<Totals> Corpus = new(Measure);

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_023_the_two_pattern_lists_hold_the_stated_numbers()
    {
        Totals totals = Corpus.Value;
        Assert.Equal(101, totals.Files);
        Assert.Equal(66609, totals.PatternsField4);
        Assert.Equal(1223, totals.PatternsField5);
        Assert.Equal(1010, totals.SlotPatterns);
        Assert.Equal(68842, totals.PatternsTotal);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_023_every_pattern_of_field_4_is_marked_permanent()
    {
        foreach (string path in NeutralAssets.ProfileFiles())
        {
            GameProfile profile = NeutralAssets.LoadProfile(Path.GetFileName(path)).Profile;
            foreach (ProfilePattern pattern in profile.PermanentPatterns)
            {
                Assert.True(pattern.Permanent, $"{Path.GetFileName(path)}: a field 4 pattern is temporary");
            }
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_034_the_variant_counts_match()
    {
        Totals totals = Corpus.Value;
        Assert.Equal(74389, totals.Variants);
        Assert.Equal(62650, totals.VariantsWithOneVariant);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_034_the_distribution_of_variants_per_pattern_matches()
    {
        Dictionary<int, int> histogram = [];
        foreach (string path in NeutralAssets.ProfileFiles())
        {
            GameProfile profile = NeutralAssets.LoadProfile(Path.GetFileName(path)).Profile;
            foreach (ProfilePattern pattern in profile.AllPatterns)
            {
                int count = pattern.Variants.Count;
                histogram[count] = histogram.GetValueOrDefault(count) + 1;
                Assert.True(count >= 1, "a pattern of fields 4 or 5 has no variant");
            }
        }

        // §5.5 and REQ-PROF-034: 1 -> 62 650, 2 -> 4 142, 3 -> 800, 4 and more -> 240.
        Assert.Equal(62650, histogram[1]);
        Assert.Equal(4142, histogram[2]);
        Assert.Equal(800, histogram[3]);
        int fourOrMore = 0;
        foreach ((int variants, int patterns) in histogram)
        {
            if (variants >= 4)
            {
                fourOrMore += patterns;
            }
        }

        Assert.Equal(240, fourOrMore);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_012_the_cell_counts_match_and_every_grid_is_rectangular()
    {
        Totals totals = Corpus.Value;
        Assert.Equal(452231, totals.FilledCells);
        Assert.Equal(124059, totals.EmptyCells);
        Assert.Equal(67832, totals.PatternsWithGrid);
        Assert.Equal(67832, totals.RectangularGrids);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_011_tiles_are_shared_in_the_stated_proportion()
    {
        Totals totals = Corpus.Value;
        Assert.Equal(225575, totals.NewTiles);
        Assert.Equal(226656, totals.TileReferences);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_035_the_texture_counts_match()
    {
        Totals totals = Corpus.Value;
        Assert.Equal(60708, totals.NullMarkerTextures);
        Assert.Equal(13539, totals.IndexedTextures);
        Assert.Equal(142, totals.DirectTextures);

        // The 1 010 patterns of the memorised slots have no texture field at all.
        Assert.Equal(1010, totals.AbsentTextures);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_024_the_script_counts_match()
    {
        Totals totals = Corpus.Value;
        Assert.Equal(579, totals.Scripts);
        Assert.Equal(89, totals.FilesWithScripts);
        Assert.Equal(556, totals.EnabledScripts);
        Assert.Equal(23, totals.DisabledScripts);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_038_the_two_name_fields_always_agree_and_no_error_flag_is_set()
    {
        // Field 5 repeats field 1 in all 579 scripts, so writing both from one model value is
        // lossless; the byte-identity test is what proves it.
        foreach (string path in NeutralAssets.ProfileFiles())
        {
            GameProfile profile = NeutralAssets.LoadProfile(Path.GetFileName(path)).Profile;
            foreach (ProfileScript script in profile.Scripts)
            {
                Assert.False(script.StoredErrorFlag, $"{Path.GetFileName(path)}: {script.Name} has the error flag");
            }
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_021_one_hundred_profiles_are_version_2_and_one_is_version_3()
    {
        Totals totals = Corpus.Value;
        Assert.Equal(100, totals.Version2);
        Assert.Equal(1, totals.Version3);

        ProfileDocument jackal = NeutralAssets.LoadProfile("Jackal (U).bytes");
        Assert.Equal("3", jackal.Profile.VersionText);
        Assert.False(jackal.Profile.IsV2Format);

        ProfileDocument balloon = NeutralAssets.LoadProfile("BalloonFight.bytes");
        Assert.True(balloon.Profile.IsV2Format);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_022_the_metadata_fields_are_present_as_often_as_stated()
    {
        Totals totals = Corpus.Value;
        Assert.Equal(56, totals.ProfilesWithId);
        Assert.Equal(17, totals.ProfilesWithTitle);
        Assert.Equal(0, totals.ProfilesWithDescription);

        // Jackal carries the title "Tennis" but the library shows the catalogue title.
        Assert.Equal("Tennis", NeutralAssets.LoadProfile("Jackal (U).bytes").Profile.Title);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_042_the_stored_shadow_flags_are_overridden_after_loading()
    {
        Totals totals = Corpus.Value;
        Assert.Equal(606, totals.StoredLayers);
        Assert.Equal(119, totals.StoredCastShadow);

        // After loading, exactly one layer per profile casts shadows: layer 2.
        for (int i = 0; i < LayerSettingsSet.LayerCount; i++)
        {
            Assert.Equal(i == 2, LayerSettingsSet.EffectiveCastShadow(i));
            Assert.Equal(i >= 2, LayerSettingsSet.EffectiveRecvShadow(i));
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetTheory]
    [InlineData("empty.bytes", 3780, 1, 0, 0, 0, 0)]
    [InlineData("Puzznic (U).bytes", 44334, 0, 74, 0, 75, 311)]
    [InlineData("flappy_fpv.bytes", 66849, 11, 67, 0, 77, 149)]
    [InlineData("Super Mario Bros.bytes", 179515, 10, 257, 1, 296, 783)]
    [InlineData("Contra.bytes", 1120081, 3, 1854, 65, 1966, 5365)]
    public void REQ_PROF_001_the_small_profiles_decode_to_the_stated_contents(
        string fileName,
        int size,
        int scripts,
        int permanentPatterns,
        int temporaryPatterns,
        int variants,
        int distinctTiles)
    {
        byte[] bytes = NeutralAssets.ReadProfile(fileName);
        Assert.Equal(size, bytes.Length);

        GameProfile profile = NeutralAssets.LoadProfile(fileName).Profile;
        Assert.Equal(scripts, profile.Scripts.Count);
        Assert.Equal(permanentPatterns, profile.PermanentPatterns.Count);
        Assert.Equal(temporaryPatterns, profile.TemporaryPatterns.Count);

        int variantCount = 0;
        foreach (ProfilePattern pattern in profile.AllPatterns)
        {
            variantCount += pattern.Variants.Count;
        }

        Assert.Equal(variants, variantCount);
        Assert.Equal(distinctTiles, profile.DistinctTileObjects.Count());
    }
}
