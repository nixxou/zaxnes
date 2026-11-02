//! The domain of a setting, the column "Domaine" of SPEC-09 §4.1, and the rule that brings a value
//! back into it (REQ-PERS-003).

use crate::num;
use crate::values::SettingValue;

/// What to do with a value that falls outside the domain (REQ-PERS-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutOfDomainRule {
    /// No rule given by the column "Domaine": the default is used, as REQ-PERS-003 requires.
    UseDefault,
    /// Bring the value back to the nearest bound.
    Clamp,
    /// Take the absolute value (`conv.sprite_tolerance`, rule 4).
    AbsoluteValue,
    /// Remainder of the division by the number of available cores (`emu.core`, rule 2).
    ModuloCoreCount,
    /// Any value is accepted: the setting has no domain.
    Unbounded,
    /// Bring the value into the bounds, then onto the nearest rung of the step (AMD-040 §3).
    Snap,
}

/// One admitted value of an enumerated domain, with the label the file and libretro use
/// (REQ-PERS-081).
#[derive(Debug, Clone, PartialEq)]
pub struct SettingOption {
    /// Numeric value, which Lua accepts as well as the named constant (REQ-PERS-004).
    pub value: f64,
    /// Stable label, written to the settings file and shown by libretro.
    pub label: String,
}

impl SettingOption {
    /// Creates an option.
    #[must_use]
    pub fn new(value: f64, label: &str) -> Self {
        Self {
            value,
            label: label.to_owned(),
        }
    }
}

/// The admitted values of a setting and the rule for the others.
#[derive(Debug, Clone, PartialEq)]
pub struct Domain {
    rule: OutOfDomainRule,
    minimum: Option<f64>,
    maximum: Option<f64>,
    minimum_exclusive: bool,
    options: Option<Vec<SettingOption>>,
    /// The distance between two admitted values of a stepped domain (AMD-040 §3).
    step: Option<f64>,
    /// Values a write may use for another: the identifier 1 of the Enhanced style stands for 4
    /// (AMD-055 REQ-SCN-078). Applied before the value is checked against the domain.
    aliases: Vec<(f64, f64)>,
}

impl Domain {
    /// A domain that accepts anything.
    #[must_use]
    pub fn unbounded() -> Self {
        Self::with_rule(OutOfDomainRule::Unbounded)
    }

    /// A closed interval; a value outside it is clamped.
    #[must_use]
    pub fn range(minimum: f64, maximum: f64) -> Self {
        Self {
            minimum: Some(minimum),
            maximum: Some(maximum),
            ..Self::with_rule(OutOfDomainRule::Clamp)
        }
    }

    /// A half-open interval `(minimum, ∞)`; a value outside it takes the default.
    #[must_use]
    pub fn greater_than(minimum: f64) -> Self {
        Self {
            minimum: Some(minimum),
            minimum_exclusive: true,
            ..Self::with_rule(OutOfDomainRule::UseDefault)
        }
    }

    /// The same domain with its published values labelled: `labels[i]` names the `i`-th rung, in
    /// order, and a rung the list does not reach keeps the label it had.
    ///
    /// For a stepped domain whose values the interface shows as a list rather than as numbers —
    /// `emu.run_ahead` and the four labels of AMD-041 §3. It changes **only** what the interface
    /// writes beside each value: the bounds, the step and the `Snap` rule of
    /// [`Domain::stepped`] are untouched, which is what keeps a write of 7 at 3 and of −1 at 0
    /// (REQ-LOOP-064) where [`Domain::choice`] would send both to the default.
    #[must_use]
    pub fn with_labels(mut self, labels: &[&str]) -> Self {
        if let Some(options) = self.options.as_mut() {
            for (option, label) in options.iter_mut().zip(labels) {
                (*label).clone_into(&mut option.label);
            }
        }
        self
    }

    /// A finite set of values, each with its label. A value outside it takes the default.
    #[must_use]
    pub fn choice(options: Vec<SettingOption>) -> Self {
        Self {
            options: Some(options),
            ..Self::with_rule(OutOfDomainRule::UseDefault)
        }
    }

