//! The interface of the curvature: the "Geometry" group of AMD-043 §6 and the "Bend 2D Mode"
//! row of AMD-046 §3, in the desktop options window and on the handheld's advanced page.
//!
//! All of it is checked on the **models**, which draw nothing, so it runs without a window.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

use zax_app::ui::handheld::advanced;
use zax_app::ui::handheld::options::OptionRow;
use zax_app::ui::options::{self, OptionsModel, OptionsTab};
use zax_app::ui::strings::{self, UiLanguage};
use zax_settings::catalogue::SettingId;

/// The five settings of the group, in the order the two amendments give them.
const GROUP: [SettingId; 5] = [
    SettingId::SceneBendLr,
    SettingId::SceneBendTb,
    SettingId::SceneArcH,
    SettingId::SceneArcCenter,
    SettingId::SceneBend2dMode,
];

/// AMD-043 §6 and AMD-046 §3: the five settings are rows of the options window — they left
/// `EXCLUDED` with this lot, which is what gives them an effect.
#[test]
fn req_scn_066_the_five_settings_are_rows_of_the_options_window() {
    let model = OptionsModel::new();
    let shown: Vec<SettingId> = model
        .all_rows()
        .map(|row| row.descriptor.id())
        .filter(|id| GROUP.contains(id))
        .collect();
    assert_eq!(GROUP.to_vec(), shown, "all five, in the amendments' order");

    for id in GROUP {
        assert!(
            !options::EXCLUDED.contains(&id),
            "{id:?} is no longer excluded"
        );
    }

    // RO-1: they are on the 3D rendering tab, which took the Scene theme off the old Video tab.
    let render: Vec<SettingId> = model
        .rows(OptionsTab::Render3d)
        .iter()
        .map(|row| row.descriptor.id())
        .collect();
    for id in GROUP {
        assert!(render.contains(&id), "{id:?} is on the 3D rendering tab");
    }
}

/// AMD-043 §6: the group is marked by a heading of its own, which holds exactly the five settings
/// and no other row of any tab. RO-1 makes it collapsible, the family having grown to five.
#[test]
fn req_scn_066_the_group_holds_exactly_the_five_settings() {
    let in_group: Vec<SettingId> = OptionsModel::new()
        .all_rows()
        .map(|row| row.descriptor.id())
        .filter(|id| options::group_of(*id) == Some(options::OptionsGroup::Geometry))
        .collect();
    assert_eq!(GROUP.to_vec(), in_group);
    assert!(options::OptionsGroup::Geometry.is_collapsible());

    for (language, expected) in [
        (UiLanguage::English, "Geometry"),
        (UiLanguage::French, "Géométrie"),
    ] {
        strings::set_language(language);
        assert_eq!(expected, options::OptionsGroup::Geometry.title());
    }
    strings::set_language(UiLanguage::English);
}

/// AMD-043 §6 and AMD-046 §3: each of the five carries the label and the help sentence the
/// amendments write, in both languages.
#[test]
fn req_scn_066_the_labels_and_the_help_come_from_the_amendments() {
    let labels_and_help = [
        (
            SettingId::SceneBendLr,
            ("Bend Left-Right", "Courbure gauche-droite"),
            ("bulge it outwards like a CRT screen", "écran cathodique"),
        ),
        (
            SettingId::SceneBendTb,
            ("Bend Top-Bottom", "Courbure haut-bas"),
            (
                "Bends the scene vertically",
                "Courbe la scène verticalement",
            ),
        ),
        (
            SettingId::SceneArcH,
            ("Arc Horizontal", "Arc horizontal"),
            ("as if it stood on a tiny planet", "toute petite planète"),
        ),
        (
            SettingId::SceneArcCenter,
            ("Arc Center", "Centre de l'arc"),
            ("the centre of the screen", "le centre de l'écran"),
        ),
        (
            SettingId::SceneBend2dMode,
            ("Bend 2D Mode", "Courber le mode 2D"),
            (
                "Turn it off to keep the picture flat",
                "garder l'image plate",
            ),
        ),
    ];

    for (id, (english, french), (english_help, french_help)) in labels_and_help {
        let descriptor = zax_settings::catalogue::descriptor(id);
        for (language, label, fragment) in [
            (UiLanguage::English, english, english_help),
            (UiLanguage::French, french, french_help),
        ] {
            strings::set_language(language);
            assert_eq!(label, zax_app::ui::text::label_of(descriptor), "{id:?}");
            let help = options::help_of(id).expect("every one of the five has a help sentence");
            assert!(
                help.contains(fragment),
                "{id:?} in {language:?}: {help:?} should mention {fragment:?}"
            );
        }
    }
    strings::set_language(UiLanguage::English);

    // A setting outside the group has no help sentence of this kind.
    assert_eq!(None, options::help_of(SettingId::CameraFov));
}

