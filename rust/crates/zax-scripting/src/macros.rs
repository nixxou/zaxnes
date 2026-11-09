//! Zaxnes code hidden in a profile's Lua comments (Rust-only, owner request): a profile stays a
//! 3dSen profile, whose scripts 3dSen runs as they are, comments ignored, and carries code for
//! Zaxnes only — commands 3dSen does not know, lighter effects on a handheld — that Zaxnes
//! reads out of the comments before compiling the script.
//!
//! ```lua
//! --[[#zax handheld
//! local SPIKE_ROW_Z = { 0, 16 }
//! #zax skip-legacy
//! ]]
//! -- #legacy-begin
//! local SPIKE_ROW_Z = { 0, 8, 16 }
//! -- #legacy-end
//! ```
//!
//! * A block opened by a line `--[[#zax <condition>` and closed by a line `]]`: when the
//!   condition holds, its lines are code (its marker lines become blank lines); otherwise it
//!   stays the comment 3dSen sees.
//! * A line `#zax skip-legacy` in a block that holds: the next legacy part — the lines between
//!   `-- #legacy-begin` and `-- #legacy-end` — is left out (blank lines).
//! * A condition is words joined by spaces, all to hold: a host condition (`zaxnes`, always
//!   true here; `handheld` or `desktop`, ...), each one possibly preceded by `not`. An empty
//!   condition holds.
//!
//! The line count never changes, so the line numbers of the Lua messages stay those of the file.

use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

/// Whether the blocks are read at all: off, the scripts run as 3dSen runs them (the editor's
/// "3D 3dSen" view).
static ENABLED: AtomicBool = AtomicBool::new(true);

/// Reads the blocks (true, the default) or leaves the scripts as 3dSen sees them (false).
pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

/// Whether the blocks are read.
#[must_use]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// The host conditions besides `zaxnes`.
#[must_use]
pub fn host_conditions() -> Vec<String> {
    HOST_CONDITIONS
        .read()
        .map(|held| held.clone())
        .unwrap_or_default()
}

/// The host conditions besides `zaxnes`, set once by the program (`handheld` or `desktop`).
static HOST_CONDITIONS: RwLock<Vec<String>> = RwLock::new(Vec::new());

/// Sets the conditions the host meets (`handheld`, `desktop`, ...), besides `zaxnes`.
pub fn set_host_conditions(conditions: &[&str]) {
    if let Ok(mut held) = HOST_CONDITIONS.write() {
        *held = conditions
            .iter()
            .map(|&condition| condition.to_owned())
            .collect();
    }
}

/// Whether the host meets `condition`.
fn holds(condition: &str, host: &[String]) -> bool {
    condition == "zaxnes" || host.iter().any(|held| held == condition)
}

/// Whether every word of `text` holds (`not` negating the word after it).
fn condition_holds(text: &str, host: &[String]) -> bool {
    let mut words = text.split_whitespace();
    while let Some(word) = words.next() {
        let value = if word == "not" {
            match words.next() {
                Some(next) => !holds(next, host),
                None => return false,
            }
        } else {
            holds(word, host)
        };
        if !value {
            return false;
        }
    }
    true
}

/// `source` with its Zaxnes blocks read for the host's conditions; borrowed when it has none.
#[must_use]
pub fn expand(source: &str) -> Cow<'_, str> {
    if !enabled() || !source.contains("#zax") {
        return Cow::Borrowed(source);
    }
    let host = HOST_CONDITIONS
        .read()
        .map(|held| held.clone())
        .unwrap_or_default();
    Cow::Owned(expand_for(source, &host))
}

/// [`expand`] for the host conditions `host`.
#[must_use]
pub fn expand_for(source: &str, host: &[String]) -> String {
    #[derive(PartialEq)]
    enum State {
        Code,
        /// In a block whose condition holds.
        Block,
        /// In a block whose condition does not hold: left as it is.
        CommentedBlock,
        /// In a legacy part to leave out.
        SkippedLegacy,
    }
    let mut state = State::Code;
    let mut skip_next_legacy = false;
    let mut out = String::with_capacity(source.len());
    for line in source.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        let ending = &line[body.len()..];
        let trimmed = body.trim();
        let keep = match state {
            State::Code => {
                if let Some(condition) = trimmed.strip_prefix("--[[#zax") {
                    if condition_holds(condition, host) {
                        state = State::Block;
                        false
                    } else {
                        state = State::CommentedBlock;
                        true
                    }
                } else if trimmed == "-- #legacy-begin" && skip_next_legacy {
                    skip_next_legacy = false;
                    state = State::SkippedLegacy;
                    true
                } else {
                    true
                }
            }
            State::Block => {
                if trimmed.starts_with("]]") {
                    state = State::Code;
                    false
                } else if trimmed == "#zax skip-legacy" {
                    skip_next_legacy = true;
                    false
                } else {
                    true
                }
            }
            State::CommentedBlock => {
                if trimmed.starts_with("]]") {
                    state = State::Code;
                }
                true
            }
            State::SkippedLegacy => {
                if trimmed == "-- #legacy-end" {
                    state = State::Code;
                    true
                } else {
                    false
                }
            }
        };
        if keep {
            out.push_str(line);
        } else {
            out.push_str(ending);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCRIPT: &str = "a = 1\n--[[#zax handheld\nb = 2\n#zax skip-legacy\n]]\n\
                          -- #legacy-begin\nb = 3\n-- #legacy-end\nc = 4\n";

    fn host(conditions: &[&str]) -> Vec<String> {
        conditions.iter().map(|&c| c.to_owned()).collect()
    }

    #[test]
    fn a_block_that_holds_is_code_and_leaves_the_legacy_part_out() {
        let expanded = expand_for(SCRIPT, &host(&["handheld"]));
        assert_eq!(
            expanded,
            "a = 1\n\nb = 2\n\n\n-- #legacy-begin\n\n-- #legacy-end\nc = 4\n"
        );
        assert_eq!(expanded.lines().count(), SCRIPT.lines().count());
    }

    #[test]
    fn a_block_that_does_not_hold_stays_the_comment_3dsen_sees() {
        assert_eq!(expand_for(SCRIPT, &host(&["desktop"])), SCRIPT);
    }

    #[test]
    fn conditions_combine_and_negate() {
        let desktop = host(&["desktop"]);
        assert!(condition_holds("", &desktop));
        assert!(condition_holds(" zaxnes", &desktop));
        assert!(condition_holds("zaxnes not handheld", &desktop));
        assert!(!condition_holds("zaxnes handheld", &desktop));
        assert!(!condition_holds("not", &desktop));
    }

    #[test]
    fn a_script_without_blocks_is_borrowed() {
        assert!(matches!(expand("a = 1\n-- a comment\n"), Cow::Borrowed(_)));
    }
}
