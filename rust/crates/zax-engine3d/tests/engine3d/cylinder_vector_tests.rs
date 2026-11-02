//! The test vectors of SPEC-04B §5.3: the four cylindrical geometry types (REQ-E3D-444 to
//! 452).
//!
//! Voxels are listed in the plane of the section, `(i, k)` for the V types (the row `j` being
//! fixed) and `(j, k)` for the H types, with the texel assigned to them — the column for V, the
//! row for H.

use zax_engine3d::instance::PatternInstance;
use zax_engine3d::settings::{Geo3DType, RenderAlgorithm};
use zax_engine3d::vectors::Vec3;

use super::support::geo::{self, close, object_with, rectangles, section, voxels, Object};

const GREEDY: RenderAlgorithm = RenderAlgorithm::Greedy;

fn cylinder(depth: i32, geometry: Geo3DType) -> PatternInstance {
    object_with(
        depth,
        Object {
            geometry,
            ..Object::default()
        },
    )
}

fn column(height: usize) -> zax_engine3d::pattern::EnginePattern {
    geo::one_tile(geo::tile(&vec!["1......."; height]))
}

fn texel_at(section: &[(i32, i32, i32)], line: i32, depth: i32) -> i32 {
    section
        .iter()
        .find(|&&(l, d, _)| l == line && d == depth)
        .map_or_else(
            || panic!("no voxel at ({line}, {depth})"),
            |&(_, _, texel)| texel,
        )
}

#[test]
fn tv_y1_a_row_of_eight_becomes_a_ring_of_twenty_four_voxels() {
    // REQ-E3D-445, 446, 447, 448.
    let pattern = geo::one_tile(geo::tile(&["11111111"]));
    let instance = cylinder(8, Geo3DType::VCylinder);
    let (fields, effective_depth) = geo::fields(&pattern, &instance, GREEDY);

    assert_eq!(8, effective_depth);
    assert_eq!(
        voxels(&[
            (0, 3, 0),
            (0, 4, 7),
            (1, 1, 1),
            (1, 2, 1),
            (1, 5, 6),
            (1, 6, 6),
            (2, 0, 2),
            (2, 1, 2),
            (2, 6, 5),
            (2, 7, 5),
            (3, 0, 3),
            (3, 7, 4),
            (4, 0, 4),
            (4, 7, 3),
            (5, 0, 5),
            (5, 1, 5),
            (5, 6, 2),
            (5, 7, 2),
            (6, 1, 6),
            (6, 2, 6),
            (6, 5, 1),
            (6, 6, 1),
            (7, 3, 7),
            (7, 4, 0),
        ]),
        section(&fields[0], 0, true)
    );

    let mesh = geo::mesh(&pattern, &instance, GREEDY, 0);
    assert_eq!(64, rectangles(&mesh));
}

#[test]
fn tv_y2_a_solid_row_of_eight_fills_forty_eight_voxels() {
    // REQ-E3D-449.
    let pattern = geo::one_tile(geo::tile(&["11111111"]));
    let instance = object_with(
        8,
        Object {
            geometry: Geo3DType::VCylinder,
            solid: true,
            ..Object::default()
        },
    );
    let (fields, _) = geo::fields(&pattern, &instance, GREEDY);
    let section = section(&fields[0], 0, true);

    // Shape (normative): i = 0 and 7 keep k in {3, 4}, i = 1 and 6 keep k in [1, 6], i = 2 to
    // 5 keep the whole depth.
    let mut expected = Vec::new();
    for i in [0, 7] {
        expected.extend([(i, 3), (i, 4)]);
    }

    for i in [1, 6] {
        expected.extend((1..=6).map(|k| (i, k)));
    }

    for i in [2, 3, 4, 5] {
        expected.extend((0..=7).map(|k| (i, k)));
    }

    expected.sort_unstable();
    let mut actual: Vec<(i32, i32)> = section
        .iter()
        .map(|&(line, depth, _)| (line, depth))
        .collect();
    actual.sort_unstable();
    assert_eq!(48, actual.len());
    assert_eq!(expected, actual);

    // Control texels of the order of REQ-E3D-449 (P2).
    assert_eq!(0, texel_at(&section, 1, 3));
    assert_eq!(7, texel_at(&section, 1, 4));
    assert_eq!(2, texel_at(&section, 3, 1));
    assert_eq!(5, texel_at(&section, 4, 1));
    assert_eq!(1, texel_at(&section, 2, 2));

    assert_eq!(30, rectangles(&geo::mesh(&pattern, &instance, GREEDY, 0)));
}

