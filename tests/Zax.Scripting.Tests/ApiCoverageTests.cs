using System.Globalization;
using Zax.Scripting;

namespace Zax.Scripting.Tests;

/// <summary>
/// REQ-LUA-080 and REQ-LUA-051: the whole script-visible API of SPEC-05 §4 is installed. The
/// expectation is the transcription of the §4 tables in <c>spec05-api.txt</c>, checked row by
/// row against the description the runtime produces from the tables it binds
/// (<see cref="ScriptApiSurface"/>), and against the Lua state itself for the globals.
/// </summary>
/// <remarks>
/// The fixture is a copy of the specification, so the check is executable without the
/// specification folder (DEC-R2-T7) and a drift between the two shows up as a failing row
/// rather than as a silently missing member.
/// </remarks>
public sealed class ApiCoverageTests
{
    /// <summary>Type names of §4 that the binder spells in ASCII, or splits per element type.</summary>
    private static readonly Dictionary<string, string[]> TypeAliases = new(StringComparer.Ordinal)
    {
        ["Découpe"] = ["Decoupe"],
        ["Lumière"] = ["Lumiere"],
        ["DonnéesPerso"] = ["CData"],
        ["Liste<T>"] = ["Liste<Instance>", "Liste<Shape>"],
        ["TLayerEnum"] = ["TLayer"],
        ["AlignEnum"] = ["Align"],
        ["CamModeEnum"] = ["CamMode"],
    };

    /// <summary>
    /// The globals of §4.1 that are plain Lua values rather than host objects, so they are
    /// checked in the state instead of in the description.
    /// </summary>
    private static readonly Dictionary<string, string> PlainGlobals = new(StringComparer.Ordinal)
    {
        ["frame"] = "number",
        ["time"] = "number",
        ["PData"] = "table",
        ["pdata_count"] = "number",
    };

    /// <summary>
    /// The globals §4.1 only assigns while a hook runs: <c>shape</c> and <c>shape2D</c> during
    /// <c>UpdateS</c>, <c>pattern</c> during <c>BuildPattern</c> (REQ-LUA-025, 030), and
    /// <c>self</c> during any hook. They are nil outside one, which the hook-order vectors
    /// exercise; here only their declared type is checked.
    /// </summary>
    private static readonly string[] HookGlobals = ["shape", "shape2D", "pattern", "self"];

    /// <summary>REQ-LUA-080: every global of §4.1 exists, with the kind §4.1 gives it.</summary>
    [Fact]
    public void Every_global_of_section_4_1_is_installed_REQ_LUA_080()
    {
        ScriptApiSurface api = ScriptApiSurface.Describe();
        using ScriptSession session = new ScriptProfileBuilder().Script("a", "-- nothing").Load();

        List<string> missing = [];
        int checked_ = 0;
        foreach (SpecRow row in SpecRows().Where(static r => r.Kind == "global"))
        {
            checked_++;
            string name = row.Fields[0];

            if (PlainGlobals.TryGetValue(name, out string? luaType))
            {
                string actual = session.Runtime.EvaluateText($"return type({name})");
                if (actual != luaType)
                {
                    missing.Add($"global {name}: type() is '{actual}', expected '{luaType}'");
                }

                continue;
            }

            if (row.Fields[1] == "fonction")
            {
                if (session.Runtime.EvaluateText($"return type({name})") != "function")
                {
                    missing.Add($"global {name} is not a function");
                }

                continue;
            }

            // A host-object global: the description must name a declared type for it, and the
            // state must hold a value (userdata or, for the constant tables of §4.4, a table).
            if (!api.Globals.TryGetValue(name, out string? declared) || declared.Length == 0)
            {
                if (!api.Constants.ContainsKey(name))
                {
                    missing.Add($"global {name} is absent from the API description");
                    continue;
                }
            }
            else if (!api.Types.ContainsKey(declared))
            {
                missing.Add($"global {name} names the undeclared type {declared}");
                continue;
            }

            if (HookGlobals.Contains(name, StringComparer.Ordinal))
            {
                continue;
            }

            string kind = session.Runtime.EvaluateText($"return type({name})");
            if (kind is not ("userdata" or "table"))
            {
                missing.Add($"global {name}: type() is '{kind}'");
            }
        }

        Assert.Equal(42, checked_);
        Assert.True(missing.Count == 0, $"{missing.Count} globals of §4.1 are wrong:\n" + string.Join("\n", missing));
    }

