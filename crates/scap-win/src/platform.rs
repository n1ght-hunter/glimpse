use glimpse_core::{Display, PlatformCapture, Target};
use windows_capture::graphics_capture_api::GraphicsCaptureApi;

use crate::targets;

pub struct WinPlatform;

impl PlatformCapture for WinPlatform {
    type Error = std::convert::Infallible;

    fn is_supported() -> bool {
        GraphicsCaptureApi::is_supported().expect("Failed to check support")
    }

    fn has_permission() -> bool {
        // Windows doesn't require explicit permission for screen capture
        true
    }

    fn request_permission() -> bool {
        true
    }

    fn get_all_targets() -> Result<Vec<Target>, Self::Error> {
        Ok(targets::get_all_targets())
    }

    fn get_main_display() -> Result<Display, Self::Error> {
        Ok(targets::get_main_display())
    }
}
