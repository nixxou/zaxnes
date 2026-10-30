//! A game's achievements as a grid of their icons (Rust-only, owner request): the unlocked ones
//! first, in colour, the others grey; the pad moves over them and the one selected is told under
//! the grid. Opened from the list of the games (R2) or from the in-game menu.

use std::cell::Cell;
use std::path::PathBuf;

use super::pad::PadButton;
use crate::achievements::{AchievementSystem, KnownAchievement, KnownGame};

/// Whose achievements the grid shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridSource {
    /// A ROM of the list, as the list studied it.
    Rom(PathBuf),
    /// The game being played, as it is now.
    Playing,
}

/// The grid: whose it is, the icon selected and how the last drawing laid it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementGridModel {
    source: GridSource,
    selected: usize,
    /// Icons per line, as last drawn.
    columns: Cell<usize>,
    /// How many icons there were, as last drawn.
    count: Cell<usize>,
}

impl AchievementGridModel {
    /// The grid of `source`, on its first icon.
    #[must_use]
    pub const fn new(source: GridSource) -> Self {
        Self {
            source,
            selected: 0,
            columns: Cell::new(1),
            count: Cell::new(0),
        }
    }

    /// Whose achievements these are.
    #[must_use]
    pub const fn source(&self) -> &GridSource {
        &self.source
    }

    /// The icon selected.
    #[must_use]
    pub fn selected(&self) -> usize {
        self.selected.min(self.count.get().saturating_sub(1))
    }

    /// What the drawing laid out: `columns` icons a line, `count` in all.
    pub fn laid_out(&self, columns: usize, count: usize) {
        self.columns.set(columns.max(1));
        self.count.set(count);
    }

    /// One press of a button; `true` closes the grid.
    pub fn handle(&mut self, button: PadButton) -> bool {
        let (columns, count) = (self.columns.get(), self.count.get());
        let last = count.saturating_sub(1);
        let selected = self.selected();
        self.selected = match button {
            PadButton::Left => selected.saturating_sub(1),
            PadButton::Right => (selected + 1).min(last),
            PadButton::Up => selected.saturating_sub(columns),
            PadButton::Down if selected + columns <= last => selected + columns,
            // The last line, which may be short: its last icon.
            PadButton::Down if selected / columns < last / columns => last,
            PadButton::B | PadButton::Menu => return true,
            _ => selected,
        };
        false
    }
}

/// The game of `source` with its achievements in the grid's order (the unlocked first, then
/// as the site lists them); `None` when there is nothing to show yet.
#[must_use]
pub fn game_of(source: &GridSource, system: &AchievementSystem) -> Option<KnownGame> {
    let mut game = match source {
        GridSource::Rom(path) => system.known_game(path).flatten().cloned(),
        GridSource::Playing => system.current_game(),
    }?;
    game.achievements
        .sort_by_key(|achievement: &KnownAchievement| !achievement.unlocked);
    Some(game)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pad_moves_over_the_lines_and_stops_at_the_edges() {
        let mut grid = AchievementGridModel::new(GridSource::Playing);
        grid.laid_out(4, 10);
        grid.handle(PadButton::Down);
        assert_eq!(4, grid.selected());
        grid.handle(PadButton::Down);
        assert_eq!(8, grid.selected());
        grid.handle(PadButton::Right);
        grid.handle(PadButton::Right);
        assert_eq!(9, grid.selected());
        grid.handle(PadButton::Up);
        assert_eq!(5, grid.selected());
        grid.handle(PadButton::Down);
        grid.handle(PadButton::Down);
        assert_eq!(9, grid.selected());
        assert!(grid.handle(PadButton::B));
    }

    #[test]
    fn down_from_a_line_above_a_short_one_goes_to_its_last_icon() {
        let mut grid = AchievementGridModel::new(GridSource::Playing);
        grid.laid_out(4, 6);
        grid.handle(PadButton::Right);
        grid.handle(PadButton::Right);
        grid.handle(PadButton::Right);
        grid.handle(PadButton::Down);
        assert_eq!(5, grid.selected());
    }
}
