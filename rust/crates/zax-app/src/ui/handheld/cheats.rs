//! The cheats of the game in play on the console interface (lot CHEATS), from the in-game menu:
//! the switch of them all, the codes of the cheat database's sheet taken in one press, and each
//! cheat on or off. Typing codes and the RAM search are the desktop's windows: a pad has no
//! keyboard worth the name for them.

use zax_cheats::sheet::SheetMatch;

use super::pad::PadButton;
use crate::application::AppHost;
use crate::emulation::host::cheats::first_line;
use crate::ui::{keys, strings};

/// The rows before the cheats.
const FIXED_ROWS: usize = 2;

/// What a row of the page shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheatRow {
    /// Its name.
    pub label: String,
    /// What is on its right.
    pub value: String,
    /// Whether the value reads as a warning (a sheet made for another version).
    pub warning: bool,
}

/// The page.
#[derive(Debug, Default)]
pub struct CheatsPage {
    selected: usize,
    /// What the database said for the game, looked up as the page opens: the sheet's name, how
    /// sure the match is and its cheats, or why there is none.
    sheet: Option<Result<Sheet, String>>,
}

/// A sheet of the database: its name, how sure its match is, its cheats.
type Sheet = (String, SheetMatch, Vec<zax_cheats::Cheat>);

impl CheatsPage {
    /// The page on its first row, the game's sheet looked up.
    #[must_use]
    pub fn open(host: &mut AppHost) -> Self {
        let sheet = match host.cheat_sheet() {
            Ok(Some(sheet)) => Some(Ok((sheet.name, sheet.matched, sheet.imported.cheats))),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        };
        Self { selected: 0, sheet }
    }

    /// The selected row.
    #[must_use]
    pub const fn selected(&self) -> usize {
        self.selected
    }

    /// The rows, in order.
    #[must_use]
    pub fn rows(&self, host: &AppHost) -> Vec<CheatRow> {
        let on = host.cheats_on();
        let state = |enabled: bool| {
            strings::get(if enabled {
                keys::CHEATS_STATE_ON
            } else {
                keys::CHEATS_STATE_OFF
            })
        };
        let mut rows = vec![CheatRow {
            label: strings::get(keys::CHEATS_ALL_ON),
            value: state(on),
            warning: false,
        }];
        rows.push(match &self.sheet {
            None => CheatRow {
                label: strings::get(keys::CHEATS_DATABASE_HEADER),
                value: strings::get(if host.cheat_database_path().is_some() {
                    keys::CHEATS_NO_SHEET
                } else {
                    keys::CHEATS_NO_DATABASE
                }),
                warning: false,
            },
            Some(Err(error)) => CheatRow {
                label: strings::get(keys::CHEATS_DATABASE_HEADER),
                value: error.clone(),
                warning: true,
            },
            Some(Ok((name, matched, _))) => CheatRow {
                label: strings::get(keys::CHEATS_ADD_ALL),
                value: name.clone(),
                warning: matches!(matched, SheetMatch::SameRegion | SheetMatch::OtherRegion),
            },
        });
        rows.extend(host.cheats().cheats.iter().map(|cheat| CheatRow {
            label: first_line(&cheat.description).to_owned(),
            value: state(cheat.enabled && on),
            warning: false,
        }));
        rows
    }

    /// The note under the list: how sure the sheet's match is, when it is not certain.
    #[must_use]
    pub fn note(&self) -> Option<String> {
        match &self.sheet {
            Some(Ok((_, SheetMatch::SameRegion, _))) => {
                Some(strings::get(keys::CHEATS_MATCH_SAME_REGION))
            }
            Some(Ok((_, SheetMatch::OtherRegion, _))) => {
                Some(strings::get(keys::CHEATS_MATCH_OTHER_REGION))
            }
            _ => None,
        }
    }

    /// A press: `true` when the page is left.
    pub fn handle(&mut self, button: PadButton, host: &mut AppHost) -> bool {
        let count = FIXED_ROWS + host.cheats().cheats.len();
        match button {
            PadButton::Up => self.selected = self.selected.checked_sub(1).unwrap_or(count - 1),
            PadButton::Down => self.selected = (self.selected + 1) % count,
            PadButton::A | PadButton::Left | PadButton::Right => self.press(host),
            PadButton::B => return true,
            _ => {}
        }
        false
    }

    fn press(&mut self, host: &mut AppHost) {
        match self.selected {
            0 => host.set_cheats_on(!host.cheats_on()),
            1 => {
                let Some(Ok((_, _, cheats))) = &self.sheet else {
                    return;
                };
                let mut list = host.cheats().clone();
                for cheat in cheats {
                    if !list.cheats.iter().any(|own| own.codes == cheat.codes) {
                        let mut cheat = cheat.clone();
                        cheat.enabled = false;
                        list.cheats.push(cheat);
                    }
                }
                host.replace_cheats(list);
            }
            row => {
                let mut list = host.cheats().clone();
                if let Some(cheat) = list.cheats.get_mut(row - FIXED_ROWS) {
                    cheat.enabled = !cheat.enabled;
                    if cheat.enabled {
                        host.set_cheats_on(true);
                    }
                    host.replace_cheats(list);
                }
            }
        }
    }
}