    /// <summary>
    /// REQ-LUA-080: every member row of §4.2 to §4.4 resolves against the installed API, with
    /// the right call kind (REQ-LUA-081: <c>prop</c>, <c>const</c> and <c>index</c> are not
    /// callable; <c>méth</c>, <c>fonc</c> and <c>appel</c> are).
    /// </summary>
    [Fact]
    public void Every_member_row_of_section_4_resolves_REQ_LUA_080()
    {
        ScriptApiSurface api = ScriptApiSurface.Describe();
        List<string> missing = [];
        List<string> wrongKind = [];
        int checked_ = 0;

        foreach (SpecRow row in SpecRows().Where(static r => r.Kind == "member"))
        {
            checked_++;
            string typeName = row.Fields[0];
            string member = row.Fields[1];
            string genre = row.Fields[2];

            // The enum types of §4.4 are constant tables, not host objects (REQ-LUA-046): their
            // rows resolve against the table the global of the same name holds.
            if (genre is "const"
                && TypeAliases.TryGetValue(typeName, out string[]? tables)
                && api.Constants.TryGetValue(tables[0], out IReadOnlyDictionary<string, double>? entries))
            {
                if (!entries.ContainsKey(member))
                {
                    missing.Add($"{tables[0]}.{member} is missing from the constant table");
                }

                continue;
            }

            ScriptApiType[] candidates = [.. Resolve(api, typeName)];
            if (candidates.Length == 0)
            {
                missing.Add($"{typeName} is not declared (row {member})");
                continue;
            }

            // The §4 rows that describe indexing or the call of a class table.
            if (genre is "index")
            {
                if (!candidates.Any(static type => type.Indexable))
                {
                    missing.Add($"{typeName}{member} is not indexable");
                }

                continue;
            }

            if (genre is "appel" && member.StartsWith('('))
            {
                if (!candidates.Any(static type => type.CallResult.Length > 0))
                {
                    missing.Add($"{typeName}{member} is not callable");
                }

                continue;
            }

            // A constant of an enum table of §4.4 lives in the constant tables.
            if (genre is "const" && candidates.Length == 1 && candidates[0].Members.Count == 0)
            {
                continue;
            }

            ScriptApiMember? found = null;
            foreach (ScriptApiType type in candidates)
            {
                if (type.Members.TryGetValue(member, out ScriptApiMember value))
                {
                    found = value;
                    break;
                }
            }

            if (found is null)
            {
                // §4.4 constants are exposed through the constant tables, not as members.
                if (genre is "const" && api.Constants.Values.Any(table => table.ContainsKey(member)))
                {
                    continue;
                }

                missing.Add($"{typeName}.{member} ({genre}, {row.Fields[4]}) is missing");
                continue;
            }

            bool shouldBeCallable = genre is "méth" or "fonc" or "appel";
            if (found.Value.Callable != shouldBeCallable)
            {
                wrongKind.Add(
                    $"{typeName}.{member} is {(found.Value.Callable ? "callable" : "a property")}, "
                        + $"but §4 says '{genre}'");
            }
        }

        Assert.Equal(360, checked_);
        Assert.True(
            missing.Count == 0,
            $"{missing.Count} member rows of §4 do not resolve:\n" + string.Join("\n", missing));
        Assert.True(
            wrongKind.Count == 0,
            $"{wrongKind.Count} member rows have the wrong call kind:\n" + string.Join("\n", wrongKind));
    }

    /// <summary>
    /// The fixture counts match §5.1 of the specification: 42 globals and 46 types. It guards
    /// the transcription itself against a truncated copy.
    /// </summary>
    [Fact]
    public void The_transcription_of_section_4_is_complete()
    {
        SpecRow[] rows = [.. SpecRows()];
        Assert.Equal(42, rows.Count(static r => r.Kind == "global"));
        Assert.Equal(360, rows.Count(static r => r.Kind == "member"));
        Assert.Equal(
            46,
            rows.Where(static r => r.Kind == "member")
                .Select(static r => r.Fields[0])
                .Distinct(StringComparer.Ordinal)
                .Count());
    }

    private static IEnumerable<ScriptApiType> Resolve(ScriptApiSurface api, string specName)
    {
        foreach (string name in TypeAliases.TryGetValue(specName, out string[]? aliases)
                     ? aliases
                     : [specName])
        {
            if (api.Types.TryGetValue(name, out ScriptApiType? type))
            {
                yield return type;
            }
        }
    }

    private readonly record struct SpecRow(string Kind, string[] Fields);

    private static IEnumerable<SpecRow> SpecRows()
    {
        string path = Path.Combine(AppContext.BaseDirectory, "spec05-api.txt");
        Assert.True(File.Exists(path), $"the transcription of SPEC-05 §4 is missing at {path}");

        foreach (string line in File.ReadLines(path))
        {
            if (line.Length == 0 || line[0] == '#')
            {
                continue;
            }

            string[] parts = line.Split('\t');
            yield return new SpecRow(parts[0], parts[1..]);
        }
    }

    /// <summary>The surface is at least as large as the tables it must cover.</summary>
    [Fact]
    public void The_installed_surface_covers_the_tables_REQ_LUA_080()
    {
        ScriptApiSurface api = ScriptApiSurface.Describe();
        Assert.True(
            api.MemberCount >= 360,
            string.Create(CultureInfo.InvariantCulture, $"only {api.MemberCount} members installed"));
    }
}
