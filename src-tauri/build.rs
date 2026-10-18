fn main() {
    #[cfg(windows)]
    {
        // Test executables do not receive the application manifest that
        // tauri-build embeds into the real binary. Without the Common
        // Controls v6 dependency, loading comctl32.dll fails at startup with
        // STATUS_ENTRYPOINT_NOT_FOUND (TaskDialogIndirect).
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
    tauri_build::build()
}
