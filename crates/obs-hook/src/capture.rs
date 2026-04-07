use crate::{
    binaries::{self, ExtractedBinaries},
    constants,
    dx::{Frame, FrameCapturer},
    error::Result,
    hook::{self, HookEvents, OwnedHandle, WindowTarget},
    ipc::{self, PipeReader},
    offsets,
    shmem::MappedView,
    types::{HookInfo, SharedTextureData},
};

const DEFAULT_HOOK_TIMEOUT_MS: u32 = 5000;

/// Configuration for starting a game capture session.
pub struct CaptureConfig {
    /// How to find the target game window.
    pub target: WindowTarget,
    /// Whether to capture overlay content (e.g. Steam overlay).
    pub capture_overlay: bool,
    /// How long to wait for the hook to become ready, in milliseconds.
    pub hook_timeout_ms: u32,
}

impl CaptureConfig {
    pub fn new(target: WindowTarget) -> Self {
        Self {
            target,
            capture_overlay: false,
            hook_timeout_ms: DEFAULT_HOOK_TIMEOUT_MS,
        }
    }
}

/// Active game capture session. Dropping this signals the hook to stop and
/// cleans up all resources.
pub struct Capture {
    pid: u32,
    binaries: ExtractedBinaries,
    frame_capturer: FrameCapturer,
    events: HookEvents,
    _keepalive: OwnedHandle,
    _pipe: PipeReader,
    capture_overlay: bool,
    hook_timeout_ms: u32,
}

impl Capture {
    /// Launch a capture session: find the window, inject the OBS graphics hook,
    /// set up IPC, and prepare D3D11 for frame readback.
    pub fn start(config: CaptureConfig) -> Result<Self> {
        let target = hook::find_window(&config.target)?;
        tracing::info!(
            pid = target.pid,
            thread_id = target.thread_id,
            "found target window"
        );

        let arch = binaries::detect_arch(target.pid)?;
        let bins = binaries::ensure_available(arch)?;

        let keepalive = hook::create_keepalive(target.pid)?;
        let pipe = ipc::start_log_pipe(target.pid)?;

        if !hook::try_restart_existing(target.pid) {
            tracing::info!("injecting graphics hook");
            hook::inject(&bins, target.thread_id, arch)?;
        } else {
            tracing::info!("reusing existing graphics hook");
        }

        Self::init_hook_info(target.pid, &bins, config.capture_overlay)?;

        let events = hook::open_events(target.pid)?;
        hook::signal_event(&events.init)?;

        tracing::info!(
            timeout_ms = config.hook_timeout_ms,
            "waiting for hook ready"
        );
        hook::wait_for_event(&events.ready, config.hook_timeout_ms)?;

        let tex_handle = Self::read_texture_handle(target.pid)?;
        tracing::info!(tex_handle, "opened shared texture");

        let frame_capturer = FrameCapturer::new(tex_handle)?;

        Ok(Self {
            pid: target.pid,
            binaries: bins,
            frame_capturer,
            events,
            _keepalive: keepalive,
            _pipe: pipe,
            capture_overlay: config.capture_overlay,
            hook_timeout_ms: config.hook_timeout_ms,
        })
    }

    /// Capture a single frame. If the hook signals a restart (e.g. the game
    /// reloaded its graphics), re-initializes automatically.
    pub fn capture_frame(&mut self) -> Result<Frame<'_>> {
        if hook::is_signalled(&self.events.restart) {
            tracing::warn!("restart event signalled, reinitializing");
            self.reinit()?;
        }

        self.frame_capturer.capture_frame()
    }

    /// Signal the hook to stop capturing.
    pub fn stop(&self) -> Result<()> {
        hook::signal_event(&self.events.stop)
    }

    fn init_hook_info(pid: u32, binaries: &ExtractedBinaries, capture_overlay: bool) -> Result<()> {
        let mut hook_info =
            MappedView::<HookInfo>::open(&constants::with_pid(constants::SHMEM_HOOK_INFO, pid))?;

        let offsets = offsets::load(binaries)?;
        hook_info.update(|info| {
            info.graphics_offsets = offsets;
            info.capture_overlay = capture_overlay;
            info.force_shmem = false;
        });

        Ok(())
    }

    fn read_texture_handle(pid: u32) -> Result<u32> {
        let hook_info =
            MappedView::<HookInfo>::open(&constants::with_pid(constants::SHMEM_HOOK_INFO, pid))?;
        let info = hook_info.read();

        let tex_name = constants::texture_shmem_name(info.window, info.map_id);
        let texture_data = MappedView::<SharedTextureData>::open(&tex_name)?;

        Ok(texture_data.read().tex_handle)
    }

    fn reinit(&mut self) -> Result<()> {
        Self::init_hook_info(self.pid, &self.binaries, self.capture_overlay)?;

        self.events = hook::open_events(self.pid)?;
        hook::signal_event(&self.events.init)?;
        hook::wait_for_event(&self.events.ready, self.hook_timeout_ms)?;

        let tex_handle = Self::read_texture_handle(self.pid)?;
        self.frame_capturer = FrameCapturer::new(tex_handle)?;

        Ok(())
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
