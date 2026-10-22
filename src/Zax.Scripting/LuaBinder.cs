using System.Runtime.CompilerServices;
using KeraLua;
using Zax.Engine3D;

namespace Zax.Scripting;

/// <summary>
/// Identity of a Lua-visible host type (SPEC-05 §4.2 and §4.3). The value is stored in the
/// first four bytes of every host userdata, which is how <c>__index</c> finds the member table
/// without a managed lookup.
/// </summary>
internal enum HostTypeId
{
    None = 0,
    Instance,
    Shape,
    Pattern,
    FrameApi,
    FrameManagerApi,
    PaletteApi,
    PatternApi,
    ScriptApi,
    ScriptObj,
    CameraApi,
    SettingApi,
    LayerConfig,
    Layer,
    Clipping,
    Light,
    BackdropApi,
    VideoApi,
    ProfileApi,
    ScriptPadApi,
    GamepadApi,
    Player,
    NotiApi,
    CustomData,
    Slot,
    InstanceList,
    ShapeList,
    Vec3Ref,
    Vec2Ref,
    Mat4Ref,
    Vector3,
    Vector2,
    Quaternion,
    Color,
    IntVector2,
    IntVector3,
    Rect,
    XrState,
    Vector3Class,
    Vector2Class,
    QuaternionClass,
    ColorClass,
    IntVector2Class,
    IntVector3Class,
    RectClass,
    Count,
}

/// <summary>Which bound vector of which owner a <c>Vec3R</c> userdata designates (§4.3).</summary>
internal enum Vec3RefKind
{
    InstanceOffset = 0,
    InstanceScale,
    InstanceRotation,
    InstancePivot,
    InstanceDeformAmplitude,
    InstanceDeformPivot,
    CameraRotation,
    CameraTarget,
    ZapperOffset,
    BackdropSize,
    BackdropOffset,
}

/// <summary>Which bound 2D vector of an instance a <c>Vec2R</c> userdata designates (§4.3).</summary>
internal enum Vec2RefKind
{
    TexOffset = 0,
    TexTilting,
}

/// <summary>Which deformation matrix of an instance a <c>Mat4R</c> userdata designates (§4.3).</summary>
internal enum Mat4RefKind
{
    DeformSpeed = 0,
    DeformOffset,
}

/// <summary>Which shared list a <c>Liste&lt;T&gt;</c> userdata designates (REQ-LUA-048).</summary>
internal enum ListKind
{
    ShapesWithTag = 0,
    PatternsWithTag,
    ShapesFromPinnedFrame,
    PatternInstances,
}

/// <summary>Reads a member; the object is at stack index 1. Returns the number of results.</summary>
internal delegate int LuaGet(LuaBinder binder, Lua state);

/// <summary>Writes a member; the object is at index 1 and the value at index 3.</summary>
internal delegate void LuaSet(LuaBinder binder, Lua state);

/// <summary>A bound function or method. Returns the number of results.</summary>
internal delegate int LuaInvoke(LuaBinder binder, Lua state);

/// <summary>
/// The description of one Lua-visible host type: its members, its metamethods and the registry
/// references of its metatable and of its two name tables.
/// </summary>
internal sealed class HostType
{
    /// <summary>Creates the description of <paramref name="id"/> named <paramref name="name"/>.</summary>
    public HostType(HostTypeId id, string name, bool reference)
    {
        Id = id;
        Name = name;
        Reference = reference;
    }

    /// <summary>The type identity stored in the userdata.</summary>
    public HostTypeId Id { get; }

    /// <summary>Name used in error messages and for the metatable.</summary>
    public string Name { get; }

    /// <summary>
    /// True when the payload is <c>(handle, member)</c> and the handle has to be released when
    /// the userdata dies; false for a value type, whose payload is the value itself.
    /// </summary>
    public bool Reference { get; }

    /// <summary>Properties and methods, in declaration order.</summary>
    public List<(string Name, LuaGet? Get, LuaSet? Set, LuaInvoke? Call, HostTypeId Result)> Members
    { get; } = [];

    /// <summary>
    /// The type of <c>obj[i]</c>, so that the static check of REQ-LUA-051 can follow an access
    /// chain through an indexing step.
    /// </summary>
    public HostTypeId ElementType { get; set; }

    /// <summary>The type <see cref="Call"/> produces, for the same reason.</summary>
    public HostTypeId CallResult { get; set; }

    /// <summary><c>obj[i]</c> read, for a type that indexes by number (REQ-LUA-045).</summary>
    public LuaGet? Indexer { get; set; }

    /// <summary><c>obj[i] = v</c>.</summary>
    public LuaSet? IndexSetter { get; set; }

    /// <summary><c>obj(...)</c>, the constructor of a value class (§4.3).</summary>
    public LuaInvoke? Call { get; set; }

    /// <summary><c>a == b</c>; both operands are of this type. Null uses the payload compare.</summary>
    public Func<LuaBinder, Lua, bool>? Equal { get; set; }

    /// <summary><c>a + b</c>.</summary>
    public LuaInvoke? Add { get; set; }

    /// <summary><c>a − b</c>.</summary>
    public LuaInvoke? Subtract { get; set; }

