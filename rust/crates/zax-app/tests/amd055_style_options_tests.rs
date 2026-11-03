//! The interface of the Enhanced style: the "Style" group of AMD-055 §3 point 6 (SPEC-10
//! REQ-UI-061 as amended), the visibility rules and the labels of REQ-SCN-079, the greyed
//! `ao.thickness` of REQ-SCN-083's fallback **(D)**, and the handheld's advanced page.
//!
//! All of it is checked on the **models**, which draw nothing, so it runs without a window — the
//! shape `bend_options_tests.rs` established for the curvature group of AMD-043.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;
mod ui_harness;

use ui_harness::UiHarness;
use zax_app::ui::handheld::advanced;
use zax_app::ui::handheld::options::OptionRow;
use zax_app::ui::options::{self, OptionsModel, OptionsTab};
use zax_app::ui::strings::{self, UiLanguage};
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

/// The style row and the twelve parameters, in the order of REQ-SCN-079's table and of §4.
const GROUP: [SettingId; 13] = [
    SettingId::RenderVisualStyle,
    SettingId::RenderStyleAo,
    SettingId::RenderStyleAoStrength,
    SettingId::RenderStyleAoRadius,
    SettingId::RenderStyleAoThickness,
    SettingId::RenderStyleAoQuality,
    SettingId::RenderStylePixelShadow,
    SettingId::RenderStyleVoxelGrid,
    SettingId::RenderStyleReflections,
    SettingId::RenderStyleSsrShine,
    SettingId::RenderStyleSsrClarity,
    SettingId::RenderStyleSsrLength,
    SettingId::RenderStyleSsrQuality,
];

/// A model in the Enhanced style, every parameter at its own default.
fn enhanced() -> SettingsModel {
    let mut model = SettingsModel::new();
    model.set(SettingId::RenderVisualStyle, SettingValue::Int(4));
    model
}

/// SPEC-10 REQ-UI-061 as amended: the thirteen are rows of the options window — they left
/// `EXCLUDED` with AMD-055's lot, which is what their effect earns them.
///
/// RO-1 splits **where** they are drawn: the six parameters that change a surface are in the
/// "Style" group of the 3D rendering tab, the six that decide where light does not reach are in
/// the "Style" group of the Light and shadows tab, beside the shadow switch they relate to. Every
/// one of the thirteen is still exactly one row.
#[test]
fn req_ui_061_the_thirteen_settings_are_rows_of_the_options_window() {
    let model = OptionsModel::new();
    let shown: Vec<SettingId> = model
        .all_rows()
        .map(|row| row.descriptor.id())
        .filter(|id| GROUP.contains(id))
        .collect();
    assert_eq!(GROUP.len(), shown.len(), "all thirteen, each once");
    for id in GROUP {
        assert!(shown.contains(&id), "{id:?} is a row");
        assert!(
            !options::EXCLUDED.contains(&id),
            "{id:?} is no longer excluded"
        );
    }

    // RO-1: the six surface effects with the style, the six light effects with the light.
    let tab = |id: SettingId| options::tab_of(id).expect("a tab");
    assert_eq!(OptionsTab::Render3d, tab(SettingId::RenderVisualStyle));
    for id in [
        SettingId::RenderStyleVoxelGrid,
        SettingId::RenderStyleReflections,
        SettingId::RenderStyleSsrShine,
        SettingId::RenderStyleSsrClarity,
        SettingId::RenderStyleSsrLength,
        SettingId::RenderStyleSsrQuality,
    ] {
        assert_eq!(OptionsTab::Render3d, tab(id), "{id:?}");
        assert_eq!(Some(options::OptionsGroup::Style), options::group_of(id));
    }
    for id in [
        SettingId::RenderStylePixelShadow,
        SettingId::RenderStyleAo,
        SettingId::RenderStyleAoStrength,
        SettingId::RenderStyleAoRadius,
        SettingId::RenderStyleAoThickness,
        SettingId::RenderStyleAoQuality,
    ] {
        assert_eq!(OptionsTab::Light, tab(id), "{id:?}");
        assert_eq!(Some(options::OptionsGroup::Style), options::group_of(id));
    }
}

