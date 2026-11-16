//! Lot MAPPERS-P: the Fukutake Study Box (mapper 186), a RAM adapter whose games come on audio
//! cassettes: the cartridge holds the BIOS, 64 KiB of work RAM and a tape drive that reads pages
//! of data off the tape while the tape's own audio (the lessons' narration) plays.
//!
//! Read from Mesen 2 (`Core/NES/Mappers/StudyBox.h` and `Loaders/StudyBoxLoader.cpp`, GPL v3;
//! read, not copied). A bare iNES image of mapper 186 (the BIOS alone) stays refused
//! (AMD-MAPPERS-N-200): what loads is a Mesen `.studybox` tape (`STBX`: `PAGE` chunks of decoded
//! data with their lead-in and data positions in the audio, then an `AUDI` WAV file), packaged by
//! [`package`] with the user's `StudyBox.bin` (256 KiB) as a NES 2.0 image whose PRG-ROM is the
//! BIOS and whose misc ROM is the tape file. The board reads the tape in place, out of the image.
//!
//! As in Mesen: page seeks take 3 000 000 CPU cycles a page, a page's data starts after its
//! lead-in (its two audio offsets apart, converted at the WAV's rate), a byte comes every 3 355
//! cycles (the first after 7 820), and the tape's audio plays from the data's start while the
//! motor runs, mixed at full scale as the expansion audio.
use crate::state::{InvalidState, StateReader, StateWriter};

/// The BIOS's size (`FirmwareHelper::LoadStudyBoxFirmware`).
pub const BIOS_SIZE: usize = 0x4_0000;
/// The work RAM: sixteen 4 KiB pages (`GetWorkRamSize`).
pub(crate) const WORK_RAM_SIZE: usize = 0x1_0000;

/// The NTSC CPU clock the tape's timings are converted with (the Study Box is a Famicom
/// accessory; Mesen uses the console's clock).
const CPU_CLOCK: u64 = 1_789_773;
/// `_seekPageDelay`, and the first and next byte delays.
const SEEK_DELAY: u32 = 3_000_000;
const FIRST_BYTE: u16 = 7820;
const NEXT_BYTE: u16 = 3355;

/// Why a tape cannot be packaged or read.
fn tape_error(message: &str) -> String {
    format!("Study Box tape: {message}.")
}

/// One `PAGE` chunk: its data's range in the image and its two audio positions (samples).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Page {
    lead_in: u32,
    audio: u32,
    data: (usize, usize),
}

/// The `AUDI` chunk's PCM samples: their range in the image, the frame size and format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Wave {
    data: (usize, usize),
    rate: u32,
    channels: usize,
    bits: u16,
}

impl Wave {
    /// The sample frame `frame` of `image`, mono, 16-bit signed; `None` past the end.
    fn sample(&self, image: &[u8], frame: u64) -> Option<i32> {
        let width = usize::from(self.bits / 8);
        let size = width * self.channels;
        let offset = usize::try_from(frame).ok()?.checked_mul(size)?;
        if size == 0 || offset + size > self.data.1 {
            return None;
        }
        let start = self.data.0 + offset;
        let bytes = image.get(start..start + size)?;
        let mut sum = 0i32;
        for channel in bytes.chunks_exact(width) {
            sum += if width == 1 {
                (i32::from(channel[0]) - 128) << 8
            } else {
                i32::from(i16::from_le_bytes([channel[0], channel[1]]))
            };
        }
        Some(sum / i32::try_from(self.channels).unwrap_or(1))
    }
}

/// The parsed tape: pages and audio, as ranges of the image they were read from.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Tape {
    pages: Vec<Page>,
    wave: Option<Wave>,
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// A RIFF/WAVE file's PCM `data` chunk, at `base` in the image.
fn parse_wave(image: &[u8], base: usize, length: usize) -> Option<Wave> {
    let file = image.get(base..base + length)?;
    if file.get(..4)? != b"RIFF" || file.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut at = 12;
    let mut format = None;
    while at + 8 <= file.len() {
        let id = &file[at..at + 4];
        let size = usize::try_from(u32_at(file, at + 4)?).ok()?;
        let body = at + 8;
        if id == b"fmt " {
            let tag = u16::from_le_bytes([*file.get(body)?, *file.get(body + 1)?]);
            let channels = u16::from_le_bytes([*file.get(body + 2)?, *file.get(body + 3)?]);
            let rate = u32_at(file, body + 4)?;
            let bits = u16::from_le_bytes([*file.get(body + 14)?, *file.get(body + 15)?]);
            if tag != 1 || channels == 0 || !matches!(bits, 8 | 16) || rate == 0 {
                return None;
            }
            format = Some((rate, usize::from(channels), bits));
        } else if id == b"data" {
            let (rate, channels, bits) = format?;
            let size = size.min(file.len() - body);
            return Some(Wave {
                data: (base + body, size),
                rate,
                channels,
                bits,
            });
        }
        at = body + size + (size & 1);
    }
    None
}