#[test]
fn tv_y3_horizontal_cylinders_mirror_the_vertical_ones() {
    // REQ-E3D-444, 452: a column of four pixels with sizeZ = 8 ends up with D' = 4.
    let pattern = column(4);
    let instance = cylinder(8, Geo3DType::HCylinder);
    let (fields, effective_depth) = geo::fields(&pattern, &instance, GREEDY);

    assert_eq!(4, effective_depth);
    assert_eq!(
        voxels(&[
            (4, 3, 3),
            (4, 4, 0),
            (5, 2, 2),
            (5, 5, 1),
            (6, 2, 1),
            (6, 5, 2),
            (7, 3, 0),
            (7, 4, 3),
        ]),
        section(&fields[0], 0, false)
    );
    assert_eq!(24, rectangles(&geo::mesh(&pattern, &instance, GREEDY, 0)));

    // A full column of eight with sizeZ = 8: the 24 voxels of TV-Y1, mirrored.
    let full = column(8);
    let (full_fields, full_depth) = geo::fields(&full, &instance, GREEDY);
    assert_eq!(8, full_depth);
    assert_eq!(24, section(&full_fields[0], 0, false).len());
    assert_eq!(64, rectangles(&geo::mesh(&full, &instance, GREEDY, 0)));

    // The same column with sizeZ = 4.
    let shallow = cylinder(4, Geo3DType::HCylinder);
    let (shallow_fields, shallow_depth) = geo::fields(&full, &shallow, GREEDY);
    assert_eq!(4, shallow_depth);
    assert_eq!(
        voxels(&[
            (0, 1, 0),
            (0, 2, 7),
            (1, 1, 6),
            (1, 2, 1),
            (2, 0, 5),
            (2, 3, 2),
            (3, 0, 4),
            (3, 3, 3),
            (4, 0, 3),
            (4, 3, 4),
            (5, 0, 2),
            (5, 3, 5),
            (6, 1, 1),
            (6, 2, 6),
            (7, 1, 7),
            (7, 2, 0),
        ]),
        section(&shallow_fields[0], 0, false)
    );
}

#[test]
fn tv_y4_solid_full_warp_fills_each_column_with_its_own_colour() {
    // REQ-E3D-450 as AMD-048 amends it: the depth interval of a point is `{z : zm <= z <= zp}`,
    // **empty** when the bounds are inverted — which replaces the reading of Q-015 (AMD-008).
    //
    // **Both cases of TV-Y4 are unchanged by that**, and the reason is worth saying in place,
    // because Q-015 and Q-082 §4.1 both claimed the contrary: the region-2 points whose bounds
    // are inverted are followed by a point of the **same `u` and the same interval** — `(3, 2)`
    // at D = 4, `(7, 2)` and `(7, 3)` at D = 8 — which writes the very same voxels. Those two
    // tickets left that point out of their own tables. A full row at D = 4 has no such point,
    // which is TV-Y10 below.
    let warped = |depth| {
        object_with(
            depth,
            Object {
                geometry: Geo3DType::VCylinder,
                solid: true,
                full_warp: true,
                ..Object::default()
            },
        )
    };
    let pattern = geo::one_tile(geo::tile(&["11111111"]));
    let (fields, _) = geo::fields(&pattern, &warped(8), GREEDY);
    let full = section(&fields[0], 0, true);

    assert_eq!(48, full.len());
    for &(line, _, texel) in &full {
        // Every voxel of a column carries the texel of that column.
        assert_eq!(line, texel);
    }

    // Row `1123....`, D = 4.
    let four = geo::one_tile(geo::tile(&["1123...."]));
    let shallow = warped(4);
    let (four_fields, _) = geo::fields(&four, &shallow, GREEDY);
    assert_eq!(
        voxels(&[
            (0, 1, 0),
            (0, 2, 0),
            (1, 0, 1),
            (1, 1, 1),
            (1, 2, 1),
            (1, 3, 1),
            (2, 0, 2),
            (2, 1, 2),
            (2, 2, 2),
            (2, 3, 2),
            (3, 1, 3),
            (3, 2, 3),
        ]),
        section(&four_fields[0], 0, true)
    );
    assert_eq!(18, rectangles(&geo::mesh(&four, &shallow, GREEDY, 0)));
}

