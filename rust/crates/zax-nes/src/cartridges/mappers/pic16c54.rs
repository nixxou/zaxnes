//! Lot MAPPERS-P: a Microchip PIC16C54, the microcontroller of mapper 355's boards.
//!
//! A port of MAME's PIC16C5x emulator as puNES 0.111 carries it (`c++/pic16c5x/pic16c5x_m.cpp`,
//! ver 1.16), limited to the PIC16C54 that board uses (9-bit program counter, 32 bytes of file
//! registers, ports A and B):
//!
//! > license:BSD-3-Clause — copyright-holders:Tony La Porta. Microchip PIC16C5x Emulator,
//! > Copyright Tony La Porta, originally written for the MAME project.
//!
//! As in puNES, the configuration word is 0 (no watchdog), the T0CKI pin is never driven, and
//! one instruction cycle lasts four CPU cycles. The program is 512 12-bit words, little-endian
//! in 1 KiB.

use crate::state::{InvalidState, StateReader, StateWriter};

/// The program counter's width (`program_width` 9) and the file registers' (`data_width` 5).
const ROM_MASK: u16 = 0x1FF;
const RAM_MASK: u8 = 0x1F;
/// `ADDR_MASK`: the stack and `goto` keep 11 bits.
const ADDR_MASK: u16 = 0x7FF;

const TMR0: usize = 1;
const PCL: usize = 2;
const STATUS: usize = 3;
const FSR: usize = 4;
const PORTA: usize = 5;
const PORTB: usize = 6;

const PA_REG: u8 = 0xE0;
const TO_FLAG: u8 = 0x10;
const PD_FLAG: u8 = 0x08;
const Z_FLAG: u8 = 0x04;
const DC_FLAG: u8 = 0x02;
const C_FLAG: u8 = 0x01;

const T0CS_FLAG: u8 = 0x20;
const T0SE_FLAG: u8 = 0x10;
const PSA_FLAG: u8 = 0x08;
const PS_REG: u8 = 0x07;

/// A port the chip writes: port A (0) or B (1), and the value with `TRIS_FLAG` set when the
/// write comes from a `tris` instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PortWrite {
    pub(crate) port: u8,
    pub(crate) value: u16,
}

/// `tris`'s marker on the written value (`0x1000 | …`).
pub(crate) const TRIS_FLAG: u16 = 0x1000;

/// The chip (`PIC16C5X` with `picmodel` `0x16C54`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pic16C54 {
    rom: Box<[u8; 0x400]>,
    pc: u16,
    prev_pc: u16,
    w: u8,
    option: u8,
    alu: u8,
    tris_a: u8,
    tris_b: u8,
    stack: [u16; 2],
    prescaler: u16,
    opcode: u16,
    ram: [u8; 32],
    icount: i32,
    delay_timer: i32,
    old_data: u8,
    inst_cycles: i32,
    clock2cycle: i32,
}

impl Pic16C54 {
    /// The constructor's zeroed registers, then a power-on reset.
    pub(crate) fn new(rom: &[u8]) -> Self {
        let mut program = Box::new([0u8; 0x400]);
        let len = rom.len().min(0x400);
        program[..len].copy_from_slice(&rom[..len]);
        let mut chip = Self {
            rom: program,
            pc: 0,
            prev_pc: 0,
            w: 0,
            option: 0,
            alu: 0,
            tris_a: 0,
            tris_b: 0,
            stack: [0; 2],
            prescaler: 0,
            opcode: 0,
            ram: [0; 32],
            icount: 0,
            delay_timer: 0,
            old_data: 0,
            inst_cycles: 0,
            clock2cycle: 0,
        };
        chip.reset(true);
        chip
    }

    /// `PIC16C5x_reset_regs` (the configuration word stays 0).
    fn reset_regs(&mut self) {
        self.pc = ROM_MASK;
        self.tris_a = 0xFF;
        self.tris_b = 0xFF;
        self.option = T0CS_FLAG | T0SE_FLAG | PSA_FLAG | PS_REG;
        self.ram[PCL] = 0xFF;
        self.ram[FSR] |= !RAM_MASK;
        self.prescaler = 0;
        self.delay_timer = 0;
        self.inst_cycles = 0;
    }

    /// `reset`: power-on (`hard`) clears the file registers; RESET sets the flags.
    pub(crate) fn reset(&mut self, hard: bool) {
        if hard {
            self.ram = [0; 32];
            self.reset_regs();
            self.ram[STATUS] &= !PA_REG;
            self.ram[STATUS] |= TO_FLAG | PD_FLAG;
            self.icount = 0;
            self.clock2cycle = 0;
        } else {
            self.ram[STATUS] |= TO_FLAG | PD_FLAG | Z_FLAG | DC_FLAG | C_FLAG;
            self.reset_regs();
        }
    }

