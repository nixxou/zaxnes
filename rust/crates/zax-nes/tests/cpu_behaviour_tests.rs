//! Per-instruction behaviour of the 6502: results, flags, stack and interrupts
//! (REQ-CORE-110, REQ-CORE-111).

mod common;

use common::TestBus;
use zax_nes::cpu::Cpu6502;

const C: u8 = Cpu6502::FLAG_C;
const Z: u8 = Cpu6502::FLAG_Z;
const I: u8 = Cpu6502::FLAG_I;
const D: u8 = Cpu6502::FLAG_D;
const V: u8 = Cpu6502::FLAG_V;
const N: u8 = Cpu6502::FLAG_N;

/// Runs a two-byte immediate instruction and checks A and C, Z, V, N.
#[allow(
    clippy::fn_params_excessive_bools,
    clippy::too_many_arguments,
    clippy::many_single_char_names
)] // Mirrors the C# theory row.
fn check_alu(
    opcode: u8,
    a: u8,
    operand: u8,
    carry_in: bool,
    result: u8,
    c: bool,
    z: bool,
    v: bool,
    n: bool,
) {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[opcode, operand]);

    let mut cpu = bus.cpu();
    cpu.a = a;
    cpu.set_flag(C, carry_in);

    cpu.step(&mut bus);

    let case = format!("{opcode:02X} a={a:02X} op={operand:02X} c={carry_in}");
    assert_eq!(result, cpu.a, "{case}");
    assert_eq!(c, cpu.get_flag(C), "{case}");
    assert_eq!(z, cpu.get_flag(Z), "{case}");
    assert_eq!(v, cpu.get_flag(V), "{case}");
    assert_eq!(n, cpu.get_flag(N), "{case}");
}

#[test]
fn req_core_110_adc_sets_carry_zero_overflow_and_negative() {
    // a, operand, carry in → result, C, Z, V, N (nesdev, "CPU ALU").
    let cases = [
        (0x00, 0x00, false, 0x00, false, true, false, false),
        (0x01, 0x01, false, 0x02, false, false, false, false),
        (0xFF, 0x01, false, 0x00, true, true, false, false),
        (0xFF, 0x01, true, 0x01, true, false, false, false),
        (0x7F, 0x01, false, 0x80, false, false, true, true), // Signed overflow.
        (0x80, 0xFF, false, 0x7F, true, false, true, false), // Signed underflow.
        (0x50, 0x50, false, 0xA0, false, false, true, true),
        (0x50, 0x90, false, 0xE0, false, false, false, true),
    ];
    for (a, operand, carry_in, result, c, z, v, n) in cases {
        check_alu(0x69, a, operand, carry_in, result, c, z, v, n); // ADC #operand
    }
}

#[test]
fn req_core_110_sbc_is_adc_of_the_complement() {
    let cases = [
        (0x05, 0x03, true, 0x02, true, false, false, false),
        (0x05, 0x05, true, 0x00, true, true, false, false),
        (0x03, 0x05, true, 0xFE, false, false, false, true),
        (0x80, 0x01, true, 0x7F, true, false, true, false),
        (0x7F, 0xFF, true, 0x80, false, false, true, true),
        (0x05, 0x03, false, 0x01, true, false, false, false), // Borrow in.
    ];
    for (a, operand, carry_in, result, c, z, v, n) in cases {
        check_alu(0xE9, a, operand, carry_in, result, c, z, v, n); // SBC #operand
    }
}

#[test]
fn req_core_110_decimal_mode_has_no_effect_on_the_2a03() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x69, 0x01]); // ADC #$01

    let mut cpu = bus.cpu();
    cpu.a = 0x09;
    cpu.set_flag(D, true);

    cpu.step(&mut bus);

    // Binary result, not the BCD 0x10.
    assert_eq!(0x0A, cpu.a);
    assert!(cpu.get_flag(D));
}

