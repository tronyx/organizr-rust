//! The plugin interface of Organizr and its bundled plugins.

#[cfg(feature = "wasm-plugins")]
pub mod extension;
pub mod health_checks;
pub mod mail;
