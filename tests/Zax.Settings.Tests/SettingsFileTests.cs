using System.Text.Json;
using System.Text.Json.Nodes;
using Zax.Settings;

namespace Zax.Settings.Tests;

/// <summary>
/// The settings file (REQ-PERS-020 to REQ-PERS-024, REQ-PERS-015, REQ-PERS-073).
/// </summary>
public sealed class SettingsFileTests
{
    private static readonly DateTimeOffset Now = new(2026, 10, 21, 14, 30, 5, TimeSpan.Zero);

    /// <summary>REQ-PERS-020: the user preferences survive a round trip.</summary>
    [Fact]
    public void User_preferences_are_written_and_read_back()
    {
        MemorySettingsStorage storage = new();
        SettingsModel written = new();
        written.SetFloat(SettingKeys.CameraFov, 85f);
        written.SetFloat(SettingKeys.AudioVolume, 0.25f);
        written.SetInt(SettingKeys.SceneSkybox, 2);
        written.SetBool(SettingKeys.UiFullscreen, true);
        written.SetString(SettingKeys.UiLastBrowseDir, @"D:\roms");
        written.SetColor(SettingKeys.SceneCustomBgColor, new SettingColor(1f, 0f, 0f, 1f));

        Assert.True(new SettingsFile(storage).Save(written));

        SettingsModel read = new();
        SettingsLoadResult result = new SettingsFile(storage).Load(read, Now);

        Assert.Equal(SettingsLoadStatus.Loaded, result.Status);
        Assert.Equal(85f, read.GetFloat(SettingKeys.CameraFov));
        Assert.Equal(0.25f, read.GetFloat(SettingKeys.AudioVolume), 4);
        Assert.Equal(2, read.GetInt(SettingKeys.SceneSkybox));
        Assert.True(read.GetBool(SettingKeys.UiFullscreen));
        Assert.Equal(@"D:\roms", read.GetString(SettingKeys.UiLastBrowseDir));
        Assert.Equal(new SettingColor(1f, 0f, 0f, 1f), read.GetColor(SettingKeys.SceneCustomBgColor));
    }

    /// <summary>REQ-PERS-002: an absent file leaves every default in place.</summary>
    [Fact]
    public void An_absent_file_leaves_the_defaults()
    {
        SettingsModel model = new();
        SettingsLoadResult result = new SettingsFile(new MemorySettingsStorage()).Load(model, Now);

        Assert.Equal(SettingsLoadStatus.Absent, result.Status);
        Assert.Null(result.Message);
        Assert.Equal(70f, model.GetFloat(SettingKeys.CameraFov));
    }

    /// <summary>
    /// REQ-PERS-020: the P and S settings are not needed at start-up, and their presence in the
    /// file is ignored.
    /// </summary>
    [Fact]
    public void Profile_and_session_settings_in_the_file_are_ignored()
    {
        MemorySettingsStorage storage = new();
        storage.Put(
            SettingsFile.DefaultName,
            """
            {
              "schemaVersion": 1,
              "values": { "conv.empty_tile": true, "camera.smooth_time": 9, "camera.fov": 55 }
            }
            """);

        SettingsModel model = new();
        Assert.Equal(SettingsLoadStatus.Loaded, new SettingsFile(storage).Load(model, Now).Status);

        Assert.False(model.GetBool(SettingKeys.ConvEmptyTile));
        Assert.Equal(0.4f, model.GetFloat(SettingKeys.CameraSmoothTime), 4);
        Assert.Equal(55f, model.GetFloat(SettingKeys.CameraFov));
    }

    /// <summary>
    /// REQ-PERS-022: an unreadable file takes the model back to the defaults, is set aside and
    /// asks for the notification, word for word.
    /// </summary>
    [Fact]
    public void An_unreadable_file_is_set_aside_and_notified()
    {
        MemorySettingsStorage storage = new();
        storage.Put(SettingsFile.DefaultName, "{{{");

        SettingsModel model = new();
        SettingsLoadResult result = new SettingsFile(storage).Load(model, Now);

        Assert.Equal(SettingsLoadStatus.Unreadable, result.Status);
        Assert.Equal("Settings file was unreadable and has been reset.", result.Message);
        Assert.False(storage.Exists(SettingsFile.DefaultName));
        Assert.Contains("settings.json.corrupt-20261021-143005", storage.Names);
        Assert.Equal(70f, model.GetFloat(SettingKeys.CameraFov));
    }

