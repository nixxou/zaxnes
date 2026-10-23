using Zax.Settings;

namespace Zax.Settings.Tests;

/// <summary>
/// The setting <c>ui.language</c> of AMD-035 §1: user scope, persisted, <c>en</c> by default,
/// <c>en</c> or <c>fr</c> as values, and anything else read as <c>en</c> without an error.
/// </summary>
public sealed class LanguageSettingTests
{
    [Fact] // AMD-035 §1
    public void The_setting_is_a_persisted_user_preference_defaulting_to_english()
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(SettingKeys.UiLanguage);

        Assert.Equal(SettingKind.String, descriptor.Kind);
        Assert.Equal(SettingScope.User, descriptor.Scope);
        Assert.Equal(SettingEffect.Immediate, descriptor.Effect);
        Assert.Equal(SettingTheme.Interface, descriptor.Theme);
        Assert.True(descriptor.IsPersisted);
        Assert.Equal("en", descriptor.DefaultValue.AsString);
        Assert.Equal(["en", "fr"], descriptor.TextOptions);
    }

    [Fact] // AMD-035 §4: a fresh user folder is English
    public void A_model_with_no_file_reads_english()
    {
        SettingsModel model = new();
        Assert.Equal("en", model.GetString(SettingKeys.UiLanguage));
    }

    [Theory] // AMD-035 §1: an unknown value falls back to en, without an error
    [InlineData("fr", "fr")]
    [InlineData("FR", "fr")]
    [InlineData("  fr  ", "fr")]
    [InlineData("en", "en")]
    [InlineData("", "en")]
    [InlineData("de", "en")]
    [InlineData("français", "en")]
    public void An_unknown_value_reads_as_english(string stored, string expected)
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(SettingKeys.UiLanguage);
        Assert.Equal(expected, descriptor.CoerceText(stored));
    }

    [Fact] // AMD-035 §1: the choice survives a restart
    public void The_choice_is_written_to_the_settings_file_and_read_back()
    {
        MemorySettingsStorage storage = new();
        SettingsModel written = new();
        written.SetString(SettingKeys.UiLanguage, "fr");

        SettingsFile file = new(storage);
        Assert.True(file.Save(written));

        SettingsModel read = new();
        SettingsFile again = new(storage);
        Assert.Equal(SettingsLoadStatus.Loaded, again.Load(read, DateTimeOffset.UnixEpoch).Status);
        Assert.Equal("fr", read.GetString(SettingKeys.UiLanguage));
    }
}
