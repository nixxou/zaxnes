//! NES 2.0 mapper 682, Broke Studio's **Rainbow** board (lot MAPPERS-N, phase 1 of
//! `docs/progress/MAPPERS-N.md`'s plan).
//!
//! Read from Mesen2 `Homebrew/Rainbow.cpp`, `Rainbow.h` and `RainbowAudio.h` (read, not copied)
//! and Broke Studio's `rainbow-net/NES/mapper-doc.md`:
//!
//! * PRG: `$8000-$FFFF` in five modes (`$4100` bits 0-2: one 32 KiB bank, two 16 KiB, 16+8+8,
//!   four 8 KiB, eight 4 KiB), each bank register 16 bits (`$4108-$410F` high, `$4118-$411F`
//!   low) with bit 15 selecting the PRG-RAM; `$6000-$7FFF` one 8 KiB or two 4 KiB banks
//!   (`$4100` bit 7, `$4106-$4107`/`$4116-$4117`) whose bits 15-14 pick PRG-ROM, PRG-RAM or the
//!   FPGA's 8 KiB; the FPGA RAM again at `$4800-$4FFF` (its last 2 KiB) and `$5000-$5FFF` (4 KiB,
//!   bank `$4115`), and through the auto-incrementing port `$415C-$415F`;
//! * CHR: `$4120` mode (1 to 16 banks, 8 KiB down to 512 bytes), source (CHR-ROM, CHR-RAM, FPGA
//!   RAM), window enable and sprite extended mode; sixteen 16-bit bank registers
//!   (`$4130-$414F`);
//! * nametables: per table a bank (`$4126-$4129`) and a control (`$412A-$412D`): source (CIRAM,
//!   CHR-RAM, FPGA RAM, CHR-ROM), fill mode (`$4124`/`$4125`), attribute and background extended
//!   modes reading one byte per tile from the FPGA RAM; a window (`$412E-$412F`,
//!   `$4170-$4175`) with its own table and scroll;
//! * IRQs: an MMC5-style scanline IRQ (`$4150-$4153`, in-frame and h-blank flags in `$4151`) and
//!   a 16-bit CPU-cycle counter (`$4158-$415B`), status at `$4161`, a jitter counter at `$4154`;
//! * NMI and IRQ vectors replaceable (`$416B-$416F`);
//! * three generated OAM routines at `$4280-$47FF` (`$4240-$4242` pages);
//! * audio: two VRC6 pulses and a VRC6 saw at `$41A0-$41A8`, output pins `$41A9`, volume `$41AA`
//!   — this core's [`Vrc6Audio`] itself, its `$9000-$B002` ports at the same offsets;
//! * Wi-Fi at `$4190-$4194`: in this phase no ESP is present (Mesen2 mainline's behaviour).
//!
//! The PPU side is Mesen2's `MapperReadVram`, split across this core's hooks: the counters and
//! the per-fetch decisions on [`Mapper::on_ppu_address_bus`], which the PPU calls with every
//! address it reads during rendering just before the read; the nametable answer in
//! [`Mapper::read_nametable_from_vram`] and the pattern answer in [`Mapper::ppu_peek`]. Not yet
//! here (phase 2): sprite extended mode's OAM copy (`$2003`/`$2004` writes), the `$4011` read,
//! and the 3D tile capture of the extended modes; phase 4 the flash self-programming
//! (FIDELITY-LEADS J35).
use super::rainbow_esp::Esp;
use super::{Mapper, NO_ROM_WINDOW};
use crate::apu::Vrc6Audio;
use crate::cartridges::{BankWindows, Cartridge, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// One nametable's control byte (`$412A-$412D`, and the window's `$412F`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct NtControl(u8);

impl NtControl {
    const fn attr_ext(self) -> bool {
        self.0 & 0x01 != 0
    }
    const fn bg_ext(self) -> bool {
        self.0 & 0x02 != 0
    }
    const fn fpga_src(self) -> usize {
        ((self.0 >> 2) & 0x03) as usize
    }
    const fn fill(self) -> bool {
        self.0 & 0x20 != 0
    }
    const fn source(self) -> u8 {
        self.0 >> 6
    }
}

/// What the next nametable read answers, decided by the fetch notification before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NtAnswer {
    /// The table's own source at the fetched address.
    Plain,
    /// A computed byte (fill mode, an extended attribute).
    Value(u8),
    /// The window's table in FPGA RAM: offset, and for an attribute the shift of its quadrant.
    Window(u16, Option<u8>),
}

/// What one write to an S29 flash asks for (Mesen2 `FlashS29.h`, read, not copied).
#[derive(Debug, Clone, PartialEq, Eq)]
enum FlashOp {
    None,
    /// Program (AND) the byte at the offset.
    Program(usize, u8),
    /// Erase to `$FF` the byte range.
    Erase(std::ops::Range<usize>),
}

/// The S29 command state machine: unlock `$AAA`/`$555` on the low 12 address bits, program
/// (`$A0`), unlock bypass (`$20`), sector and chip erase (`$80` then `$30`/`$10`), software ID
/// (`$90`, left by `$F0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct FlashS29 {
    /// 0 waiting for a command, 1 programming, 2 erasing.
    mode: u8,
    cycle: u8,
    software_id: bool,
    unlock_bypass: bool,
}

impl FlashS29 {
    const fn reset_state(&mut self) {
        self.mode = 0;
        self.cycle = 0;
    }

    /// The erase sector of `offset` in a chip of `size` bytes: 64 KiB sectors, the last 64 KiB
    /// split `{32, 8, 8, 16}` KiB on the S29AL008/016 and in eight 8 KiB on the larger chips.
    fn sector(offset: usize, size: usize) -> std::ops::Range<usize> {
        let last = size.saturating_sub(0x10000);
        if offset < last || size < 0x10000 {
            let start = offset & !0xFFFF;
            return start..(start + 0x10000).min(size);
        }
        let split: &[usize] = if size >= 0x40_0000 {
            &[8, 8, 8, 8, 8, 8, 8, 8]
        } else {
            &[32, 8, 8, 16]
        };
        let mut start = last;
        for kib in split {
            let end = start + kib * 1024;
            if offset < end {
                return start..end;
            }
            start = end;
        }
        last..size
    }

    /// The device ID byte at offset 2 (`S29AL008` `$5B`, `AL016` `$49`, `JL032`/`GL064S` `$7E`).
    const fn device_id(size: usize) -> u8 {
        match size {
            0x20_0000 => 0x49,
            0x40_0000 | 0x80_0000 => 0x7E,
            _ => 0x5B,
        }
    }

    /// A software-ID read, `None` outside that mode.
    fn read(self, offset: usize, size: usize) -> Option<u8> {
        if !self.software_id {
            return None;
        }
        let large = size >= 0x40_0000;
        Some(match offset & 0x1FF {
            0x00 => 0x01,
            0x02 => Self::device_id(size),
            0x1C if large => {
                if size == 0x40_0000 {
                    0x0A
                } else {
                    0x10
                }
            }
            0x1E if large => 0x00,
            _ => 0xFF,
        })
    }

