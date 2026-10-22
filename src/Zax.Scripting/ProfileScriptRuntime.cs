using System.Diagnostics;
using System.Globalization;
using KeraLua;
using Zax.Engine3D;
using Zax.Profiles;

namespace Zax.Scripting;

/// <summary>
/// The Lua runtime of one profile: a single Lua 5.4 state shared by every script of the profile
/// (REQ-LUA-010), the script model and the hooks of SPEC-05.
/// </summary>
/// <remarks>
/// <para>
/// One runtime per loaded profile. Creating it installs the base environment (§4.1) and runs
/// every chunk in file order (REQ-LUA-011); the host then applies the profile's settings, builds
/// the patterns — which calls <c>BuildPattern</c> through <see cref="IScriptHost"/> — and calls
/// <see cref="InvokeStart"/>, which is the order of REQ-LUA-015. Relaunching the same game is
/// <see cref="InvokeEnd"/> followed by <see cref="InvokeStart"/>, with the state kept
/// (REQ-LUA-013); changing game disposes the runtime and creates another (REQ-LUA-014).
/// </para>
/// <para>
/// A Lua error never escapes: it marks the script in error, is logged once and the sequence
/// carries on with the other scripts (REQ-LUA-060 to 063). No managed exception crosses into
/// Lua and no Lua error crosses into the host.
/// </para>
/// </remarks>
public sealed class ProfileScriptRuntime : IScriptHost, IDisposable
{
    private const int BudgetCheckInstructions = 100_000;

    private readonly Lua _state;
    private readonly LuaBinder _binder;
    private readonly ScriptEnvironment _environment;
    private readonly ConversionEngine _engine;
    private readonly ScriptSettings _settings;
    private readonly List<LoadedScript> _byFileOrder = [];
    private readonly LuaHookFunction _budgetHook;
    private readonly List<LuaFunction> _keepAlive = [];
    private readonly Stopwatch _clock = Stopwatch.StartNew();

    private readonly List<FrameInstance> _instanceQuery = [];
    private readonly List<PatternInstance> _patternQuery = [];
    private readonly List<FrameShape> _pinnedQuery = [];
    private readonly ApiTable?[] _tables = new ApiTable?[(int)HostTypeId.Count];

    private LoadedScript[] _ordered = [];
    private LoadedScript? _inputProcessScript;
    private int _originalToStringRef;
    private long _deadline = long.MaxValue;
    private bool _disposed;

    private ProfileScriptRuntime(ScriptEnvironment environment)
    {
        _environment = environment;
        _engine = environment.Engine;
        _settings = environment.Settings ?? new ScriptSettings(_engine.Settings);
        _state = new Lua(openLibs: true);
        _binder = new LuaBinder(_state);
        _budgetHook = BudgetExceeded;
    }

    /// <summary>The scripts of the profile, in the hook order of REQ-LUA-004.</summary>
    public IReadOnlyList<LoadedScript> Scripts => _ordered;

    /// <summary>The settings model the scripts see (<c>Setting</c>).</summary>
    public ScriptSettings Settings => _settings;

    /// <summary>The environment the host supplied.</summary>
    public ScriptEnvironment Environment => _environment;

    /// <summary>The frame counter the last <see cref="SetFrameGlobals"/> published.</summary>
    public int FrameCounter { get; private set; }

    /// <summary>Whether the script pad is on (<c>ScriptPad:Enable</c>, REQ-LUA-091).</summary>
    public bool ScriptPadEnabled { get; internal set; }

    /// <summary>Which player the script pad feeds, 0 or 1 (REQ-LUA-091).</summary>
    public int ScriptPadIndex { get; internal set; }

    /// <summary>The last value <c>InputProcess</c> returned (REQ-LUA-032).</summary>
    public int ScriptPadState { get; internal set; }

    /// <summary>The shape the subjective camera follows, or null (<c>Camera.FpvShape</c>).</summary>
    public FrameShape? FpvShape { get; internal set; }

    /// <summary>How long one hook call may run before it is cut off (REQ-LUA-064).</summary>
    public TimeSpan HookBudget { get; set; } = TimeSpan.FromSeconds(1);

