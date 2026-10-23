//! The checks that run on the 579 scripts of the 101 built-in profiles of
//! `assets-neutral/profiles`: the script model of REQ-LUA-001 to 004, the compilation of
//! REQ-LUA-100, the API coverage of REQ-LUA-051 and the acceptance run of REQ-ACC-011.
//!
//! The tests that read the profiles are `#[ignore]`d: set `ZAXNES_PROFILES` and run with
//! `--ignored`.

use std::path::Path;

use zax_engine3d::image::TileImageKey;
use zax_profiles::enums::TileFlip;
use zax_profiles::profile::GameProfile;
use zax_profiles::script_order;
use zax_scripting::ScriptApiSurface;

use super::lua_access_resolver::LuaAccessResolver;
use super::support::{neutral_assets, Batch, ScriptSession};

fn file_name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into_owned()
}

/// REQ-LUA-001: the catalogue holds 579 scripts in 89 of the 101 profiles.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES"]
fn corpus_shape_matches_req_lua_001() {
    let mut profiles = 0;
    let mut with_scripts = 0;
    let mut scripts = 0;
    let mut enabled = 0;

    for path in neutral_assets::profile_files() {
        let profile = neutral_assets::load_path(&path);
        profiles += 1;
        if profile.scripts.is_empty() {
            continue;
        }

        with_scripts += 1;
        scripts += profile.scripts.len();
        enabled += profile
            .scripts
            .iter()
            .filter(|script| script.enabled)
            .count();
    }

    assert_eq!(profiles, 101);
    assert_eq!(with_scripts, 89);
    assert_eq!(scripts, 579);
    assert_eq!(enabled, 556);
    assert_eq!(scripts - enabled, 23);
}

