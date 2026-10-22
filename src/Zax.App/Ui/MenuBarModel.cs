using System.Globalization;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Nes;
using Zax.Render;

namespace Zax.App.Ui;

/// <summary>What one row of the menu bar is.</summary>
internal enum MenuItemKind
{
    /// <summary>A plain entry that invokes its command.</summary>
    Command,

    /// <summary>An entry with a tick, drawn from the command's own toggle state.</summary>
    Toggle,

    /// <summary>An entry with a tick that depends on its argument — a radio choice.</summary>
    Choice,

    /// <summary>An entry that opens a submenu.</summary>
    Submenu,

    /// <summary>A horizontal rule.</summary>
    Separator,
}

/// <summary>
/// One row of the menu bar: a label, the command it invokes with its arguments, the shortcut text
/// shown on the right, and what kind of row it is (AMD-010 REQ-UI-200).
/// </summary>
internal sealed class MenuItem
{
    private MenuItem(MenuItemKind kind, string label)
    {
        Kind = kind;
        Label = label;
    }

    /// <summary>What kind of row this is.</summary>
    public MenuItemKind Kind { get; }

    /// <summary>The label shown on the left.</summary>
    public string Label { get; }

    /// <summary>The command of <see cref="ZaxCommands"/> this row invokes, or null.</summary>
    public string? Command { get; private init; }

    /// <summary>The arguments the row passes to its command.</summary>
    public CommandArgs Args { get; private init; }

    /// <summary>The default shortcut text shown on the right, or an empty string.</summary>
    public string Shortcut { get; private init; } = string.Empty;

    /// <summary>Whether a <see cref="MenuItemKind.Choice"/> row is the current one.</summary>
    public Func<bool>? IsSelected { get; private init; }

    /// <summary>The rows of a submenu, produced when the submenu is opened.</summary>
    public Func<IReadOnlyList<MenuItem>>? Children { get; private init; }

    /// <summary>A horizontal rule.</summary>
    public static MenuItem Rule { get; } = new(MenuItemKind.Separator, string.Empty);

    /// <summary>
    /// A plain entry. The label and the shortcut default to the ones the command carries, which
    /// is what makes REQ-UI-200 ("each entry shows its default shortcut") a property of the
    /// command model rather than of the drawing code.
    /// </summary>
    public static MenuItem Of(ZaxCommand command, string? label = null, CommandArgs args = default)
    {
        ArgumentNullException.ThrowIfNull(command);
        return new MenuItem(command.IsToggle ? MenuItemKind.Toggle : MenuItemKind.Command, label ?? command.Label)
        {
            Command = command.Name,
            Args = args,
            Shortcut = command.Shortcut,
        };
    }

    /// <summary>A radio entry: it invokes <paramref name="command"/> with <paramref name="args"/>.</summary>
    public static MenuItem Choice(ZaxCommand command, string label, CommandArgs args, Func<bool> isSelected)
    {
        ArgumentNullException.ThrowIfNull(command);
        ArgumentNullException.ThrowIfNull(isSelected);
        return new MenuItem(MenuItemKind.Choice, label)
        {
            Command = command.Name,
            Args = args,
            IsSelected = isSelected,
        };
    }

    /// <summary>A submenu.</summary>
    public static MenuItem Sub(string label, Func<IReadOnlyList<MenuItem>> children)
    {
        ArgumentNullException.ThrowIfNull(children);
        return new MenuItem(MenuItemKind.Submenu, label) { Children = children };
    }

    /// <summary>A disabled row that only says something, such as an empty submenu.</summary>
    public static MenuItem Note(string label) => new(MenuItemKind.Command, label);
}

/// <summary>One menu of the bar.</summary>
/// <param name="Title">Its title, as REQ-UI-200 writes it.</param>
/// <param name="Items">Its rows, in order.</param>
internal sealed record Menu(string Title, IReadOnlyList<MenuItem> Items);