    /// <summary>REQ-PERS-022: a root that is not an object counts as unreadable.</summary>
    [Fact]
    public void A_file_whose_root_is_not_an_object_counts_as_unreadable()
    {
        MemorySettingsStorage storage = new();
        storage.Put(SettingsFile.DefaultName, "[1, 2, 3]");

        SettingsLoadResult result = new SettingsFile(storage).Load(new SettingsModel(), Now);
        Assert.Equal(SettingsLoadStatus.Unreadable, result.Status);
    }

    /// <summary>
    /// REQ-PERS-022: a single value of the wrong type is replaced on its own, silently, and the
    /// other values are kept.
    /// </summary>
    [Fact]
    public void One_invalid_value_is_replaced_silently()
    {
        MemorySettingsStorage storage = new();
        storage.Put(
            SettingsFile.DefaultName,
            """
            {
              "schemaVersion": 1,
              "values": { "camera.fov": "abc", "audio.volume": 0.5 }
            }
            """);

        SettingsModel model = new();
        SettingsLoadResult result = new SettingsFile(storage).Load(model, Now);

        Assert.Equal(SettingsLoadStatus.Loaded, result.Status);
        Assert.Null(result.Message);
        Assert.Equal(70f, model.GetFloat(SettingKeys.CameraFov));
        Assert.Equal(0.5f, model.GetFloat(SettingKeys.AudioVolume), 4);
        Assert.True(storage.Exists(SettingsFile.DefaultName));
    }

    /// <summary>
    /// REQ-PERS-003: a value in the file that is outside its domain is brought back in, exactly
    /// as a change from the UI would be.
    /// </summary>
    [Fact]
    public void A_value_outside_its_domain_in_the_file_is_brought_back_in()
    {
        MemorySettingsStorage storage = new();
        storage.Put(
            SettingsFile.DefaultName,
            """
            { "schemaVersion": 1, "values": { "video.frame_rate": 75, "emu.game_speed": 12 } }
            """);

        SettingsModel model = new();
        new SettingsFile(storage).Load(model, Now);

        Assert.Equal(90, model.GetInt(SettingKeys.VideoFrameRate));
        Assert.Equal(8f, model.GetFloat(SettingKeys.EmuGameSpeed));
    }

    /// <summary>REQ-PERS-023: an unknown key survives a rewrite.</summary>
    [Fact]
    public void An_unknown_key_survives_a_rewrite()
    {
        MemorySettingsStorage storage = new();
        storage.Put(
            SettingsFile.DefaultName,
            """
            { "schemaVersion": 1, "values": { "future.x": 3, "audio.volume": 0.5 } }
            """);

        SettingsModel model = new();
        SettingsFile file = new(storage);
        file.Load(model, Now);
        model.SetFloat(SettingKeys.AudioVolume, 0.8f);
        Assert.True(file.Save(model));

        JsonObject root = (JsonObject)JsonNode.Parse(storage.Text(SettingsFile.DefaultName)!)!;
        JsonObject values = (JsonObject)root["values"]!;
        Assert.Equal(3, values["future.x"]!.GetValue<int>());
        Assert.Equal(0.8, values["audio.volume"]!.GetValue<double>(), 4);
    }

