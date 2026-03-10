fn main() {
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