    /// <inheritdoc/>
    public bool HasFrameHooks { get; private set; }

    /// <summary>The conversion engine the frame queries act on.</summary>
    internal ConversionEngine Engine => _engine;

    /// <summary>The binder, for the API registration.</summary>
    internal LuaBinder Binder => _binder;

    /// <summary>The state, for the API registration.</summary>
    internal Lua State => _state;

    /// <summary>The scripts in file order, which is the <c>InputProcess</c> order of REQ-LUA-032.</summary>
    internal IReadOnlyList<LoadedScript> ScriptsInFileOrder => _byFileOrder;

    /// <summary>The shared instance list of <c>Frame:GetShapesWithTag</c> (REQ-LUA-048).</summary>
    internal List<FrameInstance> InstanceQuery => _instanceQuery;

    /// <summary>The shared instance list of <c>Pattern.GetPatternsWithTag</c> (REQ-LUA-048).</summary>
    internal List<PatternInstance> PatternQuery => _patternQuery;

    /// <summary>The shared shape list of <c>Frame:GetShapesFromPinnedFrame</c> (REQ-LUA-048).</summary>
    internal List<FrameShape> PinnedQuery => _pinnedQuery;

    /// <summary>
    /// The single object that carries the identity of the API table <paramref name="kind"/>.
    /// Held per runtime, so two runtimes in one process share nothing (REQ-ARCH-011).
    /// </summary>
    internal ApiTable TableOf(HostTypeId kind)
        => _tables[(int)kind] ??= new ApiTable(this, kind);

    /// <summary>
    /// Creates the runtime of <paramref name="profile"/>: installs the base environment and runs
    /// every chunk in file order (REQ-LUA-011).
    /// </summary>
    /// <param name="profile">The loaded profile; its <c>scripts</c> field is read.</param>
    /// <param name="environment">What the runtime needs from the host.</param>
    public static ProfileScriptRuntime Create(GameProfile profile, ScriptEnvironment environment)
    {
        ArgumentNullException.ThrowIfNull(profile);
        ArgumentNullException.ThrowIfNull(environment);

        ProfileScriptRuntime runtime = new(environment);
        runtime.Build(profile);
        return runtime;
    }

    private void Build(GameProfile profile)
    {
        SandboxLibraries();
        LuaApi.Register(_binder, this);
        _binder.Install();
        InstallGlobals();
        LoadScripts(profile);
        _state.SetHook(_budgetHook, LuaHookMask.Count, BudgetCheckInstructions);
    }

    #region libraries and base environment

    /// <summary>
    /// Keeps the libraries REQ-LUA-050 allows and removes the rest: no <c>io</c>, no
    /// <c>debug</c>, no <c>package</c>, no <c>require</c>, no file access; <c>os</c> keeps only
    /// <c>clock</c>, <c>time</c> and <c>date</c>. The 5.1 globals REQ-LUA-101 point 4 asks for
    /// are added back.
    /// </summary>
    private void SandboxLibraries()
    {
        const string sandbox = """
            io = nil
            debug = nil
            package = nil
            require = nil
            dofile = nil
            loadfile = nil

            local clock, time, date = os.clock, os.time, os.date
            os = { clock = clock, time = time, date = date }

            -- 5.1 compatibility (REQ-LUA-101 point 4).
            unpack = table.unpack
            table.getn = function (t) return #t end
            loadstring = function (s, name) return load(s, name) end
            math.pow = function (x, y) return x ^ y end
            math.ldexp = function (m, e) return m * 2.0 ^ e end
            math.log10 = function (x) return math.log(x, 10) end
            """;

        if (_state.DoString(sandbox))
        {
            string message = _state.ToString(-1) ?? "unknown error";
            _state.SetTop(0);
            throw new LuaScriptException($"The Lua base environment failed to install: {message}");
        }

        _state.SetTop(0);
    }

