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

    /// <summary>
    /// Written values that stand for a listed one, or null when the domain has none. See
    /// <see cref="AliasedChoice"/>.
    /// </summary>
    public IReadOnlyDictionary<double, double>? Aliases { get; private init; }

    /// <summary>The step a <see cref="OutOfDomainRule.Snap"/> domain lands on, or 0.</summary>
    public double Step { get; private init; }

    /// <summary>
    /// Whether the domain is the finite reals, so that NaN and ±∞ are outside it. See
    /// <see cref="Finite"/>.
    /// </summary>
    public bool FiniteOnly { get; private init; }

    /// <summary>A closed interval; a value outside it is clamped.</summary>
    public static SettingDomain Range(double minimum, double maximum) => new(OutOfDomainRule.Clamp)
    {
        Minimum = minimum,
        Maximum = maximum,
    };

    /// <summary>
    /// Every finite real, negative included. Only NaN and ±∞ are outside, and the column
    /// "Domaine" gives no rule to bring them back, so the setting's default applies — the
    /// general rule of REQ-PERS-003. The four clip margins have this domain since AMD-047
    /// (REQ-PERS-087): 256 and 248 are no longer bounds but the reference dimensions of the
    /// picture and of the conversion grid.
    /// </summary>
    public static SettingDomain Finite { get; } = new(OutOfDomainRule.UseDefault)
    {
        FiniteOnly = true,
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

    /// <summary>
    /// A closed interval whose whole values each carry a label, so an interface can show a list
    /// while a value outside the interval is <b>clamped</b> rather than replaced by the default.
    /// That is the domain « 0 à 3 » of the Run-ahead setting (AMD-041 §3, REQ-LOOP-064).
    /// </summary>
    /// <param name="minimum">Lower bound, the value of the first label.</param>
    /// <param name="labels">
    /// One label per whole value from <paramref name="minimum"/> upwards; the last one sets the
    /// upper bound.
    /// </param>
    public static SettingDomain ClampedChoice(double minimum, params string[] labels)
    {
        ArgumentNullException.ThrowIfNull(labels);
        if (labels.Length == 0)
        {
            throw new ArgumentException("A clamped choice needs at least one label.", nameof(labels));
        }

        SettingOption[] options = new SettingOption[labels.Length];
        for (int i = 0; i < labels.Length; i++)
        {
            options[i] = new SettingOption(minimum + i, labels[i]);
        }

        return new SettingDomain(OutOfDomainRule.Clamp)
        {
            Minimum = minimum,
            Maximum = minimum + labels.Length - 1,
            Options = options,
        };
    }

    /// <summary>
    /// A regular ladder <c>a, a + pas, …, b</c> (AMD-040 §3): a value is brought into
    /// <c>[a, b]</c>, then replaced by the nearest rung — ties going to the <b>even</b>
    /// multiple — then brought back into <c>[a, b]</c>. The four curvature settings of
    /// AMD-043 REQ-SCN-066 are the ones that have it.
    /// </summary>
    /// <param name="minimum">Lower bound <c>a</c>, which is also the first rung.</param>
    /// <param name="maximum">Upper bound <c>b</c>.</param>
    /// <param name="step">The step, strictly positive.</param>
    /// <param name="format">Number format of the labels, which the options list shows.</param>
    public static SettingDomain Stepped(
        double minimum,
        double maximum,
        double step,
        string format = "0.####")
        => new(OutOfDomainRule.Snap)
        {
            Minimum = minimum,
            Maximum = maximum,
            Step = step > 0d ? step : throw new ArgumentOutOfRangeException(nameof(step)),
            Options = SettingDescriptor.Steps(minimum, maximum, step, format),
        };

    /// <summary>
    /// The same ladder with labels of its own, for a setting whose values are named rather
    /// than numbered — <c>scene.arc_center</c> of AMD-043 §6.
    /// </summary>
    /// <param name="minimum">Lower bound <c>a</c>, which is also the first rung.</param>
    /// <param name="maximum">Upper bound <c>b</c>.</param>
    /// <param name="step">The step, strictly positive.</param>
    /// <param name="options">One option per rung, in order.</param>
    public static SettingDomain Stepped(
        double minimum,
        double maximum,
        double step,
        params SettingOption[] options)
        => new(OutOfDomainRule.Snap)
        {
            Minimum = minimum,
            Maximum = maximum,
            Step = step > 0d ? step : throw new ArgumentOutOfRangeException(nameof(step)),
            Options = options,
        };

    /// <summary>
    /// A finite set of values of which some are reached through an <b>alias</b>: a written
    /// value the table maps onto one of the listed ones, which is then what is read back. The
    /// visual style of AMD-055 REQ-SCN-078 is the one that has it — 0 and 4 are the style
    /// identifiers, 1 is accepted in writing and read back as 4, anything else takes the
    /// default.
    /// </summary>
    /// <param name="aliases">Written value, then the listed value it stands for.</param>
    /// <param name="options">The listed values, each with its label.</param>
    public static SettingDomain AliasedChoice(
        IReadOnlyDictionary<double, double> aliases,
        params SettingOption[] options)
    {
        ArgumentNullException.ThrowIfNull(aliases);
        return new SettingDomain(OutOfDomainRule.UseDefault)
        {
            Options = options,
            Aliases = aliases,
        };
    }

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

        // AMD-047 REQ-PERS-087: NaN and ±∞ are outside the finite reals and no rule repairs
        // them, so the default applies (REQ-PERS-003).
        if (FiniteOnly && !double.IsFinite(number))
        {
            return fallback;
        }

        // AMD-040 §3: a stepped domain repairs any number, so its own rule runs before the
        // "not one of the listed values" test the other enumerated domains apply.
        if (Rule == OutOfDomainRule.Snap)
        {
            return Reshape(value.Kind, Snap(number));
        }

        // AMD-055 REQ-SCN-078: an alias is resolved before the list is consulted, so a written
        // 1 becomes the 4 the setting stores and reads back, and a value that is neither listed
        // nor aliased still takes the default below.
        if (Aliases is not null && Aliases.TryGetValue(number, out double listed))
        {
            number = listed;
        }

        // A listed value that is not in the list takes the default, which is the rule
        // REQ-PERS-003 gives for video.frame_rate, scene.skybox and video.res_scale. A list that
        // clamps instead — the Run-ahead of AMD-041 — says so through its rule and goes on.
        if (Options is not null && Rule == OutOfDomainRule.UseDefault && LabelOf(number) is null)
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

            case OutOfDomainRule.Snap:
            case OutOfDomainRule.Unbounded:
            default:
                break;
        }

        return Reshape(value.Kind, number);
    }

    /// <summary>
    /// The normalisation of AMD-040 §3: into <c>[a, b]</c>, onto the nearest rung — ties to the
    /// even multiple, so 7,5 gives 8 and 6,5 gives 6 — then into <c>[a, b]</c> again.
    /// </summary>
    /// <param name="value">The number to normalise.</param>
    public double Snap(double value)
    {
        if (Step <= 0d)
        {
            return value;
        }

        double low = Minimum ?? double.NegativeInfinity;
        double high = Maximum ?? double.PositiveInfinity;
        double bounded = Math.Clamp(double.IsNaN(value) ? low : value, low, high);

        // Math.Round is banker's rounding: a half lands on the even multiple, which is the tie
        // rule AMD-040 §3 spells out.
        double rungs = Math.Round((bounded - low) / Step, MidpointRounding.ToEven);
        return Math.Clamp(low + (rungs * Step), low, high);
    }

    private static SettingValue Reshape(SettingKind kind, double number) => kind switch
    {
        SettingKind.Bool => SettingValue.OfBool(number != 0d),
        SettingKind.Int => SettingValue.OfInt((int)number),
        SettingKind.Enum => SettingValue.OfEnum((int)number),
        _ => SettingValue.OfFloat((float)number),
    };
}