    /// <summary><c>a * b</c>.</summary>
    public LuaInvoke? Multiply { get; set; }

    /// <summary><c>a / b</c>.</summary>
    public LuaInvoke? Divide { get; set; }

    /// <summary><c>−a</c>.</summary>
    public LuaInvoke? Negate { get; set; }

    /// <summary><c>tostring(a)</c>.</summary>
    public LuaInvoke? Text { get; set; }

    /// <summary>Registry reference of the metatable.</summary>
    public int MetaRef { get; set; }

    /// <summary>Registry reference of the "name → getter id or function" table.</summary>
    public int GetRef { get; set; }

    /// <summary>Registry reference of the "name → setter id or false" table.</summary>
    public int SetRef { get; set; }

    /// <summary>Declares a property.</summary>
    /// <param name="name">Member name, exactly as §4.2 spells it.</param>
    /// <param name="get">Reads the member.</param>
    /// <param name="set">Writes it, or null for a read-only member.</param>
    /// <param name="result">
    /// The host type the member yields, when it yields one. It is what lets the static check of
    /// REQ-LUA-051 follow an access chain such as <c>shape.Shape2D.refPattern[0].Offset.x</c>.
    /// </param>
    public HostType Property(
        string name,
        LuaGet get,
        LuaSet? set = null,
        HostTypeId result = HostTypeId.None)
    {
        Members.Add((name, get, set, null, result));
        return this;
    }

    /// <summary>Declares a method or a table function.</summary>
    public HostType Function(string name, LuaInvoke call, HostTypeId result = HostTypeId.None)
    {
        Members.Add((name, null, null, call, result));
        return this;
    }

    /// <summary>Declares an alias of an already declared member (§4.2 alias rows).</summary>
    public HostType Alias(string name, string existing)
    {
        foreach ((string Name, LuaGet? Get, LuaSet? Set, LuaInvoke? Call, HostTypeId Result) member in Members)
        {
            if (string.Equals(member.Name, existing, StringComparison.Ordinal))
            {
                Members.Add((name, member.Get, member.Set, member.Call, member.Result));
                return this;
            }
        }

        throw new InvalidOperationException($"No member '{existing}' on '{Name}' to alias as '{name}'.");
    }
}

/// <summary>
/// The machinery that turns the host objects of SPEC-05 §4.2 and the value types of §4.3 into
/// Lua values: userdata layout, object handles, metatables, member dispatch, argument checking
/// and error raising.
/// </summary>
/// <remarks>
/// <para>
/// A host object is a full userdata of twelve bytes: the type identity, a <b>handle</b> that
/// names the C# object, and a member selector used by the descriptor types (a bound vector, a
/// layer of the settings, a player of the pads…). The same C# object always yields the same
/// userdata, because a weak-valued cache table maps the handle to it, which is what gives the
/// identity of REQ-LUA-043 and lets a script use a host object as a table key (REQ-LUA-045).
/// When the userdata dies, <c>__gc</c> releases the handle, so a game that walks hundreds of
/// frame instances per frame does not grow the handle table (REQ-LUA-111).
/// </para>
/// <para>
/// A value type is a full userdata that holds the value itself, so it is a copy, as REQ-LUA-044
/// requires; its <c>__eq</c> and its arithmetic are the ones §4.3 defines.
/// </para>
/// <para>
/// Member dispatch costs one raw table lookup: each type owns a Lua table that maps a member
/// name either to a bound function or to the index of a getter in <see cref="Getters"/>. The
/// Lua string key is never marshalled to a C# string on the hot path, which is what keeps a
/// member access allocation-free.
/// </para>
/// </remarks>
internal sealed unsafe class LuaBinder
{
    private static int _nextRuntimeId;

    private readonly HostType?[] _types = new HostType?[(int)HostTypeId.Count];
    private readonly List<object?> _objects = [null];
    private readonly List<int> _references = [0];
    private readonly Stack<int> _freeHandles = new();
    private readonly ConditionalWeakTable<object, object> _handleOf = new();
    private readonly List<LuaFunction> _keepAlive = [];
    private readonly int _runtimeId = Interlocked.Increment(ref _nextRuntimeId);

    private int _cacheRef;
    private bool _sealed;

    /// <summary>Creates a binder for <paramref name="state"/>.</summary>
    public LuaBinder(Lua state)
    {
        ArgumentNullException.ThrowIfNull(state);
        State = state;
    }

    /// <summary>The state this binder installs itself in.</summary>
    public Lua State { get; }

    /// <summary>The property getters, indexed by the ids stored in the name tables.</summary>
    public List<LuaGet> Getters { get; } = [];

    /// <summary>The property setters, indexed by the same ids; an entry may be null.</summary>
    public List<LuaSet?> Setters { get; } = [];

    /// <summary>Declares a host type. Must happen before <see cref="Install"/>.</summary>
    public HostType DeclareType(HostTypeId id, string name, bool reference = true)
    {
        if (_sealed)
        {
            throw new InvalidOperationException("The host types are already installed.");
        }

        HostType type = new(id, name, reference);
        _types[(int)id] = type;
        return type;
    }

