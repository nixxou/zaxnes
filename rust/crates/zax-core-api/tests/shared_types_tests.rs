//! What the move of lot FCEUX-23 step 2 must not have changed.
//!
//! The behaviour of the moved types themselves is already pinned by `zax-nes`'s own tests, which
//! reach them through its re-exports and so test them where they now live. What is new here is
//! what the move itself could have broken: the three audio bounds left `Apu2A03` for this crate
//! and the `Display` of [`AudioError`] prints them, so the messages are pinned to the character;
//! and the input word's layout is now stated here rather than only in `NesCore::run_frame`'s
//! doc comment, so it is pinned too.

// A test: an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use zax_core_api::{
    button, AudioError, ControllerPortType, FrameBatch, InvalidPort, NesRegion, PaletteSnapshot,
    RomLoadFailure, RomLoadResult, StateLoadFailure, TileRecord, PORT_2_SHIFT, PORT_COUNT,
    STANDARD_ROWS,
};

/// REQ-CORE-070: the messages name the bounds, and the bounds came here with the error.
#[test]
fn the_audio_messages_are_the_ones_the_apu_used_to_print() {
    assert_eq!(
        AudioError::SampleRateOutOfRange(8_000).to_string(),
        "sample rate 8000 is outside 22050..=96000"
    );
    assert_eq!(
        AudioError::SpeedNotPositive(0.0).to_string(),
        "speed 0 must be positive"
    );
    assert_eq!(
        AudioError::RatioAdjustmentOutOfRange(0.5).to_string(),
        "ratio adjustment 0.5 exceeds ±0.005"
    );
    assert_eq!(
        AudioError::OddStereoBuffer(7).to_string(),
        "a stereo buffer must hold an even number of values, not 7"
    );
}

/// REQ-CORE-070: and `zax-nes`'s APU still offers them as its own associated constants, with the
/// same values, so every use site of them is unchanged.
#[test]
fn the_apu_still_offers_the_three_bounds_it_used_to_own() {
    assert_eq!(
        zax_nes::apu::Apu2A03::MIN_SAMPLE_RATE,
        zax_core_api::MIN_SAMPLE_RATE
    );
    assert_eq!(
        zax_nes::apu::Apu2A03::MAX_SAMPLE_RATE,
        zax_core_api::MAX_SAMPLE_RATE
    );
    assert!(
        (zax_nes::apu::Apu2A03::MAX_RATIO_ADJUSTMENT - zax_core_api::MAX_RATIO_ADJUSTMENT).abs()
            < f64::EPSILON
    );
}

/// REQ-CORE-020, REQ-CORE-022, SPEC-02 §4.4: port 1 low, port 2 high, A first.
#[test]
fn the_input_word_puts_port_one_low_and_port_two_high() {
    assert_eq!(PORT_COUNT, 2);
    assert_eq!(PORT_2_SHIFT, 8);
    assert_eq!(
        [
            button::A,
            button::B,
            button::SELECT,
            button::START,
            button::UP,
            button::DOWN,
            button::LEFT,
            button::RIGHT
        ],
        [1, 2, 4, 8, 16, 32, 64, 128]
    );

    // Start on port 1 and Right on port 2, as the application's routing builds it.
    let word = u16::from(button::START) | (u16::from(button::RIGHT) << PORT_2_SHIFT);
    assert_eq!(word, 0x8008);
}

/// REQ-CORE-024: a port is 0 or 1, and the message says so.
#[test]
fn a_port_out_of_range_names_itself() {
    assert_eq!(
        InvalidPort(2).to_string(),
        "port 2 is out of range (0 or 1)"
    );
    assert_eq!(
        ControllerPortType::default(),
        ControllerPortType::Controller
    );
    assert_eq!(ControllerPortType::Zapper as u8, 1);
}

/// SPEC-02 §4.5: the persisted numeric values of the region must not move.
#[test]
fn the_region_keeps_its_persisted_numbers() {
    for (value, region) in [
        (0, NesRegion::Ntsc),
        (1, NesRegion::Pal),
        (2, NesRegion::Multi),
        (3, NesRegion::Dendy),
    ] {
        assert_eq!(NesRegion::from_u8(value), Some(region));
        assert_eq!(region as u8, value);
    }
    assert_eq!(NesRegion::from_u8(4), None);
    assert_eq!(NesRegion::default(), NesRegion::Ntsc);
}

