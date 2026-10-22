namespace Zax.Nes.Rom;

/// <summary>
/// Typed cause of a failed "load a ROM" (REQ-CORE-011, AMD-002 answer to Q-001).
/// </summary>
/// <remarks>
/// The French phrases of REQ-CORE-011 are descriptions of the cause, not literal strings:
/// the core returns this code plus a technical message in English, and the host picks the
/// text it shows (SPEC-07, SPEC-10). Only the strings written in code font in the spec are
/// literal.
/// </remarks>
public enum RomLoadFailure
{
    /// <summary>No failure.</summary>
    None = 0,

    /// <summary>
    /// The 16-byte header is missing, too short, lacks the <c>4E 45 53 1A</c> signature or
    /// announces no PRG-ROM ("en-tête corrompu").
    /// </summary>
    CorruptHeader = 1,

    /// <summary>The image is shorter than the size the header announces ("fichier tronqué").</summary>
    TruncatedFile = 2,

    /// <summary>The mapper number is not implemented ("mapper N non supporté").</summary>
    UnsupportedMapper = 3,
}