/// `StudyBoxLoader::LoadStudyBoxTape` over the tape at `base` in the image.
fn parse_tape(image: &[u8], base: usize) -> Result<Tape, String> {
    let file = image.get(base..).unwrap_or_default();
    if file.len() < 16 || &file[..4] != b"STBX" {
        return Err(tape_error("not a STBX file"));
    }
    if u32_at(file, 4) != Some(4) || u32_at(file, 8) != Some(0x100) {
        return Err(tape_error("unsupported STBX version"));
    }
    let mut tape = Tape {
        pages: Vec::new(),
        wave: None,
    };
    let mut at = 12;
    let (mut previous_audio, mut previous_lead_in) = (0, 0);
    while at + 4 < file.len() {
        let id = &file[at..at + 4];
        let size = u32_at(file, at + 4)
            .and_then(|size| usize::try_from(size).ok())
            .ok_or_else(|| tape_error("truncated chunk"))?;
        let body = at + 8;
        match id {
            b"PAGE" => {
                let lead_in = u32_at(file, body).ok_or_else(|| tape_error("truncated PAGE"))?;
                let audio = u32_at(file, body + 4).ok_or_else(|| tape_error("truncated PAGE"))?;
                if audio < lead_in {
                    return Err(tape_error("a page's data starts before its lead-in"));
                }
                if audio < previous_audio || lead_in < previous_lead_in {
                    return Err(tape_error("pages out of tape order"));
                }
                (previous_audio, previous_lead_in) = (audio, lead_in);
                let length = size
                    .checked_sub(8)
                    .filter(|length| body + 8 + length <= file.len())
                    .ok_or_else(|| tape_error("invalid PAGE size"))?;
                tape.pages.push(Page {
                    lead_in,
                    audio,
                    data: (base + body + 8, length),
                });
                at = body + size;
            }
            b"AUDI" => {
                let kind = u32_at(file, body).ok_or_else(|| tape_error("truncated AUDI"))?;
                let length = size
                    .checked_sub(4)
                    .filter(|length| body + 4 + length <= file.len())
                    .ok_or_else(|| tape_error("invalid AUDI size"))?;
                if kind != 0 {
                    return Err(tape_error("unsupported audio type"));
                }
                tape.wave = parse_wave(image, base + body + 4, length);
                break;
            }
            _ => return Err(tape_error("unsupported chunk")),
        }
    }
    if tape.pages.is_empty() {
        return Err(tape_error("no PAGE chunk"));
    }
    Ok(tape)
}

/// Whether `misc` (an image's misc ROM) is a Study Box tape.
#[must_use]
pub fn is_tape(misc: &[u8]) -> bool {
    misc.starts_with(b"STBX")
}

/// Packages a `.studybox` tape and the `StudyBox.bin` BIOS as the NES 2.0 image this core runs:
/// mapper 186, the BIOS as PRG-ROM, 64 KiB of work RAM, 8 KiB of CHR-RAM, four-screen tables,
/// and the tape file as the misc ROM.
///
/// # Errors
///
/// A BIOS of the wrong size or a tape that does not parse, as a message for the user.
pub fn package(tape: &[u8], bios: &[u8]) -> Result<Vec<u8>, String> {
    if bios.len() != BIOS_SIZE {
        return Err(format!(
            "StudyBox.bin must be {BIOS_SIZE} bytes, this one is {}.",
            bios.len()
        ));
    }
    parse_tape(tape, 0)?;
    let mut image = Vec::with_capacity(16 + bios.len() + tape.len());
    // Mapper 186 ($BA): D4-D7 of byte 6, four-screen; D4-D7 of byte 7, NES 2.0.
    image.extend_from_slice(&[
        b'N', b'E', b'S', 0x1A, 0x10, 0x00, 0xA8, 0xB8, 0x00, 0x00, 0x0A, 0x07, 0x00, 0x00, 0x01,
        0x00,
    ]);
    image.extend_from_slice(bios);
    image.extend_from_slice(tape);
    Ok(image)
}

