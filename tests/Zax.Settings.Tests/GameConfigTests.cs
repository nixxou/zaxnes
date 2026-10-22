using Zax.Settings;

namespace Zax.Settings.Tests;

/// <summary>
/// The per-ROM choices beside the library: the remembered profile and the ten recent ROMs
/// (AMD-010 REQ-UI-200, REQ-UI-201; REQ-PERS-021, REQ-PERS-022, REQ-PERS-024).
/// </summary>
public sealed class GameConfigTests
{
    private static readonly DateTimeOffset Now = new(2026, 10, 21, 9, 0, 0, TimeSpan.Zero);

    /// <summary>AMD-010 REQ-UI-200: the File menu lists the ten last ROMs, most recent first.</summary>
    [Fact]
    public void The_recent_list_keeps_the_ten_last_roms_most_recent_first()
    {
        GameConfigFile file = new(new MemorySettingsStorage());

        for (int i = 1; i <= 12; i++)
        {
            file.NoteRomOpened($@"D:\roms\game{i}.nes");
        }

        Assert.Equal(GameConfigFile.RecentRomLimit, file.RecentRoms.Count);
        Assert.Equal(@"D:\roms\game12.nes", file.RecentRoms[0]);
        Assert.Equal(@"D:\roms\game3.nes", file.RecentRoms[^1]);
        Assert.DoesNotContain(@"D:\roms\game1.nes", file.RecentRoms);
    }

    /// <summary>Opening a ROM again moves it to the head instead of adding a second entry.</summary>
    [Fact]
    public void Opening_a_rom_again_moves_it_to_the_head()
    {
        GameConfigFile file = new(new MemorySettingsStorage());

        file.NoteRomOpened(@"D:\roms\a.nes");
        file.NoteRomOpened(@"D:\roms\b.nes");
        file.NoteRomOpened(@"D:\ROMS\A.NES");

        Assert.Equal([@"D:\ROMS\A.NES", @"D:\roms\b.nes"], file.RecentRoms);
    }

    /// <summary>
    /// AMD-010 REQ-UI-201: the second opening of the same file starts straight away with the
    /// remembered profile.
    /// </summary>
    [Fact]
    public void The_profile_chosen_for_a_rom_file_is_remembered()
    {
        MemorySettingsStorage storage = new();
        GameConfigFile first = new(storage);

        Assert.Null(first.ProfileFor(@"D:\roms\digdug.nes"));
        first.SetProfileFor(@"D:\roms\digdug.nes", "10");
        first.NoteRomOpened(@"D:\roms\digdug.nes");
        Assert.True(first.Save());

        GameConfigFile second = new(storage);
        Assert.Equal(SettingsLoadStatus.Loaded, second.Load(Now).Status);
        Assert.Equal("10", second.ProfileFor(@"D:\roms\digdug.nes"));
        Assert.Equal([@"D:\roms\digdug.nes"], second.RecentRoms);
    }

    /// <summary>A disappeared ROM can be dropped from both lists.</summary>
    [Fact]
    public void A_rom_can_be_forgotten()
    {
        GameConfigFile file = new(new MemorySettingsStorage());
        file.NoteRomOpened(@"D:\roms\gone.nes");
        file.SetProfileFor(@"D:\roms\gone.nes", "7");

        Assert.True(file.ForgetRom(@"D:\roms\gone.nes"));
        Assert.Empty(file.RecentRoms);
        Assert.Null(file.ProfileFor(@"D:\roms\gone.nes"));
        Assert.False(file.ForgetRom(@"D:\roms\gone.nes"));
    }

    /// <summary>REQ-PERS-022: an unreadable file is set aside and the lists start empty.</summary>
    [Fact]
    public void An_unreadable_file_is_set_aside()
    {
        MemorySettingsStorage storage = new();
        storage.Put(GameConfigFile.DefaultName, "not json at all");

        GameConfigFile file = new(storage);
        SettingsLoadResult result = file.Load(Now);

        Assert.Equal(SettingsLoadStatus.Unreadable, result.Status);
        Assert.Equal(GameConfigFile.UnreadableMessage, result.Message);
        Assert.Contains("games.json.corrupt-20261021-090000", storage.Names);
        Assert.Empty(file.RecentRoms);
    }

    /// <summary>REQ-PERS-024: a file from a newer version is read at best and left alone.</summary>
    [Fact]
    public void A_newer_file_is_read_at_best_and_never_rewritten()
    {
        MemorySettingsStorage storage = new();
        storage.Put(
            GameConfigFile.DefaultName,
            """
            { "schemaVersion": 99, "recentRoms": ["D:\\a.nes"], "romProfiles": { "D:\\a.nes": "3" } }
            """);

        GameConfigFile file = new(storage);
        SettingsLoadResult result = file.Load(Now);

        Assert.Equal(SettingsLoadStatus.NewerVersion, result.Status);
        Assert.Equal([@"D:\a.nes"], file.RecentRoms);
        Assert.Equal("3", file.ProfileFor(@"D:\a.nes"));

        file.NoteRomOpened(@"D:\b.nes");
        Assert.False(file.Save());
        Assert.Contains("\"schemaVersion\": 99", storage.Text(GameConfigFile.DefaultName));
    }

    /// <summary>REQ-PERS-024: an older file leaves a <c>.vN.bak</c> copy behind.</summary>
    [Fact]
    public void An_older_file_leaves_a_backup_copy()
    {
        MemorySettingsStorage storage = new();
        storage.Put(GameConfigFile.DefaultName, """{ "schemaVersion": 0, "recentRoms": [] }""");

        GameConfigFile file = new(storage);
        Assert.Equal(SettingsLoadStatus.Migrated, file.Load(Now).Status);
        Assert.Contains("games.json.v0.bak", storage.Names);
    }

    /// <summary>
    /// REQ-PERS-075: an absent file is not an error — the lists simply start empty, with no
    /// dialogue and no notification.
    /// </summary>
    [Fact]
    public void An_absent_file_starts_empty_without_a_notification()
    {
        GameConfigFile file = new(new MemorySettingsStorage());
        SettingsLoadResult result = file.Load(Now);

        Assert.Equal(SettingsLoadStatus.Absent, result.Status);
        Assert.Null(result.Message);
        Assert.Empty(file.RecentRoms);
    }

    /// <summary>
    /// REQ-PERS-031: no setting is remembered per game. The file beside the library carries the
    /// recent list and the ROM-to-profile choices and nothing else.
    /// </summary>
    [Fact]
    public void The_file_carries_no_settings()
    {
        MemorySettingsStorage storage = new();
        GameConfigFile file = new(storage);
        file.NoteRomOpened(@"D:\roms\a.nes");
        file.SetProfileFor(@"D:\roms\a.nes", "4");
        file.Save();

        string written = storage.Text(GameConfigFile.DefaultName)!;
        Assert.All(
            SettingsCatalogue.Keys,
            key => Assert.DoesNotContain(key, written, StringComparison.Ordinal));
    }
}
