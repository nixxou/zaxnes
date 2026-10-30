//! What RetroAchievements shows (Rust-only, owner request), the same in the desktop interface
//! and in the console mode: the pop-ups over the game, the challenge and progress indicators,
//! the list of the game's achievements and the account section of the options.

use imgui::{Condition, ProgressBar, StyleColor, TextureId, TreeNodeFlags, Ui, WindowFlags};

use crate::achievements::{Account, AchievementSystem, PopupKind};
use crate::application::AppHost;
use crate::ui::{keys, strings};
use zax_achievements::{Achievement, Bucket};
use zax_settings::catalogue::SettingId;

/// The colour of the first line of an unlock: RetroAchievements' gold.
const GOLD: [f32; 4] = [1.0, 0.84, 0.25, 1.0];
/// The colour of a locked achievement's texts.
const DIMMED: [f32; 4] = [0.65, 0.65, 0.65, 1.0];

/// The sign-in fields, kept between frames by the interface that draws them.
#[derive(Debug, Clone, Default)]
pub struct SignInForm {
    /// The account name typed.
    pub username: String,
    /// The password typed; emptied once sent.
    pub password: String,
}

/// The icon at `url`, `size` pixels square, or an empty square of that size while it is not
/// there.
fn badge(ui: &Ui, system: &AchievementSystem, url: &str, size: f32) {
    match system.badge(url) {
        Some(id) => imgui::Image::new(TextureId::new(id), [size, size]).build(ui),
        None => ui.dummy([size, size]),
    }
}

/// The pop-ups, the challenges and the progress over the game. `top` is where the pop-up
/// starts (under the menu bar); the challenges under way show only when `challenges` (owner
/// request: an option, off by default).
pub fn draw_overlay(ui: &Ui, system: &AchievementSystem, top: f32, challenges: bool) {
    if !system.is_enabled() {
        return;
    }
    let display = ui.io().display_size;
    let line = ui.text_line_height_with_spacing();
    let flags = WindowFlags::NO_DECORATION
        | WindowFlags::ALWAYS_AUTO_RESIZE
        | WindowFlags::NO_SAVED_SETTINGS
        | WindowFlags::NO_FOCUS_ON_APPEARING
        | WindowFlags::NO_NAV
        | WindowFlags::NO_INPUTS;
    if let Some(popup) = system.popup().cloned() {
        let window = ui
            .window("##achievement_popup")
            .position([display[0] * 0.5, top + line * 0.5], Condition::Always)
            .position_pivot([0.5, 0.0])
            .bg_alpha(0.85)
            .flags(flags)
            .begin();
        if window.is_some() {
            badge(ui, system, &popup.badge_url, line * 2.6);
            ui.same_line();
            let _group = ui.begin_group();
            let colour = if popup.kind == PopupKind::Game {
                [1.0, 1.0, 1.0, 1.0]
            } else {
                GOLD
            };
            ui.text_colored(colour, &popup.title);
            ui.text(&popup.text);
        }
    }
    let mut bottom = display[1] - line * 0.5;
    if let Some(item) = system.progress_indicator().cloned() {
        let window = ui
            .window("##achievement_progress")
            .position([display[0] - line * 0.5, bottom], Condition::Always)
            .position_pivot([1.0, 1.0])
            .bg_alpha(0.75)
            .flags(flags)
            .begin();
        if window.is_some() {
            badge(ui, system, &item.badge_url, line * 1.6);
            ui.same_line();
            let _group = ui.begin_group();
            ui.text(&item.title);
            ui.text_colored(GOLD, &item.detail);
            bottom -= ui.window_size()[1] + line * 0.25;
        }
    }
    let challenges = if challenges {
        system.challenges().to_vec()
    } else {
        Vec::new()
    };
    if !challenges.is_empty() {
        let window = ui
            .window("##achievement_challenges")
            .position([display[0] - line * 0.5, bottom], Condition::Always)
            .position_pivot([1.0, 1.0])
            .bg_alpha(0.6)
            .flags(flags)
            .begin();
        if window.is_some() {
            for (index, item) in challenges.iter().enumerate() {
                if index > 0 {
                    ui.same_line();
                }
                badge(ui, system, &item.badge_url, line * 1.6);
                if ui.is_item_hovered() {
                    ui.tooltip_text(format!(
                        "{}: {}",
                        strings::get(keys::ACH_CHALLENGE),
                        item.title
                    ));
                }
            }
        }
    }
}