/// The board's state (`StudyBox`'s fields, Mesen's names).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct StudyBox {
    tape: Tape,
    /// `$4200` (work-RAM pages) and `$4201` (the `$8000` BIOS page).
    reg_4200: u8,
    prg_page: u8,
    ready_for_bit: bool,
    process_bit_delay: u16,
    reg_4202: u8,
    command_counter: u8,
    command: u8,
    current_page: u8,
    seek_page: i16,
    seek_page_delay: u32,
    enable_decoder: bool,
    audio_enabled: bool,
    motor_disabled: bool,
    byte_read_delay: u16,
    irq_enabled: bool,
    page_found: bool,
    page_index: i32,
    page_position: i32,
    in_data_delay: u32,
    in_data_region: bool,
    pub(crate) irq: bool,
    /// The tape's audio: the sample frame playing (`None` when stopped), the fraction of a frame
    /// in CPU-clock units, and the level heard.
    audio_frame: Option<u64>,
    audio_phase: u64,
    pub(crate) level: i32,
}

impl StudyBox {
    /// The board over the tape in `image` at `misc` (the image's misc ROM offset).
    pub(crate) fn new(image: &[u8], misc: usize) -> Option<Self> {
        let tape = parse_tape(image, misc).ok()?;
        Some(Self {
            tape,
            reg_4200: 0,
            prg_page: 0,
            ready_for_bit: false,
            process_bit_delay: 0,
            reg_4202: 0,
            command_counter: 0,
            command: 0,
            current_page: 0,
            seek_page: 0,
            seek_page_delay: 0,
            enable_decoder: false,
            audio_enabled: false,
            motor_disabled: true,
            byte_read_delay: 0,
            irq_enabled: false,
            page_found: false,
            page_index: 0,
            page_position: -1,
            in_data_delay: 0,
            in_data_region: false,
            irq: false,
            audio_frame: None,
            audio_phase: 0,
            level: 0,
        })
    }

    /// `Reset(true)`: the drive and the tape state, not the banks.
    pub(crate) fn soft_reset(&mut self) {
        self.ready_for_bit = false;
        self.process_bit_delay = 0;
        self.reg_4202 = 0;
        self.command_counter = 0;
        self.command = 0;
        self.current_page = 0;
        self.seek_page = 0;
        self.seek_page_delay = 0;
        self.enable_decoder = false;
        self.audio_enabled = false;
        self.motor_disabled = true;
        self.byte_read_delay = 0;
        self.irq_enabled = false;
        self.page_found = false;
        self.page_index = 0;
        self.page_position = -1;
        self.in_data_delay = 0;
        self.in_data_region = false;
        self.audio_frame = None;
        self.audio_phase = 0;
        self.level = 0;
    }

    fn page(&self) -> Option<&Page> {
        usize::try_from(self.page_index)
            .ok()
            .and_then(|index| self.tape.pages.get(index))
    }

    fn page_len(&self) -> i32 {
        self.page()
            .map_or(0, |page| i32::try_from(page.data.1).unwrap_or(i32::MAX))
    }

    /// `ReadLeadInTrack`.
    fn read_lead_in(&mut self) {
        let rate = self.tape.wave.map_or(44_100, |wave| wave.rate);
        let samples = self
            .page()
            .map_or(0, |page| u64::from(page.audio - page.lead_in));
        self.in_data_delay =
            u32::try_from(samples * CPU_CLOCK / u64::from(rate)).unwrap_or(u32::MAX);
        self.page_position = -1;
        self.byte_read_delay = 0;
        self.motor_disabled = false;
        self.page_found = true;
    }

    /// `ProcessCpuClock`, and the tape audio's clock.
    pub(crate) fn step(&mut self, image: &[u8]) {
        if self.process_bit_delay > 0 {
            self.process_bit_delay -= 1;
            if self.process_bit_delay == 0 {
                self.ready_for_bit = true;
            }
        }
        self.step_audio(image);
        if self.motor_disabled {
            return;
        }
        if self.seek_page != i16::from(self.current_page) {
            self.seek_page_delay = self.seek_page_delay.wrapping_sub(1);
            if self.seek_page_delay == 0 {
                self.seek_page_delay = SEEK_DELAY;
                if self.seek_page > i16::from(self.current_page) {
                    self.current_page = self.current_page.wrapping_add(1);
                } else {
                    self.current_page = self.current_page.wrapping_sub(1);
                }
                // The first page whose page number (data byte 5) is the one sought.
                let wanted = i32::from(self.current_page) - 1;
                self.page_index = 0;
                for (index, page) in self.tape.pages.iter().enumerate() {
                    if image.get(page.data.0 + 5).map(|byte| i32::from(*byte)) == Some(wanted)
                        && page.data.1 > 5
                    {
                        self.page_index = i32::try_from(index).unwrap_or(0);
                        break;
                    }
                }
                self.read_lead_in();
            }
        } else if self.in_data_delay > 0 {
            self.in_data_region = true;
            self.in_data_delay -= 1;
            if self.in_data_delay == 0 {
                self.byte_read_delay = FIRST_BYTE;
                if self.tape.wave.is_some() {
                    self.audio_frame = self.page().map(|page| u64::from(page.audio));
                    self.audio_phase = 0;
                }
                self.irq = true;
            }
        } else if self.byte_read_delay > 0 {
            self.byte_read_delay -= 1;
            if self.byte_read_delay == 0 {
                self.byte_read_delay = NEXT_BYTE;
                self.page_position += 1;
                if self.page_position >= self.page_len() {
                    self.page_found = false;
                    self.in_data_region = false;
                    self.motor_disabled = true;
                }
                if self.irq_enabled {
                    self.irq = true;
                }
            }
        }
    }

