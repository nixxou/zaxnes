using System.Text;
using Zax.Scripting;

namespace Zax.Scripting.Tests;

/// <summary>One member access the resolver could not satisfy (SPEC-05 REQ-LUA-051).</summary>
/// <param name="Profile">Profile file the script belongs to.</param>
/// <param name="Script">Script name.</param>
/// <param name="Line">1-based line of the access.</param>
/// <param name="Type">The script-visible type the chain had reached.</param>
/// <param name="Member">The member name that is missing.</param>
/// <param name="Separator">The separator used: <c>.</c> or <c>:</c>.</param>
public readonly record struct UnresolvedAccess(
    string Profile,
    string Script,
    int Line,
    string Type,
    string Member,
    char Separator)
{
    /// <inheritdoc/>
    public override string ToString()
        => $"{Profile}/{Script}:{Line}: {Type}{Separator}{Member}";
}

/// <summary>
/// The static check of SPEC-05 REQ-LUA-051: it reads a script, follows every access chain that
/// starts at a global of §4.1 or at a variable whose type it could infer, and resolves each
/// member against the API description the runtime produces.
/// </summary>
/// <remarks>
/// <para>
/// It is deliberately conservative: a value whose type it cannot infer — a function parameter,
/// a Lua table of the script, an arithmetic result — stops the chain and is not checked. That
/// is the same split the specification reports in §5.1 between typed and untyped accesses, and
/// it means the check can only produce <b>real</b> findings, never a false alarm on a field of
/// a script's own table.
/// </para>
/// <para>
/// A variable that is assigned once from a typed chain is typed; a variable that is also
/// assigned anything else is poisoned and left untyped, so a reused name cannot mislead it.
/// </para>
/// </remarks>
public sealed class LuaAccessResolver
{
    /// <summary>
    /// The types whose members are table functions, called with <c>.</c>, as opposed to
    /// instance methods called with <c>:</c> (SPEC-05 REQ-LUA-081).
    /// </summary>
    private static readonly HashSet<string> FunctionTables = new(StringComparer.Ordinal)
    {
        "PaletteAPI",
        "PatternAPI",
        "NotiAPI",
        "Vector3Classe",
        "Vector2Classe",
        "QuaternionClasse",
        "ColorClasse",
        "IntVector2Classe",
        "IntVector3Classe",
        "RectClasse",
    };

    /// <summary>
    /// Member names the corpus misspells and that REQ-LUA-042 requires to read as nil, so they
    /// are not findings (SPEC-05 §5.1).
    /// </summary>
    private static readonly HashSet<string> KnownTypos = new(StringComparer.Ordinal)
    {
        "hiden",
        "Hiden",
    };

    private readonly ScriptApiSurface _api;
    private readonly List<UnresolvedAccess> _unresolved = [];
    private readonly List<UnresolvedAccess> _wrongSeparator = [];
    private readonly HashSet<string> _resolvedCouples = new(StringComparer.Ordinal);

    /// <summary>Creates a resolver over <paramref name="api"/>.</summary>
    public LuaAccessResolver(ScriptApiSurface api)
    {
        ArgumentNullException.ThrowIfNull(api);
        _api = api;
    }

    /// <summary>The accesses that resolved to no member at all.</summary>
    public IReadOnlyList<UnresolvedAccess> Unresolved => _unresolved;

    /// <summary>The accesses that used the wrong separator (REQ-LUA-081).</summary>
    public IReadOnlyList<UnresolvedAccess> WrongSeparator => _wrongSeparator;

    /// <summary>The distinct <c>(type, member)</c> couples the resolver satisfied.</summary>
    public IReadOnlyCollection<string> ResolvedCouples => _resolvedCouples;

    /// <summary>How many chains were followed, typed accesses included.</summary>
    public int TypedAccessCount { get; private set; }

    /// <summary>
    /// Checks every script of one profile. The scripts share one variable map, because they
    /// share one global environment (REQ-LUA-010).
    /// </summary>
    public void Check(string profile, IEnumerable<(string Name, string Source)> scripts)
    {
        ArgumentNullException.ThrowIfNull(scripts);

        List<(string Name, Token[] Tokens)> tokenised = [];
        foreach ((string name, string source) in scripts)
        {
            tokenised.Add((name, Tokenise(source ?? string.Empty)));
        }

        // Pass 1: infer the type of the variables assigned from a typed chain.
        Dictionary<string, string> variables = new(StringComparer.Ordinal);
        HashSet<string> poisoned = new(StringComparer.Ordinal);
        foreach ((_, Token[] tokens) in tokenised)
        {
            InferVariables(tokens, variables, poisoned);
        }

        foreach (string name in poisoned)
        {
            variables.Remove(name);
        }

        // Pass 2: resolve every chain.
        foreach ((string name, Token[] tokens) in tokenised)
        {
            Resolve(profile, name, tokens, variables);
        }
    }

