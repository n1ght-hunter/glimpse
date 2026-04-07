//! macOS screen capture backend for glimpse.
//!
//! Uses ScreenCaptureKit via the `cidre` crate. Audio is captured through
//! the same ScreenCaptureKit stream.

mod engine;
pub mod ext;
pub mod pixel_buffer;
mod pixelformat;
pub mod platform;
pub mod targets;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;

use cidre::{arc, cm, sc};

use glimpse_core::Target;
use glimpse_core::capture::CaptureStream;
use glimpse_core::frame::Frame;

pub use ext::DirectDisplayIdExt;
pub use platform::MacPlatform;
pub use targets::{DisplayExt, WindowExt};

type ChannelItem = (arc::R<cm::SampleBuf>, sc::stream::OutputType);

/// macOS-specific capture options extending the base [`CaptureOptions`](glimpse_core::CaptureOptions).
#[derive(Debug, Default, Clone)]
pub struct MacCaptureOptions {
    pub base: glimpse_core::CaptureOptions,
    pub excluded_targets: Option<Vec<Target>>,
    pub exclude_current_process_audio: bool,
}

/// macOS screen capture stream backed by ScreenCaptureKit.
pub struct MacScreenCapture {
    _capturer: arc::R<engine::Capturer>,
    _error_handler: arc::R<engine::ErrorHandler>,
    stream: arc::R<sc::Stream>,
    error_flag: Arc<AtomicBool>,
    rx: mpsc::Receiver<ChannelItem>,
    options: MacCaptureOptions,
}

#[derive(thiserror::Error, Debug)]
pub enum MacCaptureError {
    #[error("capture stream error")]
    StreamError,
    #[error("channel disconnected")]
    ChannelDisconnected,
    #[error("{0}")]
    Engine(#[from] engine::CreateCapturerError),
}

impl MacScreenCapture {
    /// Build a new capture session. Does not start capturing until
    /// [`start`](CaptureStream::start) is called.
    pub fn new(options: MacCaptureOptions) -> Result<Self, MacCaptureError> {
        let (tx, rx) = mpsc::channel();
        let error_flag = Arc::new(AtomicBool::new(false));
        let (capturer, error_handler, stream) =
            engine::create_capturer(&options, tx, error_flag.clone())?;

        Ok(Self {
            _capturer: capturer,
            _error_handler: error_handler,
            stream,
            error_flag,
            rx,
            options,
        })
    }

    /// Get the dimensions the frames will be captured in.
    pub fn get_output_frame_size(&self) -> [u32; 2] {
        engine::get_output_frame_size(&self.options)
    }

    /// Receive the next raw sample buffer from the capture channel, checking
    /// the error flag between attempts.
    pub fn next_raw_sample(
        &self,
    ) -> Result<(arc::R<cm::SampleBuf>, sc::stream::OutputType), mpsc::RecvError> {
        use std::time::Duration;

        loop {
            if self.error_flag.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(mpsc::RecvError);
            }

            return match self.rx.recv_timeout(Duration::from_millis(10)) {
                Ok(v) => Ok(v),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => Err(mpsc::RecvError),
            };
        }
    }
}

impl CaptureStream for MacScreenCapture {
    type Error = MacCaptureError;

    fn start(&mut self) -> Result<(), Self::Error> {
        futures::executor::block_on(self.stream.start())
            .map_err(|_| MacCaptureError::StreamError)?;
        Ok(())
    }

    fn next_frame(&mut self) -> Result<Option<Frame>, Self::Error> {
        loop {
            let (sample, of_type) = self
                .rx
                .recv()
                .map_err(|_| MacCaptureError::ChannelDisconnected)?;

            if self.error_flag.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(MacCaptureError::StreamError);
            }

            if let Some(frame) =
                engine::process_sample_buffer(sample, of_type, self.options.base.output_type)
            {
                return Ok(Some(frame));
            }
        }
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        futures::executor::block_on(self.stream.stop())
            .map_err(|_| MacCaptureError::StreamError)?;
        Ok(())
    }

    fn output_size(&self) -> Option<[u32; 2]> {
        Some(self.get_output_frame_size())
    }
}