    private void InstallGlobals()
    {
        // print and tostring have to write an integral float without a decimal part, as the
        // single numeric type of the original dialect did (REQ-LUA-052, REQ-LUA-101 point 1).
        _state.GetGlobal("tostring");
        _originalToStringRef = _state.Ref(LuaRegistry.Index);

        Register("print", Print);
        Register("tostring", ToStringGlobal);
        // Emptied the scripting console of the original; nothing to do (REQ-LUA-053).
        Register("clear", static (_, _) => 0);
        Register("GetPData", GetPersistentData);
        Register("SetPData", SetPersistentData);
        Register("RenewId", (_, _) =>
        {
            _environment.Profile.RenewId();
            return 0;
        });

        // PData[1..64] = 0 and pdata_count = 64 (REQ-LUA-070).
        _state.CreateTable(PersistentData.EntryCount, 0);
        for (int i = 1; i <= PersistentData.EntryCount; i++)
        {
            _state.PushNumber(0.0);
            _state.RawSetInteger(-2, i);
        }

        _state.SetGlobal("PData");
        _state.PushNumber(PersistentData.EntryCount);
        _state.SetGlobal("pdata_count");

        // frame and time are 0 before the first converted frame (REQ-LUA-023).
        _state.PushNumber(0.0);
        _state.SetGlobal("frame");
        _state.PushNumber(0.0);
        _state.SetGlobal("time");

        LuaApi.InstallGlobals(_binder, this);
        _state.SetTop(0);
    }

    private void Register(string name, LuaInvoke body)
    {
        LuaFunction thunk = pointer => body(_binder, Lua.FromIntPtr(pointer)!);
        _keepAlive.Add(thunk);
        _state.PushCFunction(thunk);
        _state.SetGlobal(name);
    }

    private int Print(LuaBinder binder, Lua state)
    {
        int count = state.GetTop();
        System.Text.StringBuilder line = new();
        for (int i = 1; i <= count; i++)
        {
            if (i > 1)
            {
                line.Append('\t');
            }

            line.Append(TextOf(state, i));
        }

        _environment.Log.Print(line.ToString());
        _ = binder;
        return 0;
    }

    private int ToStringGlobal(LuaBinder binder, Lua state)
    {
        state.PushString(TextOf(state, 1));
        _ = binder;
        return 1;
    }

    /// <summary>
    /// The text of one argument: a number through the dialect's own format, anything else
    /// through the <c>tostring</c> the state was created with (REQ-LUA-052).
    /// </summary>
    private string TextOf(Lua state, int relativeIndex)
    {
        // Absolute, because the original tostring is pushed on top before the value is copied.
        int index = state.AbsIndex(relativeIndex);
        if (state.Type(index) == LuaType.Number)
        {
            return LuaNumberFormat.ToText(state.ToNumber(index));
        }

        state.RawGetInteger(LuaRegistry.Index, _originalToStringRef);
        state.PushCopy(index);
        if (state.PCall(1, 1, 0) != LuaStatus.OK)
        {
            state.Pop(1);
            return state.TypeName(state.Type(index));
        }

        string text = state.ToString(-1) ?? string.Empty;
        state.Pop(1);
        return text;
    }

    private int GetPersistentData(LuaBinder binder, Lua state)
    {
        int index = binder.CheckInteger(state, 1, "GetPData");
        state.GetGlobal("PData");
        state.RawGetInteger(-1, PersistentData.WrapIndex(index));
        state.Remove(-2);
        return 1;
    }

    private int SetPersistentData(LuaBinder binder, Lua state)
    {
        int index = binder.CheckInteger(state, 1, "SetPData");
        state.GetGlobal("PData");
        state.PushCopy(2);
        state.RawSetInteger(-2, PersistentData.WrapIndex(index));
        state.Pop(1);
        return 0;
    }

    #endregion

    #region script loading (REQ-LUA-001, 003, 004, 011)

