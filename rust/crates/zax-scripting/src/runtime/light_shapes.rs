//! Which `UpdateS` hooks may receive a light `shape` (Rust-only performance work): read once,
//! from the scripts' source, when the profile loads.
//!
//! A hook call normally publishes the `shape` and `shape2D` of its instance as the handles of
//! REQ-LUA-043: the same instance gives the same Lua value for the whole frame, and the value is
//! kept among the retired handles once the frame is over, for a script that holds it. Making and
//! keeping them is most of the cost of a call. A hook that only reads and writes fields of
//! `shape` can tell none of that apart: it never holds the value, compares it or hands it to
//! other code. Such a hook receives a light `shape`, a value of the same handle made for the call
//! and nothing more, and no `shape2D`, which it never reads.
//!
//! The reading is deliberately narrow, and anything it does not recognise keeps the normal path:
//! - the profile as a whole must not touch the language's own mechanics (metatables, raw access,
//!   the global table, chunk loading, the debug library) anywhere, so that no Lua code can run
//!   behind an operation of the hook;
//! - the script must define `UpdateS` exactly once, as `function UpdateS()`, and name it nowhere
//!   else;
//! - in its body, `shape` and `shape2D` only appear as `shape.Field` (`shape2D` not at all, for
//!   the light path skips it), no function is defined, no method is called, and the only calls
//!   are those of the `math` library.

/// A token of Lua source, as far as the reading needs one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token<'a> {
    /// A name or a keyword.
    Name(&'a str),
    /// A string literal, with its delimiters.
    Text(&'a str),
    Number,
    Symbol(&'static str),
}

const KEYWORDS: [&str; 22] = [
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

/// The names that reach the language's own mechanics: none may appear in a profile whose hooks
/// take the light path.
const MECHANICS: [&str; 14] = [
    "setmetatable",
    "getmetatable",
    "rawset",
    "rawget",
    "rawequal",
    "_G",
    "_ENV",
    "load",
    "loadstring",
    "dofile",
    "require",
    "debug",
    "getfenv",
    "setfenv",
];

const SYMBOLS: [&str; 33] = [
    "...", "..", "==", "~=", "<=", ">=", "//", "::", "<<", ">>", "+", "-", "*", "/", "%", "^", "#",
    "&", "~", "|", "<", ">", "=", "(", ")", "{", "}", "[", "]", ";", ":", ",", ".",
];

/// The tokens of `source`, or `None` for anything the reading does not understand.
fn tokenize(source: &str) -> Option<Vec<Token<'_>>> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte.is_ascii_whitespace() {
            at += 1;
        } else if source[at..].starts_with("--") {
            at += 2;
            at = match long_bracket(bytes, at) {
                Some(level) => close_long_bracket(bytes, at, level)?,
                None => bytes[at..]
                    .iter()
                    .position(|&byte| byte == b'\n')
                    .map_or(bytes.len(), |end| at + end),
            };
        } else if let Some(level) = long_bracket(bytes, at) {
            let end = close_long_bracket(bytes, at, level)?;
            tokens.push(Token::Text(&source[at..end]));
            at = end;
        } else if byte == b'"' || byte == b'\'' {
            let start = at;
            at += 1;
            loop {
                match *bytes.get(at)? {
                    b'\\' => at += 2,
                    b'\n' => return None,
                    quote if quote == byte => break,
                    _ => at += 1,
                }
            }
            at += 1;
            tokens.push(Token::Text(source.get(start..at)?));
        } else if byte.is_ascii_digit()
            || (byte == b'.' && bytes.get(at + 1).is_some_and(u8::is_ascii_digit))
        {
            at += 1;
            while let Some(&next) = bytes.get(at) {
                let exponent = matches!(bytes[at - 1], b'e' | b'E' | b'p' | b'P')
                    && matches!(next, b'+' | b'-');
                if next.is_ascii_alphanumeric() || next == b'.' || next == b'_' || exponent {
                    at += 1;
                } else {
                    break;
                }
            }
            tokens.push(Token::Number);
        } else if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = at;
            while bytes
                .get(at)
                .is_some_and(|&next| next.is_ascii_alphanumeric() || next == b'_')
            {
                at += 1;
            }
            tokens.push(Token::Name(&source[start..at]));
        } else {
            let symbol = SYMBOLS
                .iter()
                .find(|symbol| source[at..].starts_with(**symbol))?;
            at += symbol.len();
            tokens.push(Token::Symbol(symbol));
        }
    }
    Some(tokens)
}

