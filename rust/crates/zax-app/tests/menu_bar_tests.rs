//! The menu bar and the fixed shortcuts (`MenuBarTests.cs`).

#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;
mod ui_harness;

use std::collections::{HashMap, HashSet};

use ui_harness::{UiHarness, UiLanguageScope};
use zax_app::application::AppHost;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::input::default_bindings::PLAYER_ONE_KEYBOARD;
use zax_app::persistence::save_slots::EXPOSED_SLOT_COUNT;
use zax_app::persistence::save_state_file;
use zax_app::ui::commands::UiCommand;
use zax_app::ui::menu_bar::{MenuBarModel, MenuItem, MenuItemKind};
use zax_app::ui::menu_shortcuts::{self, ShortcutCommand, ShortcutModifiers};
use zax_app::ui::strings::UiLanguage;
use zax_app::ui::text;
use zax_platform::input::Key;

/// Every entry of `items`, the children of each submenu after it.
fn expand(items: &[MenuItem], host: &AppHost) -> Vec<MenuItem> {
    let mut all = Vec::new();
    for item in items {
        all.push(item.clone());
        if let Some(submenu) = item.submenu {
            all.extend(expand(&MenuBarModel::children(submenu, host), host));
        }
    }
    all
}

fn commands_of(harness: &mut UiHarness, title: &str) -> Vec<Command> {
    let menu = harness
        .ui
        .menu_bar
        .menus()
        .iter()
        .find(|menu| menu.title == title)
        .expect("the menu exists")
        .clone();
    expand(&menu.items, &harness.host)
        .iter()
        .filter_map(|item| item.command)
        .collect()
}

fn submenu_of(harness: &mut UiHarness, title: &str, submenu: &str) -> Vec<MenuItem> {
    let menu = harness
        .ui
        .menu_bar
        .menus()
        .iter()
        .find(|menu| menu.title == title)
        .expect("the menu exists")
        .clone();
    let item = menu
        .items
        .iter()
        .find(|item| item.kind == MenuItemKind::Submenu && item.label == submenu)
        .expect("the submenu exists");
    MenuBarModel::children(item.submenu.expect("a submenu"), &harness.host)
}

fn find(harness: &mut UiHarness, command: Command) -> MenuItem {
    let host = &harness.host;
    harness
        .ui
        .menu_bar
        .all_items(host)
        .into_iter()
        .find(|item| item.command == Some(command))
        .expect("the entry exists")
}

fn single(key: Key, modifiers: ShortcutModifiers) -> (ShortcutCommand, CommandArgs) {
    let mut hits = menu_shortcuts::resolve(&[key], modifiers);
    assert_eq!(1, hits.len());
    hits.remove(0)
}

#[test]
fn req_ui_200_the_five_menus_exist_with_their_titles() {
    let _language = UiLanguageScope::keep();
    let mut harness = UiHarness::new(&["Arkanoid"]);

    let titles: Vec<String> = harness
        .ui
        .menu_bar
        .menus()
        .iter()
        .map(|m| m.title.clone())
        .collect();
    assert_eq!(vec!["File", "Emulation", "Video", "Audio", "Tools"], titles);

    harness.host.update_settings(|model| {
        use zax_app::persistence::app_settings::AppSettingsMut as _;
        model.set_language("fr");
    });
    harness.pump();

    let titles: Vec<String> = harness
        .ui
        .menu_bar
        .menus()
        .iter()
        .map(|m| m.title.clone())
        .collect();
    assert_eq!(
        vec!["Fichier", "Émulation", "Vidéo", "Audio", "Outils"],
        titles
    );
}