/// The share of the achievements unlocked, in words: "34 %" (owner request).
#[must_use]
pub fn percent_text(unlocked: u32, total: u32) -> String {
    if total == 0 {
        return String::new();
    }
    let percent = (u64::from(unlocked) * 100 / u64::from(total)).min(100);
    strings::format(&strings::get(keys::ACH_PERCENT), &[&percent.to_string()])
}

/// The title of a group of the list.
fn bucket_title(bucket: Bucket, label: &str) -> String {
    let key = match bucket {
        Bucket::Locked => keys::ACH_BUCKET_LOCKED,
        Bucket::Unlocked => keys::ACH_BUCKET_UNLOCKED,
        Bucket::RecentlyUnlocked => keys::ACH_BUCKET_RECENT,
        Bucket::ActiveChallenge => keys::ACH_BUCKET_CHALLENGE,
        Bucket::AlmostThere => keys::ACH_BUCKET_ALMOST,
        Bucket::Unofficial => keys::ACH_BUCKET_UNOFFICIAL,
        Bucket::Unsupported => keys::ACH_BUCKET_UNSUPPORTED,
        Bucket::Unsynced => keys::ACH_BUCKET_UNSYNCED,
        Bucket::Other => return label.to_owned(),
    };
    strings::get(key)
}

/// The game's achievements: its title and the player's progress, then one group per bucket,
/// each achievement with its icon, points, description and measured progress.
pub fn draw_list(ui: &Ui, system: &AchievementSystem) {
    let Some(game) = system.game().cloned() else {
        ui.text_wrapped(strings::get(keys::ACH_LIST_EMPTY));
        return;
    };
    let line = ui.text_line_height_with_spacing();
    badge(ui, system, &game.badge_url, line * 2.6);
    ui.same_line();
    {
        let _group = ui.begin_group();
        ui.text(&game.title);
        let summary = game.summary;
        ui.text_disabled(strings::format(
            &strings::get(keys::ACH_GAME_SUMMARY),
            &[
                &summary.unlocked.to_string(),
                &summary.core.to_string(),
                &summary.points_unlocked.to_string(),
                &summary.points_core.to_string(),
            ],
        ));
        let presence = system.rich_presence();
        if !presence.is_empty() {
            ui.text_wrapped(presence);
        }
    }
    if game.summary.core > 0 {
        #[allow(clippy::cast_precision_loss)] // counts far below 2^24
        let fraction = game.summary.unlocked as f32 / game.summary.core as f32;
        ProgressBar::new(fraction)
            .overlay_text(percent_text(game.summary.unlocked, game.summary.core))
            .build(ui);
    }
    ui.separator();
    let list = system.achievements();
    let mut start = 0;
    while start < list.len() {
        let bucket = list[start].bucket;
        let end = list[start..]
            .iter()
            .position(|entry| entry.bucket != bucket)
            .map_or(list.len(), |offset| start + offset);
        let title = format!(
            "{} ({})###{:?}",
            bucket_title(bucket, &list[start].bucket_label),
            end - start,
            bucket
        );
        if ui.collapsing_header(&title, TreeNodeFlags::DEFAULT_OPEN) {
            for entry in &list[start..end] {
                draw_entry(ui, system, entry, line);
            }
        }
        start = end;
    }
}