/// The level of the long bracket (`[[`, `[=[`...) that opens at `at`, if one does.
fn long_bracket(bytes: &[u8], at: usize) -> Option<usize> {
    if bytes.get(at) != Some(&b'[') {
        return None;
    }
    let level = bytes[at + 1..]
        .iter()
        .take_while(|&&byte| byte == b'=')
        .count();
    (bytes.get(at + 1 + level) == Some(&b'[')).then_some(level)
}

/// Where the long bracket of `level` opened at `at` closes, past its `]=]`.
fn close_long_bracket(bytes: &[u8], at: usize, level: usize) -> Option<usize> {
    let mut closing = vec![b']'];
    closing.extend(std::iter::repeat_n(b'=', level));
    closing.push(b']');
    let from = at + level + 2;
    bytes[from..]
        .windows(closing.len())
        .position(|window| window == closing.as_slice())
        .map(|position| from + position + closing.len())
}

/// Whether the profile whose script sources are `sources` leaves the language's mechanics alone:
/// every source is understood, and none names a mechanism or a metamethod (`__` anything).
#[must_use]
pub fn profile_allows_light_shapes<'a>(sources: impl IntoIterator<Item = &'a str>) -> bool {
    sources.into_iter().all(|source| {
        tokenize(source).is_some_and(|tokens| {
            tokens.iter().all(|token| match token {
                Token::Name(name) => !MECHANICS.contains(name) && !name.starts_with("__"),
                Token::Text(text) => !text.contains("__"),
                Token::Number | Token::Symbol(_) => true,
            })
        })
    })
}

/// Whether the `UpdateS` of the script whose source is `source` may receive a light `shape` (the
/// profile's own condition, [`profile_allows_light_shapes`], apart).
#[must_use]
pub fn update_s_takes_light_shape(source: &str) -> bool {
    let Some(tokens) = tokenize(source) else {
        return false;
    };
    let mut named = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| **token == Token::Name("UpdateS"));
    let (Some((at, _)), None) = (named.next(), named.next()) else {
        return false;
    };
    // `function UpdateS()`, at the start of a statement (not `local function`).
    let header = at
        .checked_sub(1)
        .and_then(|before| tokens.get(before..at + 3));
    if header
        != Some(&[
            Token::Name("function"),
            Token::Name("UpdateS"),
            Token::Symbol("("),
            Token::Symbol(")"),
        ])
    {
        return false;
    }
    if at >= 2 && tokens[at - 2] == Token::Name("local") {
        return false;
    }

    let Some(body) = function_body(&tokens[at + 3..]) else {
        return false;
    };
    body_is_light(body)
}

/// The tokens of a function body that starts at `tokens`, up to its `end`.
fn function_body<'t, 'a>(tokens: &'t [Token<'a>]) -> Option<&'t [Token<'a>]> {
    let mut depth = 1_usize;
    for (position, token) in tokens.iter().enumerate() {
        match token {
            Token::Name("function" | "if" | "do" | "repeat") => depth += 1,
            Token::Name("end" | "until") => {
                depth -= 1;
                if depth == 0 {
                    return Some(&tokens[..position]);
                }
            }
            _ => {}
        }
    }
    None
}