    fn write(&mut self, offset: usize, value: u8, size: usize) -> FlashOp {
        let cmd = offset & 0xFFF;
        match self.mode {
            1 => {
                self.reset_state();
                return if offset < size {
                    FlashOp::Program(offset, value)
                } else {
                    FlashOp::None
                };
            }
            2 => {
                match (self.cycle, cmd, value) {
                    (3, 0xAAA, 0xAA) | (4, 0x555, 0x55) => self.cycle += 1,
                    (5, 0xAAA, 0x10) => {
                        self.reset_state();
                        return FlashOp::Erase(0..size);
                    }
                    (5, _, 0x30) => {
                        self.reset_state();
                        return FlashOp::Erase(Self::sector(offset, size));
                    }
                    _ => self.reset_state(),
                }
                return FlashOp::None;
            }
            _ => {}
        }
        if self.unlock_bypass {
            match (self.cycle, value) {
                (0, 0xA0) => self.mode = 1,
                (0, 0x90) => self.cycle = 1,
                (1, 0x00) => {
                    self.unlock_bypass = false;
                    self.cycle = 0;
                }
                _ => self.cycle = 0,
            }
            return FlashOp::None;
        }
        match (self.cycle, cmd, value) {
            (0, _, 0xF0) => {
                self.reset_state();
                self.software_id = false;
            }
            (0, 0xAAA, 0xAA) | (1, 0x555, 0x55) => self.cycle += 1,
            (2, 0xAAA, command) => {
                self.cycle = 0;
                match command {
                    0x20 => self.unlock_bypass = true,
                    0x80 => {
                        self.mode = 2;
                        self.cycle = 3;
                    }
                    0x90 => self.software_id = true,
                    0xA0 => self.mode = 1,
                    0xF0 => self.software_id = false,
                    _ => {}
                }
            }
            _ => self.cycle = 0,
        }
        FlashOp::None
    }

    fn save(self, writer: &mut StateWriter) {
        writer.write_u8(self.mode);
        writer.write_u8(self.cycle);
        writer.write_bool(self.software_id);
        writer.write_bool(self.unlock_bypass);
    }

    fn load(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mode = reader.read_u8()? % 3;
        self.cycle = reader.read_u8()? % 6;
        self.software_id = reader.read_bool()?;
        self.unlock_bypass = reader.read_bool()?;
        Ok(())
    }
}

/// Mapper 682.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rainbow {
    high_banks: [u16; 8],
    low_banks: [u16; 2],
    chr_banks: [u16; 16],
    fpga_bank: u8,
    high_mode: u8,
    low_mode: bool,
    chr_mode: u8,
    chr_source: u8,
    window_enabled: bool,
    sprite_ext: bool,
    bg_ext_offset: u8,
    nt_banks: [u8; 4],
    nt_control: [NtControl; 4],
    fill_tile: u8,
    fill_attr: u8,
    window_control: NtControl,
    window_bank: u8,
    window_x1: u8,
    window_x2: u8,
    window_y1: u8,
    window_y2: u8,
    window_scroll_x: u8,
    window_scroll_y: u8,
    in_window: bool,
    sl_irq_enabled: bool,
    sl_irq_pending: bool,
    sl_irq_scanline: u8,
    sl_irq_offset: u8,
    last_ppu_read: u16,
    scanline: i16,
    ppu_idle: u8,
    nt_read_count: u8,
    ppu_read_count: u8,
    in_frame: bool,
    in_hblank: bool,
    jitter: u8,
    cpu_irq_counter: u16,
    cpu_irq_reload: u16,
    cpu_irq_enabled: bool,
    cpu_irq_pending: bool,
    cpu_irq_enable_after_ack: bool,
    cpu_irq_ack_on_4011: bool,
    fpga_addr: u16,
    fpga_inc: u8,
    nmi_vector_enabled: bool,
    irq_vector_enabled: bool,
    nmi_vector: u16,
    irq_vector: u16,
    override_tile: bool,
    ext_data: u8,
    nt_fetch_count: u8,
    window_scanline: u8,
    sprite_ext_data: [u8; 64],
    oam_pos_y: [u8; 64],
    /// The OAM address the board's copy of `$2003` follows.
    oam_addr: u8,
    oam_mappings: [u8; 8],
    sprite_ext_bank: u8,
    large_sprites: bool,
    oam_ext_page: u8,
    oam_slow_page: u8,
    oam_code: Vec<u8>,
    oam_code_locked: bool,
    esp_enabled: bool,
    wifi_irq_enabled: bool,
    recv_dst: u8,
    send_src: u8,
    /// `$4191` bit 7: an ESP message is in FPGA RAM, not yet acknowledged.
    data_received: bool,
    /// Every this many CPU cycles the host network is polled (while it is in use).
    net_poll: u16,
    esp: Box<Esp>,
    fpga_ram: Vec<u8>,
    audio: Vrc6Audio,
    audio_pins: u8,
    audio_volume: u8,
    prg_flash: FlashS29,
    chr_flash: FlashS29,
    nt_answer: NtAnswer,
    /// The answer of the attribute fetch, kept apart so that the 3D capture can decide both.
    at_answer: NtAnswer,
    /// What the 3D capture displaced, put back by `set_background_capture_state`.
    capture_stash: Option<(NtAnswer, NtAnswer)>,
}

impl Rainbow {
    /// The FPGA's own RAM (`GetMapperRamSize`).
    const FPGA_RAM: usize = 0x2000;
    /// The generated OAM routines (`_oamCode`).
    const OAM_CODE: usize = 0x506;
    /// The flash save file's sector, the S29's smallest erase unit (`FlashRom`).
    pub(crate) const FLASH_TRACK_SECTOR: usize = 0x2000;

    pub(crate) fn new(_memory: &CartridgeMemory) -> Self {
        Self {
            high_banks: [0; 8],
            low_banks: [0; 2],
            chr_banks: [0; 16],
            fpga_bank: 0,
            high_mode: 0,
            low_mode: false,
            chr_mode: 0,
            chr_source: 0,
            window_enabled: false,
            sprite_ext: false,
            bg_ext_offset: 0,
            nt_banks: [0; 4],
            nt_control: [NtControl(0); 4],
            fill_tile: 0,
            fill_attr: 0,
            window_control: NtControl(0),
            window_bank: 0,
            window_x1: 0,
            window_x2: 0,
            window_y1: 0,
            window_y2: 0,
            window_scroll_x: 0,
            window_scroll_y: 0,
            in_window: false,
            sl_irq_enabled: false,
            sl_irq_pending: false,
            sl_irq_scanline: 0,
            sl_irq_offset: 0,
            last_ppu_read: 0,
            scanline: 0,
            ppu_idle: 0,
            nt_read_count: 0,
            ppu_read_count: 0,
            in_frame: false,
            in_hblank: false,
            jitter: 0,
            cpu_irq_counter: 0,
            cpu_irq_reload: 0,
            cpu_irq_enabled: false,
            cpu_irq_pending: false,
            cpu_irq_enable_after_ack: false,
            cpu_irq_ack_on_4011: false,
            fpga_addr: 0,
            fpga_inc: 0,
            nmi_vector_enabled: false,
            irq_vector_enabled: false,
            nmi_vector: 0,
            irq_vector: 0,
            override_tile: false,
            ext_data: 0,
            nt_fetch_count: 0,
            window_scanline: 0,
            sprite_ext_data: [0; 64],
            oam_pos_y: [0; 64],
            oam_addr: 0,
            oam_mappings: [0; 8],
            sprite_ext_bank: 0,
            large_sprites: false,
            oam_ext_page: 0,
            oam_slow_page: 0,
            oam_code: vec![0; Self::OAM_CODE],
            oam_code_locked: false,
            esp_enabled: false,
            wifi_irq_enabled: false,
            recv_dst: 0,
            send_src: 0,
            data_received: false,
            net_poll: 0,
            esp: Box::new(Esp::new()),
            fpga_ram: vec![0; Self::FPGA_RAM],
            audio: Vrc6Audio::new(),
            audio_pins: 0,
            audio_volume: 0,
            prg_flash: FlashS29::default(),
            chr_flash: FlashS29::default(),
            nt_answer: NtAnswer::Plain,
            at_answer: NtAnswer::Plain,
            capture_stash: None,
        }
    }