#[test]
fn req_core_110_cmp_sets_carry_zero_and_negative() {
    for (a, operand, c, z, n) in [
        (0x05, 0x03, true, false, false),
        (0x03, 0x03, true, true, false),
        (0x02, 0x03, false, false, true),
    ] {
        let mut bus = TestBus::new();
        bus.load(0x1000, &[0xC9, operand]); // CMP #operand

        let mut cpu = bus.cpu();
        cpu.a = a;

        cpu.step(&mut bus);

        assert_eq!(a, cpu.a); // CMP never writes A.
        assert_eq!(c, cpu.get_flag(C));
        assert_eq!(z, cpu.get_flag(Z));
        assert_eq!(n, cpu.get_flag(N));
    }
}

/// Runs a one-byte accumulator instruction and checks A and the carry.
fn check_accumulator(opcode: u8, a: u8, carry_in: bool, result: u8, carry_out: bool) {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[opcode]);

    let mut cpu = bus.cpu();
    cpu.a = a;
    cpu.set_flag(C, carry_in);

    cpu.step(&mut bus);

    assert_eq!(result, cpu.a, "{opcode:02X} a={a:02X}");
    assert_eq!(carry_out, cpu.get_flag(C), "{opcode:02X} a={a:02X}");
}

#[test]
fn req_core_110_asl_shifts_left_through_the_carry() {
    check_accumulator(0x0A, 0x81, false, 0x02, true); // ASL A.
    check_accumulator(0x0A, 0x40, false, 0x80, false);
}

#[test]
fn req_core_110_lsr_shifts_right_through_the_carry() {
    check_accumulator(0x4A, 0x81, false, 0x40, true); // LSR A.
    check_accumulator(0x4A, 0x02, true, 0x01, false);
}

#[test]
fn req_core_110_rotates_pass_through_the_carry() {
    check_accumulator(0x2A, 0x81, false, 0x02, true); // ROL A: bit 7 out, carry into bit 0.
    check_accumulator(0x2A, 0x81, true, 0x03, true);
    check_accumulator(0x6A, 0x81, false, 0x40, true); // ROR A: bit 0 out, carry into bit 7.
    check_accumulator(0x6A, 0x81, true, 0xC0, true);
}

#[test]
fn req_core_110_bit_takes_n_and_v_from_the_operand() {
    for (a, operand, z, n, v) in [
        (0xFF, 0x0F, false, false, false),
        (0x00, 0xC0, true, true, true),
        (0x01, 0x81, false, true, false),
    ] {
        let mut bus = TestBus::new();
        bus.load(0x1000, &[0x24, 0x10]); // BIT $10
        bus[0x10] = operand;

        let mut cpu = bus.cpu();
        cpu.a = a;

        cpu.step(&mut bus);

        assert_eq!(a, cpu.a);
        assert_eq!(z, cpu.get_flag(Z));
        assert_eq!(n, cpu.get_flag(N));
        assert_eq!(v, cpu.get_flag(V));
    }
}

#[test]
fn req_core_110_txs_touches_no_flag() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x9A]); // TXS

    let mut cpu = bus.cpu();
    cpu.x = 0x00;
    let before = cpu.p();

    cpu.step(&mut bus);

    assert_eq!(0x00, cpu.s);
    assert_eq!(before, cpu.p()); // Z would be set if TXS set flags.
}

#[test]
fn req_core_110_php_pushes_b_and_unused_set_plp_ignores_them() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x08]); // PHP

    let mut cpu = bus.cpu();
    cpu.set_p(0x00);

    cpu.step(&mut bus);

    // The pushed copy has bit 4 (B) and bit 5 (unused) set; the register itself reads
    // back with bit 5 set and bit 4 clear.
    assert_eq!(0x30, bus[0x01FD]);
    assert_eq!(0x20, cpu.p());

    bus.load(0x1001, &[0x28]); // PLP
    cpu.step(&mut bus);
    assert_eq!(0x20, cpu.p());
}

#[test]
fn req_core_110_jsr_pushes_the_return_address_minus_one() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x20, 0x34, 0x12]); // JSR $1234

    let mut cpu = bus.cpu();
    cpu.step(&mut bus);

    assert_eq!(0x1234, cpu.pc);
    assert_eq!(0xFB, cpu.s);

    // $1002 is pushed: the address of the last byte of the JSR.
    assert_eq!(0x10, bus[0x01FD]);
    assert_eq!(0x02, bus[0x01FC]);
}

