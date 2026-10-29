/// The bus the CPU is wired to. Every [`CpuBus::read`] and [`CpuBus::write`] is one CPU
/// cycle: the implementation advances the rest of the machine (PPU, APU, cycle-counted
/// mapper logic) inside the call, which is what makes the cycle counts of REQ-CORE-015 and
/// the interleaving of REQ-CORE-110 come out right.
pub trait CpuBus {
    /// Reads a byte, consuming one CPU cycle. Side effects of the read apply.
    fn read(&mut self, address: u16) -> u8;

    /// Reads a byte of the instruction stream, consuming one CPU cycle: an opcode, an operand,
    /// or one of the 6502's dummy reads of the program counter (single-byte instructions,
    /// `BRK`/`RTS`/`RTI`, a taken branch and its page fix-up, the return address of `RTS`, an
    /// interrupt's entry). The same as [`CpuBus::read`] unless the bus shows code somewhere to
    /// instruction fetches only (the widescreen extension's execution zone).
    fn read_code(&mut self, address: u16) -> u8 {
        self.read(address)
    }

    /// Writes a byte, consuming one CPU cycle.
    fn write(&mut self, address: u16, value: u8);

    /// Reads a byte without consuming a cycle and without any side effect, for the host
    /// memory accessor and the tracer (REQ-CORE-080, REQ-CORE-081).
    fn peek(&self, address: u16) -> u8;

    /// Whether the /NMI line is currently pulled low. The CPU detects its falling edge.
    fn nmi_asserted(&self) -> bool;

    /// Whether any IRQ source (APU frame counter, DMC, mapper) is holding /IRQ low. This
    /// is a level, not an edge.
    fn irq_asserted(&self) -> bool;

    /// Master CPU cycle count since power-on. It includes the cycles the CPU is halted
    /// for, such as OAM DMA (REQ-CORE-110, `CYC` column of `nestest.log`).
    fn cycle_count(&self) -> i64;
}
