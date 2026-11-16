//! Lot FDS: the Famicom Disk System — the RAM adapter (32 KiB of work RAM at `$6000-$DFFF`, 8 KiB
//! of CHR-RAM, the user's 8 KiB BIOS at `$E000`), its disk drive, its IRQ timer and its wavetable
//! sound.
//!
//! Read from Mesen 2 (`Core/NES/Mappers/FDS/Fds.cpp`, `FdsAudio.cpp`, `ModChannel.h`,
//! `BaseFdsChannel.h`, `FdsInputButtons.cpp` and `Loaders/FdsLoader.cpp`; GPL v3; read, not
//! copied). A `.fds` (with or without its fwNES header) or `.qd` image and `disksys.rom` are
//! packaged by [`package`] as a NES 2.0 image of mapper 20 (nesdev: "reserved for FDS images"):
//! the BIOS as PRG-ROM, the disk image as the misc ROM, submapper 1 for the QD layout. A bare
//! iNES 20 stays refused. The board rebuilds each side with its gaps, start marks and CRCs as the
//! drive sees it, and keeps what the games write as a difference from the original
//! ([`Fds::export_save`]), which the host keeps beside the ROM like a flash save.
//!
//! Disks change as in Mesen: by hand ([`Fds::insert`], [`Fds::eject`]; a disk in the drive is
//! taken out a frame, then the new one goes in [`REINSERT_FRAMES`] later), and automatically: when
//! the BIOS looks for a disk (`$E445`) the side whose header matches is put in, and when a game
//! keeps polling `$4032` after a side was read through, the disk is taken out and put back.
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// The BIOS's size (`disksys.rom`).
pub const BIOS_SIZE: usize = 0x2000;
/// The bytes of one side in a `.fds` and a `.qd` image.
const FDS_SIDE: usize = 65_500;
const QD_SIDE: usize = 0x1_0000;
/// No side in the drive.
const NO_DISK: u8 = 0xFF;
/// Frames between taking a disk out and putting the next one in by hand ("2 seconds").
pub const REINSERT_FRAMES: u16 = 120;

/// `WaveVolumeTable`: the master volume's four steps.
const WAVE_VOLUME: [u32; 4] = [36, 24, 17, 14];

/// The FDS's output as the mixer's `M`, for each of its 64 levels: a straight line whose top is
/// [`FDS_GAIN`] times an APU square at volume 15 (FIDELITY-LEADS J38).
pub(crate) const FDS_LEVELS: usize = 64;
/// How much louder than an APU square at full volume the FDS's full swing is (nesdev, "FDS
/// audio": about 2.4 times).
const FDS_GAIN: f64 = 2.4;

fn level_table() -> [i32; FDS_LEVELS] {
    use crate::apu::mixer::{pulse, round_to_even, NORMALISATION};
    #[allow(clippy::cast_precision_loss)] // FDS_LEVELS - 1 is 63.
    let step = NORMALISATION * pulse(15) * FDS_GAIN / (FDS_LEVELS - 1) as f64;
    #[allow(clippy::cast_precision_loss)] // level is at most 63.
    std::array::from_fn(|level| round_to_even(step * level as f64))
}

/// Why an FDS image cannot be packaged.
fn image_error(message: &str) -> String {
    format!("FDS image: {message}.")
}

/// Packages a disk image and `disksys.rom` as the NES 2.0 image this core runs: mapper 20, the
/// BIOS twice as 16 KiB of PRG-ROM, 32 KiB of work RAM, 8 KiB of CHR-RAM, vertical tables,
/// submapper 1 for a `.qd` image, and the disk image as the misc ROM.
///
/// # Errors
///
/// A BIOS of the wrong size or an image with no side, as a message for the user.
pub fn package(disk: &[u8], bios: &[u8], qd: bool) -> Result<Vec<u8>, String> {
    if bios.len() != BIOS_SIZE {
        return Err(format!(
            "disksys.rom must be {BIOS_SIZE} bytes, this one is {}.",
            bios.len()
        ));
    }
    if side_count(disk, qd) == 0 {
        return Err(image_error("no disk side"));
    }
    let mut image = Vec::with_capacity(16 + 2 * BIOS_SIZE + disk.len());
    // Mapper 20: D4-D7 of byte 6 ($14 with vertical tables), NES 2.0; work RAM 64 << 9 = 32 KiB,
    // CHR-RAM 64 << 7 = 8 KiB, one misc ROM.
    image.extend_from_slice(&[
        b'N',
        b'E',
        b'S',
        0x1A,
        0x01,
        0x00,
        0x41,
        0x18,
        u8::from(qd) << 4,
        0x00,
        0x09,
        0x07,
        0x00,
        0x00,
        0x01,
        0x00,
    ]);
    image.extend_from_slice(bios);
    image.extend_from_slice(bios);
    image.extend_from_slice(disk);
    Ok(image)
}

