using Zax.Profiles.Wire;

namespace Zax.Profiles.Tests;

/// <summary>
/// The script order of SPEC-03 REQ-PROF-024 and SPEC-05 REQ-LUA-004, as revised by DEC-03-Q02:
/// a non-ordinal, culture-independent collation, which is also the order the Lua hooks are
/// called in.
/// </summary>
public sealed class ScriptOrderTests
{
    private static IReadOnlyList<string> Sort(params string[] names)
    {
        List<string> sorted = [.. names];
        sorted.Sort(ScriptNameComparer.Instance);
        return sorted;
    }

    [Fact]
    public void REQ_PROF_024_punctuation_sorts_before_digits_which_sort_before_letters()
    {
        // The Excitebike example: the underscore comes before the digit 7, which an ordinal
        // sort would get the other way round and would change that profile's hook order.
        Assert.Equal(
            ["bike", "camera", "ground", "ramp_rotation", "ramp7", "scene"],
            Sort("scene", "ramp7", "ramp_rotation", "camera", "bike", "ground"));
    }

    [Fact]
    public void REQ_PROF_024_case_only_decides_at_the_last_level_lowercase_first()
    {
        Assert.Equal(["a", "b", "B"], Sort("B", "a", "b"));
    }

    [Fact]
    public void REQ_PROF_024_a_shorter_name_comes_first_when_the_primary_keys_agree()
    {
        Assert.Equal(["bg", "bg'", "bg1"], Sort("bg1", "bg'", "bg"));
    }

    [Fact]
    public void REQ_PROF_024_the_comparison_is_a_total_order()
    {
        string[] names = ["bg", "BG", "bg1", "bg'", "b g", "bg_", "Bg", "a", "A9", "a_"];
        IReadOnlyList<string> sorted = Sort(names);

        for (int i = 1; i < sorted.Count; i++)
        {
            Assert.True(
                ScriptNameComparer.Instance.Compare(sorted[i - 1], sorted[i]) <= 0,
                $"{sorted[i - 1]} should not come after {sorted[i]}");
        }

        // Comparing a name with itself is zero, and the comparison is antisymmetric.
        foreach (string left in names)
        {
            Assert.Equal(0, ScriptNameComparer.Instance.Compare(left, left));
            foreach (string right in names)
            {
                int forward = ScriptNameComparer.Instance.Compare(left, right);
                int backward = ScriptNameComparer.Instance.Compare(right, left);
                Assert.Equal(Math.Sign(forward), -Math.Sign(backward));
            }
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_024_the_order_in_every_built_in_file_is_already_the_sorted_order()
    {
        int checkedProfiles = 0;
        foreach (string path in NeutralAssets.ProfileFiles())
        {
            // Decoded without the loader, so the scripts are in file order.
            ProfileDocument document = ProfileWireCodec.Decode(File.ReadAllBytes(path));
            if (document.Profile.Scripts.Count == 0)
            {
                continue;
            }

            checkedProfiles++;
            List<string?> fileOrder = [.. document.Profile.Scripts.Select(script => script.Name)];
            List<string?> sorted = [.. fileOrder];
            sorted.Sort(ScriptNameComparer.Instance);

            Assert.Equal(fileOrder, sorted);
        }

        Assert.Equal(89, checkedProfiles);
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_024_excitebike_keeps_the_documented_order_after_loading()
    {
        GameProfile profile = NeutralAssets.LoadProfile("Excitebike.bytes").Profile;

        Assert.Equal(
            ["bike", "camera", "ground", "ramp_rotation", "ramp7", "scene"],
            profile.Scripts.Select(script => script.Name));
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_024_loading_sorts_the_scripts()
    {
        // The loader sorts, so a file whose scripts were out of order would still expose the
        // order of REQ-LUA-004.
        GameProfile profile = new();
        profile.Scripts.Add(new ProfileScript("scene", true, null, null));
        profile.Scripts.Add(new ProfileScript("bike", true, null, null));
        profile.Scripts.Add(new ProfileScript("ramp7", true, null, null));
        profile.Scripts.Add(new ProfileScript("ramp_rotation", true, null, null));
        profile.Settings = GameProfile.CreateEmpty().Settings;

        ProfileLoadResult result = ProfileFormat.Load(ProfileFormat.Save(profile));

        Assert.True(result.IsSuccess, result.Message);
        Assert.Equal(
            ["bike", "ramp_rotation", "ramp7", "scene"],
            result.Profile!.Scripts.Select(script => script.Name));
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_036_script_tags_bind_to_the_variants_that_share_one()
    {
        // Mega Man 2: the script "gear" carries the tags "gear,gear1".
        GameProfile profile = NeutralAssets.LoadProfile("Mega Man 2.bytes").Profile;
        ProfileScript? gear = profile.Scripts.FirstOrDefault(
            script => string.Equals(script.Name, "gear", StringComparison.Ordinal));

        Assert.NotNull(gear);
        Assert.Equal(["gear", "gear1"], gear!.Tags);
        Assert.False(gear.BindsToEveryVariant);

        PatternVariant tagged = new() { TagString = "gear1" };
        PatternVariant other = new() { TagString = "hud" };
        Assert.True(gear.Binds(tagged));
        Assert.False(gear.Binds(other));
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_PROF_036_a_star_in_the_tag_string_binds_a_script_to_everything()
    {
        ProfileScript build = NeutralAssets.LoadProfile("empty.bytes").Profile.Scripts[0];

        Assert.Equal("*", build.TagString);
        Assert.True(build.BindsToEveryVariant);
        Assert.True(build.Binds(new PatternVariant { TagString = null }));
    }
}