    // ------------------------------------------------------------------ CPU map

    /// Where a CPU address of `$6000-$FFFF` lands: `(kind, byte offset)`, kind 0 PRG-ROM, 1
    /// PRG-RAM, 2 FPGA RAM (`SelectHighBank`, `SelectLowBank`).
    fn cpu_target(&self, address: u16) -> (u8, usize) {
        if address < 0x8000 {
            let (reg, size) = if self.low_mode {
                (usize::from((address >> 12) & 0x01), 0x1000)
            } else {
                (0, 0x2000)
            };
            let bank = self.low_banks[reg];
            let within = usize::from(address) & (size - 1);
            return match bank >> 14 {
                0 | 1 => (0, usize::from(bank & 0x7FFF) * size + within),
                2 => (1, usize::from(bank & 0x3FFF) * size + within),
                _ => (2, usize::from(bank & 0x3FFF) * size + within),
            };
        }
        let slot = usize::from((address - 0x8000) >> 12);
        let (reg, size) = match self.high_mode {
            0 => (0, 0x8000),
            1 => (if slot < 4 { 0 } else { 4 }, 0x4000),
            2 => match slot {
                0..=3 => (0, 0x4000),
                4 | 5 => (4, 0x2000),
                _ => (6, 0x2000),
            },
            3 => (slot & !1, 0x2000),
            _ => (slot, 0x1000),
        };
        let bank = self.high_banks[reg];
        let within = usize::from(address) & (size - 1);
        let kind = u8::from(bank & 0x8000 != 0);
        (kind, usize::from(bank & 0x7FFF) * size + within)
    }

    fn read_target(&self, memory: &CartridgeMemory, kind: u8, offset: usize) -> Option<u8> {
        if kind == 0 {
            let size = memory.rom().prg_rom().len();
            if let Some(value) = self.prg_flash.read(offset % size.max(1), size) {
                return Some(value);
            }
        }
        let memory_of = match kind {
            0 => memory.rom().prg_rom(),
            1 => memory.prg_ram(),
            _ => &self.fpga_ram,
        };
        if memory_of.is_empty() {
            return None;
        }
        Some(memory_of[offset % memory_of.len()])
    }

    /// The register reads, `None` for open bus; `side` applies the read's side effects.
    fn read_register(&mut self, memory: &CartridgeMemory, address: u16, side: bool) -> Option<u8> {
        let value = match address {
            0x4100 => self.high_mode | (u8::from(self.low_mode) << 7),
            0x4120 => {
                self.chr_mode
                    | (u8::from(self.window_enabled) << 4)
                    | (u8::from(self.sprite_ext) << 5)
                    | (self.chr_source << 6)
            }
            0x412A..=0x412D => self.nt_control[usize::from(address - 0x412A)].0,
            0x412F => self.window_control.0,
            // The doc's acknowledge-on-read; the flag returned is the one before it (Mesen2
            // clears it first, so its bit 0 always reads 0; FIDELITY-LEADS J35).
            0x4151 => {
                let value = (u8::from(self.in_hblank) << 7)
                    | (u8::from(self.in_frame) << 6)
                    | u8::from(self.sl_irq_pending);
                if side {
                    self.sl_irq_pending = false;
                }
                value
            }
            0x4154 => self.jitter,
            0x415F => {
                let value = self.fpga_ram[usize::from(self.fpga_addr) & 0x1FFF];
                if side {
                    self.fpga_addr = self.fpga_addr.wrapping_add(u16::from(self.fpga_inc)) & 0x1FFF;
                }
                value
            }
            0x4160 => 0x20,
            0x4161 => {
                u8::from(self.wifi_irq_pending())
                    | (u8::from(self.cpu_irq_pending) << 6)
                    | (u8::from(self.sl_irq_pending) << 7)
            }
            0x4190 => u8::from(self.esp_enabled) | (u8::from(self.wifi_irq_enabled) << 1),
            // "Data received" (bit 7) and "Data ready", a message waiting on the ESP's side.
            0x4191 => (u8::from(self.data_received) << 7) | (u8::from(self.esp.has_answer()) << 6),
            // "Data sent": a message leaves at once here.
            0x4192 => 0x80,
            0x4280..=0x47FF => {
                if side {
                    match address {
                        0x4280 => self.generate_oam_slow_update(),
                        0x4282 => self.generate_ext_update(),
                        0x4286 => self.generate_oam_clear(),
                        _ => {}
                    }
                    if address >= 0x4286 {
                        self.oam_code_locked = false;
                    }
                }
                self.oam_code
                    .get(usize::from(address - 0x4280))
                    .copied()
                    .unwrap_or(0)
            }
            0x4800..=0x4FFF => self.fpga_ram[0x1800 + usize::from(address & 0x07FF)],
            0x5000..=0x5FFF => {
                self.fpga_ram[usize::from(self.fpga_bank) * 0x1000 + usize::from(address & 0x0FFF)]
            }
            0xFFFA | 0xFFFB => {
                if side {
                    self.in_frame = false;
                    self.last_ppu_read = 0;
                    self.scanline = 0;
                    self.sl_irq_pending = false;
                }
                if self.nmi_vector_enabled {
                    self.nmi_vector.to_le_bytes()[usize::from(address & 0x01)]
                } else {
                    let (kind, offset) = self.cpu_target(address);
                    return self.read_target(memory, kind, offset);
                }
            }
            0xFFFE | 0xFFFF if self.irq_vector_enabled => {
                self.irq_vector.to_le_bytes()[usize::from(address & 0x01)]
            }
            0x6000..=0xFFFF => {
                let (kind, offset) = self.cpu_target(address);
                return self.read_target(memory, kind, offset);
            }
            _ => return None,
        };
        Some(value)
    }