#[test]
fn tv_y5_a_degenerate_section_is_a_single_voxel() {
    // REQ-E3D-451.
    let row = geo::one_tile(geo::tile(&["11111111"]));
    let flat = cylinder(1, Geo3DType::VCylinder);
    let (fields, depth) = geo::fields(&row, &flat, GREEDY);

    assert_eq!(1, depth);
    assert_eq!(voxels(&[(3, 0, 3)]), section(&fields[0], 0, true));

    let mesh = geo::mesh(&row, &flat, GREEDY, 0);
    assert!(close(Vec3::new(-1.0, 3.0, -0.5), mesh.bounds_min()));
    assert!(close(Vec3::new(0.0, 4.0, 0.5), mesh.bounds_max()));

    // A single pixel with D = 5: one voxel in the middle slice, D' = 1.
    let pixel = geo::one_tile(geo::tile(&["1......."]));
    let deep = cylinder(5, Geo3DType::VCylinder);
    let (pixel_fields, pixel_depth) = geo::fields(&pixel, &deep, GREEDY);
    assert_eq!(1, pixel_depth);
    assert_eq!(voxels(&[(0, 2, 0)]), section(&pixel_fields[0], 0, true));
}

#[test]
fn tv_y6_anti_shadow_moves_the_texels_of_the_flanks() {
    // REQ-E3D-447.
    let pattern = geo::one_tile(geo::tile(&["12333321"]));
    let plain = cylinder(8, Geo3DType::VCylinder);
    let shaded = object_with(
        8,
        Object {
            geometry: Geo3DType::VCylinder,
            anti_shadow: true,
            ..Object::default()
        },
    );

    let without = section(&geo::fields(&pattern, &plain, GREEDY).0[0], 0, true);
    let with = section(&geo::fields(&pattern, &shaded, GREEDY).0[0], 0, true);

    // Same positions, only the texels of the region 2 voxels move.
    let positions = |list: &[(i32, i32, i32)]| -> Vec<(i32, i32)> {
        list.iter().map(|&(l, d, _)| (l, d)).collect()
    };
    assert_eq!(positions(&without), positions(&with));

    assert_eq!(0, texel_at(&without, 0, 3));
    assert_eq!(7, texel_at(&without, 0, 4));
    assert_eq!(7, texel_at(&without, 7, 3));
    assert_eq!(0, texel_at(&without, 7, 4));

    assert_eq!(2, texel_at(&with, 0, 3));
    assert_eq!(5, texel_at(&with, 0, 4));
    assert_eq!(5, texel_at(&with, 7, 3));
    assert_eq!(2, texel_at(&with, 7, 4));

    // The other voxels keep their texel.
    for &(line, depth, texel) in &without {
        if matches!(line, 0 | 7) && matches!(depth, 3 | 4) {
            continue;
        }

        assert_eq!(texel, texel_at(&with, line, depth));
    }
}