/// SPEC-10 REQ-UI-061 as amended: the `Style` row comes **after the `Skybox` row** — which
/// carries its colour button with it (REQ-UI-063) — and the parameters of that tab follow it.
#[test]
fn req_ui_061_the_style_row_comes_after_the_skybox_row() {
    let model = OptionsModel::new();
    let render: Vec<SettingId> = model
        .rows(OptionsTab::Render3d)
        .iter()
        .map(|row| row.descriptor.id())
        .collect();
    let at = |wanted: SettingId| {
        render
            .iter()
            .position(|id| *id == wanted)
            .unwrap_or_else(|| panic!("{wanted:?} is a row of the 3D rendering tab"))
    };

    assert_eq!(
        at(SettingId::SceneCustomBgColor) + 1,
        at(SettingId::RenderVisualStyle),
        "the style row follows the skybox row and its colour"
    );
    assert!(at(SettingId::SceneSkybox) < at(SettingId::RenderVisualStyle));
    // And the six of this tab follow it, contiguously, in their own order.
    for (offset, id) in [
        SettingId::RenderStyleVoxelGrid,
        SettingId::RenderStyleReflections,
        SettingId::RenderStyleSsrShine,
        SettingId::RenderStyleSsrClarity,
        SettingId::RenderStyleSsrLength,
        SettingId::RenderStyleSsrQuality,
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(
            at(SettingId::RenderVisualStyle) + 1 + offset,
            at(*id),
            "{id:?} is the {offset}-th row of the group"
        );
    }
    // The catalogue's own order — the order of the settings file — is the other one, which is why
    // the window has a layout table of its own: `render.visual_style` is written long before
    // `scene.skybox`.
    let catalogue: Vec<SettingId> = zax_settings::catalogue::descriptors()
        .map(zax_settings::meta::SettingDescriptor::id)
        .collect();
    let where_in = |wanted: SettingId| catalogue.iter().position(|id| *id == wanted).expect("key");
    assert!(where_in(SettingId::RenderVisualStyle) < where_in(SettingId::SceneSkybox));
}

/// AMD-055 §3 point 6: the group is collapsible and titled "Style" — and its rows are drawn only
/// while the style is Enhanced.
#[test]
fn req_ui_061_the_style_group_is_collapsible_and_drawn_only_in_enhanced() {
    assert!(options::OptionsGroup::Style.is_collapsible());

    for (language, expected) in [
        (UiLanguage::English, "Style"),
        (UiLanguage::French, "Style"),
    ] {
        strings::set_language(language);
        assert_eq!(expected, options::OptionsGroup::Style.title());
    }
    strings::set_language(UiLanguage::English);

    // It is the group of both tabs that carry style parameters, and of no other row.
    for id in options::STYLE {
        assert_eq!(Some(options::OptionsGroup::Style), options::group_of(id));
    }
    assert_ne!(
        Some(options::OptionsGroup::Style),
        options::group_of(SettingId::RenderVisualStyle)
    );

    assert!(
        !options::style_group_shown(&SettingsModel::new()),
        "in Classic no row of the group is drawn"
    );
    assert!(options::style_group_shown(&enhanced()));
    // The accepted identifier 1 selects Enhanced too (REQ-SCN-078), so it opens the group.
    let mut alias = SettingsModel::new();
    alias.set(SettingId::RenderVisualStyle, SettingValue::Int(1));
    assert!(options::style_group_shown(&alias));

    // RO-1: in Classic the heading is still drawn, with the reason the rows are missing — which
    // is what tells a player on the Light and shadows tab that these effects exist at all.
    assert!(options::style_group_note(&SettingsModel::new()).is_some());
    assert_eq!(None, options::style_group_note(&enhanced()));
}

/// REQ-SCN-079's visibility rules, and the amended acceptance criterion of REQ-UI-061: four of
/// the twelve with the default values, the twelve with `spl.ao` ≠ 0 and `spl.reflections` = 1,
/// and none at all in Classic.
#[test]
fn req_scn_079_four_parameters_are_visible_by_default_and_twelve_with_both_effects_on() {
    let visible = |model: &SettingsModel| -> Vec<SettingId> {
        options::STYLE
            .iter()
            .copied()
            .filter(|id| options::style_row_visible(*id, model))
            .collect()
    };

    let classic = SettingsModel::new();
    assert!(
        visible(&classic).is_empty(),
        "in Classic not one parameter is shown"
    );

    let mut model = enhanced();
    assert_eq!(
        vec![
            SettingId::RenderStyleAo,
            SettingId::RenderStylePixelShadow,
            SettingId::RenderStyleVoxelGrid,
            SettingId::RenderStyleReflections,
        ],
        visible(&model),
        "the four switches, with the default values"
    );

    // `spl.ao` ≠ 0 brings the occlusion's four; the reflections' four stay out.
    model.set(SettingId::RenderStyleAo, SettingValue::Int(2));
    assert_eq!(8, visible(&model).len());
    for id in [
        SettingId::RenderStyleAoStrength,
        SettingId::RenderStyleAoRadius,
        SettingId::RenderStyleAoThickness,
        SettingId::RenderStyleAoQuality,
    ] {
        assert!(options::style_row_visible(id, &model), "{id:?}");
    }
    assert!(!options::style_row_visible(
        SettingId::RenderStyleSsrShine,
        &model
    ));

    // `spl.reflections` = 1 brings the other four: twelve.
    model.set(SettingId::RenderStyleReflections, SettingValue::Int(1));
    assert_eq!(options::STYLE.to_vec(), visible(&model));

    // The occlusion's MSVO rung shows them too (it is not off).
    model.set(SettingId::RenderStyleAo, SettingValue::Int(1));
    assert!(options::style_row_visible(
        SettingId::RenderStyleAoStrength,
        &model
    ));

    // A row that is not one of the twelve always answers true, so the drawing site can test it
    // on every row of a tab.
    assert!(options::style_row_visible(
        SettingId::RenderVisualStyle,
        &classic
    ));
    assert!(options::style_row_visible(SettingId::CameraFov, &classic));
}