/// Whether an image's misc ROM can be an FDS disk image (mapper 20's one way to load).
#[must_use]
pub fn is_disk(misc: &[u8], qd: bool) -> bool {
    side_count(misc, qd) > 0
}

/// The sides of an image (`FdsLoader::LoadDiskData`): the fwNES header's count, else as many
/// whole sides as the file holds.
fn side_count(disk: &[u8], qd: bool) -> usize {
    let capacity = if qd { QD_SIDE } else { FDS_SIDE };
    if disk.starts_with(b"FDS\x1A") {
        usize::from(disk.get(4).copied().unwrap_or(0))
    } else {
        disk.len() / capacity
    }
}

/// One side as the drive sees it (`FdsLoader::AddGaps`): 28 300 bits of gap, then each block
/// after its `$80` start mark, followed by a CRC (`$4D $62`, a placeholder, in a `.fds`; the
/// image's own in a `.qd`) and 976 bits of gap.
fn add_gaps(side: &[u8], qd: bool) -> Vec<u8> {
    let capacity = side.len();
    let read = |at: usize| side.get(at).copied().unwrap_or(0);
    let mut out = vec![0u8; 28_300 / 8];
    let mut j = 0;
    while j < capacity {
        let mut length = match read(j) {
            1 => 56,
            2 => 2,
            3 => 16,
            4 => {
                1 + if qd {
                    usize::from(read(j.wrapping_sub(5)))
                        | (usize::from(read(j.wrapping_sub(4))) << 8)
                } else {
                    usize::from(read(j.wrapping_sub(3)))
                        | (usize::from(read(j.wrapping_sub(2))) << 8)
                }
            }
            _ => {
                // Unexpected data: the rest of the side as it is.
                out.push(0x80);
                out.extend_from_slice(&side[j..]);
                return out;
            }
        };
        if qd {
            length += 2;
        }
        if j + length >= capacity {
            return out;
        }
        out.push(0x80);
        out.extend_from_slice(&side[j..j + length]);
        if !qd {
            out.extend_from_slice(&[0x4D, 0x62]);
        }
        out.extend(std::iter::repeat_n(0, 976 / 8));
        j += length;
    }
    out
}

/// The sides of a disk image with their gaps, at least a side's capacity each, and the 56 bytes
/// of each side's disk header (for the automatic insertion).
fn load_sides(disk: &[u8], qd: bool) -> (Vec<Vec<u8>>, Vec<[u8; 56]>) {
    let capacity = if qd { QD_SIDE } else { FDS_SIDE };
    let (count, mut offset) = if disk.starts_with(b"FDS\x1A") {
        (usize::from(disk.get(4).copied().unwrap_or(0)), 16)
    } else {
        (disk.len() / capacity, 0)
    };
    let mut sides = Vec::with_capacity(count);
    let mut headers = Vec::with_capacity(count);
    for _ in 0..count {
        let mut raw = vec![0u8; capacity];
        if let Some(available) = disk.get(offset..) {
            let length = available.len().min(capacity);
            raw[..length].copy_from_slice(&available[..length]);
        }
        let mut header = [0u8; 56];
        header.copy_from_slice(&raw[1..57]);
        headers.push(header);
        let mut side = add_gaps(&raw, qd);
        if side.len() < capacity {
            side.resize(capacity, 0);
        }
        sides.push(side);
        offset += capacity;
    }
    (sides, headers)
}

/// `BaseFdsChannel`: an envelope (speed, gain, direction) and a 12-bit frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Channel {
    speed: u8,
    gain: u8,
    envelope_off: bool,
    increase: bool,
    frequency: u16,
    timer: u32,
    master_speed: u8,
}

impl Channel {
    const fn new() -> Self {
        Self {
            speed: 0,
            gain: 0,
            envelope_off: false,
            increase: false,
            frequency: 0,
            timer: 0,
            // "The BIOS initializes this to $E8."
            master_speed: 0xE8,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address & 0x03 {
            0 => {
                self.speed = value & 0x3F;
                self.increase = value & 0x40 != 0;
                self.envelope_off = value & 0x80 != 0;
                self.reset_timer();
                if self.envelope_off {
                    self.gain = self.speed;
                }
            }
            2 => self.frequency = (self.frequency & 0x0F00) | u16::from(value),
            3 => self.frequency = (self.frequency & 0x00FF) | (u16::from(value & 0x0F) << 8),
            _ => {}
        }
    }

    fn tick_envelope(&mut self) -> bool {
        if !self.envelope_off && self.master_speed > 0 {
            self.timer = self.timer.wrapping_sub(1);
            if self.timer == 0 {
                self.reset_timer();
                if self.increase && self.gain < 32 {
                    self.gain += 1;
                } else if !self.increase && self.gain > 0 {
                    self.gain -= 1;
                }
                return true;
            }
        }
        false
    }

    fn reset_timer(&mut self) {
        self.timer = 8 * (u32::from(self.speed) + 1) * u32::from(self.master_speed);
    }

    fn save(&self, writer: &mut StateWriter) {
        writer.write_raw(&[self.speed, self.gain, self.master_speed]);
        writer.write_bool(self.envelope_off);
        writer.write_bool(self.increase);
        writer.write_u16(self.frequency);
        writer.write_u32(self.timer);
    }

    fn load(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        let bytes = reader.read_raw(3)?;
        (self.speed, self.gain, self.master_speed) = (bytes[0], bytes[1], bytes[2]);
        self.envelope_off = reader.read_bool()?;
        self.increase = reader.read_bool()?;
        self.frequency = reader.read_u16()?;
        self.timer = reader.read_u32()?;
        Ok(())
    }
}

/// `ModChannel`'s own state beside its [`Channel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Modulator {
    channel: Channel,
    counter: i8,
    disabled: bool,
    table: [u8; 64],
    position: u8,
    overflow: u16,
    output: i32,
}