    /// <summary>The description of <paramref name="id"/>.</summary>
    public HostType Type(HostTypeId id) => _types[(int)id]
        ?? throw new InvalidOperationException($"Host type {id} was not declared.");

    /// <summary>Every declared type, for the API description.</summary>
    public IEnumerable<HostType> DeclaredTypes => _types.Where(static type => type is not null)!;

    /// <summary>
    /// Builds the metatables and the member tables of every declared type and installs the
    /// shared metamethods.
    /// </summary>
    public void Install()
    {
        _sealed = true;

        // The handle → userdata cache. Weak values, so a userdata nobody holds any more is
        // collected and its __gc releases the handle.
        State.NewTable();
        State.NewTable();
        State.PushString("__mode");
        State.PushString("v");
        State.RawSet(-3);
        State.SetMetaTable(-2);
        _cacheRef = State.Ref(LuaRegistry.Index);

        foreach (HostType? type in _types)
        {
            if (type is not null)
            {
                InstallType(type);
            }
        }
    }

    /// <summary>
    /// The key of a cached userdata. One C# object may be visible under several Lua types — a
    /// pattern instance is both an <c>Instance</c> and the owner of its <c>slot</c>, a shape is
    /// both a <c>Shape</c> and its own frame pattern — so the type and the member selector are
    /// part of the key, not just the handle.
    /// </summary>
    private static long CacheKey(int handle, HostTypeId id, int member)
        => ((long)handle << 10) | ((long)(int)id << 3) | (uint)(member & 7);

    /// <summary>Pushes a fresh userdata of <paramref name="size"/> bytes with its metatable.</summary>
    private byte* NewBox(Lua state, HostTypeId id, int size)
    {
        byte* block = (byte*)state.NewUserData(size);
        *(int*)block = (int)id;
        state.RawGetInteger(LuaRegistry.Index, Type(id).MetaRef);
        state.SetMetaTable(-2);
        return block;
    }

    #region handles

    /// <summary>The handle of <paramref name="value"/>, allocating one on first use.</summary>
    public int HandleOf(object value)
    {
        switch (value)
        {
            case PatternInstance instance:
                if (instance.ScriptHandle.Owner == _runtimeId)
                {
                    return instance.ScriptHandle.Handle;
                }

                int freshInstance = Allocate(value);
                instance.ScriptHandle = new ScriptHandleSlot { Owner = _runtimeId, Handle = freshInstance };
                return freshInstance;

            case FrameShape shape:
                if (shape.ScriptHandle.Owner == _runtimeId)
                {
                    return shape.ScriptHandle.Handle;
                }

                int freshShape = Allocate(value);
                shape.ScriptHandle = new ScriptHandleSlot { Owner = _runtimeId, Handle = freshShape };
                return freshShape;

            case EnginePattern pattern:
                if (pattern.ScriptHandle.Owner == _runtimeId)
                {
                    return pattern.ScriptHandle.Handle;
                }

                int freshPattern = Allocate(value);
                pattern.ScriptHandle = new ScriptHandleSlot { Owner = _runtimeId, Handle = freshPattern };
                return freshPattern;

            default:
                if (_handleOf.TryGetValue(value, out object? boxed))
                {
                    return (int)boxed;
                }

                int handle = Allocate(value);
                _handleOf.Add(value, handle);
                return handle;
        }
    }

    /// <summary>The object a handle names, or null when the handle has been released.</summary>
    public object? Resolve(int handle)
        => (uint)handle < (uint)_objects.Count ? _objects[handle] : null;

    /// <summary>How many handles are in use; the allocation check of REQ-LUA-111 reads it.</summary>
    public int LiveHandleCount => _objects.Count - 1 - _freeHandles.Count;

    /// <summary>
    /// The handle of <paramref name="value"/>, counting one more userdata that names it. Every
    /// userdata created for a handle is matched by exactly one <c>__gc</c>, so the handle is
    /// released when the last of them dies.
    /// </summary>
    private int Acquire(object value)
    {
        int handle = HandleOf(value);
        _references[handle]++;
        return handle;
    }

    private int Allocate(object value)
    {
        if (_freeHandles.Count > 0)
        {
            int reused = _freeHandles.Pop();
            _objects[reused] = value;
            _references[reused] = 0;
            return reused;
        }

        _objects.Add(value);
        _references.Add(0);
        return _objects.Count - 1;
    }

    private void Release(int handle)
    {
        if ((uint)handle >= (uint)_objects.Count)
        {
            return;
        }

        object? value = _objects[handle];
        switch (value)
        {
            case PatternInstance instance when instance.ScriptHandle.Owner == _runtimeId
                && instance.ScriptHandle.Handle == handle:
                instance.ScriptHandle = default;
                break;

            case FrameShape shape when shape.ScriptHandle.Owner == _runtimeId
                && shape.ScriptHandle.Handle == handle:
                shape.ScriptHandle = default;
                break;

            case EnginePattern pattern when pattern.ScriptHandle.Owner == _runtimeId
                && pattern.ScriptHandle.Handle == handle:
                pattern.ScriptHandle = default;
                break;

            case not null:
                _handleOf.Remove(value);
                break;
        }

        _objects[handle] = null;
        _references[handle] = 0;
        _freeHandles.Push(handle);
    }

