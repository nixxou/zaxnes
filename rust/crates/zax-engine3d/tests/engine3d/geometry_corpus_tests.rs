//! The geometry built for every 3D object of the 101 built-in profiles, checked against annex
//! A of SPEC-04B. Every test here needs the profiles (`ZAXNES_PROFILES`).
//!
//! No ROM is needed, as DEC-R2-T7 asks of an acceptance criterion of the clean side: loading a
//! profile builds the geometry of all its patterns (REQ-E3D-600).

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::time::Instant;

use zax_engine3d::engine::ConversionEngine;
use zax_engine3d::instance::InstanceRef;
use zax_profiles::enums::{MeshAlgorithm, ProfileLayer, TextureFormat, VariantGeometry, VariantRenderStyle};
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::tags;
use zax_profiles::values::Vec3F;

use super::support::neutral_assets;

fn all(profile: &GameProfile) -> impl Iterator<Item = &ProfilePattern> {
    profile
        .permanent_patterns
        .iter()
        .chain(&profile.temporary_patterns)
        .map(AsRef::as_ref)
}

fn bump<T: Eq + Hash>(counts: &mut HashMap<T, usize>, key: T) {
    *counts.entry(key).or_default() += 1;
}

fn count<T: Eq + Hash>(counts: &HashMap<T, usize>, key: T) -> usize {
    counts.get(&key).copied().unwrap_or(0)
}

fn is_set(value: Option<Vec3F>) -> bool {
    value.is_some_and(|v| v.x != 0.0 || v.y != 0.0 || v.z != 0.0)
}

fn is_scaled(value: Option<Vec3F>) -> bool {
    value.is_some_and(|v| v.x != 1.0 || v.y != 1.0 || v.z != 1.0)
}

#[derive(Default)]
struct Annex {
    objects_by_geometry: HashMap<VariantGeometry, usize>,
    profiles_by_geometry: HashMap<VariantGeometry, HashSet<String>>,
    depths: HashMap<i32, usize>,
    empty_colors: HashMap<i32, usize>,
    algorithms: HashMap<MeshAlgorithm, usize>,
    layers: HashMap<ProfileLayer, usize>,
    styles: HashMap<VariantRenderStyle, usize>,
    profiles: usize,
    version2: usize,
    patterns: usize,
    objects: usize,
    solid: usize,
    full_warp: usize,
    anti_shadow: usize,
    flip: usize,
    indexed_textures: usize,
    direct_textures: usize,
    resized_textures: usize,
    deformed: usize,
    translucent: usize,
    rotated: usize,
    scaled: usize,
    offset: usize,
    no_cast: usize,
    no_receiv: usize,
    no_recv: usize,
    global_axis: usize,
    global_axis_text: usize,
    triangles: usize,
    vertices: usize,
    built: usize,
}

impl Annex {
    /// Annex A counts the stored parameters, so they are read before the build: a cylinder's
    /// sizeZ becomes D' once its geometry exists (REQ-E3D-452).
    fn count_stored(&mut self, name: &str, profile: &GameProfile) {
        for pattern in all(profile) {
            self.patterns += 1;
            for variant in &pattern.variants {
                self.objects += 1;
                bump(&mut self.objects_by_geometry, variant.geometry);
                self.profiles_by_geometry
                    .entry(variant.geometry)
                    .or_default()
                    .insert(name.to_owned());

                bump(&mut self.depths, variant.depth);
                bump(&mut self.empty_colors, variant.empty_color());
                bump(&mut self.algorithms, variant.mesh_algorithm);
                bump(&mut self.layers, variant.layer);
                bump(&mut self.styles, variant.render_style);

                self.solid += usize::from(variant.solid);
                self.full_warp += usize::from(variant.full_warp);
                self.anti_shadow += usize::from(variant.anti_shadow);
                self.flip += usize::from(variant.flip);
                self.deformed += usize::from(is_set(variant.deform_amplitude));
                self.translucent += usize::from(variant.alpha < 1.0);
                self.rotated += usize::from(is_set(variant.rotation));
                self.scaled += usize::from(is_scaled(variant.scale));
                self.offset += usize::from(is_set(variant.offset));

                let tag_string = variant.tag_string.as_deref().unwrap_or_default();
                if tag_string.contains("_gaxis") {
                    self.global_axis_text += 1;
                }

                for tag in tags::parse(Some(tag_string)) {
                    self.no_cast += usize::from(tag == "!cast");
                    self.no_receiv += usize::from(tag == "!receiv");
                    self.no_recv += usize::from(tag == "!recv");
                    self.global_axis += usize::from(tag == "_gaxis");
                }

                let Some(texture) = variant.texture.as_ref().filter(|texture| !texture.is_null_marker()) else {
                    continue;
                };

                if texture.pixels.as_ref().map(|pixels| pixels.format) == Some(TextureFormat::Direct) {
                    self.direct_textures += 1;
                } else {
                    self.indexed_textures += 1;
                }

                let (cells_x, cells_y) = pattern.grid.as_ref().map_or_else(
                    || pattern.tile_size.map_or((0, 0), |size| (size.x, size.y)),
                    |grid| (grid.width(), grid.height()),
                );
                if texture.width != 8 * cells_x || texture.height != 8 * cells_y {
                    self.resized_textures += 1;
                }
            }
        }
    }