#[test]
fn tv_y7_quadrant_points_of_texel_zero_are_skipped() {
    // REQ-E3D-448: the test is on the texel value, not on the transparent colour.
    let pattern = geo::one_tile(geo::tile(&["1101...."]));
    let instance = object_with(
        4,
        Object {
            geometry: Geo3DType::VCylinder,
            empty_color: 4,
            ..Object::default()
        },
    );
    let (fields, _) = geo::fields(&pattern, &instance, GREEDY);

    assert_eq!(
        voxels(&[
            (0, 2, 0),
            (1, 1, 1),
            (3, 0, 3),
            (4, 3, 3),
            (6, 2, 1),
            (7, 1, 0)
        ]),
        section(&fields[0], 0, true)
    );
}

#[test]
fn tv_y8_half_cylinders_leave_their_flat_face_open() {
    // REQ-E3D-445, 447: A = 2L, the centre at s or at f depending on flip.
    let row = geo::one_tile(geo::tile(&["1111...."]));
    let column = column(4);

    let half_v = cylinder(8, Geo3DType::HalfVCylinder);
    let (half_v_fields, half_v_depth) = geo::fields(&row, &half_v, GREEDY);
    assert_eq!(7, half_v_depth);
    assert_eq!(
        voxels(&[
            (0, 1, 0),
            (0, 6, 0),
            (1, 1, 1),
            (1, 6, 1),
            (2, 1, 2),
            (2, 2, 2),
            (2, 5, 2),
            (2, 6, 2),
            (3, 3, 3),
            (3, 4, 3),
        ]),
        section(&half_v_fields[0], 0, true)
    );
    assert_eq!(26, rectangles(&geo::mesh(&row, &half_v, GREEDY, 0)));

    let half_v_flipped = object_with(
        8,
        Object {
            geometry: Geo3DType::HalfVCylinder,
            flip: true,
            ..Object::default()
        },
    );
    let (flipped_fields, flipped_depth) = geo::fields(&row, &half_v_flipped, GREEDY);
    assert_eq!(7, flipped_depth);
    assert_eq!(
        voxels(&[
            (0, 3, 0),
            (0, 4, 0),
            (1, 1, 1),
            (1, 2, 1),
            (1, 5, 1),
            (1, 6, 1),
            (2, 1, 2),
            (2, 6, 2),
            (3, 1, 3),
            (3, 6, 3),
        ]),
        section(&flipped_fields[0], 0, true)
    );
    assert_eq!(26, rectangles(&geo::mesh(&row, &half_v_flipped, GREEDY, 0)));

    let horizontal = cylinder(8, Geo3DType::HalfHCylinder);
    let (horizontal_fields, horizontal_depth) = geo::fields(&column, &horizontal, GREEDY);
    assert_eq!(7, horizontal_depth);
    assert_eq!(
        voxels(&[
            (4, 1, 3),
            (4, 6, 3),
            (5, 1, 2),
            (5, 6, 2),
            (6, 1, 1),
            (6, 2, 1),
            (6, 5, 1),
            (6, 6, 1),
            (7, 3, 0),
            (7, 4, 0),
        ]),
        section(&horizontal_fields[0], 0, false)
    );
    assert_eq!(26, rectangles(&geo::mesh(&column, &horizontal, GREEDY, 0)));
}