/// <summary>
/// The menu bar of AMD-010 REQ-UI-200 as a UI-independent model: five menus whose every entry
/// names a command of the model of REQ-UI-205 and carries that command's default shortcut.
/// </summary>
/// <remarks>
/// The bar is built once per session. Everything that changes — the recent ROMs, the slot dates,
/// which speed or region is current — is read when the row is drawn: the static rows hold labels
/// and command names, the dynamic submenus hold a factory. This is also what the tests drive:
/// walking <see cref="Menus"/> and expanding every submenu covers the whole table of REQ-UI-200
/// without an ImGui frame.
/// </remarks>
internal sealed class MenuBarModel
{
    private readonly CommandModel _commands;

    private MenuBarModel(CommandModel commands, IReadOnlyList<Menu> menus)
    {
        _commands = commands;
        Menus = menus;
    }

    /// <summary>The five menus, in the order of REQ-UI-200.</summary>
    public IReadOnlyList<Menu> Menus { get; }

    /// <summary>Every row of every menu, submenus expanded, for the tests and the shortcut audit.</summary>
    public IEnumerable<MenuItem> AllItems()
    {
        foreach (Menu menu in Menus)
        {
            foreach (MenuItem item in Expand(menu.Items))
            {
                yield return item;
            }
        }

        static IEnumerable<MenuItem> Expand(IReadOnlyList<MenuItem> items)
        {
            foreach (MenuItem item in items)
            {
                yield return item;

                if (item.Children is { } children)
                {
                    foreach (MenuItem child in Expand(children()))
                    {
                        yield return child;
                    }
                }
            }
        }
    }

    /// <summary>Whether <paramref name="item"/> can be activated right now (REQ-UI-200).</summary>
    public bool IsEnabled(MenuItem item)
    {
        ArgumentNullException.ThrowIfNull(item);
        return item.Command is { } name
            && _commands.Find(name) is { } command
            && command.CanExecute(item.Args);
    }

    /// <summary>Whether <paramref name="item"/> shows a tick.</summary>
    public bool IsChecked(MenuItem item)
    {
        ArgumentNullException.ThrowIfNull(item);
        return item.Kind switch
        {
            MenuItemKind.Choice => item.IsSelected?.Invoke() ?? false,
            MenuItemKind.Toggle => item.Command is { } name && _commands.Find(name) is { IsChecked: true },
            _ => false,
        };
    }