    private void LoadScripts(GameProfile profile)
    {
        Dictionary<string, LoadedScript> byName = new(StringComparer.Ordinal);
        int fileIndex = 0;
        foreach (ProfileScript source in profile.Scripts)
        {
            LoadedScript script = new(source, fileIndex++);
            if (byName.TryGetValue(script.Name, out LoadedScript? previous))
            {
                // REQ-LUA-003: keep the last one read and say so in the log.
                _environment.Log.Warning(
                    $"Lua: the profile carries two scripts named '{script.Name}'; "
                        + "the last one is kept.");
                _byFileOrder.Remove(previous);
            }

            byName[script.Name] = script;
            _byFileOrder.Add(script);
            Compile(script);
        }

        _ordered = [.. _byFileOrder];
        Array.Sort(_ordered, static (left, right) => ScriptNameComparer.Instance.Compare(left.Name, right.Name));

        // REQ-LUA-032: the script pad comes from the last script loaded that defines
        // InputProcess, the file order deciding.
        for (int i = _byFileOrder.Count - 1; i >= 0; i--)
        {
            if (_byFileOrder[i].Defines(ScriptHook.InputProcess))
            {
                _inputProcessScript = _byFileOrder[i];
                break;
            }
        }

        foreach (LoadedScript script in _ordered)
        {
            if (script.Defines(ScriptHook.Update)
                || script.Defines(ScriptHook.UpdateS)
                || script.Defines(ScriptHook.LateUpdate))
            {
                HasFrameHooks = true;
                break;
            }
        }
    }

    private void Compile(LoadedScript script)
    {
        // (a) the ten hook globals go to nil, so that a script never inherits another's hook
        // (REQ-LUA-011).
        foreach (string hook in LoadedScript.HookNames)
        {
            _state.PushNil();
            _state.SetGlobal(hook);
        }

        string source = script.Source.Source ?? string.Empty;
        string chunkName = "=" + (script.Name.Length == 0 ? "script" : script.Name);

        if (_state.LoadString(source, chunkName) != LuaStatus.OK)
        {
            string message = _state.ToString(-1) ?? "syntax error";
            _state.SetTop(0);
            script.Failed = true;
            _environment.Log.Error($"Lua: script '{script.Name}' failed to compile: {message}");
            return;
        }

        // (b) the chunk runs once. Globals it defined before an error stay in place
        // (REQ-LUA-060).
        BeginBudget();
        LuaStatus status = _state.PCall(0, 0, 0);
        EndBudget();
        if (status != LuaStatus.OK)
        {
            string message = _state.ToString(-1) ?? "error";
            _state.SetTop(0);
            script.Failed = true;
            _environment.Log.Error($"Lua: script '{script.Name}' failed to run: {message}");
            return;
        }

        _state.SetTop(0);

        // (c) the hook globals that hold a function become this script's hooks.
        for (int i = 0; i < LoadedScript.HookCount; i++)
        {
            _state.GetGlobal(LoadedScript.HookNames[i]);
            if (_state.IsFunction(-1))
            {
                script.SetHookRef((ScriptHook)i, _state.Ref(LuaRegistry.Index));
            }
            else
            {
                _state.Pop(1);
            }
        }
    }

    #endregion

    #region hooks

    /// <summary>
    /// Calls <c>Start</c> of every script that is not in error, active or not, in script order
    /// (REQ-LUA-020).
    /// </summary>
    public void InvokeStart()
    {
        foreach (LoadedScript script in _ordered)
        {
            if (!script.Failed)
            {
                CallHook(script, ScriptHook.Start);
            }
        }
    }

    /// <summary>
    /// Calls <c>End</c> of every script that is not in error, active or not, in script order
    /// (REQ-LUA-021).
    /// </summary>
    public void InvokeEnd()
    {
        foreach (LoadedScript script in _ordered)
        {
            if (!script.Failed)
            {
                CallHook(script, ScriptHook.End);
            }
        }
    }

    /// <summary>Calls <c>Save</c> of every active script, in script order (REQ-LUA-071).</summary>
    public void InvokeSave()
    {
        foreach (LoadedScript script in _ordered)
        {
            if (script.Active)
            {
                CallHook(script, ScriptHook.Save);
            }
        }
    }

    /// <summary>Calls <c>Load</c> of every active script, in script order (REQ-LUA-072).</summary>
    public void InvokeLoad()
    {
        foreach (LoadedScript script in _ordered)
        {
            if (script.Active)
            {
                CallHook(script, ScriptHook.Load);
            }
        }
    }

    /// <inheritdoc/>
    public void SetFrameGlobals(int frameCounter, float time)
    {
        FrameCounter = frameCounter;
        _state.PushNumber(frameCounter);
        _state.SetGlobal("frame");
        _state.PushNumber(time);
        _state.SetGlobal("time");
    }