#[test]
fn section_control_cases_of_paragraph_5_3() {
    // Row `111.....`, VCYLINDER, D = 8 -> six voxels, D' = 3.
    let three = geo::one_tile(geo::tile(&["111....."]));
    let instance = cylinder(8, Geo3DType::VCylinder);
    let (fields, depth) = geo::fields(&three, &instance, GREEDY);

    assert_eq!(3, depth);
    assert_eq!(
        voxels(&[
            (0, 3, 0),
            (0, 4, 2),
            (1, 3, 1),
            (1, 4, 1),
            (2, 3, 2),
            (2, 4, 0)
        ]),
        section(&fields[0], 0, true)
    );

    // Row `11111111`, D = 2 -> sixteen voxels and six rectangles. The texels of the two
    // extreme columns are those of REQ-E3D-446 and 447, not those of the parenthetical of
    // paragraph 5.3, which the region 2 point overwrites (Q-014).
    let row = geo::one_tile(geo::tile(&["11111111"]));
    let shallow = cylinder(2, Geo3DType::VCylinder);
    let (shallow_fields, shallow_depth) = geo::fields(&row, &shallow, GREEDY);
    let section = section(&shallow_fields[0], 0, true);

    assert_eq!(2, shallow_depth);
    assert_eq!(16, section.len());
    assert_eq!(6, rectangles(&geo::mesh(&row, &shallow, GREEDY, 0)));

    for i in 1..=6 {
        assert_eq!(i, texel_at(&section, i, 0));
        assert_eq!(7 - i, texel_at(&section, i, 1));
    }
}

/// TV-Y9 (AMD-048 §3): the ring ropes of "Punch-Out!!". A wrapped solid `HCYLINDER` at `D = 3`
/// whose segment is two pixels long keeps **one** slice of depth, not three, because the
/// region-2 point at `q = 0` has `z+ = 0` and `z- = 2` — inverted bounds, so an empty interval.
///
/// The segment is fixed at image rows 3 and 4 with **two different colours**, which is what
/// makes "le texel de sa propre rangée" verifiable (R2 48-2, 48-6: the vector of v1.0 was
/// written in a convention that made it false anywhere but `r = 3`).
#[test]
fn tv_y9_a_wrapped_solid_rope_of_two_pixels_keeps_one_slice_of_depth() {
    let warped = |depth, geometry| {
        object_with(
            depth,
            Object {
                geometry,
                solid: true,
                full_warp: true,
                ..Object::default()
            },
        )
    };
    // Column 0 only, rows 3 and 4, colours 1 and 2.
    let rope = geo::one_tile(geo::tile(&[
        "........", "........", "........", "1.......", "2.......",
    ]));
    let instance = warped(3, Geo3DType::HCylinder);
    let (fields, depth) = geo::fields(&rope, &instance, GREEDY);

    // Exactly two voxels, on the middle slice, each with the texel of its own image row
    // (REQ-E3D-421: j = 7 - y, so rows 3 and 4 are j = 4 and j = 3).
    assert_eq!(
        voxels(&[(4, 1, 3), (3, 1, 4)]),
        section(&fields[0], 0, false)
    );
    // D' = B + 1 = 2 (REQ-E3D-452), deduced from the rule and not from the box: REQ-E3D-504
    // keeps the local centring of the **construction** depth D = 3 (R2 48-7).
    assert_eq!(2, depth);

    let mesh = geo::mesh(&rope, &instance, GREEDY, 0);
    assert_eq!(6, rectangles(&mesh));
    assert!(
        close(Vec3::new(-4.0, -1.0, -0.5), mesh.bounds_min()),
        "{}",
        mesh.bounds_min()
    );
    assert!(
        close(Vec3::new(-3.0, 1.0, 0.5), mesh.bounds_max()),
        "{}",
        mesh.bounds_max()
    );

    // A whole tile of such columns: 8 x 2 x 1 units, the figure AMD-048 §3 publishes.
    let tile = geo::one_tile(geo::tile(&[
        "........", "........", "........", "11111111", "22222222",
    ]));
    let (tile_fields, tile_depth) = geo::fields(&tile, &instance, GREEDY);
    assert_eq!(2, tile_depth);
    assert_eq!(16, voxels(&all_voxels(&tile_fields[0])).len());
    let tile_mesh = geo::mesh(&tile, &instance, GREEDY, 0);
    assert_eq!(6, rectangles(&tile_mesh));
    assert!(
        close(Vec3::new(-4.0, -1.0, -0.5), tile_mesh.bounds_min()),
        "{}",
        tile_mesh.bounds_min()
    );
    assert!(
        close(Vec3::new(4.0, 1.0, 0.5), tile_mesh.bounds_max()),
        "{}",
        tile_mesh.bounds_max()
    );

    // The asymmetry AMD-048 assumes: the three other flag pairs are **unchanged**, and the same
    // two-pixel segment still occupies all three slices there (REQ-E3D-448, REQ-E3D-449).
    for (solid, full_warp) in [(false, false), (false, true), (true, false)] {
        let other = object_with(
            3,
            Object {
                geometry: Geo3DType::HCylinder,
                solid,
                full_warp,
                ..Object::default()
            },
        );
        let (other_fields, _) = geo::fields(&rope, &other, GREEDY);
        let slices: std::collections::BTreeSet<i32> = section(&other_fields[0], 0, false)
            .iter()
            .map(|&(_, k, _)| k)
            .collect();
        assert_eq!(
            vec![0, 1, 2],
            slices.into_iter().collect::<Vec<_>>(),
            "solid {solid}, full_warp {full_warp} still fills three slices"
        );
    }
}

