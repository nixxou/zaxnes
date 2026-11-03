//! AMD-055 REQ-SCN-082 needs one thing of the geometry: the depth `D` the voxel field of a tile
//! was built with, because that is what centres the mesh's local frame on `z` (REQ-E3D-504,
//! `(i − 4, j − 4, k − D/2)`).
//!
//! The pixel shadows snap a point to the centre of its voxel, so on `z` they have to know where
//! the voxel edges fall, and the **extent** of the mesh will not do: a `sizeZ` = 1 plate does not
//! span the box of a deeper build, and a sparse tile does not reach its own edges. So the mesher
//! carries `D` on the mesh it returns, and the renderer reads it per draw run.

use zax_engine3d::geometry::tile_mesh::TileMesh;
use zax_engine3d::image::TileImageKey;
use zax_engine3d::settings::RenderAlgorithm;

use super::support::geo::{self, object_3d};

/// A solid 8 × 8 tile.
fn solid() -> TileImageKey {
    geo::tile(&["11111111"; 8])
}

/// REQ-E3D-504: a `GREEDY` mesh reports the depth its field was built with, which is what its
/// local `z` is centred on — and the bounds confirm it, the box running from `−D/2` to `+D/2`.
#[test]
fn req_e3d_504_a_greedy_mesh_carries_the_depth_of_its_field() {
    for depth in [1, 2, 3, 8, 16] {
        let mesh = geo::mesh_of(solid(), &object_3d(depth), RenderAlgorithm::Greedy);
        assert_eq!(depth, mesh.build_depth(), "depth {depth}");

        #[allow(clippy::cast_precision_loss)] // Depths run to 96.
        let half = depth as f32 / 2.0;
        assert_eq!(-half, mesh.bounds_min().z, "depth {depth}");
        assert_eq!(half, mesh.bounds_max().z, "depth {depth}");
    }
}

/// The same for `GREEDY_MULTICOLOR` and for `MARCHING`, whose corner grid is offset by half a
/// voxel (REQ-E3D-522) but whose **field** is the one of REQ-E3D-504: the depth carried is the
/// field's, so all three meshers snap on the same voxel grid.
#[test]
fn req_e3d_522_every_mesher_carries_the_depth_of_its_field() {
    for algorithm in [RenderAlgorithm::GreedyMulticolor, RenderAlgorithm::Marching] {
        for depth in [1, 3, 8] {
            let mesh = geo::mesh_of(solid(), &object_3d(depth), algorithm);
            assert_eq!(depth, mesh.build_depth(), "{algorithm:?} at depth {depth}");
        }
    }
}

/// A mesh that is not a tile's — the backdrop box, the rectangle of the 2D mode, an OBJ model —
/// reports a depth of 0, the value whose voxel edges fall on the whole coordinates. Nothing reads
/// it there: REQ-SCN-082 is about tile meshes.
#[test]
fn req_scn_082_a_mesh_built_by_hand_reports_no_depth() {
    assert_eq!(0, TileMesh::empty().build_depth());
    let mesh = TileMesh::new(Vec::new(), Vec::new()).unwrap();
    assert_eq!(0, mesh.build_depth());
}

/// An empty tile (`sizeZ` = 0, REQ-E3D-420) has no geometry at all, so it has no frame to centre
/// and reports 0 — and it is never drawn.
#[test]
fn req_e3d_420_an_empty_tile_reports_no_depth() {
    let mesh = geo::mesh_of(solid(), &object_3d(0), RenderAlgorithm::Greedy);
    assert!(mesh.is_empty());
    assert_eq!(0, mesh.build_depth());
}