impl Modulator {
    /// `_modLut`, `ModReset` as `None`.
    const STEPS: [Option<i8>; 8] = [
        Some(0),
        Some(1),
        Some(2),
        Some(4),
        None,
        Some(-4),
        Some(-2),
        Some(-1),
    ];

    const fn new() -> Self {
        Self {
            channel: Channel::new(),
            counter: 0,
            disabled: false,
            table: [0; 64],
            position: 0,
            overflow: 0,
            output: 0,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x4084 | 0x4086 => self.channel.write(address, value),
            #[allow(clippy::cast_possible_wrap)]
            0x4085 => self.set_counter((value & 0x7F) as i8),
            0x4087 => {
                self.channel.write(address, value);
                self.disabled = value & 0x80 != 0;
                if self.disabled {
                    self.overflow = 0;
                }
            }
            _ => {}
        }
    }

    fn write_table(&mut self, value: u8) {
        if self.disabled {
            self.table[usize::from(self.position & 0x3F)] = value & 0x07;
            self.table[usize::from((self.position + 1) & 0x3F)] = value & 0x07;
            self.position = (self.position + 2) & 0x3F;
        }
    }

    /// `UpdateCounter`: a 7-bit signed value.
    fn set_counter(&mut self, value: i8) {
        let mut value = i16::from(value);
        if value >= 64 {
            value -= 128;
        } else if value < -64 {
            value += 128;
        }
        self.counter = i8::try_from(value).unwrap_or(0);
    }

    fn tick(&mut self) -> bool {
        let frequency = self.channel.frequency;
        if self.disabled || frequency == 0 {
            return false;
        }
        self.overflow = self.overflow.wrapping_add(frequency);
        if self.overflow < frequency {
            match Self::STEPS[usize::from(self.table[usize::from(self.position)] & 0x07)] {
                Some(step) => self.set_counter(self.counter.wrapping_add(step)),
                None => self.set_counter(0),
            }
            self.position = (self.position + 1) & 0x3F;
            return true;
        }
        false
    }

    /// `UpdateOutput` (nesdev, "FDS audio": the pitch modulation).
    fn update_output(&mut self, pitch: u16) {
        let counter = i32::from(self.counter);
        let mut temp = counter * i32::from(self.channel.gain);
        let remainder = temp & 0x0F;
        temp >>= 4;
        if remainder > 0 && temp & 0x80 == 0 {
            temp += if counter < 0 { -1 } else { 2 };
        }
        if temp >= 192 {
            temp -= 256;
        } else if temp < -64 {
            temp += 256;
        }
        temp *= i32::from(pitch);
        let remainder = temp & 0x3F;
        temp >>= 6;
        if remainder >= 32 {
            temp += 1;
        }
        self.output = temp;
    }

    fn save(&self, writer: &mut StateWriter) {
        self.channel.save(writer);
        writer.write_i8(self.counter);
        writer.write_bool(self.disabled);
        writer.write_raw(&self.table);
        writer.write_u8(self.position);
        writer.write_u16(self.overflow);
        writer.write_i32(self.output);
    }

    fn load(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.channel.load(reader)?;
        self.counter = reader.read_i8()?;
        self.disabled = reader.read_bool()?;
        self.table.copy_from_slice(reader.read_raw(64)?);
        self.position = reader.read_u8()? & 0x3F;
        self.overflow = reader.read_u16()?;
        self.output = reader.read_i32()?;
        Ok(())
    }
}

/// `FdsAudio`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Audio {
    wave: [u8; 64],
    wave_write: bool,
    volume: Channel,
    modulator: Modulator,
    disable_envelopes: bool,
    halt_wave: bool,
    master_volume: u8,
    wave_overflow: u16,
    wave_position: u8,
    output: u8,
    levels: [i32; FDS_LEVELS],
}

impl Audio {
    fn new() -> Self {
        Self {
            wave: [0; 64],
            wave_write: false,
            volume: Channel::new(),
            modulator: Modulator::new(),
            disable_envelopes: false,
            halt_wave: false,
            master_volume: 0,
            wave_overflow: 0,
            wave_position: 0,
            output: 0,
            levels: level_table(),
        }
    }