    fn write_register_value(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let set_high =
            |word: &mut u16, value: u8| *word = (*word & 0x00FF) | (u16::from(value) << 8);
        let set_low = |word: &mut u16, value: u8| *word = (*word & 0xFF00) | u16::from(value);
        match address {
            0x4100 => {
                self.high_mode = value & 0x07;
                self.low_mode = value & 0x80 != 0;
            }
            0x4106 | 0x4107 => set_high(&mut self.low_banks[usize::from(address - 0x4106)], value),
            0x4116 | 0x4117 => set_low(&mut self.low_banks[usize::from(address - 0x4116)], value),
            0x4108..=0x410F => set_high(&mut self.high_banks[usize::from(address - 0x4108)], value),
            0x4118..=0x411F => set_low(&mut self.high_banks[usize::from(address - 0x4118)], value),
            0x4115 => self.fpga_bank = value & 0x01,
            0x4120 => {
                self.chr_mode = value & 0x07;
                self.window_enabled = value & 0x10 != 0;
                self.sprite_ext = value & 0x20 != 0;
                self.chr_source = value >> 6;
            }
            0x4121 => self.bg_ext_offset = value & 0x1F,
            0x4124 => self.fill_tile = value,
            0x4125 => self.fill_attr = value & 0x03,
            0x4126..=0x4129 => self.nt_banks[usize::from(address - 0x4126)] = value,
            0x412A..=0x412D => self.nt_control[usize::from(address - 0x412A)] = NtControl(value),
            0x412E => self.window_bank = value,
            0x412F => self.window_control = NtControl(value),
            0x4130..=0x413F => set_high(&mut self.chr_banks[usize::from(address - 0x4130)], value),
            0x4140..=0x414F => set_low(&mut self.chr_banks[usize::from(address - 0x4140)], value),
            0x4150..=0x4175 => self.write_irq_window(address, value),
            0x4190 => {
                self.esp_enabled = value & 0x01 != 0;
                self.wifi_irq_enabled = value & 0x02 != 0;
            }
            0x4191 => {
                self.data_received = false;
                self.deliver_esp();
            }
            0x4192 => self.send_esp(),
            0x4193 => self.recv_dst = value & 0x07,
            0x4194 => self.send_src = value & 0x07,
            // `RainbowAudio::WriteRegister`: the three generators at the VRC6's own offsets.
            0x41A0..=0x41A8 => {
                let index = address - 0x41A0;
                let port = [0x9000u16, 0xA000, 0xB000][usize::from(index / 3)] + index % 3;
                self.audio.write(port, value);
            }
            0x41A9 => self.audio_pins = value & 0x07,
            0x41AA => self.audio_volume = value & 0x0F,
            0x4200..=0x423F => self.sprite_ext_data[usize::from(address - 0x4200)] = value,
            0x4240 => self.sprite_ext_bank = value & 0x07,
            0x4241 => self.oam_slow_page = value & 0x07,
            0x4242 => self.oam_ext_page = value & 0x1F,
            0x4800..=0x4FFF => self.fpga_ram[0x1800 + usize::from(address & 0x07FF)] = value,
            0x5000..=0x5FFF => {
                self.fpga_ram
                    [usize::from(self.fpga_bank) * 0x1000 + usize::from(address & 0x0FFF)] = value;
            }
            0x6000..=0xFFFF => {
                // PRG-ROM writes are the flash's commands (phase 4); RAM takes the byte.
                let (kind, offset) = self.cpu_target(address);
                match kind {
                    1 => {
                        let ram = memory.prg_ram_mut();
                        if !ram.is_empty() {
                            let len = ram.len();
                            ram[offset % len] = value;
                        }
                    }
                    2 => self.fpga_ram[offset % Self::FPGA_RAM] = value,
                    _ => self.write_prg_flash(memory, offset, value),
                }
            }
            _ => {}
        }
    }

    /// `$4150-$4175`: the two IRQs, the FPGA RAM port, the vectors and the window.
    fn write_irq_window(&mut self, address: u16, value: u8) {
        let set_high =
            |word: &mut u16, value: u8| *word = (*word & 0x00FF) | (u16::from(value) << 8);
        let set_low = |word: &mut u16, value: u8| *word = (*word & 0xFF00) | u16::from(value);
        match address {
            0x4150 => self.sl_irq_scanline = value,
            0x4151 => self.sl_irq_enabled = true,
            0x4152 => {
                self.sl_irq_enabled = false;
                self.sl_irq_pending = false;
            }
            0x4153 => self.sl_irq_offset = value.clamp(1, 170),
            0x4158 => set_high(&mut self.cpu_irq_reload, value),
            0x4159 => set_low(&mut self.cpu_irq_reload, value),
            0x415A => {
                self.cpu_irq_enabled = value & 0x01 != 0;
                self.cpu_irq_enable_after_ack = value & 0x02 != 0;
                self.cpu_irq_ack_on_4011 = value & 0x04 != 0;
                if self.cpu_irq_enabled {
                    self.cpu_irq_counter = self.cpu_irq_reload;
                }
            }
            0x415B => self.ack_cpu_irq(),
            0x415C => self.fpga_addr = (self.fpga_addr & 0x00FF) | (u16::from(value & 0x1F) << 8),
            0x415D => self.fpga_addr = (self.fpga_addr & 0xFF00) | u16::from(value),
            0x415E => self.fpga_inc = value,
            0x415F => {
                self.fpga_ram[usize::from(self.fpga_addr) & 0x1FFF] = value;
                self.fpga_addr = self.fpga_addr.wrapping_add(u16::from(self.fpga_inc)) & 0x1FFF;
            }
            0x416B => {
                self.nmi_vector_enabled = value & 0x01 != 0;
                self.irq_vector_enabled = value & 0x02 != 0;
            }
            0x416C => set_high(&mut self.nmi_vector, value),
            0x416D => set_low(&mut self.nmi_vector, value),
            0x416E => set_high(&mut self.irq_vector, value),
            0x416F => set_low(&mut self.irq_vector, value),
            0x4170 => self.window_x1 = value & 0x1F,
            0x4171 => self.window_x2 = value & 0x1F,
            0x4172 => self.window_y1 = value,
            0x4173 => self.window_y2 = value,
            0x4174 => self.window_scroll_x = value & 0x1F,
            0x4175 => self.window_scroll_y = value,
            _ => {}
        }
    }

    /// `WriteRegister`'s `_prgFlash->Write`: PRG-ROM through the save-tracked `FlashRom`.
    fn write_prg_flash(&mut self, memory: &mut CartridgeMemory, offset: usize, value: u8) {
        let size = memory.rom().prg_rom().len();
        match self.prg_flash.write(offset % size.max(1), value, size) {
            FlashOp::None => {}
            FlashOp::Program(offset, value) => memory.flash_program(offset, value),
            FlashOp::Erase(range) => {
                for chunk in range.step_by(Self::FLASH_TRACK_SECTOR) {
                    memory.flash_erase_sector(chunk);
                }
            }
        }
    }

    /// `MapperWriteVram`'s `_chrFlash->Write`: CHR-ROM in place (not in the save file, J35).
    fn write_chr_flash(&mut self, memory: &mut CartridgeMemory, offset: usize, value: u8) {
        let rom = memory.chr_rom_mut();
        let size = rom.len();
        if size == 0 {
            return;
        }
        match self.chr_flash.write(offset % size, value, size) {
            FlashOp::None => {}
            FlashOp::Program(offset, value) => rom[offset] &= value,
            FlashOp::Erase(range) => rom[range].fill(0xFF),
        }
    }

    /// `$4161` bit 0: a message was received with `$4190` bit 1 set.
    const fn wifi_irq_pending(&self) -> bool {
        self.wifi_irq_enabled && self.data_received
    }

    /// `$4192` write: the message at the `$4194` page, `[length][command][parameters]`, goes to
    /// the ESP (nothing with the ESP disabled).
    fn send_esp(&mut self) {
        if !self.esp_enabled {
            return;
        }
        let base = 0x1800 + usize::from(self.send_src) * 0x100;
        let length = usize::from(self.fpga_ram[base]);
        if length == 0 {
            return;
        }
        let end = (base + 1 + length).min(Self::FPGA_RAM);
        let message = self.fpga_ram[base + 1..end].to_vec();
        self.esp.command(&message);
        self.deliver_esp();
    }

