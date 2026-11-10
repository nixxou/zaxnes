//! Resolving the tiles of a "3dh" profile where the engine and the core can be seen together
//! (AMD-062 REQ-PROF-078 to REQ-PROF-081).

use std::sync::Arc;

use zax_engine3d::engine::ConversionEngine;
use zax_engine3d::geometry::builder::GeometryBuilder;
use zax_engine3d::geometry::object_geometry::ObjectGeometry;
use zax_engine3d::image::TileImageKey;
use zax_engine3d::instance::InstanceRef;
use zax_engine3d::settings::Geo3DType;
use zax_nes::rom::InesHeader;
use zax_profiles::enums::{TextureFormat, TileFlip, VariantGeometry};
use zax_profiles::hash::default_texture::{self, DefaultTexture};
use zax_profiles::hash::envelope::HashProfileManifest;
use zax_profiles::hash::raw_tile::{self, RawTile};
use zax_profiles::hash::rom_zones::RomZones;
use zax_profiles::hash::tile_id::TileId;
use zax_profiles::hash::tiles::TileResolver;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::tile::{HashTileField, PatternGrid, ProfileTile};
use zax_profiles::values::Vec2I;
use zax_profiles::variant::{HashTextureKind, PatternVariant, TexturePixels, VariantTexture};

use super::support::{cells, img, permanent, Batch, ProfileBuilder};

/// An iNES file of `prg_units` PRG banks and `chr_units` CHR banks, `byte6` and `byte7` as given.
fn ines(prg_units: u8, chr_units: u8, byte6: u8, byte7: u8, byte9: u8) -> Vec<u8> {
    let mut file = vec![0_u8; 16];
    file[..4].copy_from_slice(&[b'N', b'E', b'S', 0x1A]);
    file[4] = prg_units;
    file[5] = chr_units;
    file[6] = byte6;
    file[7] = byte7;
    file[9] = byte9;

    let header = InesHeader::try_parse(&file).unwrap();
    file.resize(header.announced_size(), 0);
    file
}

// REQ-PROF-078 point 1 reads the iNES header for itself, the profile module depending on no
// emulation core. What it reads has to be what the core reads: same offsets, same sizes, NES 2.0
// and the trainer included.
#[test]
fn the_zones_of_req_prof_078_are_the_cores_own_offsets() {
    let cases = [
        // (prg units, chr units, byte 6, byte 7, byte 9)
        (1, 1, 0x00, 0x00, 0x00),
        (2, 0, 0x01, 0x00, 0x00),
        (8, 16, 0x00, 0x00, 0x00),
        // Trainer.
        (1, 1, 0x04, 0x00, 0x00),
        (4, 2, 0x06, 0x00, 0x00),
        // NES 2.0, the high nibbles of byte 9 adding to both sizes.
        (1, 1, 0x00, 0x08, 0x00),
        (1, 1, 0x00, 0x08, 0x11),
        (0x20, 0x10, 0x04, 0x08, 0x01),
        // NES 2.0 exponent-multiplier notation, for the PRG then for the CHR.
        (0x12, 1, 0x00, 0x08, 0x0F),
        (1, 0x12, 0x00, 0x08, 0xF0),
    ];

    for (prg_units, chr_units, byte6, byte7, byte9) in cases {
        let file = ines(prg_units, chr_units, byte6, byte7, byte9);
        let header = InesHeader::try_parse(&file).unwrap();
        let zones = RomZones::of(&file).unwrap();
        let label = format!(
            "prg={prg_units} chr={chr_units} b6={byte6:#04x} b7={byte7:#04x} b9={byte9:#04x}"
        );

        assert_eq!(header.prg_rom_offset(), zones.prg.start, "{label}");
        assert_eq!(header.prg_rom_size, zones.prg.len(), "{label}");
        assert_eq!(header.chr_rom_offset(), zones.chr.start, "{label}");
        assert_eq!(header.chr_rom_size, zones.chr.len(), "{label}");
    }
}

