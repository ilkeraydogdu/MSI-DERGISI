//! Embedded Pure Rust Editorial Studio core models and WebAssembly frontend.

pub mod common;

#[cfg(any(target_arch = "wasm32", feature = "editorial-wasm"))]
pub mod wasm;
