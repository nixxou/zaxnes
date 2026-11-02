//! The spikes geometry (Rust-only, EDITOR-1): each run of opaque pixels of a row is a square
//! section in depth, repeated every `period` slices (the widest run).

use zax_engine3d::geometry::tile_voxels::AXIS_Z;
use zax_engine3d::settings::{Geo3DType, RenderAlgorithm};

use super::support::geo::{self, object_with, Object};

#[test]
fn drawn_spikes_become_rows_of_pyramids() {
    // A spike two pixels wide at the top, four at its base, over a full-width plate.
    let pattern = geo::one_tile(geo::tile(&["........", "...11...", "..1111..", "11111111"]));
    let instance = object_with(
        8,
        Object {
            geometry: Geo3DType::Spikes,
            ..Object::default()
        },
    );
    let (fields, depth) = geo::fields(&pattern, &instance, RenderAlgorithm::Greedy);
    assert_eq!(8, depth);
    let field = &fields[0];
    // Rows are stored bottom up: the drawn row y is j = 7 - y.
    let slices = |x: i32, y: i32| -> Vec<i32> {
        (0..8)
            .filter(|&k| field.occupied(x, 7 - y, k, AXIS_Z))
            .collect()
    };
    // The plate is 8 wide, the period: it fills the depth.
    assert_eq!(slices(0, 3), (0..8).collect::<Vec<_>>());
    // The base of the spike, 4 wide: the middle 4 slices of each period of 8.
    assert_eq!(slices(3, 2), [2, 3, 4, 5]);
    // Its tip, 2 wide: the middle 2.
    assert_eq!(slices(3, 1), [3, 4]);
    assert!(slices(1, 1).is_empty(), "outside the spike");
}
