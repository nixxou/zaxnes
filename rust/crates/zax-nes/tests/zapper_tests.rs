//! The Zapper's light detection (REQ-CORE-060 to REQ-CORE-062, AMD-021 R2-006). The test that
//! reads the position through the `NesCore` position source belongs to T05.

// Test values are small: the casts cannot lose anything.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::unwrap_used
)]

mod common;

use common::TileFixture;
use zax_nes::input::ControllerPortType;
use zax_nes::ppu::SCREEN_HEIGHT;
use zax_nes::{nes_colors, ZapperPosition};

const SOLID_TILE: u8 = 0xFF;
const HEIGHT: i32 = SCREEN_HEIGHT as i32;

fn plug_zapper(fixture: &mut TileFixture, aim: ZapperPosition) {
    let input = fixture.core.bus_mut().input_mut();
    input.set_port_type(1, ControllerPortType::Zapper).unwrap();
    input.set_zapper_position(1, aim).unwrap();
}

fn run_frame(fixture: &mut TileFixture, word: u16) {
    fixture.core.bus_mut().input_mut().begin_frame(word);
    fixture.core.run_frame();
}

fn read_port(fixture: &mut TileFixture) -> u8 {
    fixture.core.bus_mut().read_input_port(1, 0x00)
}

fn white_sprite_on_black() -> TileFixture {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[0x0F]); // Backdrop: black.
    fixture.write_vram(0x3F11, &[0x30]); // Sprite sub-palette 0, colour 1: white.
                                         // OAM Y is one line above the top of the sprite, so Y = 99 puts it at y = 100.
    fixture.set_sprite(0, 99, SOLID_TILE, 0x00, 100);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x10);
    fixture
}

#[test]
fn req_core_061_an_off_screen_pointer_with_the_trigger_released_reads_as_0x18() {
    let mut fixture = TileFixture::new();
    plug_zapper(&mut fixture, ZapperPosition::OFF_SCREEN);
    // Port 2 byte $10: bit 4 set is "trigger released" (DEC-02-Q1, SPEC-08 §4.1).
    run_frame(&mut fixture, 0x1000);
    assert_eq!(0x18, read_port(&mut fixture));
}

#[test]
fn req_core_061_a_scanned_white_pixel_with_the_trigger_pressed_reads_as_0x00() {
    // AMD-021 R2-006: a read made during the vblank sees the universal backdrop, so the
    // backdrop has to be the light colour for the white sprite to be read as light.
    let mut fixture = white_sprite_on_black();
    fixture.write_vram(0x3F00, &[0x30]);
    plug_zapper(&mut fixture, ZapperPosition::new(103, 103));
    run_frame(&mut fixture, 0);
    run_frame(&mut fixture, 0);
    assert_eq!(0x00, read_port(&mut fixture));
}

#[test]
fn amd_021_r2_006_a_vblank_read_sees_only_the_universal_backdrop() {
    for (backdrop, expected) in [(0x0Fu8, false), (0x30, true)] {
        // Dark backdrop: no light, even over a white sprite; light backdrop: light.
        let mut fixture = white_sprite_on_black();
        fixture.write_vram(0x3F00, &[backdrop]);
        let core = &mut fixture.core;
        core.run_frame();
        core.run_frame();
        // RunFrame returns at the start of the vblank, so the records of the frame just drawn
        // are complete, and no longer consulted (R2-006 supersedes Q-005 point 1).
        assert!(core.ppu().scanline() >= HEIGHT);
        assert_eq!(backdrop, core.ppu().universal_backdrop_color());
        assert_eq!(0x30, core.ppu().compose_recorded_color(100, 100));
        assert_eq!(expected, core.ppu().sees_light_at(100, 100));
    }
}

#[test]
fn amd_021_r2_006_the_right_hand_band_is_always_the_universal_backdrop() {
    for (backdrop, expected) in [(0x0Fu8, false), (0x30, true)] {
        let mut fixture = white_sprite_on_black();
        fixture.write_vram(0x3F00, &[backdrop]);
        // A second white sprite covering (240, 100), so the band is not empty.
        fixture.set_sprite(1, 99, SOLID_TILE, 0x00, 240);
        let core = &mut fixture.core;
        core.run_frame();
        // Walk into the next frame until the scan is past the capture of the sprite band of
        // line 100, which REQ-CORE-044 places on line 103.
        let mut guard = 0;
        while (core.ppu().scanline() < 104 || core.ppu().scanline() >= HEIGHT) && guard < 400_000 {
            let (cpu, bus) = core.cpu_and_bus_mut();
            cpu.step(bus);
            guard += 1;
        }

        let line = core.ppu().scanline();
        assert!((104..HEIGHT).contains(&line));
        // Left of the band the records still count: the white sprite at x = 100 is seen.
        assert!(core.ppu().sees_light_at(103, 100));
        // From x = 239 on, only the backdrop counts, even though a white sprite is there.
        assert_eq!(0x30, core.ppu().compose_recorded_color(240, 100));
        assert_eq!(expected, core.ppu().sees_light_at(239, 100));
        assert_eq!(expected, core.ppu().sees_light_at(240, 100));
        assert_eq!(expected, core.ppu().sees_light_at(255, 100));
    }
}

