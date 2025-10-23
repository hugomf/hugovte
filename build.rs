fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
        
        // Get GTK4 libraries and paths from pkg-config
        let gtk_config = std::process::Command::new("pkg-config")
            .args(["--libs", "gtk4"])
            .output()
            .expect("Failed to run pkg-config for gtk4");
        
        let gtk_libs = String::from_utf8(gtk_config.stdout)
            .expect("Invalid UTF-8 in pkg-config output");
        
        // Parse and add GTK library links
        for flag in gtk_libs.split_whitespace() {
            if let Some(lib) = flag.strip_prefix("-l") {
                println!("cargo:rustc-link-lib={}", lib);
            } else if let Some(path) = flag.strip_prefix("-L") {
                println!("cargo:rustc-link-search=native={}", path);
            }
        }

        // Get GTK4 include paths
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

        // Get the output directory and compile
        let out_dir = std::env::var("OUT_DIR").unwrap();
        build.out_dir(&out_dir).compile("macos_bridge");
        
        // CRITICAL: Add the output directory to the linker search path
        println!("cargo:rustc-link-search=native={}", out_dir);
        
        // CRITICAL: Link the static library
        println!("cargo:rustc-link-lib=static=macos_bridge");
        
        println!("cargo:rerun-if-changed=macos_bridge.m");
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        // For non-macOS platforms, do nothing
        println!("cargo:warning=macOS bridge not built on non-macOS platform");
    }
}