    /// <summary>
    /// REQ-PERS-024: a file from an earlier version is migrated, a <c>.vN.bak</c> copy is kept
    /// and the file is written again at the current version.
    /// </summary>
    [Fact]
    public void An_older_file_is_migrated_after_a_backup_copy()
    {
        MemorySettingsStorage storage = new();
        storage.Put(
            SettingsFile.DefaultName,
            """
            { "schemaVersion": 0, "values": { "legacy.fov": 85 } }
            """);

        SettingsModel model = new();
        SettingsFile file = new(storage, migrations: [new RenameFovMigration()]);
        SettingsLoadResult result = file.Load(model, Now);

        Assert.Equal(SettingsLoadStatus.Migrated, result.Status);
        Assert.Equal(0, result.Version);
        Assert.Equal("{ \"schemaVersion\": 0, \"values\": { \"legacy.fov\": 85 } }".Replace("\r\n", "\n"),
            storage.Text("settings.json.v0.bak")!.Replace("\r\n", "\n").Trim());
        Assert.Equal(85f, model.GetFloat(SettingKeys.CameraFov));

        Assert.True(file.Save(model));
        JsonObject root = (JsonObject)JsonNode.Parse(storage.Text(SettingsFile.DefaultName)!)!;
        Assert.Equal(SettingsFile.CurrentSchemaVersion, root["schemaVersion"]!.GetValue<int>());
        Assert.Equal(85, ((JsonObject)root["values"]!)["camera.fov"]!.GetValue<double>());
    }

    /// <summary>
    /// REQ-PERS-024: a file from a newer version is read at best and left untouched for the rest
    /// of the session, with its own notification.
    /// </summary>
    [Fact]
    public void A_newer_file_is_read_at_best_and_never_rewritten()
    {
        MemorySettingsStorage storage = new();
        storage.Put(
            SettingsFile.DefaultName,
            """
            { "schemaVersion": 99, "values": { "camera.fov": 45, "unheard.of": true } }
            """);

        SettingsModel model = new();
        SettingsFile file = new(storage);
        SettingsLoadResult result = file.Load(model, Now);

        Assert.Equal(SettingsLoadStatus.NewerVersion, result.Status);
        Assert.Equal("Settings were created by a newer version.", result.Message);
        Assert.True(result.IsReadOnlySession);
        Assert.Equal(45f, model.GetFloat(SettingKeys.CameraFov));

        model.SetFloat(SettingKeys.CameraFov, 70f);
        Assert.False(file.Save(model));
        Assert.Contains("\"schemaVersion\": 99", storage.Text(SettingsFile.DefaultName));
    }

    /// <summary>REQ-PERS-073: a store that refuses writes does not stop anything.</summary>
    [Fact]
    public void A_read_only_store_refuses_the_write_without_throwing()
    {
        MemorySettingsStorage storage = new(isWritable: false);
        SettingsModel model = new();
        model.SetFloat(SettingKeys.CameraFov, 50f);

        Assert.False(new SettingsFile(storage).Save(model));
        Assert.Empty(storage.Names);
    }

    /// <summary>
    /// REQ-PERS-015: <c>Setting:Load()</c> re-reads the file and applies only its U settings,
    /// so a session value a script put on top is dropped in favour of the file's preference.
    /// </summary>
    [Fact]
    public void Reloading_applies_only_the_user_settings_of_the_file()
    {
        MemorySettingsStorage storage = new();
        storage.Put(
            SettingsFile.DefaultName,
            """
            { "schemaVersion": 1, "values": { "audio.volume": 0.3 } }
            """);

        SettingsModel model = new();
        SettingsFile file = new(storage);
        file.Load(model, Now);
        Assert.Equal(0.3f, model.GetFloat(SettingKeys.AudioVolume), 4);

        model.SetFloat(SettingKeys.AudioVolume, 0.9f);
        file.Reload(model, Now);
        Assert.Equal(0.3f, model.GetFloat(SettingKeys.AudioVolume), 4);
    }

    /// <summary>
    /// §4.5.1: enumerations are written as the label of their domain and read back from either
    /// form.
    /// </summary>
    [Fact]
    public void Enumerations_are_written_as_labels_and_read_from_either_form()
    {
        MemorySettingsStorage storage = new();
        SettingsModel written = new();
        written.SetInt(SettingKeys.SceneSkybox, 1);
        new SettingsFile(storage).Save(written);

        JsonObject values = (JsonObject)((JsonObject)JsonNode.Parse(
            storage.Text(SettingsFile.DefaultName)!)!)["values"]!;
        Assert.Equal("Gradient", values["scene.skybox"]!.GetValue<string>());

        storage.Put(
            SettingsFile.DefaultName,
            """
            { "schemaVersion": 1, "values": { "scene.skybox": 2 } }
            """);
        SettingsModel read = new();
        new SettingsFile(storage).Load(read, Now);
        Assert.Equal(2, read.GetInt(SettingKeys.SceneSkybox));
    }

