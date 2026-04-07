use std::mem;

use serde::Deserialize;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub enum CaptureType {
    Memory = 0,
    Texture = 1,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SharedTextureData {
    pub tex_handle: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct D3D8Offsets {
    pub present: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct D3D9Offsets {
    pub present: u32,
    pub present_ex: u32,
    pub present_swap: u32,
    pub d3d9_clsoff: u32,
    pub is_d3d9ex_clsoff: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct DxgiOffsets {
    pub present: u32,
    pub present1: u32,
    pub resize: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct DDrawOffsets {
    pub surface_create: u32,
    pub surface_restore: u32,
    pub surface_release: u32,
    pub surface_unlock: u32,
    pub surface_blt: u32,
    pub surface_flip: u32,
    pub surface_set_palette: u32,
    pub palette_set_entries: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct GraphicOffsets {
    pub d3d8: D3D8Offsets,
    pub d3d9: D3D9Offsets,
    pub dxgi: DxgiOffsets,
    pub ddraw: DDrawOffsets,
}

/// Intermediate struct for deserializing the TOML output from
/// `get-graphics-offsets`. The exe only outputs d3d8/d3d9/dxgi sections.
#[derive(Deserialize)]
pub(crate) struct ParsedGraphicOffsets {
    pub d3d8: D3D8Offsets,
    pub d3d9: D3D9Offsets,
    pub dxgi: DxgiOffsets,
}

impl From<ParsedGraphicOffsets> for GraphicOffsets {
    fn from(parsed: ParsedGraphicOffsets) -> Self {
        Self {
            d3d8: parsed.d3d8,
            d3d9: parsed.d3d9,
            dxgi: parsed.dxgi,
            ddraw: DDrawOffsets::default(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HookInfo {
    pub hook_ver_major: u32,
    pub hook_ver_minor: u32,

    pub capture_type: CaptureType,
    pub window: u32,
    pub format: u32,
    pub cx: u32,
    pub cy: u32,
    _unused_base_cx: u32,
    _unused_base_cy: u32,
    pub pitch: u32,
    pub map_id: u32,
    pub map_size: u32,
    pub flip: bool,

    pub frame_interval: u64,
    _unused_use_scale: bool,
    pub force_shmem: bool,
    pub capture_overlay: bool,

    pub graphics_offsets: GraphicOffsets,

    _reserved: [u32; 128],
}

/// BGRA8 pixel format — matches the byte order of D3D11 capture output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Bgra8 {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

const _: () = assert!(mem::size_of::<SharedTextureData>() == 4);
const _: () = assert!(mem::size_of::<CaptureType>() == 4);
const _: () = assert!(mem::size_of::<D3D8Offsets>() == 4);
const _: () = assert!(mem::size_of::<D3D9Offsets>() == 20);
const _: () = assert!(mem::size_of::<DxgiOffsets>() == 12);
const _: () = assert!(mem::size_of::<DDrawOffsets>() == 32);
const _: () = assert!(mem::size_of::<GraphicOffsets>() == 68);
const _: () = assert!(mem::size_of::<HookInfo>() == 648);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_sizes() {
        assert_eq!(mem::size_of::<SharedTextureData>(), 4);
        assert_eq!(mem::size_of::<CaptureType>(), 4);
        assert_eq!(mem::size_of::<GraphicOffsets>(), 68);
        assert_eq!(mem::size_of::<HookInfo>(), 648);
    }
}
