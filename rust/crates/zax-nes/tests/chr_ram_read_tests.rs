//! Reading the cartridge's CHR-RAM between two frames (AMD-062 REQ-CORE-123), which is what the
//! tile resolution of a "3dh" profile needs for the games that build their tiles in CHR-RAM
//! instead of carrying them in CHR-ROM.

mod common;

use common::{machine_with, minimal, BatchCopy};

/// Size of the CHR-RAM REQ-CORE-012 gives a cartridge whose header announces no CHR-ROM.
const DEFAULT_CHR_RAM_SIZE: usize = 32 * 1024;

/// PPU address the test program fills, from the criterion of REQ-CORE-123. It is pattern 0x13 of
/// table $0000, so the program can also put that tile on the screen.
const TARGET: usize = 0x0130;

/// The sixteen bytes the program writes there.
const WANTED: [u8; 16] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
];

/// Frames to run before the program has had its say: it waits for two vblanks, because the PPU
/// ignores `$2000`, `$2001` and `$2006` until it has warmed up, and then does its writes.
const FRAMES_TO_SETTLE: usize = 6;

/// A mapper-0 image with header byte 5 at 0 — no CHR-ROM, so 32 KiB of CHR-RAM and no banks, and
/// PPU `$0000`-`$1FFF` is positions 0 to `0x1FFF` (REQ-CORE-012) — running `program` at $8000,
/// with every vector pointing there.
fn chr_ram_rom(program: &[u8]) -> Vec<u8> {
    const PRG_SIZE: usize = 2 * 16 * 1024;
    let mut image = vec![0_u8; 16 + PRG_SIZE];
    image[..4].copy_from_slice(b"NES\x1A");
    image[4] = 0x02;
    image[5] = 0x00;
    image[6] = 0x01;
    image[16..16 + program.len()].copy_from_slice(program);

    let vectors = 16 + PRG_SIZE - 6;
    for slot in 0..3 {
        image[vectors + 2 * slot] = 0x00;
        image[vectors + 2 * slot + 1] = 0x80;
    }

    image
}

/// Waits for two vblanks, writes `00 01 ... 0f` to PPU `$0130` through `$2006`/`$2007`, fills a
/// palette and the first rows of the nametable with pattern `$13` — the one it just wrote — then
/// turns rendering on and stops. So the frames the core produces are drawn from the CHR-RAM the
/// test reads.
///
/// ```text
/// 8000  2C 02 20     BIT $2002        ; clear a pending vblank flag
/// 8003  2C 02 20 vb1: BIT $2002
/// 8006  10 FB        BPL vb1
/// 8008  2C 02 20 vb2: BIT $2002
/// 800B  10 FB        BPL vb2
/// 800D  A9 00        LDA #$00
/// 800F  8D 00 20     STA $2000        ; address increments by 1, NMI off
/// 8012  8D 01 20     STA $2001        ; rendering off while the memories are written
/// 8015  A9 01        LDA #$01
/// 8017  8D 06 20     STA $2006        ; $0130, high byte
/// 801A  A9 30        LDA #$30
/// 801C  8D 06 20     STA $2006        ; low byte
/// 801F  A2 00        LDX #$00
/// 8021  8E 07 20 fill: STX $2007      ; the sixteen bytes of the criterion
/// 8024  E8           INX
/// 8025  E0 10        CPX #$10
/// 8027  D0 F8        BNE fill
/// 8029  A9 3F        LDA #$3F
/// 802B  8D 06 20     STA $2006        ; $3F00, the palettes
/// 802E  A9 00        LDA #$00
/// 8030  8D 06 20     STA $2006
/// 8033  A2 00        LDX #$00
/// 8035  8A       pal: TXA
/// 8036  8D 07 20     STA $2007
/// 8039  E8           INX
/// 803A  E0 20        CPX #$20
/// 803C  D0 F7        BNE pal
/// 803E  A9 20        LDA #$20
/// 8040  8D 06 20     STA $2006        ; $2000, the first nametable
/// 8043  A9 00        LDA #$00
/// 8045  8D 06 20     STA $2006
/// 8048  A2 00        LDX #$00
/// 804A  A9 13   nt:  LDA #$13
/// 804C  8D 07 20     STA $2007        ; 256 cells of pattern $13
/// 804F  E8           INX
/// 8050  D0 F8        BNE nt
/// 8052  A9 1E        LDA #$1E
/// 8054  8D 01 20     STA $2001        ; rendering on
/// 8057  4C 57 80     JMP $8057
/// ```
const RENDER_FROM_CHR_RAM: [u8; 90] = [
    0x2C, 0x02, 0x20, 0x2C, 0x02, 0x20, 0x10, 0xFB, 0x2C, 0x02, 0x20, 0x10, 0xFB, 0xA9, 0x00, 0x8D,
    0x00, 0x20, 0x8D, 0x01, 0x20, 0xA9, 0x01, 0x8D, 0x06, 0x20, 0xA9, 0x30, 0x8D, 0x06, 0x20, 0xA2,
    0x00, 0x8E, 0x07, 0x20, 0xE8, 0xE0, 0x10, 0xD0, 0xF8, 0xA9, 0x3F, 0x8D, 0x06, 0x20, 0xA9, 0x00,
    0x8D, 0x06, 0x20, 0xA2, 0x00, 0x8A, 0x8D, 0x07, 0x20, 0xE8, 0xE0, 0x20, 0xD0, 0xF7, 0xA9, 0x20,
    0x8D, 0x06, 0x20, 0xA9, 0x00, 0x8D, 0x06, 0x20, 0xA2, 0x00, 0xA9, 0x13, 0x8D, 0x07, 0x20, 0xE8,
    0xD0, 0xF8, 0xA9, 0x1E, 0x8D, 0x01, 0x20, 0x4C, 0x57, 0x80,
];