#[test]
fn req_ui_200_every_entry_of_the_table_exists_and_names_a_command() {
    let _language = UiLanguageScope::new(UiLanguage::English);
    let mut harness = UiHarness::new(&["Arkanoid"]);

    // One recent ROM, so the submenu of REQ-UI-200 has a row rather than its "vide" note.
    harness
        .host
        .settings_store_mut()
        .expect("a settings store")
        .games_mut()
        .note_rom_opened("C:\\roms\\game.nes");

    // One expectation per row of the table of REQ-UI-200: the menu it belongs to and the
    // command it must invoke.
    let expected = [
        (text::menu_file(), Command::OpenRom),
        (text::menu_file(), Command::OpenRecentRom),
        (text::menu_file(), Command::ChooseProfile),
        (text::menu_file(), Command::ImportProfile),
        (text::menu_file(), Command::ImportThreeDSen), // AMD-030 §2
        (text::menu_file(), Command::LoadSlot),
        (text::menu_file(), Command::QuickLoad),
        (text::menu_file(), Command::SaveSlot),
        (text::menu_file(), Command::QuickSave),
        (text::menu_file(), Command::Quit),
        (text::menu_emulation(), Command::PauseResume),
        (text::menu_emulation(), Command::Reset),
        (text::menu_emulation(), Command::Stop),
        (text::menu_emulation(), Command::FastForward),
        (text::menu_emulation(), Command::Rollback),
        (text::menu_emulation(), Command::SetSpeed),
        (text::menu_emulation(), Command::SetRegion),
        (text::menu_emulation(), Command::ToggleAutoRegion),
        (text::menu_emulation(), Command::SetPortType),
        (text::menu_video(), Command::ToggleTwoDimensional),
        (text::menu_video(), Command::SetCameraMode),
        (text::menu_video(), Command::ResetView),
        (text::menu_video(), Command::SetSkybox),
        (text::menu_video(), Command::ToggleShadows),
        (text::menu_video(), Command::ToggleFullscreen),
        (text::menu_video(), Command::ToggleFpsCounter),
        (text::menu_audio(), Command::SetVolume),
        (text::menu_audio(), Command::ToggleMute),
        // AMD-024 REQ-PUB-005 (revised 2026-10-21): "Musique de menu" is gone from the Audio
        // menu with the menu music itself.
        (text::menu_tools(), Command::ShowOptions),
        (text::menu_tools(), Command::ShowKeyConfig),
        (text::menu_tools(), Command::ShowDataFolder),
        (text::menu_tools(), Command::ShowAbout),
    ];

    for (menu, command) in expected {
        let commands = commands_of(&mut harness, &menu);
        assert!(
            commands.contains(&command),
            "{} is missing from {menu}",
            command.name()
        );
    }
}

#[test]
fn req_ui_200_each_entry_shows_its_default_shortcut() {
    let _language = UiLanguageScope::new(UiLanguage::English);
    let mut harness = UiHarness::new(&["Arkanoid"]);

    let mut shown: HashMap<Command, String> = HashMap::new();
    let host = &harness.host;
    for item in harness.ui.menu_bar.all_items(host) {
        if let Some(command) = item.command {
            if !item.shortcut.is_empty() {
                shown.insert(command, item.shortcut);
            }
        }
    }

    assert_eq!("Ctrl+O", shown[&Command::OpenRom]);
    assert_eq!("P", shown[&Command::PauseResume]);
    assert_eq!("Ctrl+R", shown[&Command::Reset]);
    assert_eq!("F11", shown[&Command::ToggleTwoDimensional]);
    assert_eq!("Alt+Enter", shown[&Command::ToggleFullscreen]);
    assert_eq!("Ctrl+,", shown[&Command::ShowOptions]);
    assert_eq!("Ctrl+Q", shown[&Command::Quit]);
    assert_eq!("Ctrl+F1…F10", shown[&Command::LoadSlot]);
    assert_eq!("Shift+F1…F10", shown[&Command::SaveSlot]);
    assert_eq!("FastForward shortcut", shown[&Command::FastForward]);
    assert_eq!("RollBack shortcut", shown[&Command::Rollback]);
    assert_eq!("long press on Menu", shown[&Command::ResetView]);
}

