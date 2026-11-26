//! Lot HD-PACK: the playfield's scroll an HD pack's backgrounds with a scroll ratio follow
//! (`NesCore::playfield_origin`).

mod common;

use common::TileFixture;

#[test]
fn the_playfield_origin_is_the_scroll_the_picture_was_drawn_with() {
    let mut fixture = TileFixture::new();
    fixture.core.set_tracks_playfield_scroll(true);
    fixture.set_scroll(37, 20);
    // Name table 1: the picture starts 256 pixels across the 512-pixel plane.
    fixture.set_control(0x01);
    fixture.set_mask(0x0A);
    fixture.next_batch();
    fixture.core.run_frame(0);
    assert_eq!(Some((256 + 37, 20)), fixture.core.playfield_origin());
}

#[test]
fn no_origin_while_the_lines_scroll_is_not_kept() {
    let mut fixture = TileFixture::new();
    fixture.set_scroll(37, 20);
    fixture.set_mask(0x0A);
    fixture.next_batch();
    assert_eq!(None, fixture.core.playfield_origin());
}