    /// `ClockAudio`, once per CPU cycle.
    fn clock(&mut self) {
        let frequency = self.volume.frequency;
        if !self.halt_wave && !self.disable_envelopes {
            self.volume.tick_envelope();
            if self.modulator.channel.tick_envelope() {
                self.modulator.update_output(frequency);
            }
        }
        if self.modulator.tick() {
            self.modulator.update_output(frequency);
        }
        self.update_output();
        let step = i32::from(frequency) + self.modulator.output;
        if !self.halt_wave && step > 0 {
            let step = u16::try_from(step & 0xFFFF).unwrap_or(0);
            self.wave_overflow = self.wave_overflow.wrapping_add(step);
            if self.wave_overflow < step {
                self.wave_position = (self.wave_position + 1) & 0x3F;
            }
        }
    }

    fn update_output(&mut self) {
        if self.wave_write {
            return;
        }
        let level = u32::from(self.volume.gain.min(32))
            * WAVE_VOLUME[usize::from(self.master_volume & 0x03)];
        let value = u32::from(self.wave[usize::from(self.wave_position)]) * level / 1152;
        self.output = u8::try_from(value.min(63)).unwrap_or(63);
    }

    fn level(&self) -> i32 {
        self.levels[usize::from(self.output) & (FDS_LEVELS - 1)]
    }

    fn read(&self, address: u16, open_bus: u8) -> u8 {
        let open = open_bus & 0xC0;
        match address {
            0x4040..=0x407F => {
                open | if self.wave_write {
                    self.wave[usize::from(address & 0x3F)]
                } else {
                    self.wave[usize::from(self.wave_position)]
                }
            }
            0x4090 => open | self.volume.gain,
            0x4092 => open | self.modulator.channel.gain,
            _ => open_bus,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x4040..=0x407F => {
                if self.wave_write {
                    self.wave[usize::from(address & 0x3F)] = value & 0x3F;
                }
            }
            0x4080 | 0x4082 => {
                self.volume.write(address, value);
                self.modulator.update_output(self.volume.frequency);
            }
            0x4083 => {
                self.disable_envelopes = value & 0x40 != 0;
                self.halt_wave = value & 0x80 != 0;
                if self.halt_wave {
                    self.wave_position = 0;
                }
                if self.disable_envelopes {
                    self.volume.reset_timer();
                    self.modulator.channel.reset_timer();
                }
                self.volume.write(address, value);
                self.modulator.update_output(self.volume.frequency);
            }
            0x4086 | 0x4087 => self.modulator.write(address, value),
            0x4084 | 0x4085 => {
                self.modulator.write(address, value);
                self.modulator.update_output(self.volume.frequency);
            }
            0x4088 => self.modulator.write_table(value),
            0x4089 => {
                self.master_volume = value & 0x03;
                self.wave_write = value & 0x80 != 0;
            }
            0x408A => {
                self.volume.master_speed = value;
                self.modulator.channel.master_speed = value;
            }
            _ => {}
        }
    }

    fn save(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.wave);
        writer.write_bool(self.wave_write);
        self.volume.save(writer);
        self.modulator.save(writer);
        writer.write_bool(self.disable_envelopes);
        writer.write_bool(self.halt_wave);
        writer.write_raw(&[self.master_volume, self.wave_position, self.output]);
        writer.write_u16(self.wave_overflow);
    }

    fn load(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.wave.copy_from_slice(reader.read_raw(64)?);
        self.wave_write = reader.read_bool()?;
        self.volume.load(reader)?;
        self.modulator.load(reader)?;
        self.disable_envelopes = reader.read_bool()?;
        self.halt_wave = reader.read_bool()?;
        let bytes = reader.read_raw(3)?;
        self.master_volume = bytes[0] & 0x03;
        self.wave_position = bytes[1] & 0x3F;
        self.output = bytes[2].min(63);
        self.wave_overflow = reader.read_u16()?;
        Ok(())
    }
}

/// The adapter and its drive (`Fds`, Mesen's field names).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct Fds {
    qd: bool,
    sides: Vec<Vec<u8>>,
    original: Vec<Vec<u8>>,
    headers: Vec<[u8; 56]>,
    audio: Audio,

    irq_reload: u16,
    irq_counter: u16,
    irq_enabled: bool,
    irq_repeat: bool,
    pub(crate) timer_irq: bool,
    pub(crate) disk_irq: bool,
    disk_regs: bool,
    sound_regs: bool,
    write_data: u8,
    motor_on: bool,
    reset_transfer: bool,
    read_mode: bool,
    crc_control: bool,
    disk_ready: bool,
    disk_irq_enabled: bool,
    pub(crate) horizontal: bool,
    ext_con: u8,
    bad_crc: bool,
    end_of_head: bool,
    read_data: u8,
    disk: u8,
    position: u32,
    delay: u32,
    crc: u16,
    previous_crc_control: bool,
    gap_ended: bool,
    scanning: bool,
    transfer_complete: bool,

    /// The automatic disk changes (`_autoDisk*`, frame counters) and the hand-made ones (a disk
    /// to put in after a delay).
    auto_insert: bool,
    auto_disabled: bool,
    auto_eject: i32,
    auto_switch: i32,
    restart_auto: i32,
    successive_checks: u32,
    frames_since_check: u32,
    previous_disk: u8,
    pending_insert: Option<(u8, u16)>,
    /// A read of `$E445` was seen: the BIOS looks for a disk; resolved once per frame.
    pub(crate) wants_disk: bool,
}

