//! The « 3dh = 3dn » regression of AMD-062 (changelog §2.6): a 2.1 profile resolved from its
//! ROM builds, tile for tile, the geometry its own 1.x counterpart builds.
//!
//! This is the test the V2-34 trap was caught by on the .NET side. A `textureKind = 2` texture is
//! an exclusive-or against the pattern's default texture, and the default texture depends on the
//! tiles: built while they are waiting it is `X XOR 0`, so a profile whose tiles are resolved too
//! late — or whose textures are not recomputed once they are — draws wrong colours, black cells,
//! and tiles that stop being full (24 vertices where 48 belong). None of that shows in a unit
//! test on a synthetic profile with one tile; it shows here, over five commercial games and their
//! real packs.
//!
//! Zaxnes cannot fall into the trap by construction — a variant keeps `textureKind` and field 24
//! as read and the texture is *derived* where it is used (RU21 step 4), so there is no cached
//! exclusive-or to go stale. This test is what proves that on the corpus rather than asserting it.
//!
//! **What the ROM can and cannot resolve.** REQ-PROF-078 reads the tiles out of the PRG and CHR
//! zones of the iNES file, so it resolves a manifest only as far as the cartridge *stores* its
//! tiles. On a CHR-RAM board the pattern tables are written by the running game and are in no
//! zone of the file, which is the whole reason REQ-PROF-080 point 1 exists; those tiles arrive
//! while playing, and `amd062_tile_resolution_tests` (engine) and `amd062_chr_ram_resolution_tests`
//! (app, real core) are where that path is checked. Measured over the five games of the
//! changelog, the split is clean and is asserted below: four of the five resolve their manifest
//! whole, and Contra — mapper 2, UNROM, no CHR zone at all — resolves only the 1 603 of its 4 574
//! strings that its PRG happens to contain.
//!
//! So the comparison here is run over the 3D objects whose pattern has **no** placeholder left.
//! For a pattern that still holds one, REQ-PROF-081 *asks* for a different geometry (`Default`,
//! no `fullWarp`, the waiting cell contributing no mesh and no texel), so comparing it with the
//! 1.x homologue would assert the opposite of the requirement.
//!
//! Needs the profiles (`ZAXNES_PROFILES`, both `profiles/` and `profiles-2.1/`) and the
//! commercial ROMs (`ZAXNES_ROMS`), so every test here is `#[ignore]`d.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use zax_engine3d::engine::ConversionEngine;
use zax_engine3d::geometry::builder::GeometryBuilder;
use zax_engine3d::geometry::object_geometry::ObjectGeometry;
use zax_engine3d::instance::InstanceRef;
use zax_profiles::format;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;

use super::support::neutral_assets;

/// One game of the changelog's regression: its pack's file name — the 1.x and the 2.1 profile
/// share it for all five (RU21 step 3) — the ROM the pack was authored for, and what the ROM walk
/// of REQ-PROF-078 makes of its manifest, measured.
struct Game {
    pack: &'static str,
    rom: &'static str,
    /// Manifest strings (REQ-PROF-071 §3).
    manifest: usize,
    /// Of those, the ones the walk through the PRG and CHR zones resolves.
    resolved: usize,
    /// And the ones left for the CHR-RAM of a running game (REQ-PROF-080 point 1).
    waiting: usize,
}

const GAMES: &[Game] = &[
    Game {
        pack: "Super Mario Bros 3 (U) (PRG1)",
        rom: "Super Mario Bros. 3 (USA).nes",
        manifest: 4856,
        resolved: 4856,
        waiting: 0,
    },
    Game {
        // Mapper 2, UNROM: no CHR zone at all, the game writes its pattern tables itself, so
        // only the tiles its PRG happens to hold are there to be found.
        pack: "Contra",
        rom: "Contra (USA).nes",
        manifest: 4574,
        resolved: 1603,
        waiting: 2971,
    },
    Game {
        pack: "Legend Of Zelda",
        rom: "Legend of Zelda, The (USA).nes",
        manifest: 1035,
        resolved: 1035,
        waiting: 0,
    },
    Game {
        pack: "Super Mario Bros",
        rom: "Super Mario Bros. (World).nes",
        manifest: 481,
        resolved: 481,
        waiting: 0,
    },
    Game {
        // Q-178 §2's figure, measured here from the other end: 144 of Mega Man 2's strings are in
        // neither zone of the file and stay waiting for the whole session.
        pack: "Mega Man 2",
        rom: "Mega Man 2 (USA).nes",
        manifest: 5497,
        resolved: 5353,
        waiting: 144,
    },
];

