//! `lua-api [--out <file>]`: dumps the script-visible API the Lua runtime installs, so that the
//! surface can be compared line by line with the tables of SPEC-05 §4. Port of
//! `LuaApiExport.cs`.
//!
//! The dump is read back from the very tables the runtime binds ([`ScriptApiSurface`]), so it
//! cannot drift from what a script actually sees. That is what makes the coverage of
//! REQ-LUA-080 and REQ-LUA-051 checkable from the clean side (DEC-R2-T7).

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use zax_scripting::ScriptApiSurface;

use crate::HarnessError;

/// The help text of the command.
pub const USAGE: &str = "\
lua-api [--out <file>]
  Dumps the script-visible API of SPEC-05 §4: the globals of §4.1, then one line per
  member of each type of §4.2 to §4.4. Writes to stdout without --out.
  Line format: 'global <name> <type>', 'const <table>.<name> <value>',
  'member <type>.<name> <genre> <access> <result-type>'.";

/// The options of the command.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// Where to write the dump; the standard output when absent.
    pub output: Option<PathBuf>,
}

impl Options {
    /// Reads the command line after the command name.
    ///
    /// # Errors
    ///
    /// [`HarnessError::Usage`] on an argument the command does not know.
    pub fn parse(arguments: &[String]) -> Result<Self, HarnessError> {
        let mut options = Self::default();
        let mut remaining = arguments.iter();
        while let Some(argument) = remaining.next() {
            match (argument.as_str(), remaining.as_slice()) {
                ("--out", [path, ..]) => {
                    options.output = Some(PathBuf::from(path));
                    remaining.next();
                }
                _ => {
                    return Err(HarnessError::Usage(format!(
                        "unknown argument: {argument}\n\n{USAGE}"
                    )))
                }
            }
        }

        Ok(options)
    }
}

/// The lines of the dump: the globals of §4.1, the constants of §4.4, then the members of each
/// type, and the count line.
///
/// # Errors
///
/// A line could not be formatted.
pub fn dump_lines(surface: &ScriptApiSurface) -> Result<Vec<String>, HarnessError> {
    let mut lines = Vec::new();

    for (name, kind) in &surface.globals {
        lines.push(format!("global {name} {kind}"));
    }

    let mut functions = surface.function_globals.clone();
    functions.sort();
    for name in functions {
        lines.push(format!("global {name} fonction"));
    }

    for (table, entries) in &surface.constants {
        for (key, value) in entries {
            let mut line = String::new();
            write!(line, "const {table}.{key} {value}")?;
            lines.push(line);
        }
    }

    for definition in surface.types.values() {
        if definition.indexable_by_row() {
            lines.push(format!(
                "member {}.[i] index {} {}",
                definition.name,
                if definition.index_writable { "LE" } else { "L" },
                definition.element_type
            ));
        }

        if !definition.call_result.is_empty() {
            lines.push(format!(
                "member {}.() appel - {}",
                definition.name, definition.call_result
            ));
        }

        for member in definition.members.values() {
            let genre = if member.callable { "appel" } else { "prop" };
            let access = if member.callable {
                "-"
            } else if member.writable {
                "LE"
            } else {
                "L"
            };
            lines.push(format!(
                "member {}.{} {genre} {access} {}",
                definition.name, member.name, member.result_type
            ));
        }
    }

    lines.push(format!(
        "# {} globals, {} types, {} members",
        surface.globals.len() + surface.function_globals.len(),
        surface.types.len(),
        surface.member_count()
    ));
    Ok(lines)
}

/// Runs the command and returns the message to print, if any.
///
/// # Errors
///
/// The dump could not be formatted or written.
pub fn run(options: &Options) -> Result<Option<String>, HarnessError> {
    let lines = dump_lines(&ScriptApiSurface::describe())?;
    match &options.output {
        None => Ok(Some(lines.join("\n"))),
        Some(path) => {
            let mut text = lines.join("\n");
            text.push('\n');
            fs::write(path, text)?;
            Ok(Some(format!(
                "wrote {} lines to {}",
                lines.len(),
                path.display()
            )))
        }
    }
}
