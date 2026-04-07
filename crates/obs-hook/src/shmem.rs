use std::{ffi::CString, ptr};

use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::Memory::{
            FILE_MAP_ALL_ACCESS, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile, OpenFileMappingA,
            UnmapViewOfFile,
        },
    },
    core::PCSTR,
};

use crate::error::{Error, Result};

/// RAII wrapper around a Windows file mapping. Opens an existing named shared
/// memory region and provides volatile access to the mapped data.
///
/// Uses `read_volatile`/`write_volatile` because the backing memory is shared
/// with another process (the OBS graphics hook). Standard references (`&T`,
/// `&mut T`) would be UB since the other process may write at any time.
pub struct MappedView<T> {
    ptr: *mut T,
    handle: HANDLE,
    view: MEMORY_MAPPED_VIEW_ADDRESS,
}

unsafe impl<T: Send> Send for MappedView<T> {}

impl<T: Copy> MappedView<T> {
    pub fn open(name: &str) -> Result<Self> {
        let c_name = CString::new(name).map_err(|_| Error::SharedMemoryOpen {
            name: name.to_owned(),
        })?;

        unsafe {
            let handle =
                OpenFileMappingA(FILE_MAP_ALL_ACCESS.0, false, PCSTR(c_name.as_ptr().cast()))?;

            let view = MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, 0);
            if view.Value.is_null() {
                CloseHandle(handle)?;
                return Err(Error::SharedMemoryOpen {
                    name: name.to_owned(),
                });
            }

            Ok(Self {
                ptr: view.Value.cast(),
                handle,
                view,
            })
        }
    }

    /// Read the entire mapped value via `read_volatile`.
    pub fn read(&self) -> T {
        unsafe { ptr::read_volatile(self.ptr) }
    }

    /// Overwrite the entire mapped value via `write_volatile`.
    pub fn write(&mut self, val: T) {
        unsafe { ptr::write_volatile(self.ptr, val) }
    }

    /// Read-modify-write: volatile read, apply `f`, volatile write back.
    pub fn update(&mut self, f: impl FnOnce(&mut T)) {
        let mut val = self.read();
        f(&mut val);
        self.write(val);
    }
}

impl<T> Drop for MappedView<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = UnmapViewOfFile(self.view);
            let _ = CloseHandle(self.handle);
        }
    }
}