    fn status(&self) -> u8 {
        self.ram[STATUS]
    }

    fn set_flag(&mut self, flag: u8, on: bool) {
        if on {
            self.ram[STATUS] |= flag;
        } else {
            self.ram[STATUS] &= !flag;
        }
    }

    const fn addr(&self) -> u8 {
        (self.opcode & 0x1F) as u8
    }

    const fn b0(&self) -> u8 {
        (self.opcode & 0xFF) as u8
    }

    const fn pos(&self) -> u8 {
        ((self.opcode >> 5) & 0x07) as u8
    }

    /// The file register a direct address names: indirect through FSR at 0, then bank-folded.
    fn file_address(&self, address: u8) -> usize {
        let mut address = if address == 0 {
            self.ram[FSR] & RAM_MASK
        } else {
            address
        };
        if address & 0x10 == 0 {
            address &= 0x0F;
        }
        usize::from(address)
    }

    /// `GET_REGFILE`; `read` answers the ports (`m_read`).
    fn get(&self, address: u8, read: &impl Fn(u8) -> u8) -> u8 {
        match self.file_address(address) {
            0 => 0,
            FSR => self.ram[FSR] | !RAM_MASK,
            PORTA => ((read(0) & self.tris_a) | (!self.tris_a & self.ram[PORTA])) & 0x0F,
            PORTB => (read(1) & self.tris_b) | (!self.tris_b & self.ram[PORTB]),
            other => self.ram[other & usize::from(RAM_MASK)],
        }
    }

    /// `STORE_REGFILE`; port writes go to `out`.
    fn store(&mut self, address: u8, data: u8, out: &mut impl FnMut(PortWrite)) {
        match self.file_address(address) {
            0 => {}
            TMR0 => {
                // The timer starts after the next two instructions.
                self.delay_timer = 2;
                if self.option & PSA_FLAG == 0 {
                    self.prescaler = 0;
                }
                self.ram[TMR0] = data;
            }
            PCL => {
                self.ram[PCL] = data;
                self.pc = (u16::from(self.status() & PA_REG) << 4) | u16::from(data);
            }
            STATUS => {
                self.ram[STATUS] =
                    (self.ram[STATUS] & (TO_FLAG | PD_FLAG)) | (data & !(TO_FLAG | PD_FLAG));
            }
            FSR => self.ram[FSR] = data | !RAM_MASK,
            PORTA => {
                let data = data & 0x0F;
                out(PortWrite {
                    port: 0,
                    value: u16::from(data & !self.tris_a),
                });
                self.ram[PORTA] = data;
            }
            PORTB => {
                out(PortWrite {
                    port: 1,
                    value: u16::from(data & !self.tris_b),
                });
                self.ram[PORTB] = data;
            }
            other => self.ram[other & usize::from(RAM_MASK)] = data,
        }
    }

    /// `STORE_RESULT`: the file register when the opcode's D bit (`$20`) is set, else W.
    fn store_result(&mut self, data: u8, out: &mut impl FnMut(PortWrite)) {
        if self.opcode & 0x20 != 0 {
            self.store(self.addr(), data, out);
        } else {
            self.w = data;
        }
    }

    fn z(&mut self) {
        self.set_flag(Z_FLAG, self.alu == 0);
    }

    fn skip(&mut self) {
        self.pc = self.pc.wrapping_add(1);
        self.ram[PCL] = self.pc.to_le_bytes()[0];
        self.inst_cycles += 1;
    }

