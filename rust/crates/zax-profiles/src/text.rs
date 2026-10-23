//! String comparisons with the semantics of the reference implementation: ordinal over UTF-16
//! code units, optionally after the simple (one-to-one) upper-casing of each character.

use std::cmp::Ordering;

/// Ordinal comparison over UTF-16 code units.
pub(crate) fn ordinal(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

/// Ordinal comparison after the invariant simple upper-casing of every character.
pub(crate) fn ordinal_ignore_case(left: &str, right: &str) -> Ordering {
    ordinal(&simple_upper(left), &simple_upper(right))
}

fn simple_upper(text: &str) -> String {
    text.chars()
        .map(|c| {
            let mut upper = c.to_uppercase();
            match (upper.next(), upper.next()) {
                (Some(single), None) => single,
                _ => c,
            }
        })
        .collect()
}