    #endregion

    #region pushing

    /// <summary>
    /// Pushes the host object <paramref name="value"/>, or nil when it is null. The same object
    /// and member always give the same Lua value (REQ-LUA-043).
    /// </summary>
    /// <param name="state">The state.</param>
    /// <param name="id">The Lua type.</param>
    /// <param name="value">The C# object, or null for nil.</param>
    /// <param name="member">
    /// Which view of the object: 0 for the object itself, 1 for the frame pattern of a shape.
    /// Two views of one object are two distinct Lua values, each cached on its own.
    /// </param>
    public void PushObject(Lua state, HostTypeId id, object? value, int member = 0)
    {
        if (value is null)
        {
            state.PushNil();
            return;
        }

        int handle = HandleOf(value);
        long key = CacheKey(handle, id, member);
        state.RawGetInteger(LuaRegistry.Index, _cacheRef);
        state.RawGetInteger(-1, key);
        if (state.Type(-1) == LuaType.UserData)
        {
            state.Remove(-2);
            return;
        }

        state.Pop(1);
        int* box = (int*)NewBox(state, id, 12);
        box[1] = Acquire(value);
        box[2] = member;
        state.PushCopy(-1);
        state.RawSetInteger(-3, key);
        state.Remove(-2);
    }

    /// <summary>
    /// Pushes a descriptor userdata: a bound vector, a layer, a player, a shared list… It names
    /// an owner object and a member selector, is not cached, and allocates nothing managed.
    /// </summary>
    public void PushDescriptor(Lua state, HostTypeId id, object owner, int member)
    {
        int* box = (int*)NewBox(state, id, 12);
        box[1] = Acquire(owner);
        box[2] = member;
    }

    /// <summary>Pushes a number as a Lua <b>float</b>, as REQ-LUA-040 requires.</summary>
    public static void PushNumber(Lua state, double value) => state.PushNumber(value);

    /// <summary>Pushes a <c>Vector3</c> value (§4.3).</summary>
    public void PushVector3(Lua state, Vec3 value)
    {
        float* box = (float*)NewBox(state, HostTypeId.Vector3, 16);
        box[1] = value.X;
        box[2] = value.Y;
        box[3] = value.Z;
    }

    /// <summary>Pushes a <c>Vector2</c> value (§4.3).</summary>
    public void PushVector2(Lua state, Vec2 value)
    {
        float* box = (float*)NewBox(state, HostTypeId.Vector2, 12);
        box[1] = value.X;
        box[2] = value.Y;
    }

    /// <summary>Pushes a <c>Quaternion</c> value (§4.3).</summary>
    public void PushQuaternion(Lua state, System.Numerics.Quaternion value)
    {
        float* box = (float*)NewBox(state, HostTypeId.Quaternion, 20);
        box[1] = value.X;
        box[2] = value.Y;
        box[3] = value.Z;
        box[4] = value.W;
    }

    /// <summary>Pushes a <c>Color</c> value (§4.3).</summary>
    public void PushColor(Lua state, LuaColor value)
    {
        float* box = (float*)NewBox(state, HostTypeId.Color, 20);
        box[1] = value.R;
        box[2] = value.G;
        box[3] = value.B;
        box[4] = value.A;
    }

    /// <summary>Pushes an <c>IntVector2</c> value (§4.3).</summary>
    public void PushIntVector2(Lua state, Cell value)
    {
        int* box = (int*)NewBox(state, HostTypeId.IntVector2, 12);
        box[1] = value.X;
        box[2] = value.Y;
    }

    /// <summary>Pushes an <c>IntVector3</c> value (§4.3).</summary>
    public void PushIntVector3(Lua state, LuaIntVector3 value)
    {
        int* box = (int*)NewBox(state, HostTypeId.IntVector3, 16);
        box[1] = value.X;
        box[2] = value.Y;
        box[3] = value.Z;
    }

    /// <summary>Pushes a <c>Rect</c> value (§4.3).</summary>
    public void PushRect(Lua state, Rect value)
    {
        float* box = (float*)NewBox(state, HostTypeId.Rect, 20);
        box[1] = value.X;
        box[2] = value.Y;
        box[3] = value.Width;
        box[4] = value.Height;
    }

    /// <summary>Pushes an <c>XRState</c> value; every component is zero on desktop.</summary>
    public void PushXrState(Lua state, LuaXrState value)
    {
        float* box = (float*)NewBox(state, HostTypeId.XrState, 52);
        Write(box, 1, value.Position);
        Write(box, 4, value.Velocity);
        Write(box, 7, value.Acceleration);
        Write(box, 10, value.Rotation);
        return;

        static void Write(float* box, int at, Vec3 part)
        {
            box[at] = part.X;
            box[at + 1] = part.Y;
            box[at + 2] = part.Z;
        }
    }

