//! The synthetic frames the renderer is checked against, each built from the acceptance
//! criteria of SPEC-06 so that they can be verified without the user's ROM (port of
//! `SyntheticScenes.cs`).

use std::sync::Arc;

use zax_engine3d::draw_list::FrameDrawList;
use zax_engine3d::engine::ConversionEngine;
use zax_engine3d::palette::Rgb24;
use zax_engine3d::settings::{ConversionSettings, RenderAlgorithm, ZLayer};
use zax_nes::{FrameBatch, TileRecord};
use zax_profiles::enums::{ProfileLayer, TileFlip, VariantRenderStyle};
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::tile::{PatternGrid, ProfileTile};
use zax_profiles::values::{Vec2I, Vec4F};
use zax_profiles::variant::PatternVariant;
use zax_render::numerics::Vector3;
use zax_render::scene::{SceneFrame, SceneState};

use crate::HarnessError;

/// Names of every scene, in a stable order.
pub const NAMES: [&str; 11] = [
    "framing",
    "sky",
    "shadow",
    "no-shadow",
    "layer-shadow-off",
    "nudge",
    "alpha",
    "hud",
    "clip",
    "backdrop",
    "heavy",
];

/// One synthetic frame ready to render: the engine that produced it, its draw list and the
/// scene state SPEC-06 needs.
pub struct SyntheticScene {
    /// Name of the scene, used for the capture file.
    pub name: String,
    /// One line saying what the scene checks.
    pub description: String,
    /// The scene state, settings and camera included.
    pub scene: SceneState,
    /// The engine that produced the frame.
    pub engine: ConversionEngine,
    /// The tile list of the displayed frame.
    pub draw_list: FrameDrawList,
    /// The 256 x 240 colour indices of the core's picture, empty without a ROM.
    pub nes_picture: Vec<u8>,
}

impl SyntheticScene {
    /// The frame data to render.
    #[must_use]
    pub fn frame(&self) -> SceneFrame<'_> {
        SceneFrame {
            nes_picture: &self.nes_picture,
            ..SceneFrame::with_draw_list(&self.draw_list)
        }
    }
}

/// Builds the scene called `name` with the mesh algorithm `mesh` (REQ-E3D-340).
///
/// # Errors
///
/// [`HarnessError::Usage`] when the name is not one of [`NAMES`]; [`HarnessError::Engine`]
/// when the engine refuses the profile.
pub fn build(name: &str, mesh: RenderAlgorithm) -> Result<SyntheticScene, HarnessError> {
    match name {
        "framing" => framing(mesh),
        "sky" => sky(mesh),
        "shadow" => shadow_pair(mesh, true, true),
        "no-shadow" => shadow_pair(mesh, false, true),
        "layer-shadow-off" => shadow_pair(mesh, true, false),
        "nudge" => nudge(mesh),
        "alpha" => alpha(mesh),
        "hud" => hud(mesh),
        "clip" => clip(mesh),
        "backdrop" => backdrop(mesh),
        "heavy" => heavy(mesh),
        _ => Err(HarnessError::Usage(format!(
            "unknown synthetic scene '{name}'"
        ))),
    }
}

/// A flat background of one colour over the whole picture, with the universal background
/// colour 0x22. It pins down the default camera framing (REQ-SCN-012) and the background colour
/// rule of REQ-SCN-046.
fn framing(mesh: RenderAlgorithm) -> Result<SyntheticScene, HarnessError> {
    let mut builder = Builder::new(mesh);
    builder.palette(0, 0x22);
    builder.palette(1, 0x30);
    for row in 1..30 {
        for column in 0..32 {
            builder.background(solid(1, 0), column * 8, row * 8, 0);
        }
    }

    builder.finish(
        "framing",
        "flat 0x30 background over the whole picture, background colour 0x22, default camera",
        |_| {},
    )
}

/// An empty frame: nothing but the sky, with a palette for the Fantasy sky (REQ-SCN-051).
fn sky(mesh: RenderAlgorithm) -> Result<SyntheticScene, HarnessError> {
    let mut builder = Builder::new(mesh);
    builder.palette(0, 0x0F);
    builder.palette(1, 0x21);
    builder.palette(2, 0x12);
    builder.palette(3, 0x30);
    builder.finish(
        "sky",
        "an empty frame with a palette, so that only the skybox shows",
        |_| {},
    )
}

