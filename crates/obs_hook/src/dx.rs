use std::slice;

use windows::{
    Win32::{
        Foundation::{HANDLE, HMODULE},
        Graphics::{
            Direct3D::D3D_DRIVER_TYPE_HARDWARE,
            Direct3D11::{
                D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
                D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING, D3D11CreateDevice, ID3D11Device,
                ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D,
            },
            Dxgi::{DXGI_MAP_READ, DXGI_MAPPED_RECT, IDXGISurface1},
        },
    },
    core::Interface,
};

use crate::error::{Error, Result};

struct StagingState {
    // Prevents the COM reference from being released while the resource is in use.
    #[expect(dead_code)]
    texture: ID3D11Texture2D,
    resource: ID3D11Resource,
    width: u32,
    height: u32,
}

/// Owns a D3D11 device and captures frames from a shared texture handle.
pub struct FrameCapturer {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    resource: ID3D11Resource,
    staging: Option<StagingState>,
    mapped_surface: Option<IDXGISurface1>,
}

/// A captured frame. The pixel data is valid for the lifetime of this struct.
/// Unmaps the DXGI surface on drop.
pub struct Frame<'a> {
    pub data: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    /// Raw DXGI_FORMAT value describing the pixel layout.
    pub format: u32,
    surface: &'a IDXGISurface1,
}

impl Drop for Frame<'_> {
    fn drop(&mut self) {
        unsafe {
            let _ = self.surface.Unmap();
        }
    }
}

impl FrameCapturer {
    /// Create a D3D11 device and open the shared texture resource.
    pub fn new(tex_handle: u32) -> Result<Self> {
        let (device, context) = create_device()?;
        let resource = open_shared_resource(&device, tex_handle)?;

        Ok(Self {
            device,
            context,
            resource,
            staging: None,
            mapped_surface: None,
        })
    }

    /// Capture a single frame from the shared texture.
    pub fn capture_frame(&mut self) -> Result<Frame<'_>> {
        // Unmap previous frame's surface
        if let Some(surface) = self.mapped_surface.take() {
            unsafe {
                let _ = surface.Unmap();
            }
        }

        let frame_texture: ID3D11Texture2D =
            self.resource.cast().map_err(|_| Error::CreateTexture)?;

        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { frame_texture.GetDesc(&mut desc) };

        // Reuse cached staging texture if dimensions match
        let needs_new_staging = self
            .staging
            .as_ref()
            .is_none_or(|s| s.width != desc.Width || s.height != desc.Height);

        if needs_new_staging {
            let mut staging_desc = desc;
            staging_desc.Usage = D3D11_USAGE_STAGING;
            staging_desc.BindFlags = 0u32;
            staging_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
            staging_desc.MiscFlags = 0u32;

            let texture: ID3D11Texture2D = unsafe {
                let mut tex = None;
                self.device
                    .CreateTexture2D(&staging_desc, None, Some(&mut tex))
                    .map_err(|_| Error::CreateTexture)?;
                tex.ok_or(Error::CreateTexture)?
            };

            let resource: ID3D11Resource = texture.cast().map_err(|_| Error::CreateTexture)?;
            unsafe {
                resource.SetEvictionPriority(0xC800_0000); // DXGI_RESOURCE_PRIORITY_MAXIMUM
            }

            self.staging = Some(StagingState {
                texture,
                resource,
                width: desc.Width,
                height: desc.Height,
            });
        }

        let staging = self.staging.as_ref().unwrap();

        unsafe {
            self.context.CopyResource(&staging.resource, &self.resource);
        }

        let surface: IDXGISurface1 = staging.resource.cast().map_err(|_| Error::MapSurface)?;

        let mut mapped = DXGI_MAPPED_RECT::default();
        unsafe {
            surface
                .Map(&mut mapped, DXGI_MAP_READ)
                .map_err(|_| Error::MapSurface)?;
        }

        self.mapped_surface = Some(surface);

        let byte_len = mapped.Pitch as usize * desc.Height as usize;
        let data = unsafe { slice::from_raw_parts(mapped.pBits, byte_len) };

        Ok(Frame {
            data,
            width: desc.Width,
            height: desc.Height,
            pitch: mapped.Pitch as u32,
            format: desc.Format.0 as u32,
            surface: self.mapped_surface.as_ref().unwrap(),
        })
    }
}

impl Drop for FrameCapturer {
    fn drop(&mut self) {
        if let Some(surface) = self.mapped_surface.take() {
            unsafe {
                let _ = surface.Unmap();
            }
        }
    }
}

fn create_device() -> Result<(ID3D11Device, ID3D11DeviceContext)> {
    let mut device = None;
    let mut context = None;

    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_FLAG(0),
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )
        .map_err(|_| Error::CreateDevice)?;
    }

    Ok((
        device.ok_or(Error::CreateDevice)?,
        context.ok_or(Error::CreateDevice)?,
    ))
}

fn open_shared_resource(device: &ID3D11Device, handle: u32) -> Result<ID3D11Resource> {
    let mut resource = None;
    unsafe {
        device
            .OpenSharedResource::<ID3D11Resource>(HANDLE(handle as *mut _), &mut resource)
            .map_err(|_| Error::OpenSharedResource)?;
    }
    resource.ok_or(Error::OpenSharedResource)
}
