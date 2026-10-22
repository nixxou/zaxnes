using Xunit;

// GLFW is not thread-safe: initialising it, creating a window and making a context current all
// have to happen one at a time, and a GL context is current on one thread only. xUnit runs test
// collections in parallel by default, which made this assembly fail now and then — never alone,
// only when the whole solution ran and the machine was loaded enough for two collections to
// overlap. Four GL-using classes had also been left out of GlCollection, so they were free to
// build a context while another class was building one.
//
// Two measures, on purpose. GlCollection keeps the intent readable, and this attribute makes the
// whole assembly sequential whatever a future class forgets, so the suite cannot silently become
// flaky again.
[assembly: CollectionBehavior(DisableTestParallelization = true)]

namespace Zax.Render.Tests;

/// <summary>
/// Groups every test that needs a GL context, so that xUnit never creates two at once in the
/// same process. The assembly-wide attribute above is the backstop; being in this collection is
/// the statement of intent.
/// </summary>
[CollectionDefinition(Name)]
public sealed class GlCollection
{
    /// <summary>The collection name.</summary>
    public const string Name = "opengl";
}