    /// <inheritdoc/>
    public void InvokeUpdate()
    {
        foreach (LoadedScript script in _ordered)
        {
            if (script.Active)
            {
                CallHook(script, ScriptHook.Update);
            }
        }
    }

    /// <inheritdoc/>
    public void InvokeUpdateShapes(IReadOnlyList<FrameInstance> scriptableInstances)
    {
        ArgumentNullException.ThrowIfNull(scriptableInstances);

        // Step 3: the per-script lists, in the instance order of REQ-E3D-220. A script turned
        // off during step 2 gets no UpdateS at all.
        bool any = false;
        foreach (LoadedScript script in _ordered)
        {
            script.Pending.Clear();
            if (script.Active && script.Defines(ScriptHook.UpdateS))
            {
                any = true;
            }
        }

        if (!any)
        {
            return;
        }

        foreach (FrameInstance instance in scriptableInstances)
        {
            foreach (LoadedScript script in _ordered)
            {
                if (script.Active && script.Defines(ScriptHook.UpdateS) && script.IsLinkedTo(instance))
                {
                    script.Pending.Add(instance);
                }
            }
        }

        // Step 4: the calls are grouped per script, all of its instances then the next script.
        // A script turned off during this step keeps the calls left in its list; only an error
        // drops them (REQ-LUA-025 notes, REQ-LUA-061).
        foreach (LoadedScript script in _ordered)
        {
            if (script.Pending.Count == 0 || script.Failed)
            {
                continue;
            }

            foreach (FrameInstance instance in script.Pending)
            {
                if (script.Failed)
                {
                    break;
                }

                _binder.PushObject(_state, HostTypeId.Instance, instance);
                _state.SetGlobal("shape");
                _binder.PushObject(_state, HostTypeId.Shape, instance.Shape);
                _state.SetGlobal("shape2D");
                CallHook(script, ScriptHook.UpdateS);
            }

            script.Pending.Clear();
        }
    }

    /// <inheritdoc/>
    public void InvokeLateUpdate()
    {
        foreach (LoadedScript script in _ordered)
        {
            if (script.Active)
            {
                CallHook(script, ScriptHook.LateUpdate);
            }
        }
    }

    /// <inheritdoc/>
    public void InvokeBuildPattern(PatternInstance patternInstance)
    {
        ArgumentNullException.ThrowIfNull(patternInstance);

        foreach (LoadedScript script in _ordered)
        {
            if (!script.Active
                || !script.Defines(ScriptHook.BuildPattern)
                || !script.IsLinkedTo(patternInstance))
            {
                continue;
            }

            _binder.PushObject(_state, HostTypeId.Instance, patternInstance);
            _state.SetGlobal("pattern");
            CallHook(script, ScriptHook.BuildPattern);
        }
    }

    /// <summary>
    /// Calls <c>InputProcess</c> of the script that owns the script pad and returns the action
    /// mask to or into the player's (REQ-LUA-032, SPEC-08 REQ-INP-071).
    /// </summary>
    /// <param name="frameCounter">The counter of the frame about to run.</param>
    /// <param name="time">Its time in seconds.</param>
    public int InvokeInputProcess(int frameCounter, float time)
    {
        if (!ScriptPadEnabled || _inputProcessScript is not { } script || !script.Active)
        {
            ScriptPadState = 0;
            return 0;
        }

        SetFrameGlobals(frameCounter, time);

        int top = _state.GetTop();
        _state.RawGetInteger(LuaRegistry.Index, script.HookRef(ScriptHook.InputProcess));
        _state.PushNumber(ScriptPadIndex);
        BeginBudget();
        LuaStatus status = _state.PCall(1, 1, 0);
        EndBudget();

        if (status != LuaStatus.OK)
        {
            Fail(script, ScriptHook.InputProcess);
            _state.SetTop(top);
            ScriptPadState = 0;
            return 0;
        }

        // Anything that is not a number contributes 0 (REQ-LUA-032).
        int mask = _state.Type(-1) == LuaType.Number
            ? LuaNumberFormat.ToInt32(_state.ToNumber(-1))
            : 0;
        _state.SetTop(top);
        ScriptPadState = mask;
        return mask;
    }