// A "3dh" profile the engine can load: patterns whose cells either carry their sixteen bytes or
// name a manifest string and wait for it (AMD-062 REQ-PROF-074, REQ-PROF-081).

/// One cell of a "3dh" pattern.
#[derive(Debug, Clone, Copy)]
enum Cell {
    /// A tile whose planes the profile carries.
    Known(TileImageKey),
    /// A tile that names the identifier of this image and waits for its sixteen bytes.
    Waiting(TileImageKey),
    /// An empty cell.
    Empty,
}

/// The sixteen raw bytes of a tile image, as a ROM or the CHR-RAM holds them.
fn raw_of(image: TileImageKey) -> RawTile {
    raw_tile::from_planes(image.low, image.high)
}

/// The manifest string a waiting cell names: the identifier of REQ-PROF-075.
fn key_of(image: TileImageKey) -> String {
    TileId::compute(&raw_of(image)).to_text()
}

/// Builds a "3dh" profile and the resolver of its manifest, every named tile waiting.
#[derive(Debug)]
struct HashProfileBuilder {
    profile: GameProfile,
    manifest: HashProfileManifest,
}

impl HashProfileBuilder {
    fn new() -> Self {
        Self {
            profile: GameProfile::create_empty(),
            manifest: HashProfileManifest::default(),
        }
    }

    fn pattern(&mut self, width: i32, height: i32, grid: &[Cell]) -> &mut Self {
        self.pattern_with(
            width,
            height,
            grid,
            variant(VariantGeometry::Default, false),
        )
    }

    fn pattern_with(
        &mut self,
        width: i32,
        height: i32,
        grid: &[Cell],
        variant: PatternVariant,
    ) -> &mut Self {
        let mut tiles = Vec::new();
        for cell in grid {
            tiles.push(match *cell {
                Cell::Empty => None,
                Cell::Known(image) => Some(Arc::new(ProfileTile::new(
                    image.low,
                    image.high,
                    TileFlip::NONE,
                ))),
                Cell::Waiting(image) => {
                    self.manifest
                        .add_identifier(TileId::compute(&raw_of(image)));
                    Some(Arc::new(ProfileTile::waiting(
                        Arc::from(key_of(image).as_str()),
                        HashTileField::Identifier,
                        TileFlip::NONE,
                    )))
                }
            });
        }

        self.profile
            .permanent_patterns
            .push(Arc::new(ProfilePattern {
                grid: Some(PatternGrid::new(width, height, tiles)),
                tile_size: Some(Vec2I::new(width, height)),
                permanent: true,
                variants: vec![Arc::new(variant)],
                ..ProfilePattern::default()
            }));
        self
    }

    /// The profile and a resolver over its manifest, nothing resolved yet.
    fn build(&self) -> (GameProfile, TileResolver) {
        (self.profile.clone(), TileResolver::of(&self.manifest))
    }

    /// An engine with the profile and its waiting tiles loaded, geometry on.
    fn load(&self) -> ConversionEngine {
        let (profile, resolver) = self.build();
        let mut engine = ConversionEngine::new();
        engine.use_geometry();
        engine
            .load_profile_with_tiles(&profile, true, Some(resolver))
            .unwrap();
        engine
    }
}

fn variant(geometry: VariantGeometry, full_warp: bool) -> PatternVariant {
    PatternVariant {
        depth: 3,
        geometry,
        full_warp,
        alpha: 1.0,
        ..PatternVariant::default()
    }
}

fn builder(engine: &ConversionEngine) -> &GeometryBuilder {
    engine.geometry_builder().unwrap()
}

/// The 3D object of the first instance of permanent pattern `index`.
fn object(engine: &ConversionEngine, index: usize) -> &ObjectGeometry {
    builder(engine)
        .of(InstanceRef {
            pattern: permanent(engine, index),
            index: 0,
        })
        .unwrap()
}

