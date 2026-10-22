namespace Zax.Profiles.Tests;

/// <summary>The normalisations of a newly exported profile (SPEC-03 REQ-PROF-052).</summary>
public sealed class ProfileExportTests
{
    [Fact]
    public void REQ_PROF_052_an_exported_profile_gets_the_current_signature_and_version()
    {
        GameProfile profile = new() { Signature = null, VersionText = "2" };

        ProfileExport.Normalize(profile);

        Assert.Equal("3dn", profile.Signature);
        Assert.Equal("3", profile.VersionText);
        Assert.Equal(ProfileFormatVersion.Current, profile.FormatVersion);
    }

    [Fact]
    public void REQ_PROF_052_a_profile_without_an_identifier_gets_a_twelve_character_one()
    {
        GameProfile profile = new();

        ProfileExport.Normalize(profile);

        Assert.NotNull(profile.Id);
        Assert.Equal(ProfileExport.IdentifierLength, profile.Id!.Length);
        foreach (char character in profile.Id)
        {
            Assert.True(
                char.IsAsciiLetterOrDigit(character) || character is '+' or '/',
                $"\"{character}\" is not part of the standard base64 alphabet");
        }

        // An identifier that is already there is kept.
        GameProfile known = new() { Id = "Lc89vRjlYrUL" };
        ProfileExport.Normalize(known);
        Assert.Equal("Lc89vRjlYrUL", known.Id);

        // Two generated identifiers differ.
        Assert.NotEqual(ProfileExport.NewIdentifier(), ProfileExport.NewIdentifier());
    }

    [Fact]
    public void REQ_PROF_052_an_empty_title_takes_the_rom_file_name()
    {
        GameProfile empty = new() { Title = string.Empty };
        ProfileExport.Normalize(empty, "Arkanoid (U)");
        Assert.Equal("Arkanoid (U)", empty.Title);

        // An absent title stays absent, and a real title is kept.
        GameProfile absent = new();
        ProfileExport.Normalize(absent, "Arkanoid (U)");
        Assert.Null(absent.Title);

        GameProfile titled = new() { Title = "Mine" };
        ProfileExport.Normalize(titled, "Arkanoid (U)");
        Assert.Equal("Mine", titled.Title);
    }

    [Fact]
    public void REQ_PROF_052_patterns_are_filed_by_their_own_flag_and_scripts_are_sorted()
    {
        GameProfile profile = new();
        ProfilePattern permanent = new() { Permanent = true };
        ProfilePattern temporary = new() { Permanent = false };
        ProfilePattern alsoPermanent = new() { Permanent = true };
        profile.PermanentPatterns.Add(permanent);
        profile.PermanentPatterns.Add(temporary);
        profile.TemporaryPatterns.Add(alsoPermanent);
        profile.Scripts.Add(new ProfileScript("scene", true, null, null));
        profile.Scripts.Add(new ProfileScript("bike", true, null, null));

        ProfileExport.Normalize(profile);

        Assert.Equal([permanent, alsoPermanent], profile.PermanentPatterns);
        Assert.Equal([temporary], profile.TemporaryPatterns);
        Assert.Equal(["bike", "scene"], profile.Scripts.Select(script => script.Name));
    }

    [Fact]
    public void REQ_PROF_052_an_exported_profile_reads_back_with_the_same_patterns()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        ProfilePattern pattern = new() { Permanent = true, TileSize = new Vec2I(2, 1) };
        pattern.Grid = new PatternGrid(2, 1, [new ProfileTile(1, 2, TileFlip.Horizontal), null]);
        pattern.Variants.Add(new PatternVariant { Depth = 8, TagString = "road" });
        profile.PermanentPatterns.Add(pattern);

        ProfileExport.Normalize(profile, "Test");
        ProfileLoadResult result = ProfileFormat.Load(ProfileFormat.Save(profile));

        Assert.True(result.IsSuccess, result.Message);
        ProfilePattern restored = Assert.Single(result.Profile!.PermanentPatterns);
        Assert.Equal(new Vec2I(2, 1), restored.TileSize);
        Assert.Equal(2, restored.Grid!.Cells.Count);
        Assert.Equal(new ProfileTile(1, 2, TileFlip.Horizontal), restored.Grid.Cells[0]);
        Assert.Null(restored.Grid.Cells[1]);
        Assert.Equal(8, restored.Variants[0].Depth);
        Assert.Equal(["road"], restored.Variants[0].Tags);
    }
}
