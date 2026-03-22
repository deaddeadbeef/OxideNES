fn main() {
    // Set version info at compile time
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".into());
    let is_release = std::env::var("OXIDENES_RELEASE").is_ok();

    if is_release {
        println!("cargo:rustc-env=OXIDENES_VERSION={}", version);
        println!("cargo:rustc-env=OXIDENES_BUILD_TYPE=release");
    } else {
        println!("cargo:rustc-env=OXIDENES_VERSION={}-dev", version);
        println!("cargo:rustc-env=OXIDENES_BUILD_TYPE=dev");
    }

    println!("cargo:rerun-if-env-changed=OXIDENES_RELEASE");

    #[cfg(target_os = "windows")]
    {
        if std::path::Path::new("icon.ico").exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon("icon.ico");
            if let Err(e) = res.compile() {
                eprintln!("Warning: Could not set icon: {}", e);
            }
        }
    }
}
