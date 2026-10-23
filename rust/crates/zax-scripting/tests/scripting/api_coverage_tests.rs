//! REQ-LUA-080 and REQ-LUA-051: the whole script-visible API of SPEC-05 §4 is installed. The
//! expectation is the transcription of the §4 tables in `spec05-api.txt`, checked row by row
//! against the description the runtime produces from the tables it binds
//! ([`ScriptApiSurface`]), and against the Lua state itself for the globals.
//!
//! The fixture is a copy of the specification, so the check is executable without the
//! specification folder (DEC-R2-T7) and a drift between the two shows up as a failing row
//! rather than as a silently missing member.

use std::collections::HashSet;

use zax_scripting::{ScriptApiSurface, ScriptApiType};

use super::support::ScriptProfileBuilder;

/// The transcription of the tables of SPEC-05 §4.
const SPEC_API: &str = include_str!("spec05-api.txt");

/// Type names of §4 that the binder spells in ASCII, or splits per element type.
fn type_aliases(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "Découpe" => Some(&["Decoupe"]),
        "Lumière" => Some(&["Lumiere"]),
        "DonnéesPerso" => Some(&["CData"]),
        "Liste<T>" => Some(&["Liste<Instance>", "Liste<Shape>"]),
        "TLayerEnum" => Some(&["TLayer"]),
        "AlignEnum" => Some(&["Align"]),
        "CamModeEnum" => Some(&["CamMode"]),
        _ => None,
    }
}

/// The globals of §4.1 that are plain Lua values rather than host objects, so they are checked
/// in the state instead of in the description.
fn plain_global(name: &str) -> Option<&'static str> {
    match name {
        "frame" | "time" | "pdata_count" => Some("number"),
        "PData" => Some("table"),
        _ => None,
    }
}

/// The globals §4.1 only assigns while a hook runs: `shape` and `shape2D` during `UpdateS`,
/// `pattern` during `BuildPattern` (REQ-LUA-025, 030), and `self` during any hook. They are nil
/// outside one, which the hook-order vectors exercise; here only their declared type is
/// checked.
const HOOK_GLOBALS: [&str; 4] = ["shape", "shape2D", "pattern", "self"];

struct SpecRow {
    kind: &'static str,
    fields: Vec<&'static str>,
}

fn spec_rows() -> Vec<SpecRow> {
    SPEC_API
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let mut parts = line.split('\t');
            SpecRow {
                kind: parts.next().unwrap(),
                fields: parts.collect(),
            }
        })
        .collect()
}

fn resolve<'a>(api: &'a ScriptApiSurface, spec_name: &str) -> Vec<&'a ScriptApiType> {
    let names: Vec<&str> =
        type_aliases(spec_name).map_or_else(|| vec![spec_name], <[&str]>::to_vec);
    names
        .into_iter()
        .filter_map(|name| api.types.get(name))
        .collect()
}

/// REQ-LUA-080: every global of §4.1 exists, with the kind §4.1 gives it.
#[test]
fn every_global_of_section_4_1_is_installed_req_lua_080() {
    let api = ScriptApiSurface::describe();
    let mut session = ScriptProfileBuilder::new().script("a", "-- nothing").load();

    let mut missing = Vec::new();
    let mut checked = 0;
    for row in spec_rows().iter().filter(|row| row.kind == "global") {
        checked += 1;
        let name = row.fields[0];

        if let Some(lua_type) = plain_global(name) {
            let actual = session.text(&format!("return type({name})"));
            if actual != lua_type {
                missing.push(format!(
                    "global {name}: type() is '{actual}', expected '{lua_type}'"
                ));
            }

            continue;
        }

        if row.fields[1] == "fonction" {
            if session.text(&format!("return type({name})")) != "function" {
                missing.push(format!("global {name} is not a function"));
            }

            continue;
        }

        // A host-object global: the description must name a declared type for it, and the
        // state must hold a value (userdata or, for the constant tables of §4.4, a table).
        match api
            .globals
            .get(name)
            .filter(|declared| !declared.is_empty())
        {
            None => {
                if !api.constants.contains_key(name) {
                    missing.push(format!("global {name} is absent from the API description"));
                    continue;
                }
            }
            Some(declared) => {
                if !api.types.contains_key(declared) {
                    missing.push(format!(
                        "global {name} names the undeclared type {declared}"
                    ));
                    continue;
                }
            }
        }

        if HOOK_GLOBALS.contains(&name) {
            continue;
        }

        let kind = session.text(&format!("return type({name})"));
        if kind != "userdata" && kind != "table" {
            missing.push(format!("global {name}: type() is '{kind}'"));
        }
    }

    assert_eq!(checked, 42);
    assert!(
        missing.is_empty(),
        "{} globals of §4.1 are wrong:\n{}",
        missing.len(),
        missing.join("\n")
    );
}