impl Fds {
    /// The adapter over the disk image in `image` from `misc`, side A of disk 1 in the drive.
    pub(crate) fn new(image: &[u8], misc: usize, qd: bool) -> Option<Self> {
        let disk = image.get(misc..)?;
        let (sides, headers) = load_sides(disk, qd);
        if sides.is_empty() {
            return None;
        }
        Some(Self {
            qd,
            original: sides.clone(),
            sides,
            headers,
            audio: Audio::new(),
            irq_reload: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_repeat: false,
            timer_irq: false,
            disk_irq: false,
            disk_regs: true,
            sound_regs: true,
            write_data: 0,
            motor_on: false,
            reset_transfer: false,
            read_mode: false,
            crc_control: false,
            disk_ready: false,
            disk_irq_enabled: false,
            horizontal: false,
            ext_con: 0,
            bad_crc: false,
            end_of_head: false,
            read_data: 0,
            disk: 0,
            position: 0,
            delay: 0,
            crc: 0,
            previous_crc_control: false,
            gap_ended: true,
            scanning: false,
            transfer_complete: false,
            auto_insert: true,
            auto_disabled: false,
            auto_eject: -1,
            auto_switch: -1,
            restart_auto: -1,
            successive_checks: 0,
            frames_since_check: u32::MAX,
            previous_disk: NO_DISK,
            pending_insert: None,
            wants_disk: false,
        })
    }

    /// `Reset(true)`: the automatic changes start over.
    pub(crate) fn soft_reset(&mut self) {
        self.auto_eject = -1;
        self.auto_switch = -1;
        self.auto_disabled = false;
    }

    pub(crate) const fn mirroring(&self) -> NametableMirroring {
        if self.horizontal {
            NametableMirroring::Horizontal
        } else {
            NametableMirroring::Vertical
        }
    }

    /// The number of sides.
    pub(crate) fn side_count(&self) -> usize {
        self.sides.len()
    }

    /// The side in the drive, `None` when it is empty.
    pub(crate) fn current_side(&self) -> Option<usize> {
        (self.disk != NO_DISK).then_some(usize::from(self.disk))
    }

    /// Takes the disk out.
    pub(crate) fn eject(&mut self) {
        self.disk = NO_DISK;
        self.pending_insert = None;
    }

    /// Puts `side` in (`FdsInputButtons::InsertDisk`): at once when the drive is empty, else the
    /// disk in it is taken out and `side` goes in [`REINSERT_FRAMES`] later.
    pub(crate) fn insert(&mut self, side: usize) {
        let Some(side) = u8::try_from(side)
            .ok()
            .filter(|side| usize::from(*side) < self.sides.len())
        else {
            return;
        };
        if self.disk == NO_DISK {
            self.disk = side;
            self.pending_insert = None;
        } else {
            self.disk = NO_DISK;
            self.pending_insert = Some((side, REINSERT_FRAMES));
        }
    }

    /// Turns the automatic disk changes on or off (`FdsAutoInsertDisk`).
    pub(crate) fn set_auto_insert(&mut self, enabled: bool) {
        self.auto_insert = enabled;
    }

    const fn auto_enabled(&self) -> bool {
        self.auto_insert && !self.auto_disabled
    }

    /// Once per frame: the delayed hand insertion, the BIOS's disk search (`header` the 10 bytes
    /// the BIOS compares, read by the core where `$E445`'s caller points) and Mesen's automatic
    /// eject/insert counters (`ProcessAutoDiskInsert`).
    pub(crate) fn end_frame(&mut self, header: Option<[u8; 10]>) {
        self.frames_since_check = self.frames_since_check.saturating_add(1);
        if let Some((side, frames)) = self.pending_insert {
            if frames <= 1 {
                self.pending_insert = None;
                if self.disk == NO_DISK {
                    self.disk = side;
                }
            } else {
                self.pending_insert = Some((side, frames - 1));
            }
        }
        if let Some(header) = header.filter(|_| self.auto_enabled()) {
            self.auto_find(&header);
        }
        self.wants_disk = false;
        if !self.auto_enabled() {
            return;
        }
        if self.auto_eject > 0 {
            self.auto_eject -= 1;
        } else if self.auto_switch > 0 {
            self.auto_switch -= 1;
            if self.auto_switch == 0 {
                // A disk; the side the game wants goes in at `$E445`.
                if self.disk == NO_DISK {
                    self.disk = 0;
                }
                self.restart_auto = 200;
            }
        } else if self.restart_auto > 0 {
            self.restart_auto -= 1;
            if self.restart_auto == 0 {
                // The game did not read it: take it out and try again.
                self.previous_disk = NO_DISK;
                self.auto_eject = 34;
                self.auto_switch = -1;
            }
        }
    }

