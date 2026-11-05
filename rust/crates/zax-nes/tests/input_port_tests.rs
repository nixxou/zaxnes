//! Controller ports and the Zapper byte (REQ-CORE-022, REQ-CORE-024, REQ-CORE-060,
//! REQ-CORE-061, REQ-CORE-062).

// Test indices are small: the narrowing casts cannot lose anything.
#![allow(clippy::cast_possible_truncation, clippy::unwrap_used)]

mod common;

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

use common::{loaded_core, machine_with, minimal, minimal_rom};
use zax_nes::cpu::CpuBus;
use zax_nes::input::{ControllerPortType, InputPorts};
use zax_nes::{nes_colors, NesCore, ZapperPosition};

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

/// A read on a cycle well past any strobe these tests write, so that a reload the `$4016`
/// write armed for the next put cycle has always landed (see `InputPorts::write`).
fn read(ports: &mut InputPorts, port: usize, open_bus: u8, light: bool) -> u8 {
    ports.read(port, open_bus, READ_CYCLE, |_| light)
}

/// The cycle [`read`] pretends to be on.
const READ_CYCLE: i64 = 1_000;

#[test]
fn req_core_022_input_word_0x0081_reads_a_then_right_on_port_one() {
    let image = minimal_rom(0x01, Some(&controller_reader_program()), None);
    let mut core = machine_with(&image, "Pads.nes");
    core.run_frame(0x0081);
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
    core.run_frame(0x0800);
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
    ports.write(0x01, 10); // A get cycle: the reload lands on cycle 11.
    ports.write(0x00, 14);
    for _ in 0..8 {
        assert_eq!(0, read(&mut ports, 0, 0, false) & 0x01);
    }

    for _ in 0..4 {
        assert_eq!(1, read(&mut ports, 0, 0, false) & 0x01);
    }
}

/// `AccuracyCoin`'s `Controller Strobing` 3 and 4 (REQ-CORE-022): `DEC $4016` holds the strobe
/// high for exactly one cycle — `$41` on its fifth cycle, `$40` on its sixth — and the
/// controllers are reloaded only when the **sixth** cycle, the one the raised line is actually
/// high for, is a put cycle. A put cycle is an odd one in this core's numbering, so the reload
/// happens when the *raising* write lands on an even cycle; `AccuracyCoin` is what pins that
/// mapping, and `InputPorts::write` carries the ROM's cycle annotations.
///
/// Driven on the bus rather than on [`InputPorts`] alone, so that it pins the cycle `NesBus`
/// hands the port as well as the rule itself. The two writes land on consecutive cycles, which
/// is what the ROM's read-modify-write does.
#[test]
fn req_core_022_a_one_cycle_strobe_reloads_only_on_a_put_cycle() {
    // No buttons held, so a reloaded register reads eight zeroes and a register left depleted
    // by the reads before it reads eight ones (the ninth-read rule above).
    fn eight_bits_after_a_one_cycle_strobe(raise_on_even: bool) -> u8 {
        let (_cpu, mut bus) = loaded_core(None);
        bus.input_mut().begin_frame(0x0000);

        // Strobe the ordinary way — two writes four cycles apart, as every game does — and read
        // the eight bits out, which leaves the shift register depleted.
        bus.write(0x4016, 0x01);
        bus.write(0x4016, 0x00);
        for _ in 0..8 {
            bus.read(0x4016);
        }

        // Align so that the *next* cycle, the one the raising write lands on, has the wanted
        // parity.
        while ((bus.cycle_count() + 1) & 1 == 0) != raise_on_even {
            bus.tick();
        }

        // `DEC $4016`'s last two cycles, one after the other.
        bus.write(0x4016, 0x41);
        assert_eq!(
            raise_on_even,
            bus.cycle_count() & 1 == 0,
            "the raising write did not land where the test meant it to"
        );
        bus.write(0x4016, 0x40);

        let mut bits = 0u8;
        for i in 0..8 {
            bits |= (bus.read(0x4016) & 0x01) << i;
        }
        bits
    }

    // Test 3: the raising write is on a get cycle, so the line is high for the put cycle after
    // it and the controllers are strobed. The ROM's `ReadControllerInto50_and_A` must come back
    // with $00.
    assert_eq!(0x00, eight_bits_after_a_one_cycle_strobe(true));

    // Test 4: the raising write is on a put cycle, so the line is high only for a get cycle and
    // the reload it armed for the put cycle after that is taken back by the lowering write.
    // Nothing is strobed and the depleted register keeps answering 1 — the ROM requires $FF.
    assert_eq!(0xFF, eight_bits_after_a_one_cycle_strobe(false));
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
    assert_eq!(Ok(ControllerPortType::Controller), core.port_type(0));
    assert_eq!(Ok(ControllerPortType::Controller), core.port_type(1));

    core.set_port_type(1, ControllerPortType::Zapper).unwrap();
    assert_eq!(Ok(ControllerPortType::Zapper), core.port_type(1));

    core.set_port_type(1, ControllerPortType::Controller)
        .unwrap();
    assert_eq!(Ok(ControllerPortType::Controller), core.port_type(1));
}

#[test]
fn req_core_024_an_out_of_range_port_is_rejected() {
    let mut core = NesCore::new();
    assert!(core.port_type(2).is_err());
    assert!(core.set_port_type(2, ControllerPortType::Zapper).is_err());

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
    ports.write(0x01, 10); // A get cycle: the reload lands on cycle 11.
    ports.write(0x00, 14);
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

#[test]
fn req_core_060_the_position_is_sampled_once_at_the_start_of_the_frame() {
    let mut core = machine_with(&minimal(), "Minimal.nes");
    core.set_port_type(1, ControllerPortType::Zapper).unwrap();

    let frame = Arc::new(AtomicI32::new(0));
    let queries = Arc::new(AtomicI32::new(0));
    let (source_frame, source_queries) = (Arc::clone(&frame), Arc::clone(&queries));
    core.set_zapper_position_source(Some(Box::new(move |port| {
        assert_eq!(1, port); // Only the Zapper port is queried.
        source_queries.fetch_add(1, Ordering::Relaxed);
        let frame = source_frame.load(Ordering::Relaxed);
        ZapperPosition::new(frame, frame + 1)
    })));

    for current in 0..5 {
        frame.store(current, Ordering::Relaxed);
        core.run_frame(0);
        assert_eq!(
            Ok(ZapperPosition::new(current, current + 1)),
            core.bus().input().zapper_position(1)
        );
    }

    assert_eq!(5, queries.load(Ordering::Relaxed));
}
