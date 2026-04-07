use std::{ffi::CString, mem::MaybeUninit, os::windows::process::CommandExt, process::Command};

use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, HWND, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::Threading::{
            CreateMutexA, EVENT_MODIFY_STATE, OpenEventA, SYNCHRONIZATION_ACCESS_RIGHTS,
            SYNCHRONIZATION_SYNCHRONIZE, SetEvent, WaitForSingleObject,
        },
        UI::WindowsAndMessaging::{FindWindowA, GetWindowThreadProcessId},
    },
    core::PCSTR,
};

use crate::{
    binaries::{Arch, ExtractedBinaries},
    constants::*,
    error::{Error, Result},
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Owned Win32 handle that calls `CloseHandle` on drop.
pub struct OwnedHandle(HANDLE);

impl OwnedHandle {
    pub fn new(handle: HANDLE) -> Self {
        Self(handle)
    }

    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// How to locate the target game window.
pub enum WindowTarget {
    /// Find by exact window title.
    Title(String),
    /// Find by window class name.
    ClassName(String),
    /// Use an existing window handle directly.
    Hwnd(isize),
}

/// Returned information about a target window.
pub struct TargetWindow {
    // Exposed for callers that need the raw window handle.
    #[expect(dead_code)]
    pub hwnd: HWND,
    pub pid: u32,
    pub thread_id: u32,
}

/// Find a window using the given target specifier.
pub fn find_window(target: &WindowTarget) -> Result<TargetWindow> {
    match target {
        WindowTarget::Title(title) => find_by_title(title),
        WindowTarget::ClassName(class) => find_by_class(class),
        WindowTarget::Hwnd(hwnd) => {
            let hwnd = HWND(*hwnd as *mut _);
            let mut pid = MaybeUninit::uninit();
            let thread_id = unsafe { GetWindowThreadProcessId(hwnd, Some(pid.as_mut_ptr())) };
            Ok(TargetWindow {
                hwnd,
                pid: unsafe { pid.assume_init() },
                thread_id,
            })
        }
    }
}

fn find_by_title(title: &str) -> Result<TargetWindow> {
    let c_title = CString::new(title).map_err(|_| Error::WindowNotFound(title.to_owned()))?;

    unsafe {
        let hwnd = FindWindowA(PCSTR::null(), PCSTR(c_title.as_ptr().cast()))
            .map_err(|_| Error::WindowNotFound(title.to_owned()))?;

        let mut pid = MaybeUninit::uninit();
        let thread_id = GetWindowThreadProcessId(hwnd, Some(pid.as_mut_ptr()));

        Ok(TargetWindow {
            hwnd,
            pid: pid.assume_init(),
            thread_id,
        })
    }
}

fn find_by_class(class: &str) -> Result<TargetWindow> {
    let c_class = CString::new(class).map_err(|_| Error::WindowNotFound(class.to_owned()))?;

    unsafe {
        let hwnd = FindWindowA(PCSTR(c_class.as_ptr().cast()), PCSTR::null())
            .map_err(|_| Error::WindowNotFound(class.to_owned()))?;

        let mut pid = MaybeUninit::uninit();
        let thread_id = GetWindowThreadProcessId(hwnd, Some(pid.as_mut_ptr()));

        Ok(TargetWindow {
            hwnd,
            pid: pid.assume_init(),
            thread_id,
        })
    }
}

/// Create the keepalive mutex so the hook knows a consumer is alive.
pub fn create_keepalive(pid: u32) -> Result<OwnedHandle> {
    let name = with_pid(WINDOW_HOOK_KEEPALIVE, pid);
    let c_name = CString::new(name).unwrap();

    unsafe {
        let handle = CreateMutexA(None, false, PCSTR(c_name.as_ptr().cast()))?;
        Ok(OwnedHandle::new(handle))
    }
}

const EVENT_ACCESS: SYNCHRONIZATION_ACCESS_RIGHTS =
    SYNCHRONIZATION_ACCESS_RIGHTS(EVENT_MODIFY_STATE.0 | SYNCHRONIZATION_SYNCHRONIZE.0);

/// Try to reuse an existing hook by signalling the capture restart event.
/// Returns `true` if an existing hook was found and signalled.
pub fn try_restart_existing(pid: u32) -> bool {
    let name = with_pid(EVENT_CAPTURE_RESTART, pid);
    let c_name = match CString::new(name) {
        Ok(s) => s,
        Err(_) => return false,
    };

    unsafe {
        let Ok(handle) = OpenEventA(EVENT_ACCESS, false, PCSTR(c_name.as_ptr().cast())) else {
            return false;
        };
        let owned = OwnedHandle::new(handle);
        SetEvent(owned.raw()).is_ok()
    }
}

/// Inject the graphics hook DLL into the target process.
/// When using safe injection (anti-cheat compatible), the inject-helper
/// expects a thread ID rather than a process ID.
pub fn inject(binaries: &ExtractedBinaries, thread_id: u32, _arch: Arch) -> Result<()> {
    let status = Command::new(&binaries.inject_helper)
        .args([
            binaries.graphics_hook.to_str().unwrap(),
            "1", // safe inject — uses thread ID
            &thread_id.to_string(),
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        let code = status.code().unwrap_or(-999);
        Err(Error::InjectionFailed(format!(
            "inject-helper exited with code {code}"
        )))
    }
}

/// All named events used to coordinate with the graphics hook.
pub struct HookEvents {
    pub restart: OwnedHandle,
    pub stop: OwnedHandle,
    pub init: OwnedHandle,
    pub ready: OwnedHandle,
    // Kept alive so the handle isn't closed while the hook is running.
    #[expect(dead_code)]
    pub exit: OwnedHandle,
}

fn open_event(base: &str, pid: u32) -> Result<OwnedHandle> {
    let name = with_pid(base, pid);
    let c_name = CString::new(name).unwrap();

    unsafe {
        let handle = OpenEventA(EVENT_ACCESS, false, PCSTR(c_name.as_ptr().cast()))?;
        Ok(OwnedHandle::new(handle))
    }
}

/// Open all hook coordination events. Call after the hook has been injected and
/// shared memory is set up.
pub fn open_events(pid: u32) -> Result<HookEvents> {
    Ok(HookEvents {
        restart: open_event(EVENT_CAPTURE_RESTART, pid)?,
        stop: open_event(EVENT_CAPTURE_STOP, pid)?,
        init: open_event(EVENT_HOOK_INIT, pid)?,
        ready: open_event(EVENT_HOOK_READY, pid)?,
        exit: open_event(EVENT_HOOK_EXIT, pid)?,
    })
}

/// Signal a Win32 event.
pub fn signal_event(handle: &OwnedHandle) -> Result<()> {
    unsafe { SetEvent(handle.raw())? };
    Ok(())
}

/// Non-blocking check whether an event is in the signalled state.
pub fn is_signalled(handle: &OwnedHandle) -> bool {
    unsafe { WaitForSingleObject(handle.raw(), 0) == WAIT_OBJECT_0 }
}

/// Block until the event is signalled or the timeout elapses.
pub fn wait_for_event(handle: &OwnedHandle, timeout_ms: u32) -> Result<()> {
    let result = unsafe { WaitForSingleObject(handle.raw(), timeout_ms) };
    if result == WAIT_OBJECT_0 {
        Ok(())
    } else if result == WAIT_TIMEOUT {
        Err(Error::HookTimeout { timeout_ms })
    } else {
        Err(Error::Windows(windows::core::Error::from_thread()))
    }
}