/// A CHR-RAM of `blocks` sixteen-byte blocks, holding each tile at the given block position.
fn chr_ram(blocks: usize, tiles: &[(usize, RawTile)]) -> Vec<u8> {
    let mut memory = vec![0_u8; blocks * raw_tile::BYTE_COUNT];
    for (block, raw) in tiles {
        let start = block * raw_tile::BYTE_COUNT;
        memory[start..start + raw_tile::BYTE_COUNT].copy_from_slice(raw);
    }
    memory
}

// REQ-PROF-080, the "screen only" path of Q-178 §1: no CHR-RAM is offered to the engine, so the
// tile can only come from the records of the frame. It is resolved before the recognition of
// that same frame, and the pattern is recognised whole in it.
#[test]
fn req_prof_080_a_tile_of_the_frame_is_resolved_before_that_frames_recognition() {
    let mut engine = HashProfileBuilder::new()
        .pattern(2, 1, &[Cell::Known(img::a()), Cell::Waiting(img::b())])
        .load();
    let id = permanent(&engine, 0);

    // While B waits, its cell has no image: the pattern knows A alone (REQ-PROF-081).
    let pattern = engine.patterns().get(id).unwrap();
    assert!(pattern.has_placeholders());
    assert_eq!(1, pattern.images().len());
    assert_eq!(1, engine.hash_tiles().unwrap().waiting_count());

    let mut batch = Batch::new();
    batch.bg_row(5, 10, &[img::a(), img::b()]);
    let frame = batch.convert(&mut engine, 0);

    assert_eq!(1, frame.shapes().len());
    let shape = frame.shape(frame.shapes()[0]);
    assert_eq!(2, shape.tile_count());
    assert_eq!(Some(id), shape.reference_pattern());

    let pattern = engine.patterns().get(id).unwrap();
    assert!(!pattern.has_placeholders());
    assert_eq!(2, pattern.images().len());
    let tiles = engine.hash_tiles().unwrap();
    assert_eq!(0, tiles.waiting_count());
    assert!(!tiles.is_waiting());
    assert_eq!(1, tiles.resolved_while_playing());
}

// REQ-PROF-081: a waiting tile is equal to no tile of the screen, so it can never be used to
// recognise a pattern. The pattern draws with its only resolved cell, and once the tile is
// resolved it draws like the same pattern read from a 3dn profile.
#[test]
fn req_prof_081_a_waiting_cell_draws_nothing_and_recognises_nothing() {
    let mut engine = HashProfileBuilder::new()
        .pattern(2, 1, &[Cell::Known(img::a()), Cell::Waiting(img::b())])
        .load();
    let id = permanent(&engine, 0);

    // Another tile is on the screen beside A, and the waiting cell cannot take it: the pattern
    // covers A alone.
    let mut batch = Batch::new();
    batch.bg_row(5, 10, &[img::a(), img::c()]);
    let frame = batch.convert(&mut engine, 0);
    let shapes: Vec<_> = frame
        .shapes()
        .iter()
        .map(|&shape| frame.shape(shape))
        .filter(|shape| shape.reference_pattern() == Some(id))
        .collect();
    assert_eq!(1, shapes.len());
    assert_eq!(1, shapes[0].tile_count());

    // One mesh, for the resolved cell only; the waiting cell has none.
    assert_eq!(1, object(&engine, 0).meshes().len());
    assert!(engine.hash_tiles().unwrap().is_waiting());

    // The default texture leaves the waiting cell's 8 by 8 pixels at 0 (REQ-PROF-081).
    let texture = Arc::clone(object(&engine, 0).texture());
    assert_eq!(16, texture.width());
    assert_eq!(8, texture.height());
    for y in 0..8 {
        for x in 8..16 {
            assert_eq!(0, texture.texel(x, y), "texel ({x}, {y})");
        }
    }

    // Once B arrives, the pattern is the 3dn homologue: same key, same meshes, same texture.
    engine.offer_chr_ram(&chr_ram(4, &[(2, raw_of(img::b()))]));
    let mut homologue = ConversionEngine::new();
    homologue.use_geometry();
    homologue
        .load_profile(
            &ProfileBuilder::new()
                .pattern(2, 1, &cells(&[img::a(), img::b()]))
                .build(),
            true,
        )
        .unwrap();

    let pattern = engine.patterns().get(id).unwrap();
    assert!(!pattern.has_placeholders());
    let theirs = homologue
        .patterns()
        .get(permanent(&homologue, 0))
        .unwrap()
        .key()
        .count();
    assert_eq!(theirs, pattern.key().count());
    assert_eq!(2, object(&engine, 0).meshes().len());
    assert_eq!(
        **object(&homologue, 0).texture(),
        **object(&engine, 0).texture()
    );
    for (mine, theirs) in object(&engine, 0)
        .meshes()
        .iter()
        .zip(object(&homologue, 0).meshes())
    {
        assert_eq!(theirs.triangle_count(), mine.triangle_count());
    }
}

