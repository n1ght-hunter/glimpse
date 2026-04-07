use std::{os::windows::process::CommandExt, process::Command};

use crate::{
    binaries::ExtractedBinaries,
    error::{Error, Result},
    types::{GraphicOffsets, ParsedGraphicOffsets},
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Runs the `get-graphics-offsets` binary and parses its TOML stdout output
/// into a `GraphicOffsets` struct.
pub fn load(binaries: &ExtractedBinaries) -> Result<GraphicOffsets> {
    let output = Command::new(&binaries.get_offsets)
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let parsed: ParsedGraphicOffsets =
        toml::from_str(&stdout).map_err(|e| Error::OffsetLoadFailed(e.to_string()))?;

    Ok(parsed.into())
}
