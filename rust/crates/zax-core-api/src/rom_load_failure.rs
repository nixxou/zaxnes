/// Typed cause of a failed "load a ROM" (REQ-CORE-011, AMD-002 answer to Q-001).
///
/// The French phrases of REQ-CORE-011 are descriptions of the cause, not literal strings:
/// the core returns this code plus a technical message in English, and the host picks the
/// text it shows (SPEC-07, SPEC-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum RomLoadFailure {
    /// No failure.
    #[default]
    None = 0,
    /// The 16-byte header is missing, too short, lacks the `4E 45 53 1A` signature or
    /// announces no PRG-ROM ("en-tête corrompu").
    CorruptHeader = 1,
    /// The image is shorter than the size the header announces ("fichier tronqué").
    TruncatedFile = 2,
    /// The mapper number is not implemented ("mapper N non supporté").
    UnsupportedMapper = 3,
}
