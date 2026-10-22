namespace Zax.Profiles;

/// <summary>
/// The order of the scripts of a profile (SPEC-05 REQ-LUA-004, SPEC-03 REQ-PROF-024,
/// DEC-03-Q02): a non-ordinal, culture-independent collation of the names. It is also the order
/// in which the Lua hooks are called.
/// </summary>
/// <remarks>
/// <para>
/// The comparison, applied character by character up to the first difference:
/// </para>
/// <list type="number">
/// <item>primary key of a character = its class (punctuation, space and symbols &lt; digits
/// &lt; letters), then the character lowercased;</item>
/// <item>primary keys equal over the whole common length: the shorter name first;</item>
/// <item>primary keys fully equal: the case decides, lowercase before uppercase at the first
/// difference.</item>
/// </list>
/// <para>
/// This is written out rather than delegated to <see cref="StringComparer.InvariantCulture"/>
/// on purpose. The decision asks for a linguistic collation, and the solution is built with
/// <c>InvariantGlobalization</c>, under which the framework's culture-sensitive comparisons
/// fall back to ordinal — which would put <c>ramp7</c> before <c>ramp_rotation</c> and change
/// the hook order of Excitebike. Writing the rule here also makes the order deterministic
/// across machines, as REQ-ARCH-012 requires.
/// </para>
/// <para>
/// The rule is normative for printable ASCII, which covers every built-in profile. Other
/// characters (possible in user profiles) are ordered by the same three levels, using the
/// invariant lowercase of each UTF-16 code unit; any divergence from the original there is
/// unspecified.
/// </para>
/// </remarks>
public sealed class ScriptNameComparer : IComparer<string?>, IComparer<ProfileScript>
{
    /// <summary>The single instance.</summary>
    public static ScriptNameComparer Instance { get; } = new();

    private ScriptNameComparer()
    {
    }

    /// <summary>Compares two script names.</summary>
    public int Compare(string? left, string? right)
    {
        if (ReferenceEquals(left, right))
        {
            return 0;
        }

        if (left is null)
        {
            return -1;
        }

        if (right is null)
        {
            return 1;
        }

        int common = Math.Min(left.Length, right.Length);
        for (int i = 0; i < common; i++)
        {
            int difference = ComparePrimary(left[i], right[i]);
            if (difference != 0)
            {
                return difference;
            }
        }

        if (left.Length != right.Length)
        {
            return left.Length < right.Length ? -1 : 1;
        }

        for (int i = 0; i < common; i++)
        {
            char a = left[i];
            char b = right[i];
            if (a == b)
            {
                continue;
            }

            bool lowerA = char.IsLower(a);
            bool lowerB = char.IsLower(b);
            if (lowerA != lowerB)
            {
                return lowerA ? -1 : 1;
            }

            // Two distinct characters with the same lowercase and the same case class: no rule
            // applies, so fall back to the code unit to keep the order total and deterministic.
            return a < b ? -1 : 1;
        }

        return 0;
    }

    /// <summary>Compares two scripts by name.</summary>
    public int Compare(ProfileScript? left, ProfileScript? right)
    {
        if (ReferenceEquals(left, right))
        {
            return 0;
        }

        if (left is null)
        {
            return -1;
        }

        if (right is null)
        {
            return 1;
        }

        return Compare(left.Name, right.Name);
    }

    int IComparer<ProfileScript>.Compare(ProfileScript? left, ProfileScript? right)
        => Compare(left, right);

    private static int ComparePrimary(char left, char right)
    {
        int classLeft = ClassOf(left);
        int classRight = ClassOf(right);
        if (classLeft != classRight)
        {
            return classLeft < classRight ? -1 : 1;
        }

        char lowerLeft = char.ToLowerInvariant(left);
        char lowerRight = char.ToLowerInvariant(right);
        if (lowerLeft == lowerRight)
        {
            return 0;
        }

        return lowerLeft < lowerRight ? -1 : 1;
    }

    private static int ClassOf(char value)
    {
        if (char.IsDigit(value))
        {
            return 1;
        }

        return char.IsLetter(value) ? 2 : 0;
    }
}
