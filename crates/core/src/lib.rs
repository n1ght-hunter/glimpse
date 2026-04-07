//! Core types and traits for the glimpse capture library.
//!
//! This crate is platform-agnostic — it contains frame types, target
//! abstractions, geometry helpers, and the [`CaptureStream`] trait that
//! platform-specific backends implement.

pub mod capture;
pub mod frame;
pub mod geometry;
pub mod target;

pub use capture::{CaptureOptions, CaptureStream, PlatformCapture};
pub use frame::{AudioFormat, AudioFrame, Frame, FrameType, VideoFrame};
pub use geometry::{Area, Point, Resolution, Size};
pub use target::{Display, RawHandle, Target, Window};