/// REQ-CORE-011, REQ-CORE-013: the outcome of a load, which both engines return.
#[test]
fn a_load_result_carries_the_region_or_the_typed_cause() {
    let ok = RomLoadResult::ok(NesRegion::Pal);
    assert!(ok.success);
    assert_eq!(ok.region, NesRegion::Pal);
    assert_eq!(ok.failure, RomLoadFailure::None);
    assert!(ok.error.is_none() && ok.note.is_none());

    let bad = RomLoadResult::fail(RomLoadFailure::UnsupportedMapper, "mapper 999".to_owned());
    assert!(!bad.success);
    assert_eq!(bad.failure, RomLoadFailure::UnsupportedMapper);
    assert_eq!(bad.error.as_deref(), Some("mapper 999"));
    // On a failure the region is the default, not a detected one (REQ-CORE-005).
    assert_eq!(bad.region, NesRegion::Ntsc);
}

/// REQ-CORE-120: the engine a state came from is a typed cause of its own, which step 5 needs.
#[test]
fn a_foreign_core_is_a_typed_cause_distinct_from_a_corrupt_block() {
    assert_ne!(
        StateLoadFailure::ForeignCore,
        StateLoadFailure::CorruptState
    );
    assert_eq!(
        zax_core_api::InvalidState::corrupt("short").failure,
        StateLoadFailure::CorruptState
    );
    assert_eq!(
        zax_core_api::NoRomLoaded.to_string(),
        "No ROM is loaded: there is no state to save."
    );
}

/// REQ-CORE-030, REQ-CORE-045: a batch is the records *and* the 32 palette bytes, and the
/// palette is the named snapshot both engines produce.
#[test]
fn a_frame_batch_is_the_records_and_the_palette() {
    let records = [TileRecord::new(
        1, 2, 8, 16, 3, false, false, false, 1, true,
    )];
    let palette: PaletteSnapshot = [7; 32];
    let batch = FrameBatch {
        records: &records,
        palette,
        margin_palette: None,
        mask_x: 0,
        mask_y: 0,
        margin: 0,
        visible_rows: STANDARD_ROWS,
        margin_y: 0,
    };

    assert_eq!(batch.records.len(), 1);
    assert_eq!(batch.palette.len(), 32);
    assert_eq!(batch.visible_rows.len(), 30);
    assert_eq!(batch.visible_rows[0], (0, 256));
}

/// Step 2 was a pure move: every path a consumer of `zax-nes` used still resolves, and to the
/// very same type — which is what lets the application change over one crate at a time.
#[test]
fn zax_nes_re_exports_the_moved_types_at_the_paths_they_were_at() {
    // A function generic over nothing, which only compiles if the two paths are one type.
    const fn same<T>(_: &T, _: &T) {}

    let record = TileRecord::default();
    let nes_record: zax_nes::TileRecord = record;
    same(&record, &nes_record);

    let region = NesRegion::Pal;
    let nes_region: zax_nes::NesRegion = region;
    same(&region, &nes_region);

    let result = RomLoadResult::ok(region);
    let nes_result: zax_nes::rom::RomLoadResult = result.clone();
    same(&result, &nes_result);

    let failure = StateLoadFailure::ForeignCore;
    let nes_failure: zax_nes::state::StateLoadFailure = failure;
    same(&failure, &nes_failure);

    let port = ControllerPortType::Zapper;
    let nes_port: zax_nes::input::ControllerPortType = port;
    same(&port, &nes_port);
    let nes_port_root: zax_nes::ControllerPortType = port;
    same(&port, &nes_port_root);

    let audio = AudioError::OddStereoBuffer(3);
    let nes_audio: zax_nes::apu::AudioError = audio;
    same(&audio, &nes_audio);

    let rows: zax_nes::VisibleRows = STANDARD_ROWS;
    same(&rows, &STANDARD_ROWS);
}
