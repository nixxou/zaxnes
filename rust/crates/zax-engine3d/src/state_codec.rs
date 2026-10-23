//! Reads and writes the *learned state* of the conversion: the images met while playing, the
//! learned patterns, the temporary patterns and the tracker. A savestate carries it, which is
//! what makes a reload replay identically (DEC-R2-T5, REQ-E3D-008, SPEC-07 REQ-LOOP-040).
//!
//! The profile's own patterns and images are *not* written: they come back from the profile,
//! with the same identities, because loading a profile is deterministic. A state is therefore
//! only valid for the profile that was loaded when it was written, which [`write`] records
//! and [`read`] checks.
//!
//! The format is little-endian and fully determined by the state, so writing the same state
//! twice gives the same bytes (REQ-ARCH-012).

use crate::engine::ConversionEngine;
use crate::error::EngineError;
use crate::image::TileImageKey;
use crate::instance::{PatternInstance, RenderStyle};
use crate::pattern::EnginePattern;
use crate::registry::PatternId;
use crate::settings::{Geo3DType, ZLayer};
use crate::vectors::{Cell, Mat4, Vec2, Vec3};

/// Magic of the block: `SE3D`.
pub const MAGIC: u32 = 0x4433_4553;

/// Version of the format.
pub const VERSION: u16 = 1;

/// The capacity a host may reserve for this block so that it can announce an upper bound on
/// the savestate size before serialising (REQ-ARCH-013).
pub const RESERVED_CAPACITY: usize = 8 * 1024 * 1024;

/// Whether a written block fits the reserved capacity (REQ-ARCH-013).
#[must_use]
pub const fn fits_reserved_capacity(length: usize) -> bool {
    length <= RESERVED_CAPACITY
}

/// Writes the learned state of `engine`.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // C# int fields.
pub fn write(engine: &ConversionEngine) -> Vec<u8> {
    let mut out = Writer::default();
    let patterns = engine.patterns();
    let profile_image_count = engine.profile_image_count();

    out.u32(MAGIC);
    out.u16(VERSION);
    out.i32(profile_image_count as i32);
    out.i32(patterns.profile_pattern_count());

    // Images met while playing, in registry order, so that reinterning them in that order
    // gives back the same identities (REQ-E3D-007).
    let images = engine.images().images();
    let learned_images = images.iter().skip(profile_image_count);
    out.i32(learned_images.len() as i32);
    for image in learned_images {
        let key = image.key();
        out.u64(key.low);
        out.u64(key.high);
        out.u8(key.flip());
    }

    // Learned permanent patterns, then every temporary pattern. Their order in the block is
    // their registration order, which the read puts back.
    let learned: Vec<PatternId> = patterns
        .permanent()
        .iter()
        .copied()
        .filter(|&id| {
            patterns.get(id).is_some_and(|pattern| {
                pattern.registration_index() >= patterns.profile_pattern_count()
            })
        })
        .collect();
    let temporary = patterns.temporary();

    for list in [learned.as_slice(), temporary] {
        out.i32(list.len() as i32);
        for pattern in list.iter().filter_map(|&id| patterns.get(id)) {
            write_pattern(&mut out, pattern);
        }
    }

    let tracker = engine.tracker();
    out.bool(tracker.running());
    out.bool(tracker.sprites());
    out.vec3(tracker.position());
    for list in [tracker.included(), tracker.excluded()] {
        out.i32(list.len() as i32);
        for &pattern in list {
            out.i32(reference(engine, pattern, &learned, temporary));
        }
    }

    out.bytes
}

