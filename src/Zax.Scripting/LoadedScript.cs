using Zax.Engine3D;
using Zax.Profiles;

namespace Zax.Scripting;

/// <summary>
/// The ten reserved hook names of SPEC-05 §2, in the order the runtime stores them.
/// </summary>
public enum ScriptHook
{
    /// <summary>Called at every game start (REQ-LUA-020).</summary>
    Start = 0,

    /// <summary>Called once per emulated frame, step 2 (REQ-LUA-025).</summary>
    Update = 1,

    /// <summary>Called once per linked instance, step 4 (REQ-LUA-025).</summary>
    UpdateS = 2,

    /// <summary>Called once per emulated frame, step 5 (REQ-LUA-025).</summary>
    LateUpdate = 3,

    /// <summary>Called at every game end (REQ-LUA-021).</summary>
    End = 4,

    /// <summary>Called before a pattern instance's geometry is built (REQ-LUA-030).</summary>
    BuildPattern = 5,

    /// <summary>Called after a savestate is restored (REQ-LUA-072).</summary>
    Load = 6,

    /// <summary>Called before a savestate is captured (REQ-LUA-071).</summary>
    Save = 7,

    /// <summary>Called before each emulated frame when the script pad is on (REQ-LUA-032).</summary>
    InputProcess = 8,

    /// <summary>Never called (REQ-LUA-031).</summary>
    DetectShape = 9,
}

/// <summary>
/// One script of the loaded profile as the runtime holds it: its name, its tags, its live
/// <c>enable</c> and error flags, and a registry reference per hook it defines.
/// </summary>
public sealed class LoadedScript
{
    /// <summary>How many reserved hook names there are.</summary>
    public const int HookCount = 10;

    /// <summary>The reserved hook names, in the order of <see cref="ScriptHook"/>.</summary>
    public static readonly string[] HookNames =
    [
        "Start",
        "Update",
        "UpdateS",
        "LateUpdate",
        "End",
        "BuildPattern",
        "Load",
        "Save",
        "InputProcess",
        "DetectShape",
    ];

    private readonly int[] _hooks = new int[HookCount];

    internal LoadedScript(ProfileScript source, int fileIndex)
    {
        Source = source;
        FileIndex = fileIndex;
        Name = source.Name ?? string.Empty;
        TagString = source.TagString ?? string.Empty;
        Enabled = source.Enabled;
        Universal = ProfileTags.IsUniversal(TagString);
        Tags = [.. ProfileTags.Parse(TagString)];
        Array.Fill(_hooks, 0);
    }

    /// <summary>The profile record this script was read from.</summary>
    public ProfileScript Source { get; }

    /// <summary>Position in the profile's script list, which is the chunk order (REQ-LUA-011).</summary>
    public int FileIndex { get; }

    /// <summary>Script name (REQ-PROF-038 field 1).</summary>
    public string Name { get; }

    /// <summary>Raw tag string (REQ-LUA-005).</summary>
    public string TagString { get; }

    /// <summary>True when the tag string contains <c>*</c> anywhere (REQ-LUA-005).</summary>
    public bool Universal { get; }

    /// <summary>The non-empty, trimmed tags.</summary>
    public string[] Tags { get; }

    /// <summary>
    /// The live <c>enable</c> flag (REQ-LUA-007). A script may change another's through
    /// <c>Script:Get(name).Enable</c>; the change applies at the next hook call.
    /// </summary>
    public bool Enabled { get; set; }

    /// <summary>
    /// True once compilation, the chunk or a hook raised a Lua error: the script receives no
    /// hook at all until the profile is reloaded (REQ-LUA-062).
    /// </summary>
    public bool Failed { get; internal set; }

    /// <summary>True when the script is enabled and not in error (SPEC-05 §2, "script actif").</summary>
    public bool Active => Enabled && !Failed;

    /// <summary>The instances this script's <c>UpdateS</c> must visit this frame (step 3).</summary>
    internal List<FrameInstance> Pending { get; } = [];

    /// <summary>Whether the script defines <paramref name="hook"/>.</summary>
    public bool Defines(ScriptHook hook) => _hooks[(int)hook] != 0;

    internal int HookRef(ScriptHook hook) => _hooks[(int)hook];

    internal void SetHookRef(ScriptHook hook, int reference) => _hooks[(int)hook] = reference;

    internal IEnumerable<int> HookRefs => _hooks.Where(static reference => reference != 0);

    /// <summary>
    /// True when this script is linked to <paramref name="instance"/> (REQ-LUA-006): the tag
    /// string contains <c>*</c>, or one of its tags equals one of the instance's.
    /// </summary>
    public bool IsLinkedTo(PatternInstance instance)
    {
        ArgumentNullException.ThrowIfNull(instance);
        if (Universal)
        {
            return true;
        }

        foreach (string tag in Tags)
        {
            if (instance.HasTag(tag))
            {
                return true;
            }
        }

        return false;
    }

    /// <inheritdoc/>
    public override string ToString() => Name;
}
