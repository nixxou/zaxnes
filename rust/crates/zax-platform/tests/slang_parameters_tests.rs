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