/// Puts a learned state back: drops what was learned since the profile loaded, restores the
/// images, the patterns and the tracker, and rebuilds the geometry of the restored patterns
/// (REQ-E3D-008, REQ-E3D-322).
///
/// # Errors
///
/// [`EngineError::InvalidState`] when the block is not a learned state, its version is
/// unknown, it was written with another profile loaded, or it is truncated.
pub fn read(engine: &mut ConversionEngine, block: &[u8]) -> Result<(), EngineError> {
    let mut reader = Reader {
        bytes: block,
        at: 0,
    };
    if reader.u32()? != MAGIC {
        return Err(invalid("Not a conversion learned-state block."));
    }

    let version = reader.u16()?;
    if version != VERSION {
        return Err(invalid(&format!(
            "Unsupported learned-state version {version}."
        )));
    }

    let profile_image_count = reader.i32()?;
    let profile_pattern_count = reader.i32()?;
    if usize::try_from(profile_image_count).ok() != Some(engine.profile_image_count())
        || profile_pattern_count != engine.patterns().profile_pattern_count()
    {
        return Err(invalid(
            "The learned state was written with another profile loaded.",
        ));
    }

    let profile_images = engine.profile_image_count();
    let (patterns, images) = engine.registries_mut();
    patterns.clear_learned(images);
    images.truncate_to(profile_images);

    let learned_image_count = reader.count()?;
    for _ in 0..learned_image_count {
        let low = reader.u64()?;
        let high = reader.u64()?;
        let flip = reader.u8()?;
        let (_, images) = engine.registries_mut();
        images.intern(TileImageKey::new(low, high, flip & 1 != 0, flip & 2 != 0));
    }

    let mut restored = Vec::new();
    for permanent in [true, false] {
        let count = reader.count()?;
        for _ in 0..count {
            restored.push(read_pattern(&mut reader, engine, permanent)?);
        }
    }

    let running = reader.bool()?;
    let sprites = reader.bool()?;
    let position = reader.vec3()?;
    let included = read_references(&mut reader, engine, &restored)?;
    let excluded = read_references(&mut reader, engine, &restored)?;
    engine
        .tracker_mut()
        .restore(running, sprites, position, included, excluded);

    for id in restored {
        engine.rebuild_geometry_of(id);
    }

    Ok(())
}

fn invalid(message: &str) -> EngineError {
    EngineError::InvalidState(message.to_owned())
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)] // C# (ushort), int ids.
fn write_pattern(out: &mut Writer, pattern: &EnginePattern) {
    out.u16(pattern.width() as u16);
    out.u16(pattern.height() as u16);
    out.bool(pattern.self_connected());
    out.bool(pattern.skip_geometry);
    out.i32(pattern.constraint_size.x);
    out.i32(pattern.constraint_size.y);
    out.i32(pattern.stored_pixel_count);
    out.i32(pattern.stored_color_flags);
    out.i32(pattern.stored_tile_density);
    out.i32(pattern.stored_pixel_density);

    for j in 0..pattern.height() {
        for i in 0..pattern.width() {
            out.i32(pattern.image_at(i, j).map_or(-1, |image| image.id() as i32));
        }
    }

    out.u16(pattern.instances().len() as u16);
    for instance in pattern.instances() {
        write_instance(out, instance);
    }
}

fn read_pattern(
    reader: &mut Reader<'_>,
    engine: &mut ConversionEngine,
    permanent: bool,
) -> Result<PatternId, EngineError> {
    let width = reader.u16()?;
    let height = reader.u16()?;
    let self_connected = reader.bool()?;
    let skip_geometry = reader.bool()?;
    let constraint = Cell::new(reader.i32()?, reader.i32()?);
    let stored = [reader.i32()?, reader.i32()?, reader.i32()?, reader.i32()?];

    let mut cells = Vec::with_capacity(usize::from(width) * usize::from(height));
    for _ in 0..usize::from(width) * usize::from(height) {
        let id = reader.i32()?;
        cells.push(match u32::try_from(id) {
            Ok(id) => Some(
                engine
                    .images()
                    .by_id(id)
                    .ok_or_else(|| invalid("A pattern cell names an unknown image."))?,
            ),
            Err(_) => None,
        });
    }

    let mut pattern = EnginePattern::new(width, height, cells, permanent);
    pattern.skip_geometry = skip_geometry;
    pattern.constraint_size = constraint;
    pattern.stored_pixel_count = stored[0];
    pattern.stored_color_flags = stored[1];
    pattern.stored_tile_density = stored[2];
    pattern.stored_pixel_density = stored[3];
    pattern.set_self_connected(self_connected);

    let instance_count = reader.u16()?;
    for _ in 0..instance_count {
        pattern.add_instance(read_instance(reader)?);
    }

    // Registered as recorded: the replacement rules of REQ-E3D-065 already ran when these
    // patterns were first added, and a restore must give back exactly what was saved.
    let (patterns, images) = engine.registries_mut();
    Ok(patterns.register(pattern, images))
}