/// TV-Y10 (AMD-048 §3, after R2 48-1): the case the amendment's own evidence did not cover —
/// an **even** depth. A full row of 8 pixels at `D = 4`, wrapped and solid, loses its two
/// outermost columns and is 6 units wide instead of 8, because region 1 stops at `u+ = 6` and
/// region 2 holds only its two inverted points (`2A = 14 > B^2 = 9`).
///
/// The rectangle count does **not** discriminate — it is 18 under either reading — so the voxels
/// and the local box are what this vector asserts.
#[test]
fn tv_y10_an_even_depth_loses_the_outermost_columns_of_a_full_row() {
    let row = geo::one_tile(geo::tile(&["11111111"]));
    let instance = object_with(
        4,
        Object {
            geometry: Geo3DType::VCylinder,
            solid: true,
            full_warp: true,
            ..Object::default()
        },
    );
    let (fields, depth) = geo::fields(&row, &instance, GREEDY);
    let found = section(&fields[0], 0, true);

    let mut expected = Vec::new();
    for i in 2..=5 {
        for k in 0..=3 {
            expected.push((i, k, i));
        }
    }
    for i in [1, 6] {
        for k in [1, 2] {
            expected.push((i, k, i));
        }
    }
    assert_eq!(20, expected.len());
    assert_eq!(voxels(&expected), found);
    assert!(
        !found.iter().any(|&(i, _, _)| i == 0 || i == 7),
        "no voxel in the outermost columns"
    );
    assert_eq!(4, depth);

    let mesh = geo::mesh(&row, &instance, GREEDY, 0);
    assert_eq!(18, rectangles(&mesh));
    assert!(
        close(Vec3::new(-3.0, 3.0, -2.0), mesh.bounds_min()),
        "{}",
        mesh.bounds_min()
    );
    assert!(
        close(Vec3::new(3.0, 4.0, 2.0), mesh.bounds_max()),
        "{}",
        mesh.bounds_max()
    );
}

/// Every voxel of a field, whatever its line: what a tile of identical cylinder segments needs.
fn all_voxels(field: &zax_engine3d::geometry::tile_voxels::TileVoxels) -> Vec<(i32, i32, i32)> {
    use zax_engine3d::geometry::tile_voxels::{AXIS_X, SIZE_X, SIZE_Y};
    let mut found = Vec::new();
    for i in 0..SIZE_X {
        for j in 0..SIZE_Y {
            for k in 0..field.depth() {
                if field.texel(i, j, k, AXIS_X).is_some() {
                    found.push((i, j, k));
                }
            }
        }
    }
    found
}

#[test]
fn req_e3d_452_a_cylinder_without_a_segment_has_no_depth() {
    let blank = geo::one_tile(geo::blank());
    let instance = cylinder(8, Geo3DType::VCylinder);
    let (fields, depth) = geo::fields(&blank, &instance, GREEDY);

    assert_eq!(0, depth);
    assert_eq!(section(&fields[0], 0, true), [] as [(i32, i32, i32); 0]);
}