fn roms_directory() -> PathBuf {
    let root = env::var_os("ZAXNES_ROMS").unwrap_or_else(|| {
        panic!("ZAXNES_ROMS is not set: it must name the commercial ROM folder")
    });
    PathBuf::from(root)
}

/// The 2.1 profile of `game`, resolved from its ROM, with the manifest size, how many strings the
/// ROM resolved and how many are still waiting.
fn hash_profile(game: &Game) -> (format::ProfileDocument, usize, usize, usize) {
    let path = neutral_assets::profiles_directory()
        .parent()
        .unwrap()
        .join("profiles-2.1")
        .join(format!("{}.bytes", game.pack));
    let bytes = fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let label = path.display().to_string();
    let mut document =
        format::load(&bytes, None, Some(&label)).unwrap_or_else(|error| panic!("{label}: {error}"));
    assert!(
        document.is_hash_profile(),
        "{label}: profiles-2.1 must hold \"3dh\" files (REQ-PROF-070)"
    );

    let rom_path = roms_directory().join(game.rom);
    let image =
        fs::read(&rom_path).unwrap_or_else(|error| panic!("{}: {error}", rom_path.display()));
    let manifest = document.hash_state().unwrap().manifest.keys().len();
    let resolved = document.resolve_from_rom(&image);
    let waiting = document.waiting_keys().len();
    (document, manifest, resolved, waiting)
}

fn builder(engine: &ConversionEngine) -> &GeometryBuilder {
    engine.geometry_builder().unwrap()
}

/// Loads `profile` into an engine that builds real geometry.
fn engine_for(profile: &GameProfile) -> ConversionEngine {
    let mut engine = ConversionEngine::new();
    engine.use_geometry();
    engine.load_profile(profile, true).unwrap();
    engine
}

/// Every 3D object of a loaded engine, in registry order: the pattern's slot, the instance index,
/// whether the pattern still holds a waiting tile, and the geometry.
fn objects(engine: &ConversionEngine) -> Vec<(usize, usize, bool, &ObjectGeometry)> {
    let mut out = Vec::new();
    for (slot, id) in engine.patterns().all().enumerate() {
        let Some(pattern) = engine.patterns().get(id) else {
            continue;
        };
        let waiting = pattern.has_placeholders();
        for index in 0..pattern.instances().len() {
            if let Some(geometry) = builder(engine).of(InstanceRef { pattern: id, index }) {
                out.push((slot, index, waiting, geometry));
            }
        }
    }
    out
}

/// What the comparison of one game found.
#[derive(Debug, Default, PartialEq, Eq)]
struct Report {
    objects: usize,
    /// 3D objects whose pattern still holds a waiting tile, which REQ-PROF-081 builds differently
    /// on purpose, or whose variant the owner re-authored between 1.x and 2.1 — neither is
    /// comparable, and [`authored_differences`] counts the second.
    skipped: usize,
    compared: usize,
    tiles: usize,
    differing_tiles: usize,
    differing_textures: usize,
    differing_object_shapes: usize,
}

/// All the patterns of a profile, permanent then temporary, in the order the registry loads them.
fn patterns(profile: &GameProfile) -> Vec<&Arc<ProfilePattern>> {
    profile
        .permanent_patterns
        .iter()
        .chain(&profile.temporary_patterns)
        .collect()
}