    /// The `$E445` match: the one side whose header bytes 14-23 match (`$FF` matches anything).
    fn auto_find(&mut self, wanted: &[u8; 10]) {
        let mut found = None;
        let mut matches = 0;
        for (side, header) in self.headers.iter().enumerate() {
            if wanted
                .iter()
                .zip(&header[14..24])
                .all(|(want, have)| *want == 0xFF || want == have)
            {
                matches += 1;
                found = (matches == 1).then_some(side);
            }
        }
        if matches > 1 {
            // Several sides match (unlicensed games): leave the changes to the player.
            self.auto_disabled = true;
        }
        if let Some(side) = found.and_then(|side| u8::try_from(side).ok()) {
            self.disk = side;
            self.previous_disk = side;
        }
        self.auto_switch = -1;
        self.restart_auto = -1;
    }

    /// Whether `address` is a BIOS read the board watches (`$E445`, the disk search).
    pub(crate) fn see_read(&mut self, address: u16) {
        if address == 0xE445 && self.auto_enabled() {
            self.wants_disk = true;
        }
    }

    /// `ProcessCpuClock`: the timer, the sound and the drive.
    pub(crate) fn step(&mut self) {
        if self.irq_enabled {
            if self.irq_counter == 0 {
                self.timer_irq = true;
                self.irq_counter = self.irq_reload;
                if !self.irq_repeat {
                    self.irq_enabled = false;
                }
            } else {
                self.irq_counter -= 1;
            }
        }
        self.audio.clock();
        self.step_drive();
    }

    fn step_drive(&mut self) {
        let Some(side) = self.current_side().filter(|_| self.motor_on) else {
            self.end_of_head = true;
            self.scanning = false;
            return;
        };
        if self.reset_transfer && !self.scanning {
            return;
        }
        if self.end_of_head {
            self.delay = 50_000;
            self.end_of_head = false;
            self.position = 0;
            self.gap_ended = false;
            return;
        }
        if self.delay > 0 {
            self.delay -= 1;
            return;
        }
        self.scanning = true;
        self.auto_eject = -1;
        self.auto_switch = -1;
        let position = usize::try_from(self.position).unwrap_or(usize::MAX);
        let mut need_irq = self.disk_irq_enabled;
        if self.read_mode {
            let data = self.sides[side].get(position).copied().unwrap_or(0);
            if !self.previous_crc_control {
                self.update_crc(data);
            }
            if !self.disk_ready {
                self.gap_ended = false;
                self.crc = 0;
                self.bad_crc = false;
            } else if data != 0 && !self.gap_ended {
                self.gap_ended = true;
                need_irq = false;
            }
            if self.gap_ended {
                self.transfer_complete = true;
                self.read_data = data;
                if need_irq {
                    self.disk_irq = true;
                }
            }
            if !self.previous_crc_control && self.crc_control {
                self.bad_crc = self.crc != 0;
            }
        } else {
            let mut data = 0;
            if !self.crc_control {
                self.transfer_complete = true;
                data = self.write_data;
                if need_irq {
                    self.disk_irq = true;
                }
            }
            if !self.disk_ready {
                data = 0;
                self.crc = 0;
            }
            if self.crc_control {
                data = self.crc.to_le_bytes()[0];
                self.crc >>= 8;
            } else {
                self.update_crc(data);
            }
            if let Some(byte) = self.sides[side].get_mut(position) {
                *byte = data;
            }
            self.gap_ended = false;
            self.bad_crc = false;
        }
        self.previous_crc_control = self.crc_control;
        self.position += 1;
        if usize::try_from(self.position).unwrap_or(usize::MAX) >= self.sides[side].len() {
            self.motor_on = false;
            // Kosodate Gokko's copier expects an IRQ at the end of the disk.
            if self.disk_irq_enabled {
                self.disk_irq = true;
            }
            self.auto_eject = 77;
        } else {
            // 149, not 150: Mesen's note on *Ai Senshi Nicol*.
            self.delay = 149;
        }
    }

    fn update_crc(&mut self, value: u8) {
        self.crc ^= u16::from(value);
        for _ in 0..8 {
            let carry = self.crc & 1 != 0;
            self.crc >>= 1;
            if carry {
                self.crc ^= 0x8408;
            }
        }
    }