#[test]
fn req_core_110_rts_returns_to_the_pushed_address_plus_one() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x20, 0x34, 0x12]); // JSR $1234
    bus.load(0x1234, &[0x60]); // RTS

    let mut cpu = bus.cpu();
    cpu.step(&mut bus);
    cpu.step(&mut bus);

    assert_eq!(0x1003, cpu.pc);
    assert_eq!(0xFD, cpu.s);
}

#[test]
fn req_core_111_brk_pushes_pc_plus_two_and_sets_the_interrupt_disable() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x00, 0xEA]); // BRK, then its signature byte.
    bus[0xFFFE] = 0x00;
    bus[0xFFFF] = 0x90; // IRQ vector → $9000.

    let mut cpu = bus.cpu();
    cpu.set_p(0x00);

    assert_eq!(7, cpu.step(&mut bus));
    assert_eq!(0x9000, cpu.pc);
    assert!(cpu.get_flag(I));

    // $1002 pushed, then P with the B flag set.
    assert_eq!(0x10, bus[0x01FD]);
    assert_eq!(0x02, bus[0x01FC]);
    assert_eq!(0x30, bus[0x01FB]);
}

#[test]
fn req_core_111_rti_restores_the_flags_and_the_return_address() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x40]); // RTI
    bus[0x01FB] = 0xC1; // Pushed P.
    bus[0x01FC] = 0x34; // PCL.
    bus[0x01FD] = 0x12; // PCH.

    let mut cpu = bus.create_cpu(0x1000, 0xFA);

    assert_eq!(6, cpu.step(&mut bus));
    assert_eq!(0x1234, cpu.pc);
    assert_eq!(0xE1, cpu.p()); // Bit 4 dropped, bit 5 forced.
    assert_eq!(0xFD, cpu.s);
}

#[test]
fn req_core_111_nmi_is_taken_at_the_next_instruction_boundary() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xEA, 0xEA]); // NOP, NOP
    bus[0xFFFA] = 0x00;
    bus[0xFFFB] = 0x90; // NMI vector → $9000.

    let mut cpu = bus.cpu();
    cpu.set_flag(I, true); // NMI ignores the interrupt disable.

    bus.nmi_asserted = true;
    cpu.step(&mut bus); // The NOP finishes; the edge is latched during it.

    assert_eq!(0x1001, cpu.pc);

    assert_eq!(7, cpu.step(&mut bus)); // Now the handler is entered.
    assert_eq!(0x9000, cpu.pc);

    // The pushed status has bit 4 clear: this was not a BRK.
    assert_eq!(0x00, bus[0x01FB] & 0x10);
}

#[test]
fn req_core_111_nmi_is_edge_triggered_and_fires_only_once() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xEA; 8]); // NOP sled.
    bus[0x9000] = 0x40; // RTI in the handler.
    bus[0xFFFA] = 0x00;
    bus[0xFFFB] = 0x90;

    let mut cpu = bus.cpu();
    bus.nmi_asserted = true;

    cpu.step(&mut bus); // NOP, latches the edge.
    cpu.step(&mut bus); // Handler entry.
    assert_eq!(0x9000, cpu.pc);

    cpu.step(&mut bus); // RTI back to the sled.
    assert_eq!(0x1001, cpu.pc);

    // The line is still low, but there is no new falling edge, so no second NMI.
    cpu.step(&mut bus);
    assert_eq!(0x1002, cpu.pc);
}

#[test]
fn req_core_111_irq_is_blocked_while_the_interrupt_disable_is_set() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xEA, 0xEA, 0xEA]);
    bus[0xFFFE] = 0x00;
    bus[0xFFFF] = 0x90;

    let mut cpu = bus.cpu();
    cpu.set_flag(I, true);
    bus.irq_asserted = true;

    cpu.step(&mut bus);
    cpu.step(&mut bus);

    assert_eq!(0x1002, cpu.pc); // Still running the sled.
}

