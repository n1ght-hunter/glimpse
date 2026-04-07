use std::time::SystemTime;

use glimpse_core::frame::{BGRAFrame, VideoFrame};
use glimpse_core::{CaptureStream, Frame as CoreFrame};

use crate::capture::{Capture, CaptureConfig};
use crate::error::Error;

/// Wrapper around [`Capture`] that implements [`CaptureStream`].
///
/// Converts obs-hook's borrowed `Frame<'_>` into owned `glimpse_core::Frame`
/// by copying the pixel data once (the data is already CPU-side from the
/// D3D11 staging texture).
pub struct ObsGameCapture {
    capture: Capture,
}

impl ObsGameCapture {
    pub fn new(config: CaptureConfig) -> Result<Self, Error> {
        let capture = Capture::start(config)?;
        Ok(Self { capture })
    }
}

// DXGI_FORMAT_B8G8R8A8_UNORM
const DXGI_FORMAT_B8G8R8A8_UNORM: u32 = 87;

impl CaptureStream for ObsGameCapture {
    type Error = Error;

    fn start(&mut self) -> Result<(), Self::Error> {
        // Capture::start() already initializes everything in the constructor
        Ok(())
    }

    fn next_frame(&mut self) -> Result<Option<CoreFrame>, Self::Error> {
        let raw = self.capture.capture_frame()?;

        let video = match raw.format {
            DXGI_FORMAT_B8G8R8A8_UNORM => VideoFrame::BGRA(BGRAFrame {
                display_time: SystemTime::now(),
                width: raw.width as i32,
                height: raw.height as i32,
                data: raw.data.to_vec(),
            }),
            // For unknown formats, still produce a BGRA frame and let the
            // caller inspect the raw bytes. The pixel layout may differ but
            // dimensions and data are valid.
            _ => {
                tracing::warn!(format = raw.format, "unknown DXGI format, wrapping as BGRA");
                VideoFrame::BGRA(BGRAFrame {
                    display_time: SystemTime::now(),
                    width: raw.width as i32,
                    height: raw.height as i32,
                    data: raw.data.to_vec(),
                })
            }
        };

        Ok(Some(CoreFrame::Video(video)))
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        self.capture.stop()
    }
}