// REQ-CORE-123: a mapper-0 cartridge with header byte 5 at 0 has 32 KiB of CHR-RAM, and once the
// program has written the sixteen bytes at PPU $0130 the core gives them back at the positions
// 0x130 to 0x13F, the memory being addressed linearly from 0.
#[test]
fn req_core_123_the_core_reads_the_bytes_the_program_wrote_to_chr_ram() {
    let mut core = machine_with(&chr_ram_rom(&RENDER_FROM_CHR_RAM), "ChrRam.nes");
    assert_eq!(DEFAULT_CHR_RAM_SIZE, core.chr_ram().len());
    assert!(core.chr_ram().iter().all(|byte| *byte == 0));

    for _ in 0..FRAMES_TO_SETTLE {
        core.run_frame(0);
    }

    let memory = core.chr_ram();
    assert_eq!(DEFAULT_CHR_RAM_SIZE, memory.len());
    assert_eq!(&WANTED, &memory[TARGET..TARGET + WANTED.len()]);
    // The sixteen bytes are the only ones written: the rest of the memory is still the zeroes it
    // came up with, which is how a block of CHR-RAM that no game has touched looks.
    assert!(memory[..TARGET].iter().all(|byte| *byte == 0));
    assert!(memory[TARGET + WANTED.len()..]
        .iter()
        .all(|byte| *byte == 0));

    // And the frame really is drawn from there: pattern $13 of the first table is those bytes.
    let batch = BatchCopy::of(&core);
    let background = batch.background();
    assert_ne!(0, background.len(), "the frame drew no background tile");
    let planes = (
        u64::from_le_bytes([0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]),
        u64::from_le_bytes([0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]),
    );
    assert!(
        background
            .iter()
            .any(|record| (record.plane0, record.plane1) == planes),
        "no background tile carries the planes written to CHR-RAM"
    );
}

// REQ-CORE-123: a cartridge with CHR-ROM has no CHR-RAM, and so a size of 0 — and so does a core
// with no ROM at all.
#[test]
fn req_core_123_a_chr_rom_cartridge_has_no_chr_ram() {
    let mut core = machine_with(&minimal(), "Minimal.nes");
    assert_eq!(0, core.chr_ram().len());
    core.run_frame(0);
    assert_eq!(0, core.chr_ram().len());

    assert_eq!(0, zax_nes::NesCore::new().chr_ram().len());
}

// REQ-CORE-123: the read has no side effect on the emulation or on its determinism. Two runs of
// the same ROM, one reading the CHR-RAM at every frame and the other never, give the same frames
// and the same machine state.
#[test]
fn req_core_123_reading_the_chr_ram_changes_nothing() {
    let image = chr_ram_rom(&RENDER_FROM_CHR_RAM);
    let mut reading = machine_with(&image, "ChrRam.nes");
    let mut silent = machine_with(&image, "ChrRam.nes");

    let mut digest = 0_u64;
    let mut drawn_frames = 0;
    for _ in 0..FRAMES_TO_SETTLE + 6 {
        reading.run_frame(0);
        silent.run_frame(0);

        // The read under test, once per frame, with something done with what it gives so that it
        // cannot be optimised away.
        for byte in reading.chr_ram() {
            digest = digest.wrapping_mul(31).wrapping_add(u64::from(*byte));
        }

        let expected = BatchCopy::of(&silent);
        if !expected.records.is_empty() {
            drawn_frames += 1;
        }
        assert_eq!(expected, BatchCopy::of(&reading));
    }

    assert_ne!(0, digest);
    assert!(drawn_frames > 0, "no frame drew anything to compare");
    assert_eq!(
        silent.save_state(false).unwrap(),
        reading.save_state(false).unwrap()
    );
    assert_eq!(silent.chr_ram(), reading.chr_ram());
}