// REQ-PROF-081: a variant of a pattern that holds a waiting tile is built with the `Default`
// geometry and without `fullWarp`, and gets its own back when the last tile is resolved.
#[test]
fn req_prof_081_a_waiting_pattern_is_built_with_the_default_geometry() {
    let mut engine = HashProfileBuilder::new()
        .pattern_with(
            2,
            1,
            &[Cell::Known(img::a()), Cell::Waiting(img::b())],
            variant(VariantGeometry::Cube, true),
        )
        .load();
    let id = permanent(&engine, 0);

    let instance = &engine.patterns().get(id).unwrap().instances()[0];
    assert_eq!(Geo3DType::Default, instance.geometry);
    assert!(!instance.full_warp);

    engine.offer_chr_ram(&chr_ram(2, &[(1, raw_of(img::b()))]));

    let instance = &engine.patterns().get(id).unwrap().instances()[0];
    assert_eq!(Geo3DType::Cube, instance.geometry);
    assert!(instance.full_warp);
}

// REQ-PROF-083 and the V2-34 regression: the texture of a variant of `textureKind = 2` is the
// exclusive-or of field 24 with the pattern's default texture, so it is only worth the resolved
// tiles. The instance has to take the *new* texture when a resolution rebuilds its pattern, not
// only its geometry.
#[test]
fn req_prof_083_a_xor_texture_is_rebuilt_when_the_tiles_are_resolved() {
    // The texture the variant means to use: 8 by 8 indexed texels, all of value 2.
    let words = vec![i32::from_le_bytes([0xAA; 4]); 4];
    let target = VariantTexture::new(
        8,
        8,
        Some(TexturePixels::new(
            TextureFormat::Indexed,
            words,
            vec![0; 4],
        )),
    );

    // Field 24 is that texture exclusive-ored with the default texture of the *resolved* pattern.
    let resolved_pattern = ProfilePattern {
        grid: Some(PatternGrid::new(
            1,
            1,
            vec![Some(Arc::new(ProfileTile::new(
                img::a().low,
                img::a().high,
                TileFlip::NONE,
            )))],
        )),
        tile_size: Some(Vec2I::new(1, 1)),
        ..ProfilePattern::default()
    };
    let default = DefaultTexture::of(&resolved_pattern);
    let mut xor = variant(VariantGeometry::Default, false);
    xor.texture_kind = Some(HashTextureKind::Xor);
    xor.xor_texture = Some(default_texture::exclusive_or(&target, &default));

    let mut engine = HashProfileBuilder::new()
        .pattern_with(1, 1, &[Cell::Waiting(img::a())], xor)
        .load();

    // While the tile waits the default texture is all zeros, so the exclusive-or gives field 24
    // itself, which is not the texture the variant means to use.
    let waiting = Arc::clone(object(&engine, 0).texture());
    assert!((0..8).any(|x| (0..8).any(|y| waiting.texel(x, y) != 2)));

    engine.offer_chr_ram(&chr_ram(3, &[(1, raw_of(img::a()))]));

    let texture = Arc::clone(object(&engine, 0).texture());
    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(2, texture.texel(x, y), "texel ({x}, {y})");
        }
    }
}