/// The acceptance scene of REQ-SCN-030: a solid tile in L1 above a flat tile in L2 that
/// receives the shadows, light (1, −1, 1).
fn shadow_pair(
    mesh: RenderAlgorithm,
    shadows: bool,
    layer_casts: bool,
) -> Result<SyntheticScene, HarnessError> {
    let mut builder = Builder::new(mesh);
    builder.palette(0, 0x0F);
    builder.palette(1, 0x30);
    builder.palette(2, 0x16);

    // A 16 × 10 tile floor in L2, flat (depth 1), which receives the shadows.
    builder.add_pattern(solid(1, 0), 1, ProfileLayer::L2, 1.0, true, 1);
    for row in 10..20 {
        for column in 8..24 {
            builder.background(solid(1, 0), column * 8, row * 8, 0);
        }
    }

    // A 2 × 2 solid block in L1, deep, which casts on the floor.
    builder.add_pattern(solid(2, 0), 8, ProfileLayer::L1, 1.0, true, 1);
    for row in 13..15 {
        for column in 14..16 {
            builder.background(solid(2, 0), column * 8, row * 8, 0);
        }
    }

    let name = match (shadows, layer_casts) {
        (false, _) => "no-shadow",
        (true, true) => "shadow",
        (true, false) => "layer-shadow-off",
    };
    let mut scene = builder.finish(
        name,
        "solid L1 block over a flat L2 floor, light (1, -1, 1)",
        |settings| {
            settings.shadow = shadows;
            if !layer_casts {
                // REQ-SCN-032: clearing the casting flag of a layer removes its shadows from
                // the next frame onwards.
                settings.layer_mut(ZLayer::L1).cast_shadow = false;
            }
        },
    )?;
    scene.scene.settings.shadows = shadows;
    Ok(scene)
}

/// The acceptance scene of REQ-SCN-037: a flat sprite laid over a flat background tile at the
/// same depth. The ±0.1 nudge the renderer applies has to keep the sprite in front.
fn nudge(mesh: RenderAlgorithm) -> Result<SyntheticScene, HarnessError> {
    let mut builder = Builder::new(mesh);
    builder.palette(0, 0x0F);
    builder.palette(1, 0x21);
    builder.palette(17, 0x16);

    builder.add_pattern(solid(1, 0), 1, ProfileLayer::L1, 1.0, true, 1);
    for row in 12..18 {
        for column in 12..20 {
            builder.background(solid(1, 0), column * 8, row * 8, 0);
        }
    }

    // The sprite uses palette 0 of the sprite tables, so colour index 1 is entry 17.
    builder.add_pattern(solid(2, 0), 1, ProfileLayer::L1, 1.0, true, 1);
    builder.palette(18, 0x16);
    for row in 13..17 {
        for column in 13..19 {
            builder.sprite(solid(2, 0), column * 8, row * 8, 0);
        }
    }

    builder.finish(
        "nudge",
        "flat sprite over a flat background tile at the same depth (REQ-SCN-037)",
        |_| {},
    )
}

/// A pattern of alpha 0.5 over a red background (REQ-SCN-040).
fn alpha(mesh: RenderAlgorithm) -> Result<SyntheticScene, HarnessError> {
    let mut builder = Builder::new(mesh);
    builder.palette(0, 0x0F);
    builder.palette(1, 0x16);
    builder.palette(5, 0x30);

    builder.add_pattern(solid(1, 0), 1, ProfileLayer::L2, 1.0, true, 1);
    for row in 10..20 {
        for column in 8..24 {
            builder.background(solid(1, 0), column * 8, row * 8, 0);
        }
    }

    builder.add_pattern(solid(2, 0), 6, ProfileLayer::L1, 0.5, true, 1);
    builder.palette(2, 0x30);
    for row in 13..17 {
        for column in 13..19 {
            builder.background(solid(2, 0), column * 8, row * 8, 0);
        }
    }

    builder.finish(
        "alpha",
        "alpha 0.5 pattern in front of a red wall (REQ-SCN-040)",
        |_| {},
    )
}

