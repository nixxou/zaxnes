//! Controller ports and the Zapper byte (REQ-CORE-022, REQ-CORE-024, REQ-CORE-060,
//! REQ-CORE-061, REQ-CORE-062). The tests that go through the `NesCore` facade (the Zapper
//! position source, `GetPortType` on a core) belong to T05.

// Test indices are small: the narrowing casts cannot lose anything.
#![allow(clippy::cast_possible_truncation, clippy::unwrap_used)]

mod common;

use common::{machine_with, minimal, minimal_rom};
use zax_nes::input::{ControllerPortType, InputPorts};
use zax_nes::{nes_colors, ZapperPosition};

fn controller_reader_program() -> Vec<u8> {
    // LDA #$01 / STA $4016 / LDA #$00 / STA $4016: strobe the controllers.
    let mut program = vec![0xA9, 0x01, 0x8D, 0x16, 0x40, 0xA9, 0x00, 0x8D, 0x16, 0x40];
    for i in 0..8u8 {
        // LDA $4016 / AND #$01 / STA $0300+i
        program.extend([0xAD, 0x16, 0x40, 0x29, 0x01, 0x8D, i, 0x03]);
    }

    for i in 0..8u8 {
        // LDA $4017 / AND #$01 / STA $0310+i
        program.extend([0xAD, 0x17, 0x40, 0x29, 0x01, 0x8D, 0x10 + i, 0x03]);
    }

    // JMP $8000
    program.extend([0x4C, 0x00, 0x80]);
    program
}

fn read(ports: &mut InputPorts, port: usize, open_bus: u8, light: bool) -> u8 {
    ports.read(port, open_bus, |_| light)
}

#[test]
fn req_core_022_input_word_0x0081_reads_a_then_right_on_port_one() {
    let image = minimal_rom(0x01, Some(&controller_reader_program()), None);
    let mut core = machine_with(&image, "Pads.nes");
    core.bus_mut().input_mut().begin_frame(0x0081);
    core.run_frame();
    // Serial order: A, B, Select, Start, Up, Down, Left, Right.
    let expected = [1u8, 0, 0, 0, 0, 0, 0, 1];
    for (i, want) in expected.iter().enumerate() {
        assert_eq!(*want, core.read_memory(0x0300 + i as u16), "read {i}");
    }
}

#[test]
fn req_core_022_input_word_0x0800_presses_start_on_port_two() {
    let image = minimal_rom(0x01, Some(&controller_reader_program()), None);
    let mut core = machine_with(&image, "Pads.nes");
    core.bus_mut().input_mut().begin_frame(0x0800);
    core.run_frame();
    // Bit 11 is bit 3 of port 2's byte: Start, the fourth bit read on $4017.
    let expected = [0u8, 0, 0, 1, 0, 0, 0, 0];
    for (i, want) in expected.iter().enumerate() {
        assert_eq!(*want, core.read_memory(0x0310 + i as u16), "read {i}");
    }

    // Port 1 saw nothing.
    for i in 0..8u16 {
        assert_eq!(0, core.read_memory(0x0300 + i));
    }
}

#[test]
fn req_core_022_the_ninth_read_and_the_ones_after_it_return_one() {
    let mut ports = InputPorts::new();
    ports.begin_frame(0x0000);
    ports.write(0x01);
    ports.write(0x00);
    for _ in 0..8 {
        assert_eq!(0, read(&mut ports, 0, 0, false) & 0x01);
    }

    for _ in 0..4 {
        assert_eq!(1, read(&mut ports, 0, 0, false) & 0x01);
    }
}

#[test]
fn req_core_022_frontend_action_bits_never_reach_the_core() {
    // The host strips the bits above $FF of a player's mask before composing the word
    // (SPEC-02 §4.4); the core only ever sees 16 bits, 8 per port.
    let mut ports = InputPorts::new();
    ports.begin_frame(0xFFFF);
    assert_eq!(0xFF, ports.port_byte(0));
    assert_eq!(0xFF, ports.port_byte(1));
}

