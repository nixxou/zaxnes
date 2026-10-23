//! Escaping of the strings the harness writes into hand-built JSON.

use std::fmt::Write as _;

/// Escapes `value` for the inside of a JSON string literal. Backslash and double quote are
/// escaped as `JsonWriter.cs` does, so ordinary text stays byte-identical to the C# harness;
/// control characters, which C# would leave raw and so write invalid JSON, use `\n`, `\r`, `\t`
/// or `\u00XX`. Other characters, non-ASCII included, are written as they are.
#[must_use]
pub(crate) fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control.is_control() && u32::from(control) < 0x20 => {
                // Writing to a `String` cannot fail.
                let _ = write!(out, "\\u{:04x}", u32::from(control));
            }
            other => out.push(other),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::escape;

    #[test]
    fn a_windows_path_with_quotes_and_a_control_character_parses_back() {
        let path = "C:\\Users\\me\\\"Zax\" profile\u{1}\n\té";
        let text = format!("{{ \"description\": \"{}\" }}", escape(path));
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(value["description"], path);
    }

    #[test]
    fn ordinary_text_is_unchanged() {
        assert_eq!(escape("scene one / é"), "scene one / é");
    }
}