fn write_instance(out: &mut Writer, instance: &PatternInstance) {
    out.i32(instance.depth);
    out.u8(instance.geometry as u8);
    out.u8(instance.layer as u8);
    out.u8(instance.style as u8);
    out.bool(instance.anti_shadow);
    out.bool(instance.flip);
    out.bool(instance.solid);
    out.bool(instance.full_warp);
    out.bool(instance.cast_shadow);
    out.bool(instance.receive_shadow);
    out.vec3(instance.scale);
    out.vec3(instance.rotation);
    out.vec3(instance.offset);
    out.vec3(instance.pivot);
    out.f32(instance.alpha);
    out.i32(instance.mesh_algorithm);
    out.vec3(instance.deform_amplitude);
    out.vec3(instance.deform_pivot);
    out.mat4(&instance.deform_speed);
    out.mat4(&instance.deform_offset);
    out.i32(instance.empty_color);
    out.i32(instance.custom_palette);
    out.f32(instance.tex_offset.x);
    out.f32(instance.tex_offset.y);
    out.f32(instance.tex_tilting.x);
    out.f32(instance.tex_tilting.y);
    out.f32(instance.fill_rate);
    out.string(instance.tag_string());
}

fn read_instance(reader: &mut Reader<'_>) -> Result<PatternInstance, EngineError> {
    let mut instance = PatternInstance::new();
    instance.depth = reader.i32()?;
    instance.geometry = Geo3DType::from_raw(i32::from(reader.u8()?));
    instance.layer = ZLayer::from_raw(i32::from(reader.u8()?));
    instance.style = RenderStyle::from_raw(i32::from(reader.u8()?));
    instance.anti_shadow = reader.bool()?;
    instance.flip = reader.bool()?;
    instance.solid = reader.bool()?;
    instance.full_warp = reader.bool()?;
    let cast_shadow = reader.bool()?;
    let receive_shadow = reader.bool()?;
    instance.scale = reader.vec3()?;
    instance.rotation = reader.vec3()?;
    instance.offset = reader.vec3()?;
    instance.pivot = reader.vec3()?;
    instance.alpha = reader.f32()?;
    instance.mesh_algorithm = reader.i32()?;
    instance.deform_amplitude = reader.vec3()?;
    instance.deform_pivot = reader.vec3()?;
    instance.deform_speed = reader.mat4()?;
    instance.deform_offset = reader.mat4()?;
    instance.empty_color = reader.i32()?;
    instance.custom_palette = reader.i32()?;
    instance.tex_offset = Vec2::new(reader.f32()?, reader.f32()?);
    instance.tex_tilting = Vec2::new(reader.f32()?, reader.f32()?);
    instance.fill_rate = reader.f32()?;

    // Writing the tag string recomputes the shadow flags, so they are restored after it.
    instance.set_tag_string(&reader.string()?);
    instance.cast_shadow = cast_shadow;
    instance.receive_shadow = receive_shadow;
    Ok(instance)
}

fn read_references(
    reader: &mut Reader<'_>,
    engine: &ConversionEngine,
    restored: &[PatternId],
) -> Result<Vec<PatternId>, EngineError> {
    let count = reader.count()?;
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        let reference = reader.i32()?;
        if let Some(pattern) = resolve(engine, reference, restored) {
            result.push(pattern);
        }
    }

    Ok(result)
}

/// A stable reference to a registry pattern: its registration index when the profile
/// supplied it, and `−1 − k` for the `k`-th pattern of this block otherwise.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // C# int indices.
fn reference(
    engine: &ConversionEngine,
    pattern: PatternId,
    learned: &[PatternId],
    temporary: &[PatternId],
) -> i32 {
    let patterns = engine.patterns();
    if let Some(found) = patterns.get(pattern) {
        if found.registration_index() < patterns.profile_pattern_count() && found.permanent() {
            return found.registration_index();
        }
    }

    if let Some(index) = learned.iter().position(|&p| p == pattern) {
        return -1 - index as i32;
    }

    temporary
        .iter()
        .position(|&p| p == pattern)
        .map_or(i32::MIN, |index| -1 - learned.len() as i32 - index as i32)
}

