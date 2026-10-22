namespace Zax.Profiles.Tests;

/// <summary>
/// Field-by-field comparison of two loaded profiles, used for the lossless rewrite of
/// SPEC-03 REQ-PROF-050: "decode, write, decode gives a model equal field by field, list orders
/// and shared identities included".
/// </summary>
/// <remarks>
/// Sharing is checked as well as the values: the comparer keeps a map between the objects of
/// the two models and fails when one profile shares an object where the other does not.
/// </remarks>
internal static class ProfileModelComparer
{
    public static void AssertEqual(GameProfile expected, GameProfile actual, string source)
    {
        Dictionary<object, object> pairs = new(ReferenceEqualityComparer.Instance);

        Assert.Equal(expected.Signature, actual.Signature);
        Assert.Equal(expected.VersionText, actual.VersionText);
        Assert.Equal(expected.Id, actual.Id);
        Assert.Equal(expected.Title, actual.Title);
        Assert.Equal(expected.Author, actual.Author);
        Assert.Equal(expected.Publisher, actual.Publisher);
        Assert.Equal(expected.Description, actual.Description);

        Assert.Equal(expected.Scripts.Count, actual.Scripts.Count);
        for (int i = 0; i < expected.Scripts.Count; i++)
        {
            AssertScript(expected.Scripts[i], actual.Scripts[i], $"{source} script {i}");
        }

        AssertPatternList(expected.PermanentPatterns, actual.PermanentPatterns, pairs, $"{source} patterns");
        AssertPatternList(expected.TemporaryPatterns, actual.TemporaryPatterns, pairs, $"{source} temp patterns");

        Assert.Equal(expected.Settings is null, actual.Settings is null);
        if (expected.Settings is not null)
        {
            AssertSettings(expected.Settings, actual.Settings!, pairs, source);
        }
    }

    private static void AssertScript(ProfileScript expected, ProfileScript actual, string where)
    {
        Assert.Equal(expected.Name, actual.Name);
        Assert.Equal(expected.Enabled, actual.Enabled);
        Assert.Equal(expected.StoredErrorFlag, actual.StoredErrorFlag);
        Assert.Equal(expected.Source, actual.Source);
        Assert.Equal(expected.TagString, actual.TagString);
        Assert.NotNull(where);
    }

    private static void AssertPatternList(
        IList<ProfilePattern> expected,
        IList<ProfilePattern> actual,
        Dictionary<object, object> pairs,
        string where)
    {
        Assert.Equal(expected.Count, actual.Count);
        for (int i = 0; i < expected.Count; i++)
        {
            AssertPattern(expected[i], actual[i], pairs, $"{where}[{i}]");
        }
    }

    private static void AssertPattern(
        ProfilePattern expected,
        ProfilePattern actual,
        Dictionary<object, object> pairs,
        string where)
    {
        if (!AssertSameSharing(expected, actual, pairs, where))
        {
            return;
        }

        Assert.Equal(expected.ConstraintSize, actual.ConstraintSize);
        Assert.Equal(expected.TileSize, actual.TileSize);
        Assert.Equal(expected.Permanent, actual.Permanent);
        Assert.Equal(expected.PixelCount, actual.PixelCount);
        Assert.Equal(expected.ColorFlags, actual.ColorFlags);
        Assert.Equal(expected.TileDensity, actual.TileDensity);
        Assert.Equal(expected.PixelDensity, actual.PixelDensity);
        Assert.Equal(expected.SelfConnected, actual.SelfConnected);
        Assert.Equal(expected.SkipGeometry, actual.SkipGeometry);

        Assert.Equal(expected.Grid is null, actual.Grid is null);
        if (expected.Grid is not null)
        {
            PatternGrid expectedGrid = expected.Grid;
            PatternGrid actualGrid = actual.Grid!;
            Assert.Equal(expectedGrid.Width, actualGrid.Width);
            Assert.Equal(expectedGrid.Height, actualGrid.Height);
            Assert.Equal(expectedGrid.Cells.Count, actualGrid.Cells.Count);
            for (int i = 0; i < expectedGrid.Cells.Count; i++)
            {
                ProfileTile? expectedCell = expectedGrid.Cells[i];
                ProfileTile? actualCell = actualGrid.Cells[i];
                Assert.Equal(expectedCell is null, actualCell is null);
                if (expectedCell is null)
                {
                    continue;
                }

                if (AssertSameSharing(expectedCell, actualCell!, pairs, $"{where} cell {i}"))
                {
                    Assert.Equal(expectedCell, actualCell);
                }
            }
        }

        Assert.Equal(expected.Variants.Count, actual.Variants.Count);
        for (int i = 0; i < expected.Variants.Count; i++)
        {
            AssertVariant(expected.Variants[i], actual.Variants[i], pairs, $"{where} variant {i}");
        }
    }