/// The body rules of the module documentation.
fn body_is_light(body: &[Token<'_>]) -> bool {
    for (position, token) in body.iter().enumerate() {
        let before = position.checked_sub(1).map(|before| body[before]);
        let after = body.get(position + 1).copied();
        let field_of_something = matches!(before, Some(Token::Symbol("." | ":")));
        match token {
            Token::Name("function" | "_G" | "_ENV" | "shape2D") if !field_of_something => {
                return false;
            }
            // `shape` only as `shape.Field`.
            Token::Name("shape") if !field_of_something => {
                if after != Some(Token::Symbol("."))
                    || !matches!(body.get(position + 2), Some(Token::Name(_)))
                {
                    return false;
                }
            }
            // No method call.
            Token::Symbol(":") => return false,
            // A call: `f(...)`, `f{...}` or `f"..."` after an expression; only `math.name(...)`.
            Token::Symbol("(" | "{") | Token::Text(_) if is_callee(before) => {
                let callee = position.checked_sub(3).map(|start| &body[start..position]);
                let global_math = position
                    .checked_sub(4)
                    .is_none_or(|start| !matches!(body[start], Token::Symbol("." | ":")));
                let is_math = matches!(
                    callee,
                    Some([Token::Name("math"), Token::Symbol("."), Token::Name(_)])
                );
                if !(is_math && global_math && *token == Token::Symbol("(")) {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

/// Whether a call can follow `token`: an expression ends there (a name that is not a keyword,
/// a closing bracket).
fn is_callee(token: Option<Token<'_>>) -> bool {
    match token {
        Some(Token::Name(name)) => !KEYWORDS.contains(&name),
        Some(Token::Symbol(")" | "]")) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIVOT: &str = "function UpdateS()\r\n\t--if shape.TopRight.x < shape.size.x then\r\n\
        \tif shape.TopRight.y < shape.size.y + Setting.Clipping[1] then\r\n\
        \t\t--print(shape.TopRight.y)\r\n\
        \t\tshape.Pivot.y = ((shape.TopRight.y - 16) - shape.BottomLeft.y) / shape.size.y\r\n\
        \tend\r\nend";

    #[test]
    fn a_hook_that_only_reads_and_writes_fields_takes_the_light_shape() {
        assert!(update_s_takes_light_shape(PIVOT));
        assert!(update_s_takes_light_shape(
            "local k = 2\nfunction Start() end\nfunction UpdateS()\n\
             for i = 1, 3 do shape.Offset.z = shape.Offset.z - math.floor(k * i) end\n\
             if scene == 4 then shape.Scale.z = 1/2 elseif x then repeat k = k - 1 until k < 0 end\n\
             end\nfunction LateUpdate() print(shape) end"
        ));
    }

    #[test]
    fn a_hook_that_could_hold_compare_or_pass_shape_keeps_the_normal_path() {
        for body in [
            "slot.shape = shape",
            "local s = shape",
            "if shape == other then end",
            "Connect(shape, 1)",
            "print(shape.TopRight.y)",
            "shape:SetColor(1)",
            "local x = shape2D.size.x",
            "local f = function() return shape.size end",
            "t[shape] = 1",
            "local v = shape['size']",
            "for k, shape in pairs(t) do end",
            "local c = _G.shape.size",
            "x = math.floor(shape.size.x) .. tostring(1)",
            "x = string.format('%d', 1)",
            "f{1}",
            "f'text'",
            "x = y.math.floor(1)",
        ] {
            let source = format!("function UpdateS()\n{body}\nend");
            assert!(!update_s_takes_light_shape(&source), "{body}");
        }
    }

    #[test]
    fn the_hook_must_be_defined_once_and_named_nowhere_else() {
        assert!(!update_s_takes_light_shape("function Update() end"));
        assert!(!update_s_takes_light_shape(
            "function UpdateS() end\nfunction UpdateS() end"
        ));
        assert!(!update_s_takes_light_shape(
            "function UpdateS() end\nfunction Start() UpdateS = nil end"
        ));
        assert!(!update_s_takes_light_shape("local function UpdateS() end"));
        assert!(!update_s_takes_light_shape("UpdateS = function() end"));
        assert!(!update_s_takes_light_shape("function UpdateS(a) end"));
        assert!(!update_s_takes_light_shape("function t.UpdateS() end"));
    }

    #[test]
    fn strings_and_comments_hide_nothing_and_mislead_nothing() {
        // `shape` inside strings and comments is not a use; the code around it is still read.
        assert!(update_s_takes_light_shape(
            "function UpdateS()\nlocal a = \"shape\" .. 'shape2D'\n--[==[ shape ]==] local b = [[x]]\nend"
        ));
        assert!(!update_s_takes_light_shape(
            "function UpdateS()\n--[[ ]] local s = shape\nend"
        ));
        // What the reading does not understand keeps the normal path.
        assert!(!update_s_takes_light_shape(
            "function UpdateS()\nlocal a = \"open\nend"
        ));
        assert!(!update_s_takes_light_shape(
            "function UpdateS()\n--[[ never closed\nend"
        ));
        assert!(!update_s_takes_light_shape(
            "function UpdateS() local a = 1 @ 2 end"
        ));
    }

    #[test]
    fn a_profile_that_touches_the_language_mechanics_takes_no_light_shape() {
        assert!(profile_allows_light_shapes([
            PIVOT,
            "function Start() x = 1 end"
        ]));
        for source in [
            "setmetatable(t, m)",
            "local m = { __index = f }",
            "t['__add'] = f",
            "rawset(t, 1, 2)",
            "local g = _G",
            "load('x = 1')()",
            "debug.sethook()",
            "local s = 'unclosed",
        ] {
            assert!(!profile_allows_light_shapes([PIVOT, source]), "{source}");
        }
    }
}
