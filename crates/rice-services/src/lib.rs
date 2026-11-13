//! File-backed services of the rice backend. No Windows APIs, so everything
//! here is covered by `cargo test`.

pub mod banners;
pub mod config;
pub mod fsutil;
pub mod imaging;
pub mod themes;
pub mod watch;
