//! The static check of SPEC-05 REQ-LUA-051: it reads a script, follows every access chain that
//! starts at a global of §4.1 or at a variable whose type it could infer, and resolves each
//! member against the API description the runtime produces.
//!
//! It is deliberately conservative: a value whose type it cannot infer — a function parameter,
//! a Lua table of the script, an arithmetic result — stops the chain and is not checked. That is
//! the same split the specification reports in §5.1 between typed and untyped accesses, and it
//! means the check can only produce *real* findings, never a false alarm on a field of a
//! script's own table.
//!
//! A variable that is assigned once from a typed chain is typed; a variable that is also
//! assigned anything else is poisoned and left untyped, so a reused name cannot mislead it.

use std::collections::{HashMap, HashSet};
use std::fmt;

use zax_scripting::ScriptApiSurface;

/// One member access the resolver could not satisfy (SPEC-05 REQ-LUA-051).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedAccess {
    /// Profile file the script belongs to.
    pub profile: String,
    /// Script name.
    pub script: String,
    /// 1-based line of the access.
    pub line: usize,
    /// The script-visible type the chain had reached.
    pub type_name: String,
    /// The member name that is missing.
    pub member: String,
    /// The separator used: `.` or `:`.
    pub separator: char,
}

impl fmt::Display for UnresolvedAccess {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}/{}:{}: {}{}{}",
            self.profile, self.script, self.line, self.type_name, self.separator, self.member
        )
    }
}

/// The types whose members are table functions, called with `.`, as opposed to instance
/// methods called with `:` (SPEC-05 REQ-LUA-081).
const FUNCTION_TABLES: [&str; 10] = [
    "PaletteAPI",
    "PatternAPI",
    "NotiAPI",
    "Vector3Classe",
    "Vector2Classe",
    "QuaternionClasse",
    "ColorClasse",
    "IntVector2Classe",
    "IntVector3Classe",
    "RectClasse",
];

/// Member names the corpus misspells and that REQ-LUA-042 requires to read as nil, so they are
/// not findings (SPEC-05 §5.1).
const KNOWN_TYPOS: [&str; 2] = ["hiden", "Hiden"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Name,
    Number,
    Text,
    Punctuation,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    text: String,
    line: usize,
}

/// A finding of a walk along a chain.
enum Finding {
    Missing(usize, String, String, char),
    WrongSeparator(usize, String, String),
}

/// Resolves the access chains of the scripts of a profile against the API description.
pub struct LuaAccessResolver<'a> {
    api: &'a ScriptApiSurface,
    unresolved: Vec<UnresolvedAccess>,
    wrong_separator: Vec<UnresolvedAccess>,
    resolved_couples: HashSet<String>,
}

impl<'a> LuaAccessResolver<'a> {
    /// Creates a resolver over `api`.
    pub fn new(api: &'a ScriptApiSurface) -> Self {
        Self {
            api,
            unresolved: Vec::new(),
            wrong_separator: Vec::new(),
            resolved_couples: HashSet::new(),
        }
    }

    /// The accesses that resolved to no member at all.
    pub fn unresolved(&self) -> &[UnresolvedAccess] {
        &self.unresolved
    }

    /// The accesses that used the wrong separator (REQ-LUA-081).
    pub fn wrong_separator(&self) -> &[UnresolvedAccess] {
        &self.wrong_separator
    }

    /// The distinct `(type, member)` couples the resolver satisfied.
    pub fn resolved_couples(&self) -> &HashSet<String> {
        &self.resolved_couples
    }

    /// Checks every script of one profile. The scripts share one variable map, because they
    /// share one global environment (REQ-LUA-010).
    pub fn check(&mut self, profile: &str, scripts: &[(String, String)]) {
        let tokenised: Vec<(&str, Vec<Token>)> = scripts
            .iter()
            .map(|(name, source)| (name.as_str(), tokenise(source)))
            .collect();

        // Pass 1: infer the type of the variables assigned from a typed chain.
        let mut variables: HashMap<String, String> = HashMap::new();
        let mut poisoned: HashSet<String> = HashSet::new();
        for (_, tokens) in &tokenised {
            self.infer_variables(tokens, &mut variables, &mut poisoned);
        }

        for name in &poisoned {
            variables.remove(name);
        }

        // Pass 2: resolve every chain.
        for (name, tokens) in &tokenised {
            self.resolve(profile, name, tokens, &variables);
        }
    }

