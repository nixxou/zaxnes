//! The parameters of real presets, read from a copy of libretro/slang-shaders named by
//! `ZAXNES_SLANG_SHADERS` (skipped without it).
#![cfg(feature = "shaders")]

use std::path::PathBuf;

use zax_platform::slang_chain::preset_parameters;

fn collection() -> Option<PathBuf> {
    std::env::var_os("ZAXNES_SLANG_SHADERS").map(PathBuf::from)
}

#[test]
fn crt_easymode_lists_its_parameters_in_declaration_order() {
    let Some(root) = collection() else {
        eprintln!("skipped: ZAXNES_SLANG_SHADERS is not set");
        return;
    };
    let parameters = preset_parameters(&root.join("crt/crt-easymode.slangp")).unwrap();
    let names: Vec<&str> = parameters.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names.first(), Some(&"SHARPNESS_H"));
    assert_eq!(names.get(2), Some(&"MASK_STRENGTH"));
    assert_eq!(names.len(), 17);
    let mask = &parameters[2];
    assert_eq!(mask.description, "Mask Strength");
    assert!((mask.initial - 0.3).abs() < 1.0e-6);
    assert!((mask.step - 0.01).abs() < 1.0e-6);
}

#[test]
fn mega_bezel_reads_with_its_headings() {
    let Some(root) = collection() else {
        eprintln!("skipped: ZAXNES_SLANG_SHADERS is not set");
        return;
    };
    let parameters =
        preset_parameters(&root.join("bezel/Mega_Bezel/Presets/MBZ__3__STD.slangp")).unwrap();
    assert!(parameters.len() > 100, "{}", parameters.len());
    assert!(parameters
        .iter()
        .any(zax_platform::slang_chain::ParameterInfo::is_heading));
    // Declared order: the first one is not simply the alphabetical first.
    let mut sorted: Vec<&str> = parameters.iter().map(|p| p.name.as_str()).collect();
    sorted.sort_unstable();
    assert_ne!(parameters[0].name, sorted[0]);
}

/// A combination (owner request): nnedi3 then a CRT. The passes follow one another, numbered on,
/// and nnedi3's last pass, which resampled to the window, draws at the size of its input instead,
/// so the CRT reads the enlarged picture rather than the window.
#[test]
fn a_combination_puts_the_passes_one_after_the_other() {
    use librashader::presets::ScaleType;
    use zax_platform::slang_chain::parse_combination;

    let Some(root) = collection() else {
        eprintln!("skipped: ZAXNES_SLANG_SHADERS is not set");
        return;
    };
    let first = root.join("edge-smoothing/nnedi3/nnedi3-nns32-4x-rgb.slangp");
    let second = root.join("crt/crt-easymode.slangp");
    let alone = parse_combination(&first).unwrap();
    assert!(matches!(
        alone.passes.last().unwrap().meta.scaling.x.scale_type,
        ScaleType::Viewport
    ));
    let both = PathBuf::from(format!("{}|{}", first.display(), second.display()));
    let combined = parse_combination(&both).unwrap();
    let crt = parse_combination(&second).unwrap();
    assert_eq!(combined.passes.len(), alone.passes.len() + crt.passes.len());
    assert_eq!(
        combined.pass_count,
        i32::try_from(combined.passes.len()).unwrap()
    );
    let joint = &combined.passes[alone.passes.len() - 1].meta;
    assert!(matches!(joint.scaling.x.scale_type, ScaleType::Input));
    assert!(matches!(joint.scaling.y.scale_type, ScaleType::Input));
    for (index, pass) in combined.passes.iter().enumerate() {
        assert_eq!(pass.meta.id, i32::try_from(index).unwrap());
    }
    // The parameters of both are offered.
    let names: Vec<String> = preset_parameters(&both)
        .unwrap()
        .into_iter()
        .map(|parameter| parameter.name)
        .collect();
    assert!(names.iter().any(|name| name == "NNEDI_CSHIFT_PIXELS"));
    assert!(names.iter().any(|name| name == "SHARPNESS_H"));
}
