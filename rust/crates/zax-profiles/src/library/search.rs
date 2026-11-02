//! The title search of the library (AMD-053 REQ-UI-211): an entry is kept when its title
//! **contains** the typed text, case and accents ignored.
//!
//! The requirement words the comparison as NFD decomposition, the dropping of every mark of
//! Unicode category `Mn`, and a culture-invariant lower-casing. The standard library has neither
//! Unicode normalisation nor general categories, and this workspace adds no crate for them, so
//! the three parts are done with what is here: the combining-mark blocks are dropped by their code
//! ranges, and [`BASE_LETTERS`] supplies the canonical decompositions for the Latin letters, which
//! is what a game title carries. A character NFD does **not** decompose — « ß », « œ », « Ø »,
//! « Đ » — is left alone, which is what NFD would do too.
//!
//! The .NET build met the same wall for a different reason (invariant globalisation makes
//! `String.Normalize` a no-op) and R1 confirmed this table as the answer; Q-090 holds the reasoning
//! and the limit that comes with it: outside the Latin alphabet a precomposed diacritic is not
//! removed. No title of the catalogue is in that case, and every criterion of REQ-UI-211 is on
//! Latin letters.
// Q-090

/// The Latin letters whose canonical decomposition is a base letter plus a mark of category `Mn`:
/// Latin-1 Supplement and Latin Extended-A. Aligned with [`BASE_LETTERS`], character by character.
const PRECOMPOSED: &str = concat!(
    "ÀÁÂÃÄÅÇÈÉÊËÌÍÎÏÑÒÓÔÕÖÙÚÛÜÝ",
    "àáâãäåçèéêëìíîïñòóôõöùúûüýÿ",
    "ĀāĂăĄąĆćĈĉĊċČčĎďĒēĔĕĖėĘęĚěĜĝĞğĠġĢģĤĥĨĩĪīĬĭĮįİĴĵĶķĹĺĻļĽľ",
    "ŃńŅņŇňŌōŎŏŐőŔŕŖŗŘřŚśŜŝŞşŠšŢţŤťŨũŪūŬŭŮůŰűŲųŴŵŶŷŸŹźŻżŽž",
);

/// The base letter each character of [`PRECOMPOSED`] reduces to.
const BASE_LETTERS: &str = concat!(
    "AAAAAACEEEEIIIINOOOOOUUUUY",
    "aaaaaaceeeeiiiinooooouuuuyy",
    "AaAaAaCcCcCcCcDdEeEeEeEeEeGgGgGgGgHhIiIiIiIiIJjKkLlLlLl",
    "NnNnNnOoOoOoRrRrRrSsSsSsSsTtTtUuUuUuUuUuUuWwYyYZzZzZz",
);

/// Whether a character is a combining mark: the Unicode blocks that hold them (`Mn` in practice
/// for the diacritics NFD produces).
fn is_combining_mark(character: char) -> bool {
    matches!(character as u32,
        0x0300..=0x036F      // Combining Diacritical Marks
        | 0x1AB0..=0x1AFF    // ... Extended
        | 0x1DC0..=0x1DFF    // ... Supplement
        | 0x20D0..=0x20F0    // ... for Symbols
        | 0xFE20..=0xFE2F    // Combining Half Marks
    )
}

/// The comparison form of a title or of a typed text (REQ-UI-211).
#[must_use]
pub fn normalize(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    for character in text.chars() {
        if is_combining_mark(character) {
            continue;
        }
        match PRECOMPOSED.chars().position(|known| known == character) {
            Some(at) => kept.extend(BASE_LETTERS.chars().nth(at)),
            None => kept.push(character),
        }
    }
    kept.to_lowercase()
}

/// Whether `title` is kept by `search`. An empty or blank text excludes nothing.
#[must_use]
pub fn matches(title: &str, search: &str) -> bool {
    let needle = normalize(search.trim());
    needle.is_empty() || normalize(title).contains(&needle)
}

/// Whether the two tables line up. A test asserts it, so a letter added to one and forgotten in
/// the other cannot ship.
#[must_use]
pub fn tables_are_aligned() -> bool {
    PRECOMPOSED.chars().count() == BASE_LETTERS.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_tables_line_up() {
        assert!(tables_are_aligned());
    }

    #[test]
    fn a_letter_nfd_does_not_decompose_is_left_alone() {
        // Neither « ß » nor « œ » nor « Ø » nor « Đ » has a canonical decomposition, so NFD
        // would leave them as they are and so does the table; only the case changes.
        assert_eq!("ßœøđ", normalize("ßŒØĐ"));
    }
}
