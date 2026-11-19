//! The experimental smooth scrolling in 3D (owner request, 2026-11-20): which tiles of the
//! playfield band slide between two frames.

use zax_render::scene::ThreeDShift;

#[test]
fn the_band_slides_but_not_the_status_bar_nor_the_sprites_that_follow_the_camera() {
    let shift = ThreeDShift {
        amount: 0.75,
        start: 16,
        end: 200,
        steady: vec![3, 67],
    };
    // A background tile and a sprite of the band move.
    assert_eq!(Some(0.75), shift.of_tile(16, -1));
    assert_eq!(Some(0.75), shift.of_tile(199, 5));
    // The status bar, above or under the band, stays.
    assert_eq!(None, shift.of_tile(8, -1));
    assert_eq!(None, shift.of_tile(200, -1));
    // A sprite that follows the camera (the player) stays, the lower half of an 8 x 16 too.
    assert_eq!(None, shift.of_tile(100, 3));
    assert_eq!(None, shift.of_tile(108, 67));
}
