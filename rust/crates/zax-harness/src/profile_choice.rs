//! `--profile standard|neshawk`: which of the two timing profiles of the 2A03 a command runs on.
//!
//! The profile is a **type** in `zax-nes` (`zax_nes::Standard` and `zax_nes::NesHawkCompat`, lot
//! NESHAWK-PROFILE), so a command that offers the choice dispatches once on this enum and then
//! calls a function generic over the profile. That is the whole of the run-time cost: one match,
//! once per command.
//!
//! The `.bk2` player chooses for itself from the movie's `Core` header and does not need this; the
//! option exists to **override** that choice, and to run a ROM or a movie on the other chip on
//! purpose — which is how the test-ROM matrix of `docs/review/NESHAWK-PROFILE.md` was measured.

use crate::HarnessError;

/// Which timing profile to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProfileChoice {
    /// Zaxnes's own 2A03 (`zax_nes::Standard`): the chip every game and every `.fm2` runs on.
    #[default]
    Standard,
    /// The NesHawk-compatible chip (`zax_nes::NesHawkCompat`).
    NesHawk,
}

impl ProfileChoice {
    /// The names `--profile` accepts, for a usage string.
    pub const NAMES: &'static str = "standard|neshawk";

    /// Parses a `--profile` value.
    ///
    /// # Errors
    ///
    /// [`HarnessError::Usage`] for anything but the two names, with both of them in the message:
    /// a silent fall-back to the standard profile would make a mistyped option look like a
    /// measurement of the other chip.
    pub fn parse(value: &str) -> Result<Self, HarnessError> {
        match value {
            "standard" => Ok(Self::Standard),
            "neshawk" => Ok(Self::NesHawk),
            _ => Err(HarnessError::Usage(format!(
                "--profile: {value:?} is not one of {}",
                Self::NAMES
            ))),
        }
    }

    /// The name this profile is written with, which is `zax_nes::TimingProfile::NAME`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::NesHawk => "neshawk",
        }
    }
}
