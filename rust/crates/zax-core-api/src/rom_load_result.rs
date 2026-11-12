use crate::{NesRegion, RomLoadFailure};

/// Outcome of "load a ROM" (REQ-CORE-010, REQ-CORE-011, REQ-CORE-013).
///
/// On failure the instance is left stopped, as if no ROM were loaded (REQ-CORE-005);
/// `failure` is the typed cause and `error` a technical message in English, the host
/// choosing the text it displays (AMD-002, answer to Q-001). On success `region` carries the
/// detected region, which the core does *not* apply by itself: the host calls "choose the
/// region" according to its "automatic region" setting (REQ-CORE-013).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomLoadResult {
    /// Whether the ROM was accepted.
    pub success: bool,
    /// Region detected from the header, meaningful only on success.
    pub region: NesRegion,
    /// Typed cause of the failure, [`RomLoadFailure::None`] on success.
    pub failure: RomLoadFailure,
    /// Technical message naming the cause, or `None` on success.
    pub error: Option<String>,
    /// A message about the image that is **not** a failure, for the host to log: what was
    /// detected and what of it this build does not emulate. `None` for an ordinary NES image.
    ///
    /// Lot MF-VS added it for the VS. System, whose load note names the PPU model, where the
    /// model came from and every piece the build leaves out (`crate::vs::VsSystem::load_note`);
    /// an unidentified VS image says so here, which is what the owner's brief asks for.
    pub note: Option<String>,
}

impl RomLoadResult {
    /// Builds a successful result carrying the detected region.
    #[must_use]
    pub const fn ok(region: NesRegion) -> Self {
        Self {
            success: true,
            region,
            failure: RomLoadFailure::None,
            error: None,
            note: None,
        }
    }

    /// The same, with a message about the image for the host to log ([`Self::note`]).
    #[must_use]
    pub const fn ok_with_note(region: NesRegion, note: String) -> Self {
        Self {
            success: true,
            region,
            failure: RomLoadFailure::None,
            error: None,
            note: Some(note),
        }
    }

    /// Builds a failed result carrying the typed cause and its message.
    #[must_use]
    pub const fn fail(failure: RomLoadFailure, error: String) -> Self {
        Self {
            success: false,
            region: NesRegion::Ntsc,
            failure,
            error: Some(error),
            note: None,
        }
    }
}
