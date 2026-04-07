use core_graphics_helmer_fork::access::ScreenCaptureAccess;
use sysinfo::System;

use glimpse_core::capture::PlatformCapture;
use glimpse_core::{Display, Target};

/// macOS platform capture implementation.
pub struct MacPlatform;

#[derive(thiserror::Error, Debug)]
pub enum MacPlatformError {
    #[error("failed to query targets")]
    QueryFailed,
}

impl PlatformCapture for MacPlatform {
    type Error = MacPlatformError;

    fn is_supported() -> bool {
        let os_version = System::os_version()
            .expect("Failed to get macOS version")
            .as_bytes()
            .to_vec();

        let min_version: Vec<u8> = "12.3\n".as_bytes().to_vec();

        os_version >= min_version
    }

    fn has_permission() -> bool {
        ScreenCaptureAccess.preflight()
    }

    fn request_permission() -> bool {
        ScreenCaptureAccess.request()
    }

    fn get_all_targets() -> Result<Vec<Target>, Self::Error> {
        Ok(crate::targets::get_all_targets())
    }

    fn get_main_display() -> Result<Display, Self::Error> {
        Ok(crate::targets::get_main_display())
    }
}