    private void InferVariables(
        Token[] tokens,
        Dictionary<string, string> variables,
        HashSet<string> poisoned)
    {
        for (int i = 0; i < tokens.Length; i++)
        {
            // NAME = <expression>, with no comma before the '=' (a multiple assignment is
            // skipped) and no '.' or ':' after the name (that is a member write, not a binding).
            if (tokens[i].Kind != TokenKind.Name
                || i + 1 >= tokens.Length
                || tokens[i + 1].Text != "=")
            {
                continue;
            }

            if (i > 0 && (tokens[i - 1].Text == "," || tokens[i - 1].Text == "."
                || tokens[i - 1].Text == ":"))
            {
                continue;
            }

            if (i + 2 < tokens.Length && tokens[i + 2].Text == ",")
            {
                // The right-hand side is a list; its parts are not followed here.
                poisoned.Add(tokens[i].Text);
                continue;
            }

            string? inferred = Follow(tokens, i + 2, variables, report: null);
            if (inferred is null)
            {
                poisoned.Add(tokens[i].Text);
            }
            else if (variables.TryGetValue(tokens[i].Text, out string? existing)
                && !string.Equals(existing, inferred, StringComparison.Ordinal))
            {
                poisoned.Add(tokens[i].Text);
            }
            else
            {
                variables[tokens[i].Text] = inferred;
            }
        }
    }

    private void Resolve(
        string profile,
        string script,
        Token[] tokens,
        Dictionary<string, string> variables)
    {
        for (int i = 0; i < tokens.Length; i++)
        {
            if (tokens[i].Kind != TokenKind.Name)
            {
                continue;
            }

            // Only the head of a chain starts a resolution.
            if (i > 0 && (tokens[i - 1].Text == "." || tokens[i - 1].Text == ":"))
            {
                continue;
            }

            if (!RootType(tokens[i].Text, variables, out string _))
            {
                continue;
            }

            TypedAccessCount++;
            Follow(tokens, i, variables, report: (line, type, member, separator) =>
            {
                UnresolvedAccess finding = new(profile, script, line, type, member, separator);
                if (separator == '!')
                {
                    _wrongSeparator.Add(finding with { Separator = '.' });
                }
                else
                {
                    _unresolved.Add(finding);
                }
            });
        }
    }

    private bool RootType(string name, Dictionary<string, string> variables, out string type)
    {
        if (_api.Globals.TryGetValue(name, out string? global) && global.Length > 0)
        {
            type = global;
            return true;
        }

        if (variables.TryGetValue(name, out string? inferred))
        {
            type = inferred;
            return true;
        }

        type = string.Empty;
        return false;
    }

    /// <summary>
    /// Follows the chain that starts at <paramref name="start"/> and returns the type it ends
    /// on, or null when the type is unknown. <paramref name="report"/> receives every member
    /// that could not be resolved; passing null makes the walk silent.
    /// </summary>
    private string? Follow(
        Token[] tokens,
        int start,
        Dictionary<string, string> variables,
        Action<int, string, string, char>? report)
    {
        if (start >= tokens.Length || tokens[start].Kind != TokenKind.Name)
        {
            return null;
        }

        if (!RootType(tokens[start].Text, variables, out string type))
        {
            return null;
        }

        int i = start + 1;
        while (i < tokens.Length)
        {
            string punctuation = tokens[i].Text;
            if (punctuation is "." or ":")
            {
                if (i + 1 >= tokens.Length || tokens[i + 1].Kind != TokenKind.Name)
                {
                    return null;
                }

                string member = tokens[i + 1].Text;
                bool called = i + 2 < tokens.Length && tokens[i + 2].Text == "(";
                if (!_api.Types.TryGetValue(type, out ScriptApiType? described))
                {
                    return null;
                }

                if (!described.Members.TryGetValue(member, out ScriptApiMember found))
                {
                    // hiden and Hiden are misspellings of Hidden present in the corpus; the
                    // specification requires them to read as nil, not to be members
                    // (REQ-LUA-042, §5.1).
                    if (!KnownTypos.Contains(member))
                    {
                        report?.Invoke(tokens[i + 1].Line, type, member, punctuation[0]);
                    }

                    return null;
                }

                _resolvedCouples.Add($"{type}.{member}");

                // A table function is called with '.', an instance method with ':'
                // (REQ-LUA-081). A call through the wrong separator is reported apart.
                if (found.Callable && called)
                {
                    bool wantsDot = FunctionTables.Contains(type);
                    if ((punctuation == "." && !wantsDot) || (punctuation == ":" && wantsDot))
                    {
                        report?.Invoke(tokens[i + 1].Line, type, member, '!');
                    }
                }

                if (found.Callable && !called)
                {
                    // The function value itself; the chain stops.
                    return null;
                }

                if (found.ResultType.Length == 0)
                {
                    return null;
                }

                type = found.ResultType;
                i = called ? SkipBalanced(tokens, i + 2, "(", ")") : i + 2;
                continue;
            }

            if (punctuation == "[")
            {
                if (!_api.Types.TryGetValue(type, out ScriptApiType? described)
                    || described.ElementType.Length == 0)
                {
                    return null;
                }

                type = described.ElementType;
                i = SkipBalanced(tokens, i, "[", "]");
                continue;
            }

            if (punctuation == "(")
            {
                if (!_api.Types.TryGetValue(type, out ScriptApiType? described)
                    || described.CallResult.Length == 0)
                {
                    return null;
                }

                type = described.CallResult;
                i = SkipBalanced(tokens, i, "(", ")");
                continue;
            }

            break;
        }

        return type;
    }