/// REQ-LUA-080: every member row of §4.2 to §4.4 resolves against the installed API, with the
/// right call kind (REQ-LUA-081: `prop`, `const` and `index` are not callable; `méth`, `fonc`
/// and `appel` are).
#[test]
fn every_member_row_of_section_4_resolves_req_lua_080() {
    let api = ScriptApiSurface::describe();
    let mut missing = Vec::new();
    let mut wrong_kind = Vec::new();
    let mut checked = 0;

    for row in spec_rows().iter().filter(|row| row.kind == "member") {
        checked += 1;
        let type_name = row.fields[0];
        let member = row.fields[1];
        let genre = row.fields[2];

        // The enum types of §4.4 are constant tables, not host objects (REQ-LUA-046): their
        // rows resolve against the table the global of the same name holds.
        if genre == "const" {
            if let Some(table) = type_aliases(type_name).and_then(|tables| {
                api.constants
                    .get(tables[0])
                    .map(|entries| (tables[0], entries))
            }) {
                if !table.1.contains_key(member) {
                    missing.push(format!(
                        "{}.{member} is missing from the constant table",
                        table.0
                    ));
                }

                continue;
            }
        }

        let candidates = resolve(&api, type_name);
        if candidates.is_empty() {
            missing.push(format!("{type_name} is not declared (row {member})"));
            continue;
        }

        // The §4 rows that describe indexing or the call of a class table.
        if genre == "index" {
            if !candidates.iter().any(|definition| definition.indexable()) {
                missing.push(format!("{type_name}{member} is not indexable"));
            }

            continue;
        }

        if genre == "appel" && member.starts_with('(') {
            if !candidates
                .iter()
                .any(|definition| !definition.call_result.is_empty())
            {
                missing.push(format!("{type_name}{member} is not callable"));
            }

            continue;
        }

        // A constant of an enum table of §4.4 lives in the constant tables.
        if genre == "const" && candidates.len() == 1 && candidates[0].members.is_empty() {
            continue;
        }

        let found = candidates
            .iter()
            .find_map(|definition| definition.members.get(member));
        let Some(found) = found else {
            // §4.4 constants are exposed through the constant tables, not as members.
            if genre == "const"
                && api
                    .constants
                    .values()
                    .any(|table| table.contains_key(member))
            {
                continue;
            }

            missing.push(format!(
                "{type_name}.{member} ({genre}, {}) is missing",
                row.fields[4]
            ));
            continue;
        };

        let should_be_callable = matches!(genre, "méth" | "fonc" | "appel");
        if found.callable != should_be_callable {
            wrong_kind.push(format!(
                "{type_name}.{member} is {}, but §4 says '{genre}'",
                if found.callable {
                    "callable"
                } else {
                    "a property"
                }
            ));
        }
    }

    assert_eq!(checked, 360);
    assert!(
        missing.is_empty(),
        "{} member rows of §4 do not resolve:\n{}",
        missing.len(),
        missing.join("\n")
    );
    assert!(
        wrong_kind.is_empty(),
        "{} member rows have the wrong call kind:\n{}",
        wrong_kind.len(),
        wrong_kind.join("\n")
    );
}

/// The fixture counts match §5.1 of the specification: 42 globals and 46 types. It guards the
/// transcription itself against a truncated copy.
#[test]
fn the_transcription_of_section_4_is_complete() {
    let rows = spec_rows();
    assert_eq!(rows.iter().filter(|row| row.kind == "global").count(), 42);
    assert_eq!(rows.iter().filter(|row| row.kind == "member").count(), 360);
    let types: HashSet<&str> = rows
        .iter()
        .filter(|row| row.kind == "member")
        .map(|row| row.fields[0])
        .collect();
    assert_eq!(types.len(), 46);
}

/// The surface is at least as large as the tables it must cover.
#[test]
fn the_installed_surface_covers_the_tables_req_lua_080() {
    let api = ScriptApiSurface::describe();
    assert!(
        api.member_count() >= 360,
        "only {} members installed",
        api.member_count()
    );
}
