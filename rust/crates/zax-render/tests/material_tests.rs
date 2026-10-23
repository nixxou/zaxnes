//! Texturing of the tiles: the indexed palette lookup of SPEC-06 REQ-SCN-034 and the texel
//! addressing of SPEC-04B REQ-E3D-506 and REQ-E3D-563, checked on captures where every texel
//! has a known place (port of `MaterialTests.cs`).

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{solid, GlFixture, SceneFixture, Tile};
use zax_profiles::enums::ProfileLayer;

const BLACK: i32 = 0;

#[test]
fn req_e3d_506_the_texture_of_a_tile_is_not_flipped_or_mirrored() {
    // Colour 1 fills the top-left quadrant, colour 2 the bottom-right one.
    let quadrants = quadrants();

    let built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x16) // red,  top-left
        .palette(2, 0x12) // blue, bottom-right
        .pattern(quadrants, 1, ProfileLayer::L1)
        .rect(quadrants, 14, 14, 2, 2)
        .build();
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let capture = gl.render_built(&mut renderer, &built);
    let (first_row, last_row) = capture.row_span(BLACK);
    let (first_column, last_column) = capture.column_span(BLACK);

    // Sample well inside the two quadrants of the top-left tile of the 2 x 2 block.
    let quarter_x = first_column + ((last_column - first_column) / 8);
    let quarter_y = first_row + ((last_row - first_row) / 8);
    let three_quarter_x = last_column - ((last_column - first_column) / 8);
    let three_quarter_y = last_row - ((last_row - first_row) / 8);

    let (top_left_r, _, top_left_b) = capture.at(quarter_x, quarter_y);
    let (bottom_right_r, _, bottom_right_b) = capture.at(three_quarter_x, three_quarter_y);

    assert!(
        top_left_r > top_left_b,
        "the top-left quadrant is not red: ({top_left_r}, _, {top_left_b})"
    );
    assert!(
        bottom_right_b > bottom_right_r,
        "the bottom-right quadrant is not blue: ({bottom_right_r}, _, {bottom_right_b})"
    );
}

#[test]
fn req_e3d_506_each_cell_of_a_two_cell_pattern_keeps_its_own_texels() {
    // A two-cell pattern [A B] whose texture is 16 x 8 texels: A has to stay on the left.
    let left = solid(1, 0);
    let right = solid(2, 1);

    let mut fixture = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x16)
        .palette(2, 0x12)
        .pattern_row(1, ProfileLayer::L1, 1.0, &[left, right]);
    for row in 12..18 {
        fixture = fixture
            .tile(left, 15 * 8, row * 8, 0)
            .tile(right, 16 * 8, row * 8, 0);
    }

    let built = fixture.build();
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let capture = gl.render_built(&mut renderer, &built);
    let (first_column, last_column) = capture.column_span(BLACK);
    let middle_y = capture.height / 2;

    let (left_r, _, left_b) =
        capture.at(first_column + ((last_column - first_column) / 4), middle_y);
    let (right_r, _, right_b) =
        capture.at(last_column - ((last_column - first_column) / 4), middle_y);

    assert!(
        left_r > left_b,
        "the left cell is not red: ({left_r}, _, {left_b})"
    );
    assert!(
        right_b > right_r,
        "the right cell is not blue: ({right_r}, _, {right_b})"
    );
}

#[test]
fn req_scn_055_in_2d_mode_a_learned_band_keeps_its_tiles_in_order() {
    // No profile at all: every tile image is learned as it appears, and the background pass
    // groups them into a band (SPEC-04 REQ-E3D-381). Four tiles of four different colours in a
    // row have to come out in that order.
    let a = solid(1, 0);
    let b = solid(2, 1);
    let c = solid(3, 2);
    let d = solid(1, 3);

    let mut fixture = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x16) // red
        .palette(2, 0x1A) // green
        .palette(3, 0x12); // blue
    fixture.two_dimensional_mode = true;

    for row in 13..17 {
        fixture = fixture
            .tile(a, 12 * 8, row * 8, 0)
            .tile(b, 13 * 8, row * 8, 0)
            .tile(c, 14 * 8, row * 8, 0)
            .tile(d, 15 * 8, row * 8, 0);
    }

    let built = fixture.build();
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let capture = gl.render_built(&mut renderer, &built);
    let (first_column, last_column) = capture.column_span(BLACK);
    let middle_y = capture.height / 2;
    let span = last_column - first_column + 1;

    let first = capture.at(first_column + (span / 8), middle_y);
    let second = capture.at(first_column + (3 * span / 8), middle_y);
    let third = capture.at(first_column + (5 * span / 8), middle_y);

    assert!(
        first.0 > first.1 && first.0 > first.2,
        "cell 1 is not red: {first:?}"
    );
    assert!(
        second.1 > second.0 && second.1 > second.2,
        "cell 2 is not green: {second:?}"
    );
    assert!(
        third.2 > third.0 && third.2 > third.1,
        "cell 3 is not blue: {third:?}"
    );
}