#[test]
fn req_core_061_a_zapper_read_does_not_advance_a_shift_register() {
    let mut fixture = TileFixture::new();
    plug_zapper(&mut fixture, ZapperPosition::OFF_SCREEN);
    run_frame(&mut fixture, 0x1000);
    for _ in 0..16 {
        assert_eq!(0x18, read_port(&mut fixture));
    }

    // Back to a controller, the serial protocol works again: Start on port 2 is bit 3, so the
    // fourth read is the one that is set (REQ-CORE-022, REQ-CORE-024).
    fixture
        .core
        .bus_mut()
        .input_mut()
        .set_port_type(1, ControllerPortType::Controller)
        .unwrap();
    run_frame(&mut fixture, 0x0800);
    fixture.core.bus_mut().input_mut().write(0x01);
    fixture.core.bus_mut().input_mut().write(0x00);
    assert_eq!(0x00, read_port(&mut fixture) & 0x01);
    assert_eq!(0x00, read_port(&mut fixture) & 0x01);
    assert_eq!(0x00, read_port(&mut fixture) & 0x01);
    assert_eq!(0x01, read_port(&mut fixture) & 0x01);
}

#[test]
fn req_core_062_light_is_seen_from_the_line_after_the_aimed_pixel() {
    let mut fixture = white_sprite_on_black();
    let core = &mut fixture.core;
    // One frame so that the programmed state is fully in force, then walk through the next
    // frame instruction by instruction and watch the probe line by line.
    core.run_frame();
    while core.ppu().scanline() != 0 {
        let (cpu, bus) = core.cpu_and_bus_mut();
        cpu.step(bus);
    }

    let mut light = vec![false; SCREEN_HEIGHT];
    let mut guard = 0;
    while core.ppu().scanline() < HEIGHT && guard < 200_000 {
        guard += 1;
        let line = usize::try_from(core.ppu().scanline()).unwrap();
        light[line] |= core.ppu().sees_light_at(103, 103);
        let (cpu, bus) = core.cpu_and_bus_mut();
        cpu.step(bus);
    }

    for (line, seen) in light.iter().enumerate().take(104) {
        assert!(!seen, "line {line} should not see light yet");
    }

    for (line, seen) in light.iter().enumerate().skip(104) {
        assert!(seen, "line {line} should see light");
    }
}

#[test]
fn req_core_062_a_pointer_outside_the_image_never_sees_light() {
    // Condition 1 of REQ-CORE-062 (0 <= x < 256, 0 <= y < 240) is unchanged by R2-006, which
    // only redefines the colour considered inside the image. The backdrop is light here, so
    // the only reason to see no light is the aim being out of the image.
    let mut fixture = white_sprite_on_black();
    fixture.write_vram(0x3F00, &[0x30]);
    let core = &mut fixture.core;
    core.run_frame();
    core.run_frame();
    assert!(!core.ppu().sees_light_at(-1, 103));
    assert!(!core.ppu().sees_light_at(103, -1));
    assert!(!core.ppu().sees_light_at(256, 103));
    assert!(!core.ppu().sees_light_at(103, 240));
    assert!(core.ppu().sees_light_at(103, 103));
}

#[test]
fn req_core_062_the_displayed_colour_follows_the_nes_composition_order() {
    let cases = [
        (true, true, 0x03u8), // The front sprite wins.
        (false, true, 0x01),  // No front sprite: the background shows.
        (false, false, 0x02), // Transparent background: the hidden sprite shows.
    ];
    for (with_front_sprite, with_solid_background, expected) in cases {
        let mut fixture = TileFixture::new();
        fixture.write_vram(0x3F00, &[0x04]); // Universal backdrop.
        fixture.write_vram(0x3F01, &[0x01]); // Background sub-palette 0, colour 1.
        fixture.write_vram(0x3F15, &[0x02]); // Sprite sub-palette 1, colour 1.
        fixture.write_vram(0x3F19, &[0x03]); // Sprite sub-palette 2, colour 1.
        if with_solid_background {
            fixture.fill_vram(0x2000, 960, SOLID_TILE);
        }

        // Sprite 0 is behind the background, sprite 1 in front of it; both cover (103, 103).
        fixture.set_sprite(0, 99, SOLID_TILE, 0x21, 100);
        if with_front_sprite {
            fixture.set_sprite(1, 99, SOLID_TILE, 0x02, 100);
        }

        fixture.set_scroll(0, 0);
        fixture.set_control(0x00);
        fixture.set_mask(0x1E);
        fixture.next_batch();
        assert_eq!(
            expected,
            fixture.core.ppu().compose_recorded_color(103, 103),
            "front sprite {with_front_sprite}, solid background {with_solid_background}"
        );
    }
}

#[test]
fn req_core_062_an_empty_frame_composes_to_the_universal_backdrop() {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[0x04]);
    fixture.set_mask(0x00);
    fixture.next_batch();
    assert_eq!(0x04, fixture.core.ppu().compose_recorded_color(103, 103));
}

#[test]
fn req_core_062_exactly_nine_of_the_64_colours_are_dark() {
    let mut dark = 0;
    for color in 0..nes_colors::COUNT {
        let color = color as u8;
        if nes_colors::is_dark(color) {
            dark += 1;
        }

        assert_ne!(nes_colors::is_dark(color), nes_colors::is_light(color));
    }

    assert_eq!(9, dark);
    assert_eq!(
        [0x0D, 0x0E, 0x0F, 0x1E, 0x1F, 0x2E, 0x2F, 0x3E, 0x3F],
        nes_colors::DARK_COLORS
    );
    // The six unused bits of a colour byte are ignored.
    assert!(nes_colors::is_dark(0x4F));
    assert!(!nes_colors::is_dark(0x70));
}