/// REQ-SCN-079: `ao.thickness` is presented **greyed** outside the MSVO mode, with the reason
/// beside it — and it is the only row that is. The setting itself is kept and persisted: greying is
/// a presentation, not a refusal.
///
/// RU-6 landed the MSVO port, so the row is **live** at `spl.ao` = 1 and greyed at 0 and 2; before
/// it, the fallback **(D)** of REQ-SCN-083 made the mode never effective and the row was greyed at
/// all three. `zax_render::style::thickness_acts` is the one predicate both the renderer and this
/// rule read, so the two cannot drift apart.
#[test]
fn req_scn_079_the_thickness_is_greyed_outside_the_msvo_mode() {
    const { assert!(zax_render::style::MSVO_PORTED) };

    let mut model = enhanced();
    for mode in [0, 1, 2] {
        model.set(SettingId::RenderStyleAo, SettingValue::Int(mode));
        let note = options::inert_note(SettingId::RenderStyleAoThickness, &model);
        if mode == 1 {
            assert_eq!(None, note, "spl.ao = 1 is the MSVO mode: the row acts");
        } else {
            assert!(note.is_some(), "greyed at spl.ao = {mode}");
        }
        // And it is the only greyed row of the thirteen while the shadows are on, which is their
        // default — RO-1 greys `spl.pixel_shadow` when they are off, and nothing else.
        for other in GROUP {
            if other == SettingId::RenderStyleAoThickness {
                continue;
            }
            assert_eq!(
                None,
                options::inert_note(other, &model),
                "{other:?} is live"
            );
        }
    }
    // The language fragments below are read at a mode where the row **is** greyed.
    model.set(SettingId::RenderStyleAo, SettingValue::Int(2));

    for (language, fragment) in [
        (UiLanguage::English, "Depth occlusion"),
        (UiLanguage::French, "l'occlusion Depth"),
    ] {
        strings::set_language(language);
        let note = options::inert_note(SettingId::RenderStyleAoThickness, &model)
            .expect("the note is shown");
        assert!(note.contains(fragment), "{language:?}: {note:?}");
    }
    strings::set_language(UiLanguage::English);

    // The acceptance criterion of REQ-SCN-079 on the value: it is written, bounded and read back
    // whatever the greying says.
    model.set(SettingId::RenderStyleAoThickness, SettingValue::Int(9));
    assert_eq!(5, model.get(SettingId::RenderStyleAoThickness).as_int());
}