#[test]
fn req_core_111_cli_delays_irq_recognition_by_one_instruction() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x58, 0xEA, 0xEA]); // CLI, NOP, NOP
    bus[0xFFFE] = 0x00;
    bus[0xFFFF] = 0x90;

    let mut cpu = bus.cpu();
    cpu.set_flag(I, true);
    bus.irq_asserted = true;

    cpu.step(&mut bus); // CLI: the IRQ is not recognised at this boundary.
    assert_eq!(0x1001, cpu.pc);

    cpu.step(&mut bus); // The NOP runs, and the IRQ is latched during it.
    assert_eq!(0x1002, cpu.pc);

    assert_eq!(7, cpu.step(&mut bus)); // Handler entry.
    assert_eq!(0x9000, cpu.pc);
}

#[test]
fn req_core_004_reset_takes_seven_cycles_and_leaves_sp_at_fd() {
    let mut bus = TestBus::new();
    bus[0xFFFC] = 0x00;
    bus[0xFFFD] = 0xC0; // Reset vector → $C000.

    let mut cpu = Cpu6502::new();
    cpu.reset(&mut bus);

    assert_eq!(7, bus.cycle_count);
    assert_eq!(0xC000, cpu.pc);
    assert_eq!(0xFD, cpu.s);
    assert_eq!(0x24, cpu.p()); // Interrupt disable plus the unused bit.
    assert_eq!(0x00, cpu.a);
    assert_eq!(0x00, cpu.x);
    assert_eq!(0x00, cpu.y);
}

// ------------------------------------------------------- unofficial opcodes

#[test]
fn req_core_110_lax_loads_a_and_x_together() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xA7, 0x10]); // *LAX $10
    bus[0x10] = 0x81;

    let mut cpu = bus.cpu();
    cpu.step(&mut bus);

    assert_eq!(0x81, cpu.a);
    assert_eq!(0x81, cpu.x);
    assert!(cpu.get_flag(N));
}

#[test]
fn req_core_110_sax_stores_a_and_x_without_touching_flags() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x87, 0x10]); // *SAX $10

    let mut cpu = bus.cpu();
    cpu.a = 0xF0;
    cpu.x = 0x3C;
    let before = cpu.p();

    cpu.step(&mut bus);

    assert_eq!(0x30, bus[0x10]);
    assert_eq!(before, cpu.p());
}

#[test]
fn req_core_110_slo_shifts_memory_then_ors_into_a() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x07, 0x10]); // *SLO $10
    bus[0x10] = 0x81;

    let mut cpu = bus.cpu();
    cpu.a = 0x10;

    cpu.step(&mut bus);

    assert_eq!(0x02, bus[0x10]); // $81 << 1.
    assert_eq!(0x12, cpu.a); // $10 | $02.
    assert!(cpu.get_flag(C)); // Bit 7 shifted out.
}

#[test]
fn req_core_110_rla_rotates_memory_then_ands_into_a() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x27, 0x10]); // *RLA $10
    bus[0x10] = 0x81;

    let mut cpu = bus.cpu();
    cpu.a = 0xFF;
    cpu.set_flag(C, true);

    cpu.step(&mut bus);

    assert_eq!(0x03, bus[0x10]); // ($81 << 1) | carry.
    assert_eq!(0x03, cpu.a);
    assert!(cpu.get_flag(C));
}

#[test]
fn req_core_110_sre_shifts_memory_then_eors_into_a() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x47, 0x10]); // *SRE $10
    bus[0x10] = 0x03;

    let mut cpu = bus.cpu();
    cpu.a = 0xFF;

    cpu.step(&mut bus);

    assert_eq!(0x01, bus[0x10]);
    assert_eq!(0xFE, cpu.a);
    assert!(cpu.get_flag(C));
}

#[test]
fn req_core_110_rra_rotates_memory_then_adds_it_to_a() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x67, 0x10]); // *RRA $10
    bus[0x10] = 0x02;

    let mut cpu = bus.cpu();
    cpu.a = 0x10;
    cpu.set_flag(C, false);

    cpu.step(&mut bus);

    assert_eq!(0x01, bus[0x10]); // $02 >> 1, carry in was 0.
    assert_eq!(0x11, cpu.a); // The ROR cleared the carry, so A + 1 + 0.
}

