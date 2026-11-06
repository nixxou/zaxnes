use super::mixer;
use crate::state::{InvalidState, StateReader, StateWriter};

/// Top index of [`mixer::NAMCO163_TABLE`], named so that the clamp in `refresh_level` reads as
/// what it is.
const LEVEL_MAX: usize = mixer::NAMCO163_LEVELS - 1;

/// The Namco 163's expansion audio: one to eight wavetable channels, time-multiplexed out of the
/// chip's own 128 bytes of RAM (REQ-CORE-075, optional; nesdev, "Namco 163 audio").
///
/// **The 128 bytes are not in here.** They are the last 128 bytes of the cartridge's PRG-RAM, so
/// that REQ-CORE-095's single battery slice carries them — see the note on
/// [`Namco163`](crate::cartridges::mappers::Namco163). Every method that needs them takes them,
/// which also keeps this type free of any copy that could go stale.
///
/// The chip has **no registers of its own** beyond the two this struct holds: the address port
/// and its auto-increment flag, written through `$F800`, and the sound-disable bit of `$E000`.
/// Everything else a channel has — its frequency, its phase, its waveform address and length and
/// its volume — is bytes $40-$7F of that RAM, which the game writes through the `$4800` data
/// port like any other byte of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Namco163Audio {
    /// `$F800`'s low seven bits: which byte of the chip's RAM `$4800` reads and writes.
    ram_position: u8,
    /// `$F800` bit 7: whether an access through `$4800` then steps the port on.
    auto_increment: bool,
    /// `$E000` bit 6, "Disable sound if set". It stops the multiplexer, so a channel keeps
    /// whatever output it last had and nothing advances.
    disabled: bool,

    /// CPU cycles since the last channel update. "It takes exactly 15 CPU cycles to update and
    /// output one channel."
    update_counter: u8,
    /// Which channel the multiplexer updates next, counting **down** from 7 as the chip does.
    current_channel: u8,
    /// Each channel's last output, `sample * volume`, held until its turn comes round again —
    /// which is what the time multiplexing means for the mix.
    ///
    /// Unipolar, where Mesen centres it on `(sample - 8) * volume`: the chip's own converter is
    /// unipolar, and it is what lets `M` stay positive, which [`mixer::clip`] depends on
    /// (AMD-061 §6.2 and the note on [`mixer::NAMCO163_TABLE`]).
    channel_output: [u16; Self::CHANNELS as usize],

    /// `M`, recomputed by `refresh_level` whenever a channel is updated and handed out by
    /// [`Self::output`] as a plain load.
    ///
    /// Derived from the outputs above and from `$7F`, and serialised all the same -- see
    /// [`Self::save_state`] for the one reason why.
    level: i32,
}

impl Namco163Audio {
    /// "Eight channels occupy addresses $40-$7F, with each channel using 8 bytes."
    pub const CHANNELS: u8 = 8;

    /// Where channel 0's eight bytes start: "Channel 1: $40-$47" through "Channel 8: $78-$7F",
    /// numbered 0 to 7 upwards from $40 here as Mesen numbers them.
    const CHANNEL_BASE: usize = 0x40;

    /// Bytes one channel's registers take, "each channel using 8 bytes".
    const CHANNEL_STRIDE: usize = 8;

    /// CPU cycles one channel's turn takes: "It takes exactly 15 CPU cycles to update and output
    /// one channel."
    pub const CYCLES_PER_CHANNEL: u8 = 15;

    /// A chip's sound at power-on (REQ-CORE-004).
    ///
    /// The multiplexer starts on channel 7, which is the one channel that is always enabled —
    /// "when C=0, only channel 8 enabled" — so the first update after power-on is a real one
    /// whatever `$7F` holds. Every output is 0, which is silence; the chip's RAM is PRG-RAM and
    /// the core clears it.
    pub fn new() -> Self {
        Self {
            ram_position: 0,
            auto_increment: false,
            disabled: false,
            update_counter: 0,
            current_channel: Self::CHANNELS - 1,
            channel_output: [0; Self::CHANNELS as usize],
            level: 0,
        }
    }

    /// Back to the power-on state (REQ-CORE-004).
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// `M` of REQ-CORE-073: the board's output in the mixer's integer units, a **positive**
    /// quantity the APU subtracts (AMD-061 §6.2).
    pub const fn output(&self) -> i32 {
        self.level
    }

    /// `$E000` bit 6, "Disable sound if set".
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// `$F800`: the RAM address port in the low seven bits and the auto-increment in bit 7.
    ///
    /// The same byte carries the Namco 163's PRG-RAM write protection, which is the board's
    /// business; this takes only the two fields the sound chip owns.
    pub fn set_ram_port(&mut self, value: u8) {
        self.ram_position = value & 0x7F;
        self.auto_increment = (value & 0x80) != 0;
    }

