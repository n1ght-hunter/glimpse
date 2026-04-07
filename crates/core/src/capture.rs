use crate::frame::{Frame, FrameType};
use crate::geometry::{Area, Resolution};
use crate::target::{Display, Target};

/// Options for configuring a capture session.
#[derive(Debug, Default, Clone)]
pub struct CaptureOptions {
    pub fps: u32,
    pub show_cursor: bool,
    pub show_highlight: bool,
    pub target: Option<Target>,
    pub crop_area: Option<Area>,
    pub output_type: FrameType,
    pub output_resolution: Resolution,
    pub captures_audio: bool,
}

/// Pull-based capture stream.
///
/// Implementations block in [`next_frame`](CaptureStream::next_frame) until a
/// frame is available, making this trait suitable for both push-based backends
/// (which internally recv from a channel) and poll-based backends.
pub trait CaptureStream {
    type Error: std::error::Error + Send + 'static;

    fn start(&mut self) -> Result<(), Self::Error>;

    /// Block until the next frame is available.
    /// Returns `None` when the stream has ended.
    fn next_frame(&mut self) -> Result<Option<Frame>, Self::Error>;

    fn stop(&mut self) -> Result<(), Self::Error>;

    /// Frame dimensions, if known before the first frame arrives.
    fn output_size(&self) -> Option<[u32; 2]> {
        None
    }
}

/// Platform-level queries: permission checks and target enumeration.
pub trait PlatformCapture {
    type Error: std::error::Error + Send + 'static;

    fn is_supported() -> bool;
    fn has_permission() -> bool;
    fn request_permission() -> bool;
    fn get_all_targets() -> Result<Vec<Target>, Self::Error>;
    fn get_main_display() -> Result<Display, Self::Error>;
}
