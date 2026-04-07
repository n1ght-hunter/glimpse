pub mod engine;

use std::{error::Error, sync::mpsc};

use engine::ChannelItem;

use crate::{
    frame::{Frame, FrameType, VideoFrame},
    has_permission, is_supported,
    targets::Target,
};

pub use engine::get_output_frame_size;
pub use glimpse_core::geometry::{Area, Point, Resolution, Size};

/// Options passed to the screen capturer
#[derive(Debug, Default, Clone)]
pub struct Options {
    pub fps: u32,
    pub show_cursor: bool,
    pub show_highlight: bool,
    pub target: Option<Target>,
    pub crop_area: Option<Area>,
    pub output_type: FrameType,
    pub output_resolution: Resolution,
    // excluded targets will only work on macOS
    pub excluded_targets: Option<Vec<Target>>,
    /// Only implemented for Windows and macOS currently
    pub captures_audio: bool,
    pub exclude_current_process_audio: bool,
}

/// Screen capturer class
pub struct Capturer {
    engine: engine::Engine,
    rx: mpsc::Receiver<ChannelItem>,
}

#[derive(Debug)]
pub enum CapturerBuildError {
    NotSupported,
    PermissionNotGranted,
}

impl std::fmt::Display for CapturerBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapturerBuildError::NotSupported => write!(f, "Screen capturing is not supported"),
            CapturerBuildError::PermissionNotGranted => {
                write!(f, "Permission to capture the screen is not granted")
            }
        }
    }
}

impl Error for CapturerBuildError {}

impl Capturer {
    /// Build a new [Capturer] instance with the provided options
    pub fn build(options: Options) -> Result<Capturer, CapturerBuildError> {
        if !is_supported() {
            return Err(CapturerBuildError::NotSupported);
        }

        if !has_permission() {
            return Err(CapturerBuildError::PermissionNotGranted);
        }

        let (tx, rx) = mpsc::channel();
        let engine = engine::Engine::new(&options, tx);

        Ok(Capturer { engine, rx })
    }

    // TODO
    // Prevent starting capture if already started
    /// Start capturing the frames
    pub fn start_capture(&mut self) {
        self.engine.start();
    }

    /// Stop the capturer
    pub fn stop_capture(&mut self) {
        self.engine.stop();
    }

    /// Get the next captured frame
    pub fn get_next_frame(&self) -> Result<Frame, mpsc::RecvError> {
        loop {
            let res = self.rx.recv()?;

            if let Some(frame) = self.engine.process_channel_item(res) {
                return Ok(frame);
            }
        }
    }

    /// Returns the target being captured, if one was specified
    pub fn target(&self) -> Option<&Target> {
        self.engine.target()
    }

    /// Get the dimensions the frames will be captured in
    pub fn get_output_frame_size(&mut self) -> [u32; 2] {
        self.engine.get_output_frame_size()
    }

    pub fn raw(&self) -> RawCapturer {
        RawCapturer { capturer: self }
    }
}

pub struct RawCapturer<'a> {
    capturer: &'a Capturer,
}
