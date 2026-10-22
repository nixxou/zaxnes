namespace Zax.Engine3D.Tests;

/// <summary>
/// Architecture checks for Zax.Engine3D: the conversion engine produces abstract data and must
/// not depend on a windowing, graphics, audio or input API (REQ-ARCH-002).
/// </summary>
public sealed class ScaffoldTests
{
    /// <summary>
    /// Assembly name prefixes that must never appear among Zax.Engine3D's references:
    /// the engine emits abstract meshes and must not know about a graphics API
    /// (PLAN.md, libretro portability section).
    /// </summary>
    private static readonly string[] ForbiddenPrefixes =
    [
        "Silk.NET",
        "ImGui",
        "OpenTK",
        "PresentationCore",
        "System.Windows",
    ];

    [Fact]
    public void Module_is_identified()
    {
        Assert.Equal("Zax.Engine3D", Engine3DModule.Name);
        Assert.False(string.IsNullOrWhiteSpace(Engine3DModule.Role));
    }

    [Fact]
    public void REQ_ARCH_002_the_assembly_has_no_graphics_dependency()
    {
        string[] offenders = typeof(Engine3DModule).Assembly
            .GetReferencedAssemblies()
            .Select(static name => name.Name ?? string.Empty)
            .Where(static name => ForbiddenPrefixes.Any(
                prefix => name.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.Empty(offenders);
    }
}