    /// Puts the next ESP answer in the `$4193` page when the previous one was acknowledged.
    fn deliver_esp(&mut self) {
        if !self.esp_enabled || self.data_received {
            return;
        }
        if let Some(answer) = self.esp.pop_answer() {
            let base = 0x1800 + usize::from(self.recv_dst) * 0x100;
            let length = answer.len().min(Self::FPGA_RAM - base).min(0x100);
            self.fpga_ram[base..base + length].copy_from_slice(&answer[..length]);
            self.data_received = true;
        }
    }

    fn ack_cpu_irq(&mut self) {
        self.cpu_irq_enabled = self.cpu_irq_enable_after_ack;
        if self.cpu_irq_enabled {
            self.cpu_irq_counter = self.cpu_irq_reload;
        }
        self.cpu_irq_pending = false;
    }

    /// `Rainbow::Reset`: the registers Mesen2 writes on a reset, in its order.
    fn reset_registers(&mut self, memory: &mut CartridgeMemory) {
        for (address, value) in [
            (0x4100, 0x00),
            (0x4108, 0x00),
            (0x4118, 0x00),
            (0x4120, 0x00),
            (0x4130, 0x00),
            (0x4140, 0x00),
            (0x4126, 0x00),
            (0x4127, 0x00),
            (0x4128, 0x01),
            (0x4129, 0x01),
            (0x412E, 0x00),
            (0x412A, 0x00),
            (0x412B, 0x00),
            (0x412C, 0x00),
            (0x412D, 0x00),
            (0x412F, 0x80),
            (0x4241, 0x07),
            (0x4242, 0x1B),
            (0x4152, 0x00),
            (0x4153, 0x87),
            (0x415A, 0x00),
            (0x4190, 0x00),
            (0x41A9, 0x03),
            (0x41AA, 0x0F),
        ] {
            self.write_register_value(memory, address, value);
        }
    }

    // ------------------------------------------------------------------ OAM routines

    fn generate_oam_clear(&mut self) {
        if self.oam_code_locked {
            return;
        }
        self.oam_code_locked = true;
        let mut code = Vec::with_capacity(Self::OAM_CODE);
        code.extend_from_slice(&self.oam_code[..6]);
        for sprite in 0..64u8 {
            code.extend_from_slice(&[0xA9, sprite * 4]);
            if sprite == 0 {
                code.extend_from_slice(&[0xAA, 0xCA]);
            }
            code.extend_from_slice(&[0x8D, 0x03, 0x20, 0x8E, 0x04, 0x20]);
        }
        code.push(0x60);
        self.store_code(&code);
    }

    fn generate_ext_update(&mut self) {
        if self.oam_code_locked {
            return;
        }
        self.oam_code_locked = true;
        let mut code = Vec::with_capacity(Self::OAM_CODE);
        code.extend_from_slice(&self.oam_code[..2]);
        let base = 0x1800 + usize::from(self.oam_ext_page) * 0x40;
        for sprite in 0..64u8 {
            let value = self.fpga_ram[(base + usize::from(sprite)) & 0x1FFF];
            code.extend_from_slice(&[0xA9, value, 0x8D, sprite, 0x42]);
        }
        code.push(0x60);
        self.store_code(&code);
    }

    fn generate_oam_slow_update(&mut self) {
        if self.oam_code_locked {
            return;
        }
        self.oam_code_locked = true;
        let mut code = vec![0xA9, 0x00, 0x8D, 0x03, 0x20];
        let base = 0x1800 + usize::from(self.oam_slow_page) * 0x100;
        for j in 0..256 {
            let value = self.fpga_ram[(base + j) & 0x1FFF];
            code.extend_from_slice(&[0xA9, value, 0x8D, 0x04, 0x20]);
        }
        code.push(0x60);
        self.store_code(&code);
    }

    /// Writes `code` from the start of the routine area, leaving the rest as it was (Mesen2
    /// overwrites in place).
    fn store_code(&mut self, code: &[u8]) {
        let length = code.len().min(Self::OAM_CODE);
        self.oam_code[..length].copy_from_slice(&code[..length]);
    }

    // ------------------------------------------------------------------ PPU side

    /// Reads CHR through the extended modes' wide address (`ReadChr`).
    fn read_chr_wide(&self, memory: &CartridgeMemory, address: usize) -> u8 {
        let memory_of: &[u8] = match self.chr_source {
            0 => {
                let size = memory.rom().chr_rom().len();
                if let Some(value) = self.chr_flash.read(address % size.max(1), size) {
                    return value;
                }
                memory.rom().chr_rom()
            }
            1 => memory.chr_ram(),
            _ => return self.fpga_ram[address & 0x1FFF],
        };
        if memory_of.is_empty() {
            return 0;
        }
        memory_of[address % memory_of.len()]
    }

    /// The plain pattern-table read (`SelectChrBank`).
    fn read_chr_banked(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        if self.chr_source >= 2 {
            return self.fpga_ram[usize::from(address & 0x0FFF)];
        }
        let size = 0x2000usize >> self.chr_mode.min(4);
        let slot = usize::from(address) / size;
        let offset =
            usize::from(self.chr_banks[slot & 0x0F]) * size + (usize::from(address) & (size - 1));
        self.read_chr_wide(memory, offset)
    }

    /// `UpdateInWindowFlag`.
    fn update_in_window(&mut self) {
        let scanline = self.window_line_scanline();
        let y_match = if self.window_y1 >= self.window_y2 {
            scanline <= self.window_y2 || scanline > self.window_y1
        } else {
            scanline >= self.window_y1 && scanline <= self.window_y2
        };
        let column = (self.nt_fetch_count + 1) % 42;
        let x_match = if self.window_x1 >= self.window_x2 {
            column <= self.window_x2 || column > self.window_x1
        } else {
            column >= self.window_x1 && column <= self.window_x2
        };
        self.in_window = x_match && y_match;
    }

    /// The scanline a fetch belongs to: the next one from the prefetch of fetch 41 on.
    fn window_line_scanline(&self) -> u8 {
        let line = if self.nt_fetch_count >= 41 {
            self.scanline + 1
        } else {
            self.scanline
        };
        line.to_le_bytes()[0]
    }

    /// `DetectScanlineStart`: three identical nametable reads end a line.
    fn detect_scanline_start(&mut self, address: u16) {
        if !(0x2000..=0x2FFF).contains(&address) {
            self.nt_read_count = 0;
            return;
        }
        if self.last_ppu_read != address {
            self.nt_read_count = 0;
            return;
        }
        self.nt_read_count += 1;
        if self.nt_read_count >= 2 {
            if self.in_frame {
                self.scanline += 1;
            } else {
                self.in_frame = true;
                self.scanline = 0;
            }
            self.process_sprite_eval();
            self.nt_fetch_count = 0;
            self.ppu_read_count = 0;
            self.nt_read_count = 0;
            self.in_hblank = false;
        }
    }

