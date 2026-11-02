//! Reads a real `.fm2` movie and checks it against its ROM. The files are not in the repository:
//! `ZAXNES_FM2=<movie.fm2> ZAXNES_FM2_ROM=<rom.nes> cargo test -p zax-nes --release --test
//! fm2_file_tests -- --ignored`.

use zax_nes::movie::fm2::Fm2Movie;

#[test]
#[ignore = "needs ZAXNES_FM2 and ZAXNES_FM2_ROM"]
fn real_movie_reads_and_matches_its_rom() {
    let (Ok(movie_path), Ok(rom_path)) =
        (std::env::var("ZAXNES_FM2"), std::env::var("ZAXNES_FM2_ROM"))
    else {
        return;
    };
    let movie = std::fs::read(movie_path).map(|file| Fm2Movie::parse(&file));
    let Ok(Ok(movie)) = movie else {
        panic!("the movie does not read: {movie:?}");
    };
    let rom = std::fs::read(rom_path).unwrap_or_default();
    assert_eq!(movie.rom_matches(&rom), Some(true));
    println!(
        "{} frames, {} resets, {} rerecords",
        movie.frames.len(),
        movie.resets().len(),
        movie.header.rerecord_count.unwrap_or(0)
    );
    assert_ne!(movie.frames.len(), 0);
}
