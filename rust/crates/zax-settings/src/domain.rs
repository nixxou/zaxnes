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

    /// A finite set of values, each with its label. A value outside it takes the default.
    #[must_use]
    pub fn choice(options: Vec<SettingOption>) -> Self {
        Self {
            options: Some(options),
            ..Self::with_rule(OutOfDomainRule::UseDefault)
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

    /// The finite ordered set of admitted values, or none when there is none.
    #[must_use]
    pub fn options(&self) -> Option<&[SettingOption]> {
        self.options.as_deref()
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
        let mut number = value.as_double();
        if self.options.is_some()
            && self.rule != OutOfDomainRule::Unbounded
            && self.label_of(number).is_none()
        {
            return fallback.clone();
        }
        match self.rule {
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
        }
    }
}

/// Exact equality of two discrete option values, as the C# `Equals` (NaN equals NaN).
#[allow(
    clippy::float_cmp,
    reason = "options are discrete values compared exactly, as the C# Equals"
)]
fn same_number(a: f64, b: f64) -> bool {
    a == b || (a.is_nan() && b.is_nan())
}
