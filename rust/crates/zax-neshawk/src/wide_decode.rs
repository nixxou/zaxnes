//! Zaxnes's margin decoders on the NesHawk engine (lot NESHAWK-HACKS step 4).
//!
//! **Not a port of anything**: `zax_nes::ppu::Ppu2C02::decode_margins`, run on what this engine
//! keeps of a frame — the background rows the capture kept ([`crate::wide::CapturedRow`], each
//! with a copy of the pattern table and the nametable map it was drawn with) and the tile
//! records copied off the machine. The decoders themselves are `zax_nes::wide_decode`'s, the
//! very same code. It reads the machine and never writes it: decoding happens after the frame,
//! on the engine's copy of the records.
//!
//! Left out for now, as in [`crate::wide_picture`]: the vertical margins (`decode_extra_rows`),
//! the playfield's end for the smooth scrolling, and `hides_overscan`.

use zax_core_api::TileRecord;
use zax_nes::wide_decode::{BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

use crate::nes::Nes;
use crate::wide::{CapturedRow, WideFrame};

/// What a decoded frame adds to its picture.
#[derive(Debug, Clone, Copy)]
pub struct Decoded {
    /// The decoder's margin palette (sub-palettes 4-7 of its background records).
    pub margin_palette: Option<[u8; 16]>,
    /// Per band: the x the centre's PPU pixels start at (columns a decoder redraws on the left).
    pub redrawn_to: [i16; 30],
    /// Per band: the x the centre's PPU pixels stop at (columns redrawn on the right).
    pub redrawn_from: [i16; 30],
}

impl Default for Decoded {
    fn default() -> Self {
        Self {
            margin_palette: None,
            redrawn_to: [0; 30],
            redrawn_from: [256; 30],
        }
    }
}

/// `zax_nes`'s `column_x`: the screen x where column `column` starts, 0 at least.
#[allow(clippy::cast_possible_truncation)] // 0..256.
fn column_x(column: i32, fine_x: i32) -> i16 {
    ((8 * column) - fine_x).max(0) as i16
}

/// `Ppu2C02::rows_to_decode`: the kept rows, each followed by the strip down to the next kept
/// row when that one starts 9 to 15 lines below it.
fn rows_to_decode(rows: &[Option<CapturedRow>]) -> Vec<(usize, CapturedRow, bool)> {
    let mut out = Vec::with_capacity(32);
    for (index, captured) in rows.iter().enumerate() {
        let Some(captured) = captured else {
            continue;
        };
        out.push((index, captured.clone(), false));
        let Some(next_y) = rows[index + 1..].iter().flatten().map(|next| next.y).next() else {
            continue;
        };
        let strip = captured.y + 8;
        if next_y <= strip || next_y >= strip + 8 {
            continue;
        }
        let v = captured.row.v;
        let v = match (v >> 5) & 0x1F {
            29 => (v & !0x03E0) ^ 0x0800,
            31 => v & !0x03E0,
            _ => v + 0x20,
        };
        let row = BackgroundRow {
            line: captured.row.line + 8,
            v,
            ..captured.row
        };
        out.push((
            index,
            CapturedRow {
                row,
                y: strip,
                ..captured.clone()
            },
            true,
        ));
    }
    out
}

/// `Ppu2C02::decode_margins` on the frame just run: the decoded rows' margins replace their
/// nametable columns in `records`, `frame`'s visible rows widen to the whole margin on them.
#[allow(clippy::too_many_lines)] // One pass over the rows, its parts sharing their locals.
pub fn decode_margins(
    decoder: &dyn MarginDecoder,
    nes: &Nes,
    records: &mut Vec<TileRecord>,
    frame: &mut WideFrame,
    left_column_hidden: bool,
) -> Decoded {
    let mut decoded = Decoded::default();
    let Some(wide) = nes.b.wide.as_deref() else {
        return decoded;
    };
    let extra = frame.margin / 8;
    if extra == 0 {
        return decoded;
    }
    let base = nes.b.board.base();
    let memory = DecoderMemory {
        ram: &nes.b.ram,
        prg_ram: &base.wram,
        prg_rom: &base.rom,
        nametable: &|_| 0,
        nametable_written: &|_| 0,
    };
    let row_width = 33 + (2 * extra);
    let masked_column = decoder.decodes_masked_column() && left_column_hidden;
    let left_columns = decoder.redrawn_left_columns().clamp(0, 31);
    let first_centre = i32::from(masked_column).max(left_columns);
    let masking = decoder.masking_sprites(&memory);
    let first_right = masking.map_or(32, |sprites| i32::from(sprites.x.clamp(8, 248)) / 8);
    let first_right = first_right.min(32 - decoder.redrawn_right_columns().clamp(0, 31));
    let mut decoded_rows = [false; 31];
    let mut tiles = Vec::new();
    let mut added = Vec::new();
    decoded.margin_palette = decoder.margin_palette(&memory);
    let palette_mask = if decoded.margin_palette.is_some() {
        7
    } else {
        3
    };
    let mut decoded_above = None;
    let strips = decoder.decodes_split_strips();
    for (row_index, captured, strip) in rows_to_decode(&wide.rows) {
        if strip && (!strips || decoded_above != Some(row_index)) {
            continue;
        }
        let map = captured.map;
        let offset = move |address: u16| -> usize {
            let block = map[usize::from((address >> 10) & 3)];
            #[allow(clippy::cast_sign_loss)] // A block number, 0..4.
            let block = block as usize;
            ((block << 10) | usize::from(address & 0x03FF)) & 0x07FF
        };
        let ciram = &nes.b.ciram;
        let nametable = |address: u16| ciram[offset(address)];
        let stamps = &wide.vram_written;
        let written = |address: u16| stamps.get(offset(address)).copied().unwrap_or(0);
        let memory = DecoderMemory {
            nametable: &nametable,
            nametable_written: &written,
            ..memory
        };
        let fine_x = i32::from(captured.row.fine_x);
        let last = 32 + extra - i32::from(fine_x == 0);
        let mut columns = (-extra..first_centre).chain(first_right..=last);
        tiles.clear();
        if !decoder.decode_row(&memory, &captured.row, &mut columns, &mut tiles) {
            continue;
        }
        decoded_above = Some(row_index);
        decoded_rows[row_index] = true;
        if let Some(to) = decoded
            .redrawn_to
            .get_mut(row_index)
            .filter(|_| left_columns > 0)
        {
            *to = column_x(left_columns, fine_x);
        }
        if let Some(from) = decoded
            .redrawn_from
            .get_mut(row_index)
            .filter(|_| first_right < 32)
        {
            *from = column_x(first_right, fine_x);
        }
        for &DecodedTile {
            column,
            tile,
            palette,
        } in &tiles
        {
            let at = usize::from(tile) * 16;
            let mut plane0 = 0u64;
            let mut plane1 = 0u64;
            for row in 0..8 {
                plane0 |= u64::from(captured.chr[at + row]) << (8 * row);
                plane1 |= u64::from(captured.chr[at + 8 + row]) << (8 * row);
            }
            #[allow(clippy::cast_possible_truncation)] // -519..768.
            let x = ((8 * column) - fine_x) as i16;
            added.push(TileRecord::new(
                plane0,
                plane1,
                x,
                captured.y,
                (i32::try_from(row_index).unwrap_or(0) * row_width) + column + extra,
                false,
                false,
                false,
                palette & palette_mask,
                true,
            ));
        }
    }

    let placed = decoded_rows.iter().filter(|decoded| **decoded).count();
    if placed < decoder.minimum_rows() {
        return Decoded::default();
    }

    #[allow(clippy::cast_possible_truncation)] // At most 512.
    let margin = frame.margin as i16;
    for (range, _) in frame
        .visible_rows
        .iter_mut()
        .zip(decoded_rows)
        .filter(|(_, decoded)| *decoded)
    {
        *range = (-margin, 256 + margin);
    }
    records.retain(|record| {
        if !record.background {
            return true;
        }
        let row = usize::try_from(record.index / row_width).unwrap_or(usize::MAX);
        let column = (record.index % row_width) - extra;
        !decoded_rows.get(row).copied().unwrap_or(false)
            || (first_centre..first_right).contains(&column)
    });
    records.extend(added);
    if let Some(sprites) = masking {
        records.retain(|record| {
            record.background
                || record.x != sprites.x
                || record.index >= 128
                || !(sprites.first..=sprites.last).contains(&(record.index % 64))
        });
    }
    decoded
}
