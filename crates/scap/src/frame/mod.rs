// Re-export frame types from core
pub use glimpse_core::frame::*;

// Re-export the top-level Frame enum
pub use glimpse_core::Frame;

#[cfg(windows)]
pub fn audio_format_from_cpal(value: cpal::SampleFormat) -> AudioFormat {
    match value {
        cpal::SampleFormat::F32 => AudioFormat::F32,
        cpal::SampleFormat::F64 => AudioFormat::F64,
        cpal::SampleFormat::I8 => AudioFormat::I8,
        cpal::SampleFormat::I16 => AudioFormat::I16,
        cpal::SampleFormat::I32 => AudioFormat::I32,
        cpal::SampleFormat::I64 => AudioFormat::I64,
        cpal::SampleFormat::U8 => AudioFormat::U8,
        cpal::SampleFormat::U16 => AudioFormat::U16,
        cpal::SampleFormat::U32 => AudioFormat::U32,
        cpal::SampleFormat::U64 => AudioFormat::U64,
        _ => panic!("sample format {value:?} not supported"),
    }
}
