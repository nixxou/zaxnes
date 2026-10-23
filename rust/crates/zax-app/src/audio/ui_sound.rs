//! The four sounds of the interface (SPEC-11 §3.2) and what supplies them.

use zax_platform::WaveAudio;

/// An interface sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiSound {
    /// Another tab was selected.
    TabChange,
    /// Another page of a list was selected.
    PageChange,
    /// The selection moved.
    Move,
    /// A choice was validated.
    Confirm,
}

impl UiSound {
    /// Every sound, in declaration order.
    pub const ALL: [Self; 4] = [Self::TabChange, Self::PageChange, Self::Move, Self::Confirm];
}

/// Where the sounds and the menu music tracks come from.
pub trait UiAudioLibrary {
    /// How many menu music tracks there are.
    fn track_count(&self) -> usize;

    /// The clip of `sound`, if there is one.
    fn sound(&self, sound: UiSound) -> Option<&WaveAudio>;

    /// Loads track `index`, or `None` when it cannot be read.
    fn load_track(&self, index: usize) -> Option<WaveAudio>;
}
