//! Windows screen capture backend for glimpse.
//!
//! Uses the Windows.Graphics.Capture API via the `windows-capture` crate
//! for video, and `cpal` for audio loopback capture.

mod audio;
mod engine;
mod platform;
pub mod targets;

pub use platform::WinPlatform;
pub use targets::{DisplayExt, WindowExt};

use glimpse_core::frame::Frame;
use glimpse_core::{CaptureOptions, CaptureStream};
use std::sync::mpsc;

/// Windows-specific capture options.
pub struct WinCaptureOptions {
    pub base: CaptureOptions,
    pub exclude_current_process_audio: bool,
}

impl From<CaptureOptions> for WinCaptureOptions {
    fn from(base: CaptureOptions) -> Self {
        Self {
            base,
            exclude_current_process_audio: false,
        }
    }
}

/// Windows screen capture session implementing [`CaptureStream`].
pub struct WinScreenCapture {
    engine: engine::WCStream,
    rx: mpsc::Receiver<Frame>,
    options: WinCaptureOptions,
}

#[derive(Debug, thiserror::Error)]
pub enum WinCaptureError {
    #[error("screen capturing is not supported")]
    NotSupported,
    #[error("failed to create capturer: {0:?}")]
    Create(engine::CreateCapturerError),
    #[error("capture channel closed")]
    ChannelClosed,
}

impl WinScreenCapture {
    pub fn new(options: WinCaptureOptions) -> Result<Self, WinCaptureError> {
        let (tx, rx) = mpsc::channel();
        let engine = engine::create_capturer(&options, tx).map_err(WinCaptureError::Create)?;

        Ok(Self {
            engine,
            rx,
            options,
        })
    }
}

impl CaptureStream for WinScreenCapture {
    type Error = WinCaptureError;

    fn start(&mut self) -> Result<(), Self::Error> {
        self.engine.start_capture();
        Ok(())
    }

    fn next_frame(&mut self) -> Result<Option<Frame>, Self::Error> {
        match self.rx.recv() {
            Ok(frame) => Ok(Some(frame)),
            Err(_) => Ok(None),
        }
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        self.engine.stop_capture();
        Ok(())
    }

    fn output_size(&self) -> Option<[u32; 2]> {
        Some(engine::get_output_frame_size(&self.options))
    }
}
