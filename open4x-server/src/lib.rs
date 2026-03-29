#![allow(dead_code)]

/// Shared wire-protocol types (compiles for both native and wasm32 targets).
pub mod types;

/// Server-only modules (Axum, game state, WebSocket, REST API).
#[cfg(feature = "ssr")]
pub mod server;

/// Client-only frontend components (Leptos CSR).
#[cfg(feature = "csr")]
pub mod components;

/// Client-only page components.
#[cfg(feature = "csr")]
pub mod pages;

/// Client-only tab components for the game interface.
#[cfg(feature = "csr")]
pub mod tabs;
