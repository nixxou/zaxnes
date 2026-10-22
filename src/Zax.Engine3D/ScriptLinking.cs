using Zax.Profiles;

namespace Zax.Engine3D;

/// <summary>
/// How a script is linked to an instance (REQ-E3D-321, SPEC-05 REQ-LUA-005, REQ-LUA-006).
/// </summary>
/// <remarks>
/// The Lua runtime uses this to build the per-script lists the pipeline asks for in
/// <see cref="IScriptHost.InvokeUpdateShapes"/>. It lives here because REQ-E3D-321 states the
/// rule and the pipeline's ordering depends on it.
/// </remarks>
public static class ScriptLinking
{
    /// <summary>
    /// True when a script whose tag string is <paramref name="scriptTags"/> is linked to
    /// <paramref name="instance"/>: the string contains <c>*</c> anywhere, or one of its tags
    /// equals one of the instance's (exact, case-sensitive).
    /// </summary>
    public static bool IsLinked(string? scriptTags, PatternInstance instance)
    {
        ArgumentNullException.ThrowIfNull(instance);
        if (scriptTags is null)
        {
            return false;
        }

        if (scriptTags.Contains('*', StringComparison.Ordinal))
        {
            return true;
        }

        foreach (string tag in ProfileTags.Parse(scriptTags))
        {
            if (instance.HasTag(tag))
            {
                return true;
            }
        }

        return false;
    }
}