    /// `WriteRegister`, `$4020-$409F`.
    pub(crate) fn write(&mut self, address: u16, value: u8) {
        if (!self.disk_regs && (0x4024..=0x4026).contains(&address))
            || (!self.sound_regs && address >= 0x4040)
        {
            return;
        }
        match address {
            0x4020 => self.irq_reload = (self.irq_reload & 0xFF00) | u16::from(value),
            0x4021 => self.irq_reload = (self.irq_reload & 0x00FF) | (u16::from(value) << 8),
            0x4022 => {
                self.irq_repeat = value & 0x01 != 0;
                self.irq_enabled = value & 0x02 != 0 && self.disk_regs;
                if self.irq_enabled {
                    self.irq_counter = self.irq_reload;
                } else {
                    self.timer_irq = false;
                }
            }
            0x4023 => {
                self.disk_regs = value & 0x01 != 0;
                self.sound_regs = value & 0x02 != 0;
                if !self.disk_regs {
                    self.irq_enabled = false;
                    self.timer_irq = false;
                    self.disk_irq = false;
                }
            }
            0x4024 => {
                self.write_data = value;
                self.transfer_complete = false;
                self.disk_irq = false;
            }
            0x4025 => {
                self.motor_on = value & 0x01 != 0;
                self.reset_transfer = value & 0x02 != 0;
                self.read_mode = value & 0x04 != 0;
                self.horizontal = value & 0x08 != 0;
                self.crc_control = value & 0x10 != 0;
                self.disk_ready = value & 0x40 != 0;
                self.disk_irq_enabled = value & 0x80 != 0;
                self.disk_irq = false;
            }
            0x4026 => self.ext_con = value,
            0x4040..=0x409F => self.audio.write(address, value),
            _ => {}
        }
    }

    /// `ReadRegister`'s value, without its side effects; `open_bus` the value an unanswered bit
    /// reads, `None` for an address the adapter does not answer.
    pub(crate) fn peek(&self, address: u16, open_bus: u8) -> Option<u8> {
        if self.sound_regs && (0x4040..=0x4092).contains(&address) {
            return Some(self.audio.read(address, open_bus));
        }
        if !self.disk_regs || !(0x4030..=0x4033).contains(&address) {
            return None;
        }
        let inserted = self.disk != NO_DISK;
        Some(match address {
            0x4030 => {
                (open_bus & 0x24)
                    | u8::from(self.timer_irq)
                    | (u8::from(self.transfer_complete) << 1)
                    | (u8::from(self.horizontal) << 3)
                    | (u8::from(self.qd && self.bad_crc) << 4)
            }
            0x4031 => self.read_data,
            0x4032 => {
                (open_bus & 0xF8)
                    | u8::from(!inserted)
                    | (u8::from(!inserted || !self.scanning) << 1)
                    | (u8::from(!inserted) << 2)
            }
            _ => self.ext_con,
        })
    }

    /// `ReadRegister`, with its side effects.
    pub(crate) fn read(&mut self, address: u16, open_bus: u8) -> Option<u8> {
        let value = self.peek(address, open_bus)?;
        match address {
            0x4030 => {
                self.transfer_complete = false;
                self.timer_irq = false;
                self.disk_irq = false;
            }
            0x4031 => {
                self.transfer_complete = false;
                self.disk_irq = false;
            }
            0x4032 if self.auto_enabled() => {
                self.successive_checks = if self.frames_since_check < 100 {
                    self.successive_checks + 1
                } else {
                    0
                };
                self.frames_since_check = 0;
                if self.successive_checks > 20 && self.auto_eject == 0 && self.auto_switch == -1 {
                    // The game asks for another disk: out now, in ~77 frames.
                    self.successive_checks = 0;
                    self.frames_since_check = u32::MAX;
                    self.auto_switch = 77;
                    self.previous_disk = self.disk;
                    self.disk = NO_DISK;
                }
            }
            _ => {}
        }
        Some(value)
    }

    pub(crate) fn audio_level(&self) -> i32 {
        self.audio.level()
    }

    /// What the games wrote, as the file the host keeps: `ZXFD`, a version, then for each side
    /// its changed 256-byte blocks (16-bit side, 16-bit block, 256 bytes); empty when nothing
    /// changed.
    pub(crate) fn export_save(&self) -> Vec<u8> {
        let blocks = self.changed_blocks();
        if blocks.is_empty() {
            return Vec::new();
        }
        let mut file = b"ZXFD\x01".to_vec();
        for (side, block, data) in blocks {
            file.extend_from_slice(&side.to_le_bytes());
            file.extend_from_slice(&block.to_le_bytes());
            file.extend_from_slice(data);
        }
        file
    }

    /// The 256-byte blocks that differ from the original image, as `(side, block, bytes)`; a
    /// side's last block may be shorter.
    fn changed_blocks(&self) -> Vec<(u16, u16, &[u8])> {
        let mut blocks = Vec::new();
        for (side, (now, before)) in self.sides.iter().zip(&self.original).enumerate() {
            for (block, (data, old)) in now.chunks(256).zip(before.chunks(256)).enumerate() {
                if data != old {
                    blocks.push((
                        u16::try_from(side).unwrap_or(0),
                        u16::try_from(block).unwrap_or(0),
                        data,
                    ));
                }
            }
        }
        blocks
    }