    /// A write through the `$4800` data port: the byte lands at the port's position, and the port
    /// steps on when bit 7 of `$F800` asked it to.
    pub fn write_ram(&mut self, ram: &mut [u8], value: u8) {
        if let Some(byte) = ram.get_mut(usize::from(self.ram_position)) {
            *byte = value;
        }

        self.step_port();
    }

    /// A read through the `$4800` data port. It auto-increments exactly as a write does, which is
    /// why the board implements `cpu_read_override` and `cpu_peek_override` separately
    /// (REQ-CORE-081).
    pub fn read_ram(&mut self, ram: &[u8]) -> u8 {
        let value = self.peek_ram(ram);
        self.step_port();
        value
    }

    /// The same byte with no side effect at all: the port is not moved (REQ-CORE-081).
    pub fn peek_ram(&self, ram: &[u8]) -> u8 {
        ram.get(usize::from(self.ram_position))
            .copied()
            .unwrap_or(0)
    }

    fn step_port(&mut self) {
        if self.auto_increment {
            self.ram_position = (self.ram_position + 1) & 0x7F;
        }
    }

    /// How many channels `$7F` enables, 1 to 8: "bits [6:4] control enabled channels", and
    /// "when C=0, only channel 8 enabled; when C=1, channels 8 and 7 enabled" and so on.
    ///
    /// `$7F` is the volume byte of channel 8 as well, which is why the field is read out of the
    /// RAM rather than cached: a game that changes the count changes nothing else, and a game
    /// that writes channel 8's volume changes the count with it.
    pub fn channel_count(ram: &[u8]) -> u8 {
        (ram.get(0x7F).copied().unwrap_or(0) >> 4 & 0x07) + 1
    }

    /// The lowest channel index the multiplexer visits, given how many `$7F` enables. The chip
    /// walks **downwards** from 7, so eight channels start at 0 and one channel never leaves 7.
    fn lowest_channel(ram: &[u8]) -> u8 {
        Self::CHANNELS - Self::channel_count(ram)
    }

    /// Advances the multiplexer by one CPU cycle.
    ///
    /// "It takes exactly 15 CPU cycles to update and output one channel", and with several
    /// enabled the chip cycles through them — so a channel's own update rate is 15 × *n* CPU
    /// cycles, which is the page's "higher channel counts result in lower update frequencies".
    /// The sound-disable bit of `$E000` stops the whole multiplexer, so every channel holds the
    /// output it last had.
    pub fn step_cpu_cycle(&mut self, ram: &mut [u8]) {
        if self.disabled {
            return;
        }

        self.update_counter += 1;
        if self.update_counter < Self::CYCLES_PER_CHANNEL {
            return;
        }

        self.update_counter = 0;
        self.update_channel(ram);

        // Downwards, and back to 7 once the lowest enabled channel has had its turn. The bound is
        // re-read each time because `$7F` is a byte of the RAM like any other and a game may
        // change the count between two turns.
        if self.current_channel <= Self::lowest_channel(ram) {
            self.current_channel = Self::CHANNELS - 1;
        } else {
            self.current_channel -= 1;
        }
    }

    /// Advances one channel's phase by its frequency and takes the sample the new phase lands on.
    ///
    /// The page's own arithmetic, field by field:
    ///
    /// * the frequency is 18 bits, `+4`'s low two bits over `+2` over `+0`, and "the 18-bit
    ///   frequency value is added to the 24-bit phase accumulator";
    /// * the phase is 24 bits, `+5` over `+3` over `+1`, and it is **stored back into the RAM** --
    ///   a game can read its own channel's phase through `$4800`, which is why the accumulator is
    ///   not a field of this struct;
    /// * the waveform length is "(64-L)*4 4-bit samples" from `+4`'s top six bits, which the page
    ///   also writes as "256 - %LLLLLL00";
    /// * "The high byte of the 24-bit phase value directly determines the current sample
    ///   position", modulo that length, plus `+6`'s waveform address "in 4-bit samples";
    /// * the sample is a nibble, "each step in the waveform is 4-bits wide" stored
    ///   "little-endian", so an even position is the low nibble;
    /// * the volume is `+7`'s low four bits, "Linear Volume".
    ///
    /// `(phase + freq) % (length << 16)` is the wrap the page's formula implies, and it is
    /// Mesen's line for line (`Namco163Audio.h:79-100`) -- a second source on the one point the
    /// page states as a formula rather than as prose.
    fn update_channel(&mut self, ram: &mut [u8]) {
        let base = Self::CHANNEL_BASE + usize::from(self.current_channel) * Self::CHANNEL_STRIDE;
        let Some(registers) = ram.get(base..base + Self::CHANNEL_STRIDE) else {
            return;
        };

        let frequency = (u32::from(registers[4] & 0x03) << 16)
            | (u32::from(registers[2]) << 8)
            | u32::from(registers[0]);
        let phase = (u32::from(registers[5]) << 16)
            | (u32::from(registers[3]) << 8)
            | u32::from(registers[1]);
        let length = 256 - u32::from(registers[4] & 0xFC);
        let address = registers[6];
        let volume = u16::from(registers[7] & 0x0F);

        // `length` is between 4 and 256 by construction -- `$FC` leaves the two low bits clear,
        // so 256 minus it is a positive multiple of four -- and the modulus is never zero. The
        // phase is therefore below `256 << 16`, which is what makes `bytes[2]` the whole of the
        // sample position and `bytes[3]` always zero.
        let phase = (phase + frequency) % (length << 16);
        let bytes = phase.to_le_bytes();
        let sample_position = bytes[2];
        ram[base + 1] = bytes[0];
        ram[base + 3] = bytes[1];
        ram[base + 5] = sample_position;

        let position = usize::from(sample_position.wrapping_add(address));
        // A position is a nibble index over the whole 128 bytes, so `position / 2` is in range by
        // construction: that is what keeps a bounds check off this path.
        let byte = ram[(position / 2) & (Self::RAM_SIZE - 1)];
        let sample = if (position & 0x01) == 0 {
            byte & 0x0F
        } else {
            byte >> 4
        };

        self.channel_output[usize::from(self.current_channel)] = u16::from(sample) * volume;
        self.refresh_level(ram);
    }

