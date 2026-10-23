//! Savestate of the PPU (REQ-CORE-090, REQ-CORE-091): the registers, the memories, the
//! rendering pipeline and the tile records of the frame.

use super::Ppu2C02;
use crate::state::{InvalidState, StateReader, StateWriter};

impl Ppu2C02 {
    /// Appends the PPU state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_blob(&self.vram);
        writer.write_blob(&self.oam);
        writer.write_blob(&self.palette);
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
        writer.write_bool(self.write_toggle);
        writer.write_u8(self.read_buffer);
        writer.write_u8(self.decay);
        writer.write_bool(self.skip_latch);
        for stamp in self.decay_stamp {
            writer.write_i64(stamp);
        }

        writer.write_bool(self.vblank_just_started);
        writer.write_bool(self.nmi_line_sample);

        self.save_render_state(writer);

        writer.write_i32(self.mask_x);
        writer.write_i32(self.mask_y);
        self.tiles.save_state(writer);
    }

    /// Restores the PPU state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or inconsistent.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_blob_into(&mut self.vram)?;
        reader.read_blob_into(&mut self.oam)?;
        reader.read_blob_into(&mut self.palette)?;
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
        self.write_toggle = reader.read_bool()?;
        self.read_buffer = reader.read_u8()?;
        self.decay = reader.read_u8()?;
        self.skip_latch = reader.read_bool()?;
        for stamp in &mut self.decay_stamp {
            *stamp = reader.read_i64()?;
        }

        self.vblank_just_started = reader.read_bool()?;
        self.nmi_line_sample = reader.read_bool()?;

        self.load_render_state(reader)?;

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
