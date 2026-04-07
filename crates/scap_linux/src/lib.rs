//! Linux screen capture backend for glimpse.
//!
//! Uses PipeWire for video capture and the D-Bus ScreenCast portal for
//! permission and stream setup.

mod engine;
pub mod error;
pub mod platform;
#[allow(dead_code)]
pub mod portal;
pub mod targets;

use std::sync::mpsc;

use glimpse_core::{CaptureOptions, CaptureStream, Frame};

use crate::engine::LinuxCapturer;
use crate::error::LinCapError;

pub use crate::platform::LinuxPlatform;

/// Pull-based Linux screen capture stream backed by PipeWire.
pub struct LinuxScreenCapture {
    engine: LinuxCapturer,
    rx: mpsc::Receiver<Frame>,
}

impl LinuxScreenCapture {
    pub fn new(options: CaptureOptions) -> Self {
        let (tx, rx) = mpsc::channel();
        let engine = engine::create_capturer(&options, tx);
        Self { engine, rx }
    }
}

impl CaptureStream for LinuxScreenCapture {
    type Error = LinCapError;

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
}
