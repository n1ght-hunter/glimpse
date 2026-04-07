pub const EVENT_CAPTURE_RESTART: &str = "CaptureHook_Restart";
pub const EVENT_CAPTURE_STOP: &str = "CaptureHook_Stop";
pub const EVENT_HOOK_READY: &str = "CaptureHook_HookReady";
pub const EVENT_HOOK_EXIT: &str = "CaptureHook_Exit";
pub const EVENT_HOOK_INIT: &str = "CaptureHook_Initialize";

pub const WINDOW_HOOK_KEEPALIVE: &str = "CaptureHook_KeepAlive";

pub const SHMEM_HOOK_INFO: &str = "CaptureHook_HookInfo";
pub const SHMEM_TEXTURE: &str = "CaptureHook_Texture";

pub const PIPE_NAME: &str = "CaptureHook_Pipe";

pub fn with_pid(base: &str, pid: u32) -> String {
    format!("{base}{pid}")
}

pub fn texture_shmem_name(window: u32, map_id: u32) -> String {
    format!("{SHMEM_TEXTURE}_{window}_{map_id}")
}