    /// Runs one opcode (`s_opcode_main`/`s_opcode_00x`), its cycle count in `inst_cycles`.
    #[allow(clippy::too_many_lines)]
    fn execute(&mut self, read: &impl Fn(u8) -> u8, out: &mut impl FnMut(PortWrite)) {
        let op = self.opcode;
        if op & 0xFF0 == 0 {
            self.inst_cycles = 1;
            match op & 0x1F {
                0x02 => self.option = self.w & (T0CS_FLAG | T0SE_FLAG | PSA_FLAG | PS_REG),
                // `sleep`: the watchdog is off (CONFIG 0).
                0x03 => {
                    if self.option & PSA_FLAG != 0 {
                        self.prescaler = 0;
                    }
                    self.set_flag(TO_FLAG, true);
                    self.set_flag(PD_FLAG, false);
                }
                // `clrwdt`.
                0x04 => {
                    if self.option & PSA_FLAG != 0 {
                        self.prescaler = 0;
                    }
                    self.set_flag(TO_FLAG, true);
                    self.set_flag(PD_FLAG, true);
                }
                // `tris` A and B (C is illegal on the 16C54).
                0x05 if self.tris_a != self.w => {
                    self.tris_a = self.w | 0xF0;
                    out(PortWrite {
                        port: 0,
                        value: TRIS_FLAG | u16::from(self.ram[PORTA] & !self.tris_a & 0x0F),
                    });
                }
                0x06 if self.tris_b != self.w => {
                    self.tris_b = self.w;
                    out(PortWrite {
                        port: 1,
                        value: TRIS_FLAG | u16::from(self.ram[PORTB] & !self.tris_b),
                    });
                }
                // `nop` and the illegal ones.
                _ => {}
            }
            return;
        }
        let major = (op >> 4) & 0xFF;
        self.inst_cycles = if (0x80..0xC0).contains(&major) { 2 } else { 1 };
        let address = self.addr();
        match major {
            // `movwf`.
            0x02 | 0x03 => self.store(address, self.w, out),
            // `clrw`.
            0x04 => {
                self.w = 0;
                self.set_flag(Z_FLAG, true);
            }
            // `clrf`.
            0x06 | 0x07 => {
                self.store(address, 0, out);
                self.set_flag(Z_FLAG, true);
            }
            // `subwf`.
            0x08..=0x0B => {
                self.old_data = self.get(address, read);
                self.alu = self.old_data.wrapping_sub(self.w);
                self.store_result(self.alu, out);
                self.z();
                self.set_flag(C_FLAG, self.old_data >= self.alu);
                self.set_flag(DC_FLAG, self.old_data & 0x0F >= self.alu & 0x0F);
            }
            // `decf`.
            0x0C..=0x0F => {
                self.alu = self.get(address, read).wrapping_sub(1);
                self.store_result(self.alu, out);
                self.z();
            }
            // `iorwf`, `andwf`, `xorwf`.
            0x10..=0x1B => {
                let file = self.get(address, read);
                self.alu = match major {
                    0x10..=0x13 => file | self.w,
                    0x14..=0x17 => file & self.w,
                    _ => file ^ self.w,
                };
                self.store_result(self.alu, out);
                self.z();
            }
            // `addwf`.
            0x1C..=0x1F => {
                self.old_data = self.get(address, read);
                self.alu = self.old_data.wrapping_add(self.w);
                self.store_result(self.alu, out);
                self.z();
                self.set_flag(C_FLAG, self.old_data > self.alu);
                self.set_flag(DC_FLAG, self.old_data & 0x0F > self.alu & 0x0F);
            }
            // `movf`, `comf`, `incf`.
            0x20..=0x2B => {
                let file = self.get(address, read);
                self.alu = match major {
                    0x20..=0x23 => file,
                    0x24..=0x27 => !file,
                    _ => file.wrapping_add(1),
                };
                self.store_result(self.alu, out);
                self.z();
            }
            // `decfsz`, `incfsz`.
            0x2C..=0x2F | 0x3C..=0x3F => {
                let file = self.get(address, read);
                self.alu = if major < 0x30 {
                    file.wrapping_sub(1)
                } else {
                    file.wrapping_add(1)
                };
                self.store_result(self.alu, out);
                if self.alu == 0 {
                    self.skip();
                }
            }
            // `rrf`, `rlf`: the carry last (ver 1.15).
            0x30..=0x37 => {
                let file = self.get(address, read);
                let carry = self.status() & C_FLAG != 0;
                let (result, out_bit) = if major < 0x34 {
                    ((file >> 1) | if carry { 0x80 } else { 0 }, file & 0x01 != 0)
                } else {
                    ((file << 1) | u8::from(carry), file & 0x80 != 0)
                };
                self.alu = result;
                self.store_result(result, out);
                self.set_flag(C_FLAG, out_bit);
            }
            // `swapf`.
            0x38..=0x3B => {
                let file = self.get(address, read);
                self.alu = file.rotate_left(4);
                self.store_result(self.alu, out);
            }
            // `bcf`, `bsf`.
            0x40..=0x5F => {
                let bit = 1u8 << self.pos();
                let file = self.get(address, read);
                self.alu = if major < 0x50 {
                    file & !bit
                } else {
                    file | bit
                };
                self.store(address, self.alu, out);
            }
            // `btfsc`, `btfss`.
            0x60..=0x7F => {
                let set = self.get(address, read) & (1 << self.pos()) != 0;
                if set == (major >= 0x70) {
                    self.skip();
                }
            }
            // `retlw`.
            0x80..=0x8F => {
                self.w = self.b0();
                self.pc = self.stack[1] & ADDR_MASK;
                self.stack[1] = self.stack[0];
                self.ram[PCL] = self.pc.to_le_bytes()[0];
            }
            // `call`.
            0x90..=0x9F => {
                self.stack[0] = self.stack[1];
                self.stack[1] = self.pc & ADDR_MASK;
                self.pc = ((u16::from(self.status() & PA_REG) << 4) | u16::from(self.b0())) & 0x6FF;
                self.ram[PCL] = self.pc.to_le_bytes()[0];
            }
            // `goto`.
            0xA0..=0xBF => {
                self.pc = ((u16::from(self.status() & PA_REG) << 4) | (op & 0x1FF)) & ADDR_MASK;
                self.ram[PCL] = self.pc.to_le_bytes()[0];
            }
            // `movlw`.
            0xC0..=0xCF => self.w = self.b0(),
            // `iorlw`, `andlw`, `xorlw`.
            0xD0..=0xFF => {
                self.alu = match major {
                    0xD0..=0xDF => self.b0() | self.w,
                    0xE0..=0xEF => self.b0() & self.w,
                    _ => self.w ^ self.b0(),
                };
                self.w = self.alu;
                self.z();
            }
            // 0x01, 0x05: illegal.
            _ => {}
        }
    }