    /// `ProcessSpriteEval`: the first eight sprites of the copy of OAM's Y bytes on this line.
    fn process_sprite_eval(&mut self) {
        let line = i32::from(self.scanline);
        let height = if self.large_sprites { 16 } else { 8 };
        self.oam_mappings = [0; 8];
        let mut count = 0;
        for (index, y) in self.oam_pos_y.iter().enumerate() {
            let y = i32::from(*y);
            if line >= y && line < y + height {
                self.oam_mappings[count] = u8::try_from(index).unwrap_or(0);
                count += 1;
                if count >= 8 {
                    break;
                }
            }
        }
    }

    /// The nametable fetch's decisions (`MapperReadVram`, nametable half).
    fn decide_nametable(&mut self, address: u16) {
        let attribute = address & 0x03FF >= 0x03C0;
        if !attribute {
            self.nt_fetch_count = self.nt_fetch_count.wrapping_add(1);
            if self.nt_fetch_count == 33 {
                self.in_hblank = true;
            }
            self.in_window = false;
            if self.window_enabled {
                self.update_in_window();
            }
        }
        let control = if self.in_window {
            self.window_control
        } else {
            self.nt_control[usize::from((address >> 10) & 0x03)]
        };
        let mut offset = address & 0x03FF;
        let mut shift = None;
        if self.in_window {
            let window_line =
                (u16::from(self.window_line_scanline()) + u16::from(self.window_scroll_y)) % 240;
            self.window_scanline = u8::try_from(window_line).unwrap_or(0);
            let column = u16::from((self.nt_fetch_count + 1) % 42);
            let x = (column + u16::from(self.window_scroll_x)) & 0x1F;
            let nt = (window_line / 8) * 32 + x;
            if attribute {
                offset = 0x3C0 + (((window_line >> 2) & 0xF8) | (x >> 2));
                shift = Some(u8::try_from(((nt >> 4) & 0x04) | (nt & 0x02)).unwrap_or(0));
            } else {
                offset = nt;
            }
        }
        if attribute {
            self.at_answer = if control.attr_ext() {
                NtAnswer::Value(((self.ext_data & 0xC0) >> 6) * 0x55)
            } else if control.fill() {
                NtAnswer::Value(self.fill_attr * 0x55)
            } else if self.in_window {
                NtAnswer::Window(offset, shift)
            } else {
                NtAnswer::Plain
            };
            return;
        }
        self.override_tile = control.bg_ext();
        if control.attr_ext() || control.bg_ext() {
            self.ext_data =
                self.fpga_ram[(control.fpga_src() * 0x400 + usize::from(offset & 0x03FF)) & 0x1FFF];
        }
        self.nt_answer = if control.fill() {
            NtAnswer::Value(self.fill_tile)
        } else if self.in_window {
            NtAnswer::Window(offset, None)
        } else {
            NtAnswer::Plain
        };
    }

    /// A table's own byte, from its source (`UpdateState`'s nametable mappings).
    fn plain_nametable(&self, memory: &CartridgeMemory, address: u16, vram: &[u8]) -> u8 {
        let table = usize::from((address >> 10) & 0x03);
        let bank = usize::from(self.nt_banks[table]);
        let offset = usize::from(address & 0x03FF);
        let memory_of: &[u8] = match self.nt_control[table].source() {
            0 => {
                return vram
                    .get(((bank & 0x01) << 10) | offset)
                    .copied()
                    .unwrap_or(0)
            }
            1 => memory.chr_ram(),
            2 => return self.fpga_ram[((bank & 0x03) << 10) | offset],
            _ => memory.rom().chr_rom(),
        };
        if memory_of.is_empty() {
            return 0;
        }
        memory_of[((bank << 10) | offset) % memory_of.len()]
    }
}

impl Mapper for Rainbow {
    fn mapper_number(&self) -> i32 {
        682
    }

    fn banks(&self) -> Option<&BankWindows> {
        None
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        None
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        for word in self
            .high_banks
            .iter()
            .chain(&self.low_banks)
            .chain(&self.chr_banks)
        {
            writer.write_u16(*word);
        }
        for byte in [
            self.fpga_bank,
            self.high_mode,
            u8::from(self.low_mode),
            self.chr_mode,
            self.chr_source,
            u8::from(self.window_enabled),
            u8::from(self.sprite_ext),
            self.bg_ext_offset,
            self.fill_tile,
            self.fill_attr,
            self.window_control.0,
            self.window_bank,
            self.window_x1,
            self.window_x2,
            self.window_y1,
            self.window_y2,
            self.window_scroll_x,
            self.window_scroll_y,
            u8::from(self.in_window),
            u8::from(self.sl_irq_enabled),
            u8::from(self.sl_irq_pending),
            self.sl_irq_scanline,
            self.sl_irq_offset,
            self.ppu_idle,
            self.nt_read_count,
            self.ppu_read_count,
            u8::from(self.in_frame),
            u8::from(self.in_hblank),
            self.jitter,
            u8::from(self.cpu_irq_enabled),
            u8::from(self.cpu_irq_pending),
            u8::from(self.cpu_irq_enable_after_ack),
            u8::from(self.cpu_irq_ack_on_4011),
            self.fpga_inc,
            u8::from(self.nmi_vector_enabled),
            u8::from(self.irq_vector_enabled),
            u8::from(self.override_tile),
            self.ext_data,
            self.nt_fetch_count,
            self.window_scanline,
            self.sprite_ext_bank,
            u8::from(self.large_sprites),
            self.oam_ext_page,
            self.oam_slow_page,
            u8::from(self.oam_code_locked),
            u8::from(self.esp_enabled),
            u8::from(self.wifi_irq_enabled),
            self.recv_dst,
            self.send_src,
            self.audio_pins,
            self.audio_volume,
            self.oam_addr,
        ] {
            writer.write_u8(byte);
        }
        writer.write_raw(&self.nt_banks);
        writer.write_raw(&self.nt_control.map(|control| control.0));
        for word in [
            self.last_ppu_read,
            u16::from_le_bytes(self.scanline.to_le_bytes()),
            self.cpu_irq_counter,
            self.cpu_irq_reload,
            self.fpga_addr,
            self.nmi_vector,
            self.irq_vector,
        ] {
            writer.write_u16(word);
        }
        writer.write_raw(&self.sprite_ext_data);
        writer.write_raw(&self.oam_pos_y);
        writer.write_raw(&self.oam_mappings);
        writer.write_raw(&self.oam_code);
        writer.write_raw(&self.fpga_ram);
        self.audio.save_state(writer);
        self.prg_flash.save(writer);
        self.chr_flash.save(writer);
        writer.write_bool(self.data_received);
        self.esp.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        for word in self
            .high_banks
            .iter_mut()
            .chain(self.low_banks.iter_mut())
            .chain(self.chr_banks.iter_mut())
        {
            *word = reader.read_u16()?;
        }
        let mut bytes = [0u8; 52];
        reader.read_into(&mut bytes)?;
        let flag = |index: usize| bytes[index] != 0;
        self.fpga_bank = bytes[0];
        self.high_mode = bytes[1];
        self.low_mode = flag(2);
        self.chr_mode = bytes[3];
        self.chr_source = bytes[4];
        self.window_enabled = flag(5);
        self.sprite_ext = flag(6);
        self.bg_ext_offset = bytes[7];
        self.fill_tile = bytes[8];
        self.fill_attr = bytes[9];
        self.window_control = NtControl(bytes[10]);
        self.window_bank = bytes[11];
        self.window_x1 = bytes[12];
        self.window_x2 = bytes[13];
        self.window_y1 = bytes[14];
        self.window_y2 = bytes[15];
        self.window_scroll_x = bytes[16];
        self.window_scroll_y = bytes[17];
        self.in_window = flag(18);
        self.sl_irq_enabled = flag(19);
        self.sl_irq_pending = flag(20);
        self.sl_irq_scanline = bytes[21];
        self.sl_irq_offset = bytes[22];
        self.ppu_idle = bytes[23];
        self.nt_read_count = bytes[24];
        self.ppu_read_count = bytes[25];
        self.in_frame = flag(26);
        self.in_hblank = flag(27);
        self.jitter = bytes[28];
        self.cpu_irq_enabled = flag(29);
        self.cpu_irq_pending = flag(30);
        self.cpu_irq_enable_after_ack = flag(31);
        self.cpu_irq_ack_on_4011 = flag(32);
        self.fpga_inc = bytes[33];
        self.nmi_vector_enabled = flag(34);
        self.irq_vector_enabled = flag(35);
        self.override_tile = flag(36);
        self.ext_data = bytes[37];
        self.nt_fetch_count = bytes[38];
        self.window_scanline = bytes[39];
        self.sprite_ext_bank = bytes[40];
        self.large_sprites = flag(41);
        self.oam_ext_page = bytes[42];
        self.oam_slow_page = bytes[43];
        self.oam_code_locked = flag(44);
        self.esp_enabled = flag(45);
        self.wifi_irq_enabled = flag(46);
        self.recv_dst = bytes[47];
        self.send_src = bytes[48];
        self.audio_pins = bytes[49];
        self.audio_volume = bytes[50];
        self.oam_addr = bytes[51];
        reader.read_into(&mut self.nt_banks)?;
        let mut controls = [0u8; 4];
        reader.read_into(&mut controls)?;
        self.nt_control = controls.map(NtControl);
        self.last_ppu_read = reader.read_u16()?;
        self.scanline = i16::from_le_bytes(reader.read_u16()?.to_le_bytes());
        self.cpu_irq_counter = reader.read_u16()?;
        self.cpu_irq_reload = reader.read_u16()?;
        self.fpga_addr = reader.read_u16()? & 0x1FFF;
        self.nmi_vector = reader.read_u16()?;
        self.irq_vector = reader.read_u16()?;
        reader.read_into(&mut self.sprite_ext_data)?;
        reader.read_into(&mut self.oam_pos_y)?;
        reader.read_into(&mut self.oam_mappings)?;
        reader.read_into(&mut self.oam_code)?;
        reader.read_into(&mut self.fpga_ram)?;
        self.audio.load_state(reader)?;
        self.prg_flash.load(reader)?;
        self.chr_flash.load(reader)?;
        self.data_received = reader.read_bool()?;
        self.esp.load_state(reader)?;
        self.nt_answer = NtAnswer::Plain;
        self.at_answer = NtAnswer::Plain;
        self.capture_stash = None;
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.audio.reset();
        self.reset_registers(memory);
    }