    /// The chip's own RAM, "$80 bytes of sound RAM shared by channel registers and wavetable
    /// samples". It is a power of two, which is what lets a nibble position index it without a
    /// bounds check.
    pub const RAM_SIZE: usize = 0x80;

    /// Appends the sound state to `writer` (REQ-CORE-090).
    ///
    /// The chip's 128 bytes are **not** written here: they are PRG-RAM, which
    /// `Cartridge::save_state` writes with the rest of it. What is here is the state that is not
    /// in that RAM.
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.ram_position);
        writer.write_bool(self.auto_increment);
        writer.write_bool(self.disabled);
        writer.write_u8(self.update_counter);
        writer.write_u8(self.current_channel);
        for output in self.channel_output {
            writer.write_u16(output);
        }

        // `M` itself. It follows from the outputs above and from `$7F`, so it *is* derived --
        // but `$7F` is a byte of the chip's RAM and `Mapper::load_registers` is not given the
        // cartridge's memories, so recomputing it on load is not possible from there. One i32 in
        // the block is cheaper than threading the RAM through the trait for it, and it makes the
        // restored `M` exactly the saved one rather than one update late (REQ-CORE-091).
        writer.write_i32(self.level);
    }

    /// Restores the sound state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.ram_position = reader.read_u8()? & 0x7F;
        self.auto_increment = reader.read_bool()?;
        self.disabled = reader.read_bool()?;
        self.update_counter = reader.read_u8()?;
        self.current_channel = reader.read_u8()?;
        if self.current_channel >= Self::CHANNELS {
            return Err(InvalidState::corrupt(format!(
                "Corrupt savestate: Namco 163 channel {} is outside 0..{}.",
                self.current_channel,
                Self::CHANNELS
            )));
        }
        for output in &mut self.channel_output {
            *output = reader.read_u16()?;
        }

        self.level = reader.read_i32()?;
        Ok(())
    }

    /// Recomputes `M` from the channel outputs: "simply sum the channel outputs, and divide the
    /// output volume by the number of active channels".
    ///
    /// The enabled channels are the **top** ones -- "when C=0, only channel 8 enabled" -- so the
    /// sum runs from [`Self::lowest_channel`] up to 7, and a channel the count has disabled
    /// contributes nothing even though its own registers still hold the output it had.
    ///
    /// The page notes that dividing by the count goes "slightly too loud" from six channels up,
    /// because it does not account for the energy the serial cycling moves to high frequencies.
    /// It is the approximation the page itself recommends, it is Mesen's
    /// (`Namco163Audio.h:102-112`), and the alternative is a filter this mixer does not have;
    /// **Q-127** records it together with the level.
    fn refresh_level(&mut self, ram: &[u8]) {
        let count = Self::channel_count(ram);
        let mut sum = 0;
        for channel in Self::lowest_channel(ram)..Self::CHANNELS {
            sum += usize::from(self.channel_output[usize::from(channel)]);
        }

        // The divided sum is at most one channel's full swing, 15 x 15, by construction; the
        // clamp is what makes the index provably in range and keeps a bounds check off this path
        // (the trap MF-4d step 2 met in `NesBus::write_internal`).
        let step = (sum / usize::from(count)).min(LEVEL_MAX);
        self.level = mixer::NAMCO163_TABLE[step];
    }
}