    /// `MixAudio`: while the motor runs, the tape's audio advances at its own rate.
    fn step_audio(&mut self, image: &[u8]) {
        let (Some(frame), Some(wave)) = (self.audio_frame, self.tape.wave) else {
            return;
        };
        if self.motor_disabled {
            self.level = 0;
            return;
        }
        self.audio_phase += u64::from(wave.rate);
        let mut frame = frame;
        while self.audio_phase >= CPU_CLOCK {
            self.audio_phase -= CPU_CLOCK;
            frame += 1;
        }
        let sample = wave.sample(image, frame);
        self.level = sample.unwrap_or(0);
        self.audio_frame = sample.map(|_| frame);
    }

    /// The level the mixer hears: the tape while the motor runs.
    pub(crate) const fn audio_level(&self) -> i32 {
        if self.motor_disabled || self.audio_frame.is_none() {
            0
        } else {
            self.level
        }
    }

    /// `ReadRegister` at `$4200-$4203`, with its side effects.
    pub(crate) fn read_register(&mut self, image: &[u8], address: u16) -> u8 {
        let value = self.peek_register(image, address);
        match address {
            0x4200 => self.irq = false,
            0x4201 => self.page_found = false,
            _ => {}
        }
        value
    }

    /// The same without side effects.
    pub(crate) fn peek_register(&self, image: &[u8], address: u16) -> u8 {
        match address {
            // The page's byte; after command `$86` games expect one `$AA` before the `$C5`
            // header.
            0x4200 => {
                if self.page_position >= 0 && self.page_position < self.page_len() {
                    self.page()
                        .and_then(|page| {
                            image.get(page.data.0 + usize::try_from(self.page_position).ok()?)
                        })
                        .copied()
                        .unwrap_or(0xAA)
                } else {
                    0xAA
                }
            }
            0x4201 => {
                (if self.in_data_region { 0x20 } else { 0 })
                    | (if self.page_found { 0x40 } else { 0 })
                    | (if self.enable_decoder { 0x80 } else { 0 })
            }
            0x4202 if self.ready_for_bit => 0x40,
            _ => 0,
        }
    }

    /// `WriteRegister` at `$4200-$4203`.
    pub(crate) fn write_register(&mut self, address: u16, value: u8) {
        match address {
            0x4200 => self.reg_4200 = value,
            0x4201 => self.prg_page = value,
            0x4202 => self.write_drive(value),
            _ => {}
        }
    }

    /// `$4202`, the tape drive's control: D4's falling edge clocks D7 into the command, D5's
    /// resets the drive, D0 enables the decoder and D1 the IRQ.
    fn write_drive(&mut self, value: u8) {
        if self.reg_4202 & 0x10 != 0 && value & 0x10 == 0 {
            self.command = (self.command << 1) | (value >> 7);
            self.command_counter += 1;
            if self.command_counter == 8 {
                self.command_counter = 0;
                self.run_command();
            }
        }
        if value & 0x10 != 0 {
            self.ready_for_bit = false;
            self.process_bit_delay = 100;
        }
        if self.reg_4202 & 0x20 != 0 && value & 0x20 == 0 {
            self.command = 0;
            self.command_counter = 0;
            self.ready_for_bit = true;
        }
        if (value ^ self.reg_4202) & 0x04 != 0 {
            self.audio_enabled = value & 0x04 == 0;
        }
        self.reg_4202 = value;
        self.enable_decoder = value & 0x01 != 0;
        self.irq_enabled = value & 0x02 != 0;
        self.irq = false;
    }