/// AMD-055 §4: the thirteen labels and the thirteen help sentences, in both languages, in the
/// amendment's own words.
#[test]
fn req_scn_079_the_labels_and_the_help_come_from_the_amendment() {
    let labels_and_help = [
        (
            SettingId::RenderVisualStyle,
            ("Style", "Style"),
            ("Enhanced adds optional effects", "effets optionnels"),
        ),
        (
            SettingId::RenderStyleAo,
            ("Ambient Occlusion", "Occlusion ambiante"),
            ("corners and hollows", "les coins et les creux"),
        ),
        (
            SettingId::RenderStyleAoStrength,
            ("Strength", "Force"),
            (
                "How dark the shading gets",
                "Intensité de l'assombrissement",
            ),
        ),
        (
            SettingId::RenderStyleAoRadius,
            ("Radius", "Rayon"),
            ("How far the shading reaches", "Portée de l'assombrissement"),
        ),
        (
            SettingId::RenderStyleAoThickness,
            ("Thickness", "Épaisseur"),
            (
                "No effect with Depth occlusion",
                "Sans effet avec l'occlusion Depth",
            ),
        ),
        (
            SettingId::RenderStyleAoQuality,
            ("AO Quality", "Qualité de l'occlusion"),
            ("More samples", "Plus d'échantillons"),
        ),
        (
            SettingId::RenderStylePixelShadow,
            ("Pixel Shadows", "Ombres pixel"),
            ("the game's pixel grid", "la grille de pixels du jeu"),
        ),
        (
            SettingId::RenderStyleVoxelGrid,
            ("Voxel Grid", "Grille de voxels"),
            ("like toy bricks", "comme des briques de jouet"),
        ),
        (
            SettingId::RenderStyleReflections,
            ("Reflections", "Reflets"),
            ("like a polished surface", "comme une surface polie"),
        ),
        (
            SettingId::RenderStyleSsrShine,
            ("Floor Shine", "Brillance du sol"),
            ("How strong the reflection is", "Intensité du reflet"),
        ),
        (
            SettingId::RenderStyleSsrClarity,
            ("Clarity", "Netteté"),
            ("keep the reflection sharp", "garde le reflet net"),
        ),
        (
            SettingId::RenderStyleSsrLength,
            ("Length", "Longueur"),
            ("How far the reflection reaches", "Portée du reflet"),
        ),
        (
            SettingId::RenderStyleSsrQuality,
            ("Reflection Quality", "Qualité des reflets"),
            ("fewer gaps", "moins de trous"),
        ),
    ];
    assert_eq!(GROUP.len(), labels_and_help.len());

    for (id, (english, french), (english_help, french_help)) in labels_and_help {
        let descriptor = zax_settings::catalogue::descriptor(id);
        for (language, label, fragment) in [
            (UiLanguage::English, english, english_help),
            (UiLanguage::French, french, french_help),
        ] {
            strings::set_language(language);
            assert_eq!(label, zax_app::ui::text::label_of(descriptor), "{id:?}");
            let help = options::help_of(id).expect("every one of the thirteen has a help sentence");
            assert!(
                help.contains(fragment),
                "{id:?} in {language:?}: {help:?} should mention {fragment:?}"
            );
        }
    }
    strings::set_language(UiLanguage::English);
}

/// REQ-SCN-079 and AMD-055 §4: the four lists publish their named values (REQ-PERS-081), and they
/// do it **without** losing the normalisation — a stepped domain whose rungs are named, as
/// `scene.arc_center` is, so a write of 9 lands on 5 rather than on the default.
#[test]
fn req_scn_079_the_four_lists_name_their_values_and_keep_their_rungs() {
    let values = |id: SettingId| -> Vec<String> {
        zax_settings::catalogue::descriptor(id)
            .domain()
            .options()
            .expect("rungs")
            .iter()
            .map(|option| option.label.clone())
            .collect()
    };

    assert_eq!(
        ["Off", "MSVO", "Depth"],
        values(SettingId::RenderStyleAo)[..]
    );
    assert_eq!(
        ["Low", "Medium", "High"],
        values(SettingId::RenderStyleAoQuality)[..]
    );
    assert_eq!(
        ["Off", "Every 2 voxels", "Every 4 voxels", "Every 8 voxels"],
        values(SettingId::RenderStyleVoxelGrid)[..]
    );
    assert_eq!(
        ["Low", "Medium", "High", "Ultra"],
        values(SettingId::RenderStyleSsrQuality)[..]
    );

    // The eight ladders of 1 to 5 keep their numeric rungs: §4 gives them no value labels.
    assert_eq!(
        ["1", "2", "3", "4", "5"],
        values(SettingId::RenderStyleAoStrength)[..]
    );

    // The bounds, the step and the snapping are the stepped domain's, on all twelve.
    for id in options::STYLE {
        let domain = zax_settings::catalogue::descriptor(id).domain();
        assert_eq!(Some(1.0), domain.step(), "{id:?}");
        assert!(
            domain.minimum().is_some() && domain.maximum().is_some(),
            "{id:?}"
        );
    }
    let mut model = enhanced();
    model.set(SettingId::RenderStyleVoxelGrid, SettingValue::Int(4));
    assert_eq!(3, model.get(SettingId::RenderStyleVoxelGrid).as_int());

    // The translations of the named values, both languages (AMD-055 §4's table).
    for (language, off, quality, every_two, ultra) in [
        (
            UiLanguage::English,
            "Off",
            "Medium",
            "Every 2 voxels",
            "Ultra",
        ),
        (
            UiLanguage::French,
            "Désactivée",
            "Moyenne",
            "Tous les 2 voxels",
            "Ultra",
        ),
    ] {
        strings::set_language(language);
        assert_eq!(off, zax_app::ui::text::label_of_option("Off"));
        assert_eq!(quality, zax_app::ui::text::label_of_option("Medium"));
        assert_eq!(
            every_two,
            zax_app::ui::text::label_of_option("Every 2 voxels")
        );
        assert_eq!(ultra, zax_app::ui::text::label_of_option("Ultra"));
    }
    strings::set_language(UiLanguage::English);
}