    /// <summary>Reads an <c>XRState</c> value.</summary>
    public bool TryGetXrState(Lua state, int index, out LuaXrState value)
    {
        if (TypeIdAt(state, index) != HostTypeId.XrState)
        {
            value = default;
            return false;
        }

        float* box = (float*)state.ToUserData(index);
        value = new LuaXrState(
            new Vec3(box[1], box[2], box[3]),
            new Vec3(box[4], box[5], box[6]),
            new Vec3(box[7], box[8], box[9]),
            new Vec3(box[10], box[11], box[12]));
        return true;
    }

    #endregion

    #region reading

    /// <summary>The type identity of the value at <paramref name="index"/>, or None.</summary>
    public HostTypeId TypeIdAt(Lua state, int index)
    {
        if (state.Type(index) != LuaType.UserData)
        {
            return HostTypeId.None;
        }

        if ((long)state.RawLen(index) < 8L)
        {
            return HostTypeId.None;
        }

        int id = *(int*)state.ToUserData(index);
        return (uint)id < (uint)(int)HostTypeId.Count ? (HostTypeId)id : HostTypeId.None;
    }

    /// <summary>The handle stored in the descriptor or object at <paramref name="index"/>.</summary>
    public int HandleAt(Lua state, int index) => ((int*)state.ToUserData(index))[1];

    /// <summary>The member selector stored at <paramref name="index"/>.</summary>
    public int MemberAt(Lua state, int index) => ((int*)state.ToUserData(index))[2];

    /// <summary>The object a host userdata names, or null.</summary>
    public object? ObjectAt(Lua state, int index, HostTypeId expected)
        => TypeIdAt(state, index) == expected ? Resolve(HandleAt(state, index)) : null;

    /// <summary>
    /// The object the method's receiver names, or null when index 1 does not hold a userdata of
    /// <paramref name="expected"/> — which is what calling an instance method with <c>.</c>
    /// produces (REQ-LUA-081).
    /// </summary>
    public T? Receiver<T>(Lua state, HostTypeId expected)
        where T : class
        => ObjectAt(state, 1, expected) as T;

    /// <summary>The value at <paramref name="index"/> read as a <c>Vector3</c>.</summary>
    /// <remarks>
    /// A bound vector is accepted wherever a <c>Vector3</c> is expected, because the corpus
    /// passes one straight through (<c>shape:GetPos(shape.Pivot)</c>); it is read by copy, so
    /// the callee cannot write back through it.
    /// </remarks>
    public bool TryGetVector3(Lua state, int index, out Vec3 value)
    {
        HostTypeId id = TypeIdAt(state, index);
        if (id == HostTypeId.Vector3)
        {
            float* box = (float*)state.ToUserData(index);
            value = new Vec3(box[1], box[2], box[3]);
            return true;
        }

        if (id == HostTypeId.Vec3Ref)
        {
            value = ReadVec3Ref(state, index);
            return true;
        }

        if (id == HostTypeId.Vector2)
        {
            float* box = (float*)state.ToUserData(index);
            value = new Vec3(box[1], box[2], 0f);
            return true;
        }

        value = default;
        return false;
    }

    /// <summary>The value at <paramref name="index"/> read as a <c>Vector2</c>.</summary>
    public bool TryGetVector2(Lua state, int index, out Vec2 value)
    {
        HostTypeId id = TypeIdAt(state, index);
        if (id == HostTypeId.Vector2)
        {
            float* box = (float*)state.ToUserData(index);
            value = new Vec2(box[1], box[2]);
            return true;
        }

        if (id is HostTypeId.Vector3 or HostTypeId.Vec3Ref)
        {
            if (TryGetVector3(state, index, out Vec3 three))
            {
                value = new Vec2(three.X, three.Y);
                return true;
            }
        }

        if (id == HostTypeId.IntVector2)
        {
            int* box = (int*)state.ToUserData(index);
            value = new Vec2(box[1], box[2]);
            return true;
        }

        value = default;
        return false;
    }

    /// <summary>The value at <paramref name="index"/> read as a <c>Quaternion</c>.</summary>
    public bool TryGetQuaternion(Lua state, int index, out System.Numerics.Quaternion value)
    {
        if (TypeIdAt(state, index) != HostTypeId.Quaternion)
        {
            value = default;
            return false;
        }

        float* box = (float*)state.ToUserData(index);
        value = new System.Numerics.Quaternion(box[1], box[2], box[3], box[4]);
        return true;
    }

    /// <summary>The value at <paramref name="index"/> read as a <c>Color</c>.</summary>
    public bool TryGetColor(Lua state, int index, out LuaColor value)
    {
        if (TypeIdAt(state, index) != HostTypeId.Color)
        {
            value = default;
            return false;
        }

        float* box = (float*)state.ToUserData(index);
        value = new LuaColor(box[1], box[2], box[3], box[4]);
        return true;
    }

    /// <summary>The value at <paramref name="index"/> read as an <c>IntVector2</c>.</summary>
    public bool TryGetIntVector2(Lua state, int index, out Cell value)
    {
        HostTypeId id = TypeIdAt(state, index);
        if (id == HostTypeId.IntVector2)
        {
            int* box = (int*)state.ToUserData(index);
            value = new Cell(box[1], box[2]);
            return true;
        }

        if (id is HostTypeId.Vector2 or HostTypeId.Vector3 or HostTypeId.Vec3Ref
            && TryGetVector2(state, index, out Vec2 pair))
        {
            value = new Cell((int)pair.X, (int)pair.Y);
            return true;
        }

        value = default;
        return false;
    }