    /// A complete 8-bit drive command: forward or back `n` pages, back to this page's start, or
    /// `$86`, the next page.
    fn run_command(&mut self) {
        let current = i16::from(self.current_page);
        match self.command {
            1..=0x3F => {
                self.seek_page = i16::from(self.command) + current;
                self.seek_page_delay = SEEK_DELAY;
                self.motor_disabled = false;
            }
            0x41..=0x7F => {
                self.seek_page = -(i16::from(self.command) - 0x40) + current;
                self.seek_page_delay = SEEK_DELAY;
                self.motor_disabled = false;
            }
            0 => {
                self.seek_page = current;
                self.current_page = self.current_page.wrapping_sub(1);
                self.seek_page_delay = SEEK_DELAY;
                self.motor_disabled = false;
            }
            0x86 => {
                // The next page, or back to the tape's start after the last (as Mesen does).
                let last = i32::try_from(self.tape.pages.len()).unwrap_or(1) - 1;
                self.page_index = if self.page_index < last {
                    self.page_index + 1
                } else {
                    0
                };
                self.read_lead_in();
            }
            _ => {}
        }
    }

    /// The work-RAM offset of `$4400-$7FFF`: page 8 at `$4400-$4FFF`, `$4200` D0-D2 + 8 at
    /// `$5000`, `$4200` D6-D7 × 2 at `$6000` and the next page at `$7000`.
    pub(crate) fn ram_offset(&self, address: u16) -> Option<usize> {
        let page = match address {
            0x4400..=0x4FFF => 8,
            0x5000..=0x5FFF => usize::from(self.reg_4200 & 0x07) + 8,
            0x6000..=0x6FFF => usize::from((self.reg_4200 & 0xC0) >> 5),
            0x7000..=0x7FFF => usize::from((self.reg_4200 & 0xC0) >> 5) + 1,
            _ => return None,
        };
        Some(page * 0x1000 + usize::from(address & 0x0FFF))
    }

    /// The BIOS offset of `$8000-$FFFF`: `$4201`'s 16 KiB page, then page 0.
    pub(crate) fn prg_offset(&self, address: u16) -> usize {
        let page = if address < 0xC000 {
            usize::from(self.prg_page)
        } else {
            0
        };
        (page * 0x4000 + usize::from(address & 0x3FFF)) % BIOS_SIZE
    }

    pub(crate) fn save_state(&self, writer: &mut StateWriter) {
        writer.write_raw(&[
            self.reg_4200,
            self.prg_page,
            self.reg_4202,
            self.command_counter,
            self.command,
            self.current_page,
        ]);
        for flag in [
            self.ready_for_bit,
            self.enable_decoder,
            self.audio_enabled,
            self.motor_disabled,
            self.irq_enabled,
            self.page_found,
            self.in_data_region,
            self.irq,
        ] {
            writer.write_bool(flag);
        }
        writer.write_u16(self.process_bit_delay);
        writer.write_i16(self.seek_page);
        writer.write_u32(self.seek_page_delay);
        writer.write_u16(self.byte_read_delay);
        writer.write_i32(self.page_index);
        writer.write_i32(self.page_position);
        writer.write_u32(self.in_data_delay);
        writer.write_i64(
            self.audio_frame
                .map_or(-1, |frame| i64::try_from(frame).unwrap_or(-1)),
        );
        writer.write_u64(self.audio_phase);
        writer.write_i32(self.level);
    }

    pub(crate) fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        let bytes = reader.read_raw(6)?;
        [
            self.reg_4200,
            self.prg_page,
            self.reg_4202,
            self.command_counter,
            self.command,
            self.current_page,
        ] = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]];
        for flag in [
            &mut self.ready_for_bit,
            &mut self.enable_decoder,
            &mut self.audio_enabled,
            &mut self.motor_disabled,
            &mut self.irq_enabled,
            &mut self.page_found,
            &mut self.in_data_region,
            &mut self.irq,
        ] {
            *flag = reader.read_bool()?;
        }
        self.process_bit_delay = reader.read_u16()?;
        self.seek_page = reader.read_i16()?;
        self.seek_page_delay = reader.read_u32()?;
        self.byte_read_delay = reader.read_u16()?;
        self.page_index = reader.read_i32()?;
        self.page_position = reader.read_i32()?;
        self.in_data_delay = reader.read_u32()?;
        let frame = reader.read_i64()?;
        self.audio_frame = u64::try_from(frame).ok();
        self.audio_phase = reader.read_u64()?;
        self.level = reader.read_i32()?;
        Ok(())
    }
}
