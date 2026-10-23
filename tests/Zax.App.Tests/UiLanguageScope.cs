using Zax.App.Ui;

// The interface language of AMD-035 is a property of the running emulator, so it is a static of
// Zax.App. A test that switches it would be seen by a test running beside it, which is why this
// assembly runs its collections one at a time — as Zax.Render.Tests already does for the GL
// context. UiLanguageScope then puts the language back, so the order of the tests cannot matter.
[assembly: CollectionBehavior(DisableTestParallelization = true)]

namespace Zax.App.Tests;

/// <summary>
/// Holds the interface language for the span of a test and puts the previous one back
/// (AMD-035).
/// </summary>
internal sealed class UiLanguageScope : IDisposable
{
    private readonly UiLanguage _previous;

    /// <summary>Switches to <paramref name="language"/> until this scope is disposed.</summary>
    public UiLanguageScope(UiLanguage language)
    {
        _previous = UiStrings.Language;
        UiStrings.Language = language;
    }

    /// <summary>Keeps the language of the moment, whatever a test does to it afterwards.</summary>
    public UiLanguageScope()
        : this(UiStrings.Language)
    {
    }

    /// <inheritdoc />
    public void Dispose() => UiStrings.Language = _previous;
}
