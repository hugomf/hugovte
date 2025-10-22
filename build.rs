fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
        
        // Get GTK4 include paths from pkg-config
        let gtk_includes = std::process::Command::new("pkg-config")
            .args(["--cflags", "gtk4"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        let mut build = cc::Build::new();
        build.file("macos_bridge.m");
        build.flag("-fobjc-arc");
        
        // Parse include paths from pkg-config
        for flag in gtk_includes.split_whitespace() {
            if let Some(path) = flag.strip_prefix("-I") {
                build.include(path);
            }
        }
        // Add common GTK include paths as fallback
        let common_paths = [
            "/opt/homebrew/include/gtk-4.0",
            "/opt/homebrew/include/glib-2.0", 
            "/opt/homebrew/lib/glib-2.0/include",
            "/opt/homebrew/include/pango-1.0",
            "/opt/homebrew/include/cairo",
            "/opt/homebrew/include/gdk-pixbuf-2.0",
            "/opt/homebrew/include/harfbuzz",
        ];
        
        for path in &common_paths {
            if std::path::Path::new(path).exists() {
                build.include(path);
            }
        }

        build.compile("macos_bridge");
        
        println!("cargo:rerun-if-changed=macos_bridge.m");
    }
}