use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
        Security::{
            InitializeSecurityDescriptor, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
            SECURITY_DESCRIPTOR, SetSecurityDescriptorDacl,
        },
        Storage::FileSystem::{FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX, ReadFile},
        System::{
            IO::OVERLAPPED,
            Pipes::{
                ConnectNamedPipe, CreateNamedPipeA, PIPE_READMODE_MESSAGE, PIPE_TYPE_MESSAGE,
                PIPE_WAIT,
            },
            Threading::{CreateEventA, WaitForSingleObject},
        },
    },
    core::PCSTR,
};

use crate::{constants, error::Result, hook::OwnedHandle};

const PIPE_BUFFER_SIZE: u32 = 1024;

/// Owns the named pipe handle and its background reader thread.
/// Cleanly shuts down on drop.
pub struct PipeReader {
    _pipe: OwnedHandle,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl PipeReader {
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl Drop for PipeReader {
    fn drop(&mut self) {
        self.stop();
        if let Some(handle) = self.thread.take() {
            // Best-effort join — don't block forever if the thread is stuck
            // waiting on ConnectNamedPipe.
            let _ = handle.join();
        }
    }
}

/// Creates the named pipe the hook sends log messages to, and spawns a
/// background thread that forwards them to `tracing`.
pub fn start_log_pipe(pid: u32) -> Result<PipeReader> {
    let pipe_name = format!("\\\\.\\pipe\\{}{}\0", constants::PIPE_NAME, pid);

    let handle = unsafe {
        let mut sd = SECURITY_DESCRIPTOR::default();
        InitializeSecurityDescriptor(
            PSECURITY_DESCRIPTOR(&raw mut sd as *mut _),
            1, // SECURITY_DESCRIPTOR_REVISION
        )?;
        SetSecurityDescriptorDacl(
            PSECURITY_DESCRIPTOR(&raw mut sd as *mut _),
            true,
            None,
            false,
        )?;

        let sa = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: &raw mut sd as *mut _,
            bInheritHandle: false.into(),
        };

        CreateNamedPipeA(
            PCSTR(pipe_name.as_ptr()),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
            PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
            1,
            PIPE_BUFFER_SIZE,
            PIPE_BUFFER_SIZE,
            0,
            Some(&sa),
        )?
    };

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    // HANDLE contains a raw pointer which isn't Send, but pipe handles are
    // safe to use from any thread.
    let reader_raw = handle.0 as usize;

    let join = thread::spawn(move || {
        let reader_handle = HANDLE(reader_raw as *mut _);
        unsafe {
            let _ = ConnectNamedPipe(reader_handle, None);
        }

        let mut buf = [0u8; PIPE_BUFFER_SIZE as usize];

        while running_clone.load(Ordering::Relaxed) {
            let Ok(event) = (unsafe { CreateEventA(None, true, false, PCSTR::null()) }) else {
                break;
            };

            let mut overlapped = OVERLAPPED {
                hEvent: event,
                ..Default::default()
            };

            let mut bytes_read = 0u32;
            let ok = unsafe {
                ReadFile(
                    reader_handle,
                    Some(&mut buf),
                    Some(&mut bytes_read),
                    Some(&mut overlapped),
                )
            };

            if ok.is_err() {
                let wait = unsafe { WaitForSingleObject(event, 1000) };
                if wait != WAIT_OBJECT_0 {
                    unsafe {
                        let _ = CloseHandle(event);
                    }
                    continue;
                }
            }

            if bytes_read > 0
                && let Ok(msg) = std::str::from_utf8(&buf[..bytes_read as usize])
            {
                tracing::debug!(target: "obs_hook", "{}", msg.trim());
            }

            unsafe {
                let _ = CloseHandle(event);
            }
        }
    });

    Ok(PipeReader {
        _pipe: OwnedHandle::new(handle),
        running,
        thread: Some(join),
    })
}
