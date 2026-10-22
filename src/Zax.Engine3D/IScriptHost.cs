namespace Zax.Engine3D;

/// <summary>
/// What the conversion pipeline needs from the Lua runtime: the hook call points of
/// REQ-E3D-320 to 324, in the order the pipeline calls them.
/// </summary>
/// <remarks>
/// <para>
/// The pipeline owns <b>when</b> the hooks run; the runtime (SPEC-05) owns the order of the
/// scripts, the per-script lists and the error handling. In particular
/// <see cref="InvokeUpdateShapes"/> receives the scriptable instances in the order of
/// REQ-E3D-220 and the runtime groups them per script (SPEC-05 REQ-LUA-025 steps 3 and 4).
/// </para>
/// <para>
/// An implementation may throw: the pipeline catches, counts the error in
/// <see cref="ConversionFrame.ScriptErrorCount"/> and carries on, because a Lua error must not
/// interrupt the conversion (REQ-E3D-324).
/// </para>
/// </remarks>
public interface IScriptHost
{
    /// <summary>
    /// True when at least one script defines a frame hook. The pipeline uses it only to skip
    /// work; the result must not depend on it.
    /// </summary>
    bool HasFrameHooks { get; }

    /// <summary>
    /// Step 1: publishes the frame counter and the frame time as the globals <c>frame</c> and
    /// <c>time</c> (REQ-E3D-320, SPEC-05 REQ-LUA-025).
    /// </summary>
    void SetFrameGlobals(int frameCounter, float time);

    /// <summary>Step 2: calls <c>Update</c> of every active script, in script order.</summary>
    void InvokeUpdate();

    /// <summary>
    /// Steps 3 and 4: builds the per-script list of linked instances out of
    /// <paramref name="scriptableInstances"/> (REQ-E3D-321) and calls <c>UpdateS</c> once per
    /// instance, grouped by script.
    /// </summary>
    void InvokeUpdateShapes(IReadOnlyList<FrameInstance> scriptableInstances);

    /// <summary>Step 5: calls <c>LateUpdate</c> of every active script, in script order.</summary>
    void InvokeLateUpdate();

    /// <summary>
    /// Calls <c>BuildPattern</c> for every active script linked to
    /// <paramref name="patternInstance"/>, before its geometry is built (REQ-E3D-322,
    /// SPEC-05 REQ-LUA-030). Modifications the hook makes are permanent for the pattern.
    /// </summary>
    void InvokeBuildPattern(PatternInstance patternInstance);
}

/// <summary>
/// A script host that does nothing, used when no profile script is loaded and by the tests of
/// the pipeline alone.
/// </summary>
public sealed class NullScriptHost : IScriptHost
{
    /// <summary>The single instance.</summary>
    public static NullScriptHost Instance { get; } = new();

    private NullScriptHost()
    {
    }

    /// <inheritdoc/>
    public bool HasFrameHooks => false;

    /// <inheritdoc/>
    public void SetFrameGlobals(int frameCounter, float time)
    {
    }

    /// <inheritdoc/>
    public void InvokeUpdate()
    {
    }

    /// <inheritdoc/>
    public void InvokeUpdateShapes(IReadOnlyList<FrameInstance> scriptableInstances)
    {
    }

    /// <inheritdoc/>
    public void InvokeLateUpdate()
    {
    }

    /// <inheritdoc/>
    public void InvokeBuildPattern(PatternInstance patternInstance)
    {
    }
}
