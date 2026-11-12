//! `$2000`-`$2007`.
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/PPU.regs.cs`. These are `impl Nes` rather
//! than `impl Ppu` because each one of them reaches the board (`nes.Board.AddressPpu`) or the CPU
//! (`nes.cpu.TotalExecutedCycles`) the way the C# does through its `nes` field.
//! MIT; see `LICENSE-BizHawk`.

use super::DecayType;
use crate::nes::Nes;

impl Nes {
    //PPU CONTROL (write)
    fn write_2000(&mut self, value: u8) {
        if self.ppu.reg_2000.vblank_nmi_gen == 0
            && ((value & 0x80) != 0)
            && self.ppu.reg2002_vblank_active
            && !self.ppu.reg2002_vblank_clear_pending
        {
            //if we just unleashed the vblank interrupt then activate it now
            self.ppu.nmi_pending_instructions = 2;
        }
        self.ppu.set_reg_2000_value(value);
    }

    //PPU MASK (write)
    fn write_2001(&mut self, value: u8) {
        self.ppu.reg_2001.set_value(value);
        self.ppu.install_2001 = 2;
    }

    //PPU STATUS (read)
    fn read_2002(&mut self) -> u8 {
        let ret = self.peek_2002();

        // reading from $2002 resets the destination for $2005 and $2006 writes
        self.ppu.vtoggle = false;
        self.ppu.reg2002_vblank_active = false;
        self.ppu.reg2002_vblank_active_pending = false;

        // update the open bus here
        self.ppu.ppu_open_bus = ret;
        self.ppu.ppu_open_bus_decay(DecayType::High);
        ret
    }

    fn peek_2002(&self) -> u8 {
        // the `_isVS2c05` variants are not ported; VS. System is out of this lot's scope.
        ((i32::from(self.ppu.reg2002_vblank_active) << 7)
            | (i32::from(self.ppu.reg2002_objhit) << 6)
            | (i32::from(self.ppu.reg2002_objoverflow) << 5)
            | i32::from(self.ppu.ppu_open_bus & 0x1F)) as u8
    }

    //OAM ADDRESS (write)
    fn write_2003(&mut self, _addr: i32, value: u8) {
        // in NTSC this does several glitchy things to corrupt OAM; the C#'s loop is commented out
        self.ppu.reg_2003 = value;
    }

    //OAM DATA (write)
    fn write_2004(&mut self, value: u8) {
        let mut value = value;
        if (self.ppu.reg_2003 & 3) == 2 {
            //some of the OAM bits are unwired so we mask them out here
            value &= 0xE3;
        }
        if self.ppu.ppur.status.rendering() {
            // don't write to OAM if the screen is on and we are in the active display area
            // this impacts sprite evaluation
            if self.ppu.show_bg_new || self.ppu.show_obj_new {
                // glitchy increment of OAM index
                self.ppu.oam_index += 4;
                self.ppu.oam_index &= 0x1FC;
                self.ppu.reg_2003 = self.ppu.reg_2003.wrapping_add(4);
                self.ppu.reg_2003 &= 0xFC;
            } else {
                self.ppu.oam[self.ppu.reg_2003 as usize] = value;
                self.ppu.reg_2003 = self.ppu.reg_2003.wrapping_add(1);
            }
        } else {
            self.ppu.oam[self.ppu.reg_2003 as usize] = value;
            self.ppu.reg_2003 = self.ppu.reg_2003.wrapping_add(1);
        }
    }

    fn read_2004(&mut self) -> u8 {
        let ret;
        // behaviour depends on whether things are being rendered or not
        if self.ppu.ppuon() {
            if self.ppu.ppur.status.sl < 241 {
                if self.ppu.ppur.status.cycle <= 64 {
                    ret = 0xFF; // during this time all reads return FF
                } else if self.ppu.ppur.status.cycle <= 256 {
                    ret = self.ppu.read_value;
                } else if self.ppu.ppur.status.cycle <= 320 {
                    ret = self.ppu.read_value;
                } else {
                    ret = self.ppu.soam[0];
                }
            } else {
                ret = self.ppu.oam[self.ppu.reg_2003 as usize];
            }
        } else {
            ret = self.ppu.oam[self.ppu.reg_2003 as usize];
        }

        self.ppu.ppu_open_bus = ret;
        self.ppu.ppu_open_bus_decay(DecayType::All);
        ret
    }

    //SCROLL (write)
    fn write_2005(&mut self, value: u8) {
        if !self.ppu.vtoggle {
            self.ppu.ppur._ht = i32::from(value) >> 3;
            self.ppu.ppur.fh = i32::from(value) & 7;
        } else {
            self.ppu.ppur._vt = i32::from(value) >> 3;
            self.ppu.ppur._fv = i32::from(value) & 7;
        }
        self.ppu.vtoggle = !self.ppu.vtoggle;
    }

    //VRAM address register (write)
    fn write_2006(&mut self, value: u8) {
        let v = i32::from(value);
        if !self.ppu.vtoggle {
            self.ppu.ppur._vt &= 0x07;
            self.ppu.ppur._vt |= (v & 0x3) << 3;
            self.ppu.ppur._h = (v >> 2) & 1;
            self.ppu.ppur._v = (v >> 3) & 1;
            self.ppu.ppur._fv = (v >> 4) & 3;
            self.ppu.reg_2006_2 = value;
        } else {
            self.ppu.ppur._vt &= 0x18;
            self.ppu.ppur._vt |= v >> 5;
            self.ppu.ppur._ht = v & 31;

            // testing indicates that this operation is delayed by 3 pixels
            self.ppu.install_2006 = 3;
        }

        self.ppu.vtoggle = !self.ppu.vtoggle;
    }

    //VRAM data register (r/w)
    fn write_2007(&mut self, value: u8) {
        let mut addr = self.ppu.ppur.get_2007access();
        if self.ppu.ppuphase == super::PPU_PHASE_BG && self.ppu.show_bg_new {
            addr = self.ppu.ppur.get_ntread();
        }

        if (addr & 0x3F00) == 0x3F00 {
            //handle palette. this is being done nestopia style
            let addr = addr & 0x1F;
            let color = value & 0x3F; //are these bits really unwired?

            self.ppu.palram[addr as usize] = color;
            if (addr & 3) == 0 {
                self.ppu.palram[(addr ^ 0x10) as usize] = color;
            }
        } else {
            let addr = addr & 0x3FFF;

            self.ppubus_write(addr, value);
        }

        let rendering = self.ppu.ppur.status.rendering() && self.ppu.ppuon();
        let by32 = self.ppu.reg_2000.vram_incr32 != 0;
        self.ppu.ppur.increment2007(rendering, by32);

        //see comments in $2006
        if self.ppu.ppur.status.sl >= 241 || !self.ppu.ppuon() {
            let a = self.ppu.ppur.get_2007access();
            self.board.address_ppu(a);
        }
    }

    fn read_2007(&mut self) -> u8 {
        let addr = self.ppu.ppur.get_2007access() & 0x3FFF;
        let mut bus_case = 0;
        //ordinarily we return the buffered values
        let mut ret = self.ppu.vram_buffer;

        //in any case, we read from the ppu bus
        self.ppu.vram_buffer = self.ppubus_read(addr, false, false);

        //but reads from the palette are implemented in the PPU and return immediately
        if (addr & 0x3F00) == 0x3F00 {
            ret =
                self.ppu.palram[(addr & 0x1F) as usize].wrapping_add(self.ppu.ppu_open_bus & 0xC0);
            bus_case = 1;
        }

        let rendering = self.ppu.ppur.status.rendering() && self.ppu.ppuon();
        let by32 = self.ppu.reg_2000.vram_incr32 != 0;
        self.ppu.ppur.increment2007(rendering, by32);

        //see comments in $2006
        if self.ppu.ppur.status.sl >= 241 || !self.ppu.ppuon() {
            let a = self.ppu.ppur.get_2007access();
            self.board.address_ppu(a);
        }

        // update open bus here
        self.ppu.ppu_open_bus = ret;
        if bus_case == 0 {
            self.ppu.ppu_open_bus_decay(DecayType::All);
        } else {
            self.ppu.ppu_open_bus_decay(DecayType::Low);
        }

        ret
    }

    fn peek_2007(&self) -> u8 {
        let addr = self.ppu.ppur.get_2007access() & 0x3FFF;

        //ordinarily we return the buffered values
        let mut ret = self.ppu.vram_buffer;

        //but reads from the palette are implemented in the PPU and return immediately
        if (addr & 0x3F00) == 0x3F00 {
            ret = self.ppu.palram[(addr & 0x1F) as usize];
        }

        ret
    }

    /// `PPU.ReadReg`.
    pub(crate) fn ppu_read_reg(&mut self, addr: i32) -> u8 {
        match addr {
            0 => self.ppu.ppu_open_bus, // read_2000
            1 => self.ppu.ppu_open_bus, // read_2001
            2 => self.read_2002(),
            3 => self.ppu.ppu_open_bus, // read_2003
            4 => self.read_2004(),
            5 => self.ppu.ppu_open_bus, // read_2005
            6 => self.ppu.ppu_open_bus, // read_2006
            7 => {
                if self.cpu.total_executed_cycles == self.ppu.double_2007_read && !self.dmc_dma_exec
                {
                    self.ppu.ppu_open_bus
                } else {
                    let ret_spec = self.read_2007();
                    self.ppu.double_2007_read = self.cpu.total_executed_cycles + 1;
                    ret_spec
                }
            }
            // `default: throw new InvalidOperationException()`; unreachable, the caller masks
            _ => 0,
        }
    }

    /// `PPU.PeekReg`.
    pub(crate) fn ppu_peek_reg(&self, addr: i32) -> u8 {
        match addr {
            0 | 1 | 3 | 5 | 6 => self.ppu.ppu_open_bus,
            2 => self.peek_2002(),
            4 => self.ppu.oam[self.ppu.reg_2003 as usize],
            7 => self.peek_2007(),
            _ => 0,
        }
    }

    /// `PPU.WriteReg`.
    pub(crate) fn ppu_write_reg(&mut self, addr: i32, value: u8) {
        self.ppu.ppu_gen_latch = value;
        self.ppu.ppu_open_bus = value;

        match addr & 0x07 {
            0 => self.write_2000(value),
            1 => self.write_2001(value),
            2 => { /* write_2002 is empty */ }
            3 => self.write_2003(addr, value),
            4 => self.write_2004(value),
            5 => self.write_2005(value),
            6 => self.write_2006(value),
            7 => self.write_2007(value),
            _ => {}
        }
    }
}
