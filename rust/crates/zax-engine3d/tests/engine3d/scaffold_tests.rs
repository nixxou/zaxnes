//! Architecture checks for zax-engine3d: the conversion engine produces abstract data and must
//! not depend on a windowing, graphics, audio or input API (REQ-ARCH-002).

/// Crate name prefixes that must never appear among zax-engine3d's dependencies: the engine
/// emits abstract meshes and must not know about a graphics API.
const FORBIDDEN_PREFIXES: [&str; 6] = ["glow", "sdl2", "imgui", "wgpu", "winit", "glutin"];

#[test]
fn module_is_identified() {
    assert_eq!("Zax.Engine3D", zax_engine3d::MODULE_NAME);
    assert_ne!(zax_engine3d::MODULE_ROLE.trim(), "");
}

#[test]
fn req_arch_002_the_assembly_has_no_graphics_dependency() {
    let manifest = include_str!("../../Cargo.toml");
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .unwrap();
    let offenders: Vec<&str> = dependencies
        .lines()
        .filter(|line| {
            FORBIDDEN_PREFIXES
                .iter()
                .any(|prefix| line.trim_start().to_ascii_lowercase().starts_with(prefix))
        })
        .collect();

    assert!(offenders.is_empty(), "{offenders:?}");
}
