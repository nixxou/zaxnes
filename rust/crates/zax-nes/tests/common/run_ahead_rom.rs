//! The test vector ROM of AMD-041 REQ-LOOP-065: it shows, on the image of a frame, the value a
//! per-frame counter had at the **beginning** of that frame and the state of button A read
//! during that same frame.
//!
//! Built here from the specification text, like every other home-made test ROM of this
//! workspace. It is an NROM image (32 KiB of PRG, 8 KiB of CHR) whose CHR is the one of SPEC-02
//! §5.1: tile `n` has plane 0 = eight bytes equal to `n` and plane 1 = eight bytes equal to
//! `255 − n`. Writing the counter as the tile index of the top-left cell therefore prints the
//! counter's **bits** on the first eight pixels of the picture, and [`displayed`] reads them
//! back.
//!
//! One frame of the core runs from the start of a vblank to the start of the next one, so the NMI
//! handler of a frame runs first and the visible lines that follow render what it wrote: the
//! picture a frame produces is the one its own handler prepared, from the counter value it had on
//! entry and from the controller word that frame was given.
//!
//! The module is shared by the core tests of `zax-nes` and the loop tests of `zax-app`
//! (`#[path]`), so there is one definition of the vector.

#![allow(dead_code)] // Each test binary uses a part of it.

use zax_nes::NesCore;

/// Zero-page address holding the value the current picture shows.
pub const DISPLAYED_ADDRESS: u16 = 0x0010;

/// Zero-page address of the counter, incremented once per emulated frame.
pub const COUNTER_ADDRESS: u16 = 0x0011;

/// Zero-page address holding the A bit read during the current frame.
pub const BUTTON_ADDRESS: u16 = 0x0012;

/// Where the light the Zapper saw is recorded, one byte per emulated frame: `$08` when port 2
/// reported light at least once during the frame, 0 when it never did (REQ-LOOP-068).
pub const PROBE_LOG: u16 = 0x0200;

/// Zero-page index of the next entry of [`PROBE_LOG`].
pub const PROBE_INDEX_ADDRESS: u16 = 0x0013;

/// Zero-page accumulator of the light bit for the frame under way.
pub const PROBE_ACCUMULATOR_ADDRESS: u16 = 0x0014;

/// NES colour the pattern bits set to 1 are drawn in (palette entry 1).
const ONE_COLOUR: u8 = 0x21;

/// Reset handler, at `$8000`: the two-vblank warm-up of SPEC-02 §5.1, the palette, the picture
/// and the NMI on, then the main loop, which keeps reading port 2 so that the light the Zapper
/// sees is sampled on the visible lines and not only during the vblank (AMD-021 R2-006).
const RESET: [u8; 84] = [
    0x78, // $8000 SEI
    0xA2, 0xFF, //       LDX #$FF
    0x9A, //             TXS
    0xA9, 0x00, //       LDA #$00
    0x8D, 0x00, 0x20, // STA $2000      NMI off while the PPU warms up
    0x8D, 0x01, 0x20, // STA $2001      picture off
    0x85, 0x10, //       STA $10        displayed value
    0x85, 0x11, //       STA $11        counter
    0x85, 0x12, //       STA $12        A bit
    0x85, 0x13, //       STA $13        probe index
    0x85, 0x14, //       STA $14        probe accumulator
    0xAD, 0x02, 0x20, // L1: LDA $2002
    0x10, 0xFB, //       BPL L1         first vblank
    0xAD, 0x02, 0x20, // L2: LDA $2002
    0x10, 0xFB, //       BPL L2         second vblank: writes now stick
    0xA9, 0x3F, //       LDA #$3F
    0x8D, 0x06, 0x20, // STA $2006
    0xA9, 0x00, //       LDA #$00
    0x8D, 0x06, 0x20, // STA $2006      PPUADDR = $3F00
    0xA9, 0x0F, //       LDA #$0F
    0x8D, 0x07, 0x20, // STA $2007      universal backdrop
    0xA9, ONE_COLOUR, // LDA #$21
    0x8D, 0x07, 0x20, // STA $2007      pattern bits at 1
    0xA9, 0x11, //       LDA #$11
    0x8D, 0x07, 0x20, // STA $2007      pattern bits at 0
    0xA9, 0x30, //       LDA #$30
    0x8D, 0x07, 0x20, // STA $2007
    0xA9, 0x80, //       LDA #$80
    0x8D, 0x00, 0x20, // STA $2000      NMI on, patterns at $0000
    0xA9, 0x0A, //       LDA #$0A
    0x8D, 0x01, 0x20, // STA $2001      background on, leftmost column shown
    0xAD, 0x17, 0x40, // $8048 LDA $4017   port 2, the Zapper's byte
    0x29, 0x08, //       AND #$08       the light bit
    0x05, 0x14, //       ORA $14
    0x85, 0x14, //       STA $14        light seen at any point this frame
    0x4C, 0x48, 0x80, // JMP $8048
];