fn resolve(engine: &ConversionEngine, reference: i32, restored: &[PatternId]) -> Option<PatternId> {
    if reference == i32::MIN {
        return None;
    }

    if reference >= 0 {
        let patterns = engine.patterns();
        return patterns.permanent().iter().copied().find(|&id| {
            patterns
                .get(id)
                .is_some_and(|pattern| pattern.registration_index() == reference)
        });
    }

    let index = usize::try_from(-1 - i64::from(reference)).ok()?;
    restored.get(index).copied()
}

/// The little-endian writer of the block.
#[derive(Debug, Default)]
struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn f32(&mut self, value: f32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn vec3(&mut self, value: Vec3) {
        self.f32(value.x);
        self.f32(value.y);
        self.f32(value.z);
    }

    fn mat4(&mut self, value: &Mat4) {
        for cell in value.to_row_major() {
            self.f32(cell);
        }
    }

    /// A string as the .NET binary writer stores it: its UTF-8 length as a 7-bit encoded
    /// integer, then the bytes.
    fn string(&mut self, value: &str) {
        let mut length = value.len();
        loop {
            #[allow(clippy::cast_possible_truncation)] // Seven bits at a time.
            let low = (length & 0x7F) as u8;
            length >>= 7;
            if length == 0 {
                self.u8(low);
                break;
            }

            self.u8(low | 0x80);
        }

        self.bytes.extend_from_slice(value.as_bytes());
    }
}

/// The little-endian reader of the block; every read fails cleanly past the end.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], EngineError> {
        let slice = self
            .bytes
            .get(self.at..self.at + N)
            .ok_or_else(|| invalid("The learned-state block is truncated."))?;
        self.at += N;
        let mut out = [0; N];
        out.copy_from_slice(slice);
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, EngineError> {
        Ok(self.take::<1>()?[0])
    }

    fn bool(&mut self) -> Result<bool, EngineError> {
        Ok(self.u8()? != 0)
    }

    fn u16(&mut self) -> Result<u16, EngineError> {
        self.take().map(u16::from_le_bytes)
    }

    fn u32(&mut self) -> Result<u32, EngineError> {
        self.take().map(u32::from_le_bytes)
    }

    fn i32(&mut self) -> Result<i32, EngineError> {
        self.take().map(i32::from_le_bytes)
    }

    /// A count stored as an `i32`; a negative count reads as an empty list, as the C# loops
    /// do.
    fn count(&mut self) -> Result<usize, EngineError> {
        Ok(usize::try_from(self.i32()?).unwrap_or(0))
    }

    fn u64(&mut self) -> Result<u64, EngineError> {
        self.take().map(u64::from_le_bytes)
    }

    fn f32(&mut self) -> Result<f32, EngineError> {
        self.take().map(f32::from_le_bytes)
    }

    fn vec3(&mut self) -> Result<Vec3, EngineError> {
        Ok(Vec3::new(self.f32()?, self.f32()?, self.f32()?))
    }

    fn mat4(&mut self) -> Result<Mat4, EngineError> {
        let mut matrix = Mat4::default();
        for row in 0..4 {
            for column in 0..4 {
                matrix[(row, column)] = self.f32()?;
            }
        }

        Ok(matrix)
    }

    fn string(&mut self) -> Result<String, EngineError> {
        let mut length = 0usize;
        let mut shift = 0;
        loop {
            let byte = self.u8()?;
            length |= usize::from(byte & 0x7F) << shift;
            if byte & 0x80 == 0 {
                break;
            }

            shift += 7;
            if shift > 28 {
                return Err(invalid(
                    "A string length of the learned-state block is malformed.",
                ));
            }
        }

        let bytes = self
            .bytes
            .get(self.at..self.at + length)
            .ok_or_else(|| invalid("The learned-state block is truncated."))?;
        self.at += length;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| invalid("A string of the learned-state block is not UTF-8."))
    }
}
