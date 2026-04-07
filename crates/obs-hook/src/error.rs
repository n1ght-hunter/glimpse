use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("window not found: {0}")]
    WindowNotFound(String),

    #[error("hook injection failed: {0}")]
    InjectionFailed(String),

    #[error("offset loader failed: {0}")]
    OffsetLoadFailed(String),

    #[error("shared memory open failed: {name}")]
    SharedMemoryOpen { name: String },

    #[error("D3D11 device creation failed")]
    CreateDevice,

    #[error("failed to open shared D3D11 resource")]
    OpenSharedResource,

    #[error("failed to create staging texture")]
    CreateTexture,

    #[error("failed to map DXGI surface")]
    MapSurface,

    #[error("hook did not become ready within {timeout_ms}ms")]
    HookTimeout { timeout_ms: u32 },

    #[error(transparent)]
    Windows(#[from] windows::core::Error),

    #[error(transparent)]
    Io(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
