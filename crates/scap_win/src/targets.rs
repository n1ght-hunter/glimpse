use glimpse_core::{Display, RawHandle, Target, Window};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI};
use windows::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::HMONITOR,
};
use windows_capture::{monitor::Monitor, window::Window as WCWindow};

pub trait WindowExt {
    fn hwnd(&self) -> HWND;
}

impl WindowExt for Window {
    fn hwnd(&self) -> HWND {
        HWND(self.raw_handle.0 as *mut _)
    }
}

pub trait DisplayExt {
    fn hmonitor(&self) -> HMONITOR;
}

impl DisplayExt for Display {
    fn hmonitor(&self) -> HMONITOR {
        HMONITOR(self.raw_handle.0 as *mut _)
    }
}

fn monitor_title(monitor: &Monitor) -> String {
    monitor
        .name()
        .or_else(|_| monitor.device_string())
        .or_else(|_| monitor.device_name())
        .unwrap_or_default()
}

pub fn get_all_targets() -> Vec<Target> {
    let mut targets: Vec<Target> = Vec::new();

    // Add displays to targets
    let displays = Monitor::enumerate().expect("Failed to enumerate monitors");
    for display in displays {
        let id = display.as_raw_hmonitor() as u32;
        let title = monitor_title(&display);

        let target = Target::Display(Display {
            id,
            title,
            raw_handle: RawHandle(display.as_raw_hmonitor() as isize),
            width: display.width().unwrap_or(0),
            height: display.height().unwrap_or(0),
        });
        targets.push(target);
    }

    // Add windows to targets
    let windows = WCWindow::enumerate().expect("Failed to enumerate windows");
    for window in windows {
        let id = window.as_raw_hwnd() as u32;
        let title = window.title().unwrap().to_string();

        let target = Target::Window(Window {
            id,
            title,
            raw_handle: RawHandle(window.as_raw_hwnd() as isize),
        });
        targets.push(target);
    }

    targets
}

pub fn get_main_display() -> Display {
    let display = Monitor::primary().expect("Failed to get primary monitor");
    let id = display.as_raw_hmonitor() as u32;

    Display {
        id,
        title: monitor_title(&display),
        raw_handle: RawHandle(display.as_raw_hmonitor() as isize),
        width: display.width().unwrap_or(0),
        height: display.height().unwrap_or(0),
    }
}

// Referred to: https://github.com/tauri-apps/tao/blob/ab792dbd6c5f0a708c818b20eaff1d9a7534c7c1/src/platform_impl/windows/dpi.rs#L50
pub fn get_scale_factor(target: &Target) -> f64 {
    const BASE_DPI: u32 = 96;

    let mut dpi_x = 0;
    let mut dpi_y = 0;

    let dpi = match target {
        Target::Window(window) => unsafe { GetDpiForWindow(window.hwnd()) },
        Target::Display(display) => unsafe {
            if GetDpiForMonitor(
                display.hmonitor(),
                MDT_EFFECTIVE_DPI,
                &mut dpi_x,
                &mut dpi_y,
            )
            .is_ok()
            {
                dpi_x
            } else {
                BASE_DPI
            }
        },
    };

    dpi as f64 / BASE_DPI as f64
}

pub fn get_target_dimensions(target: &Target) -> (u64, u64) {
    match target {
        Target::Window(window) => unsafe {
            let hwnd = window.hwnd();
            let mut rect = RECT::default();
            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            (width as u64, height as u64)
        },
        Target::Display(display) => {
            let monitor = Monitor::from_raw_hmonitor(display.raw_handle.0 as *mut _);
            (
                monitor.width().unwrap() as u64,
                monitor.height().unwrap() as u64,
            )
        }
    }
}
