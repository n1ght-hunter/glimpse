//! macOS screen capture backend for glimpse.
//!
//! Uses ScreenCaptureKit via the `cidre` crate. Audio is captured through
//! the same ScreenCaptureKit stream.
//!
//! TODO: Port engine, targets, and platform code from scap/src/capturer/engine/mac/,
//! scap/src/targets/mac/, and scap/src/utils/mac/ when building on macOS.
