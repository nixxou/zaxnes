using Zax.Profiles;

namespace Zax.Scripting;

/// <summary>
/// One member of a script-visible type (SPEC-05 §4.2, §4.3).
/// </summary>
/// <param name="Name">Member name, exactly as the specification spells it.</param>
/// <param name="Writable">Whether a script may assign it (<c>LE</c> in §4).</param>
/// <param name="Callable">Whether it is a method or a table function rather than a property.</param>
/// <param name="ResultType">
/// The name of the script-visible type the member yields, or the empty string when it yields a
/// number, a boolean, a string or nil.
/// </param>
public readonly record struct ScriptApiMember(
    string Name,
    bool Writable,
    bool Callable,
    string ResultType);

/// <summary>One script-visible type and its members.</summary>
public sealed class ScriptApiType
{
    internal ScriptApiType(
        string name,
        IReadOnlyDictionary<string, ScriptApiMember> members,
        string elementType,
        string callResult,
        bool indexed,
        bool indexWritable)
    {
        Name = name;
        Members = members;
        ElementType = elementType;
        CallResult = callResult;
        Indexed = indexed;
        IndexWritable = indexWritable;
    }

    /// <summary>Type name, as §4.2 and §4.3 name it.</summary>
    public string Name { get; }

    /// <summary>The members, by name.</summary>
    public IReadOnlyDictionary<string, ScriptApiMember> Members { get; }

    /// <summary>
    /// The type <c>obj[i]</c> yields, or the empty string when the type is not indexable
    /// (REQ-LUA-045).
    /// </summary>
    public string ElementType { get; }

    /// <summary>
    /// The type <c>obj(...)</c> yields, or the empty string when the type is not callable
    /// (the constructors of §4.3).
    /// </summary>
    public string CallResult { get; }

    /// <summary>
    /// True when the type answers <c>obj[i]</c> (REQ-LUA-045), whatever the element type: the
    /// <c>[i]</c> rows of §4 include types whose element is a plain number, such as
    /// <c>DonnéesPerso</c> and the bound vectors.
    /// </summary>
    public bool Indexed { get; }

    /// <summary>True when <c>obj[i] = v</c> is allowed.</summary>
    public bool IndexWritable { get; }

    /// <summary>True when the type is indexable by a number.</summary>
    public bool Indexable => Indexed || ElementType.Length > 0
        || Name.StartsWith("Liste", StringComparison.Ordinal);
}

/// <summary>
/// A description of the whole script-visible API: the globals of SPEC-05 §4.1, the types of
/// §4.2 and §4.3 with their members, and the constant tables of §4.4.
/// </summary>
/// <remarks>
/// It is produced from the very tables the runtime installs, so it cannot drift from what the
/// scripts actually see. That is what makes the static check of REQ-LUA-051 — resolving every
/// access chain of the 579 built-in scripts — executable by the clean side (DEC-R2-T7).
/// </remarks>
public sealed class ScriptApiSurface
{
    private ScriptApiSurface(
        IReadOnlyDictionary<string, string> globals,
        IReadOnlyDictionary<string, ScriptApiType> types,
        IReadOnlyDictionary<string, IReadOnlyDictionary<string, double>> constants,
        IReadOnlyList<string> functionGlobals)
    {
        Globals = globals;
        Types = types;
        Constants = constants;
        FunctionGlobals = functionGlobals;
    }

    /// <summary>Global name → the name of its type, for the host-object globals of §4.1.</summary>
    public IReadOnlyDictionary<string, string> Globals { get; }

    /// <summary>Type name → its description.</summary>
    public IReadOnlyDictionary<string, ScriptApiType> Types { get; }

    /// <summary>The constant tables of §4.4: table name → entry name → value.</summary>
    public IReadOnlyDictionary<string, IReadOnlyDictionary<string, double>> Constants { get; }

    /// <summary>The globals of §4.1 that are plain functions.</summary>
    public IReadOnlyList<string> FunctionGlobals { get; }

    /// <summary>How many members the description holds, all types together.</summary>
    public int MemberCount
    {
        get
        {
            int total = 0;
            foreach (ScriptApiType type in Types.Values)
            {
                total += type.Members.Count;
            }

            return total;
        }
    }

    /// <summary>
    /// Describes the API by building a runtime on an empty profile and reading back the tables
    /// it installed.
    /// </summary>
    public static ScriptApiSurface Describe()
    {
        ConversionEngineHolder holder = new();
        using ProfileScriptRuntime runtime = ProfileScriptRuntime.Create(
            GameProfile.CreateEmpty(),
            new ScriptEnvironment { Engine = holder.Engine });
        return runtime.DescribeApi();
    }

    internal static ScriptApiSurface From(LuaBinder binder)
    {
        Dictionary<string, ScriptApiType> types = new(StringComparer.Ordinal);
        Dictionary<HostTypeId, string> names = [];
        foreach (HostType type in binder.DeclaredTypes)
        {
            names[type.Id] = type.Name;
        }

        foreach (HostType type in binder.DeclaredTypes)
        {
            Dictionary<string, ScriptApiMember> members = new(StringComparer.Ordinal);
            foreach ((string name, _, LuaSet? set, LuaInvoke? call, HostTypeId result) in type.Members)
            {
                members[name] = new ScriptApiMember(
                    name,
                    set is not null,
                    call is not null,
                    NameOf(result));
            }

            types[type.Name] = new ScriptApiType(
                type.Name,
                members,
                NameOf(type.ElementType),
                NameOf(type.CallResult),
                type.Indexer is not null,
                type.IndexSetter is not null);
        }

        Dictionary<string, string> globals = new(StringComparer.Ordinal);
        foreach ((string name, HostTypeId id) in LuaApi.GlobalObjects)
        {
            globals[name] = NameOf(id);
        }

        Dictionary<string, IReadOnlyDictionary<string, double>> constants = new(StringComparer.Ordinal);
        foreach ((string name, (string Key, int Value)[] entries) in LuaApi.ConstantTables)
        {
            Dictionary<string, double> table = new(StringComparer.Ordinal);
            foreach ((string key, int value) in entries)
            {
                table[key] = value;
            }

            constants[name] = table;
        }

        return new ScriptApiSurface(globals, types, constants, FunctionGlobalNames);

        string NameOf(HostTypeId id)
            => id != HostTypeId.None && names.TryGetValue(id, out string? found) ? found : string.Empty;
    }

    /// <summary>The globals of §4.1 that are functions rather than objects.</summary>
    private static readonly string[] FunctionGlobalNames =
        ["print", "clear", "GetPData", "SetPData", "RenewId"];

    /// <summary>Holds an engine alive while the description is built.</summary>
    private sealed class ConversionEngineHolder
    {
        public Engine3D.ConversionEngine Engine { get; } = new();
    }
}