    /// The scanline IRQ follows the PPU's fetches.
    fn ppu_irq_horizon(&self) -> Option<i64> {
        None
    }

    fn irq_asserted(&self) -> bool {
        (self.cpu_irq_enabled && self.cpu_irq_pending)
            || (self.sl_irq_enabled && self.sl_irq_pending)
            || self.wifi_irq_pending()
    }

    /// `ProcessCpuClock`.
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        self.jitter = self.jitter.wrapping_add(1);
        self.net_poll = self.net_poll.wrapping_add(1);
        if matches!(self.net_poll & 0x0FFF, 0) && self.esp.polls() {
            self.esp.poll();
            self.deliver_esp();
        }
        self.audio.step_cpu_cycle();
        if self.cpu_irq_counter != 0 {
            self.cpu_irq_counter -= 1;
            if self.cpu_irq_counter == 0 {
                self.cpu_irq_counter = self.cpu_irq_reload;
                if !self.irq_asserted() {
                    self.jitter = 0;
                }
                self.cpu_irq_pending = true;
            }
        }
        if self.ppu_idle != 0 {
            self.ppu_idle -= 1;
            if self.ppu_idle == 0 {
                // "cleared when the PPU is no longer rendering … 3 CPU cycles … without a PPU
                // read" (the doc, quoted by Mesen2).
                self.in_frame = false;
                self.in_hblank = false;
                self.scanline = -1;
                self.nt_fetch_count = 0;
                self.nt_read_count = 0;
            }
        }
    }

    /// The VRC6 generators' level, scaled by `$41AA` and gated by `$41A9`'s two pins.
    fn expansion_audio_output(&self) -> Option<i32> {
        if matches!(self.audio_pins & 0x03, 0) {
            return Some(0);
        }
        Some(self.audio.output() * i32::from(self.audio_volume) / 15)
    }

    fn prg_ram_enabled(&self) -> bool {
        false
    }

    fn rom_windows(&self, _memory: &CartridgeMemory) -> [usize; 4] {
        [NO_ROM_WINDOW; 4]
    }

    fn cpu_read_override(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        (address >= 0x4100).then(|| self.read_register(memory, address, true))
    }

    fn cpu_peek_override(&self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        if address < 0x4100 {
            return None;
        }
        let mut copy = self.clone();
        Some(copy.read_register(memory, address, false))
    }

    /// Every write of `$4100-$FFFF` is the board's: registers, FPGA RAM, and PRG-RAM through the
    /// banks rather than the cartridge's linear window.
    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if address < 0x4100 {
            return false;
        }
        self.write_register_value(memory, address, value);
        true
    }

    fn on_ppu_control(&mut self, control: u8) {
        self.large_sprites = control & 0x20 != 0;
    }

    fn set_network_enabled(&mut self, enabled: bool) {
        self.esp.set_network_enabled(enabled);
    }

    /// `WriteRam`'s `$2003`/`$2004` arms: the Y byte of every sprite written through `$2004`, for
    /// `ProcessSpriteEval` (an OAM DMA does not pass here, as in Mesen2).
    fn snoop_cpu_bus_write(&mut self, address: u16, value: u8) {
        match address {
            0x2003 => self.oam_addr = value,
            0x2004 => {
                if matches!(self.oam_addr & 0x03, 0) {
                    self.oam_pos_y[usize::from(self.oam_addr >> 2)] = value;
                }
                self.oam_addr = self.oam_addr.wrapping_add(1);
            }
            _ => {}
        }
    }

    /// `ReadRam`'s `$4011` arm: with `$415A` bit 2 the read acknowledges the CPU IRQ and answers
    /// the audio DAC doubled (`GetLastOutput() << 1`); otherwise the open bus.
    fn snoop_cpu_bus_read(&mut self, address: u16) -> Option<u8> {
        if address != 0x4011 || !self.cpu_irq_ack_on_4011 {
            return None;
        }
        self.ack_cpu_irq();
        Some(u8::try_from(self.audio.dac() << 1).unwrap_or(0xFF))
    }

    /// `MapperReadVram`'s bookkeeping, for every address the PPU reads while rendering.
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, _clock: i64) {
        let address = address & 0x3FFF;
        self.ppu_read_count = self.ppu_read_count.wrapping_add(1);
        self.detect_scanline_start(address);
        if i16::from(self.sl_irq_scanline) == self.scanline
            && self.sl_irq_offset == self.ppu_read_count
        {
            if !self.irq_asserted() {
                self.jitter = 0;
            }
            self.sl_irq_pending = true;
        }
        self.ppu_idle = 3;
        self.last_ppu_read = address;
        if !self.in_frame {
            self.nt_answer = NtAnswer::Plain;
            self.at_answer = NtAnswer::Plain;
            return;
        }
        if (0x2000..=0x2FFF).contains(&address) {
            self.decide_nametable(address);
        }
    }

    fn nametable_page(&self, _table: i32) -> i32 {
        Cartridge::SERVED_BY_CARTRIDGE
    }

    fn read_nametable_from_vram(&self, memory: &CartridgeMemory, address: u16, vram: &[u8]) -> u8 {
        if self.in_frame {
            let answer = if address & 0x03FF >= 0x03C0 {
                self.at_answer
            } else {
                self.nt_answer
            };
            match answer {
                NtAnswer::Value(value) => return value,
                NtAnswer::Window(offset, shift) => {
                    let byte = self.fpga_ram
                        [(usize::from(self.window_bank) * 0x400 + usize::from(offset)) & 0x1FFF];
                    return match shift {
                        Some(shift) => ((byte >> shift) & 0x03) * 0x55,
                        None => byte,
                    };
                }
                NtAnswer::Plain => {}
            }
        }
        self.plain_nametable(memory, address, vram)
    }

    /// CIRAM tables take `$2007` writes in console VRAM; the others are written by
    /// [`Mapper::write_nametable`].
    fn nametable_write_offset(&self, address: u16) -> Option<usize> {
        let table = usize::from((address >> 10) & 0x03);
        (self.nt_control[table].source() == 0).then(|| {
            ((usize::from(self.nt_banks[table]) & 0x01) << 10) | usize::from(address & 0x03FF)
        })
    }

    fn write_nametable(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let table = usize::from((address >> 10) & 0x03);
        let bank = usize::from(self.nt_banks[table]);
        let offset = usize::from(address & 0x03FF);
        match self.nt_control[table].source() {
            1 => {
                let ram = memory.chr_ram_mut();
                if !ram.is_empty() {
                    let len = ram.len();
                    ram[((bank << 10) | offset) % len] = value;
                }
            }
            2 => self.fpga_ram[((bank & 0x03) << 10) | offset] = value,
            _ => {}
        }
    }

    /// Every extended mode and the window change what a background tile shows.
    fn substitutes_background_tiles(&self) -> bool {
        self.window_enabled
            || self.nt_control.iter().any(|control| {
                control.attr_ext() || control.bg_ext() || control.fill() || control.source() != 0
            })
    }

    /// The per-fetch state the capture moves: in-frame, scanline, fetch count, window and tile
    /// flags, the extended byte and the window line.
    fn background_capture_state(&self) -> u64 {
        u64::from(self.in_frame)
            | (u64::from(u16::from_le_bytes(self.scanline.to_le_bytes())) << 1)
            | (u64::from(self.nt_fetch_count) << 17)
            | (u64::from(self.in_window) << 25)
            | (u64::from(self.override_tile) << 26)
            | (u64::from(self.ext_data) << 27)
            | (u64::from(self.window_scanline) << 35)
    }

    fn set_background_capture_state(&mut self, state: u64) {
        self.in_frame = state & 0x01 != 0;
        self.scanline = i16::from_le_bytes(
            u16::try_from((state >> 1) & 0xFFFF)
                .unwrap_or(0)
                .to_le_bytes(),
        );
        self.nt_fetch_count = u8::try_from((state >> 17) & 0xFF).unwrap_or(0);
        self.in_window = state & (1 << 25) != 0;
        self.override_tile = state & (1 << 26) != 0;
        self.ext_data = u8::try_from((state >> 27) & 0xFF).unwrap_or(0);
        self.window_scanline = u8::try_from((state >> 35) & 0xFF).unwrap_or(0);
        if let Some((nt, at)) = self.capture_stash.take() {
            self.nt_answer = nt;
            self.at_answer = at;
        }
    }

    /// Replays the two nametable fetches of column `column` of line `line` as the rendering makes
    /// them: tiles 0 and 1 are the previous line's fetches 41 and 42, tile `c` the line's fetch
    /// `c - 1` (`UpdateInWindowFlag`'s `(_ntFetchCounter + 1) % 42`).
    fn position_captured_background_tile(&mut self, address: u16, column: i32, line: i32) {
        if self.capture_stash.is_none() {
            self.capture_stash = Some((self.nt_answer, self.at_answer));
        }
        let column = u8::try_from(column.rem_euclid(42)).unwrap_or(0);
        let line = i16::try_from(line).unwrap_or(0);
        self.in_frame = true;
        self.scanline = if column < 2 { line - 1 } else { line };
        self.nt_fetch_count = (column + 40) % 42;
        self.decide_nametable(address);
        let coarse_y = (address >> 5) & 0x1F;
        let tile_column = address & 0x1F;
        let attribute = (address & 0x2C00) | 0x03C0 | ((coarse_y & 0x1C) << 1) | (tile_column >> 2);
        self.decide_nametable(attribute);
    }

    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    fn chr_peek_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    /// The pattern half of `MapperReadVram`: background extended tiles, window tiles and sprite
    /// extended tiles while in frame, the banked table otherwise.
    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        if self.in_frame {
            let background = self.nt_fetch_count < 33 || self.nt_fetch_count >= 41;
            let wide = |offset: usize| self.read_chr_wide(memory, offset);
            let ext = usize::from(self.ext_data & 0x3F) << 12;
            let bank = usize::from(self.bg_ext_offset) << 18;
            if self.in_window && background {
                let line = usize::from(self.window_scanline & 0x07);
                if self.override_tile {
                    return wide((usize::from(address) & 0x0FF8) | line | ext | bank);
                }
                return self.read_chr_banked(
                    memory,
                    (address & 0x1FF8) | u16::from(self.window_scanline & 0x07),
                );
            }
            if self.override_tile && background {
                return wide((usize::from(address) & 0x0FFF) | ext | bank);
            }
            if self.sprite_ext && !background {
                let slot = usize::from(self.nt_fetch_count.saturating_sub(33)).min(7);
                let sprite = usize::from(self.oam_mappings[slot]) & 0x3F;
                let data = usize::from(self.sprite_ext_data[sprite]);
                let bank = usize::from(self.sprite_ext_bank);
                return wide(if self.large_sprites {
                    (bank << 21) | (data << 13) | (usize::from(address) & 0x1FFF)
                } else {
                    (bank << 20) | (data << 12) | (usize::from(address) & 0x0FFF)
                });
            }
        }
        self.read_chr_banked(memory, address)
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let size = 0x2000usize >> self.chr_mode.min(4);
        let slot = usize::from(address) / size;
        let offset =
            usize::from(self.chr_banks[slot & 0x0F]) * size + (usize::from(address) & (size - 1));
        match self.chr_source {
            0 => self.write_chr_flash(memory, offset, value),
            1 => {
                let ram = memory.chr_ram_mut();
                if !ram.is_empty() {
                    let len = ram.len();
                    ram[offset % len] = value;
                }
            }
            _ => self.fpga_ram[usize::from(address & 0x0FFF)] = value,
        }
    }
}