    private static void AssertVariant(
        PatternVariant expected,
        PatternVariant actual,
        Dictionary<object, object> pairs,
        string where)
    {
        if (!AssertSameSharing(expected, actual, pairs, where))
        {
            return;
        }

        Assert.Equal(expected.Depth, actual.Depth);
        Assert.Equal(expected.Geometry, actual.Geometry);
        Assert.Equal(expected.AntiShadow, actual.AntiShadow);
        Assert.Equal(expected.Flip, actual.Flip);
        Assert.Equal(expected.Solid, actual.Solid);
        Assert.Equal(expected.FullWarp, actual.FullWarp);
        Assert.Equal(expected.Layer, actual.Layer);
        Assert.Equal(expected.Scale, actual.Scale);
        Assert.Equal(expected.Rotation, actual.Rotation);
        Assert.Equal(expected.Offset, actual.Offset);
        Assert.Equal(expected.Pivot, actual.Pivot);
        Assert.Equal(BitConverter.SingleToInt32Bits(expected.Alpha), BitConverter.SingleToInt32Bits(actual.Alpha));
        Assert.Equal(expected.MeshAlgorithm, actual.MeshAlgorithm);
        Assert.Equal(expected.DeformAmplitude, actual.DeformAmplitude);
        Assert.Equal(expected.DeformPivot, actual.DeformPivot);
        Assert.Equal(expected.DeformSpeed, actual.DeformSpeed);
        Assert.Equal(expected.DeformOffset, actual.DeformOffset);
        Assert.Equal(expected.RenderStyle, actual.RenderStyle);
        Assert.Equal(expected.TagString, actual.TagString);
        Assert.Equal(expected.StoredEmptyColor, actual.StoredEmptyColor);

        Assert.Equal(expected.Texture is null, actual.Texture is null);
        if (expected.Texture is not null)
        {
            VariantTexture expectedTexture = expected.Texture;
            VariantTexture actualTexture = actual.Texture!;
            Assert.Equal(expectedTexture.Width, actualTexture.Width);
            Assert.Equal(expectedTexture.Height, actualTexture.Height);
            Assert.Equal(expectedTexture.Pixels is null, actualTexture.Pixels is null);
            if (expectedTexture.Pixels is not null)
            {
                Assert.Equal(expectedTexture.Pixels.Format, actualTexture.Pixels!.Format);
                Assert.Equal(expectedTexture.Pixels.Data, actualTexture.Pixels.Data);
                Assert.Equal(expectedTexture.Pixels.Extra, actualTexture.Pixels.Extra);
            }
        }
    }

