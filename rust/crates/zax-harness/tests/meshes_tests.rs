//! `meshes` on synthetic frames: a profile built in memory, written as a `.3dn` file and
//! exported, gives the counts the engine tests expect (SPEC-04B REQ-E3D-501, REQ-E3D-540,
//! REQ-E3D-641): a full tile is a prism of six rectangles, the meshes of a pattern are one per
//! distinct image, and every tile laid out by the synthetic frames is drawn.

// A test: a failed write or a malformed export should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use zax_harness::meshes::{self, Options};
use zax_profiles::enums::{TileFlip, VariantGeometry, VariantRenderStyle};
use zax_profiles::format;
use zax_profiles::layout::FieldPresence;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::tile::{PatternGrid, ProfileTile};
use zax_profiles::values::Vec2I;
use zax_profiles::variant::PatternVariant;

/// A full tile of colour 1 made unique by one marker pixel of colour 3, as the engine tests'
/// `Img.Full` does.
fn full(id: u32) -> Arc<ProfileTile> {
    Arc::new(ProfileTile::new(u64::MAX, 1 << id, TileFlip::NONE))
}

fn pattern(
    width: i32,
    tiles: Vec<Arc<ProfileTile>>,
    geometry: VariantGeometry,
    depth: i32,
) -> Arc<ProfilePattern> {
    Arc::new(ProfilePattern {
        grid: Some(PatternGrid::new(
            width,
            1,
            tiles.into_iter().map(Some).collect(),
        )),
        tile_size: Some(Vec2I::new(width, 1)),
        permanent: true,
        variants: vec![Arc::new(PatternVariant {
            depth,
            geometry,
            alpha: 1.0,
            render_style: VariantRenderStyle::Normal,
            ..PatternVariant::default()
        })],
        ..ProfilePattern::default()
    })
}

fn export(name: &str, profile: &GameProfile) -> (PathBuf, Value) {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(&root).unwrap();
    let file = root.join("synthetic.3dn");
    fs::write(&file, format::save(profile, FieldPresence::All)).unwrap();

    let output = root.join("out");
    let arguments: Vec<String> = [
        "--profile",
        file.to_str().unwrap(),
        "--out",
        output.to_str().unwrap(),
        "--local",
    ]
    .iter()
    .map(|&argument| argument.to_owned())
    .collect();
    let options = Options::parse(&arguments).unwrap();
    let summary = meshes::run(&options).unwrap();
    (output, serde_json::from_str(&summary).unwrap())
}

#[test]
fn synthetic_frames_export_the_counts_of_the_engine_tests() {
    let mut profile = GameProfile::create_empty();
    profile.permanent_patterns = vec![
        // [A B]: two distinct full tiles, two prisms of six rectangles (TV-G7).
        pattern(2, vec![full(0), full(1)], VariantGeometry::Default, 4),
        // [C]: a cube of a full tile, twelve rectangles (TV-C1).
        pattern(1, vec![full(2)], VariantGeometry::Cube, 4),
    ];

    let (output, summary) = export("meshes-synthetic", &profile);

    assert_eq!(2, summary["patterns"]);
    assert_eq!(2, summary["objects"]);
    assert_eq!(3, summary["tileMeshes"]);
    assert_eq!(2 * 6 * 2 + 12 * 2, summary["triangles"]);
    assert_eq!(2 * 6 * 4 + 12 * 4, summary["vertices"]);
    assert_eq!(1, summary["framesExported"]);
    assert_eq!(3, summary["tilesDrawn"]);
    assert_eq!(1, summary["objectsByGeometry"]["Default"]);
    assert_eq!(1, summary["objectsByGeometry"]["Cube"]);
    assert_eq!(Some(1), summary["loadProgress"].as_i64());

    // The frame list carries one entry per drawn tile with its mesh (REQ-E3D-641). The two
    // tiles of [A B] are drawn without the wall they share (Q-067), five rectangles each; the
    // cube is another object and keeps all of its faces.
    let frame: Value =
        serde_json::from_str(&fs::read_to_string(output.join("frame0000.json")).unwrap()).unwrap();
    assert_eq!(3, frame["tileCount"]);
    let triangles: Vec<i64> = frame["tiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tile| tile["meshTriangles"].as_i64().unwrap())
        .collect();
    assert_eq!(vec![10, 10, 24], triangles);

    // The OBJ files hold every vertex and every triangle of the drawn tiles and of the
    // objects' own meshes.
    for (file, vertices, faces) in [("frame0000.obj", 88, 44), ("objects.obj", 96, 48)] {
        let text = fs::read_to_string(output.join(file)).unwrap();
        assert_eq!(
            vertices,
            text.lines().filter(|line| line.starts_with("v ")).count(),
            "{file}"
        );
        assert_eq!(
            faces,
            text.lines().filter(|line| line.starts_with("f ")).count(),
            "{file}"
        );
    }
}

#[test]
fn a_frame_dump_replays_its_tiles() {
    let mut profile = GameProfile::create_empty();
    profile.permanent_patterns = vec![pattern(1, vec![full(0)], VariantGeometry::Default, 2)];

    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("meshes-dump");
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(&root).unwrap();
    let file = root.join("one.3dn");
    fs::write(&file, format::save(&profile, FieldPresence::All)).unwrap();
    let dump = root.join("frames.txt");
    fs::write(
        &dump,
        "# two frames of one known tile and one unknown sprite\n\
         palette 0F160000000000000000000000000000000000000000000000000000000000\n\
         bg 16 32 0 ffffffffffffffff 1 0 0\n\
         sp 100 100 1 ffffffffffffffff 4 0 0 1\n\
         end\n\
         bg 24 32 0 ffffffffffffffff 1 0 0\n",
    )
    .unwrap();

    let output = root.join("out");
    let arguments: Vec<String> = [
        "--profile",
        file.to_str().unwrap(),
        "--out",
        output.to_str().unwrap(),
        "--frames",
        dump.to_str().unwrap(),
    ]
    .iter()
    .map(|&argument| argument.to_owned())
    .collect();
    let summary: Value =
        serde_json::from_str(&meshes::run(&Options::parse(&arguments).unwrap()).unwrap()).unwrap();

    // The sprite is learned (REQ-E3D-161), so both frames draw their tiles.
    assert_eq!(2, summary["framesExported"]);
    assert_eq!(3, summary["tilesDrawn"]);
    assert_eq!(2, summary["objects"]);
    assert!(output.join("frame0001.obj").is_file());
}

#[test]
fn the_command_line_is_checked() {
    let parse = |arguments: &[&str]| {
        Options::parse(&arguments.iter().map(|&a| a.to_owned()).collect::<Vec<_>>())
    };

    assert!(parse(&[]).is_err());
    assert!(parse(&["--profile"]).is_err());
    assert!(parse(&["--profile", "x", "--algorithm", "smooth"]).is_err());
    assert!(parse(&["profile"]).is_err());

    let options = parse(&[
        "--profile",
        "dir/Contra.bytes",
        "--algorithm",
        "marching",
        "--handedness",
        "left",
        "--frame-count",
        "0",
    ])
    .unwrap();
    assert_eq!(PathBuf::from("mesh-export").join("Contra"), options.output);
    assert_eq!(
        zax_engine3d::settings::RenderAlgorithm::Marching,
        options.algorithm
    );
    assert!(!options.right_handed);
    assert_eq!(1, options.frame_count);
}