    /// <summary>
    /// AMD-031 §3: a value stored for the TV box key is read without error, has no effect, and
    /// is not written back — unlike the unknown keys of REQ-PERS-023, which survive a rewrite.
    /// </summary>
    [Theory] // AMD-031 §3
    [InlineData("1")]
    [InlineData("\"TV\"")]
    [InlineData("\"TV 3\"")]
    [InlineData("3")]
    public void AMD_031_a_stored_TV_box_value_is_ignored_and_dropped(string stored)
    {
        MemorySettingsStorage storage = new();
        storage.Put(
            SettingsFile.DefaultName,
            $$"""
            {
              "schemaVersion": 1,
              "values": { "scene.skybox": 1, "scene.tv_box": {{stored}}, "vr.unknown": 7 }
            }
            """);

        SettingsModel model = new();
        SettingsFile file = new(storage);
        SettingsLoadResult result = file.Load(model, Now);

        // Read without error, and the rest of the file is applied as usual.
        Assert.Equal(SettingsLoadStatus.Loaded, result.Status);
        Assert.Equal(1, model.GetInt(SettingKeys.SceneSkybox));
        Assert.Null(SettingsCatalogue.Find("scene.tv_box"));

        Assert.True(file.Save(model));
        JsonObject values = (JsonObject)((JsonObject)JsonNode.Parse(
            storage.Text(SettingsFile.DefaultName)!)!)["values"]!;

        // The retired key is gone; a genuinely unknown key is still kept (REQ-PERS-023).
        Assert.False(values.ContainsKey("scene.tv_box"));
        Assert.Equal(7, values["vr.unknown"]!.GetValue<int>());
    }

    /// <summary>§4.5.1: a vector is an array of numbers and a colour the <c>#RRGGBBAA</c> form.</summary>
    [Fact]
    public void Vectors_and_rectangles_are_arrays_of_numbers()
    {
        MemorySettingsStorage storage = new();
        SettingsModel written = new();
        written.SetRect(SettingKeys.UiWindowRect, new SettingRect(10, 20, 1920, 1080));
        new SettingsFile(storage).Save(written);

        JsonObject values = (JsonObject)((JsonObject)JsonNode.Parse(
            storage.Text(SettingsFile.DefaultName)!)!)["values"]!;
        Assert.Equal(JsonValueKind.Array, values["ui.window_rect"]!.GetValueKind());

        SettingsModel read = new();
        new SettingsFile(storage).Load(read, Now);
        Assert.Equal(new SettingRect(10, 20, 1920, 1080), read.GetRect(SettingKeys.UiWindowRect));
    }

    /// <summary>A preference back at its default is dropped from the file.</summary>
    [Fact]
    public void A_preference_back_at_its_default_leaves_the_file()
    {
        MemorySettingsStorage storage = new();
        SettingsModel model = new();
        SettingsFile file = new(storage);

        model.SetFloat(SettingKeys.CameraFov, 50f);
        file.Save(model);
        Assert.Contains("camera.fov", storage.Text(SettingsFile.DefaultName));

        model.Reset(SettingKeys.CameraFov);
        file.Save(model);
        Assert.DoesNotContain("camera.fov", storage.Text(SettingsFile.DefaultName));
    }

    /// <summary>A migration step used only by the test above.</summary>
    private sealed class RenameFovMigration : ISettingsMigration
    {
        public int FromVersion => 0;

        public void Apply(JsonObject values)
        {
            if (values.Remove("legacy.fov", out JsonNode? node))
            {
                values[SettingKeys.CameraFov] = node;
            }
        }
    }
}