/// AMD-055 §4 shows `spl.pixel_shadow` and `spl.reflections` as a **check box**; the model keeps
/// them as the whole numbers Lua reads and writes.
#[test]
fn req_scn_079_the_two_switches_are_check_boxes_over_whole_numbers() {
    assert_eq!(
        vec![
            SettingId::RenderStylePixelShadow,
            SettingId::RenderStyleReflections
        ],
        options::SWITCHES.to_vec()
    );
    for id in options::SWITCHES {
        let descriptor = zax_settings::catalogue::descriptor(id);
        assert_eq!(
            zax_settings::meta::SettingKind::Int,
            descriptor.kind(),
            "{id:?} stays a whole number"
        );
        assert_eq!(Some(0.0), descriptor.domain().minimum(), "{id:?}");
        assert_eq!(Some(1.0), descriptor.domain().maximum(), "{id:?}");
    }
}

/// The window **draws** the group: the collapsible header, the two check boxes of §4 and the
/// greyed `Thickness` all go through the interface at least once, with the twelve rows visible,
/// and none of the thirteen values moves by being drawn.
#[test]
fn req_ui_061_the_window_draws_the_style_group() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    harness.host.update_settings(|model| {
        model.set(SettingId::RenderVisualStyle, SettingValue::Int(4));
        model.set(SettingId::RenderStyleAo, SettingValue::Int(2));
        model.set(SettingId::RenderStyleReflections, SettingValue::Int(1));
        model.set(SettingId::RenderStylePixelShadow, SettingValue::Int(1));
    });
    harness.ui.state.options = true;
    harness.draw_frames(3);

    assert!(harness.ui.state.options);
    assert_eq!(
        4,
        harness
            .host
            .settings()
            .get(SettingId::RenderVisualStyle)
            .as_int()
    );
    assert_eq!(
        2,
        harness
            .host
            .settings()
            .get(SettingId::RenderStyleAo)
            .as_int()
    );
    assert_eq!(
        1,
        harness
            .host
            .settings()
            .get(SettingId::RenderStyleReflections)
            .as_int()
    );
    assert_eq!(
        1,
        harness
            .host
            .settings()
            .get(SettingId::RenderStylePixelShadow)
            .as_int()
    );
    assert_eq!(
        3,
        harness
            .host
            .settings()
            .get(SettingId::RenderStyleAoThickness)
            .as_int(),
        "a greyed row is drawn and writes nothing"
    );
}

/// The brief's handheld condition: the thirteen are offered on the console's advanced options
/// page, which picks up every steppable setting of the four desktop tabs by construction — a
/// powerful Linux handheld or Android device may afford an effect, and the defaults stay
/// economical (`handheld::defaults`).
#[test]
fn req_scn_078_the_handheld_advanced_page_offers_the_style_and_its_parameters() {
    let rows = advanced::rows();
    for id in GROUP {
        assert!(
            rows.contains(&OptionRow::Setting(id)),
            "{id:?} is on the handheld's advanced page"
        );
    }

    // And the page can step each one, which is what putting it there is for.
    let mut model = SettingsModel::new();
    advanced::step(SettingId::RenderVisualStyle, &mut model, 1);
    assert_eq!(
        4,
        model.get(SettingId::RenderVisualStyle).as_int(),
        "Classic then Enhanced"
    );
    advanced::step(SettingId::RenderStyleVoxelGrid, &mut model, 1);
    assert_eq!(1, model.get(SettingId::RenderStyleVoxelGrid).as_int());
    advanced::step(SettingId::RenderStyleVoxelGrid, &mut model, -1);
    assert_eq!(0, model.get(SettingId::RenderStyleVoxelGrid).as_int());

    // The value the page shows is the named one, not the number.
    assert_eq!(
        "Off",
        advanced::value_text(SettingId::RenderStyleVoxelGrid, &model)
    );
    advanced::step(SettingId::RenderStyleAo, &mut model, 2);
    assert_eq!(
        "Depth",
        advanced::value_text(SettingId::RenderStyleAo, &model)
    );
    // A ladder shows its number, having no named rungs.
    assert_eq!(
        "3",
        advanced::value_text(SettingId::RenderStyleAoStrength, &model)
    );
}
