//! The "3dh" profile format of 3dSen 2.1 (AMD-062): a profile that carries a sixteen-byte
//! content identifier for each tile instead of the tile's two bit planes, and whose explicit
//! variant textures are stored exclusive-ored with the pattern's default texture.
//!
//! The tiles are found again in the user's ROM at load time (REQ-PROF-078) or, for the games that
//! compress their tiles into CHR-RAM, while playing (REQ-PROF-080). Everything else about the
//! profile — patterns, variants, scripts, settings — is the schema of SPEC-03.
//!
//! A profile is **never** converted from one format to the other (owner's decision, AMD-062 §2):
//! this module reads and rewrites a 3dh as a 3dh, next to the 3dn reader and writer of
//! [`crate::wire`].

pub mod default_texture;
pub mod envelope;
pub mod profile;
pub mod raw_tile;
pub mod tile_id;
