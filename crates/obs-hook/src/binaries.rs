use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use crate::{error::Result, hook::OwnedHandle};

/// OBS Studio release version to download binaries from.
const OBS_VERSION: &str = "32.1.1";

/// File names we need from the OBS release archive, relative to
/// `data/obs-plugins/win-capture/`.
const BINARY_NAMES: &[&str] = &[
    "inject-helper32.exe",
    "inject-helper64.exe",
    "graphics-hook32.dll",
    "graphics-hook64.dll",
    "get-graphics-offsets32.exe",
    "get-graphics-offsets64.exe",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86,
    X64,
}

pub struct ExtractedBinaries {
    // Exposed for callers that need the cache directory path.
    #[expect(dead_code)]
    pub dir: PathBuf,
    pub inject_helper: PathBuf,
    pub graphics_hook: PathBuf,
    pub get_offsets: PathBuf,
}

fn cache_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("recon_obs").join(OBS_VERSION)
}

fn all_binaries_present(dir: &Path) -> bool {
    BINARY_NAMES.iter().all(|name| dir.join(name).exists())
}

/// Downloads the OBS Studio Windows x64 release ZIP and extracts the hook
/// binaries we need. Caches them in `%LOCALAPPDATA%/recon_obs/{version}/`.
fn download_and_extract(dir: &Path) -> Result<()> {
    let url = format!(
        "https://github.com/obsproject/obs-studio/releases/download/{version}/OBS-Studio-{version}-Windows-x64.zip",
        version = OBS_VERSION
    );

    tracing::info!(%url, "downloading OBS binaries");

    let response = ureq::get(&url).call().map_err(|e| {
        crate::error::Error::InjectionFailed(format!("failed to download OBS release: {e}"))
    })?;

    let mut body = Vec::new();
    response
        .into_body()
        .into_reader()
        .read_to_end(&mut body)
        .map_err(|e| {
            crate::error::Error::InjectionFailed(format!("failed to read OBS release body: {e}"))
        })?;

    let cursor = io::Cursor::new(body);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
        crate::error::Error::InjectionFailed(format!("failed to open OBS release ZIP: {e}"))
    })?;

    fs::create_dir_all(dir)?;

    let prefix = "data/obs-plugins/win-capture/";

    (0..archive.len()).try_for_each(|i| {
        let mut entry = archive.by_index(i).map_err(|e| {
            crate::error::Error::InjectionFailed(format!("failed to read ZIP entry: {e}"))
        })?;

        let name = match entry.name().strip_prefix(prefix) {
            Some(n) if BINARY_NAMES.contains(&n) => n.to_owned(),
            _ => return Ok(()),
        };

        let dest = dir.join(&name);
        let mut file = fs::File::create(&dest)?;
        io::copy(&mut entry, &mut file)?;
        tracing::debug!(%name, "extracted");

        Ok::<_, crate::error::Error>(())
    })?;

    Ok(())
}

/// Ensures OBS hook binaries are available locally, downloading from GitHub
/// releases if needed. Returns paths to the arch-specific binaries.
pub fn ensure_available(arch: Arch) -> Result<ExtractedBinaries> {
    let dir = cache_dir();

    if !all_binaries_present(&dir) {
        download_and_extract(&dir)?;
    }

    if !all_binaries_present(&dir) {
        return Err(crate::error::Error::InjectionFailed(
            "OBS binaries missing after download".into(),
        ));
    }

    let (inject_name, hook_name, offsets_name) = match arch {
        Arch::X86 => (
            "inject-helper32.exe",
            "graphics-hook32.dll",
            "get-graphics-offsets32.exe",
        ),
        Arch::X64 => (
            "inject-helper64.exe",
            "graphics-hook64.dll",
            "get-graphics-offsets64.exe",
        ),
    };

    Ok(ExtractedBinaries {
        dir: dir.clone(),
        inject_helper: dir.join(inject_name),
        graphics_hook: dir.join(hook_name),
        get_offsets: dir.join(offsets_name),
    })
}

/// Detect whether a process is 32-bit (WoW64) or native 64-bit.
pub fn detect_arch(pid: u32) -> Result<Arch> {
    use windows::{
        Win32::System::Threading::{
            IsWow64Process, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        },
        core::BOOL,
    };

    unsafe {
        let handle = OwnedHandle::new(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)?);
        let mut is_wow64 = BOOL(0);
        IsWow64Process(handle.raw(), &mut is_wow64)?;

        if is_wow64.as_bool() {
            Ok(Arch::X86)
        } else {
            Ok(Arch::X64)
        }
    }
}