    fn infer_variables(
        &mut self,
        tokens: &[Token],
        variables: &mut HashMap<String, String>,
        poisoned: &mut HashSet<String>,
    ) {
        for i in 0..tokens.len() {
            // NAME = <expression>, with no comma before the '=' (a multiple assignment is
            // skipped) and no '.' or ':' after the name (that is a member write, not a
            // binding).
            if tokens[i].kind != TokenKind::Name
                || i + 1 >= tokens.len()
                || tokens[i + 1].text != "="
            {
                continue;
            }

            if i > 0 && matches!(tokens[i - 1].text.as_str(), "," | "." | ":") {
                continue;
            }

            if i + 2 < tokens.len() && tokens[i + 2].text == "," {
                // The right-hand side is a list; its parts are not followed here.
                poisoned.insert(tokens[i].text.clone());
                continue;
            }

            match self.follow(tokens, i + 2, variables, None) {
                None => {
                    poisoned.insert(tokens[i].text.clone());
                }
                Some(inferred) => match variables.get(&tokens[i].text) {
                    Some(existing) if *existing != inferred => {
                        poisoned.insert(tokens[i].text.clone());
                    }
                    _ => {
                        variables.insert(tokens[i].text.clone(), inferred);
                    }
                },
            }
        }
    }

    fn resolve(
        &mut self,
        profile: &str,
        script: &str,
        tokens: &[Token],
        variables: &HashMap<String, String>,
    ) {
        for i in 0..tokens.len() {
            if tokens[i].kind != TokenKind::Name {
                continue;
            }

            // Only the head of a chain starts a resolution.
            if i > 0 && matches!(tokens[i - 1].text.as_str(), "." | ":") {
                continue;
            }

            if self.root_type(&tokens[i].text, variables).is_none() {
                continue;
            }

            let mut findings = Vec::new();
            self.follow(tokens, i, variables, Some(&mut findings));
            for finding in findings {
                match finding {
                    Finding::Missing(line, type_name, member, separator) => {
                        self.unresolved.push(UnresolvedAccess {
                            profile: profile.to_owned(),
                            script: script.to_owned(),
                            line,
                            type_name,
                            member,
                            separator,
                        });
                    }
                    Finding::WrongSeparator(line, type_name, member) => {
                        self.wrong_separator.push(UnresolvedAccess {
                            profile: profile.to_owned(),
                            script: script.to_owned(),
                            line,
                            type_name,
                            member,
                            separator: '.',
                        });
                    }
                }
            }
        }
    }

    fn root_type(&self, name: &str, variables: &HashMap<String, String>) -> Option<String> {
        if let Some(global) = self
            .api
            .globals
            .get(name)
            .filter(|global| !global.is_empty())
        {
            return Some(global.clone());
        }

        variables.get(name).cloned()
    }

