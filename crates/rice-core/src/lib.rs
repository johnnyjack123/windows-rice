//! Platform-independent core of the rice backend.
//!
//! Everything here runs under `cargo test` on any machine: the wire types of
//! `web/src/backend/contract.ts`, the settings patch semantics, island
//! geometry (hit testing, edge detection, placement) and input validation.

pub mod contract;
pub mod devices;
pub mod edge;
pub mod error;
pub mod geometry;
pub mod media;
pub mod patch;
pub mod settings;
pub mod shell;
pub mod theme;
pub mod time;

pub use error::{BackendError, BackendResult, ErrorCode};