    private static int SkipBalanced(Token[] tokens, int open, string opening, string closing)
    {
        int depth = 0;
        for (int i = open; i < tokens.Length; i++)
        {
            if (tokens[i].Text == opening)
            {
                depth++;
            }
            else if (tokens[i].Text == closing)
            {
                depth--;
                if (depth == 0)
                {
                    return i + 1;
                }
            }
        }

        return tokens.Length;
    }

    private enum TokenKind
    {
        Name,
        Number,
        Text,
        Punctuation,
    }

    private readonly record struct Token(TokenKind Kind, string Text, int Line);

    /// <summary>
    /// Splits a Lua source into names, numbers, strings and punctuation, dropping comments.
    /// Long strings and long comments are handled, which the corpus uses.
    /// </summary>
    private static Token[] Tokenise(string source)
    {
        List<Token> tokens = [];
        int line = 1;
        int i = 0;
        while (i < source.Length)
        {
            char c = source[i];
            if (c == '\n')
            {
                line++;
                i++;
                continue;
            }

            if (char.IsWhiteSpace(c))
            {
                i++;
                continue;
            }

            // Comments.
            if (c == '-' && i + 1 < source.Length && source[i + 1] == '-')
            {
                i += 2;
                if (TryLongBracket(source, ref i, ref line))
                {
                    continue;
                }

                while (i < source.Length && source[i] != '\n')
                {
                    i++;
                }

                continue;
            }

            // Long strings.
            if (c == '[' && i + 1 < source.Length && (source[i + 1] == '[' || source[i + 1] == '='))
            {
                int saved = i;
                if (TryLongBracket(source, ref i, ref line))
                {
                    tokens.Add(new Token(TokenKind.Text, "<long>", line));
                    continue;
                }

                i = saved;
            }

            if (c is '"' or '\'')
            {
                i++;
                StringBuilder text = new();
                while (i < source.Length && source[i] != c)
                {
                    if (source[i] == '\\' && i + 1 < source.Length)
                    {
                        i += 2;
                        continue;
                    }

                    if (source[i] == '\n')
                    {
                        line++;
                    }

                    text.Append(source[i]);
                    i++;
                }

                i++;
                tokens.Add(new Token(TokenKind.Text, text.ToString(), line));
                continue;
            }

            if (char.IsLetter(c) || c == '_')
            {
                int begin = i;
                while (i < source.Length && (char.IsLetterOrDigit(source[i]) || source[i] == '_'))
                {
                    i++;
                }

                tokens.Add(new Token(TokenKind.Name, source[begin..i], line));
                continue;
            }

            if (char.IsDigit(c))
            {
                int begin = i;
                while (i < source.Length
                    && (char.IsLetterOrDigit(source[i]) || source[i] == '.'
                        || ((source[i] == '-' || source[i] == '+')
                            && (source[i - 1] == 'e' || source[i - 1] == 'E'))))
                {
                    i++;
                }

                tokens.Add(new Token(TokenKind.Number, source[begin..i], line));
                continue;
            }

            // Two-character operators matter only in so far as they are not '.' or ':'.
            if (i + 1 < source.Length)
            {
                string pair = source.Substring(i, 2);
                if (pair is "==" or "~=" or "<=" or ">=" or ".." or "::")
                {
                    tokens.Add(new Token(TokenKind.Punctuation, pair, line));
                    i += 2;
                    continue;
                }
            }

            tokens.Add(new Token(TokenKind.Punctuation, c.ToString(), line));
            i++;
        }

        return [.. tokens];
    }

    /// <summary>
    /// Consumes a long bracket <c>[[…]]</c> or <c>[=[…]=]</c> starting at <paramref name="i"/>,
    /// and says whether there was one.
    /// </summary>
    private static bool TryLongBracket(string source, ref int i, ref int line)
    {
        if (i >= source.Length || source[i] != '[')
        {
            return false;
        }

        int level = 0;
        int probe = i + 1;
        while (probe < source.Length && source[probe] == '=')
        {
            level++;
            probe++;
        }

        if (probe >= source.Length || source[probe] != '[')
        {
            return false;
        }

        string closing = "]" + new string('=', level) + "]";
        int end = source.IndexOf(closing, probe + 1, StringComparison.Ordinal);
        int stop = end < 0 ? source.Length : end + closing.Length;
        for (int k = i; k < stop; k++)
        {
            if (source[k] == '\n')
            {
                line++;
            }
        }

        i = stop;
        return true;
    }
}
