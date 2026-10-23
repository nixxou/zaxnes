//! Field numbers of the profile messages (SPEC-03 §4) and the set of fields each one holds.


pub(super) mod root {
    use crate::layout::FieldMask;

    pub(crate) const TYPE: u32 = 1;
    pub(crate) const VER: u32 = 2;
    pub(crate) const SCRIPTS: u32 = 3;
    pub(crate) const PATTERNS: u32 = 4;
    pub(crate) const TEMP_PATTERNS: u32 = 5;
    pub(crate) const SETTINGS: u32 = 6;
    pub(crate) const ID: u32 = 7;
    pub(crate) const TITLE: u32 = 8;
    pub(crate) const MAKER: u32 = 9;
    pub(crate) const GAME_DEV: u32 = 10;
    pub(crate) const DES: u32 = 11;

    pub(crate) const MASK: FieldMask = FieldMask::range(1, 11);
}

/// The reference envelope that shares an object between several places (REQ-PROF-011).
pub(super) mod envelope {
    pub(crate) const EXISTING_KEY: u32 = 1;
    pub(crate) const NEW_KEY: u32 = 2;
    pub(crate) const MESSAGE: u32 = 10;
}

pub(super) mod script {
    use crate::layout::FieldMask;

    pub(crate) const NAME: u32 = 1;
    pub(crate) const ENABLE: u32 = 2;
    pub(crate) const ERROR: u32 = 3;
    pub(crate) const SOURCE: u32 = 4;
    pub(crate) const NAME_AGAIN: u32 = 5;
    pub(crate) const TAG_STR: u32 = 6;

    pub(crate) const MASK: FieldMask = FieldMask::range(1, 6);
}

pub(super) mod pattern {
    use crate::layout::FieldMask;

    pub(crate) const GRID: u32 = 1;
    pub(crate) const CONSTRAINT_SIZE: u32 = 2;
    pub(crate) const TILE_SIZE: u32 = 3;
    pub(crate) const PERMANENT: u32 = 4;
    pub(crate) const PIXEL_COUNT: u32 = 5;
    pub(crate) const COLOR_FLAGS: u32 = 6;
    pub(crate) const TILE_DENSITY: u32 = 7;
    pub(crate) const PIXEL_DENSITY: u32 = 8;
    pub(crate) const VARIANTS: u32 = 10;
    pub(crate) const SELF_CONNECTED: u32 = 11;
    pub(crate) const SKIP_GEO: u32 = 12;

    pub(crate) const MASK: FieldMask = FieldMask::of(&[1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12]);
}

pub(super) mod cell {
    pub(crate) const ENVELOPE: u32 = 1;
    pub(crate) const PLANES: u32 = 1;
}

pub(super) mod tile_planes {
    use crate::layout::FieldMask;

    pub(crate) const LOW_PLANE: u32 = 1;
    pub(crate) const HIGH_PLANE: u32 = 2;
    pub(crate) const FLIP: u32 = 3;

    pub(crate) const MASK: FieldMask = FieldMask::range(1, 3);
}

pub(super) mod variant {
    use crate::layout::FieldMask;

    pub(crate) const SIZE_Z: u32 = 1;
    pub(crate) const GEO: u32 = 2;
    pub(crate) const ANTI_SHADOW: u32 = 3;
    pub(crate) const FLIP: u32 = 4;
    pub(crate) const SOLID: u32 = 5;
    pub(crate) const FULL_WARP: u32 = 6;
    pub(crate) const LAYER: u32 = 7;
    pub(crate) const SCALE: u32 = 8;
    pub(crate) const ROT: u32 = 9;
    pub(crate) const OFFSET: u32 = 10;
    pub(crate) const PIVOT: u32 = 11;
    pub(crate) const ALPHA: u32 = 12;
    pub(crate) const MESH_ALGORITHM: u32 = 13;
    pub(crate) const DELTA_AMP: u32 = 14;
    pub(crate) const DELTA_PIVOT: u32 = 15;
    pub(crate) const DELTA_SPEED: u32 = 16;
    pub(crate) const DELTA_OFFSET: u32 = 17;
    pub(crate) const RENDER_STYLE: u32 = 18;
    pub(crate) const TEX: u32 = 19;
    pub(crate) const TAG_STR: u32 = 20;
    pub(crate) const EMPTY_COLOR: u32 = 23;

    pub(crate) const MASK: FieldMask = FieldMask::of(&[
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 23,
    ]);
}

pub(super) mod tex_rect {
    use crate::layout::FieldMask;

    pub(crate) const WIDTH: u32 = 1;
    pub(crate) const HEIGHT: u32 = 2;
    pub(crate) const PAYLOAD: u32 = 10;

