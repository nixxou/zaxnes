//! The profile chooser as data: the list of games, the search text and the selection
//! (AMD-010 REQ-UI-201, SPEC-10 §2). It draws nothing.

use zax_profiles::catalogue::ProfileOrigin;
use zax_profiles::library::{search, GameLibrary, GameProfileSource, LibrarySortOption};

use super::keys;
use super::strings;

/// One line of the chooser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileChoice {
    /// The identifier of the game.
    pub id: String,
    /// The title shown.
    pub title: String,
    /// Where the profile comes from.
    pub source: GameProfileSource,
    /// The "2D Mode" entry.
    pub is_two_dimensional: bool,
    /// The generic 3D entry, without a profile.
    pub is_default_three_d: bool,
    /// Which folder a user profile was found in.
    pub origin: ProfileOrigin,
    /// Whether the entry is marked as a favourite (AMD-053 REQ-UI-210). A favourite entry is
    /// distinguished visually; the form of the marker is free.
    pub favorite: bool,
}

impl ProfileChoice {
    /// Whether the entry is one of the two generic ones.
    #[must_use]
    pub const fn is_generic(&self) -> bool {
        self.is_two_dimensional || self.is_default_three_d
    }

    /// The label of the source column, in the language in force.
    #[must_use]
    pub fn source_label(&self) -> String {
        strings::get(if self.is_two_dimensional {
            keys::PROFILE_SOURCE_TWO_DIMENSIONAL
        } else if self.is_default_three_d {
            keys::PROFILE_SOURCE_DEFAULT_THREE_D
        } else if self.source != GameProfileSource::User {
            keys::PROFILE_SOURCE_BUILT_IN
        } else if self.origin == ProfileOrigin::ProgramFolder {
            keys::PROFILE_SOURCE_PROGRAM_FOLDER
        } else {
            keys::PROFILE_SOURCE_USER
        })
    }
}

/// The games of the library, in the order of the chooser, with the search filter.
#[derive(Debug, Clone)]
pub struct ProfileChooserModel {
    all: Vec<ProfileChoice>,
    filtered: Vec<usize>,
    search: String,
    favorites_only: bool,
    /// The ROM the chosen profile will launch; none when the chooser only picks a profile.
    pub rom_path: Option<String>,
    /// The identifier of the selected game.
    pub selected_id: Option<String>,
}

impl ProfileChooserModel {
    /// Builds the list from `library`: "2D Mode" first, then the generic 3D entry, then the
    /// titles in the order of `option` (Q-065 for the two generic entries, SPEC-09 REQ-PERS-033
    /// for the rest, « Favorites first » included).
    #[must_use]
    pub fn with_order(library: &GameLibrary, option: LibrarySortOption) -> Self {
        // `sorted` settles the order inside each rank, so a stable sort by rank keeps it.
        let mut all: Vec<ProfileChoice> = library
            .sorted(option)
            .into_iter()
            .map(|entry| ProfileChoice {
                id: entry.id().to_owned(),
                title: entry.display_title().to_owned(),
                source: entry.source(),
                is_two_dimensional: entry.is_2d_mode(),
                is_default_three_d: entry.is_default_three_dimensional(),
                origin: entry.origin(),
                favorite: entry.favorite,
            })
            .collect();
        all.sort_by_key(|choice| {
            if choice.is_two_dimensional {
                0
            } else {
                u8::from(!choice.is_default_three_d) + 1
            }
        });
        let mut model = Self {
            all,
            filtered: Vec::new(),
            search: String::new(),
            favorites_only: false,
            rom_path: None,
            selected_id: None,
        };
        model.refresh();
        model
    }

    /// The list in alphabetical order, which is what the chooser showed before `ui.sort_option`
    /// reached it.
    #[must_use]
    pub fn new(library: &GameLibrary) -> Self {
        Self::with_order(library, LibrarySortOption::Alphabetical)
    }

    /// Every game, in the order of the chooser.
    #[must_use]
    pub fn all(&self) -> &[ProfileChoice] {
        &self.all
    }

    /// The games whose title contains the search text, in order.
    pub fn filtered(&self) -> impl Iterator<Item = &ProfileChoice> {
        self.filtered
            .iter()
            .filter_map(|index| self.all.get(*index))
    }

    /// The search text.
    #[must_use]
    pub fn search(&self) -> &str {
        &self.search
    }

    /// Sets the search text and filters the list.
    pub fn set_search(&mut self, value: &str) {
        value.clone_into(&mut self.search);
        self.refresh();
    }

    /// Whether the list keeps only the favourites (AMD-053 REQ-UI-210, command
    /// `library.filter_favorites`).
    #[must_use]
    pub const fn favorites_only(&self) -> bool {
        self.favorites_only
    }

    /// Ticks or unticks the favourites filter and filters the list.
    pub fn set_favorites_only(&mut self, favorites_only: bool) {
        self.favorites_only = favorites_only;
        self.refresh();
    }

    /// Marks or unmarks the entry of `id` in the list, so the marker follows the toggle without
    /// the whole list being rebuilt.
    pub fn set_favorite(&mut self, id: &str, favorite: bool) {
        if let Some(choice) = self.all.iter_mut().find(|choice| choice.id == id) {
            choice.favorite = favorite;
        }
        self.refresh();
    }

    /// Starts a new choice for `rom_path`: no selection, no search. The favourites filter is left
    /// as the user set it.
    pub fn reset(&mut self, rom_path: Option<&str>) {
        self.rom_path = rom_path.map(str::to_owned);
        self.selected_id = None;
        self.set_search("");
    }

    /// REQ-UI-211: an entry is kept when it satisfies **both** the search — case and accents
    /// ignored — and the favourites filter.
    fn refresh(&mut self) {
        self.filtered = self
            .all
            .iter()
            .enumerate()
            .filter(|(_, choice)| {
                (!self.favorites_only || choice.favorite)
                    && search::matches(&choice.title, &self.search)
            })
            .map(|(index, _)| index)
            .collect();
    }
}