    /// <summary>The value at <paramref name="index"/> read as a <c>Rect</c>.</summary>
    public bool TryGetRect(Lua state, int index, out Rect value)
    {
        if (TypeIdAt(state, index) != HostTypeId.Rect)
        {
            value = default;
            return false;
        }

        float* box = (float*)state.ToUserData(index);
        value = new Rect(box[1], box[2], box[3], box[4]);
        return true;
    }

    #endregion

    #region argument checking (REQ-LUA-041)

    /// <summary>
    /// The number at <paramref name="index"/>. A string, even a numeric one, is refused
    /// (REQ-LUA-041).
    /// </summary>
    public double CheckNumber(Lua state, int index, string member)
    {
        if (state.Type(index) != LuaType.Number)
        {
            InvalidArguments(state, member);
        }

        return state.ToNumber(index);
    }

    /// <summary>The number at <paramref name="index"/>, truncated towards zero.</summary>
    public int CheckInteger(Lua state, int index, string member)
        => LuaNumberFormat.ToInt32(CheckNumber(state, index, member));

    /// <summary>The number at <paramref name="index"/>, or <paramref name="fallback"/> if absent.</summary>
    public double OptionalNumber(Lua state, int index, string member, double fallback)
    {
        if (state.IsNoneOrNil(index))
        {
            return fallback;
        }

        return CheckNumber(state, index, member);
    }

    /// <summary>The string at <paramref name="index"/>; a number is refused (REQ-LUA-041).</summary>
    public string CheckString(Lua state, int index, string member)
    {
        if (state.Type(index) != LuaType.String)
        {
            InvalidArguments(state, member);
        }

        return state.ToString(index) ?? string.Empty;
    }

    /// <summary>The string at <paramref name="index"/>, or the empty string if absent.</summary>
    public string OptionalString(Lua state, int index, string member, string fallback)
        => state.IsNoneOrNil(index) ? fallback : CheckString(state, index, member);

    /// <summary>The truth value at <paramref name="index"/> (REQ-LUA-041).</summary>
    public static bool CheckBoolean(Lua state, int index) => state.ToBoolean(index);

    /// <summary>The <c>Vector3</c> at <paramref name="index"/>, or an error (REQ-LUA-041).</summary>
    public Vec3 CheckVector3(Lua state, int index, string member)
    {
        if (!TryGetVector3(state, index, out Vec3 value))
        {
            InvalidArguments(state, member);
        }

        return value;
    }

    /// <summary>The <c>Rect</c> at <paramref name="index"/>, or an error.</summary>
    public Rect CheckRect(Lua state, int index, string member)
    {
        if (!TryGetRect(state, index, out Rect value))
        {
            InvalidArguments(state, member);
        }

        return value;
    }

    /// <summary>Raises <c>invalid arguments to &lt;member&gt;</c> (REQ-LUA-041).</summary>
    public int InvalidArguments(Lua state, string member)
        => Raise(state, $"invalid arguments to {member}");

    /// <summary>
    /// Raises a Lua error carrying the position of the calling script line, which is the form
    /// REQ-LUA-061 logs.
    /// </summary>
    public int Raise(Lua state, string message)
    {
        state.Where(1);
        state.PushString(message);
        state.Concat(2);
        return state.Error();
    }

    /// <summary>Checks a 0-based index against a count, raising a Lua error (REQ-LUA-045).</summary>
    public void CheckBounds(Lua state, int index, int count, string member)
    {
        if ((uint)index >= (uint)count)
        {
            Raise(state, $"index {index} is out of range for {member} (count {count})");
        }
    }

    #endregion

    #region bound vectors

    /// <summary>The owner of a bound-vector descriptor and the kind it designates.</summary>
    public (object? Owner, int Kind) RefTargetAt(Lua state, int index)
        => (Resolve(HandleAt(state, index)), MemberAt(state, index));

    /// <summary>Reads the three components a <c>Vec3R</c> designates.</summary>
    public Vec3 ReadVec3Ref(Lua state, int index)
    {
        (object? owner, int kind) = RefTargetAt(state, index);
        return Vec3RefAccess.Read(owner, (Vec3RefKind)kind);
    }

    /// <summary>Writes the three components a <c>Vec3R</c> designates.</summary>
    public void WriteVec3Ref(Lua state, int index, Vec3 value)
    {
        (object? owner, int kind) = RefTargetAt(state, index);
        Vec3RefAccess.Write(owner, (Vec3RefKind)kind, value);
    }

    #endregion

    #region installation