    /// Applies a file of [`Self::export_save`] over the original image; `false`, the disks
    /// untouched, when it is not one for these disks.
    pub(crate) fn import_save(&mut self, data: &[u8]) -> bool {
        let Some(mut body) = data.strip_prefix(b"ZXFD\x01".as_slice()) else {
            return false;
        };
        let mut sides = self.original.clone();
        while !body.is_empty() {
            if body.len() < 4 {
                return false;
            }
            let side = usize::from(u16::from_le_bytes([body[0], body[1]]));
            let block = usize::from(u16::from_le_bytes([body[2], body[3]]));
            let Some(target) = sides.get_mut(side) else {
                return false;
            };
            let start = block * 256;
            let end = (start + 256).min(target.len().max(start));
            let Some(slice) = target.get_mut(start..end) else {
                return false;
            };
            let length = slice.len();
            if length == 0 || body.len() < 4 + length {
                return false;
            }
            slice.copy_from_slice(&body[4..4 + length]);
            body = &body[4 + length..];
        }
        self.sides = sides;
        true
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn save_state(&self, writer: &mut StateWriter) {
        self.audio.save(writer);
        for value in [self.irq_reload, self.irq_counter, self.crc] {
            writer.write_u16(value);
        }
        for flag in [
            self.irq_enabled,
            self.irq_repeat,
            self.timer_irq,
            self.disk_irq,
            self.disk_regs,
            self.sound_regs,
            self.motor_on,
            self.reset_transfer,
            self.read_mode,
            self.crc_control,
            self.disk_ready,
            self.disk_irq_enabled,
            self.horizontal,
            self.bad_crc,
            self.end_of_head,
            self.previous_crc_control,
            self.gap_ended,
            self.scanning,
            self.transfer_complete,
            self.auto_insert,
            self.auto_disabled,
            self.wants_disk,
        ] {
            writer.write_bool(flag);
        }
        writer.write_raw(&[
            self.write_data,
            self.ext_con,
            self.read_data,
            self.disk,
            self.previous_disk,
        ]);
        writer.write_u32(self.position);
        writer.write_u32(self.delay);
        for value in [self.auto_eject, self.auto_switch, self.restart_auto] {
            writer.write_i32(value);
        }
        writer.write_u32(self.successive_checks);
        writer.write_u32(self.frames_since_check);
        match self.pending_insert {
            Some((side, frames)) => {
                writer.write_bool(true);
                writer.write_u8(side);
                writer.write_u16(frames);
            }
            None => writer.write_bool(false),
        }
        writer.write_blob(&self.export_save());
    }

    pub(crate) fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.audio.load(reader)?;
        self.irq_reload = reader.read_u16()?;
        self.irq_counter = reader.read_u16()?;
        self.crc = reader.read_u16()?;
        for flag in [
            &mut self.irq_enabled,
            &mut self.irq_repeat,
            &mut self.timer_irq,
            &mut self.disk_irq,
            &mut self.disk_regs,
            &mut self.sound_regs,
            &mut self.motor_on,
            &mut self.reset_transfer,
            &mut self.read_mode,
            &mut self.crc_control,
            &mut self.disk_ready,
            &mut self.disk_irq_enabled,
            &mut self.horizontal,
            &mut self.bad_crc,
            &mut self.end_of_head,
            &mut self.previous_crc_control,
            &mut self.gap_ended,
            &mut self.scanning,
            &mut self.transfer_complete,
            &mut self.auto_insert,
            &mut self.auto_disabled,
            &mut self.wants_disk,
        ] {
            *flag = reader.read_bool()?;
        }
        let bytes = reader.read_raw(5)?;
        (
            self.write_data,
            self.ext_con,
            self.read_data,
            self.disk,
            self.previous_disk,
        ) = (bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]);
        if self.disk != NO_DISK && usize::from(self.disk) >= self.sides.len() {
            return Err(InvalidState::corrupt(
                "Corrupt savestate: FDS side out of range.".to_owned(),
            ));
        }
        self.position = reader.read_u32()?;
        self.delay = reader.read_u32()?;
        self.auto_eject = reader.read_i32()?;
        self.auto_switch = reader.read_i32()?;
        self.restart_auto = reader.read_i32()?;
        self.successive_checks = reader.read_u32()?;
        self.frames_since_check = reader.read_u32()?;
        self.pending_insert = if reader.read_bool()? {
            let side = reader.read_u8()?;
            let frames = reader.read_u16()?;
            Some((side, frames))
        } else {
            None
        };
        let save = reader.read_blob()?;
        if save.is_empty() {
            self.sides.clone_from(&self.original);
        } else if !self.import_save(&save) {
            return Err(InvalidState::corrupt(
                "Corrupt savestate: FDS disk data.".to_owned(),
            ));
        }
        Ok(())
    }
}
