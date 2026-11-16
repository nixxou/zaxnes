//! The in-game resolution of a "3dh" profile driven by a real core on a cartridge with CHR-RAM,
//! through a whole session (AMD-062 REQ-PROF-080, SPEC-02 REQ-CORE-123).
//!
//! The criterion of REQ-PROF-080 has the second tile "appear only on screen (sprite)". On a
//! mapper 0 cartridge with CHR-RAM that cannot happen: the pattern data a sprite is drawn from
//! **is** the CHR-RAM, and the PPU reads it at a sixteen-byte aligned address, so the stateless
//! walk of REQ-PROF-080 always holds it too. The two paths are therefore checked separately:
//! this run shows the CHR-RAM path on a real core, and
//! `zax-engine3d`'s `amd062_tile_resolution_tests` shows the frame-tile path with no CHR-RAM
//! offered at all, which is exactly "only on screen". See Q-178 §1.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

use std::sync::Arc;

use zax_app::game_session::{GameSession, SessionRequest};
use zax_app::input::zapper::RELEASED_BYTE;
use zax_app::input::InputAction;
use zax_nes::ZapperPosition;
use zax_profiles::enums::TileFlip;
use zax_profiles::hash::envelope::HashProfileManifest;
use zax_profiles::hash::raw_tile::{self, RawTile};
use zax_profiles::hash::tile_id::TileId;
use zax_profiles::hash::tiles::TileResolver;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::tile::{HashTileField, PatternGrid, ProfileTile};
use zax_profiles::values::Vec2I;
use zax_profiles::variant::PatternVariant;
use zax_render::scene::SceneState;
use zax_scripting::NullScriptLog;
use zax_settings::model::SettingsModel;

/// Where the first tile is written: aligned, and outside the pattern table a sprite is drawn
/// from.
const FIRST_TILE_ADDRESS: u16 = 0x1000;

/// Where the second tile is written: pattern 1 of the sprite table at `$0000`.
const SECOND_TILE_ADDRESS: u16 = 0x0010;

fn raw_one() -> RawTile {
    raw_tile::from_planes(0x0F0F_0F0F_0F0F_0F0F, 0x00FF_00FF_00FF_00FF)
}

fn raw_two() -> RawTile {
    raw_tile::from_planes(0x1234_5678_9ABC_DEF0, 0x0FED_CBA9_8765_4321)
}

// REQ-PROF-080: the first tile is resolved on the very frame the test writes it into the
// CHR-RAM, the second on the frame it is written and shown, and neither before.
#[test]
fn req_prof_080_a_tile_written_into_the_chr_ram_is_resolved_on_that_frame() {
    let mut run = Run::new();

    for frame in 0..10 {
        run.frame();
        assert_eq!(2, run.waiting(), "frame {frame}");
    }

    // Frame 10: the first tile goes into the CHR-RAM, mirrored vertically and aligned.
    run.write_tile(
        FIRST_TILE_ADDRESS,
        &raw_tile::flipped(&raw_one(), TileFlip::VERTICAL),
    );
    run.frame();
    assert_eq!(1, run.waiting());
    assert_eq!(1, run.resolved_while_playing());

    for frame in 11..20 {
        run.frame();
        assert_eq!(1, run.waiting(), "frame {frame}");
    }

    // Frame 20: the second tile appears, as the pattern of a displayed sprite.
    run.write_tile(SECOND_TILE_ADDRESS, &raw_two());
    run.show_sprite(1, 64, 64);
    run.frame();

    assert_eq!(0, run.waiting());
    assert_eq!(2, run.resolved_while_playing());
    // Both patterns are whole again, so each has the image of its cell and its own geometry.
    for index in 0..2 {
        let pattern = run.permanent_pattern(index);
        assert!(!pattern.0, "pattern {index} still has a placeholder");
        assert_eq!(1, pattern.1, "pattern {index} images");
    }
}