    /// The rungs `minimum`, `minimum + step`, … up to `maximum`, each labelled with `decimals`
    /// decimals (REQ-PERS-081). A value is brought into the bounds and then onto the nearest
    /// rung, ties going to the even one — the normalisation of AMD-040 §3, which AMD-043
    /// REQ-SCN-066 and AMD-055 REQ-SCN-079 both ask for.
    #[must_use]
    pub fn stepped(minimum: f64, maximum: f64, step: f64, decimals: usize) -> Self {
        let count = num::round_to_i32((maximum - minimum) / step).max(0) + 1;
        let options = (0..count)
            .map(|index| {
                let value = rung(minimum, step, f64::from(index));
                SettingOption::new(value, &format!("{value:.decimals$}"))
            })
            .collect();
        Self {
            minimum: Some(minimum),
            maximum: Some(maximum),
            options: Some(options),
            step: Some(step),
            ..Self::with_rule(OutOfDomainRule::Snap)
        }
    }

    /// A finite set of values of which some are written another way: `aliases` maps a value a
    /// write may use onto the one the model keeps (the style identifier 1 onto 4, AMD-055
    /// REQ-SCN-078). A value that is neither an option nor an alias takes the default.
    #[must_use]
    pub fn aliased_choice(aliases: Vec<(f64, f64)>, options: Vec<SettingOption>) -> Self {
        Self {
            aliases,
            ..Self::choice(options)
        }
    }

    /// The two booleans, labelled as libretro expects them (REQ-PERS-083).
    #[must_use]
    pub fn switch() -> Self {
        Self {
            options: Some(vec![
                SettingOption::new(0.0, "disabled"),
                SettingOption::new(1.0, "enabled"),
            ]),
            ..Self::with_rule(OutOfDomainRule::Unbounded)
        }
    }

    /// A whole number taken modulo the number of available cores (rule 2).
    #[must_use]
    pub fn core_index() -> Self {
        Self::with_rule(OutOfDomainRule::ModuloCoreCount)
    }

    /// A number whose sign is dropped (rule 4).
    #[must_use]
    pub fn non_negative_by_absolute_value() -> Self {
        Self::with_rule(OutOfDomainRule::AbsoluteValue)
    }

    /// A vector that the domain "non nul" refuses to be zero.
    #[must_use]
    pub fn non_zero_vector() -> Self {
        Self::greater_than(0.0)
    }

    /// The lower bound, when the domain has one.
    #[must_use]
    pub const fn minimum(&self) -> Option<f64> {
        self.minimum
    }

    /// The upper bound, when the domain has one.
    #[must_use]
    pub const fn maximum(&self) -> Option<f64> {
        self.maximum
    }

    /// The finite ordered set of admitted values, or none when there is none.
    #[must_use]
    pub fn options(&self) -> Option<&[SettingOption]> {
        self.options.as_deref()
    }

    /// The distance between two admitted values, for a stepped domain.
    #[must_use]
    pub const fn step(&self) -> Option<f64> {
        self.step
    }

    /// `value` normalised as AMD-040 §3 requires: brought into the bounds, then replaced by
    /// `a + n × step` with `n` the integer nearest `(value − a) / step` — the **even** one when
    /// two are equally near — then brought into the bounds again. A domain without a step
    /// answers its own bounds, or `value` when it has none.
    ///
    /// This is the one place the rounding of a style parameter happens, whoever writes: a Lua
    /// call, the options window or the settings file (AMD-040 REQ-LUA-114).
    #[must_use]
    pub fn snap(&self, value: f64) -> f64 {
        let bounded = self.bounded(value);
        let (Some(low), Some(step)) = (self.minimum, self.step) else {
            return bounded;
        };
        if step <= 0.0 || step.is_nan() || !bounded.is_finite() {
            return bounded;
        }
        self.bounded(rung(low, step, ((bounded - low) / step).round_ties_even()))
    }

    fn bounded(&self, value: f64) -> f64 {
        let mut bounded = value;
        if let Some(low) = self.minimum {
            if bounded < low {
                bounded = low;
            }
        }
        if let Some(high) = self.maximum {
            if bounded > high {
                bounded = high;
            }
        }
        bounded
    }

