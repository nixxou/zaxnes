//! The order of a profile's scripts (SPEC-03 REQ-PROF-024, SPEC-05 REQ-LUA-004).
//!
//! Punctuation sorts before digits, which sort before letters; inside each class the comparison
//! is case-insensitive; a shorter name comes first when the primary keys agree; case decides
//! last, lowercase first. The order is total and deterministic.

use std::cmp::Ordering;

/// Compares two script names; an absent name sorts first.
#[must_use]
pub fn compare_names(left: Option<&str>, right: Option<&str>) -> Ordering {
    match (left, right) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(left), Some(right)) => compare(left, right),
    }
}

/// Compares two names over their UTF-16 code units, as the reference order is defined.
fn compare(left: &str, right: &str) -> Ordering {
    let left: Vec<u16> = left.encode_utf16().collect();
    let right: Vec<u16> = right.encode_utf16().collect();

    let primary = left
        .iter()
        .zip(&right)
        .map(|(a, b)| compare_primary(*a, *b))
        .find(|ordering| ordering.is_ne());
    if let Some(ordering) = primary {
        return ordering;
    }
    if left.len() != right.len() {
        return left.len().cmp(&right.len());
    }
    left.iter()
        .zip(&right)
        .find(|(a, b)| a != b)
        .map_or(Ordering::Equal, |(a, b)| {
            let (lower_a, lower_b) = (is_lower(*a), is_lower(*b));
            if lower_a == lower_b {
                // Two distinct characters with the same lowercase and the same case class: no
                // rule applies, so fall back to the code unit to keep the order total.
                a.cmp(b)
            } else if lower_a {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        })
}

fn compare_primary(left: u16, right: u16) -> Ordering {
    class_of(left)
        .cmp(&class_of(right))
        .then_with(|| to_lower(left).cmp(&to_lower(right)))
}

/// 0 punctuation and anything else, 1 digit, 2 letter.
fn class_of(unit: u16) -> u8 {
    match char::from_u32(u32::from(unit)) {
        // `is_numeric` is Unicode Nd, Nl and No where .NET's `IsDigit` is Nd only; the
        // difference is letter-like numerals, which a script name does not contain.
        Some(c) if c.is_numeric() => 1,
        Some(c) if c.is_alphabetic() => 2,
        _ => 0,
    }
}

fn is_lower(unit: u16) -> bool {
    char::from_u32(u32::from(unit)).is_some_and(char::is_lowercase)
}

/// The invariant lowercase of a code unit; unchanged when it has no single-unit lowercase.
fn to_lower(unit: u16) -> u16 {
    let Some(c) = char::from_u32(u32::from(unit)) else {
        return unit;
    };
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(single), None) => u16::try_from(u32::from(single)).unwrap_or(unit),
        _ => unit,
    }
}