    private void InstallType(HostType type)
    {
        Lua state = State;

        // name → getter id or bound function
        state.NewTable();
        int getTable = state.GetTop();

        // name → setter id, or false for a read-only member
        state.NewTable();
        int setTable = state.GetTop();

        foreach ((string name, LuaGet? get, LuaSet? set, LuaInvoke? call, _) in type.Members)
        {
            if (call is not null)
            {
                state.PushString(name);
                PushBoundFunction(state, call);
                state.RawSet(getTable);

                state.PushString(name);
                state.PushBoolean(false);
                state.RawSet(setTable);
                continue;
            }

            int id = Getters.Count;
            Getters.Add(get ?? throw new InvalidOperationException($"Member '{name}' has no getter."));
            Setters.Add(set);

            state.PushString(name);
            state.PushInteger(id);
            state.RawSet(getTable);

            state.PushString(name);
            if (set is null)
            {
                state.PushBoolean(false);
            }
            else
            {
                state.PushInteger(id);
            }

            state.RawSet(setTable);
        }

        type.SetRef = state.Ref(LuaRegistry.Index);
        type.GetRef = state.Ref(LuaRegistry.Index);

        state.NewTable();
        Metamethod(state, "__index", IndexDispatch);
        Metamethod(state, "__newindex", NewIndexDispatch);
        Metamethod(state, "__eq", EqualDispatch);
        if (type.Reference)
        {
            Metamethod(state, "__gc", CollectDispatch);
        }

        if (type.Call is not null)
        {
            Metamethod(state, "__call", type.Call);
        }

        if (type.Add is not null)
        {
            Metamethod(state, "__add", type.Add);
        }

        if (type.Subtract is not null)
        {
            Metamethod(state, "__sub", type.Subtract);
        }

        if (type.Multiply is not null)
        {
            Metamethod(state, "__mul", type.Multiply);
        }

        if (type.Divide is not null)
        {
            Metamethod(state, "__div", type.Divide);
        }

        if (type.Negate is not null)
        {
            Metamethod(state, "__unm", type.Negate);
        }

        if (type.Text is not null)
        {
            Metamethod(state, "__tostring", type.Text);
        }

        state.PushString("__metatable");
        state.PushString(type.Name);
        state.RawSet(-3);

        type.MetaRef = state.Ref(LuaRegistry.Index);
    }

    private void Metamethod(Lua state, string name, LuaInvoke body)
    {
        state.PushString(name);
        PushBoundFunction(state, body);
        state.RawSet(-3);
    }

    /// <summary>
    /// Pushes <paramref name="body"/> as a Lua C function. The delegate is kept alive for the
    /// life of the binder, because Lua holds only the unmanaged thunk.
    /// </summary>
    public void PushBoundFunction(Lua state, LuaInvoke body)
    {
        LuaFunction thunk = pointer => body(this, Lua.FromIntPtr(pointer)!);
        _keepAlive.Add(thunk);
        state.PushCFunction(thunk);
    }

    private static int IndexDispatch(LuaBinder binder, Lua state)
    {
        HostTypeId id = binder.TypeIdAt(state, 1);
        if (id == HostTypeId.None)
        {
            state.PushNil();
            return 1;
        }

        HostType type = binder.Type(id);
        state.RawGetInteger(LuaRegistry.Index, type.GetRef);
        state.PushCopy(2);
        state.RawGet(-2);

        switch (state.Type(-1))
        {
            case LuaType.Function:
                state.Remove(-2);
                return 1;

            case LuaType.Number:
                int member = (int)state.ToInteger(-1);
                state.Pop(2);
                return binder.Getters[member](binder, state);

            default:
                state.Pop(2);
                break;
        }

        if (type.Indexer is { } indexer && state.Type(2) == LuaType.Number)
        {
            return indexer(binder, state);
        }

        // An unknown member reads as nil, without an error (REQ-LUA-042).
        state.PushNil();
        return 1;
    }

    private static int NewIndexDispatch(LuaBinder binder, Lua state)
    {
        HostTypeId id = binder.TypeIdAt(state, 1);
        if (id == HostTypeId.None)
        {
            return binder.Raise(state, "cannot set a field on this value");
        }

        HostType type = binder.Type(id);
        state.RawGetInteger(LuaRegistry.Index, type.SetRef);
        state.PushCopy(2);
        state.RawGet(-2);

        LuaType found = state.Type(-1);
        if (found == LuaType.Number)
        {
            int member = (int)state.ToInteger(-1);
            state.Pop(2);
            LuaSet? setter = binder.Setters[member];
            if (setter is null)
            {
                return binder.ReadOnly(state);
            }

            setter(binder, state);
            return 0;
        }

        state.Pop(2);
        if (found == LuaType.Boolean)
        {
            return binder.ReadOnly(state);
        }

        if (type.IndexSetter is { } setIndex && state.Type(2) == LuaType.Number)
        {
            setIndex(binder, state);
            return 0;
        }

        // Writing a name that is not a member is an error with this exact wording
        // (REQ-LUA-042, TV-LUA-13).
        string name = state.ToString(2, false) ?? "?";
        return binder.Raise(state, $"cannot set {name}, no such field");
    }

    private int ReadOnly(Lua state)
    {
        string name = state.ToString(2, false) ?? "?";
        return Raise(state, $"cannot set {name}, it is read only");
    }