/// NMI handler, at `$9000`: print the counter, print the A bit the controller reports for this
/// frame, increment the counter, put the scroll back, and log the light of the frame that ends.
const NMI: [u8; 76] = [
    0xA9, 0x20, //       LDA #$20
    0x8D, 0x06, 0x20, // STA $2006
    0xA9, 0x00, //       LDA #$00
    0x8D, 0x06, 0x20, // STA $2006      PPUADDR = $2000
    0xA5, 0x11, //       LDA $11        the counter, as it is on entry
    0x85, 0x10, //       STA $10        that is the value this picture shows
    0x8D, 0x07, 0x20, // STA $2007      tile index of the top-left cell
    0xE6, 0x11, //       INC $11        one increment per emulated frame
    0xA9, 0x01, //       LDA #$01
    0x8D, 0x16, 0x40, // STA $4016      strobe
    0xA9, 0x00, //       LDA #$00
    0x8D, 0x16, 0x40, // STA $4016
    0xAD, 0x16, 0x40, // LDA $4016      first read: button A
    0x29, 0x01, //       AND #$01
    0x85, 0x12, //       STA $12
    0xA9, 0x20, //       LDA #$20
    0x8D, 0x06, 0x20, // STA $2006
    0xA9, 0x01, //       LDA #$01
    0x8D, 0x06, 0x20, // STA $2006      PPUADDR = $2001
    0xA5, 0x12, //       LDA $12
    0x18, //             CLC
    0x69, 0x80, //       ADC #$80       tile $80 released, $81 held
    0x8D, 0x07, 0x20, // STA $2007
    0xA9, 0x00, //       LDA #$00
    0x8D, 0x06, 0x20, // STA $2006
    0x8D, 0x06, 0x20, // STA $2006      v = t = $0000: nametable 0, no scroll at all
    0xA5, 0x14, //       LDA $14        the light of the frame that has just ended
    0xA6, 0x13, //       LDX $13
    0x9D, 0x00, 0x02, // STA $0200,X    one record per emulated frame
    0xE6, 0x13, //       INC $13
    0xA9, 0x00, //       LDA #$00
    0x85, 0x14, //       STA $14        the next frame starts over
    0x40, //             RTI
];

/// Builds the image: NROM, vertical mirroring, reset at `$8000`, NMI at `$9000`.
#[must_use]
pub fn build() -> Vec<u8> {
    const PRG_SIZE: usize = 32 * 1024;
    const CHR_SIZE: usize = 8 * 1024;
    let mut image = vec![0_u8; 16 + PRG_SIZE + CHR_SIZE];
    image[..4].copy_from_slice(b"NES\x1A");
    image[4] = 2; // 2 x 16 KiB PRG.
    image[5] = 1; // 1 x 8 KiB CHR.
    image[6] = 0x01; // Vertical mirroring, mapper 0.

    image[16..16 + RESET.len()].copy_from_slice(&RESET);
    let nmi_at = 16 + 0x1000;
    image[nmi_at..nmi_at + NMI.len()].copy_from_slice(&NMI);

    // Vectors: NMI at $9000, RESET and IRQ at $8000.
    let vectors = 16 + PRG_SIZE - 6;
    image[vectors..vectors + 6].copy_from_slice(&[0x00, 0x90, 0x00, 0x80, 0x00, 0x80]);

    // The CHR of SPEC-02 §5.1; tile 0 of table $0000 stays entirely zero.
    let chr = 16 + PRG_SIZE;
    for table in 0..2 {
        for tile in 0..256_usize {
            if table == 0 && tile == 0 {
                continue;
            }
            let offset = chr + (table * 4096) + (tile * 16);
            for row in 0..8 {
                #[allow(clippy::cast_possible_truncation)] // 0..256.
                {
                    image[offset + row] = tile as u8;
                    image[offset + 8 + row] = (255 - tile) as u8;
                }
            }
        }
    }

    image
}

/// The value the picture of `core` shows: the eight bits of the top-left tile, most significant
/// first.
///
/// The pixel image is **not** part of the in-memory state of REQ-CORE-121, so what it holds after
/// a real frame of run-ahead is what the displayed frame drew — which is exactly what the
/// criterion of REQ-LOOP-065 asks to be read.
#[must_use]
pub fn displayed(core: &NesCore) -> u32 {
    let mut value = 0_u32;
    for (bit, pixel) in core.pixel_frame().iter().take(8).enumerate() {
        if *pixel == ONE_COLOUR {
            value |= 0x80 >> bit;
        }
    }
    value
}

/// Whether the picture of `core` shows button A held.
///
/// The second cell holds tile `$80` or `$81`, so its last pixel carries bit 0 of the tile index,
/// which is the A bit the frame read.
#[must_use]
pub fn shows_button_a(core: &NesCore) -> bool {
    core.pixel_frame()[15] == ONE_COLOUR
}