    /// Follows the chain that starts at `start` and returns the type it ends on, or `None`
    /// when the type is unknown. `report` receives every member that could not be resolved;
    /// passing `None` makes the walk silent.
    fn follow(
        &mut self,
        tokens: &[Token],
        start: usize,
        variables: &HashMap<String, String>,
        mut report: Option<&mut Vec<Finding>>,
    ) -> Option<String> {
        if start >= tokens.len() || tokens[start].kind != TokenKind::Name {
            return None;
        }

        let mut type_name = self.root_type(&tokens[start].text, variables)?;
        let mut i = start + 1;
        while i < tokens.len() {
            let punctuation = tokens[i].text.as_str();
            if punctuation == "." || punctuation == ":" {
                if i + 1 >= tokens.len() || tokens[i + 1].kind != TokenKind::Name {
                    return None;
                }

                let member = tokens[i + 1].text.as_str();
                let called = i + 2 < tokens.len() && tokens[i + 2].text == "(";
                let described = self.api.types.get(&type_name)?;

                let Some(found) = described.members.get(member) else {
                    // hiden and Hiden are misspellings of Hidden present in the corpus; the
                    // specification requires them to read as nil, not to be members
                    // (REQ-LUA-042, §5.1).
                    if !KNOWN_TYPOS.contains(&member) {
                        if let Some(findings) = report.as_deref_mut() {
                            findings.push(Finding::Missing(
                                tokens[i + 1].line,
                                type_name.clone(),
                                member.to_owned(),
                                punctuation.chars().next().unwrap_or('.'),
                            ));
                        }
                    }

                    return None;
                };

                self.resolved_couples
                    .insert(format!("{type_name}.{member}"));

                // A table function is called with '.', an instance method with ':'
                // (REQ-LUA-081). A call through the wrong separator is reported apart.
                if found.callable && called {
                    let wants_dot = FUNCTION_TABLES.contains(&type_name.as_str());
                    if (punctuation == "." && !wants_dot) || (punctuation == ":" && wants_dot) {
                        if let Some(findings) = report.as_deref_mut() {
                            findings.push(Finding::WrongSeparator(
                                tokens[i + 1].line,
                                type_name.clone(),
                                member.to_owned(),
                            ));
                        }
                    }
                }

                if found.callable && !called {
                    // The function value itself; the chain stops.
                    return None;
                }

                if found.result_type.is_empty() {
                    return None;
                }

                type_name.clone_from(&found.result_type);
                i = if called {
                    skip_balanced(tokens, i + 2, "(", ")")
                } else {
                    i + 2
                };
                continue;
            }

            if punctuation == "[" {
                let described = self.api.types.get(&type_name)?;
                if described.element_type.is_empty() {
                    return None;
                }

                type_name.clone_from(&described.element_type);
                i = skip_balanced(tokens, i, "[", "]");
                continue;
            }

            if punctuation == "(" {
                let described = self.api.types.get(&type_name)?;
                if described.call_result.is_empty() {
                    return None;
                }

                type_name.clone_from(&described.call_result);
                i = skip_balanced(tokens, i, "(", ")");
                continue;
            }

            break;
        }

        Some(type_name)
    }
}

fn skip_balanced(tokens: &[Token], open: usize, opening: &str, closing: &str) -> usize {
    let mut depth = 0_i32;
    for (i, token) in tokens.iter().enumerate().skip(open) {
        if token.text == opening {
            depth += 1;
        } else if token.text == closing {
            depth -= 1;
            if depth == 0 {
                return i + 1;
            }
        }
    }

    tokens.len()
}

