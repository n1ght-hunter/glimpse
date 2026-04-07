//! Cross-platform screen and game capture library.
//!
//! Re-exports core types from [`glimpse_core`] and platform-specific backends
//! behind feature flags.
//!
//! # Features
//!
//! - `screen` (default) — screen capture using the OS-native API
//! - `game` (Windows only) — game capture via OBS graphics-hook injection

pub use glimpse_core::*;

/// Screen capture backend for the current platform.
#[cfg(all(target_os = "windows", feature = "screen"))]
pub use glimpse_scap_win as screen;

/// Screen capture backend for the current platform.
#[cfg(all(target_os = "macos", feature = "screen"))]
pub use glimpse_scap_mac as screen;

/// Screen capture backend for the current platform.
#[cfg(all(target_os = "linux", feature = "screen"))]
pub use glimpse_scap_linux as screen;

/// Game capture backend (OBS graphics-hook, Windows only).
#[cfg(all(target_os = "windows", feature = "game"))]
pub use glimpse_obs_hook as game;