#[test]
fn req_ui_200_entries_follow_the_availability_of_their_command() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    let reset = find(&mut harness, Command::Reset);
    let shadows = find(&mut harness, Command::ToggleShadows);

    assert!(!MenuBarModel::is_enabled(&reset, &harness.host));
    assert!(MenuBarModel::is_checked(&shadows, &harness.host));

    harness.run(Command::ToggleShadows);
    assert!(!MenuBarModel::is_checked(&shadows, &harness.host));

    harness.launch("g0");
    assert!(MenuBarModel::is_enabled(&reset, &harness.host));
}

#[test]
fn req_ui_208_the_slot_submenus_show_a_date_or_vide() {
    let _language = UiLanguageScope::new(UiLanguage::English);
    let mut harness = UiHarness::new(&["Arkanoid"]);

    let slots = submenu_of(&mut harness, &text::menu_file(), &text::save_state());

    // Ten numbered rows, a rule, and the quick slot (REQ-UI-208).
    assert_eq!(EXPOSED_SLOT_COUNT + 2, slots.len());
    for item in slots
        .iter()
        .filter(|item| item.kind != MenuItemKind::Separator)
    {
        assert!(item.label.contains(&text::empty_slot()), "{}", item.label);
    }

    harness.launch("g0");
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(3));

    let slots = submenu_of(&mut harness, &text::menu_file(), &text::load_state());
    assert!(slots[2].label.contains(&save_state_file::format_timestamp(
        common::START_UNIX_SECONDS
    )));
    assert!(slots[2]
        .label
        .starts_with(&format!("{} 3 ", text::slot_prefix())));

    // The slots beyond ten are never exposed (REQ-UI-208, accepted divergence D).
    assert_eq!(
        10,
        slots
            .iter()
            .filter(|item| item.label.starts_with(&text::slot_prefix()))
            .count()
    );
}

#[test]
fn req_ui_200_the_recent_rom_submenu_lists_the_ten_last_files() {
    let _language = UiLanguageScope::new(UiLanguage::English);
    let mut harness = UiHarness::new(&["Arkanoid"]);

    let recent = submenu_of(&mut harness, &text::menu_file(), &text::recent_roms());
    assert_eq!(1, recent.len());
    assert_eq!(text::nothing_here(), recent[0].label);

    for index in 0..12 {
        harness
            .host
            .settings_store_mut()
            .expect("a settings store")
            .games_mut()
            .note_rom_opened(&format!("C:\\roms\\rom{index}.nes"));
    }

    let recent = submenu_of(&mut harness, &text::menu_file(), &text::recent_roms());
    assert_eq!(10, recent.len());
    // The file name of a Windows path, whatever the platform running the test.
    assert!(
        recent[0].label.ends_with("rom11.nes"),
        "{}",
        recent[0].label
    );
}

#[test]
fn req_ui_204_the_shortcut_table_resolves_each_combination() {
    use ShortcutCommand::Host;
    let none = ShortcutModifiers::NONE;
    let control = ShortcutModifiers::CONTROL;
    assert_eq!(Host(Command::PauseResume), single(Key::P, none).0);
    assert_eq!(Host(Command::Reset), single(Key::R, control).0);
    assert_eq!(Host(Command::Quit), single(Key::Q, control).0);
    assert_eq!(Host(Command::OpenRom), single(Key::O, control).0);
    assert_eq!(Host(Command::ShowOptions), single(Key::Comma, control).0);
    assert_eq!(
        Host(Command::ToggleFullscreen),
        single(Key::Enter, ShortcutModifiers::ALT).0
    );
    assert_eq!(
        Host(Command::ToggleTwoDimensional),
        single(Key::F11, none).0
    );
    assert_eq!(
        ShortcutCommand::Ui(UiCommand::ToggleDebugOverlay),
        single(Key::F12, none).0
    );

    // AMD-013 (Q-030): Ctrl+Fn loads slot n, Maj+Fn saves it.
    let (command, args) = single(Key::F3, control);
    assert_eq!(Host(Command::LoadSlot), command);
    assert_eq!(3, args.number);

    let (command, args) = single(Key::F3, ShortcutModifiers::SHIFT);
    assert_eq!(Host(Command::SaveSlot), command);
    assert_eq!(3, args.number);

    // An exact modifier match: P under Ctrl is nothing at all.
    assert_eq!(0, menu_shortcuts::resolve(&[Key::P], control).len());
}