    /// <summary>
    /// Builds the bar of REQ-UI-200 over <paramref name="host"/>'s command model.
    /// </summary>
    /// <param name="host">The emulation host, whose state the rows read.</param>
    /// <param name="recentRoms">The ten recent ROMs of <c>games.json</c>, most recent first.</param>
    /// <param name="camera">The scene camera, for the current camera mode.</param>
    public static MenuBarModel Build(
        EmulationHost host,
        Func<IReadOnlyList<string>> recentRoms,
        SceneCamera camera)
    {
        ArgumentNullException.ThrowIfNull(host);
        ArgumentNullException.ThrowIfNull(recentRoms);
        ArgumentNullException.ThrowIfNull(camera);

        CommandModel commands = host.Commands;
        AppSettings settings = host.Settings;

        ZaxCommand Need(string name) => commands.Find(name)
            ?? throw new InvalidOperationException($"the command '{name}' is not registered");

        Menu file = new(UiText.MenuFile,
        [
            MenuItem.Of(Need(ZaxCommands.OpenRom)),
            MenuItem.Sub(UiText.RecentRoms, () => RecentRomItems(Need(ZaxCommands.OpenRecentRom), recentRoms())),
            MenuItem.Rule,
            MenuItem.Of(Need(ZaxCommands.ChooseProfile)),
            MenuItem.Of(Need(ZaxCommands.ImportProfile)),

            // SPEC-10 REQ-UI-037 with AMD-010 §3: the deletion of a user profile is one of the
            // functions the amendment keeps mandatory, and the tile button it describes becomes a
            // menu entry here — the presentation is informative, the function is not.
            MenuItem.Of(Need(ZaxCommands.DeleteGame)),
            MenuItem.Rule,

            // REQ-PERS-036 is P3 after AMD-020 (Q-050); the two sources a menu can offer.
            MenuItem.Of(Need(ZaxCommands.CreateGameArt)),
            MenuItem.Of(Need(ZaxCommands.ImportCover)),
            MenuItem.Rule,
            MenuItem.Sub(UiText.LoadState, () => SlotItems(
                Need(ZaxCommands.LoadSlot),
                Need(ZaxCommands.QuickLoad),
                host.Slots)),
            MenuItem.Sub(UiText.SaveState, () => SlotItems(
                Need(ZaxCommands.SaveSlot),
                Need(ZaxCommands.QuickSave),
                host.Slots)),

            // REQ-UI-052: the slot deletion of SPEC-09 REQ-PERS-054 needs a route of its own,
            // since a classic menu has no selected tile to hang a small button on.
            MenuItem.Sub(UiText.DeleteState, () => DeleteSlotItems(
                Need(ZaxCommands.DeleteSlot),
                host.Slots)),
            MenuItem.Rule,
            MenuItem.Of(Need(ZaxCommands.Quit)),
        ]);

        Menu emulation = new(UiText.MenuEmulation,
        [
            MenuItem.Of(Need(ZaxCommands.PauseResume)),
            MenuItem.Of(Need(ZaxCommands.Reset)),
            MenuItem.Of(Need(ZaxCommands.Stop)),
            MenuItem.Rule,
            MenuItem.Of(Need(ZaxCommands.FastForward)),
            MenuItem.Of(Need(ZaxCommands.Rollback)),
            MenuItem.Rule,
            MenuItem.Sub(UiText.Speed, () => SpeedItems(
                Need(ZaxCommands.SetSpeed),
                Need(ZaxCommands.SpeedUp),
                Need(ZaxCommands.SpeedDown),
                settings)),
            MenuItem.Sub(UiText.Region, () => RegionItems(
                Need(ZaxCommands.SetRegion),
                Need(ZaxCommands.ToggleAutoRegion),
                settings)),
            MenuItem.Sub(UiText.Ports, () => PortItems(Need(ZaxCommands.SetPortType), host)),
        ]);

        Menu video = new(UiText.MenuVideo,
        [
            MenuItem.Of(Need(ZaxCommands.ToggleTwoDimensional)),
            MenuItem.Sub(UiText.CameraMode, () => CameraItems(
                Need(ZaxCommands.SetCameraMode),
                Need(ZaxCommands.ToggleLocalCamera),
                camera)),
            MenuItem.Of(Need(ZaxCommands.ResetView)),
            MenuItem.Rule,
            MenuItem.Sub(UiText.LabelOf(Zax.Settings.SettingsCatalogue.Get(Zax.Settings.SettingKeys.SceneSkybox)),
                () => SkyboxItems(Need(ZaxCommands.SetSkybox), settings)),
            MenuItem.Sub(UiText.LabelOf(Zax.Settings.SettingsCatalogue.Get(Zax.Settings.SettingKeys.SceneTvBox)),
                () => TvBoxItems(Need(ZaxCommands.SetTvBox), settings)),
            MenuItem.Of(Need(ZaxCommands.ToggleShadows)),
            MenuItem.Rule,
            MenuItem.Of(Need(ZaxCommands.ToggleFullscreen)),
            MenuItem.Of(Need(ZaxCommands.ToggleFpsCounter)),
        ]);

        Menu audio = new(UiText.MenuAudio,
        [
            MenuItem.Sub(UiText.Volume, () => VolumeItems(
                Need(ZaxCommands.SetVolume),
                Need(ZaxCommands.VolumeUp),
                Need(ZaxCommands.VolumeDown),
                settings)),
            MenuItem.Of(Need(ZaxCommands.ToggleMute)),

            // AMD-024 REQ-PUB-005 (revised 2026-10-21): there is no menu music any more, so the
            // « Musique de menu » entry is gone. The setting itself stays in the model for the
            // compatibility of settings.json, without effect.
        ]);

        Menu tools = new(UiText.MenuTools,
        [
            MenuItem.Of(Need(ZaxCommands.ShowOptions)),
            MenuItem.Of(Need(ZaxCommands.ShowKeyConfig)),
            MenuItem.Rule,
            MenuItem.Of(Need(ZaxCommands.ShowDataFolder)),
            MenuItem.Of(Need(ZaxCommands.ShowAbout)),
        ]);

        return new MenuBarModel(commands, [file, emulation, video, audio, tools]);
    }

