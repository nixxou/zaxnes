namespace Zax.Settings;

/// <summary>
/// The domain of a setting — the column "Domaine" of SPEC-09 §4.1 — and the rule that brings a
/// value back into it (REQ-PERS-003).
/// </summary>
public sealed class SettingDomain
{
    /// <summary>A domain that accepts anything.</summary>
    public static SettingDomain Unbounded { get; } = new(OutOfDomainRule.Unbounded);

    private SettingDomain(OutOfDomainRule rule)
    {
        Rule = rule;
    }

    /// <summary>What to do with a value outside the domain.</summary>
    public OutOfDomainRule Rule { get; private init; }

    /// <summary>Lower bound, or null.</summary>
    public double? Minimum { get; private init; }

    /// <summary>Upper bound, or null.</summary>
    public double? Maximum { get; private init; }

    /// <summary>Whether <see cref="Minimum"/> is excluded, as in the domain "&gt; 0".</summary>
    public bool MinimumExclusive { get; private init; }

    /// <summary>The finite ordered set of admitted values, or null when there is none.</summary>
    public IReadOnlyList<SettingOption>? Options { get; private init; }

    /// <summary>A closed interval; a value outside it is clamped.</summary>
    public static SettingDomain Range(double minimum, double maximum) => new(OutOfDomainRule.Clamp)
    {
        Minimum = minimum,
        Maximum = maximum,
    };

    /// <summary>A half-open interval <c>(minimum, ∞)</c>; a value outside takes the default.</summary>
    public static SettingDomain GreaterThan(double minimum) => new(OutOfDomainRule.UseDefault)
    {
        Minimum = minimum,
        MinimumExclusive = true,
    };

    /// <summary>A lower bound only; a value below it is clamped.</summary>
    public static SettingDomain AtLeast(double minimum) => new(OutOfDomainRule.Clamp)
    {
        Minimum = minimum,
    };

    /// <summary>
    /// A finite set of values, each with its label. A value that is not in the set takes the
    /// default, which is the rule REQ-PERS-003 gives for <c>video.frame_rate</c>,
    /// <c>scene.skybox</c> and <c>video.res_scale</c>.
    /// </summary>
    public static SettingDomain Choice(params SettingOption[] options) => new(OutOfDomainRule.UseDefault)
    {
        Options = options,
    };

    /// <summary>The two booleans, labelled as libretro expects them (REQ-PERS-083).</summary>
    public static SettingDomain Switch { get; } = new(OutOfDomainRule.Unbounded)
    {
        Options = [new SettingOption(0d, "disabled"), new SettingOption(1d, "enabled")],
    };

    /// <summary>A whole number taken modulo the number of available cores (rule 2).</summary>
    public static SettingDomain CoreIndex { get; } = new(OutOfDomainRule.ModuloCoreCount);

    /// <summary>A number whose sign is dropped (rule 4).</summary>
    public static SettingDomain NonNegativeByAbsoluteValue { get; } = new(OutOfDomainRule.AbsoluteValue);

    /// <summary>A vector the domain "non nul" refuses to be zero.</summary>
    public static SettingDomain NonZeroVector { get; } = new(OutOfDomainRule.UseDefault)
    {
        Minimum = 0d,
        MinimumExclusive = true,
    };

    /// <summary>The label of <paramref name="value"/> in this domain, or null.</summary>
    public string? LabelOf(double value)
    {
        if (Options is null)
        {
            return null;
        }

        foreach (SettingOption option in Options)
        {
            if (option.Value.Equals(value))
            {
                return option.Label;
            }
        }

        return null;
    }

    /// <summary>The value <paramref name="label"/> designates, or null.</summary>
    public double? ValueOf(string label)
    {
        if (Options is null || label is null)
        {
            return null;
        }

        foreach (SettingOption option in Options)
        {
            if (string.Equals(option.Label, label, StringComparison.OrdinalIgnoreCase))
            {
                return option.Value;
            }
        }

        return null;
    }

    /// <summary>
    /// Brings <paramref name="value"/> into the domain (REQ-PERS-003). A value that no rule can
    /// repair becomes <paramref name="fallback"/>, the setting's default.
    /// </summary>
    /// <param name="value">The value to validate.</param>
    /// <param name="fallback">The setting's default value.</param>
    /// <param name="coreCount">Number of available cores, at least 1 (REQ-CORE-120).</param>
    public SettingValue Coerce(SettingValue value, SettingValue fallback, int coreCount = 1)
    {
        if (value.Kind == SettingKind.Vector3)
        {
            return Rule == OutOfDomainRule.UseDefault && value.AsVector3.IsZero ? fallback : value;
        }

        if (value.Kind is SettingKind.Color or SettingKind.String or SettingKind.Rect or SettingKind.Opaque)
        {
            return value;
        }

        double number = value.Kind == SettingKind.Bool ? (value.AsBool ? 1d : 0d) : value.AsDouble;

        if (Options is not null && Rule != OutOfDomainRule.Unbounded && LabelOf(number) is null)
        {
            return fallback;
        }

        switch (Rule)
        {
            case OutOfDomainRule.Clamp:
                if (Minimum is { } low && number < low)
                {
                    number = low;
                }

                if (Maximum is { } high && number > high)
                {
                    number = high;
                }

                break;

            case OutOfDomainRule.AbsoluteValue:
                number = Math.Abs(number);
                break;

            case OutOfDomainRule.ModuloCoreCount:
                int cores = Math.Max(1, coreCount);
                number = ((int)number % cores + cores) % cores;
                break;

            case OutOfDomainRule.UseDefault:
                bool belowMinimum = Minimum is { } bound
                    && (MinimumExclusive ? number <= bound : number < bound);
                if (belowMinimum || (Maximum is { } ceiling && number > ceiling))
                {
                    return fallback;
                }

                break;

            case OutOfDomainRule.Unbounded:
            default:
                break;
        }

        return value.Kind switch
        {
            SettingKind.Bool => SettingValue.OfBool(number != 0d),
            SettingKind.Int => SettingValue.OfInt((int)number),
            SettingKind.Enum => SettingValue.OfEnum((int)number),
            _ => SettingValue.OfFloat((float)number),
        };
    }
}
