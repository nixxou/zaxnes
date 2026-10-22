namespace Zax.Profiles.Tests;

/// <summary>
/// The rules that decide whether a profile loads at all: the signature (REQ-PROF-020), the
/// version (REQ-PROF-021), the settings a profile must carry (REQ-PROF-025, REQ-PROF-043), the
/// all-or-nothing refusal (REQ-PROF-033) and the robustness against corrupted files
/// (REQ-ACC-041).
/// </summary>
public sealed class ProfileValidationTests
{
    private static GameProfile ValidProfile()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        foreach (RecordedPatternSlot slot in RecordedPatternSlot.DefaultShapeSlots())
        {
            profile.Settings!.SavedPatterns.Add(slot);
        }

        foreach (RecordedPatternSlot slot in RecordedPatternSlot.DefaultDeformSlots())
        {
            profile.Settings!.SavedAnimPatterns.Add(slot);
        }

        return profile;
    }

    private static ProfileLoadResult Load(GameProfile profile)
        => ProfileFormat.Load(ProfileFormat.Save(profile));

    [Fact]
    public void REQ_PROF_020_a_profile_whose_signature_is_not_3dn_is_refused()
    {
        GameProfile profile = ValidProfile();
        profile.Signature = "3dx";

        ProfileLoadResult result = Load(profile);

        Assert.False(result.IsSuccess);
        Assert.Equal(ProfileLoadError.WrongSignature, result.Error);
        Assert.Empty(result.ProfileOrEmpty().PermanentPatterns);
    }

    [Fact]
    public void REQ_PROF_020_an_absent_signature_is_not_an_error()
    {
        GameProfile profile = ValidProfile();
        profile.Signature = null;

        Assert.True(Load(profile).IsSuccess);
    }

    [Theory]
    [InlineData("1", ProfileFormatVersion.Unsupported)]
    [InlineData("v1", ProfileFormatVersion.Unsupported)]
    [InlineData("2", ProfileFormatVersion.V2)]
    [InlineData("3", ProfileFormatVersion.Current)]
    [InlineData("2.0", ProfileFormatVersion.Current)]
    [InlineData("$3", ProfileFormatVersion.Current)]
    [InlineData("", ProfileFormatVersion.Current)]
    [InlineData(null, ProfileFormatVersion.Current)]
    public void REQ_PROF_021_the_version_string_is_filtered_then_classified(
        string? versionText,
        ProfileFormatVersion expected)
    {
        Assert.Equal(expected, ProfileVersionRules.Classify(versionText));
    }

    [Theory]
    [InlineData("v1", "1")]
    [InlineData("ver 2.0", "2.0")]
    [InlineData("3$", "3$")]
    [InlineData("a-b", "")]
    public void REQ_PROF_021_filtering_keeps_only_digits_a_dot_and_a_dollar(string input, string expected)
    {
        Assert.Equal(expected, ProfileVersionRules.Sanitize(input));
    }

    [Fact]
    public void REQ_PROF_021_a_version_1_profile_is_refused_and_a_version_2_profile_is_flagged()
    {
        GameProfile refused = ValidProfile();
        refused.VersionText = "1";
        ProfileLoadResult result = Load(refused);
        Assert.False(result.IsSuccess);
        Assert.Equal(ProfileLoadError.UnsupportedVersion, result.Error);

        GameProfile v2 = ValidProfile();
        v2.VersionText = "2";
        ProfileLoadResult loaded = Load(v2);
        Assert.True(loaded.IsSuccess);
        Assert.True(loaded.Profile!.IsV2Format);

        GameProfile v3 = ValidProfile();
        v3.VersionText = "2.0";
        Assert.False(Load(v3).Profile!.IsV2Format);
    }

    [Fact]
    public void REQ_PROF_025_a_profile_without_settings_is_refused()
    {
        GameProfile profile = ValidProfile();
        profile.Settings = null;

        ProfileLoadResult result = Load(profile);

        Assert.False(result.IsSuccess);
        Assert.Equal(ProfileLoadError.MissingSettings, result.Error);
    }

    [Fact]
    public void REQ_PROF_025_a_profile_whose_settings_lack_the_light_the_clipping_or_the_layers_is_refused()
    {
        GameProfile withoutLight = ValidProfile();
        withoutLight.Settings!.Light = null;
        Assert.Equal(ProfileLoadError.IncompleteSettings, Load(withoutLight).Error);

        GameProfile withoutClipping = ValidProfile();
        withoutClipping.Settings!.Clipping = null;
        Assert.Equal(ProfileLoadError.IncompleteSettings, Load(withoutClipping).Error);

        GameProfile withoutLayers = ValidProfile();
        withoutLayers.Settings!.Layers = null;
        Assert.Equal(ProfileLoadError.IncompleteSettings, Load(withoutLayers).Error);
    }

    [Fact]
    public void REQ_PROF_043_a_profile_that_does_not_hold_six_layers_is_refused()
    {
        GameProfile profile = ValidProfile();
        profile.Settings!.Layers!.Layers.RemoveAt(5);

        ProfileLoadResult result = Load(profile);

        Assert.False(result.IsSuccess);
        Assert.Equal(ProfileLoadError.WrongLayerCount, result.Error);
    }

    [Fact]
    public void REQ_PROF_025_a_profile_without_the_memorised_slots_is_accepted_and_the_defaults_rebuilt()
    {
        // DEC-03-Q03, an accepted divergence: the original refuses such a profile.
        GameProfile profile = GameProfile.CreateEmpty();
        Assert.Empty(profile.Settings!.SavedPatterns);

        ProfileLoadResult result = Load(profile);

        Assert.True(result.IsSuccess);
        Assert.Equal(
            new[] { "Config1", "Config2", "Config3", "Config4", "Config5" },
            result.Profile!.Settings!.SavedPatterns.Select(slot => slot.Name));
        Assert.Equal(
            new[] { "Deform1", "Deform2", "Deform3", "Deform4", "Deform5" },
            result.Profile.Settings.SavedAnimPatterns.Select(slot => slot.Name));

        // The rebuilt slots are not written back: the file said nothing about them.
        byte[] rewritten = result.Document!.Save();
        Assert.Equal(ProfileFormat.Save(profile), rewritten);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetTheory]
    [InlineData(0)]
    [InlineData(2)]
    [InlineData(5)]
    public void REQ_PROF_001_an_unknown_field_is_skipped_and_the_model_is_unchanged(int wireType)
    {
        // REQ-PROF-001: a file with field 99 of any wire type decodes to the same model.
        byte[] original = NeutralAssets.ReadProfile("empty.bytes");
        ProfileDocument reference = NeutralAssets.LoadProfile("empty.bytes");

        byte[] tag = [(byte)(0x98 | wireType), 0x06];
        byte[] payload = wireType switch
        {
            0 => [0x2a],
            2 => [0x03, 0x61, 0x62, 0x63],
            _ => [0x00, 0x00, 0x00, 0x00],
        };

        byte[] patched = [.. original, .. tag, .. payload];
        ProfileLoadResult result = ProfileFormat.Load(patched);

        Assert.True(result.IsSuccess, result.Message);
        Assert.Equal(1, result.Document!.UnknownFieldCount);
        ProfileModelComparer.AssertEqual(reference.Profile, result.Profile!, "empty.bytes + field 99");
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_033_a_profile_truncated_in_half_loads_as_an_empty_profile()
    {
        byte[] original = NeutralAssets.ReadProfile("BalloonFight.bytes");
        byte[] half = original.AsSpan(0, original.Length / 2).ToArray();

        ProfileLoadResult result = ProfileFormat.Load(half);

        Assert.False(result.IsSuccess);
        Assert.Equal(ProfileLoadError.Malformed, result.Error);

        GameProfile fallback = result.ProfileOrEmpty();
        Assert.Empty(fallback.PermanentPatterns);
        Assert.Empty(fallback.TemporaryPatterns);
        Assert.Empty(fallback.Scripts);
        Assert.NotNull(fallback.Settings);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_ACC_041_no_truncation_of_a_profile_ever_throws()
    {
        byte[] original = NeutralAssets.ReadProfile("Puzznic (U).bytes");
        for (int length = 0; length <= original.Length; length += 97)
        {
            byte[] truncated = original.AsSpan(0, length).ToArray();

            // Either it is refused or it decodes; what matters is that it never throws and
            // never leaves a half-built profile behind.
            ProfileLoadResult result = ProfileFormat.Load(truncated);
            if (result.IsSuccess)
            {
                Assert.NotNull(result.Profile!.Settings);
            }
            else
            {
                Assert.Null(result.Profile);
            }
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_ACC_041_no_single_byte_corruption_of_a_profile_ever_throws()
    {
        byte[] original = NeutralAssets.ReadProfile("empty.bytes");
        int accepted = 0;
        int refused = 0;
        for (int index = 0; index < original.Length; index += 7)
        {
            byte[] corrupted = [.. original];
            corrupted[index] ^= 0xFF;

            ProfileLoadResult result = ProfileFormat.Load(corrupted);
            if (result.IsSuccess)
            {
                accepted++;
            }
            else
            {
                refused++;
            }
        }

        Assert.True(accepted + refused > 0);
    }

    [Fact]
    public void REQ_ACC_041_random_bytes_are_refused_without_throwing()
    {
        Random random = new(0x3D5E);
        for (int attempt = 0; attempt < 200; attempt++)
        {
            byte[] noise = new byte[random.Next(0, 512)];
            random.NextBytes(noise);

            ProfileLoadResult result = ProfileFormat.Load(noise);
            if (result.IsSuccess)
            {
                // Random bytes that happen to parse must still have passed every rule.
                Assert.NotNull(result.Profile!.Settings);
            }
        }
    }

    [Fact]
    public void REQ_PROF_011_a_reference_to_an_unknown_key_is_refused()
    {
        // Root field 4 holding an envelope whose field 1 names key 7, which no object has.
        byte[] bytes = [0x0a, 0x03, 0x33, 0x64, 0x6e, 0x22, 0x02, 0x08, 0x07];

        ProfileLoadResult result = ProfileFormat.Load(bytes);

        Assert.False(result.IsSuccess);
        Assert.Equal(ProfileLoadError.Malformed, result.Error);
        Assert.Contains("unknown key", result.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void REQ_PROF_011_a_key_out_of_sequence_is_refused()
    {
        // Root field 4 holding an envelope with the new key 4 instead of 1.
        byte[] bytes = [0x0a, 0x03, 0x33, 0x64, 0x6e, 0x22, 0x04, 0x10, 0x04, 0x52, 0x00];

        ProfileLoadResult result = ProfileFormat.Load(bytes);

        Assert.False(result.IsSuccess);
        Assert.Equal(ProfileLoadError.Malformed, result.Error);
        Assert.Contains("out of sequence", result.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void REQ_PROF_033_a_refused_profile_is_logged()
    {
        RecordingProfileLog log = new();
        ProfileFormat.Load([0x0a, 0x03, 0x33, 0x64, 0x78], log, "Broken.3dn");

        Assert.Contains(log.Errors, line => line.StartsWith("Broken.3dn:", StringComparison.Ordinal));
    }
}
