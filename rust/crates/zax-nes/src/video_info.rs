/// Dimensions of the tile grid the 3D engine allocates (REQ-CORE-031, SPEC-02 §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VideoInfo {
    /// Background tile columns: 33 (32 plus one, because of fine X scroll).
    pub width: i32,
    /// Background tile rows: 31 (30 plus one).
    pub height: i32,
    /// Sprite record slots: 128 (64 sprites, two records each in 8x16 mode).
    pub sprite_count: i32,
}

impl VideoInfo {
    /// The one and only value the core reports: (33, 31, 128).
    pub const STANDARD: Self = Self {
        width: 33,
        height: 31,
        sprite_count: 128,
    };

    /// Maximum background records in a frame batch: 33 x 31 (REQ-CORE-032).
    pub const MAX_BACKGROUND_RECORDS: usize = 1023;

    /// Maximum sprite records in a frame batch (REQ-CORE-032).
    pub const MAX_SPRITE_RECORDS: usize = 128;

    /// Maximum records of any kind in a frame batch (REQ-CORE-032).
    pub const MAX_RECORDS: usize = Self::MAX_BACKGROUND_RECORDS + Self::MAX_SPRITE_RECORDS;
}