#[test]
fn req_scn_034_a_background_tile_reads_its_colour_at_4p_plus_the_texel() {
    // Palette 2 of the background, colour index 1, is entry 2*4 + 1 = 9 of the frame palette.
    let built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .palette(9, 0x16)
        .pattern(solid(1, 0), 1, ProfileLayer::L1)
        .rect_palette(solid(1, 0), 12, 12, 8, 6, 2)
        .build();
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let capture = gl.render_built(&mut renderer, &built);
    let (r, g, b) = capture.at(capture.width / 2, capture.height / 2);

    // NES 0x16 is (252, 56, 0); lit at N.L = 0.577 it stays strongly red.
    assert!(
        r > 150 && g < 100 && b < 60,
        "expected the palette-2 red, got ({r}, {g}, {b})"
    );
}

#[test]
fn req_scn_034_a_sprite_tile_reads_its_colour_at_4_times_palette_plus_four() {
    // Palette 1 of the sprites, colour index 1, is entry (1 + 4) * 4 + 1 = 21.
    let mut fixture = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .palette(21, 0x1A)
        .pattern(solid(1, 0), 1, ProfileLayer::L1);

    for row in 12..18 {
        for column in 12..20 {
            fixture.push_sprite(solid(1, 0), column * 8, row * 8, 1, false);
        }
    }

    let built = fixture.build();
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let capture = gl.render_built(&mut renderer, &built);
    let (r, g, b) = capture.at(capture.width / 2, capture.height / 2);

    // NES 0x1A is (0, 168, 0): a strong green.
    assert!(
        g > r + 40 && g > b + 40,
        "expected the sprite palette green, got ({r}, {g}, {b})"
    );
}

#[test]
fn req_e3d_565_a_palette_change_shows_on_the_next_frame_without_a_rebuild() {
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x16)
        .pattern(solid(1, 0), 1, ProfileLayer::L1)
        .rect(solid(1, 0), 12, 12, 8, 6)
        .build();
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let red = gl.render_built(&mut renderer, &built);
    let uploads_after_first = renderer.statistics().mesh_uploads;

    // Change colour 1 of the frame palette only; the meshes must not be rebuilt.
    assert!(built.engine.set_color_index(1, 0x12, false));
    built.rebuild_draw_list();

    let blue = gl.render_built(&mut renderer, &built);

    let (red_r, _, red_b) = red.at(red.width / 2, red.height / 2);
    let (blue_r, _, blue_b) = blue.at(blue.width / 2, blue.height / 2);

    assert!(red_r > red_b);
    assert!(blue_b > blue_r);
    assert_eq!(uploads_after_first, renderer.statistics().mesh_uploads);
}

/// A tile image whose top-left 4 × 4 pixels are colour 1 and whose bottom-right 4 × 4 are
/// colour 2; the rest is transparent.
fn quadrants() -> Tile {
    let mut low = 0_u64;
    let mut high = 0_u64;
    for y in 0..8 {
        for x in 0..8 {
            let color = if x < 4 && y < 4 {
                1
            } else if x >= 4 && y >= 4 {
                2
            } else {
                0
            };
            let bit = (y << 3) | (7 - x);
            if color & 1 != 0 {
                low |= 1 << bit;
            }
            if color & 2 != 0 {
                high |= 1 << bit;
            }
        }
    }

    (low, high)
}