    /// <summary>
    /// Points the subjective camera at the followed shape, if any and if it is still part of
    /// the frame being converted (<c>Camera.FpvShape</c>, REQ-LUA-088).
    /// </summary>
    public void ApplyCameraTracking()
    {
        if (FpvShape is not { } shape || shape.Instances.Count == 0)
        {
            return;
        }

        // REQ-SCN-021: the tracker of SPEC-04 is what finds the shape again on the next frames.
        _engine.StartTracker(shape);
        _environment.Camera.LookAt = shape.Instances[0].GetPos(_engine.Settings);
    }

    /// <summary>
    /// Moves the subjective camera onto the followed shape for the frame being converted
    /// (SPEC-06 REQ-SCN-021): in FPV, the tracker of SPEC-04 finds the shape again and the
    /// target follows it; when the tracker loses it, the follow stops and the target stays
    /// where it was.
    /// </summary>
    public void AdvanceCameraTracking()
    {
        if (FpvShape is null || _environment.Camera.Mode != 1)
        {
            return;
        }

        FrameShape? found = _engine.Tracker.Track(_engine.ConvertingFrame);
        if (found is null || found.Instances.Count == 0)
        {
            FpvShape = null;
            _engine.Tracker.Stop();
            return;
        }

        FpvShape = found;
        _environment.Camera.LookAt = found.Instances[0].GetPos(_engine.Settings);
    }

    private void CallHook(LoadedScript script, ScriptHook hook)
    {
        int reference = script.HookRef(hook);
        if (reference == 0)
        {
            // An absent hook is simply not called (REQ-LUA-022).
            return;
        }

        int top = _state.GetTop();
        _binder.PushObject(_state, HostTypeId.ScriptObj, script);
        _state.SetGlobal("self");

        _state.RawGetInteger(LuaRegistry.Index, reference);
        BeginBudget();
        LuaStatus status = _state.PCall(0, 0, 0);
        EndBudget();
        if (status != LuaStatus.OK)
        {
            Fail(script, hook);
        }

        _state.SetTop(top);
    }

    private void Fail(LoadedScript script, ScriptHook hook)
    {
        string message = _state.ToString(-1) ?? "error";
        script.Failed = true;
        _environment.Log.Error(string.Create(
            CultureInfo.InvariantCulture,
            $"Lua: script '{script.Name}' hook {LoadedScript.HookNames[(int)hook]} raised: {message}"));
    }

    private void BeginBudget()
        => _deadline = _clock.ElapsedTicks + (long)(HookBudget.TotalSeconds * Stopwatch.Frequency);

    private void EndBudget() => _deadline = long.MaxValue;

    /// <summary>
    /// The instruction-count hook of REQ-LUA-064: a script that never gives control back is cut
    /// off as if it had raised an error, so the emulation resumes.
    /// </summary>
    private void BudgetExceeded(IntPtr statePointer, IntPtr _)
    {
        if (_clock.ElapsedTicks <= _deadline)
        {
            return;
        }

        _deadline = long.MaxValue;
        Lua state = Lua.FromIntPtr(statePointer)!;
        state.PushString("the hook used up its execution budget");
        state.Error();
    }

    #endregion

    #region persistent data (REQ-LUA-070 to 073)

    /// <summary>
    /// The 64 entries <c>PData[1]</c> to <c>PData[64]</c> as single-precision reals, the block
    /// a savestate carries (REQ-LUA-071, REQ-LUA-073).
    /// </summary>
    public byte[] CapturePersistentData()
    {
        float[] values = new float[PersistentData.EntryCount];
        _state.GetGlobal("PData");
        if (_state.IsTable(-1))
        {
            for (int i = 0; i < PersistentData.EntryCount; i++)
            {
                _state.RawGetInteger(-1, i + 1);
                values[i] = ToStoredFloat(_state, -1);
                _state.Pop(1);
            }
        }

        _state.Pop(1);
        return PersistentData.Write(values);
    }

