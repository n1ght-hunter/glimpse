use glimpse_core::Target;
use glimpse_core::frame::{BGRAFrame, Frame, FrameType, RGBxFrame, VideoFrame};
use glimpse_core::geometry::{Area, Point, Resolution, Size};
use std::cmp;
use std::sync::mpsc;
use std::time::SystemTime;
use windows_capture::{
    capture::{CaptureControl, Context, GraphicsCaptureApiHandler},
    frame::Frame as WCFrame,
    graphics_capture_api::{GraphicsCaptureApi, InternalCaptureControl},
    monitor::Monitor as WCMonitor,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings as WCSettings,
    },
    window::Window as WCWindow,
};

use crate::WinCaptureOptions;
use crate::audio::{AudioStreamControl, AudioStreamHandle, CreateAudioError, spawn_audio_stream};
use crate::targets::{self, DisplayExt, WindowExt};

#[derive(Debug)]
struct FrameHandler {
    pub tx: mpsc::Sender<Frame>,
    pub crop: Option<Area>,
    pub color_format: ColorFormat,
}

#[derive(Clone)]
enum Settings {
    Window(WCSettings<FlagStruct, WCWindow>),
    Display(WCSettings<FlagStruct, WCMonitor>),
}

pub struct WCStream {
    settings: Settings,
    capture_control: Option<CaptureControl<FrameHandler, Box<dyn std::error::Error + Send + Sync>>>,
    audio_stream: Option<AudioStreamHandle>,
}

impl GraphicsCaptureApiHandler for FrameHandler {
    type Flags = FlagStruct;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(context: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            tx: context.flags.tx,
            crop: context.flags.crop,
            color_format: context.flags.color_format,
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut WCFrame,
        _: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let display_time = SystemTime::now();

        let (width, height, data) = match &self.crop {
            Some(cropped_area) => {
                let start_x = cropped_area.origin.x as u32;
                let start_y = cropped_area.origin.y as u32;
                let end_x = (cropped_area.origin.x + cropped_area.size.width) as u32;
                let end_y = (cropped_area.origin.y + cropped_area.size.height) as u32;

                let mut cropped_buffer = frame
                    .buffer_crop(start_x, start_y, end_x, end_y)
                    .expect("Failed to crop buffer");

                let raw_frame_buffer = match cropped_buffer.as_nopadding_buffer() {
                    Ok(buffer) => buffer,
                    Err(_) => return Err(("Failed to get raw buffer").into()),
                };

                (
                    cropped_area.size.width as i32,
                    cropped_area.size.height as i32,
                    raw_frame_buffer.to_vec(),
                )
            }
            None => {
                let width = frame.width() as i32;
                let height = frame.height() as i32;
                let mut frame_buffer = frame.buffer().unwrap();
                let raw_frame_buffer = frame_buffer.as_raw_buffer();
                (width, height, raw_frame_buffer.to_vec())
            }
        };

        let video_frame = match self.color_format {
            ColorFormat::Bgra8 => VideoFrame::BGRA(BGRAFrame {
                display_time,
                width,
                height,
                data,
            }),
            ColorFormat::Rgba8 => VideoFrame::RGBx(RGBxFrame {
                display_time,
                width,
                height,
                data,
            }),
            _ => VideoFrame::BGRA(BGRAFrame {
                display_time,
                width,
                height,
                data,
            }),
        };

        let _ = self.tx.send(Frame::Video(video_frame));
        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        tracing::debug!("Screen capture stream closed");
        Ok(())
    }
}

impl WCStream {
    pub fn start_capture(&mut self) {
        let cc = match &self.settings {
            Settings::Display(st) => FrameHandler::start_free_threaded(st.to_owned()).unwrap(),
            Settings::Window(st) => FrameHandler::start_free_threaded(st.to_owned()).unwrap(),
        };

        if let Some(audio_stream) = &self.audio_stream {
            let _ = audio_stream.ctrl_tx.send(AudioStreamControl::Start);
        }

        self.capture_control = Some(cc)
    }

    pub fn stop_capture(&mut self) {
        let capture_control = self.capture_control.take().unwrap();
        let _ = capture_control.stop();

        if let Some(audio_stream) = &self.audio_stream {
            let _ = audio_stream.ctrl_tx.send(AudioStreamControl::Stop);
        }
    }
}