/// A UI pattern placed outside the visible rectangle, which has to stay whole (REQ-SCN-044).
fn hud(mesh: RenderAlgorithm) -> Result<SyntheticScene, HarnessError> {
    let mut builder = Builder::new(mesh);
    builder.palette(0, 0x0F);
    builder.palette(1, 0x30);
    builder.add_pattern(solid(1, 0), 2, ProfileLayer::Ui, 1.0, true, 1);

    // Screen row 0 is scene y in [240, 248], above the top edge of R (REQ-E3D-410).
    for column in 4..12 {
        builder.background(solid(1, 0), column * 8, 0, 0);
    }

    builder.finish(
        "hud",
        "UI pattern above the visible rectangle (REQ-SCN-044)",
        |_| {},
    )
}

/// An object straddling the left edge of R, which has to be cut (REQ-SCN-036).
fn clip(mesh: RenderAlgorithm) -> Result<SyntheticScene, HarnessError> {
    let mut builder = Builder::new(mesh);
    builder.palette(0, 0x0F);
    builder.palette(1, 0x30);
    builder.add_pattern(solid(1, 0), 8, ProfileLayer::L1, 1.0, true, 1);
    for row in 8..22 {
        for column in 0..6 {
            builder.background(solid(1, 0), column * 8, row * 8, 0);
        }
    }

    let mut scene = builder.finish(
        "clip",
        "block straddling the left edge of R (REQ-SCN-036)",
        |_| {},
    )?;
    scene.scene.clipping = Vec4F::new(24.1, 8.1, 8.1, 0.1);
    Ok(scene)
}

/// The backdrop of REQ-SCN-048, 26 units behind the scene.
fn backdrop(mesh: RenderAlgorithm) -> Result<SyntheticScene, HarnessError> {
    let mut scene = shadow_pair(mesh, true, true)?;
    "backdrop".clone_into(&mut scene.name);
    "black backdrop at z = 25.5 receiving the front shadows".clone_into(&mut scene.description);
    let backdrop = &mut scene.scene.backdrop;
    backdrop.enabled = true;
    backdrop.color = Rgb24::new(0, 0, 0);
    backdrop.offset = Vector3::new(0.0, 0.0, 26.0);
    Ok(scene)
}

/// The heavy frame of REQ-SCN-064: a full background grid plus 64 sprites, every tile its own
/// shape, which is what the performance target is measured on.
fn heavy(mesh: RenderAlgorithm) -> Result<SyntheticScene, HarnessError> {
    let mut builder = Builder::new(mesh);
    builder.palette(0, 0x22);
    for entry in 1..64_u8 {
        builder.palette(usize::from(entry), 0x01 + (entry % 0x3F));
    }

    // 32 distinct images, each its own non-self-connected pattern carrying two 3D objects
    // (REQ-E3D-541), so that one tile is one shape and two drawn tiles: 960 background shapes
    // plus 64 sprite shapes make 1024 shapes and 2048 tiles, the load REQ-SCN-064 names.
    for image in 0..32 {
        builder.add_pattern(solid(1, image), 6, ProfileLayer::L1, 1.0, false, 2);
    }

    let mut n = 0;
    for row in 1..31 {
        for column in 0..32 {
            builder.background(
                solid(1, n % 32),
                column * 8,
                row * 8,
                low_bits(row + column),
            );
            n += 1;
        }
    }

    for sprite in 0..64 {
        builder.sprite(
            solid(1, sprite % 32),
            (sprite * 3) % 248,
            8 + ((sprite * 5) % 224),
            low_bits(sprite),
        );
    }

    builder.finish(
        "heavy",
        "full background grid and 64 sprites, one shape per tile",
        |_| {},
    )
}

/// The two low bits of `value`, a palette number.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Masked to 0..3.
const fn low_bits(value: i32) -> u8 {
    (value & 3) as u8
}

/// A full tile of colour `color` whose identity is unique: one pixel carries the extra colour
/// bit.
fn solid(color: i32, id: i32) -> (u64, u64) {
    let mut low = if color & 1 != 0 { u64::MAX } else { 0 };
    let mut high = if color & 2 != 0 { u64::MAX } else { 0 };
    if color & 2 == 0 {
        high = 1 << (id & 63);
    } else {
        low = if color & 1 != 0 {
            u64::MAX
        } else {
            1 << (id & 63)
        };
    }

    (low, high)
}