// REQ-PROF-080: the resolved set is session knowledge. A rollback of five frames after frame 20
// puts neither tile back to waiting, and neither does a reset.
#[test]
fn req_prof_080_a_rollback_puts_no_tile_back_to_waiting() {
    let mut run = Run::new();
    let mut snapshot = None;

    for frame in 0..=25 {
        if frame == 16 {
            // The state the rollback of five frames will come back to.
            snapshot = Some(run.save_state());
        }
        if frame == 10 {
            run.write_tile(
                FIRST_TILE_ADDRESS,
                &raw_tile::flipped(&raw_one(), TileFlip::VERTICAL),
            );
        }
        if frame == 20 {
            run.write_tile(SECOND_TILE_ADDRESS, &raw_two());
            run.show_sprite(1, 64, 64);
        }
        run.frame();
    }

    assert_eq!(0, run.waiting());

    run.load_state(&snapshot.expect("a snapshot"));
    assert_eq!(0, run.waiting());
    for index in 0..2 {
        assert!(!run.permanent_pattern(index).0);
    }

    // The tiles are still there after a reset as well (SPEC-02 REQ-CORE-122).
    run.reset();
    run.frame();
    assert_eq!(0, run.waiting());
}

// REQ-ARCH-012 as REQ-PROF-080 states it: at equal initial resolved set — an empty cache — two
// runs of the same ROM, profile and inputs give the same frames.
#[test]
fn req_prof_080_two_runs_with_an_empty_cache_give_the_same_frames() {
    let first = capture();
    let second = capture();

    assert_eq!(second.len(), first.len());
    assert_eq!(second, first);
}

fn capture() -> Vec<String> {
    let mut run = Run::new();
    let mut frames = Vec::new();
    for frame in 0..=25 {
        if frame == 10 {
            run.write_tile(
                FIRST_TILE_ADDRESS,
                &raw_tile::flipped(&raw_one(), TileFlip::VERTICAL),
            );
        }
        if frame == 20 {
            run.write_tile(SECOND_TILE_ADDRESS, &raw_two());
            run.show_sprite(1, 64, 64);
        }
        run.frame();
        frames.push(run.frame_summary(frame));
    }

    frames
}

/// One run: a session over a mapper 0 cartridge with 32 KiB of CHR-RAM, playing a "3dh" profile
/// whose two tiles are nowhere in the ROM file.
struct Run {
    session: GameSession,
    scene: SceneState,
    model: SettingsModel,
}

impl Run {
    fn new() -> Self {
        let (profile, resolver) = hash_profile();
        // REQ-PROF-078 and REQ-PROF-082: the ROM holds neither tile and no cache is applied, so
        // both start waiting.
        assert_eq!(2, resolver.waiting_count());

        let mut scene = SceneState::default();
        let mut model = SettingsModel::new();
        let request = SessionRequest {
            profile: &profile,
            title: "ChrRam",
            scene: &mut scene,
            log: Box::new(NullScriptLog),
            different_game: true,
            two_dimensional_mode: Some(false),
            console_interface: false,
            tiles: Some(resolver),
            engine: zax_core_api::EngineKind::Zaxnes,
        };
        let mut session = GameSession::create(request, &mut model).unwrap();
        assert!(session.load_rom(&chr_ram_rom(), "ChrRam.nes").success);
        session.start(&mut model, &mut scene);
        Self {
            session,
            scene,
            model,
        }
    }

    fn frame(&mut self) {
        self.session.run_frame(
            &mut self.model,
            &mut self.scene,
            InputAction::NONE,
            InputAction::NONE,
            RELEASED_BYTE,
            ZapperPosition::new(0, 0),
        );
    }

    /// How many manifest strings a cell is still waiting for (REQ-PROF-081).
    fn waiting(&self) -> usize {
        self.session
            .engine()
            .hash_tiles()
            .expect("a 3dh profile")
            .waiting_count()
    }

    fn resolved_while_playing(&self) -> usize {
        self.session
            .engine()
            .hash_tiles()
            .expect("a 3dh profile")
            .resolved_while_playing()
    }

    /// Whether permanent pattern `index` still holds a waiting cell, and how many images it has.
    fn permanent_pattern(&self, index: usize) -> (bool, usize) {
        let engine = self.session.engine();
        let id = engine.patterns().permanent()[index];
        let pattern = engine.patterns().get(id).expect("a pattern");
        (pattern.has_placeholders(), pattern.images().len())
    }