/// Splits a Lua source into names, numbers, strings and punctuation, dropping comments. Long
/// strings and long comments are handled, which the corpus uses.
fn tokenise(source: &str) -> Vec<Token> {
    let source: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut line = 1_usize;
    let mut i = 0;
    while i < source.len() {
        let c = source[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '-' && source.get(i + 1) == Some(&'-') {
            skip_comment(&source, &mut i, &mut line);
        } else {
            tokens.push(read_token(&source, &mut i, &mut line, c));
        }
    }

    tokens
}

/// Skips a comment: a long one or the rest of the line.
fn skip_comment(source: &[char], i: &mut usize, line: &mut usize) {
    *i += 2;
    if try_long_bracket(source, i, line) {
        return;
    }

    while *i < source.len() && source[*i] != '\n' {
        *i += 1;
    }
}

/// Reads the token that starts with `c` at `i`.
fn read_token(source: &[char], i: &mut usize, line: &mut usize, c: char) -> Token {
    // Long strings.
    if c == '[' && matches!(source.get(*i + 1), Some('[' | '=')) {
        let saved = *i;
        if try_long_bracket(source, i, line) {
            return Token {
                kind: TokenKind::Text,
                text: "<long>".to_owned(),
                line: *line,
            };
        }

        *i = saved;
    }

    if c == '"' || c == '\'' {
        return read_quoted(source, i, line, c);
    }

    if c.is_alphabetic() || c == '_' {
        let begin = *i;
        while *i < source.len() && (source[*i].is_alphanumeric() || source[*i] == '_') {
            *i += 1;
        }

        return Token {
            kind: TokenKind::Name,
            text: source[begin..*i].iter().collect(),
            line: *line,
        };
    }

    if c.is_ascii_digit() {
        let begin = *i;
        while *i < source.len()
            && (source[*i].is_alphanumeric()
                || source[*i] == '.'
                || ((source[*i] == '-' || source[*i] == '+')
                    && (source[*i - 1] == 'e' || source[*i - 1] == 'E')))
        {
            *i += 1;
        }

        return Token {
            kind: TokenKind::Number,
            text: source[begin..*i].iter().collect(),
            line: *line,
        };
    }

    // Two-character operators matter only in so far as they are not '.' or ':'.
    if *i + 1 < source.len() {
        let pair: String = source[*i..*i + 2].iter().collect();
        if matches!(pair.as_str(), "==" | "~=" | "<=" | ">=" | ".." | "::") {
            *i += 2;
            return Token {
                kind: TokenKind::Punctuation,
                text: pair,
                line: *line,
            };
        }
    }

    *i += 1;
    Token {
        kind: TokenKind::Punctuation,
        text: c.to_string(),
        line: *line,
    }
}

/// Reads a quoted string that starts at `i` with the quote `quote`.
fn read_quoted(source: &[char], i: &mut usize, line: &mut usize, quote: char) -> Token {
    *i += 1;
    let mut text = String::new();
    while *i < source.len() && source[*i] != quote {
        if source[*i] == '\\' && *i + 1 < source.len() {
            *i += 2;
            continue;
        }

        if source[*i] == '\n' {
            *line += 1;
        }

        text.push(source[*i]);
        *i += 1;
    }

    *i += 1;
    Token {
        kind: TokenKind::Text,
        text,
        line: *line,
    }
}

/// Consumes a long bracket `[[…]]` or `[=[…]=]` starting at `i`, and says whether there was
/// one.
fn try_long_bracket(source: &[char], i: &mut usize, line: &mut usize) -> bool {
    if source.get(*i) != Some(&'[') {
        return false;
    }

    let mut level = 0;
    let mut probe = *i + 1;
    while source.get(probe) == Some(&'=') {
        level += 1;
        probe += 1;
    }

    if source.get(probe) != Some(&'[') {
        return false;
    }

    let closing: Vec<char> = std::iter::once(']')
        .chain(std::iter::repeat_n('=', level))
        .chain(std::iter::once(']'))
        .collect();
    let end = (probe + 1..source.len()).find(|&at| source[at..].starts_with(&closing));
    let stop = end.map_or(source.len(), |end| end + closing.len());
    *line += source[*i..stop].iter().filter(|&&c| c == '\n').count();
    *i = stop;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_member_and_a_wrong_separator_are_found() {
        let api = ScriptApiSurface::describe();
        let mut resolver = LuaAccessResolver::new(&api);
        resolver.check(
            "p",
            &[(
                "s".to_owned(),
                "local a = Frame:NoSuch()\nlocal b = Vector3.Cross(Vector3.up, Vector3.up)\nlocal c = Frame.GetShape(0)"
                    .to_owned(),
            )],
        );

        let unresolved: Vec<String> = resolver
            .unresolved()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(unresolved, ["p/s:1: FrameAPI:NoSuch"]);
        let wrong: Vec<String> = resolver
            .wrong_separator()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(wrong, ["p/s:3: FrameAPI.GetShape"]);
        assert!(resolver.resolved_couples().contains("Vector3Classe.Cross"));
    }
}