    fn assert_geometry(&self, geometry: VariantGeometry, objects: usize, profiles: usize) {
        assert_eq!(objects, count(&self.objects_by_geometry, geometry));
        assert_eq!(
            profiles,
            self.profiles_by_geometry.get(&geometry).map_or(0, HashSet::len)
        );
    }
}

#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES"]
fn annex_a_every_object_of_the_101_profiles_builds_and_the_counts_match() {
    let mut annex = Annex::default();
    let mut problems = Vec::new();
    let watch = Instant::now();

    for path in neutral_assets::profile_files() {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let profile = neutral_assets::load_path(&path);
        annex.profiles += 1;
        annex.version2 += usize::from(profile.is_v2_format());
        annex.count_stored(&name, &profile);

        let mut engine = ConversionEngine::new();
        engine.use_geometry();
        if let Err(error) = engine.load_profile(&profile, true) {
            problems.push(format!("{name}: {error}"));
            continue;
        }

        let progress = engine.geometry().progress().value();
        if progress < 1.0 {
            problems.push(format!("{name}: the build stopped at {progress}"));
        }

        let builder = engine.geometry_builder().unwrap();
        let patterns = engine.patterns();
        for id in patterns.all() {
            for index in 0..patterns.get(id).unwrap().instances().len() {
                let Some(object) = builder.of(InstanceRef { pattern: id, index }) else {
                    problems.push(format!("{name}: an object has no geometry"));
                    continue;
                };

                annex.built += 1;
                for mesh in object.meshes() {
                    annex.triangles += mesh.triangle_count();
                    annex.vertices += mesh.vertex_count();
                }
            }
        }

        engine.unload();
    }

    let elapsed = watch.elapsed();
    let shown: Vec<_> = problems.iter().take(20).cloned().collect();
    assert!(problems.is_empty(), "{}", shown.join("\n"));

    // Header of annex A.
    assert_eq!(101, annex.profiles);
    assert_eq!(100, annex.version2);
    assert_eq!(67832, annex.patterns);
    assert_eq!(74389, annex.objects);
    assert_eq!(74389, annex.built);

    // Geometry types: objects and profiles that use them.
    annex.assert_geometry(VariantGeometry::Default, 63633, 100);
    annex.assert_geometry(VariantGeometry::VerticalCylinder, 7121, 99);
    annex.assert_geometry(VariantGeometry::HorizontalCylinder, 2373, 87);
    annex.assert_geometry(VariantGeometry::Cube, 872, 64);
    annex.assert_geometry(VariantGeometry::HalfHorizontalCylinder, 246, 36);
    annex.assert_geometry(VariantGeometry::HalfVerticalCylinder, 144, 30);

    // Options.
    assert_eq!(13475, annex.solid);
    assert_eq!(10518, annex.full_warp);
    assert_eq!(2568, annex.anti_shadow);
    assert_eq!(311, annex.flip);

    // Mesh algorithm and transparent colour.
    assert_eq!(73356, count(&annex.algorithms, MeshAlgorithm::FromSettings));
    assert_eq!(400, count(&annex.algorithms, MeshAlgorithm::Greedy));
    assert_eq!(633, count(&annex.algorithms, MeshAlgorithm::Marching));
    assert_eq!(0, count(&annex.algorithms, MeshAlgorithm::GreedyMulticolor));

    assert_eq!(62717, count(&annex.empty_colors, 0));
    assert_eq!(271, count(&annex.empty_colors, 1));
    assert_eq!(109, count(&annex.empty_colors, 2));
    assert_eq!(245, count(&annex.empty_colors, 3));
    assert_eq!(11047, count(&annex.empty_colors, 4));

    // Textures, deformation, transparency, render style.
    assert_eq!(13539, annex.indexed_textures);
    assert_eq!(142, annex.direct_textures);
    assert_eq!(4, annex.resized_textures);
    assert_eq!(289, annex.deformed);
    assert_eq!(628, annex.translucent);
    assert_eq!(1639, count(&annex.styles, VariantRenderStyle::Hidden));
    assert_eq!(72744, count(&annex.styles, VariantRenderStyle::Normal));
    assert_eq!(6, count(&annex.styles, VariantRenderStyle::ShadowOnly));

    // Layers and transforms.
    assert_eq!(30762, count(&annex.layers, ProfileLayer::Unidentified));
    assert_eq!(156, count(&annex.layers, ProfileLayer::Ui));
    assert_eq!(32456, count(&annex.layers, ProfileLayer::L1));
    assert_eq!(9534, count(&annex.layers, ProfileLayer::L2));
    assert_eq!(1318, count(&annex.layers, ProfileLayer::L3));
    assert_eq!(163, count(&annex.layers, ProfileLayer::L4));
    assert_eq!(3248, annex.rotated);
    assert_eq!(784, annex.scaled);
    assert_eq!(15080, annex.offset);

    // Tags. Annex A counts 42 objects whose tag string holds the text `_gaxis`, but one of
    // them — in "Road Fighter (Japan)" — stores `mc._gaxis`, a single tag that is not
    // `_gaxis`. REQ-E3D-407 asks for a tag of the comma-separated list, so 41 objects really
    // take the swapped rotation order (Q-018).
    assert_eq!(980, annex.no_cast);
    assert_eq!(160, annex.no_receiv);
    assert_eq!(406, annex.no_recv);
    assert_eq!(42, annex.global_axis_text);
    assert_eq!(41, annex.global_axis);

    // Depth distribution of annex A.
    for (depth, expected) in [
        (0, 51),
        (1, 2007),
        (2, 26398),
        (3, 13084),
        (4, 10589),
        (6, 1974),
        (8, 5895),
        (12, 1499),
        (16, 3224),
        (24, 799),
        (32, 1687),
        (48, 2172),
    ] {
        assert_eq!(expected, count(&annex.depths, depth), "depth {depth}");
    }
    assert_eq!(Some(96), annex.depths.keys().copied().max());

    assert!(annex.triangles > 0);
    assert!(annex.vertices > 0);

    // Reported, not asserted: the reference machine of SPEC-12 is not this one.
    println!(
        "annex A: {} objects, {} triangles, {} vertices in {:.1} s",
        annex.objects,
        annex.triangles,
        annex.vertices,
        elapsed.as_secs_f64()
    );
}

#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES"]
fn req_e3d_564_the_biggest_built_in_textures_stay_within_the_capacity() {
    let mut widest = 0;
    let mut tallest = 0;
    let mut texels = 0i64;

    for path in neutral_assets::profile_files() {
        let profile = neutral_assets::load_path(&path);
        let mut profile_texels = 0i64;
        for pattern in all(&profile) {
            for variant in &pattern.variants {
                let Some(texture) = variant.texture.as_ref().filter(|texture| !texture.is_null_marker()) else {
                    continue;
                };

                widest = widest.max(texture.width);
                tallest = tallest.max(texture.height);
                profile_texels += i64::from(texture.width) * i64::from(texture.height);
            }
        }

        texels = texels.max(profile_texels);
    }

    // No object goes past 512 x 512 texels, and no profile past the simultaneous budget.
    assert!(widest <= 512, "a texture is {widest} texels wide");
    assert!(tallest <= 512, "a texture is {tallest} texels tall");
    assert!(texels <= 64 * 512 * 512, "a profile needs {texels} texels at once");
}
