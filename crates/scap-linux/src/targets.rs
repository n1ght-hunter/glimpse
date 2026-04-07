use glimpse_core::Target;

/// On Linux, the target is selected when a Recorder is instanciated because this
/// requires user interaction via the D-Bus ScreenCast portal.
pub fn get_all_targets() -> Vec<Target> {
    Vec::new()
}
