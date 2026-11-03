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
/// `EXCLUDED` with this lot, which is what their effect earns them.
#[test]
fn req_ui_061_the_thirteen_settings_are_rows_of_the_options_window() {
    let model = OptionsModel::new();
    let shown: Vec<SettingId> = model
        .all_rows()
        .map(|row| row.descriptor.id())
        .filter(|id| GROUP.contains(id))
        .collect();
    assert_eq!(GROUP.to_vec(), shown, "all thirteen, in the §4 order");

    for id in GROUP {
        assert!(
            !options::EXCLUDED.contains(&id),
            "{id:?} is no longer excluded"
        );
    }

    // They belong to the Video tab, which is where the Render theme goes (REQ-UI-202).
    let video: Vec<SettingId> = model
        .rows(OptionsTab::Video)
        .iter()
        .map(|row| row.descriptor.id())
        .collect();
    for id in GROUP {
        assert!(video.contains(&id), "{id:?} is on the Video tab");
    }
}

/// SPEC-10 REQ-UI-061 as amended: the `Style` row comes **after the `Skybox` row** — which
/// carries its colour button with it (REQ-UI-063) — and the twelve parameters follow it.
#[test]
fn req_ui_061_the_style_row_comes_after_the_skybox_row() {
    let model = OptionsModel::new();
    let video: Vec<SettingId> = model
        .rows(OptionsTab::Video)
        .iter()
        .map(|row| row.descriptor.id())
        .collect();
    let at = |wanted: SettingId| {
        video
            .iter()
            .position(|id| *id == wanted)
            .unwrap_or_else(|| panic!("{wanted:?} is a row of the Video tab"))
    };

    assert_eq!(
        at(SettingId::SceneCustomBgColor) + 1,
        at(SettingId::RenderVisualStyle),
        "the style row follows the skybox row and its colour"
    );
    assert!(at(SettingId::SceneSkybox) < at(SettingId::RenderVisualStyle));
    // And the twelve follow it, contiguously, in their own order.
    for (offset, id) in GROUP.iter().enumerate() {
        assert_eq!(
            at(SettingId::RenderVisualStyle) + offset,
            at(*id),
            "{id:?} is the {offset}-th row of the group"
        );
    }
    // The catalogue's own order — the order of the settings file — is the other one, which is
    // the whole reason the window reorders: `render.visual_style` is written long before
    // `scene.skybox`.
    let catalogue: Vec<SettingId> = zax_settings::catalogue::descriptors()
        .map(zax_settings::meta::SettingDescriptor::id)
        .collect();
    let where_in = |wanted: SettingId| catalogue.iter().position(|id| *id == wanted).expect("key");
    assert!(where_in(SettingId::RenderVisualStyle) < where_in(SettingId::SceneSkybox));
}

/// AMD-055 §3 point 6: the group is collapsible and titled "Style", opened by its **first**
/// parameter and by no other row of any tab — and it is drawn only while the style is Enhanced.
#[test]
fn req_ui_061_only_the_first_parameter_opens_the_collapsible_group() {
    let model = OptionsModel::new();
    let opens: Vec<SettingId> = model
        .all_rows()
        .map(|row| row.descriptor.id())
        .filter(|id| options::style_heading(*id).is_some())
        .collect();
    assert_eq!(vec![SettingId::RenderStyleAo], opens);

    for (language, expected) in [
        (UiLanguage::English, "Style"),
        (UiLanguage::French, "Style"),
    ] {
        strings::set_language(language);
        assert_eq!(
            Some(expected.to_owned()),
            options::style_heading(SettingId::RenderStyleAo)
        );
    }
    strings::set_language(UiLanguage::English);

    assert!(
        !options::style_group_shown(&SettingsModel::new()),
        "in Classic the group is not drawn at all"
    );
    assert!(options::style_group_shown(&enhanced()));
    // The accepted identifier 1 selects Enhanced too (REQ-SCN-078), so it opens the group.
    let mut alias = SettingsModel::new();
    alias.set(SettingId::RenderVisualStyle, SettingValue::Int(1));
    assert!(options::style_group_shown(&alias));
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

/// REQ-SCN-079: `ao.thickness` is presented **greyed** for as long as the MSVO mode is rendered
/// as Depth (REQ-SCN-083's fallback **(D)**), with the reason beside it — and it is the only row
/// that is. The setting itself is kept and persisted: greying is a presentation, not a refusal.
#[test]
fn req_scn_079_the_thickness_is_greyed_while_msvo_is_rendered_as_depth() {
    // This lot renders MSVO as Depth: REQ-SCN-083's declared fallback (D). The day the port
    // lands, this is the assertion that sends whoever changes the constant to the greying rule.
    const { assert!(!zax_render::style::MSVO_PORTED) };

    let mut model = enhanced();
    for mode in [0, 1, 2] {
        model.set(SettingId::RenderStyleAo, SettingValue::Int(mode));
        let note = options::style_inert_note(SettingId::RenderStyleAoThickness, &model);
        assert!(
            note.is_some(),
            "greyed at spl.ao = {mode} while MSVO is not ported"
        );
        for other in GROUP {
            if other == SettingId::RenderStyleAoThickness {
                continue;
            }
            assert_eq!(
                None,
                options::style_inert_note(other, &model),
                "{other:?} is live"
            );
        }
    }

    for (language, fragment) in [
        (UiLanguage::English, "Depth occlusion"),
        (UiLanguage::French, "l'occlusion Depth"),
    ] {
        strings::set_language(language);
        let note = options::style_inert_note(SettingId::RenderStyleAoThickness, &model)
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