/// Assembles a profile and a batch of tile records, then converts them with a real engine and a
/// real geometry builder.
struct Builder {
    mesh: RenderAlgorithm,
    profile: GameProfile,
    records: Vec<TileRecord>,
    palette: [u8; 32],
    sprite_order: i32,
}

impl Builder {
    fn new(mesh: RenderAlgorithm) -> Self {
        Self {
            mesh,
            profile: GameProfile::create_empty(),
            records: Vec::new(),
            palette: [0; 32],
            sprite_order: 0,
        }
    }

    /// Sets one of the 32 PPU palette bytes; entry 0 is the universal background colour.
    fn palette(&mut self, entry: usize, nes_color: u8) {
        if let Some(slot) = self.palette.get_mut(entry) {
            *slot = nes_color;
        }
    }

    fn add_pattern(
        &mut self,
        (low, high): (u64, u64),
        depth: i32,
        layer: ProfileLayer,
        alpha: f32,
        self_connected: bool,
        objects: usize,
    ) {
        let tile = Arc::new(ProfileTile::new(low, high, TileFlip::NONE));
        let mut pattern = ProfilePattern {
            grid: Some(PatternGrid::new(1, 1, vec![Some(tile)])),
            tile_size: Some(Vec2I::new(1, 1)),
            permanent: true,
            self_connected,
            constraint_size: Vec2I::UNCONSTRAINED,
            ..ProfilePattern::default()
        };
        for _ in 0..objects {
            pattern.variants.push(Arc::new(PatternVariant {
                depth,
                layer,
                alpha,
                render_style: VariantRenderStyle::Normal,
                ..PatternVariant::default()
            }));
        }
        self.profile.permanent_patterns.push(Arc::new(pattern));
    }

    fn background(&mut self, tile: (u64, u64), x: i32, y: i32, palette: u8) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        // A frame holds far fewer records.
        let index = self.records.len() as i32;
        self.record(tile, x, y, index, palette, true);
    }

    fn sprite(&mut self, tile: (u64, u64), x: i32, y: i32, palette: u8) {
        let order = self.sprite_order;
        self.sprite_order += 1;
        self.record(tile, x, y, order, palette, false);
    }

    #[allow(clippy::cast_possible_truncation)] // Screen positions, as the C# (short) casts.
    fn record(
        &mut self,
        (low, high): (u64, u64),
        x: i32,
        y: i32,
        index: i32,
        palette: u8,
        background: bool,
    ) {
        self.records.push(TileRecord::new(
            low, high, x as i16, y as i16, index, false, false, false, palette, background,
        ));
    }

    fn finish(
        self,
        name: &'static str,
        description: &'static str,
        configure: impl FnOnce(&mut ConversionSettings),
    ) -> Result<SyntheticScene, HarnessError> {
        let mut engine = ConversionEngine::new();
        engine.use_geometry();
        engine.settings_mut().render_algorithm = self.mesh;
        engine.load_profile(&self.profile, true)?;

        // After the load, because loading a profile resets the layer shadow permissions
        // (REQ-SCN-033, REQ-PROF-042).
        configure(engine.settings_mut());

        let batch = FrameBatch {
            records: &self.records,
            palette: self.palette,
            mask_x: 0,
            mask_y: 0,
            margin: 0,
            visible_rows: zax_nes::STANDARD_ROWS,
        };
        engine.convert(&batch, 0);
        let time = engine.publish().time();
        let background = engine.frame_color(0);

        let mut scene = SceneState::default();
        scene.camera.snap_to_setpoints();
        scene.game_running = true;
        scene.clipping = engine.settings().clipping;
        scene.nes_background_color = background;
        scene.emulated_time = time;
        let draw_list = engine.build_draw_list().clone();

        Ok(SyntheticScene {
            name: name.to_owned(),
            description: description.to_owned(),
            scene,
            engine,
            draw_list,
            nes_picture: Vec::new(),
        })
    }
}
