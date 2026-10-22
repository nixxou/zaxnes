using Zax.Nes.Cpu;

namespace Zax.Nes.Tests;

/// <summary>
/// Per-instruction behaviour of the 6502: results, flags, stack and interrupts
/// (REQ-CORE-110, REQ-CORE-111).
/// </summary>
public sealed class CpuBehaviourTests
{
    [Theory]
    // a, operand, carry in → result, C, Z, V, N (nesdev, "CPU ALU").
    [InlineData(0x00, 0x00, false, 0x00, false, true, false, false)]
    [InlineData(0x01, 0x01, false, 0x02, false, false, false, false)]
    [InlineData(0xFF, 0x01, false, 0x00, true, true, false, false)]
    [InlineData(0xFF, 0x01, true, 0x01, true, false, false, false)]
    [InlineData(0x7F, 0x01, false, 0x80, false, false, true, true)] // Signed overflow.
    [InlineData(0x80, 0xFF, false, 0x7F, true, false, true, false)] // Signed underflow.
    [InlineData(0x50, 0x50, false, 0xA0, false, false, true, true)]
    [InlineData(0x50, 0x90, false, 0xE0, false, false, false, true)]
    public void REQ_CORE_110_adc_sets_carry_zero_overflow_and_negative(
        byte a, byte operand, bool carryIn, byte result, bool c, bool z, bool v, bool n)
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x69, operand); // ADC #operand

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = a;
        cpu.SetFlag(Cpu6502.FlagC, carryIn);

        cpu.Step();

        Assert.Equal(result, cpu.A);
        Assert.Equal(c, cpu.GetFlag(Cpu6502.FlagC));
        Assert.Equal(z, cpu.GetFlag(Cpu6502.FlagZ));
        Assert.Equal(v, cpu.GetFlag(Cpu6502.FlagV));
        Assert.Equal(n, cpu.GetFlag(Cpu6502.FlagN));
    }

    [Theory]
    [InlineData(0x05, 0x03, true, 0x02, true, false, false, false)]
    [InlineData(0x05, 0x05, true, 0x00, true, true, false, false)]
    [InlineData(0x03, 0x05, true, 0xFE, false, false, false, true)]
    [InlineData(0x80, 0x01, true, 0x7F, true, false, true, false)]
    [InlineData(0x7F, 0xFF, true, 0x80, false, false, true, true)]
    [InlineData(0x05, 0x03, false, 0x01, true, false, false, false)] // Borrow in.
    public void REQ_CORE_110_sbc_is_adc_of_the_complement(
        byte a, byte operand, bool carryIn, byte result, bool c, bool z, bool v, bool n)
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0xE9, operand); // SBC #operand

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = a;
        cpu.SetFlag(Cpu6502.FlagC, carryIn);

        cpu.Step();

        Assert.Equal(result, cpu.A);
        Assert.Equal(c, cpu.GetFlag(Cpu6502.FlagC));
        Assert.Equal(z, cpu.GetFlag(Cpu6502.FlagZ));
        Assert.Equal(v, cpu.GetFlag(Cpu6502.FlagV));
        Assert.Equal(n, cpu.GetFlag(Cpu6502.FlagN));
    }

    [Fact]
    public void REQ_CORE_110_decimal_mode_has_no_effect_on_the_2a03()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x69, 0x01); // ADC #$01

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = 0x09;
        cpu.SetFlag(Cpu6502.FlagD, true);

        cpu.Step();

        // Binary result, not the BCD 0x10.
        Assert.Equal(0x0A, cpu.A);
        Assert.True(cpu.GetFlag(Cpu6502.FlagD));
    }

    [Theory]
    [InlineData(0x05, 0x03, true, false, false)]
    [InlineData(0x03, 0x03, true, true, false)]
    [InlineData(0x02, 0x03, false, false, true)]
    public void REQ_CORE_110_cmp_sets_carry_zero_and_negative(
        byte a, byte operand, bool c, bool z, bool n)
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0xC9, operand); // CMP #operand

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = a;

        cpu.Step();

        Assert.Equal(a, cpu.A); // CMP never writes A.
        Assert.Equal(c, cpu.GetFlag(Cpu6502.FlagC));
        Assert.Equal(z, cpu.GetFlag(Cpu6502.FlagZ));
        Assert.Equal(n, cpu.GetFlag(Cpu6502.FlagN));
    }

    [Theory]
    [InlineData(0x81, false, 0x02, true)] // ASL A.
    [InlineData(0x40, false, 0x80, false)]
    public void REQ_CORE_110_asl_shifts_left_through_the_carry(byte a, bool carryIn, byte result, bool carryOut)
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x0A); // ASL A

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = a;
        cpu.SetFlag(Cpu6502.FlagC, carryIn);

        cpu.Step();

        Assert.Equal(result, cpu.A);
        Assert.Equal(carryOut, cpu.GetFlag(Cpu6502.FlagC));
    }

    [Theory]
    [InlineData(0x81, false, 0x40, true)] // LSR A.
    [InlineData(0x02, true, 0x01, false)]
    public void REQ_CORE_110_lsr_shifts_right_through_the_carry(byte a, bool carryIn, byte result, bool carryOut)
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x4A); // LSR A

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = a;
        cpu.SetFlag(Cpu6502.FlagC, carryIn);

        cpu.Step();

        Assert.Equal(result, cpu.A);
        Assert.Equal(carryOut, cpu.GetFlag(Cpu6502.FlagC));
    }

    [Theory]
    [InlineData(0x2A, 0x81, false, 0x02, true)] // ROL A: bit 7 out, carry into bit 0.
    [InlineData(0x2A, 0x81, true, 0x03, true)]
    [InlineData(0x6A, 0x81, false, 0x40, true)] // ROR A: bit 0 out, carry into bit 7.
    [InlineData(0x6A, 0x81, true, 0xC0, true)]
    public void REQ_CORE_110_rotates_pass_through_the_carry(
        byte opcode, byte a, bool carryIn, byte result, bool carryOut)
    {
        var bus = new TestBus();
        bus.Load(0x1000, opcode);

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = a;
        cpu.SetFlag(Cpu6502.FlagC, carryIn);

        cpu.Step();

        Assert.Equal(result, cpu.A);
        Assert.Equal(carryOut, cpu.GetFlag(Cpu6502.FlagC));
    }

    [Theory]
    [InlineData(0xFF, 0x0F, false, false, false)] // BIT: N and V come from the operand.
    [InlineData(0x00, 0xC0, true, true, true)]
    [InlineData(0x01, 0x81, false, true, false)]
    public void REQ_CORE_110_bit_takes_n_and_v_from_the_operand(
        byte a, byte operand, bool z, bool n, bool v)
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x24, 0x10); // BIT $10
        bus[0x10] = operand;

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = a;

        cpu.Step();

        Assert.Equal(a, cpu.A);
        Assert.Equal(z, cpu.GetFlag(Cpu6502.FlagZ));
        Assert.Equal(n, cpu.GetFlag(Cpu6502.FlagN));
        Assert.Equal(v, cpu.GetFlag(Cpu6502.FlagV));
    }

    [Fact]
    public void REQ_CORE_110_txs_touches_no_flag()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x9A); // TXS

        Cpu6502 cpu = bus.CreateCpu();
        cpu.X = 0x00;
        byte before = cpu.P;

        cpu.Step();

        Assert.Equal(0x00, cpu.S);
        Assert.Equal(before, cpu.P); // Z would be set if TXS set flags.
    }

    [Fact]
    public void REQ_CORE_110_php_pushes_b_and_unused_set_plp_ignores_them()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x08); // PHP

        Cpu6502 cpu = bus.CreateCpu();
        cpu.P = 0x00;

        cpu.Step();

        // The pushed copy has bit 4 (B) and bit 5 (unused) set; the register itself reads
        // back with bit 5 set and bit 4 clear.
        Assert.Equal(0x30, bus[0x01FD]);
        Assert.Equal(0x20, cpu.P);

        bus.Load(0x1001, 0x28); // PLP
        cpu.Step();
        Assert.Equal(0x20, cpu.P);
    }

    [Fact]
    public void REQ_CORE_110_jsr_pushes_the_return_address_minus_one()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x20, 0x34, 0x12); // JSR $1234

        Cpu6502 cpu = bus.CreateCpu();
        cpu.Step();

        Assert.Equal(0x1234, cpu.PC);
        Assert.Equal(0xFB, cpu.S);

        // $1002 is pushed: the address of the last byte of the JSR.
        Assert.Equal(0x10, bus[0x01FD]);
        Assert.Equal(0x02, bus[0x01FC]);
    }

    [Fact]
    public void REQ_CORE_110_rts_returns_to_the_pushed_address_plus_one()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x20, 0x34, 0x12); // JSR $1234
        bus.Load(0x1234, 0x60); // RTS

        Cpu6502 cpu = bus.CreateCpu();
        cpu.Step();
        cpu.Step();

        Assert.Equal(0x1003, cpu.PC);
        Assert.Equal(0xFD, cpu.S);
    }

    [Fact]
    public void REQ_CORE_111_brk_pushes_pc_plus_two_and_sets_the_interrupt_disable()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x00, 0xEA); // BRK, then its signature byte.
        bus[0xFFFE] = 0x00;
        bus[0xFFFF] = 0x90; // IRQ vector → $9000.

        Cpu6502 cpu = bus.CreateCpu();
        cpu.P = 0x00;

        Assert.Equal(7, cpu.Step());
        Assert.Equal(0x9000, cpu.PC);
        Assert.True(cpu.GetFlag(Cpu6502.FlagI));

        // $1002 pushed, then P with the B flag set.
        Assert.Equal(0x10, bus[0x01FD]);
        Assert.Equal(0x02, bus[0x01FC]);
        Assert.Equal(0x30, bus[0x01FB]);
    }

    [Fact]
    public void REQ_CORE_111_rti_restores_the_flags_and_the_return_address()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x40); // RTI
        bus[0x01FB] = 0xC1; // Pushed P.
        bus[0x01FC] = 0x34; // PCL.
        bus[0x01FD] = 0x12; // PCH.

        Cpu6502 cpu = bus.CreateCpu(0x1000, stackPointer: 0xFA);

        Assert.Equal(6, cpu.Step());
        Assert.Equal(0x1234, cpu.PC);
        Assert.Equal(0xE1, cpu.P); // Bit 4 dropped, bit 5 forced.
        Assert.Equal(0xFD, cpu.S);
    }

    [Fact]
    public void REQ_CORE_111_nmi_is_taken_at_the_next_instruction_boundary()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0xEA, 0xEA); // NOP, NOP
        bus[0xFFFA] = 0x00;
        bus[0xFFFB] = 0x90; // NMI vector → $9000.

        Cpu6502 cpu = bus.CreateCpu();
        cpu.SetFlag(Cpu6502.FlagI, true); // NMI ignores the interrupt disable.

        bus.NmiAsserted = true;
        cpu.Step(); // The NOP finishes; the edge is latched during it.

        Assert.Equal(0x1001, cpu.PC);

        Assert.Equal(7, cpu.Step()); // Now the handler is entered.
        Assert.Equal(0x9000, cpu.PC);

        // The pushed status has bit 4 clear: this was not a BRK.
        Assert.Equal(0x00, bus[0x01FB] & 0x10);
    }

    [Fact]
    public void REQ_CORE_111_nmi_is_edge_triggered_and_fires_only_once()
    {
        var bus = new TestBus();
        for (int i = 0; i < 8; i++)
        {
            bus[0x1000 + i] = 0xEA; // NOP sled.
        }

        bus[0x9000] = 0x40; // RTI in the handler.
        bus[0xFFFA] = 0x00;
        bus[0xFFFB] = 0x90;

        Cpu6502 cpu = bus.CreateCpu();
        bus.NmiAsserted = true;

        cpu.Step(); // NOP, latches the edge.
        cpu.Step(); // Handler entry.
        Assert.Equal(0x9000, cpu.PC);

        cpu.Step(); // RTI back to the sled.
        Assert.Equal(0x1001, cpu.PC);

        // The line is still low, but there is no new falling edge, so no second NMI.
        cpu.Step();
        Assert.Equal(0x1002, cpu.PC);
    }

    [Fact]
    public void REQ_CORE_111_irq_is_blocked_while_the_interrupt_disable_is_set()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0xEA, 0xEA, 0xEA);
        bus[0xFFFE] = 0x00;
        bus[0xFFFF] = 0x90;

        Cpu6502 cpu = bus.CreateCpu();
        cpu.SetFlag(Cpu6502.FlagI, true);
        bus.IrqAsserted = true;

        cpu.Step();
        cpu.Step();

        Assert.Equal(0x1002, cpu.PC); // Still running the sled.
    }

    [Fact]
    public void REQ_CORE_111_cli_delays_irq_recognition_by_one_instruction()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x58, 0xEA, 0xEA); // CLI, NOP, NOP
        bus[0xFFFE] = 0x00;
        bus[0xFFFF] = 0x90;

        Cpu6502 cpu = bus.CreateCpu();
        cpu.SetFlag(Cpu6502.FlagI, true);
        bus.IrqAsserted = true;

        cpu.Step(); // CLI: the IRQ is not recognised at this boundary.
        Assert.Equal(0x1001, cpu.PC);

        cpu.Step(); // The NOP runs, and the IRQ is latched during it.
        Assert.Equal(0x1002, cpu.PC);

        Assert.Equal(7, cpu.Step()); // Handler entry.
        Assert.Equal(0x9000, cpu.PC);
    }

    [Fact]
    public void REQ_CORE_004_reset_takes_seven_cycles_and_leaves_sp_at_fd()
    {
        var bus = new TestBus();
        bus[0xFFFC] = 0x00;
        bus[0xFFFD] = 0xC0; // Reset vector → $C000.

        var cpu = new Cpu6502(bus);
        cpu.Reset();

        Assert.Equal(7, bus.CycleCount);
        Assert.Equal(0xC000, cpu.PC);
        Assert.Equal(0xFD, cpu.S);
        Assert.Equal(0x24, cpu.P); // Interrupt disable plus the unused bit.
        Assert.Equal(0x00, cpu.A);
        Assert.Equal(0x00, cpu.X);
        Assert.Equal(0x00, cpu.Y);
    }

    // ------------------------------------------------------- unofficial opcodes

    [Fact]
    public void REQ_CORE_110_lax_loads_a_and_x_together()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0xA7, 0x10); // *LAX $10
        bus[0x10] = 0x81;

        Cpu6502 cpu = bus.CreateCpu();
        cpu.Step();

        Assert.Equal(0x81, cpu.A);
        Assert.Equal(0x81, cpu.X);
        Assert.True(cpu.GetFlag(Cpu6502.FlagN));
    }

    [Fact]
    public void REQ_CORE_110_sax_stores_a_and_x_without_touching_flags()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x87, 0x10); // *SAX $10

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = 0xF0;
        cpu.X = 0x3C;
        byte before = cpu.P;

        cpu.Step();

        Assert.Equal(0x30, bus[0x10]);
        Assert.Equal(before, cpu.P);
    }

    [Fact]
    public void REQ_CORE_110_slo_shifts_memory_then_ors_into_a()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x07, 0x10); // *SLO $10
        bus[0x10] = 0x81;

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = 0x10;

        cpu.Step();

        Assert.Equal(0x02, bus[0x10]); // $81 << 1.
        Assert.Equal(0x12, cpu.A); // $10 | $02.
        Assert.True(cpu.GetFlag(Cpu6502.FlagC)); // Bit 7 shifted out.
    }

    [Fact]
    public void REQ_CORE_110_rla_rotates_memory_then_ands_into_a()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x27, 0x10); // *RLA $10
        bus[0x10] = 0x81;

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = 0xFF;
        cpu.SetFlag(Cpu6502.FlagC, true);

        cpu.Step();

        Assert.Equal(0x03, bus[0x10]); // ($81 << 1) | carry.
        Assert.Equal(0x03, cpu.A);
        Assert.True(cpu.GetFlag(Cpu6502.FlagC));
    }

    [Fact]
    public void REQ_CORE_110_sre_shifts_memory_then_eors_into_a()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x47, 0x10); // *SRE $10
        bus[0x10] = 0x03;

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = 0xFF;

        cpu.Step();

        Assert.Equal(0x01, bus[0x10]);
        Assert.Equal(0xFE, cpu.A);
        Assert.True(cpu.GetFlag(Cpu6502.FlagC));
    }

    [Fact]
    public void REQ_CORE_110_rra_rotates_memory_then_adds_it_to_a()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x67, 0x10); // *RRA $10
        bus[0x10] = 0x02;

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = 0x10;
        cpu.SetFlag(Cpu6502.FlagC, false);

        cpu.Step();

        Assert.Equal(0x01, bus[0x10]); // $02 >> 1, carry in was 0.
        Assert.Equal(0x11, cpu.A); // The ROR cleared the carry, so A + 1 + 0.
    }

    [Fact]
    public void REQ_CORE_110_dcp_decrements_memory_then_compares_it_with_a()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0xC7, 0x10); // *DCP $10
        bus[0x10] = 0x43;

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = 0x42;

        cpu.Step();

        Assert.Equal(0x42, bus[0x10]);
        Assert.Equal(0x42, cpu.A);
        Assert.True(cpu.GetFlag(Cpu6502.FlagZ)); // A equals the decremented value.
        Assert.True(cpu.GetFlag(Cpu6502.FlagC));
    }

    [Fact]
    public void REQ_CORE_110_isb_increments_memory_then_subtracts_it_from_a()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0xE7, 0x10); // *ISB $10
        bus[0x10] = 0x01;

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = 0x10;
        cpu.SetFlag(Cpu6502.FlagC, true);

        cpu.Step();

        Assert.Equal(0x02, bus[0x10]);
        Assert.Equal(0x0E, cpu.A); // $10 - $02.
    }

    [Fact]
    public void REQ_CORE_110_unofficial_sbc_eb_behaves_like_the_official_one()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0xEB, 0x03); // *SBC #$03

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = 0x05;
        cpu.SetFlag(Cpu6502.FlagC, true);

        cpu.Step();

        Assert.Equal(0x02, cpu.A);
        Assert.True(cpu.GetFlag(Cpu6502.FlagC));
    }

    [Fact]
    public void REQ_CORE_110_anc_copies_bit_seven_of_the_result_into_the_carry()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x0B, 0xFF); // *ANC #$FF

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = 0x80;

        cpu.Step();

        Assert.Equal(0x80, cpu.A);
        Assert.True(cpu.GetFlag(Cpu6502.FlagC));
        Assert.True(cpu.GetFlag(Cpu6502.FlagN));
    }

    [Fact]
    public void REQ_CORE_110_alr_ands_then_shifts_right()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x4B, 0x03); // *ALR #$03

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = 0xFF;

        cpu.Step();

        Assert.Equal(0x01, cpu.A); // ($FF & $03) >> 1.
        Assert.True(cpu.GetFlag(Cpu6502.FlagC)); // Bit 0 of the AND result.
    }

    [Fact]
    public void REQ_CORE_110_sbx_subtracts_from_a_and_x()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0xCB, 0x02); // *SBX #$02

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = 0x0F;
        cpu.X = 0x07;

        cpu.Step();

        Assert.Equal(0x05, cpu.X); // ($0F & $07) - $02.
        Assert.Equal(0x0F, cpu.A);
        Assert.True(cpu.GetFlag(Cpu6502.FlagC));
    }

    [Fact]
    public void REQ_CORE_110_the_stack_wraps_inside_page_one()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x48, 0x48); // PHA, PHA

        Cpu6502 cpu = bus.CreateCpu(0x1000, stackPointer: 0x00);
        cpu.A = 0xAA;

        cpu.Step();
        Assert.Equal(0xAA, bus[0x0100]);
        Assert.Equal(0xFF, cpu.S); // Wrapped, not $00FF.

        cpu.A = 0xBB;
        cpu.Step();
        Assert.Equal(0xBB, bus[0x01FF]);
        Assert.Equal(0xFE, cpu.S);

        // Nothing was written outside page one.
        Assert.Equal(0x00, bus[0x00FF]);
        Assert.Equal(0x00, bus[0x0200]);
    }

    [Fact]
    public void REQ_CORE_110_pulling_past_the_top_of_the_stack_wraps_too()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x68); // PLA
        bus[0x0100] = 0x5C;

        Cpu6502 cpu = bus.CreateCpu(0x1000, stackPointer: 0xFF);

        cpu.Step();

        Assert.Equal(0x5C, cpu.A);
        Assert.Equal(0x00, cpu.S);
    }

    [Fact]
    public void REQ_CORE_110_nop_variants_still_consume_their_operand()
    {
        // *NOP $xxxx,X must advance PC by three and leave every register alone.
        var bus = new TestBus();
        bus.Load(0x1000, 0x1C, 0x10, 0x20); // *NOP $2010,X

        Cpu6502 cpu = bus.CreateCpu();
        cpu.A = 0x11;
        cpu.X = 0x22;
        cpu.Y = 0x33;
        byte flags = cpu.P;

        cpu.Step();

        Assert.Equal(0x1003, cpu.PC);
        Assert.Equal(0x11, cpu.A);
        Assert.Equal(0x22, cpu.X);
        Assert.Equal(0x33, cpu.Y);
        Assert.Equal(flags, cpu.P);
    }
}
