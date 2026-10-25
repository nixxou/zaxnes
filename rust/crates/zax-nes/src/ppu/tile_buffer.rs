use crate::state::{InvalidState, StateReader, StateWriter};
use crate::TileRecord;

/// The tile records captured during one frame, with the capacity limits of REQ-CORE-032.
///
/// At most 1 023 background records (33 x 31) and 128 sprite records, 1 151 in all. Records
/// that would exceed either limit are dropped without an error and the emulation carries on
/// (REQ-CORE-032); the reference stops emulating instead, which no embedded profile ever
/// triggers. The storage is reserved once, so a frame never allocates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileBuffer {
    records: Vec<TileRecord>,
    /// Background records the current frame may hold: [`TileBuffer::MAX_BACKGROUND`], or more
    /// when the widescreen extension captures extra columns.
    background_limit: usize,
    background_count: usize,
    sprite_count: usize,
    dropped_background: usize,
    dropped_sprites: usize,
}

impl Default for TileBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl TileBuffer {
    /// Maximum number of background records in one frame (33 x 31, REQ-CORE-032).
    pub const MAX_BACKGROUND: usize = 1023;

    /// Maximum number of sprite records in one frame (REQ-CORE-032).
    pub const MAX_SPRITES: usize = 256;

    /// Maximum number of background records in one widescreen frame: 31 rows of the 33
    /// columns plus the extra columns of the widest margin on both sides.
    #[allow(clippy::cast_sign_loss)] // A positive constant.
    pub const MAX_WIDE_BACKGROUND: usize = (31 + 2 * crate::wide::MAX_EXTRA_ROWS as usize)
        * (33 + 2 * crate::wide::MAX_EXTRA_COLUMNS as usize);

    /// Maximum number of records in one frame, widescreen included.
    pub const CAPACITY: usize = Self::MAX_WIDE_BACKGROUND + Self::MAX_SPRITES;

    /// Creates an empty buffer with its full capacity reserved.
    #[must_use]
    pub fn new() -> Self {
        Self {
            records: Vec::with_capacity(Self::CAPACITY),
            background_limit: Self::MAX_BACKGROUND,
            background_count: 0,
            sprite_count: 0,
            dropped_background: 0,
            dropped_sprites: 0,
        }
    }

    /// Records in capture order (REQ-CORE-044).
    #[must_use]
    pub fn records(&self) -> &[TileRecord] {
        &self.records
    }

    /// Number of background records dropped by the capacity limit, for the tests.
    #[must_use]
    pub const fn dropped_background(&self) -> usize {
        self.dropped_background
    }

    /// Number of sprite records dropped by the capacity limit, for the tests.
    #[must_use]
    pub const fn dropped_sprites(&self) -> usize {
        self.dropped_sprites
    }

    /// Sets how many extra background columns each side of the frame being captured has, which
    /// raises the background limit accordingly (widescreen extension).
    pub fn set_extra_columns(&mut self, extra: i32) {
        self.set_extra_area(extra, 0);
    }

    /// Room for `extra` more columns on each side and `rows` more rows above and below.
    pub fn set_extra_area(&mut self, extra: i32, rows: i32) {
        let extra = usize::try_from(extra.clamp(0, crate::wide::MAX_EXTRA_COLUMNS)).unwrap_or(0);
        let rows = usize::try_from(rows.clamp(0, crate::wide::MAX_EXTRA_ROWS)).unwrap_or(0);
        self.background_limit = (31 + (2 * rows)) * (33 + (2 * extra));
    }

    /// Empties the buffer at the start of a rendered frame.
    /// Keeps the background records `keep` accepts, and every sprite record.
    pub fn retain_background(&mut self, mut keep: impl FnMut(&TileRecord) -> bool) {
        self.records
            .retain(|record| !record.background || keep(record));
        self.background_count = self
            .records
            .iter()
            .filter(|record| record.background)
            .count();
    }

    pub fn clear(&mut self) {
        self.records.clear();
        self.background_count = 0;
        self.sprite_count = 0;
        self.dropped_background = 0;
        self.dropped_sprites = 0;
    }

    /// Appends `record`, dropping it when its kind is full.
    pub fn add(&mut self, record: TileRecord) {
        if record.background {
            if self.background_count >= self.background_limit {
                self.dropped_background += 1;
                return;
            }

            self.background_count += 1;
        } else {
            if self.sprite_count >= Self::MAX_SPRITES {
                self.dropped_sprites += 1;
                return;
            }

            self.sprite_count += 1;
        }

        self.records.push(record);
    }
}

impl TileBuffer {
    /// Appends the buffer to `writer` (REQ-CORE-090).
    ///
    /// The records are part of the state because the Zapper's light detection reads them
    /// back during the vblank that follows the frame (REQ-CORE-062), so a restore has to
    /// reproduce them exactly (REQ-CORE-092).
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_usize(self.records.len());
        writer.write_usize(self.background_count);
        writer.write_usize(self.sprite_count);

        for record in &self.records {
            writer.write_u64(record.plane0);
            writer.write_u64(record.plane1);
            writer.write_i16(record.x);
            writer.write_i16(record.y);
            writer.write_i32(record.index);
            writer.write_u8(record.palette);
            writer.write_u8(
                u8::from(record.mirror_h)
                    | (u8::from(record.mirror_v) << 1)
                    | (u8::from(record.hidden) << 2)
                    | (u8::from(record.background) << 3),
            );
        }
    }

    /// Restores the buffer from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or the record count is impossible.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        let count = reader.read_usize()?;
        if count > Self::CAPACITY {
            return Err(InvalidState::corrupt(
                "Corrupt savestate: impossible tile record count.",
            ));
        }

        self.background_count = reader.read_usize()?;
        self.sprite_count = reader.read_usize()?;
        self.dropped_background = 0;
        self.dropped_sprites = 0;
        self.records.clear();

        for _ in 0..count {
            let plane0 = reader.read_u64()?;
            let plane1 = reader.read_u64()?;
            let x = reader.read_i16()?;
            let y = reader.read_i16()?;
            let index = reader.read_i32()?;
            let palette = reader.read_u8()?;
            let flags = reader.read_u8()?;

            self.records.push(TileRecord::new(
                plane0,
                plane1,
                x,
                y,
                index,
                flags & 1 != 0,
                flags & 2 != 0,
                flags & 4 != 0,
                palette,
                flags & 8 != 0,
            ));
        }

        Ok(())
    }
}
