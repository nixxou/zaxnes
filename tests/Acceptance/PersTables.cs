using System.Text.RegularExpressions;

namespace Zax.Acceptance.Tests;

/// <summary>One row of the settings catalogue of SPEC-09 §4.1, as the document writes it.</summary>
/// <param name="Key">Stable key (column « Clé »), brackets expanded.</param>
/// <param name="Lua">Lua access path (column « Lua »), or null when the row shows « — ».</param>
/// <param name="Aliases">The other Lua names the same cell lists.</param>
/// <param name="ReadOnlyLua">Whether the Lua cell carries <c>(lecture)</c>.</param>
/// <param name="Scope">First letter of the column « Portée »: U, P, S, D.</param>
/// <param name="Section">The §4.1.x the row comes from.</param>
public readonly record struct PersSettingRow(
    string Key,
    string? Lua,
    IReadOnlyList<string> Aliases,
    bool ReadOnlyLua,
    char Scope,
    string Section);

/// <summary>
/// Reads the normative tables of SPEC-09 §4.1 and §4.3 out of the specification document, so the
/// settings model of REQ-PERS-001 is compared with the document rather than with a transcription
/// made by the implementers.
/// </summary>
public static class PersTables
{
    private static readonly Lazy<string> Document = new(static () => File.ReadAllText(Locate("SPEC-09-persistance.md")));

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
            catch (Exception exception) when (exception is FileNotFoundException or DirectoryNotFoundException)
            {
                return false;
            }
        }
    }

    /// <summary>
    /// Every row of §4.1.1 to §4.1.10 still in force, in document order: the rows an amendment
    /// has retired since — <c>scene.tv_box</c>, which AMD-031 removed with the decorative TV
    /// box — are dropped, so the comparison is against the specification <b>as amended</b>.
    /// </summary>
    public static IReadOnlyList<PersSettingRow> Settings()
    {
        List<PersSettingRow> rows = [];
        string text = Document.Value;
        int start = text.IndexOf("### 4.1 Catalogue des réglages", StringComparison.Ordinal);
        Assert.True(start >= 0, "SPEC-09 §4.1 is missing.");
        int end = text.IndexOf("### 4.2 Membres Lua", start, StringComparison.Ordinal);
        string section = end > start ? text[start..end] : text[start..];

        string current = "4.1";
        string[] lines = section.Split('\n');
        string[]? header = null;

        foreach (string raw in lines)
        {
            string line = raw.Trim();

            if (line.StartsWith("#### ", StringComparison.Ordinal))
            {
                Match heading = Regex.Match(line, @"4\.1\.\d+");
                current = heading.Success ? heading.Value : current;
                header = null;
                continue;
            }

            if (!line.StartsWith('|') || !line.EndsWith('|'))
            {
                header = null;
                continue;
            }

            string[] cells = [.. line[1..^1].Split('|').Select(static cell => cell.Trim())];
            if (header is null)
            {
                header = cells;
                continue;
            }

            if (cells[0].StartsWith("---", StringComparison.Ordinal))
            {
                continue;
            }

            int keyColumn = Array.IndexOf(header, "Clé");
            int luaColumn = Array.IndexOf(header, "Lua");
            int scopeColumn = Array.IndexOf(header, "Portée");
            if (keyColumn < 0 || keyColumn >= cells.Length)
            {
                continue;
            }

            string luaCell = luaColumn >= 0 && luaColumn < cells.Length ? cells[luaColumn] : "—";
            string scopeCell = scopeColumn >= 0 && scopeColumn < cells.Length ? cells[scopeColumn] : string.Empty;

            // §4.1.8 has no "Portée" column at all: its heading says every row is P.
            char scope = ScopeOf(scopeCell, current);

            List<string> names = [.. Names(luaCell)];
            bool readOnly = luaCell.Contains("(lecture)", StringComparison.Ordinal);

            foreach (string key in Keys(cells[keyColumn]))
            {
                // AMD-031 §3: a retired key is no longer a setting at all.
                if (Zax.Settings.SettingsCatalogue.IsRetired(key))
                {
                    continue;
                }

                string? lua = names.Count > 0 ? Expand(names[0], key) : null;
                List<string> aliases = [.. names.Skip(1).Select(name => Expand(name, key))];
                rows.Add(new PersSettingRow(key, lua, aliases, readOnly, scope, current));
            }
        }

        return rows;
    }

    /// <summary>The constants of §4.3 (REQ-PERS-014): name and value, as text.</summary>
    public static IReadOnlyList<(string Name, string Value)> Constants()
    {
        List<(string, string)> rows = [];
        string text = Document.Value;
        int start = text.IndexOf("### 4.3 Constantes Lua", StringComparison.Ordinal);
        Assert.True(start >= 0, "SPEC-09 §4.3 is missing.");
        int end = text.IndexOf("### 4.4", start, StringComparison.Ordinal);
        string section = end > start ? text[start..end] : text[start..];

        foreach (string raw in section.Split('\n'))
        {
            string line = raw.Trim();
            if (!line.StartsWith('|') || !line.EndsWith('|'))
            {
                continue;
            }

            string[] cells = [.. line[1..^1].Split('|').Select(static cell => cell.Trim())];
            if (cells.Length < 2 || cells[0].StartsWith("---", StringComparison.Ordinal))
            {
                continue;
            }

            Add(cells, 0);
            if (cells.Length >= 5)
            {
                Add(cells, 3);
            }
        }

        return rows;

        void Add(string[] cells, int index)
        {
            if (index + 1 >= cells.Length)
            {
                return;
            }

            foreach (string name in Names(cells[index]))
            {
                string value = cells[index + 1].Trim('"', '`', ' ');
                if (name.Length > 0 && value.Length > 0)
                {
                    rows.Add((name, value));
                }
            }
        }
    }

    private static char ScopeOf(string cell, string section)
    {
        if (section == "4.1.8")
        {
            return 'P';
        }

        foreach (char c in cell)
        {
            if (c is 'U' or 'P' or 'S' or 'D' or 'J')
            {
                return c;
            }
        }

        return '?';
    }

    private static IEnumerable<string> Keys(string cell)
    {
        foreach (string name in Names(cell))
        {
            if (name.Contains("[i]", StringComparison.Ordinal))
            {
                for (int index = 0; index < 6; index++)
                {
                    yield return name.Replace("[i]", $"[{index}]", StringComparison.Ordinal);
                }
            }
            else
            {
                yield return name;
            }
        }
    }

    private static string Expand(string name, string key)
    {
        Match index = Regex.Match(key, @"\[(\d+)\]");
        return index.Success
            ? name.Replace("[i]", $"[{index.Groups[1].Value}]", StringComparison.Ordinal)
            : name;
    }

    private static IEnumerable<string> Names(string cell)
    {
        foreach (Match match in Regex.Matches(cell, "`([^`]+)`"))
        {
            string name = match.Groups[1].Value.Trim();
            if (name.Length > 0 && name != "—")
            {
                yield return name;
            }
        }
    }

    private static string Locate(string file)
    {
        DirectoryInfo? at = new(AppContext.BaseDirectory);
        while (at is not null)
        {
            string candidate = Path.Combine(at.FullName, "spec", file);
            if (File.Exists(candidate))
            {
                return candidate;
            }

            at = at.Parent;
        }

        throw new FileNotFoundException($"spec/{file} was not found above the test binary.");
    }
}

/// <summary>A fact skipped, not failed, when SPEC-09 is not reachable.</summary>
public sealed class PersFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public PersFactAttribute()
    {
        if (!PersTables.Available)
        {
            Skip = "spec/SPEC-09-persistance.md is not reachable above the test binary.";
        }
    }
}