    private static int EqualDispatch(LuaBinder binder, Lua state)
    {
        HostTypeId left = binder.TypeIdAt(state, 1);
        HostTypeId right = binder.TypeIdAt(state, 2);
        if (left == HostTypeId.None || left != right)
        {
            state.PushBoolean(false);
            return 1;
        }

        HostType type = binder.Type(left);
        if (type.Equal is { } equal)
        {
            state.PushBoolean(equal(binder, state));
            return 1;
        }

        if (!type.Reference)
        {
            state.PushBoolean(state.RawEqual(1, 2));
            return 1;
        }

        // Two userdata name the same host object when they resolve to the same C# object and
        // select the same member (REQ-LUA-043).
        object? a = binder.Resolve(binder.HandleAt(state, 1));
        object? b = binder.Resolve(binder.HandleAt(state, 2));
        bool same = ReferenceEquals(a, b)
            && a is not null
            && binder.MemberAt(state, 1) == binder.MemberAt(state, 2);
        state.PushBoolean(same);
        return 1;
    }

    private static int CollectDispatch(LuaBinder binder, Lua state)
    {
        // Must never raise: this runs inside Lua's collector.
        try
        {
            HostTypeId id = binder.TypeIdAt(state, 1);
            int handle = binder.HandleAt(state, 1);
            int member = binder.MemberAt(state, 1);
            if ((uint)handle >= (uint)binder._references.Count)
            {
                return 0;
            }

            // The weak cache is cleared before the finalizers run, so a push made in between
            // may already have put a replacement there. Only drop the entry when it still
            // points at this very userdata.
            long key = CacheKey(handle, id, member);
            state.RawGetInteger(LuaRegistry.Index, binder._cacheRef);
            state.RawGetInteger(-1, key);
            bool mine = state.Type(-1) != LuaType.UserData || state.RawEqual(-1, 1);
            state.Pop(1);
            if (mine)
            {
                state.PushNil();
                state.RawSetInteger(-2, key);
            }

            state.Pop(1);

            if (--binder._references[handle] <= 0)
            {
                binder.Release(handle);
            }
        }
        catch (Exception)
        {
            // A failure here would corrupt the collector; a leaked handle is the only harm.
        }

        return 0;
    }

    #endregion
}

/// <summary>
/// Reads and writes the vectors a <c>Vec3R</c> designates (SPEC-05 §4.3): they are live views,
/// so writing one component writes the owning object (REQ-LUA-044).
/// </summary>
internal static class Vec3RefAccess
{
    /// <summary>Reads the designated vector, or zero when the owner is gone (REQ-LUA-028).</summary>
    public static Vec3 Read(object? owner, Vec3RefKind kind) => kind switch
    {
        Vec3RefKind.InstanceOffset => owner is PatternInstance i ? i.Offset : default,
        Vec3RefKind.InstanceScale => owner is PatternInstance i ? i.Scale : default,
        Vec3RefKind.InstanceRotation => owner is PatternInstance i ? i.Rotation : default,
        Vec3RefKind.InstancePivot => owner is PatternInstance i ? i.Pivot : default,
        Vec3RefKind.InstanceDeformAmplitude => owner is PatternInstance i ? i.DeformAmplitude : default,
        Vec3RefKind.InstanceDeformPivot => owner is PatternInstance i ? i.DeformPivot : default,
        Vec3RefKind.CameraRotation => owner is ScriptCamera c ? c.Angles : default,
        Vec3RefKind.CameraTarget => owner is ScriptCamera c ? c.LookAt : default,
        Vec3RefKind.ZapperOffset => owner is ScriptInput p ? p.ZapperOffset : default,
        Vec3RefKind.BackdropSize => owner is ScriptBackdrop b ? b.Size : default,
        Vec3RefKind.BackdropOffset => owner is ScriptBackdrop b ? b.Offset : default,
        _ => default,
    };

    /// <summary>Writes the designated vector; a write to a dead object does nothing.</summary>
    public static void Write(object? owner, Vec3RefKind kind, Vec3 value)
    {
        switch (kind)
        {
            case Vec3RefKind.InstanceOffset when owner is PatternInstance i:
                i.Offset = value;
                break;
            case Vec3RefKind.InstanceScale when owner is PatternInstance i:
                i.Scale = value;
                break;
            case Vec3RefKind.InstanceRotation when owner is PatternInstance i:
                i.Rotation = value;
                break;
            case Vec3RefKind.InstancePivot when owner is PatternInstance i:
                i.Pivot = value;
                break;
            case Vec3RefKind.InstanceDeformAmplitude when owner is PatternInstance i:
                i.DeformAmplitude = value;
                break;
            case Vec3RefKind.InstanceDeformPivot when owner is PatternInstance i:
                i.DeformPivot = value;
                break;
            case Vec3RefKind.CameraRotation when owner is ScriptCamera c:
                c.Angles = value;
                break;
            case Vec3RefKind.CameraTarget when owner is ScriptCamera c:
                c.LookAt = value;
                break;
            case Vec3RefKind.ZapperOffset when owner is ScriptInput p:
                p.ZapperOffset = value;
                break;
            case Vec3RefKind.BackdropSize when owner is ScriptBackdrop b:
                b.Size = value;
                break;
            case Vec3RefKind.BackdropOffset when owner is ScriptBackdrop b:
                b.Offset = value;
                break;
            default:
                break;
        }
    }
}