    private static void AssertSettings(
        ProfileSettings expected,
        ProfileSettings actual,
        Dictionary<object, object> pairs,
        string where)
    {
        Assert.Equal(expected.Light is null, actual.Light is null);
        if (expected.Light is not null)
        {
            Assert.Equal(expected.Light.Direction, actual.Light!.Direction);
            Assert.Equal(expected.Light.Color, actual.Light.Color);
            Assert.Equal(
                BitConverter.SingleToInt32Bits(expected.Light.Intensity),
                BitConverter.SingleToInt32Bits(actual.Light.Intensity));
        }

        Assert.Equal(expected.UseNesBackgroundColor, actual.UseNesBackgroundColor);
        Assert.Equal(expected.CustomBackgroundColor, actual.CustomBackgroundColor);
        Assert.Equal(expected.Clipping, actual.Clipping);
        Assert.Equal(expected.MapEnable, actual.MapEnable);

        Assert.Equal(expected.Layers is null, actual.Layers is null);
        if (expected.Layers is not null)
        {
            Assert.Equal(expected.Layers.DefaultLayer, actual.Layers!.DefaultLayer);
            Assert.Equal(expected.Layers.Auto, actual.Layers.Auto);
            Assert.Equal(expected.Layers.Layers.Count, actual.Layers.Layers.Count);
            for (int i = 0; i < expected.Layers.Layers.Count; i++)
            {
                LayerSettings expectedLayer = expected.Layers.Layers[i];
                LayerSettings actualLayer = actual.Layers.Layers[i];
                Assert.Equal(
                    BitConverter.SingleToInt32Bits(expectedLayer.Offset),
                    BitConverter.SingleToInt32Bits(actualLayer.Offset));
                Assert.Equal(expectedLayer.Aligned, actualLayer.Aligned);
                Assert.Equal(expectedLayer.StoredCastShadow, actualLayer.StoredCastShadow);
                Assert.Equal(expectedLayer.StoredRecvShadow, actualLayer.StoredRecvShadow);
            }
        }

        Assert.Equal(expected.GraphicQuality, actual.GraphicQuality);
        Assert.Equal(expected.VrRuntime, actual.VrRuntime);
        Assert.Equal(expected.VSync, actual.VSync);
        Assert.Equal(expected.FrameRate, actual.FrameRate);
        Assert.Equal(
            BitConverter.SingleToInt32Bits(expected.GameSpeed),
            BitConverter.SingleToInt32Bits(actual.GameSpeed));
        Assert.Equal(
            BitConverter.SingleToInt32Bits(expected.Volume),
            BitConverter.SingleToInt32Bits(actual.Volume));
        Assert.Equal(expected.MeshAlgorithm, actual.MeshAlgorithm);
        Assert.Equal(expected.ControlMethod, actual.ControlMethod);
        Assert.Equal(expected.Shadow, actual.Shadow);
        Assert.Equal(expected.SameBackgroundPalette, actual.SameBackgroundPalette);
        Assert.Equal(expected.SpriteTolerance, actual.SpriteTolerance);
        Assert.Equal(expected.ScanDirection, actual.ScanDirection);
        Assert.Equal(expected.Skybox, actual.Skybox);
        Assert.Equal(expected.Core, actual.Core);
        Assert.Equal(expected.Mirroring, actual.Mirroring);
        Assert.Equal(expected.EmptyTile, actual.EmptyTile);
        Assert.Equal(expected.LocalCamera, actual.LocalCamera);
        Assert.Equal(
            BitConverter.SingleToInt32Bits(expected.SceneRatio),
            BitConverter.SingleToInt32Bits(actual.SceneRatio));
        Assert.Equal(expected.EmulatedMenuButton, actual.EmulatedMenuButton);
        Assert.Equal(expected.RenderMethod, actual.RenderMethod);
        Assert.Equal(expected.ResolutionScaleIndex, actual.ResolutionScaleIndex);
        Assert.Equal(expected.SkipFrame, actual.SkipFrame);
        Assert.Equal(expected.BackgroundMusic, actual.BackgroundMusic);
        Assert.Equal(expected.AntiAlias, actual.AntiAlias);
        Assert.Equal(expected.AutoTvMode, actual.AutoTvMode);
        Assert.Equal(expected.TvMode, actual.TvMode);
        Assert.Equal(expected.TvBox, actual.TvBox);
        Assert.Equal(expected.SortOption, actual.SortOption);
        Assert.Equal(expected.SameSpritePalette, actual.SameSpritePalette);
        Assert.Equal(BitConverter.SingleToInt32Bits(expected.Fov), BitConverter.SingleToInt32Bits(actual.Fov));

        AssertSlots(expected.SavedPatterns, actual.SavedPatterns, pairs, $"{where} saved patterns");
        AssertSlots(expected.SavedAnimPatterns, actual.SavedAnimPatterns, pairs, $"{where} saved anim patterns");
    }

    private static void AssertSlots(
        IList<RecordedPatternSlot> expected,
        IList<RecordedPatternSlot> actual,
        Dictionary<object, object> pairs,
        string where)
    {
        Assert.Equal(expected.Count, actual.Count);
        for (int i = 0; i < expected.Count; i++)
        {
            Assert.Equal(expected[i].Name, actual[i].Name);
            Assert.Equal(expected[i].Pattern is null, actual[i].Pattern is null);
            if (expected[i].Pattern is not null)
            {
                AssertPattern(expected[i].Pattern!, actual[i].Pattern!, pairs, $"{where}[{i}]");
            }
        }
    }

    /// <summary>
    /// Pairs an object of the first model with one of the second, and fails when a previous
    /// pairing contradicts it: that is how the sharing structure is compared. Returns false
    /// when the pair was already checked, so the caller can skip the values.
    /// </summary>
    private static bool AssertSameSharing(
        object expected,
        object actual,
        Dictionary<object, object> pairs,
        string where)
    {
        if (pairs.TryGetValue(expected, out object? known))
        {
            Assert.True(
                ReferenceEquals(known, actual),
                $"{where}: an object shared in one model is not shared in the other");
            return false;
        }

        pairs.Add(expected, actual);
        return true;
    }
}