    /// What the conversion made of the frame, as a line two runs are compared on.
    fn frame_summary(&self, frame: i32) -> String {
        let converted = self.session.engine().converting_frame();
        format!(
            "{frame}:{}:{}:{}",
            self.waiting(),
            converted.shapes().len(),
            converted.instances().len()
        )
    }

    /// Writes sixteen bytes at a PPU address through `$2006` and `$2007`, as the game would.
    fn write_tile(&mut self, address: u16, raw: &RawTile) {
        let bus = self.session.core_mut().bus_mut().expect("Zaxnes's bus");
        bus.poke(0x2000, 0x00);
        #[allow(clippy::cast_possible_truncation)] // The two halves of an address.
        bus.poke(0x2006, (address >> 8) as u8);
        #[allow(clippy::cast_possible_truncation)]
        bus.poke(0x2006, address as u8);
        for value in raw {
            bus.poke(0x2007, *value);
        }
    }

    /// Puts sprite 0 on screen and turns sprite rendering on.
    fn show_sprite(&mut self, tile: u8, x: u8, y: u8) {
        let bus = self.session.core_mut().bus_mut().expect("Zaxnes's bus");
        bus.poke(0x2003, 0x00);
        bus.poke(0x2004, y.wrapping_sub(1));
        bus.poke(0x2004, tile);
        bus.poke(0x2004, 0x00);
        bus.poke(0x2004, x);

        // Sprite palette entry 1 must not be the backdrop, or the sprite draws nothing.
        bus.poke(0x2006, 0x3F);
        bus.poke(0x2006, 0x11);
        bus.poke(0x2007, 0x21);
        bus.poke(0x2006, 0x3F);
        bus.poke(0x2006, 0x12);
        bus.poke(0x2007, 0x11);

        bus.poke(0x2000, 0x00);
        bus.poke(0x2001, 0x10);
    }

    fn save_state(&self) -> Vec<u8> {
        self.session.core().save_state(false).expect("a state")
    }

    fn load_state(&mut self, state: &[u8]) {
        self.session
            .core_mut()
            .try_load_state(state, 0)
            .expect("the state loads");
    }

    fn reset(&mut self) {
        self.session.core_mut().reset();
    }
}

/// A "3dh" profile of two one-cell patterns, each waiting for one of the two tiles.
fn hash_profile() -> (GameProfile, TileResolver) {
    let mut profile = GameProfile::create_empty();
    let mut manifest = HashProfileManifest::default();
    for raw in [raw_one(), raw_two()] {
        let id = TileId::compute(&raw);
        manifest.add_identifier(id);
        let tile = ProfileTile::waiting(
            Arc::from(id.to_text().as_str()),
            HashTileField::Identifier,
            TileFlip::NONE,
        );
        profile.permanent_patterns.push(Arc::new(ProfilePattern {
            grid: Some(PatternGrid::new(1, 1, vec![Some(Arc::new(tile))])),
            tile_size: Some(Vec2I::new(1, 1)),
            permanent: true,
            variants: vec![Arc::new(PatternVariant {
                depth: 3,
                alpha: 1.0,
                ..PatternVariant::default()
            })],
            ..ProfilePattern::default()
        }));
    }

    (profile, TileResolver::of(&manifest))
}

/// A mapper 0 image with no CHR-ROM: 32 KiB of CHR-RAM (SPEC-02 REQ-CORE-012). The program does
/// nothing — the test drives the PPU itself.
fn chr_ram_rom() -> Vec<u8> {
    const PRG_SIZE: usize = 2 * 16 * 1024;
    let mut image = vec![0_u8; 16 + PRG_SIZE];
    image[..4].copy_from_slice(&[b'N', b'E', b'S', 0x1A]);
    image[4] = 0x02;
    image[5] = 0x00;
    image[6] = 0x01;

    // JMP $8000.
    image[16] = 0x4C;
    image[17] = 0x00;
    image[18] = 0x80;

    let vectors = 16 + PRG_SIZE - 6;
    for vector in 0..3 {
        image[vectors + (2 * vector)] = 0x00;
        image[vectors + (2 * vector) + 1] = 0x80;
    }

    image
}
