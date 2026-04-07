//! Captures a single frame from a game window and saves it as a PNG.
//!
//! Usage:
//!   cargo run -p glimpse_obs_hook --example screenshot -- "Window Title"

use std::env;
use std::fs::File;
use std::io::BufWriter;

use glimpse_obs_hook::{Capture, CaptureConfig, WindowTarget};

fn main() -> glimpse_obs_hook::Result<()> {
    tracing_subscriber::fmt::init();

    let title = env::args()
        .nth(1)
        .expect("usage: screenshot <window title>");

    let config = CaptureConfig::new(WindowTarget::Title(title.clone()));
    let mut capture = Capture::start(config)?;

    println!("capturing frame from \"{title}\"...");
    let frame = capture.capture_frame()?;

    println!(
        "got {}x{} frame (pitch={}, format={})",
        frame.width, frame.height, frame.pitch, frame.format
    );

    // Strip row padding and copy pixel data into a contiguous RGBA buffer.
    let stride = frame.width as usize * 4;
    let mut rgba = vec![0u8; stride * frame.height as usize];

    for (dst_row, src_row) in rgba
        .chunks_exact_mut(stride)
        .zip(frame.data.chunks(frame.pitch as usize))
    {
        dst_row.copy_from_slice(&src_row[..stride]);
    }

    // DXGI_FORMAT_B8G8R8A8_UNORM (87) = BGRA — swizzle B↔R in place.
    if frame.format == 87 {
        for px in rgba.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
    }

    let path = "screenshot.png";
    let file = File::create(path).expect("failed to create output file");
    let w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, frame.width, frame.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().expect("failed to write PNG header");
    writer.write_image_data(&rgba).expect("failed to write PNG data");

    println!("saved to {path}");
    Ok(())
}
