//! Savestate of the PPU (REQ-CORE-090, REQ-CORE-091): the registers, the memories, the
//! rendering pipeline and the tile records of the frame.

use super::Ppu2C02;
use crate::state::{InvalidState, StateReader, StateWriter};

impl Ppu2C02 {
    /// Appends the PPU state to `writer` (REQ-CORE-090).
    ///
    /// The block is the machine's half followed by the frame's half, in that order; a reader
    /// that wants both calls [`Ppu2C02::load_state`], which reads them the same way. The split
    /// exists for the in-memory state of AMD-041 REQ-CORE-121, which keeps the machine alone —
    /// every field still has exactly one definition, so the two paths cannot drift apart and the
    /// savestate format is unchanged.
    pub fn save_state(&self, writer: &mut StateWriter) {
        self.save_machine_state(writer);
        self.save_frame_records(writer);
    }

    /// Restores the PPU state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or inconsistent.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.load_machine_state(reader)?;
        self.load_frame_records(reader)
    }

    /// The half of the PPU state the emulation reads back: the memories, the registers, the
    /// scan position and the rendering pipeline (AMD-041 REQ-CORE-121).
    pub fn save_machine_state(&self, writer: &mut StateWriter) {
        writer.write_blob(&self.vram);
        writer.write_blob(&self.oam);
        writer.write_blob(&self.palette_raw);
        writer.write_i32(self.dot);
        writer.write_i32(self.scanline);
        writer.write_i32(self.dot_accumulator);
        writer.write_bool(self.odd_frame);
        writer.write_i64(self.ppu_clock);
        writer.write_u8(self.control);
        writer.write_u8(self.mask);
        writer.write_bool(self.vblank_flag);
        writer.write_bool(self.sprite_zero_hit);
        writer.write_bool(self.sprite_overflow);
        writer.write_bool(self.suppress_vblank);
        writer.write_u8(self.oam_address);
        writer.write_u16(self.v);
        writer.write_u16(self.t);
        writer.write_u16(self.v_row_start);
        writer.write_u16(self.v_row_next);
        writer.write_u8(self.fine_x);
        // Bit 1: the reset line's write window (lot FIDELITY step 34), in the byte that held the
        // toggle alone before, so older states load with the window open.
        // Bits 2 to 7: the top six bits of the latch's last address (step 39); its two low bits
        // are bits 1 and 2 of the frame-skip byte below. Older states load with zeros there.
        writer.write_u8(
            u8::from(self.write_toggle) | (u8::from(self.after_reset) << 1) | (self.ale_low & 0xFC),
        );
        writer.write_u8(self.read_buffer);
        writer.write_u8(self.decay);
        writer.write_u8(u8::from(self.skip_latch) | ((self.ale_low & 0x03) << 1));
        for stamp in self.decay_stamp {
            writer.write_i64(stamp);
        }

        writer.write_bool(self.vblank_just_started);
        writer.write_bool(self.nmi_line_sample);

        self.save_render_state(writer);
    }

    /// The half of the PPU state that *is* the frame just run: the unreliable edge bands and the
    /// tile records. Left out of the in-memory state of AMD-041 REQ-CORE-121 — the records of
    /// every frame are cleared unconditionally by [`Ppu2C02::begin_rendered_frame`] at dot 1 of
    /// its pre-render line, so the emulation never reads them back, and leaving them where they
    /// are is what makes the displayed frame of REQ-LOOP-067 the one the host sees.
    pub fn save_frame_records(&self, writer: &mut StateWriter) {
        writer.write_i32(self.mask_x);
        writer.write_i32(self.mask_y);
        self.tiles.save_state(writer);
    }

    /// Restores the half [`Ppu2C02::save_machine_state`] wrote.
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or inconsistent.
    pub fn load_machine_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.sprite_line_stale = true;
        reader.read_blob_into(&mut self.vram)?;
        reader.read_blob_into(&mut self.oam)?;
        reader.read_blob_into(&mut self.palette_raw)?;
        // AMD-050 REQ-CORE-049: the shadow palette is derived, so it is not in the blob (the
        // state of a 1.x savestate has to keep reading) and is rebuilt here instead. **The blob
        // carries palette RAM as the game wrote it**, which is what it always carried: the
        // shown palette of lot MF-VS step 2 is the second derived view and is rebuilt with the
        // first, so the format is unchanged and a NES savestate is byte for byte the same.
        self.refresh_palette_views();
        self.dot = reader.read_i32()?;
        self.scanline = reader.read_i32()?;
        self.dot_accumulator = reader.read_i32()?;
        self.odd_frame = reader.read_bool()?;
        self.ppu_clock = reader.read_i64()?;
        self.control = reader.read_u8()?;
        self.mask = reader.read_u8()?;
        self.vblank_flag = reader.read_bool()?;
        self.sprite_zero_hit = reader.read_bool()?;
        self.sprite_overflow = reader.read_bool()?;
        self.suppress_vblank = reader.read_bool()?;
        self.oam_address = reader.read_u8()?;
        self.v = reader.read_u16()?;
        self.t = reader.read_u16()?;
        self.v_row_start = reader.read_u16()?;
        self.v_row_next = reader.read_u16()?;
        self.fine_x = reader.read_u8()?;
        let toggle = reader.read_u8()?;
        self.write_toggle = (toggle & 1) != 0;
        self.after_reset = (toggle & 2) != 0;
        let ale_high = toggle & 0xFC;
        self.read_buffer = reader.read_u8()?;
        // Not in the format, and nothing is lost: the window in which a `$2007` read still sees
        // the byte before the last refill is shorter than two CPU cycles, and the only pair of
        // `$2007` reads that close together is the dummy read and the read of one indexed
        // instruction — which no save point can fall inside, a state being written between
        // instructions. `read_buffer_prev` is only looked at inside that window.
        self.read_buffer_clock = 0;
        self.decay = reader.read_u8()?;
        let skip = reader.read_u8()?;
        self.skip_latch = (skip & 1) != 0;
        self.ale_low = ale_high | ((skip >> 1) & 0x03);
        for stamp in &mut self.decay_stamp {
            *stamp = reader.read_i64()?;
        }
        // Not in the format: the OAM decay stamps (`refresh_oam_row`) restart from the clock
        // restored, so a row cannot decay before 8 frames after a load.
        self.restamp_oam_rows();

        self.vblank_just_started = reader.read_bool()?;
        self.nmi_line_sample = reader.read_bool()?;

        self.load_render_state(reader)
    }

    /// Restores the half [`Ppu2C02::save_frame_records`] wrote.
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or inconsistent.
    pub fn load_frame_records(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mask_x = reader.read_i32()?;
        self.mask_y = reader.read_i32()?;
        self.tiles.load_state(reader)
    }

    fn save_render_state(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.secondary_oam);
        writer.write_raw(&self.sprite_pattern_low);
        writer.write_raw(&self.sprite_pattern_high);
        writer.write_raw(&self.sprite_attribute);
        writer.write_raw(&self.sprite_x);

        writer.write_u8(self.nametable_latch);
        writer.write_u8(self.attribute_latch);
        writer.write_u8(self.pattern_low_latch);
        writer.write_u8(self.pattern_high_latch);
        writer.write_u16(self.pattern_shift_low);
        writer.write_u16(self.pattern_shift_high);
        writer.write_u16(self.attribute_shift_low);
        writer.write_u16(self.attribute_shift_high);

        writer.write_u8(self.oam_eval_latch);
        writer.write_i32(self.eval_sprite);
        writer.write_i32(self.eval_byte);
        writer.write_i32(self.eval_found);
        writer.write_i32(self.overflow_copy);
        writer.write_bool(self.eval_finished);
        writer.write_bool(self.sprite_zero_found);
        writer.write_i32(self.sprites_loaded);
        writer.write_bool(self.sprite_zero_loaded);
    }

    fn load_render_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.secondary_oam)?;
        reader.read_into(&mut self.sprite_pattern_low)?;
        reader.read_into(&mut self.sprite_pattern_high)?;
        reader.read_into(&mut self.sprite_attribute)?;
        reader.read_into(&mut self.sprite_x)?;

        self.nametable_latch = reader.read_u8()?;
        self.attribute_latch = reader.read_u8()?;
        self.pattern_low_latch = reader.read_u8()?;
        self.pattern_high_latch = reader.read_u8()?;
        self.pattern_shift_low = reader.read_u16()?;
        self.pattern_shift_high = reader.read_u16()?;
        self.attribute_shift_low = reader.read_u16()?;
        self.attribute_shift_high = reader.read_u16()?;

        self.oam_eval_latch = reader.read_u8()?;
        self.eval_sprite = reader.read_i32()?;
        self.eval_byte = reader.read_i32()?;
        self.eval_found = reader.read_i32()?;
        self.overflow_copy = reader.read_i32()?;
        self.eval_finished = reader.read_bool()?;
        self.sprite_zero_found = reader.read_bool()?;
        self.sprites_loaded = reader.read_i32()?;
        self.sprite_zero_loaded = reader.read_bool()?;
        Ok(())
    }
}