/// For each pattern slot, whether the two packs declare the **same** authored variants.
///
/// The 2.1 packs are not re-encodings of the 1.x ones: the owner edited a few variants between
/// the two releases — Super Mario Bros's pattern 135 declares `fullWarp` in 1.x and not in 2.1,
/// which moves a mesh from 108 triangles to 64. That is a difference of artwork, not of format,
/// so the geometry comparison steps over such a pattern; the plane comparison, which is what the
/// format is answerable for, covers it all the same.
fn authored_differences(legacy: &GameProfile, hash: &GameProfile) -> (Vec<bool>, usize) {
    let a = patterns(legacy);
    let b = patterns(hash);
    let mut same = Vec::with_capacity(a.len());
    let mut differing = 0;
    for (p, q) in a.iter().zip(&b) {
        let agrees = p.variants.len() == q.variants.len()
            && p.variants.iter().zip(&q.variants).all(|(v, w)| {
                v.geometry == w.geometry
                    && v.depth == w.depth
                    && v.full_warp == w.full_warp
                    && v.alpha == w.alpha
                    && v.render_style == w.render_style
                    && v.mesh_algorithm == w.mesh_algorithm
            });
        if !agrees {
            differing += 1;
        }
        same.push(agrees);
    }
    (same, differing)
}

/// The resolved tile planes of the two formats, cell by cell: how many cells were compared and
/// how many differ. A cell still waiting in the 3dh is not compared — it has no planes yet by
/// REQ-PROF-081 — and is counted apart.
fn compare_planes(legacy: &GameProfile, hash: &GameProfile) -> (usize, usize, usize) {
    let (a, b) = (patterns(legacy), patterns(hash));
    let mut compared = 0;
    let mut differing = 0;
    let mut waiting = 0;
    for (p, q) in a.iter().zip(&b) {
        let (Some(one), Some(other)) = (p.grid.as_ref(), q.grid.as_ref()) else {
            continue;
        };
        if one.width() != other.width() || one.height() != other.height() {
            differing += 1;
            continue;
        }
        for (x, y) in one.cells().iter().zip(other.cells()) {
            match (x, y) {
                (Some(x), Some(y)) => {
                    if y.is_waiting() {
                        waiting += 1;
                        continue;
                    }
                    compared += 1;
                    if x.low_plane != y.low_plane
                        || x.high_plane != y.high_plane
                        || x.flip != y.flip
                    {
                        differing += 1;
                    }
                }
                (None, None) => {}
                _ => differing += 1,
            }
        }
    }
    (compared, differing, waiting)
}

/// Loads both formats of one game and compares their geometry object by object, tile by tile.
fn compare(game: &Game) -> (Report, usize, usize, usize) {
    let legacy = neutral_assets::load(&format!("{}.bytes", game.pack));
    let (document, manifest, resolved, waiting) = hash_profile(game);
    let (agrees, authored) = authored_differences(&legacy, &document.profile);
    if authored > 0 {
        println!(
            "  {}: {authored} pattern(s) whose variants the owner re-authored between 1.x \
             and 2.1, not compared",
            game.pack
        );
    }

    let theirs = engine_for(&legacy);
    let mine = engine_for(&document.profile);

    let a = objects(&theirs);
    let b = objects(&mine);
    assert_eq!(
        a.len(),
        b.len(),
        "{}: the two formats must hold the same 3D objects",
        game.pack
    );

    let mut report = Report::default();
    for ((slot, index, _, left), (_, _, placeholders, right)) in a.iter().zip(&b) {
        report.objects += 1;
        if *placeholders || !agrees.get(*slot).copied().unwrap_or(true) {
            report.skipped += 1;
            continue;
        }
        report.compared += 1;

        if left.meshes().len() != right.meshes().len()
            || left.build_depth() != right.build_depth()
            || left.effective_depth() != right.effective_depth()
        {
            report.differing_object_shapes += 1;
            continue;
        }
        if **left.texture() != **right.texture() {
            report.differing_textures += 1;
        }
        for (one, other) in left.meshes().iter().zip(right.meshes()) {
            report.tiles += 1;
            // The whole mesh, not just its triangle count: the V2-34 trap moves vertices and
            // colours, and a count alone would let a wrongly coloured tile through.
            if **one != **other {
                report.differing_tiles += 1;
                if report.differing_tiles <= 3 {
                    println!(
                        "  {}: object ({slot}, {index}) differs ({} vs {} triangles)",
                        game.pack,
                        other.triangle_count(),
                        one.triangle_count()
                    );
                }
            }
        }
    }

    (report, manifest, resolved, waiting)
}

