#[test]
fn rendering_runtime_path_uses_std_chunks_only() {
    let rendering_src = include_str!("../src/rendering.rs");

    assert!(
        !rendering_src.contains("use rayon::prelude::*;"),
        "runtime rendering should not import rayon prelude"
    );
    assert!(
        !rendering_src.contains(".par_chunks_mut("),
        "runtime rendering should use std slice chunks_mut instead of rayon par_chunks_mut"
    );
    assert!(
        rendering_src.contains("processing chunk"),
        "rendering chunk comment should describe processing, not rayon work"
    );
}

#[test]
fn main_does_not_initialize_global_rayon_pool() {
    let main_src = include_str!("../src/main.rs");

    assert!(
        !main_src.contains("rayon::ThreadPoolBuilder::new()"),
        "main should not initialize a global rayon thread pool for runtime rendering"
    );
}

#[test]
fn cargo_keeps_rayon_as_dev_dependency_only() {
    let cargo_toml = include_str!("../Cargo.toml");
    let dependencies = cargo_toml
        .split("[dependencies]")
        .nth(1)
        .and_then(|section| section.split("[dev-dependencies]").next())
        .expect("Cargo.toml should have a [dependencies] section before [dev-dependencies]");
    let dev_dependencies = cargo_toml
        .split("[dev-dependencies]")
        .nth(1)
        .and_then(|section| section.split("\n[").next())
        .expect("Cargo.toml should have a [dev-dependencies] section");

    assert!(
        dev_dependencies.contains("rayon = \"1.10\""),
        "rayon should remain available for benches under dev-dependencies"
    );
    assert!(
        !dependencies.contains("rayon = \"1.10\""),
        "rayon must not remain in release dependencies"
    );
}
