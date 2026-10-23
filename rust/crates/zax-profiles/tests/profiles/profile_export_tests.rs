use std::sync::Arc;

use zax_profiles::enums::{ProfileFormatVersion, TileFlip};
use zax_profiles::export::{self, IDENTIFIER_LENGTH};
use zax_profiles::format;
use zax_profiles::layout::FieldPresence;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::script::ProfileScript;
use zax_profiles::tile::{PatternGrid, ProfileTile};
use zax_profiles::values::Vec2I;
use zax_profiles::variant::PatternVariant;

#[test]
fn req_prof_052_an_exported_profile_gets_the_current_signature_and_version() {
    let mut profile = GameProfile { signature: None, version_text: Some("2".to_owned()), ..GameProfile::default() };

    export::normalize(&mut profile, None).unwrap();

    assert_eq!(Some("3dn"), profile.signature.as_deref());
    assert_eq!(Some("3"), profile.version_text.as_deref());
    assert_eq!(ProfileFormatVersion::Current, profile.format_version());
}

#[test]
fn req_prof_052_a_profile_without_an_identifier_gets_a_twelve_character_one() {
    let mut profile = GameProfile::default();

    export::normalize(&mut profile, None).unwrap();

    let id = profile.id.as_deref().unwrap();
    assert_eq!(IDENTIFIER_LENGTH, id.len());
    for character in id.chars() {
        assert!(
            character.is_ascii_alphanumeric() || character == '+' || character == '/',
            "\"{character}\" is not part of the standard base64 alphabet"
        );
    }

    // An identifier that is already there is kept.
    let mut known = GameProfile { id: Some("Lc89vRjlYrUL".to_owned()), ..GameProfile::default() };
    export::normalize(&mut known, None).unwrap();
    assert_eq!(Some("Lc89vRjlYrUL"), known.id.as_deref());

    // Two generated identifiers differ.
    assert_ne!(export::new_identifier().unwrap(), export::new_identifier().unwrap());
}

#[test]
fn req_prof_052_an_empty_title_takes_the_rom_file_name() {
    let mut empty = GameProfile { title: Some(String::new()), ..GameProfile::default() };
    export::normalize(&mut empty, Some("Arkanoid (U)")).unwrap();
    assert_eq!(Some("Arkanoid (U)"), empty.title.as_deref());

    // An absent title stays absent, and a real title is kept.
    let mut absent = GameProfile::default();
    export::normalize(&mut absent, Some("Arkanoid (U)")).unwrap();
    assert_eq!(None, absent.title);

    let mut titled = GameProfile { title: Some("Mine".to_owned()), ..GameProfile::default() };
    export::normalize(&mut titled, Some("Arkanoid (U)")).unwrap();
    assert_eq!(Some("Mine"), titled.title.as_deref());
}

#[test]
fn req_prof_052_patterns_are_filed_by_their_own_flag_and_scripts_are_sorted() {
    let mut profile = GameProfile::default();
    let permanent = Arc::new(ProfilePattern { permanent: true, ..ProfilePattern::default() });
    let temporary = Arc::new(ProfilePattern { permanent: false, ..ProfilePattern::default() });
    let also_permanent = Arc::new(ProfilePattern { permanent: true, ..ProfilePattern::default() });
    profile.permanent_patterns.push(Arc::clone(&permanent));
    profile.permanent_patterns.push(Arc::clone(&temporary));
    profile.temporary_patterns.push(Arc::clone(&also_permanent));
    profile.scripts.push(ProfileScript::new(Some("scene"), true, None, None));
    profile.scripts.push(ProfileScript::new(Some("bike"), true, None, None));

    export::normalize(&mut profile, None).unwrap();

    assert_eq!(2, profile.permanent_patterns.len());
    assert!(Arc::ptr_eq(&permanent, &profile.permanent_patterns[0]));
    assert!(Arc::ptr_eq(&also_permanent, &profile.permanent_patterns[1]));
    assert_eq!(1, profile.temporary_patterns.len());
    assert!(Arc::ptr_eq(&temporary, &profile.temporary_patterns[0]));
    let names: Vec<Option<&str>> = profile.scripts.iter().map(|script| script.name.as_deref()).collect();
    assert_eq!(vec![Some("bike"), Some("scene")], names);
}

#[test]
fn req_prof_052_an_exported_profile_reads_back_with_the_same_patterns() {
    let mut profile = GameProfile::create_empty();
    let pattern = ProfilePattern {
        permanent: true,
        tile_size: Some(Vec2I::new(2, 1)),
        grid: Some(PatternGrid::new(
            2,
            1,
            vec![Some(Arc::new(ProfileTile::new(1, 2, TileFlip::HORIZONTAL))), None],
        )),
        variants: vec![Arc::new(PatternVariant {
            depth: 8,
            tag_string: Some("road".to_owned()),
            ..PatternVariant::default()
        })],
        ..ProfilePattern::default()
    };
    profile.permanent_patterns.push(Arc::new(pattern));

    export::normalize(&mut profile, Some("Test")).unwrap();
    let result = format::load(&format::save(&profile, FieldPresence::All), None, None).unwrap();

    assert_eq!(1, result.profile.permanent_patterns.len());
    let restored = &result.profile.permanent_patterns[0];
    assert_eq!(Some(Vec2I::new(2, 1)), restored.tile_size);
    let grid = restored.grid.as_ref().unwrap();
    assert_eq!(2, grid.cells().len());
    assert_eq!(ProfileTile::new(1, 2, TileFlip::HORIZONTAL), **grid.cells()[0].as_ref().unwrap());
    assert!(grid.cells()[1].is_none());
    assert_eq!(8, restored.variants[0].depth);
    assert_eq!(vec!["road"], restored.variants[0].tags());
}
