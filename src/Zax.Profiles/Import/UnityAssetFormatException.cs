namespace Zax.Profiles.Import;

/// <summary>
/// Raised when a Unity asset bundle, or something inside it, cannot be read.
/// </summary>
/// <remarks>
/// AMD-030 §5 asks for « un message d'erreur clair, jamais un plantage » on a truncated file, an
/// unknown signature, an unsupported compression or an inconsistent table. Every refusal of the
/// reader is this one exception, carrying a sentence that says what was expected and what was
/// found; the importer turns it into the message the interface and the command line show.
/// </remarks>
public sealed class UnityAssetFormatException : Exception
{
    /// <summary>Creates the exception.</summary>
    public UnityAssetFormatException(string message)
        : base(message)
    {
    }

    /// <summary>Creates the exception over an underlying failure.</summary>
    public UnityAssetFormatException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}