/// AMD-043 §6: the arc centre is a list of two named values, not a number — the idiom of
/// `emu.run_ahead`, which keeps the domain's bounds and step and only names its rungs.
#[test]
fn req_scn_066_the_arc_centre_is_a_list_of_two_named_values() {
    let descriptor = zax_settings::catalogue::descriptor(SettingId::SceneArcCenter);
    let options = descriptor.domain().options().expect("two rungs");
    assert_eq!(2, options.len());
    assert_eq!("Screen Center", options[0].label);
    assert_eq!("Screen Edge", options[1].label);
    assert_eq!(0.0, options[0].value);
    assert_eq!(1.0, options[1].value);

    for (language, centre, edge) in [
        (UiLanguage::English, "Screen Center", "Screen Edge"),
        (UiLanguage::French, "Centre de l'écran", "Bord de l'écran"),
    ] {
        strings::set_language(language);
        assert_eq!(centre, zax_app::ui::text::label_of_option("Screen Center"));
        assert_eq!(edge, zax_app::ui::text::label_of_option("Screen Edge"));
    }
    strings::set_language(UiLanguage::English);

    // And the three angles keep their thirteen rungs of 30 degrees (REQ-SCN-066).
    for id in [
        SettingId::SceneBendLr,
        SettingId::SceneBendTb,
        SettingId::SceneArcH,
    ] {
        let domain = zax_settings::catalogue::descriptor(id).domain();
        assert_eq!(Some(30.0), domain.step());
        assert_eq!(Some(-180.0), domain.minimum());
        assert_eq!(Some(180.0), domain.maximum());
        assert_eq!(13, domain.options().expect("rungs").len());
    }
}

/// The brief's handheld condition: the curvature settings are offered among the handheld's
/// advanced options **because** at 0 they cost nothing — no shader variant is compiled and no
/// surface is subdivided (measured in `docs/PERFORMANCE-HISTORY.md`). They are picked up by the
/// page's own rule (a steppable setting of the four desktop tabs), so there is nothing special
/// about them there.
#[test]
fn req_scn_066_the_handheld_advanced_page_offers_the_five() {
    let rows = advanced::rows();
    for id in GROUP {
        assert!(
            rows.contains(&OptionRow::Setting(id)),
            "{id:?} is on the handheld's advanced page"
        );
    }

    // And the page can step each one, which is what putting it there is for.
    let mut model = zax_settings::model::SettingsModel::new();
    advanced::step(SettingId::SceneBendLr, &mut model, 1);
    assert_eq!(30, model.get(SettingId::SceneBendLr).as_int());
    advanced::step(SettingId::SceneBendLr, &mut model, -1);
    assert_eq!(0, model.get(SettingId::SceneBendLr).as_int());

    advanced::step(SettingId::SceneBend2dMode, &mut model, 1);
    assert!(!model.get(SettingId::SceneBend2dMode).as_bool(), "toggled");

    // The value the page shows for the arc centre is the named one, not the number.
    advanced::step(SettingId::SceneArcCenter, &mut model, 1);
    assert_eq!(
        "Screen Edge",
        advanced::value_text(SettingId::SceneArcCenter, &model)
    );
}