    /// `PIC16C5x_update_timer`.
    fn update_timer(&mut self, counts: i32) {
        let counts = u16::try_from(counts).unwrap_or(0);
        if self.option & PSA_FLAG == 0 {
            let rate = 2u16 << (self.option & PS_REG);
            self.prescaler += counts;
            if self.prescaler >= rate {
                let ticks = (self.prescaler / rate).to_le_bytes()[0];
                self.ram[TMR0] = self.ram[TMR0].wrapping_add(ticks);
                self.prescaler %= rate;
            }
        } else {
            self.ram[TMR0] = self.ram[TMR0].wrapping_add(counts.to_le_bytes()[0]);
        }
    }

    /// `run`, once per CPU cycle: an instruction cycle every fourth. `read` answers port reads;
    /// port writes go to `out`.
    pub(crate) fn run(&mut self, read: &impl Fn(u8) -> u8, out: &mut impl FnMut(PortWrite)) {
        self.clock2cycle = self.clock2cycle.wrapping_add(1);
        if self.clock2cycle.trailing_zeros() >= 2 {
            self.icount += 1;
        }
        while self.icount > 0 {
            if self.status() & PD_FLAG == 0 {
                // Sleep: the watchdog that would wake it is off.
                self.inst_cycles = 1;
            } else {
                self.prev_pc = self.pc;
                let word = usize::from(self.pc & ROM_MASK) << 1;
                self.opcode = u16::from_le_bytes([self.rom[word], self.rom[word | 1]]);
                self.pc = self.pc.wrapping_add(1);
                self.ram[PCL] = self.ram[PCL].wrapping_add(1);
                self.execute(read, out);
                if self.option & T0CS_FLAG == 0 {
                    if self.delay_timer != 0 {
                        self.delay_timer -= 1;
                    } else {
                        self.update_timer(self.inst_cycles);
                    }
                }
            }
            self.icount -= self.inst_cycles;
        }
    }

    pub(crate) fn save_state(&self, writer: &mut StateWriter) {
        for value in [
            self.pc,
            self.prev_pc,
            self.stack[0],
            self.stack[1],
            self.prescaler,
            self.opcode,
        ] {
            writer.write_u16(value);
        }
        writer.write_raw(&[
            self.w,
            self.option,
            self.alu,
            self.tris_a,
            self.tris_b,
            self.old_data,
        ]);
        writer.write_raw(&self.ram);
        for value in [
            self.icount,
            self.delay_timer,
            self.inst_cycles,
            self.clock2cycle,
        ] {
            writer.write_i32(value);
        }
    }

    pub(crate) fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.pc = reader.read_u16()?;
        self.prev_pc = reader.read_u16()?;
        for value in &mut self.stack {
            *value = reader.read_u16()?;
        }
        self.prescaler = reader.read_u16()?;
        self.opcode = reader.read_u16()?;
        let bytes = reader.read_raw(6)?;
        [
            self.w,
            self.option,
            self.alu,
            self.tris_a,
            self.tris_b,
            self.old_data,
        ] = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]];
        self.ram.copy_from_slice(reader.read_raw(32)?);
        for value in [
            &mut self.icount,
            &mut self.delay_timer,
            &mut self.inst_cycles,
            &mut self.clock2cycle,
        ] {
            *value = reader.read_i32()?;
        }
        Ok(())
    }
}