    /// <summary>
    /// Puts the 64 entries back as Lua floats, which a savestate load does before <c>Start</c>
    /// (REQ-LUA-072).
    /// </summary>
    public void RestorePersistentData(ReadOnlySpan<byte> block)
    {
        float[] values = PersistentData.Read(block);
        _state.GetGlobal("PData");
        if (!_state.IsTable(-1))
        {
            _state.Pop(1);
            return;
        }

        for (int i = 0; i < PersistentData.EntryCount; i++)
        {
            _state.PushNumber(values[i]);
            _state.RawSetInteger(-2, i + 1);
        }

        _state.Pop(1);
    }

    /// <summary>The current value of <c>PData[index]</c> for a test or the harness.</summary>
    public double ReadPersistentData(int index)
    {
        _state.GetGlobal("PData");
        _state.RawGetInteger(-1, index);
        double value = _state.Type(-1) == LuaType.Number ? _state.ToNumber(-1) : 0.0;
        _state.Pop(2);
        return value;
    }

    private static float ToStoredFloat(Lua state, int index)
    {
        // A non-numeric value, nil included, stores 0; a numeric string stores its value
        // (REQ-LUA-071).
        if (state.Type(index) == LuaType.Number)
        {
            return (float)state.ToNumber(index);
        }

        if (state.Type(index) == LuaType.String
            && double.TryParse(
                state.ToString(index, false),
                NumberStyles.Float,
                CultureInfo.InvariantCulture,
                out double parsed))
        {
            return (float)parsed;
        }

        return 0f;
    }

    #endregion

    /// <summary>
    /// Runs a chunk in the profile's state, for the tests and the acceptance harness. The chunk
    /// sees exactly what a profile script sees.
    /// </summary>
    /// <exception cref="LuaScriptException">The chunk failed.</exception>
    public void Evaluate(string chunk)
    {
        ArgumentNullException.ThrowIfNull(chunk);
        _state.SetTop(0);
        BeginBudget();
        bool failed = _state.DoString(chunk);
        EndBudget();
        if (failed)
        {
            string message = _state.ToString(-1) ?? "error";
            _state.SetTop(0);
            throw new LuaScriptException(message);
        }

        _state.SetTop(0);
    }

    /// <summary>
    /// Runs a chunk that returns one value and gives it back as a double, for the tests.
    /// </summary>
    public double EvaluateNumber(string chunk)
    {
        ArgumentNullException.ThrowIfNull(chunk);
        _state.SetTop(0);
        BeginBudget();
        bool failed = _state.DoString(chunk);
        EndBudget();
        if (failed)
        {
            string message = _state.ToString(-1) ?? "error";
            _state.SetTop(0);
            throw new LuaScriptException(message);
        }

        double value = _state.Type(-1) == LuaType.Number ? _state.ToNumber(-1) : double.NaN;
        _state.SetTop(0);
        return value;
    }

    /// <summary>Runs a chunk that returns one value and gives it back as text, for the tests.</summary>
    public string EvaluateText(string chunk)
    {
        ArgumentNullException.ThrowIfNull(chunk);
        _state.SetTop(0);
        BeginBudget();
        bool failed = _state.DoString(chunk);
        EndBudget();
        if (failed)
        {
            string message = _state.ToString(-1) ?? "error";
            _state.SetTop(0);
            throw new LuaScriptException(message);
        }

        string text = TextOf(_state, -1);
        _state.SetTop(0);
        return text;
    }

    /// <summary>
    /// A description of the script-visible API this runtime installed: the globals of SPEC-05
    /// §4.1, the types of §4.2 and §4.3 with their members, and the constant tables of §4.4.
    /// </summary>
    public ScriptApiSurface DescribeApi() => ScriptApiSurface.From(_binder);

    /// <summary>The script of this name, or null (<c>Script:Get</c>).</summary>
    public LoadedScript? FindScript(string name)
    {
        foreach (LoadedScript script in _ordered)
        {
            if (string.Equals(script.Name, name, StringComparison.Ordinal))
            {
                return script;
            }
        }

        return null;
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _state.SetHook(null!, LuaHookMask.Disabled, 0);
        _state.Dispose();
        _keepAlive.Clear();
    }
}
