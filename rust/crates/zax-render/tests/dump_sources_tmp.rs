//! Temporary (RU-5): dumps every assembled shader source, so a step can prove byte identity of
//! the ones it must not touch. Deliberately not committed; see docs/progress/RU-5.md, step 1.
use zax_render::shaders;

#[test]
fn dump() {
    let Some(directory) = std::env::var_os("ZAX_DUMP_DIR") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    let mut sources: Vec<(String, String)> = Vec::new();
    for solid in [false, true] {
        for srgb in [false, true] {
            for bend in [false, true] {
                for centroid in [false, true] {
                    for grid in [false, true] {
                        for shadow in [false, true] {
                            push(
                                &mut sources,
                                &shaders::scene(shaders::SceneVariant {
                                    solid,
                                    srgb_palette: srgb,
                                    bend,
                                    centroid,
                                    voxel_grid: grid,
                                    pixel_shadow: shadow,
                                }),
                            );
                        }
                    }
                }
            }
        }
    }
    for bend in [false, true] {
        push(&mut sources, &shaders::shadow(bend));
        for grid in [false, true] {
            push(&mut sources, &shaders::backdrop(bend, grid));
        }
    }
    for (name, source) in sources {
        std::fs::write(directory.join(name), source).unwrap();
    }
}

fn push(into: &mut Vec<(String, String)>, source: &shaders::ProgramSource) {
    for embedded in [false, true] {
        let suffix = if embedded { "gles" } else { "desktop" };
        into.push((
            format!("{}.{suffix}.vert", source.name),
            shaders::assemble(&source.vertex, false, embedded),
        ));
        into.push((
            format!("{}.{suffix}.frag", source.name),
            shaders::assemble(&source.fragment, true, embedded),
        ));
    }
}