/// REQ-LUA-100: the 579 scripts compile under the chosen Lua 5.4 runtime, with no change,
/// including the two profiles that use `goto` and labels.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES"]
fn every_script_compiles_req_lua_100() {
    let mut failures = Vec::new();
    let mut compiled = 0;

    for path in neutral_assets::profile_files() {
        let profile = neutral_assets::load_path(&path);
        if profile.scripts.is_empty() {
            continue;
        }

        let session = ScriptSession::create(&profile);
        compiled += profile.scripts.len();
        for error in session.log.errors() {
            failures.push(format!("{}: {error}", file_name(&path)));
        }
    }

    assert_eq!(compiled, 579);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// REQ-LUA-004: for each of the 89 profiles that carry scripts, sorting the names with the
/// collation of REQ-LUA-004 gives back the order of the file.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES"]
fn script_order_reproduces_the_file_order_req_lua_004() {
    let mut mismatches = Vec::new();

    for path in neutral_assets::profile_files() {
        let profile = neutral_assets::load_path(&path);
        if profile.scripts.is_empty() {
            continue;
        }

        let in_file: Vec<String> = profile
            .scripts
            .iter()
            .map(|script| script.name.clone().unwrap_or_default())
            .collect();
        let mut sorted = in_file.clone();
        sorted.sort_by(|left, right| script_order::compare_names(Some(left), Some(right)));
        if in_file != sorted {
            mismatches.push(format!("{}: [{}]", file_name(&path), in_file.join(", ")));
        }
    }

    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// REQ-LUA-051: a static resolution of every access chain of the 579 scripts, starting at the
/// globals of §4.1, finds no missing member and no wrong `.`/`:` separator.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES"]
fn every_member_access_of_the_corpus_resolves_req_lua_051() {
    let api = ScriptApiSurface::describe();
    let mut resolver = LuaAccessResolver::new(&api);

    for path in neutral_assets::profile_files() {
        let profile = neutral_assets::load_path(&path);
        if profile.scripts.is_empty() {
            continue;
        }

        let scripts: Vec<(String, String)> = profile
            .scripts
            .iter()
            .map(|script| {
                (
                    script.name.clone().unwrap_or_default(),
                    script.source.clone().unwrap_or_default(),
                )
            })
            .collect();
        resolver.check(
            path.file_stem().unwrap().to_string_lossy().as_ref(),
            &scripts,
        );
    }

    let unique = |accesses: &[super::lua_access_resolver::UnresolvedAccess]| {
        let mut lines: Vec<String> = accesses.iter().map(ToString::to_string).collect();
        lines.dedup();
        lines
    };
    assert!(
        resolver.unresolved().is_empty(),
        "members the corpus reads that the API does not declare:\n{}",
        unique(resolver.unresolved()).join("\n")
    );
    assert!(
        resolver.wrong_separator().is_empty(),
        "calls made with the wrong '.'/':' separator:\n{}",
        unique(resolver.wrong_separator()).join("\n")
    );

    // The corpus has to exercise a good part of the surface; a resolver that silently gave up
    // on everything would otherwise pass.
    assert!(
        resolver.resolved_couples().len() >= 150,
        "only {} (type, member) couples resolved",
        resolver.resolved_couples().len()
    );
}

/// The errors the synthetic frame of the acceptance run provokes, which REQ-ACC-011 tolerates
/// once documented. They come from the frame's contents, not from the API surface.
///
/// "punchout", script `zmac_punch`: it reads the global `opY`, which the script of the opponent
/// sets in its own `UpdateS`. The synthetic frame has no shape with the opponent's tag, so that
/// script never runs and `opY` stays nil (`math.min(nil, 76)`). A real game frame always
/// carries both shapes.
const DOCUMENTED_FRAME_DATA_ERRORS: [&str; 1] = [
    "punchout: Lua: script 'zmac_punch' hook UpdateS raised: attempt to compare number with nil",
];

/// REQ-ACC-011: for each of the 89 profiles that carry scripts, `Start`, then 600 frames of
/// `Update`, `UpdateS` and `LateUpdate` on a synthetic frame, then `End`, produce no Lua error
/// caused by a missing API member.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES"]
fn acceptance_run_of_the_89_profiles_req_acc_011() {
    const FRAMES: i32 = 600;
    let mut missing_member = Vec::new();
    let mut data_caused = Vec::new();
    let mut clean = 0;
    let mut total = 0;

    for path in neutral_assets::profile_files() {
        let profile = neutral_assets::load_path(&path);
        if profile.scripts.is_empty() {
            continue;
        }

        total += 1;
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let mut session = ScriptSession::create(&profile);
        let batch = synthetic_frame(&profile);

        session.start();
        for frame in 1..=FRAMES {
            batch.convert_and_publish(&mut session.engine, frame);
        }

        session.end();

        let errors = session.log.errors();
        if errors.is_empty() {
            clean += 1;
            continue;
        }

        for error in errors {
            // Only the binding layer produces these three, so they are the signature of an API
            // member that is absent or that refused its arguments.
            if error.contains("no such field")
                || error.contains("invalid arguments to")
                || error.contains("it is read only")
            {
                missing_member.push(format!("{name}: {error}"));
            } else {
                data_caused.push(format!("{name}: {error}"));
            }
        }
    }

    assert_eq!(total, 89);
    assert!(
        missing_member.is_empty(),
        "{} errors point at the API surface:\n{}",
        missing_member.len(),
        missing_member.join("\n")
    );

    // An error the synthetic frame itself causes is tolerated when it is documented
    // (REQ-ACC-011 acceptance criterion). Any other one fails the test.
    let undocumented: Vec<&String> = data_caused
        .iter()
        .filter(|error| {
            !DOCUMENTED_FRAME_DATA_ERRORS
                .iter()
                .any(|known| error.contains(known))
        })
        .collect();
    assert!(
        undocumented.is_empty(),
        "{clean}/{total} profiles ran without any error; undocumented errors:\n{}",
        undocumented
            .iter()
            .map(|error| error.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(clean, total - DOCUMENTED_FRAME_DATA_ERRORS.len());
}

/// REQ-LUA-100 and the two `goto` profiles of §5.3: Burger Time and Jackal load and run their
/// hooks.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES"]
fn profiles_that_use_goto_load_and_run() {
    for prefix in ["Burger", "Jackal"] {
        let profile = neutral_assets::load_named(prefix)
            .unwrap_or_else(|| panic!("no built-in profile starts with {prefix}"));
        assert!(profile.scripts.iter().any(|script| script
            .source
            .as_deref()
            .unwrap_or_default()
            .contains("goto")));

        let mut session = ScriptSession::create(&profile);
        assert!(session.log.errors().is_empty());

        session.start();
        let batch = synthetic_frame(&profile);
        for frame in 1..=8 {
            batch.convert_and_publish(&mut session.engine, frame);
        }

        session.end();
        assert!(!session.log.errors().iter().any(|error| {
            error.contains("no such field") || error.contains("invalid arguments to")
        }));
    }
}

/// REQ-LUA-004: the Excitebike order, where the underscore sorts before the digit, is the one
/// the collation produces and the one the hooks follow.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES"]
fn excitebike_hook_order_req_lua_004() {
    let profile = neutral_assets::load_named("Excitebike")
        .unwrap_or_else(|| panic!("no built-in profile starts with Excitebike"));

    let mut session = ScriptSession::create(&profile);
    let order: Vec<String> = session.runtime(|runtime, _| {
        runtime
            .scripts()
            .iter()
            .map(|script| script.name().to_owned())
            .collect()
    });
    assert_eq!(
        order,
        [
            "bike",
            "camera",
            "ground",
            "ramp_rotation",
            "ramp7",
            "scene"
        ]
    );
}

/// A synthetic frame built from the profile's own patterns, so that the shapes the segmentation
/// recognises carry the tags the scripts are linked to (REQ-ACC-011).
fn synthetic_frame(profile: &GameProfile) -> Batch {
    let mut batch = Batch::new();
    for index in 0..32_usize {
        // A palette with no two equal entries, so that a colour read is meaningful.
        batch = batch.palette(index, u8::try_from((index * 3) % 64).unwrap());
    }

    let mut column = 0;
    let mut row = 0;
    let mut row_height = 0;
    let mut sprites = 0_i32;

    for pattern in &profile.permanent_patterns {
        let Some(grid) = &pattern.grid else {
            continue;
        };

        if grid.width() <= 0 || grid.height() <= 0 || grid.width() > 30 || grid.height() > 28 {
            continue;
        }

        if column + grid.width() > 30 {
            column = 0;
            row += row_height.max(1);
            row_height = 0;
        }

        if row + grid.height() > 28 {
            break;
        }

        for j in 0..grid.height() {
            for i in 0..grid.width() {
                let Some(tile) = grid.get(i, j) else {
                    continue;
                };

                let key = TileImageKey::new(
                    tile.low_plane,
                    tile.high_plane,
                    tile.flip.contains(TileFlip::HORIZONTAL),
                    tile.flip.contains(TileFlip::VERTICAL),
                );
                if key.is_empty() {
                    continue;
                }

                batch = batch.bg_cell_palette(
                    key,
                    column + i,
                    row + j,
                    u8::try_from((i + j) & 3).unwrap(),
                );

                // A few sprite tiles as well, so that sprite shapes exist too.
                if sprites < 48 && i == 0 && j == 0 {
                    batch = batch.sprite_with(
                        key,
                        8 + ((sprites * 5) % 232),
                        16 + ((sprites * 7) % 200),
                        1,
                    );
                    sprites += 1;
                }
            }
        }

        column += grid.width();
        row_height = row_height.max(grid.height());
    }

    batch
}

/// The API description the runtime produces is complete enough to be useful: every global of
/// §4.1 names a declared type, and the surface has the order of magnitude §5.1 reports.
#[test]
fn api_description_is_consistent_req_lua_080() {
    let api = ScriptApiSurface::describe();

    for (global, kind) in &api.globals {
        assert!(
            kind.is_empty() || api.types.contains_key(kind),
            "global {global} names the undeclared type {kind}"
        );
    }

    for definition in api.types.values() {
        for member in definition.members.values() {
            assert!(
                member.result_type.is_empty() || api.types.contains_key(&member.result_type),
                "{}.{} yields the undeclared type {}",
                definition.name,
                member.name,
                member.result_type
            );
        }
    }

    assert!(
        api.member_count() >= 370,
        "only {} members declared",
        api.member_count()
    );
    assert_eq!(api.constants.len(), 3);
}