    /// `value` with the aliases of the domain applied (REQ-SCN-078).
    #[must_use]
    pub fn canonical(&self, value: f64) -> f64 {
        self.aliases
            .iter()
            .find(|(from, _)| same_number(*from, value))
            .map_or(value, |(_, to)| *to)
    }

    /// The label of `value` in this domain, or none.
    #[must_use]
    pub fn label_of(&self, value: f64) -> Option<&str> {
        self.options
            .as_deref()?
            .iter()
            .find(|option| same_number(option.value, value))
            .map(|option| option.label.as_str())
    }

    /// The value `label` designates, or none. Labels are compared ignoring ASCII case.
    #[must_use]
    pub fn value_of(&self, label: &str) -> Option<f64> {
        self.options
            .as_deref()?
            .iter()
            .find(|option| option.label.eq_ignore_ascii_case(label))
            .map(|option| option.value)
    }

    /// Brings `value` into the domain (REQ-PERS-003). A value that no rule can repair becomes
    /// `fallback`, the setting's default.
    #[must_use]
    pub fn coerce(
        &self,
        value: &SettingValue,
        fallback: &SettingValue,
        core_count: i32,
    ) -> SettingValue {
        if let SettingValue::Vector3(vector) = value {
            return if self.rule == OutOfDomainRule::UseDefault && vector.is_zero() {
                fallback.clone()
            } else {
                value.clone()
            };
        }
        if matches!(
            value,
            SettingValue::Color(_)
                | SettingValue::Text(_)
                | SettingValue::Rect(_)
                | SettingValue::Opaque(_)
        ) {
            return value.clone();
        }
        let mut number = self.canonical(value.as_double());
        // A stepped domain publishes its rungs as options for REQ-PERS-081, but a value between
        // two of them is snapped, not refused (AMD-040 §3).
        if self.options.is_some()
            && !matches!(
                self.rule,
                OutOfDomainRule::Unbounded | OutOfDomainRule::Snap
            )
            && self.label_of(number).is_none()
        {
            return fallback.clone();
        }
        match self.rule {
            OutOfDomainRule::Snap => number = self.snap(number),
            OutOfDomainRule::Clamp => {
                if let Some(low) = self.minimum {
                    if number < low {
                        number = low;
                    }
                }
                if let Some(high) = self.maximum {
                    if number > high {
                        number = high;
                    }
                }
            }
            OutOfDomainRule::AbsoluteValue => number = number.abs(),
            OutOfDomainRule::ModuloCoreCount => {
                let cores = core_count.max(1);
                number = f64::from(num::truncate_to_i32(number).rem_euclid(cores));
            }
            OutOfDomainRule::UseDefault => {
                let below = self.minimum.is_some_and(|bound| {
                    if self.minimum_exclusive {
                        number <= bound
                    } else {
                        number < bound
                    }
                });
                let above = self.maximum.is_some_and(|ceiling| number > ceiling);
                if below || above {
                    return fallback.clone();
                }
            }
            OutOfDomainRule::Unbounded => {}
        }
        match value {
            SettingValue::Bool(_) => SettingValue::Bool(number != 0.0),
            SettingValue::Int(_) => SettingValue::Int(num::truncate_to_i32(number)),
            SettingValue::Enum(_) => SettingValue::Enum(num::truncate_to_i32(number)),
            _ => SettingValue::Float(num::to_f32(number)),
        }
    }

    fn with_rule(rule: OutOfDomainRule) -> Self {
        Self {
            rule,
            minimum: None,
            maximum: None,
            minimum_exclusive: false,
            options: None,
            step: None,
            aliases: Vec::new(),
        }
    }
}

/// `minimum + index × step`, rounded to the millionth as the discretisation of REQ-PERS-081
/// writes it, so that a rung reads back exactly as the option list publishes it.
fn rung(minimum: f64, step: f64, index: f64) -> f64 {
    ((minimum + index * step) * 1e6).round_ties_even() / 1e6
}

/// Exact equality of two discrete option values, as the C# `Equals` (NaN equals NaN).
#[allow(
    clippy::float_cmp,
    reason = "options are discrete values compared exactly, as the C# Equals"
)]
fn same_number(a: f64, b: f64) -> bool {
    a == b || (a.is_nan() && b.is_nan())
}