#[test]
fn req_ui_204_the_shortcut_table_has_no_conflict() {
    let mut seen = HashSet::new();
    for shortcut in menu_shortcuts::all() {
        let combination = (
            shortcut.key as i32,
            shortcut.modifiers.control,
            shortcut.modifiers.shift,
            shortcut.modifiers.alt,
        );
        assert!(seen.insert(combination), "{} is used twice", shortcut.text);
        assert_ne!(shortcut.text, "");
    }
}

#[test]
fn req_ui_204_no_bare_shortcut_collides_with_a_default_game_key() {
    // Player 2 has no default keyboard binding (REQ-INP-011), so player 1's table is the whole of
    // §4.2.
    let game_keys: HashSet<i32> = PLAYER_ONE_KEYBOARD
        .iter()
        .map(|(_, key)| *key as i32)
        .collect();

    for key in menu_shortcuts::bare_keys() {
        assert!(!game_keys.contains(&(key as i32)), "{}", key.name());
    }
}

#[test]
fn req_ui_205_a_shortcut_drives_the_command_model() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    assert!(!harness.ui.state.options);

    harness
        .ui
        .handle_shortcuts(&mut harness.host, &[Key::Comma], ShortcutModifiers::CONTROL);
    harness.host.apply_commands();
    harness.pump();

    assert!(harness.ui.state.options);
}

/// The menu of the options of the game in play (Rust-only): greyed with no game; with one, it
/// draws its rows and the options of that ROM are in force while it plays.
#[test]
fn the_game_options_menu_follows_the_game_in_play() {
    use zax_app::persistence::app_settings::AppSettings as _;
    use zax_app::ui::{keys, strings};
    use zax_settings::catalogue::SettingId;
    use zax_settings::values::SettingValue;

    let mut harness = UiHarness::new(&["Castle Quest"]);
    let title = strings::get(keys::HANDHELD_HINT_GAME_OPTIONS);
    harness.ui.force_open_menu = Some(title.clone());
    harness.draw_frames(2);
    assert_eq!(None, harness.host.current_rom_path());

    let shadows = harness.host.settings().shadow();
    let rom = harness.write_rom("game.nes");
    harness.host.set_game_options(
        &rom,
        vec![(SettingId::RenderShadow, SettingValue::Bool(!shadows))],
    );
    harness.launch("g0");
    assert!(harness.host.current_rom_path().is_some());
    assert_eq!(!shadows, harness.host.settings().shadow());
    harness.ui.force_open_menu = Some(title);
    harness.draw_frames(3);
    assert_eq!(
        1,
        harness.host.game_options(&rom).len(),
        "drawing changes nothing"
    );
}

/// The console mode of the desktop (Rust-only): F1 and Home switch to it, Ctrl+S and Ctrl+L ask
/// for its state screens; the front end takes the request.
#[test]
fn the_console_shortcuts_ask_the_front_end() {
    let mut harness = UiHarness::new(&["Castle Quest"]);
    for (key, modifiers, expected) in [
        (Key::F1, ShortcutModifiers::NONE, UiCommand::ToggleConsole),
        (Key::Home, ShortcutModifiers::NONE, UiCommand::ToggleConsole),
        (Key::S, ShortcutModifiers::CONTROL, UiCommand::SaveStates),
        (Key::L, ShortcutModifiers::CONTROL, UiCommand::LoadStates),
    ] {
        harness
            .ui
            .handle_shortcuts(&mut harness.host, &[key], modifiers);
        assert_eq!(Some(expected), harness.ui.take_console_request());
    }
    // Shift+F1 still saves slot 1.
    harness
        .ui
        .handle_shortcuts(&mut harness.host, &[Key::F1], ShortcutModifiers::SHIFT);
    assert_eq!(None, harness.ui.take_console_request());
}