    private static IReadOnlyList<MenuItem> RecentRomItems(ZaxCommand open, IReadOnlyList<string> roms)
    {
        if (roms.Count == 0)
        {
            return [MenuItem.Note(UiText.NothingHere)];
        }

        List<MenuItem> items = new(roms.Count);
        for (int index = 0; index < roms.Count; index++)
        {
            items.Add(MenuItem.Of(open, Path.GetFileName(roms[index]), CommandArgs.Of(index)));
        }

        return items;
    }

    /// <summary>
    /// The ten numbered slots plus the quick slot of REQ-UI-208, each showing the date and time
    /// of its state or « vide ».
    /// </summary>
    private static IReadOnlyList<MenuItem> SlotItems(
        ZaxCommand numbered,
        ZaxCommand quick,
        SaveSlotIndex slots)
    {
        List<MenuItem> items = new(SaveSlotIndex.ExposedSlotCount + 2);
        for (int slot = 1; slot <= SaveSlotIndex.ExposedSlotCount; slot++)
        {
            items.Add(MenuItem.Of(
                numbered,
                SlotLabel(slot, slots.Find(SaveSlotIndex.NumberedSlotId(slot))),
                CommandArgs.Of(slot)));
        }

        items.Add(MenuItem.Rule);
        items.Add(MenuItem.Of(
            quick,
            $"{UiText.QuickSlot} — {StateStamp(slots.Find(SaveSlotIndex.QuickSlotId))}"));
        return items;
    }

    /// <summary>
    /// The deletion rows of REQ-UI-052: only the occupied slots, the quick slot included, each
    /// invoking <c>state.delete_slot</c> by identifier so the confirmation of REQ-UI-110 names
    /// the right one.
    /// </summary>
    private static IReadOnlyList<MenuItem> DeleteSlotItems(ZaxCommand delete, SaveSlotIndex slots)
    {
        List<MenuItem> items = [];
        for (int slot = 1; slot <= SaveSlotIndex.ExposedSlotCount; slot++)
        {
            string id = SaveSlotIndex.NumberedSlotId(slot);
            if (slots.Find(id) is { IsOccupied: true } record)
            {
                items.Add(MenuItem.Of(delete, SlotLabel(slot, record), CommandArgs.Of(id)));
            }
        }

        if (slots.Find(SaveSlotIndex.QuickSlotId) is { IsOccupied: true } quick)
        {
            items.Add(MenuItem.Of(
                delete,
                $"{UiText.QuickSlot} — {StateStamp(quick)}",
                CommandArgs.Of(SaveSlotIndex.QuickSlotId)));
        }

        return items.Count == 0 ? [MenuItem.Note(UiText.NothingHere)] : items;
    }

    /// <summary>The label of a numbered slot row: « Emplacement 3 — 2026-10-21 14:03 ».</summary>
    public static string SlotLabel(int slot, SaveSlotRecord? record)
        => string.Create(
            CultureInfo.InvariantCulture,
            $"{UiText.SlotPrefix} {slot} — {StateStamp(record)}");

    private static string StateStamp(SaveSlotRecord? record)
        => record is { IsOccupied: true }
            ? SaveStateFile.FormatTimestamp(record.SavedAt)
            : UiText.EmptySlot;

    private static IReadOnlyList<MenuItem> SpeedItems(
        ZaxCommand set,
        ZaxCommand up,
        ZaxCommand down,
        AppSettings settings)
    {
        List<MenuItem> items =
        [
            MenuItem.Of(up),
            MenuItem.Of(down),
            MenuItem.Rule,
        ];

        for (int value = SpeedSetting.Maximum; value >= SpeedSetting.Minimum; value--)
        {
            int captured = value;
            items.Add(MenuItem.Choice(
                set,
                UiText.SpeedLabel(captured),
                CommandArgs.Of(captured),
                () => settings.GameSpeed == captured));
        }

        return items;
    }

    private static IReadOnlyList<MenuItem> RegionItems(
        ZaxCommand set,
        ZaxCommand auto,
        AppSettings settings)
        =>
        [
            MenuItem.Of(auto, "Auto"),
            MenuItem.Rule,
            MenuItem.Choice(set, "NTSC", CommandArgs.Of((int)TvRegionSetting.Ntsc),
                () => !settings.AutoTvMode && settings.TvMode == TvRegionSetting.Ntsc),
            MenuItem.Choice(set, "PAL", CommandArgs.Of((int)TvRegionSetting.Pal),
                () => !settings.AutoTvMode && settings.TvMode == TvRegionSetting.Pal),
            MenuItem.Choice(set, "Dendy", CommandArgs.Of((int)TvRegionSetting.Dendy),
                () => !settings.AutoTvMode && settings.TvMode == TvRegionSetting.Dendy),
        ];