    pub(crate) const MASK: FieldMask = FieldMask::of(&[1, 2, 10]);
}

pub(super) mod tex_payload {
    use crate::layout::FieldMask;

    pub(crate) const DATA: u32 = 1;
    pub(crate) const FORMAT: u32 = 2;
    pub(crate) const EXTRA: u32 = 3;

    pub(crate) const MASK: FieldMask = FieldMask::range(1, 3);
}

pub(super) mod settings {
    use crate::layout::FieldMask;

    pub(crate) const LIGHT: u32 = 1;
    pub(crate) const BG_COLOR: u32 = 2;
    pub(crate) const CUSTOM_BG_COLOR: u32 = 3;
    pub(crate) const CLIPPING: u32 = 4;
    pub(crate) const MAP_ENABLE: u32 = 5;
    pub(crate) const LAYER: u32 = 6;
    pub(crate) const GRAPHIC_QUALITY: u32 = 7;
    pub(crate) const VR_RUNTIME: u32 = 8;
    pub(crate) const VSYNC: u32 = 9;
    pub(crate) const FRAME_RATE: u32 = 10;
    pub(crate) const GAME_SPEED: u32 = 11;
    pub(crate) const VOLUME: u32 = 12;
    pub(crate) const RENDER_MODE: u32 = 13;
    pub(crate) const SAVED_PATTERN_3D: u32 = 14;
    pub(crate) const SAVE_ANIM_PATTERN_3D: u32 = 15;
    pub(crate) const CONTROL_METHOD: u32 = 16;
    pub(crate) const SHADOW: u32 = 17;
    pub(crate) const SAME_BG_PALETTE: u32 = 18;
    pub(crate) const SPRITE_TOLERANCE: u32 = 19;
    pub(crate) const SCAN_DIRECTION: u32 = 20;
    pub(crate) const SKYBOX: u32 = 21;
    pub(crate) const CORE: u32 = 22;
    pub(crate) const MIRRORING: u32 = 23;
    pub(crate) const EMPTY_TILE: u32 = 24;
    pub(crate) const LOCAL_CAM: u32 = 25;
    pub(crate) const SCENE_RATIO: u32 = 26;
    pub(crate) const EMULATED_MENU_BUTTON: u32 = 27;
    pub(crate) const RENDER_METHOD: u32 = 28;
    pub(crate) const RES_SCALE_INDEX: u32 = 29;
    pub(crate) const SKIP_FRAME: u32 = 30;
    pub(crate) const BG_MUSIC: u32 = 31;
    pub(crate) const ANTI_ALIAS: u32 = 32;
    pub(crate) const AUTO_TV_MODE: u32 = 33;
    pub(crate) const TV_MODE: u32 = 34;
    pub(crate) const TV_BOX: u32 = 35;
    pub(crate) const SORT_OPTION: u32 = 36;
    pub(crate) const SAME_SP_PALETTE: u32 = 37;
    pub(crate) const FOV: u32 = 38;

    pub(crate) const MASK: FieldMask = FieldMask::range(1, 38);
}

pub(super) mod light {
    use crate::layout::FieldMask;

    pub(crate) const DIRECTION: u32 = 1;
    pub(crate) const COLOR: u32 = 2;
    pub(crate) const INTENSITY: u32 = 3;

    pub(crate) const MASK: FieldMask = FieldMask::range(1, 3);
}

pub(super) mod layers {
    use crate::layout::FieldMask;

    pub(crate) const ITEMS: u32 = 1;
    pub(crate) const DEF_LAYER: u32 = 2;
    pub(crate) const AUTO: u32 = 3;

    pub(crate) const MASK: FieldMask = FieldMask::range(1, 3);
}

pub(super) mod layer_params {
    use crate::layout::FieldMask;

    pub(crate) const OFFSET: u32 = 1;
    pub(crate) const ALIGNED: u32 = 2;
    pub(crate) const CAST_SHADOW: u32 = 3;
    pub(crate) const RECV_SHADOW: u32 = 4;

    pub(crate) const MASK: FieldMask = FieldMask::range(1, 4);
}

pub(super) mod recorded_slot {
    use crate::layout::FieldMask;

    pub(crate) const NAME: u32 = 1;
    pub(crate) const PATTERN: u32 = 3;

    pub(crate) const MASK: FieldMask = FieldMask::of(&[1, 3]);
}

/// Field numbers of the small value messages.
pub(super) mod values {
    pub(crate) const X: u32 = 1;
    pub(crate) const Y: u32 = 2;
    pub(crate) const Z: u32 = 3;
    pub(crate) const W: u32 = 4;
    pub(crate) const ARGB_VALUE: u32 = 1;
    pub(crate) const MAT4_FIRST_CELL: u32 = 1;
}
