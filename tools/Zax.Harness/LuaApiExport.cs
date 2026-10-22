using System.Globalization;
using Zax.Scripting;

namespace Zax.Harness;

/// <summary>
/// The <c>lua-api</c> command: dumps the script-visible API the Lua runtime installs, so that
/// the surface can be compared line by line with the tables of SPEC-05 §4.
/// </summary>
/// <remarks>
/// The dump is read back from the very tables <see cref="ProfileScriptRuntime"/> binds
/// (<see cref="ScriptApiSurface"/>), so it cannot drift from what a script actually sees. That
/// is what makes the coverage of REQ-LUA-080 and REQ-LUA-051 checkable from the clean side
/// (DEC-R2-T7).
/// </remarks>
internal static class LuaApiExport
{
    /// <summary>Help text of the command.</summary>
    public const string Usage = """
        lua-api [--out <file>]
          Dumps the script-visible API of SPEC-05 §4: the globals of §4.1, then one line per
          member of each type of §4.2 to §4.4. Writes to stdout without --out.
          Line format: 'global <name> <type>', 'const <table>.<name> <value>',
          'member <type>.<name> <genre> <access> <result-type>'.
        """;

    /// <summary>Runs the command.</summary>
    public static int Run(string[] args)
    {
        string? output = null;
        for (int i = 0; i < args.Length; i++)
        {
            if (args[i] is "--out" && i + 1 < args.Length)
            {
                output = args[++i];
                continue;
            }

            Console.Error.WriteLine($"unknown argument: {args[i]}");
            Console.Error.WriteLine();
            Console.Error.WriteLine(Usage);
            return 2;
        }

        ScriptApiSurface surface = ScriptApiSurface.Describe();
        List<string> lines = [];

        foreach ((string name, string type) in surface.Globals.OrderBy(static g => g.Key, StringComparer.Ordinal))
        {
            lines.Add($"global {name} {type}");
        }

        foreach (string name in surface.FunctionGlobals.OrderBy(static g => g, StringComparer.Ordinal))
        {
            lines.Add($"global {name} fonction");
        }

        foreach ((string table, IReadOnlyDictionary<string, double> entries)
                 in surface.Constants.OrderBy(static c => c.Key, StringComparer.Ordinal))
        {
            foreach ((string key, double value) in entries.OrderBy(static e => e.Key, StringComparer.Ordinal))
            {
                lines.Add(string.Create(CultureInfo.InvariantCulture, $"const {table}.{key} {value:G}"));
            }
        }

        foreach (ScriptApiType type in surface.Types.Values.OrderBy(static t => t.Name, StringComparer.Ordinal))
        {
            if (type.Indexed || type.ElementType.Length > 0)
            {
                lines.Add($"member {type.Name}.[i] index {(type.IndexWritable ? "LE" : "L")} {type.ElementType}");
            }

            if (type.CallResult.Length > 0)
            {
                lines.Add($"member {type.Name}.() appel - {type.CallResult}");
            }

            foreach (ScriptApiMember member in type.Members.Values.OrderBy(static m => m.Name, StringComparer.Ordinal))
            {
                string genre = member.Callable ? "appel" : "prop";
                string access = member.Callable ? "-" : member.Writable ? "LE" : "L";
                lines.Add($"member {type.Name}.{member.Name} {genre} {access} {member.ResultType}");
            }
        }

        lines.Add(string.Create(
            CultureInfo.InvariantCulture,
            $"# {surface.Globals.Count + surface.FunctionGlobals.Count} globals, "
                + $"{surface.Types.Count} types, {surface.MemberCount} members"));

        if (output is null)
        {
            foreach (string line in lines)
            {
                Console.Out.WriteLine(line);
            }
        }
        else
        {
            File.WriteAllLines(output, lines);
            Console.Out.WriteLine($"wrote {lines.Count} lines to {output}");
        }

        return 0;
    }
}
