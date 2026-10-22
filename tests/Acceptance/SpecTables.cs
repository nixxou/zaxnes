using System.Text.RegularExpressions;

namespace Zax.Acceptance.Tests;

/// <summary>One row of a member table of SPEC-05 §4.2 to §4.4.</summary>
/// <param name="Type">The type the row belongs to, as the document spells it.</param>
/// <param name="Member">Member name, or <c>[i]</c> / <c>()</c>.</param>
/// <param name="Kind">Genre: <c>prop</c>, <c>méth</c>, <c>fonc</c>, <c>const</c>, <c>index</c>, <c>appel</c>.</param>
/// <param name="ResultType">The Type column, verbatim.</param>
/// <param name="Access">Access column: <c>L</c>, <c>LE</c> or <c>—</c>.</param>
/// <param name="Priority">Priority column: <c>P1</c>, <c>P2</c> or <c>P3</c>.</param>
public readonly record struct SpecMemberRow(
    string Type,
    string Member,
    string Kind,
    string ResultType,
    string Access,
    string Priority);

/// <summary>One row of the globals table of SPEC-05 §4.1.</summary>
/// <param name="Name">Global name.</param>
/// <param name="Type">Type column, verbatim.</param>
/// <param name="Access">Access column.</param>
/// <param name="Priority">Priority column.</param>
public readonly record struct SpecGlobalRow(string Name, string Type, string Access, string Priority);

/// <summary>
/// Reads the normative tables of SPEC-05 §4 straight out of the specification document, so that
/// the API check of REQ-LUA-080 compares the runtime against the document rather than against a
/// transcription made by the implementers.
/// </summary>
public static class SpecTables
{
    private static readonly Lazy<string> Document = new(static () => File.ReadAllText(Locate()));

    /// <summary>Whether the specification tree was found next to the solution.</summary>
    public static bool Available
    {
        get
        {
            try
            {
                _ = Document.Value;
                return true;
            }
            catch (FileNotFoundException)
            {
                return false;
            }
            catch (DirectoryNotFoundException)
            {
                return false;
            }
        }
    }

    /// <summary>The rows of the globals table of §4.1.</summary>
    public static IReadOnlyList<SpecGlobalRow> Globals()
    {
        List<SpecGlobalRow> rows = [];
        string section = Section("### 4.1 Globales", "### 4.2");
        foreach (string[] cells in TableRows(section, 5))
        {
            rows.Add(new SpecGlobalRow(Name(cells[0]), Clean(cells[1]), Clean(cells[2]), Clean(cells[3])));
        }

        return rows;
    }

    /// <summary>The member rows of every type table of §4.2 and §4.3.</summary>
    public static IReadOnlyList<SpecMemberRow> Members()
    {
        List<SpecMemberRow> rows = [];
        string section = Section("### 4.2 Objets hôtes", "### 4.4");

        foreach (Match heading in Regex.Matches(section, @"^#### Type `([^`]+)`", RegexOptions.Multiline))
        {
            string type = heading.Groups[1].Value;
            int start = heading.Index + heading.Length;
            Match next = Regex.Match(
                section[start..], @"^#### Type `", RegexOptions.Multiline);
            string body = next.Success ? section[start..(start + next.Index)] : section[start..];

            foreach (string[] cells in TableRows(body, 6))
            {
                rows.Add(new SpecMemberRow(
                    type,
                    Name(cells[0]),
                    Clean(cells[1]),
                    Clean(cells[2]),
                    Clean(cells[3]),
                    Clean(cells[4])));
            }
        }

        return rows;
    }

    /// <summary>The constant rows of the enumeration tables of §4.4.</summary>
    public static IReadOnlyList<(string Table, string Name, double Value)> Constants()
    {
        List<(string, string, double)> rows = [];
        string section = Section("### 4.4 Énumérations", "### 4.5");

        foreach (Match heading in Regex.Matches(section, @"^#### Type `([^`]+)`", RegexOptions.Multiline))
        {
            string type = heading.Groups[1].Value;
            int start = heading.Index + heading.Length;
            Match next = Regex.Match(section[start..], @"^#### Type `", RegexOptions.Multiline);
            string body = next.Success ? section[start..(start + next.Index)] : section[start..];

            foreach (string[] cells in TableRows(body, 6))
            {
                // The "Effet" column of a constant row starts with its integer value.
                Match value = Regex.Match(Clean(cells[5]), @"^(-?\d+)");
                if (value.Success)
                {
                    rows.Add((type, Name(cells[0]), double.Parse(value.Groups[1].Value,
                        System.Globalization.CultureInfo.InvariantCulture)));
                }
            }
        }

        return rows;
    }

    private static string Section(string from, string to)
    {
        string text = Document.Value;
        int start = text.IndexOf(from, StringComparison.Ordinal);
        Assert.True(start >= 0, $"the section '{from}' is missing from SPEC-05.");
        int end = text.IndexOf(to, start, StringComparison.Ordinal);
        return end > start ? text[start..end] : text[start..];
    }

    private static IEnumerable<string[]> TableRows(string body, int columns)
    {
        foreach (string line in body.Split('\n'))
        {
            string trimmed = line.Trim();
            if (!trimmed.StartsWith('|') || !trimmed.EndsWith('|'))
            {
                continue;
            }

            string[] cells = trimmed[1..^1].Split('|');
            if (cells.Length != columns)
            {
                continue;
            }

            string first = Clean(cells[0]);

            // Skip the header row and the separator row.
            if (first is "Membre" or "Nom" || first.StartsWith("---", StringComparison.Ordinal))
            {
                continue;
            }

            yield return cells;
        }
    }

    private static string Clean(string cell) => cell.Trim();

    private static string Name(string cell)
    {
        string text = Clean(cell);
        return text.StartsWith('`') && text.EndsWith('`') ? text[1..^1] : text;
    }

    private static string Locate()
    {
        DirectoryInfo? at = new(AppContext.BaseDirectory);
        while (at is not null)
        {
            string candidate = Path.Combine(at.FullName, "spec", "SPEC-05-lua.md");
            if (File.Exists(candidate))
            {
                return candidate;
            }

            at = at.Parent;
        }

        throw new FileNotFoundException("spec/SPEC-05-lua.md was not found above the test binary.");
    }
}

/// <summary>A fact skipped, not failed, when the specification tree is not reachable.</summary>
public sealed class SpecFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public SpecFactAttribute()
    {
        if (!SpecTables.Available)
        {
            Skip = "spec/SPEC-05-lua.md is not reachable above the test binary.";
        }
    }
}
