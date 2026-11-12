pub mod config;
pub mod controllers;
pub mod db;
pub mod engine;
pub mod models;
pub mod services;

/// Per-request CSP nonce stored in Axum request extensions.
/// Controllers extract this to inject `nonce="..."` on inline `<script>` tags.
#[derive(Clone)]
pub struct CspNonce(pub String);