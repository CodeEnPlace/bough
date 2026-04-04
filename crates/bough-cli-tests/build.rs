fn main() {
    // Ensure the bough binary is built before running tests.
    // The binary will be at <workspace>/target/<profile>/bough.
    // We use CARGO_MANIFEST_DIR to find the workspace root.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let bin_path = workspace_root.join("target").join(profile).join("bough");
    println!(
        "cargo::rustc-env=BOUGH_BIN={}",
        bin_path.to_string_lossy()
    );
    // Re-run if the binary changes.
    println!("cargo::rerun-if-changed={}", bin_path.to_string_lossy());
}