// REQ-PROF-080: every aligned sixteen-byte block of the CHR-RAM is confronted with the manifest,
// with its four mirrorings (REQ-PROF-079); a tile that sits at an unaligned position is not a
// block of the CHR-RAM and is not resolved by that path.
#[test]
fn req_prof_080_the_chr_ram_is_read_by_aligned_blocks_with_their_four_mirrorings() {
    let mut engine = HashProfileBuilder::new()
        .pattern(1, 1, &[Cell::Waiting(img::b())])
        .load();

    // Unaligned: half a block in, half out. Nothing resolves.
    let mut memory = vec![0_u8; 4 * raw_tile::BYTE_COUNT];
    memory[8..24].copy_from_slice(&raw_of(img::b()));
    assert_eq!(0, engine.offer_chr_ram(&memory));
    assert!(engine.hash_tiles().unwrap().is_waiting());

    // Aligned and mirrored vertically: resolved, as REQ-PROF-079 asks.
    let mirrored = raw_tile::flipped(&raw_of(img::b()), TileFlip::VERTICAL);
    assert_eq!(1, engine.offer_chr_ram(&chr_ram(4, &[(2, mirrored)])));
    assert!(!engine.hash_tiles().unwrap().is_waiting());
    assert!(!engine
        .patterns()
        .get(permanent(&engine, 0))
        .unwrap()
        .has_placeholders());
}

// REQ-PROF-080 allows skipping a block whose bytes have not changed, as long as the resolved set
// of each frame is the full walk's: the waiting set only shrinks, so a block already confronted
// with the manifest resolves nothing new.
#[test]
fn req_prof_080_skipping_an_unchanged_block_resolves_what_the_full_walk_resolves() {
    let first = chr_ram(8, &[(1, raw_of(img::a()))]);
    let second = chr_ram(8, &[(1, raw_of(img::a())), (5, raw_of(img::b()))]);
    let profile = {
        let mut builder = HashProfileBuilder::new();
        builder.pattern(2, 1, &[Cell::Waiting(img::a()), Cell::Waiting(img::b())]);
        builder
    };

    // Frame after frame, one block changing between two of them.
    let mut stepwise = profile.load();
    assert_eq!(1, stepwise.offer_chr_ram(&first));
    assert_eq!(0, stepwise.offer_chr_ram(&first));
    assert_eq!(1, stepwise.offer_chr_ram(&second));
    assert_eq!(0, stepwise.offer_chr_ram(&second));

    // The full walk, once, over the final memory.
    let mut full = profile.load();
    assert_eq!(2, full.offer_chr_ram(&second));

    let mine = stepwise.hash_tiles().unwrap().resolver();
    let theirs = full.hash_tiles().unwrap().resolver();
    assert_eq!(theirs.resolved_count(), mine.resolved_count());
    assert_eq!(theirs.entries(), mine.entries());
    assert_eq!(0, mine.waiting_count());
}