    /// <summary>The two ports of REQ-INP-060, each offering Controller or Zapper.</summary>
    private static IReadOnlyList<MenuItem> PortItems(ZaxCommand set, EmulationHost host)
    {
        List<MenuItem> items = [];
        for (int port = 0; port < 2; port++)
        {
            int captured = port;
            items.Add(MenuItem.Sub(
                string.Create(CultureInfo.InvariantCulture, $"Port {captured + 1}"),
                () =>
                [
                    MenuItem.Choice(
                        set,
                        "Manette",
                        new CommandArgs(captured, null, (int)ControllerPortType.Controller),
                        () => host.PortType(captured) == ControllerPortType.Controller),
                    MenuItem.Choice(
                        set,
                        "Zapper",
                        new CommandArgs(captured, null, (int)ControllerPortType.Zapper),
                        () => host.PortType(captured) == ControllerPortType.Zapper),
                ]));
        }

        return items;
    }

    /// <summary>
    /// « Mode caméra ▸ » (REQ-UI-200). The amendment names the submenu without listing it;
    /// SPEC-06 offers the two modes of REQ-SCN-006 and the local camera of REQ-SCN-009, and the
    /// submenu carries both. See <c>spec/changes/Q-038.md</c>.
    /// </summary>
    // Q-038
    private static IReadOnlyList<MenuItem> CameraItems(
        ZaxCommand set,
        ZaxCommand local,
        SceneCamera camera)
        =>
        [
            MenuItem.Choice(set, "Orbitale (TPV)", CommandArgs.Of(0),
                () => camera.Mode == Zax.Render.CameraMode.Tpv),
            MenuItem.Choice(set, "Première personne (FPV)", CommandArgs.Of(1),
                () => camera.Mode == Zax.Render.CameraMode.Fpv),
            MenuItem.Rule,
            MenuItem.Of(local),
        ];

    private static IReadOnlyList<MenuItem> SkyboxItems(ZaxCommand set, AppSettings settings)
        => ChoiceItems(set, Zax.Settings.SettingKeys.SceneSkybox, () => settings.Skybox);

    private static IReadOnlyList<MenuItem> TvBoxItems(ZaxCommand set, AppSettings settings)
        => ChoiceItems(set, Zax.Settings.SettingKeys.SceneTvBox, () => settings.TvBox);

    /// <summary>
    /// The rows of an enumerated setting, taken from its domain in the catalogue rather than
    /// written out again, so a domain change cannot leave the menu behind.
    /// </summary>
    private static IReadOnlyList<MenuItem> ChoiceItems(ZaxCommand set, string key, Func<int> current)
    {
        Zax.Settings.SettingDescriptor descriptor = Zax.Settings.SettingsCatalogue.Get(key);
        IReadOnlyList<Zax.Settings.SettingOption> options = descriptor.Domain.Options ?? [];

        List<MenuItem> items = new(options.Count);
        foreach (Zax.Settings.SettingOption option in options)
        {
            int value = (int)option.Value;
            items.Add(MenuItem.Choice(
                set,
                UiText.LabelOf(descriptor, option.Label),
                CommandArgs.Of(value),
                () => current() == value));
        }

        return items;
    }

    private static IReadOnlyList<MenuItem> VolumeItems(
        ZaxCommand set,
        ZaxCommand up,
        ZaxCommand down,
        AppSettings settings)
    {
        List<MenuItem> items =
        [
            MenuItem.Of(up),
            MenuItem.Of(down),
            MenuItem.Rule,
        ];

        for (int percent = 100; percent >= 0; percent -= 10)
        {
            int captured = percent;
            items.Add(MenuItem.Choice(
                set,
                string.Create(CultureInfo.InvariantCulture, $"{captured} %"),
                CommandArgs.Of(captured),
                () => (int)MathF.Round(settings.Volume * 100f) == captured));
        }

        return items;
    }
}
