//! Pure Rust CLI task (`--build-editorial-wasm`) compiling the embedded
//! `msi_dergisi_platform` WebAssembly editorial client into `storage/editorial-pkg/`.

use std::process::Command;

/// Compiles `src/editorial/wasm` to `wasm32-unknown-unknown` and generates
/// WebAssembly bindings under `storage/editorial-pkg/`.
pub fn run_build_editorial_wasm() -> Result<(), String> {
    let cargo_bin = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(&cargo_bin)
        .args([
            "build",
            "--lib",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--no-default-features",
            "--features",
            "editorial-wasm",
            "--config",
            "profile.release.strip=\"none\"",
        ])
        .status()
        .map_err(|e| format!("Failed to invoke cargo ({cargo_bin}): {e}"))?;
    if !status.success() {
        return Err(format!("cargo wasm32 build exited with status {status}"));
    }

    std::fs::create_dir_all("storage/editorial-pkg")
        .map_err(|e| format!("Failed to create storage/editorial-pkg: {e}"))?;

    let wasm_in = "target/wasm32-unknown-unknown/release/msi_dergisi_platform.wasm";
    let bindgen_status = Command::new("wasm-bindgen")
        .args([
            "--target",
            "web",
            "--no-typescript",
            "--out-name",
            "editorial_studio",
            "--out-dir",
            "storage/editorial-pkg",
            wasm_in,
        ])
        .status()
        .map_err(|e| format!("Failed to invoke wasm-bindgen: {e}"))?;
    if !bindgen_status.success() {
        return Err(format!("wasm-bindgen exited with status {bindgen_status}"));
    }

    println!("Built embedded editorial WASM artifacts in storage/editorial-pkg/");
    Ok(())
}