#[derive(Clone, Debug)]
struct FlagStruct {
    pub tx: mpsc::Sender<Frame>,
    pub crop: Option<Area>,
    pub color_format: ColorFormat,
}

#[derive(Debug)]
pub enum CreateCapturerError {
    Audio(CreateAudioError),
}

pub fn create_capturer(
    options: &WinCaptureOptions,
    tx: mpsc::Sender<Frame>,
) -> Result<WCStream, CreateCapturerError> {
    let target = options
        .base
        .target
        .clone()
        .unwrap_or_else(|| Target::Display(targets::get_main_display()));

    let color_format = match options.base.output_type {
        FrameType::BGRAFrame => ColorFormat::Bgra8,
        _ => ColorFormat::Rgba8,
    };

    let show_cursor = match options.base.show_cursor {
        true => CursorCaptureSettings::WithCursor,
        false => CursorCaptureSettings::WithoutCursor,
    };

    let draw_border = if GraphicsCaptureApi::is_border_settings_supported().unwrap_or(false) {
        if options.base.show_highlight {
            DrawBorderSettings::WithBorder
        } else {
            DrawBorderSettings::WithoutBorder
        }
    } else {
        DrawBorderSettings::Default
    };

    let settings = match &target {
        Target::Display(display) => Settings::Display(WCSettings::new(
            WCMonitor::from_raw_hmonitor(display.hmonitor().0),
            show_cursor,
            draw_border,
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Default,
            DirtyRegionSettings::Default,
            color_format,
            FlagStruct {
                tx: tx.clone(),
                crop: Some(get_crop_area(options)),
                color_format,
            },
        )),
        Target::Window(window) => Settings::Window(WCSettings::new(
            WCWindow::from_raw_hwnd(window.hwnd().0),
            show_cursor,
            draw_border,
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Default,
            DirtyRegionSettings::Default,
            color_format,
            FlagStruct {
                tx: tx.clone(),
                crop: Some(get_crop_area(options)),
                color_format,
            },
        )),
    };

    let audio_stream = if options.base.captures_audio {
        let (ctrl_tx, ctrl_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();

        spawn_audio_stream(tx.clone(), ready_tx, ctrl_rx);

        match ready_rx.recv() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(CreateCapturerError::Audio(e)),
            Err(_) => panic!("Audio spawn panicked"),
        }

        Some(AudioStreamHandle { ctrl_tx })
    } else {
        None
    };

    Ok(WCStream {
        settings,
        capture_control: None,
        audio_stream,
    })
}

pub fn get_output_frame_size(options: &WinCaptureOptions) -> [u32; 2] {
    let crop_area = get_crop_area(options);

    let mut output_width = (crop_area.size.width) as u32;
    let mut output_height = (crop_area.size.height) as u32;

    match options.base.output_resolution {
        Resolution::Captured => {}
        _ => {
            let [resolved_width, resolved_height] = options
                .base
                .output_resolution
                .value((crop_area.size.width as f32) / (crop_area.size.height as f32));
            output_width = cmp::min(output_width, resolved_width);
            output_height = cmp::min(output_height, resolved_height);
        }
    }

    output_width -= output_width % 2;
    output_height -= output_height % 2;

    [output_width, output_height]
}

fn get_absolute_value(value: f64, _scale_factor: f64) -> f64 {
    let value = (value).floor();
    value + value % 2.0
}

pub fn get_crop_area(options: &WinCaptureOptions) -> Area {
    let target = options
        .base
        .target
        .clone()
        .unwrap_or_else(|| Target::Display(targets::get_main_display()));

    let (width, height) = targets::get_target_dimensions(&target);

    let scale_factor = targets::get_scale_factor(&target);
    options
        .base
        .crop_area
        .as_ref()
        .map(|val| Area {
            origin: Point {
                x: get_absolute_value(val.origin.x, scale_factor),
                y: get_absolute_value(val.origin.y, scale_factor),
            },
            size: Size {
                width: get_absolute_value(val.size.width, scale_factor),
                height: get_absolute_value(val.size.height, scale_factor),
            },
        })
        .unwrap_or_else(|| Area {
            origin: Point { x: 0.0, y: 0.0 },
            size: Size {
                width: width as f64,
                height: height as f64,
            },
        })
}
