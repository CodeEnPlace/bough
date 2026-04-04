use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let nested_target = out_dir.join("bough-cli-build");
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let bin_path = nested_target.join(profile).join("bough");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut cmd = std::process::Command::new(&cargo);
    cmd.args(["build", "--quiet", "-p", "bough-cli", "--bin", "bough"]);
    if profile == "release" {
        cmd.arg("--release");
    }
    cmd.env("CARGO_TARGET_DIR", &nested_target);
    cmd.current_dir(workspace_root);
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to invoke `{cargo} build -p bough-cli`: {e}"));
    assert!(status.success(), "building bough-cli failed");

    println!("cargo::rustc-env=BOUGH_BIN={}", bin_path.to_string_lossy());

    // Re-run when bough-cli sources change so the binary stays fresh.
    let cli_src = workspace_root.join("crates").join("bough-cli").join("src");
    rerun_if_dir_changed(&cli_src);
    println!(
        "cargo::rerun-if-changed={}",
        workspace_root.join("crates").join("bough-cli").join("Cargo.toml").to_string_lossy()
    );
    println!("cargo::rerun-if-changed=build.rs");
}

fn rerun_if_dir_changed(dir: &Path) {
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        println!("cargo::rerun-if-changed={}", d.to_string_lossy());
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                println!("cargo::rerun-if-changed={}", path.to_string_lossy());
            }
        }
    }
}
