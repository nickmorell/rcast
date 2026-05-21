fn main() {
    // Validate that required icon files exist
    let icon_files = vec![
        "assets/icons/icon-256x256.png",
        "assets/icons/icon-128x128.png",
        "assets/icons/icon-32x32.png",
        "assets/icons/icon.ico",
        "assets/icons/icon.icns",
    ];

    for icon_file in icon_files {
        if !std::path::Path::new(icon_file).exists() {
            panic!("Required icon file missing: {}", icon_file);
        }
    }

    println!("cargo:rerun-if-changed=assets/icons/");
}
