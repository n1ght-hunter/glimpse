/// Opaque platform handle. Interpretation depends on the backend.
///
/// On Windows, wraps an `HWND` or `HMONITOR`. On macOS, wraps a
/// `CGWindowID` or `CGDirectDisplayID`. Platform crates provide typed
/// accessors via extension traits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawHandle(pub isize);

#[derive(Debug, Clone)]
pub struct Window {
    pub id: u32,
    pub title: String,
    pub raw_handle: RawHandle,
}

#[derive(Debug, Clone)]
pub struct Display {
    pub id: u32,
    pub title: String,
    pub raw_handle: RawHandle,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub enum Target {
    Window(Window),
    Display(Display),
}