// REQ-PROF-080: the resolution is monotone and costs nothing once nothing waits. A 3dn profile
// has no table at all, which is what "a 3dn profile must cost nothing more" rests on.
#[test]
fn req_prof_080_the_resolution_stops_costing_once_nothing_waits() {
    let mut engine = HashProfileBuilder::new()
        .pattern(1, 1, &[Cell::Waiting(img::a())])
        .load();

    assert_eq!(
        1,
        engine.offer_chr_ram(&chr_ram(2, &[(0, raw_of(img::a()))]))
    );
    assert!(!engine.hash_tiles().unwrap().is_waiting());

    // Nothing waits: another memory, holding a tile of no interest, costs nothing.
    assert_eq!(
        0,
        engine.offer_chr_ram(&chr_ram(2, &[(1, raw_of(img::b()))]))
    );
    let mut batch = Batch::new();
    batch.bg_row(5, 10, &[img::b(), img::c()]);
    batch.convert(&mut engine, 0);
    assert_eq!(1, engine.hash_tiles().unwrap().resolved_while_playing());

    // A 3dn profile has no waiting tile at all, and no table to walk.
    let mut plain = ProfileBuilder::new()
        .pattern(1, 1, &cells(&[img::a()]))
        .load();
    assert!(plain.hash_tiles().is_none());
    assert_eq!(
        0,
        plain.offer_chr_ram(&chr_ram(2, &[(0, raw_of(img::a()))]))
    );
    assert!(!plain
        .patterns()
        .get(permanent(&plain, 0))
        .unwrap()
        .has_placeholders());
}

// REQ-PROF-080: a resolved tile never goes back to waiting, a state load included. The image a
// resolved cell gained was interned after the profile's, so the restore drops it; the pattern
// takes it again, and goes on being recognised.
#[test]
fn req_prof_080_a_state_load_does_not_put_a_resolved_tile_back_to_waiting() {
    let mut engine = HashProfileBuilder::new()
        .pattern(2, 1, &[Cell::Known(img::a()), Cell::Waiting(img::b())])
        .load();
    let id = permanent(&engine, 0);

    // A frame first, so that the state carries images met while playing, then the resolution.
    let mut batch = Batch::new();
    batch.bg_row(5, 10, &[img::u(), img::u()]);
    batch.convert_and_publish(&mut engine, 0);
    engine.offer_chr_ram(&chr_ram(4, &[(3, raw_of(img::b()))]));
    assert!(!engine.patterns().get(id).unwrap().has_placeholders());

    let state = zax_engine3d::state_codec::write(&engine);
    zax_engine3d::state_codec::read(&mut engine, &state).unwrap();

    let pattern = engine.patterns().get(id).unwrap();
    assert!(!pattern.has_placeholders());
    assert_eq!(2, pattern.images().len());
    assert!(!engine.hash_tiles().unwrap().is_waiting());

    // The resolved cell is still a candidate on its image: the pattern is recognised whole.
    let mut batch = Batch::new();
    batch.bg_row(5, 10, &[img::a(), img::b()]);
    let frame = batch.convert(&mut engine, 1);
    let shape = frame.shape(frame.shapes()[0]);
    assert_eq!(Some(id), shape.reference_pattern());
    assert_eq!(2, shape.tile_count());
}

// REQ-PROF-080: the rebuild keeps the registry pattern itself — its slot, its registration index
// and its instances — so every handle a frame or a script holds stays valid.
#[test]
fn req_prof_080_the_rebuild_keeps_the_pattern_its_slot_and_its_instances() {
    let mut engine = HashProfileBuilder::new()
        .pattern(2, 1, &[Cell::Known(img::a()), Cell::Empty])
        .pattern(2, 1, &[Cell::Known(img::c()), Cell::Waiting(img::b())])
        .load();
    let untouched = permanent(&engine, 0);
    let id = permanent(&engine, 1);
    let before = engine.patterns().get(id).unwrap();
    let index = before.registration_index();
    let instances = before.instances().len();

    engine.offer_chr_ram(&chr_ram(2, &[(1, raw_of(img::b()))]));

    assert_eq!(vec![untouched, id], engine.patterns().permanent().to_vec());
    let after = engine.patterns().get(id).unwrap();
    assert_eq!(index, after.registration_index());
    assert_eq!(instances, after.instances().len());
    assert_eq!(2, after.images().len());

    // The pattern is a candidate on the image its cell gained (REQ-E3D-091).
    let gained = after.images()[1];
    assert!(engine.images().permanent_patterns(&gained).contains(&id));
    assert!(engine.images().is_known(&gained));
}
