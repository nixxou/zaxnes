//! Records the `target-cpu` of the build flags, for the frame profiler's start-up line.

fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
    let flags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    let mut parts = flags.split('\u{1f}');
    let mut cpu = None;
    while let Some(part) = parts.next() {
        let codegen = if part == "-C" {
            parts.next()
        } else {
            part.strip_prefix("-C")
        };
        if let Some(value) = codegen.and_then(|option| option.strip_prefix("target-cpu=")) {
            cpu = Some(value.to_owned());
        }
    }
    if let Some(cpu) = cpu {
        println!("cargo:rustc-env=ZAXNES_TARGET_CPU={cpu}");
    }
}