#[test]
fn req_core_024_ports_default_to_controllers_and_can_be_switched() {
    let mut core = machine_with(&minimal(), "Minimal.nes");
    let input = core.bus_mut().input_mut();
    assert_eq!(Ok(ControllerPortType::Controller), input.port_type(0));
    assert_eq!(Ok(ControllerPortType::Controller), input.port_type(1));
    input.set_port_type(1, ControllerPortType::Zapper).unwrap();
    assert_eq!(Ok(ControllerPortType::Zapper), input.port_type(1));
    input
        .set_port_type(1, ControllerPortType::Controller)
        .unwrap();
    assert_eq!(Ok(ControllerPortType::Controller), input.port_type(1));
}

#[test]
fn req_core_024_an_out_of_range_port_is_rejected() {
    let mut ports = InputPorts::new();
    assert!(ports.port_type(2).is_err());
    assert!(ports.set_port_type(2, ControllerPortType::Zapper).is_err());
}

#[test]
fn req_core_061_zapper_byte_is_trigger_in_bit_four_and_light_in_bit_three() {
    let mut ports = InputPorts::new();
    ports.set_port_type(1, ControllerPortType::Zapper).unwrap();
    // Trigger released (bit 4 = 1), aim off screen: no light, so bit 3 is 1 -> $18.
    ports.begin_frame(0x1000);
    ports
        .set_zapper_position(1, ZapperPosition::OFF_SCREEN)
        .unwrap();
    assert_eq!(0x18, read(&mut ports, 1, 0xFF, false));
    // Trigger pressed (bit 4 = 0), light seen: both bits clear -> $00.
    ports.begin_frame(0x0000);
    assert_eq!(0x00, read(&mut ports, 1, 0xFF, true));
    // Trigger released and light seen -> $10.
    ports.begin_frame(0x1000);
    assert_eq!(0x10, read(&mut ports, 1, 0xFF, true));
}

#[test]
fn req_core_061_a_zapper_read_does_not_advance_a_shift_register() {
    let mut ports = InputPorts::new();
    ports.set_port_type(1, ControllerPortType::Zapper).unwrap();
    ports.begin_frame(0x1000);
    for _ in 0..20 {
        assert_eq!(0x18, read(&mut ports, 1, 0x00, false));
    }
}

#[test]
fn req_core_024_switching_back_to_a_controller_restores_serial_reads() {
    let mut ports = InputPorts::new();
    ports.set_port_type(1, ControllerPortType::Zapper).unwrap();
    ports.begin_frame(0x0100); // Port 2 byte = $01: button A pressed, trigger pressed.
                               // Bit 4 of $01 is clear, so the Zapper byte is just "no light".
    assert_eq!(0x08, read(&mut ports, 1, 0x00, false));
    ports
        .set_port_type(1, ControllerPortType::Controller)
        .unwrap();
    ports.write(0x01);
    ports.write(0x00);
    assert_eq!(1, read(&mut ports, 1, 0x00, false) & 0x01); // A.
    assert_eq!(0, read(&mut ports, 1, 0x00, false) & 0x01); // B.
}

#[test]
fn req_core_062_exactly_nine_of_the_sixty_four_colours_are_dark() {
    // SPEC-02 §5.4 asks for this to be checked exhaustively.
    let mut dark = Vec::new();
    for color in 0..nes_colors::COUNT {
        let color = color as u8;
        if nes_colors::is_dark(color) {
            dark.push(color);
        }

        assert_ne!(nes_colors::is_dark(color), nes_colors::is_light(color));
    }

    assert_eq!(9, dark.len());
    assert_eq!(
        vec![0x0D, 0x0E, 0x0F, 0x1E, 0x1F, 0x2E, 0x2F, 0x3E, 0x3F],
        dark
    );
    assert_eq!(nes_colors::DARK_COLORS.to_vec(), dark);
}

#[test]
fn req_core_062_the_aim_must_be_inside_the_image() {
    assert!(ZapperPosition::new(0, 0).is_on_screen());
    assert!(ZapperPosition::new(255, 239).is_on_screen());
    assert!(!ZapperPosition::new(256, 0).is_on_screen());
    assert!(!ZapperPosition::new(0, 240).is_on_screen());
    assert!(!ZapperPosition::new(-1, 0).is_on_screen());
    assert!(!ZapperPosition::OFF_SCREEN.is_on_screen());
}