#[test]
fn req_core_110_dcp_decrements_memory_then_compares_it_with_a() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xC7, 0x10]); // *DCP $10
    bus[0x10] = 0x43;

    let mut cpu = bus.cpu();
    cpu.a = 0x42;

    cpu.step(&mut bus);

    assert_eq!(0x42, bus[0x10]);
    assert_eq!(0x42, cpu.a);
    assert!(cpu.get_flag(Z)); // A equals the decremented value.
    assert!(cpu.get_flag(C));
}

#[test]
fn req_core_110_isb_increments_memory_then_subtracts_it_from_a() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xE7, 0x10]); // *ISB $10
    bus[0x10] = 0x01;

    let mut cpu = bus.cpu();
    cpu.a = 0x10;
    cpu.set_flag(C, true);

    cpu.step(&mut bus);

    assert_eq!(0x02, bus[0x10]);
    assert_eq!(0x0E, cpu.a); // $10 - $02.
}

#[test]
fn req_core_110_unofficial_sbc_eb_behaves_like_the_official_one() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xEB, 0x03]); // *SBC #$03

    let mut cpu = bus.cpu();
    cpu.a = 0x05;
    cpu.set_flag(C, true);

    cpu.step(&mut bus);

    assert_eq!(0x02, cpu.a);
    assert!(cpu.get_flag(C));
}

#[test]
fn req_core_110_anc_copies_bit_seven_of_the_result_into_the_carry() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x0B, 0xFF]); // *ANC #$FF

    let mut cpu = bus.cpu();
    cpu.a = 0x80;

    cpu.step(&mut bus);

    assert_eq!(0x80, cpu.a);
    assert!(cpu.get_flag(C));
    assert!(cpu.get_flag(N));
}

#[test]
fn req_core_110_alr_ands_then_shifts_right() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x4B, 0x03]); // *ALR #$03

    let mut cpu = bus.cpu();
    cpu.a = 0xFF;

    cpu.step(&mut bus);

    assert_eq!(0x01, cpu.a); // ($FF & $03) >> 1.
    assert!(cpu.get_flag(C)); // Bit 0 of the AND result.
}

#[test]
fn req_core_110_sbx_subtracts_from_a_and_x() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xCB, 0x02]); // *SBX #$02

    let mut cpu = bus.cpu();
    cpu.a = 0x0F;
    cpu.x = 0x07;

    cpu.step(&mut bus);

    assert_eq!(0x05, cpu.x); // ($0F & $07) - $02.
    assert_eq!(0x0F, cpu.a);
    assert!(cpu.get_flag(C));
}

#[test]
fn req_core_110_the_stack_wraps_inside_page_one() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x48, 0x48]); // PHA, PHA

    let mut cpu = bus.create_cpu(0x1000, 0x00);
    cpu.a = 0xAA;

    cpu.step(&mut bus);
    assert_eq!(0xAA, bus[0x0100]);
    assert_eq!(0xFF, cpu.s); // Wrapped, not $00FF.

    cpu.a = 0xBB;
    cpu.step(&mut bus);
    assert_eq!(0xBB, bus[0x01FF]);
    assert_eq!(0xFE, cpu.s);

    // Nothing was written outside page one.
    assert_eq!(0x00, bus[0x00FF]);
    assert_eq!(0x00, bus[0x0200]);
}

#[test]
fn req_core_110_pulling_past_the_top_of_the_stack_wraps_too() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x68]); // PLA
    bus[0x0100] = 0x5C;

    let mut cpu = bus.create_cpu(0x1000, 0xFF);

    cpu.step(&mut bus);

    assert_eq!(0x5C, cpu.a);
    assert_eq!(0x00, cpu.s);
}

#[test]
fn req_core_110_nop_variants_still_consume_their_operand() {
    // *NOP $xxxx,X must advance PC by three and leave every register alone.
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x1C, 0x10, 0x20]); // *NOP $2010,X

    let mut cpu = bus.cpu();
    cpu.a = 0x11;
    cpu.x = 0x22;
    cpu.y = 0x33;
    let flags = cpu.p();

    cpu.step(&mut bus);

    assert_eq!(0x1003, cpu.pc);
    assert_eq!(0x11, cpu.a);
    assert_eq!(0x22, cpu.x);
    assert_eq!(0x33, cpu.y);
    assert_eq!(flags, cpu.p());
}
