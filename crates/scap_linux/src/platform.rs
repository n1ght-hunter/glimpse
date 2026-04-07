use glimpse_core::{Display, PlatformCapture, Target};

use crate::error::LinCapError;
use crate::targets;

/// Linux platform capture support via PipeWire and the D-Bus ScreenCast portal.
pub struct LinuxPlatform;

impl PlatformCapture for LinuxPlatform {
    type Error = LinCapError;

    fn is_supported() -> bool {
        // TODO: detect PipeWire availability
        true
    }

    fn has_permission() -> bool {
        // On Linux, permission is requested interactively via the portal when
        // a capture session is created.
        true
    }

    fn request_permission() -> bool {
        // The portal handles permission prompts at stream creation time.
        true
    }

    fn get_all_targets() -> Result<Vec<Target>, Self::Error> {
        Ok(targets::get_all_targets())
    }

    fn get_main_display() -> Result<Display, Self::Error> {
        // The D-Bus ScreenCast portal does not expose individual display
        // enumeration — the user picks the source interactively.
        Err(LinCapError::new(
            "Linux ScreenCast portal does not support display enumeration".to_string(),
        ))
    }
}
