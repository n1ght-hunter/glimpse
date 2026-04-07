#![cfg(target_os = "windows")]

mod binaries;
mod capture;
mod constants;
mod dx;
pub mod error;
mod hook;
mod ipc;
mod offsets;
mod shmem;
pub mod types;

pub use capture::{Capture, CaptureConfig};
pub use dx::Frame;
pub use error::{Error, Result};
pub use hook::WindowTarget;
pub use types::{Bgra8, GraphicOffsets, HookInfo};
