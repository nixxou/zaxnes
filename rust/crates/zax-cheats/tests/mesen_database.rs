//! Reads every file of a Mesen-format cheat database (BeesNES's `Cheats.zip`, extracted) and
//! reports what does not read. `ZAXNES_CHEAT_DB=<folder> cargo test -p zax-cheats -- --ignored`.

#[test]
#[ignore = "needs ZAXNES_CHEAT_DB, a folder of Mesen JSON cheat files"]
fn every_cheat_of_the_database_reads() {
    let Some(folder) = std::env::var_os("ZAXNES_CHEAT_DB") else {
        panic!("set ZAXNES_CHEAT_DB");
    };
    let (mut files, mut cheats, mut skipped) = (0usize, 0usize, Vec::new());
    for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
        let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
        let imported = zax_cheats::import::mesen_json(&text)
            .unwrap_or_else(|error| panic!("{}: {error}", entry.path().display()));
        files += 1;
        cheats += imported.cheats.len();
        for (description, error) in imported.skipped {
            skipped.push(format!("{}: {description}: {error}", entry.path().display()));
        }
    }
    println!("{files} files, {cheats} cheats, {} skipped", skipped.len());
    for line in skipped.iter().take(40) {
        println!("{line}");
    }
    assert!(files > 0);
    // the database holds notes without codes and templates with `??` to fill in
    assert!(skipped.len() * 200 < cheats, "more than 0.5 % of the cheats do not read");
}
