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

/// A taken branch that crosses a page polls twice — before its second cycle and before its
/// fourth — and the two polls are a **set**, not an overwrite: an IRQ the first one saw is
/// still serviced when the line has gone back up before the second.
///
/// `AccuracyCoin`'s `Interrupt flag latency` test E is this experiment on hardware, with the
/// acknowledgement made by a dummy read of `$4015` inside the branch: "An interrupt polled
/// successfully on the first poll of a branch, cleared, and then polled again, should still
/// occur" (REQ-CORE-111).
#[test]
fn req_core_111_a_page_crossing_branch_keeps_what_its_first_poll_found() {
    /// `BEQ -$10` at $10FE: PC is $1100 after the operand, the target is $10F0, so the branch
    /// is taken and crosses a page — four cycles, the fourth being the PCH fixup. `window` is
    /// the half-open range of branch cycles on which the IRQ line is low; the handler entry is
    /// then run, and the PC it reaches says whether the IRQ was recognised.
    fn after(window: (i64, i64), opcodes: &[u8]) -> u16 {
        let mut bus = TestBus::new();
        bus.load(0x10F0, &[0xEA; 8]); // A NOP sled at either target, so that a boundary with
        bus.load(0x1110, &[0xEA; 8]); // no interrupt is told apart from the handler entry.
        bus.load(0x10FE, opcodes);
        bus[0xFFFE] = 0x00;
        bus[0xFFFF] = 0x90;

        let mut cpu = bus.create_cpu(0x10FE, 0xFD);
        cpu.set_flag(Z, true);
        cpu.set_flag(I, false);
        bus.irq_asserted = true;
        bus.irq_window = Some(window);

        cpu.step(&mut bus);
        bus.irq_asserted = false;
        cpu.step(&mut bus);
        cpu.pc
    }

    const CROSSES: &[u8] = &[0xF0, 0xF0]; // BEQ -$10 at $10FE → $10F0, four cycles.
    const STAYS: &[u8] = &[0xF0, 0x10]; // BEQ +$10 at $10FE → $1110, three cycles.

    // The line is low for cycle 1 only: the first poll sees it, the second does not, and the
    // handler still runs. This is the case the lot adds, and `Interrupt flag latency` test E.
    assert_eq!(0x9000, after((1, 2), CROSSES));

    // Low for cycles 1 to 3: both polls see it. Unchanged.
    assert_eq!(0x9000, after((1, 4), CROSSES));

    // Low only for cycle 3, after the first poll: the second poll catches it, which is what
    // `cpu_interrupts_v2` part 5 measures of a page-crossing branch, and the handler runs
    // instead of the instruction after the branch ($10F1 would be the NOP sled).
    assert_eq!(0x9000, after((3, 4), CROSSES));

    // A taken branch that stays in its page has only the first poll, so a line that goes low
    // during its second cycle is not seen at all: the next instruction runs first. That is the
    // delay `5-branch_delays_irq` pins, and the sticky poll must not undo it.
    assert_eq!(0x1111, after((2, 4), STAYS));
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

/// The five `SH*` stores AND the register with the high byte of the un-indexed address plus one —
/// **unless** a DMA held the CPU on the dummy read cycle right before the write, in which case the
/// register is stored whole. The address still takes the AND-ed high byte on a page cross.
///
/// nesdev's "Programming with unofficial opcodes" records the instability; `AccuracyCoin`'s `SH*`
/// error codes 7 to C measure this case ("If the RDY line goes low 2 cycles before the write
/// cycle, the target address of the instruction was not correct") and the five rows pass with it.
/// The scan below pins *which* cycle it is rather than asserting a constant: only a halt on the
/// cycle the write follows drops the AND (REQ-CORE-110).
#[test]
fn req_core_110_a_dma_on_the_dummy_read_drops_the_and_of_the_sh_stores() {
    /// Runs one `SH* $0568,X/Y` at $1000, with the register the opcode uses as its source set to
    /// `$8F` and the index register to 0, and the bus claiming a DMA gave the CPU its read back on
    /// cycle `halt_on`. Returns the address written and the byte written.
    fn store(opcode: u8, halt_on: i64) -> (u16, u8) {
        let mut bus = TestBus::new();
        bus.load(0x1000, &[opcode, 0x68, 0x05]);
        let mut cpu = bus.cpu();
        match opcode {
            // SHY abs,X: the source is Y and the index is X.
            0x9C => {
                cpu.y = 0x8F;
                cpu.x = 0x00;
            }
            // SHX abs,Y: the source is X and the index is Y.
            0x9E => {
                cpu.x = 0x8F;
                cpu.y = 0x00;
            }
            // SHA and SHS abs,Y: the source is A & X and the index is Y.
            _ => {
                cpu.a = 0x8F;
                cpu.x = 0xFF;
                cpu.y = 0x00;
            }
        }
        bus.dma_gave_back = halt_on;
        cpu.step(&mut bus);
        assert_eq!(5, bus.cycle_count, "absolute-indexed write: five cycles");
        let address = *bus.access_log.last().unwrap();
        (address, bus[usize::from(address)])
    }

    /// The same with a page cross: the high byte of the target is replaced by the AND-ed value
    /// whether or not the DMA happened, and only the byte written changes.
    fn crossed(halt_on: i64) -> (u16, u8) {
        let mut bus = TestBus::new();
        bus.load(0x1000, &[0x9E, 0xF0, 0x1F]); // SHX $1FF0,Y with Y = $20 → $2010, page crossed.
        let mut cpu = bus.cpu();
        cpu.x = 0x0A;
        cpu.y = 0x20;
        bus.dma_gave_back = halt_on;
        cpu.step(&mut bus);
        let address = *bus.access_log.last().unwrap();
        (address, bus[usize::from(address)])
    }

    // SHA abs,Y ($9F) stores A & X = $8F, AND-ed with $05 + 1 = $06.
    assert_eq!((0x0568, 0x06), store(0x9F, i64::MIN), "no DMA at all");
    for halt_on in [1, 2, 3, 5] {
        assert_eq!(
            (0x0568, 0x06),
            store(0x9F, halt_on),
            "a halt on cycle {halt_on} is not the dummy read"
        );
    }
    // Cycle 4 is the dummy read, the one the write follows.
    assert_eq!((0x0568, 0x8F), store(0x9F, 4));

    // The other three absolute forms, each with its own source register.
    for opcode in [0x9E, 0x9C, 0x9B] {
        assert_eq!((0x0568, 0x06), store(opcode, i64::MIN), "{opcode:02X}");
        assert_eq!((0x0568, 0x8F), store(opcode, 4), "{opcode:02X}");
    }

    // X & ($1F + 1) = $0A & $20 = $00, so the high byte becomes $00.
    assert_eq!((0x0010, 0x00), crossed(i64::MIN));
    assert_eq!((0x0010, 0x0A), crossed(4));
}