fn draw_entry(ui: &Ui, system: &AchievementSystem, entry: &Achievement, line: f32) {
    let _id = ui.push_id_usize(entry.id as usize);
    let url = if entry.unlocked {
        &entry.badge_url
    } else {
        &entry.badge_locked_url
    };
    badge(ui, system, url, line * 2.4);
    ui.same_line();
    let _group = ui.begin_group();
    let colour = (!entry.unlocked).then(|| ui.push_style_color(StyleColor::Text, DIMMED));
    ui.text(&entry.title);
    ui.same_line();
    ui.text_disabled(strings::format(
        &strings::get(keys::ACH_POINTS),
        &[&entry.points.to_string()],
    ));
    ui.text_wrapped(&entry.description);
    drop(colour);
    if !entry.unlocked && !entry.progress.is_empty() {
        ProgressBar::new((entry.percent / 100.0).clamp(0.0, 1.0))
            .overlay_text(&entry.progress)
            .size([line * 10.0, 0.0])
            .build(ui);
    }
    if entry.rarity > 0.0 {
        ui.text_disabled(strings::format(
            &strings::get(keys::ACH_RARITY),
            &[&format!("{:.1}", entry.rarity)],
        ));
    }
}

/// The account, as a line of text.
#[must_use]
pub fn account_text(system: &AchievementSystem) -> String {
    match system.account() {
        Account::SignedOut => strings::get(keys::ACH_SIGNED_OUT),
        Account::SigningIn => strings::get(keys::ACH_SIGNING_IN),
        Account::SignedIn { name, points } => strings::format(
            &strings::get(keys::ACH_SIGNED_IN),
            &[name, &points.to_string()],
        ),
        Account::Failed(message) => {
            strings::format(&strings::get(keys::ACH_SIGN_IN_FAILED), &[message])
        }
    }
}

/// The account section of the options: the state of the account, then the sign-in fields or
/// the sign-out button. Returns whether a button was pressed.
pub fn draw_account(ui: &Ui, host: &mut AppHost, form: &mut SignInForm) -> bool {
    ui.text_wrapped(strings::get(keys::ACH_SOFTCORE_HINT));
    ui.spacing();
    if !host
        .settings()
        .get(SettingId::AchievementsEnabled)
        .as_bool()
    {
        ui.text_disabled(strings::get(keys::ACH_TURN_ON_HINT));
        return false;
    }
    let account = host.achievements.account().clone();
    let colour = match account {
        Account::SignedIn { .. } => [0.5, 1.0, 0.5, 1.0],
        Account::Failed(_) => [1.0, 0.4, 0.4, 1.0],
        _ => [1.0, 1.0, 1.0, 1.0],
    };
    ui.text_colored(colour, account_text(&host.achievements));
    if let Account::SignedIn { .. } = account {
        if ui.button(strings::get(keys::ACH_SIGN_OUT)) {
            host.sign_out_of_achievements();
            return true;
        }
        return false;
    }
    if form.username.is_empty() {
        host.settings()
            .get(SettingId::AchievementsUsername)
            .as_text()
            .clone_into(&mut form.username);
    }
    let width = ui.push_item_width(ui.text_line_height() * 14.0);
    ui.input_text(strings::get(keys::ACH_USERNAME), &mut form.username)
        .build();
    let entered = ui
        .input_text(strings::get(keys::ACH_PASSWORD), &mut form.password)
        .password(true)
        .enter_returns_true(true)
        .build();
    drop(width);
    let ready = !form.username.trim().is_empty()
        && !form.password.is_empty()
        && account != Account::SigningIn;
    let pressed = ui.button(strings::get(keys::ACH_SIGN_IN));
    if ready && (pressed || entered) {
        let username = form.username.trim().to_owned();
        let password = std::mem::take(&mut form.password);
        host.sign_in_to_achievements(&username, &password);
        return true;
    }
    ui.text_disabled(strings::get(keys::ACH_PASSWORD_HINT));
    false
}