// Changelog §2.6, the V2-34 regression: for each of the five games, every 3D object the 2.1
// profile has fully resolved builds the mesh and the texture its 1.x homologue builds — 0 tile
// different, 0 texture different. That is what says the texture a `textureKind = 2` variant
// derives is the homologue's and not an exclusive-or against a default built while the tiles
// were still waiting.
#[test]
#[ignore = "needs ZAXNES_PROFILES (profiles/ and profiles-2.1/) and ZAXNES_ROMS"]
fn amd_062_a_3dh_profile_resolved_from_its_rom_builds_the_geometry_of_its_3dn_homologue() {
    let mut compared_games = 0;
    for game in GAMES {
        let (report, manifest, resolved, waiting) = compare(game);
        println!(
            "{}: manifest {manifest}, resolved {resolved}, waiting {waiting} | \
             {} objects, {} compared, {} skipped, {} tiles",
            game.pack, report.objects, report.compared, report.skipped, report.tiles
        );
        assert!(
            report.tiles > 0,
            "{}: nothing was compared at all",
            game.pack
        );
        assert_eq!(
            Report {
                differing_tiles: 0,
                differing_textures: 0,
                differing_object_shapes: 0,
                ..report
            },
            report,
            "{}: the 3dh geometry must be the 3dn geometry, tile for tile",
            game.pack
        );
        compared_games += 1;
    }
    assert_eq!(GAMES.len(), compared_games);
}

// REQ-PROF-078: what the ROM walk makes of each of the five manifests, to the string. The walk
// reads the PRG and CHR zones of the iNES file, so it finds a tile only where the cartridge
// stores one: Contra (mapper 2, no CHR zone) leaves 2 971 of its 4 574 strings for the CHR-RAM of
// the running game, and Mega Man 2 leaves the 144 Q-178 §2 names even though its board has a CHR
// zone. The three other packs are resolved whole before the first frame.
#[test]
#[ignore = "needs ZAXNES_PROFILES (profiles/ and profiles-2.1/) and ZAXNES_ROMS"]
fn req_prof_078_the_rom_walk_resolves_each_of_the_five_manifests_to_the_string() {
    for game in GAMES {
        let (_, manifest, resolved, waiting) = hash_profile(game);
        println!(
            "{}: manifest {manifest}, resolved {resolved}, waiting {waiting}",
            game.pack
        );
        assert_eq!(
            manifest,
            resolved + waiting,
            "{}: the manifest must split into what the ROM resolved and what still waits",
            game.pack
        );
        assert_eq!(
            (game.manifest, game.resolved, game.waiting),
            (manifest, resolved, waiting),
            "{}: the ROM walk no longer resolves what it did",
            game.pack
        );
    }
}

// The claim the format is answerable for, and the strongest form of « 3dh = 3dn »: for each of
// the five games, every cell the ROM resolved carries **exactly** the planes and the mirroring its
// 1.x homologue carries. The identifier of REQ-PROF-075, the mixing prefix of REQ-PROF-077, the
// four mirrorings of REQ-PROF-079 and the iNES zones of REQ-PROF-078 are all in this one figure:
// a single wrong bit anywhere in the chain would move a cell.
#[test]
#[ignore = "needs ZAXNES_PROFILES (profiles/ and profiles-2.1/) and ZAXNES_ROMS"]
fn amd_062_every_cell_the_rom_resolves_carries_the_planes_of_its_3dn_homologue() {
    let mut total = 0;
    for game in GAMES {
        let legacy = neutral_assets::load(&format!("{}.bytes", game.pack));
        let (document, _, _, _) = hash_profile(game);
        let (compared, differing, waiting) = compare_planes(&legacy, &document.profile);
        println!(
            "{}: {compared} cells compared, {differing} differing, {waiting} still waiting",
            game.pack
        );
        assert!(compared > 0, "{}: no cell was compared", game.pack);
        assert_eq!(
            0, differing,
            "{}: a resolved cell must hold the planes of its 3dn homologue",
            game.pack
        );
        total += compared;
    }
    println!("{total} resolved cells compared over the five games, 0 differing");
}
