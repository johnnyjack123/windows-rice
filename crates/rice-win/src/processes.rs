//! Running processes: CPU share, working set, exe path and icon, terminating;
//! app icons by AUMID.

use crate::from_wide;
use rice_core::sysmon::process_cpu;
use std::collections::HashMap;
use std::mem;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER, FILETIME, HANDLE};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDIBits,
    GetObjectW, HBITMAP, HDC,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::{
    GetActiveProcessorCount, GetProcessTimes, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE, QueryFullProcessImageNameW, TerminateProcess,
};
use crate::com::ComGuard;
use windows::Win32::Foundation::SIZE;
use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SHDefExtractIconW, SIIGBF_ICONONLY};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};
use windows::core::{HSTRING, PWSTR};

/// All processor groups for `GetActiveProcessorCount`.
const ALL_PROCESSOR_GROUPS: u16 = 0xffff;
/// A baseline older than this is too stale for meaningful percentages.
const BASELINE_MAX_AGE: Duration = Duration::from_secs(10);

/// One process in a sample.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessSample {
    pub pid: u32,
    pub name: String,
    /// Percent of total CPU since the previous sample.
    pub cpu: f64,
    /// Working set in bytes.
    pub memory: u64,
}

/// Remembers CPU times between samples.
#[derive(Default)]
pub struct ProcessSampler {
    /// pid → (creation time, kernel + user time); the creation time detects reused pids.
    previous: HashMap<u32, (u64, u64)>,
    previous_at: Option<Instant>,
}

impl ProcessSampler {
    /// Whether the last sample is recent enough to compute percentages from.
    pub fn has_baseline(&self) -> bool {
        self.previous_at.is_some_and(|at| at.elapsed() < BASELINE_MAX_AGE)
    }

    /// Every process that can be queried, with its CPU share since the last call.
    pub fn sample(&mut self) -> Vec<ProcessSample> {
        let now = Instant::now();
        let elapsed = self.previous_at.map(|at| now.duration_since(at));
        // SAFETY: no arguments.
        let processors = unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) };
        let mut next = HashMap::new();
        let mut out = Vec::new();
        for (pid, name) in list() {
            let Some((created, cpu_ticks, memory)) = query(pid) else { continue };
            next.insert(pid, (created, cpu_ticks));
            let cpu = match (self.previous.get(&pid), elapsed) {
                (Some(&(was_created, previous_ticks)), Some(elapsed)) if was_created == created => {
                    let elapsed_ticks = u64::try_from(elapsed.as_nanos() / 100).unwrap_or(u64::MAX);
                    process_cpu(previous_ticks, cpu_ticks, elapsed_ticks, processors)
                }
                _ => 0.0,
            };
            out.push(ProcessSample { pid, name, cpu, memory });
        }
        self.previous = next;
        self.previous_at = Some(now);
        out
    }
}

/// (pid, exe name) of every process.
fn list() -> Vec<(u32, String)> {
    let mut out = Vec::new();
    // SAFETY: snapshot handle closed below; entries are plain structs.
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return out;
        };
        let mut entry = PROCESSENTRY32W { dwSize: mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut ok = Process32FirstW(snapshot, &mut entry).is_ok();
        while ok {
            // pid 0 is the idle process, not a real one.
            if entry.th32ProcessID != 0 {
                out.push((entry.th32ProcessID, from_wide(&entry.szExeFile)));
            }
            ok = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
    }
    out
}

/// Closes a process handle when dropped.
struct Process(HANDLE);

impl Process {
    fn open(pid: u32, access: windows::Win32::System::Threading::PROCESS_ACCESS_RIGHTS) -> windows::core::Result<Self> {
        // SAFETY: plain handle request; closed in Drop.
        unsafe { OpenProcess(access, false, pid) }.map(Self)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        // SAFETY: handle opened by `Process::open`.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn ticks(time: FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}

/// (creation time, CPU time, working set) of one process.
fn query(pid: u32) -> Option<(u64, u64, u64)> {
    let process = Process::open(pid, PROCESS_QUERY_LIMITED_INFORMATION).ok()?;
    let (mut created, mut exited, mut kernel, mut user) =
        (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
    let mut counters = PROCESS_MEMORY_COUNTERS { cb: mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32, ..Default::default() };
    // SAFETY: live handle and out-pointers to locals.
    unsafe {
        GetProcessTimes(process.0, &mut created, &mut exited, &mut kernel, &mut user).ok()?;
        GetProcessMemoryInfo(process.0, &mut counters, counters.cb).ok()?;
    }
    Some((ticks(created), ticks(kernel) + ticks(user), counters.WorkingSetSize as u64))
}

/// Full path of a process' executable.
pub fn exe_path(pid: u32) -> Option<PathBuf> {
    let process = Process::open(pid, PROCESS_QUERY_LIMITED_INFORMATION).ok()?;
    let mut buffer = vec![0u16; 1024];
    let mut size = buffer.len() as u32;
    // SAFETY: buffer of `size` characters.
    unsafe { QueryFullProcessImageNameW(process.0, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut size) }.ok()?;
    Some(PathBuf::from(String::from_utf16_lossy(&buffer[..size as usize])))
}

/// Why a process could not be ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KillError {
    AccessDenied,
    NotFound,
    Failed(String),
}

/// Ends a process immediately (`TerminateProcess`).
pub fn kill(pid: u32) -> Result<(), KillError> {
    let process = Process::open(pid, PROCESS_TERMINATE).map_err(|e| match e.code() {
        code if code == ERROR_ACCESS_DENIED.to_hresult() => KillError::AccessDenied,
        code if code == ERROR_INVALID_PARAMETER.to_hresult() => KillError::NotFound,
        _ => KillError::Failed(e.to_string()),
    })?;
    // SAFETY: handle with terminate rights.
    unsafe { TerminateProcess(process.0, 1) }.map_err(|e| {
        if e.code() == ERROR_ACCESS_DENIED.to_hresult() { KillError::AccessDenied } else { KillError::Failed(e.to_string()) }
    })
}

/// The first icon of an executable as RGBA pixels (width, height, pixels).
pub fn icon_rgba(path: &std::path::Path, size: u32) -> Option<(u32, u32, Vec<u8>)> {
    let mut icon = HICON::default();
    // SAFETY: NUL-terminated path; the icon is destroyed below.
    unsafe {
        SHDefExtractIconW(&HSTRING::from(path.as_os_str()), 0, 0, Some(&mut icon), None, size).ok().ok()?;
        if icon.is_invalid() {
            return None;
        }
        let pixels = icon_pixels(icon);
        let _ = DestroyIcon(icon);
        pixels
    }
}

/// The Start menu icon of an app by AUMID (`shell:AppsFolder\<AUMID>`) as
/// RGBA pixels. Works for packaged and classic apps alike.
pub fn app_icon_rgba(aumid: &str, size: u32) -> Option<(u32, u32, Vec<u8>)> {
    if aumid.is_empty() {
        return None;
    }
    let _com = ComGuard::sta();
    let side = i32::try_from(size).ok()?;
    // SAFETY: NUL-terminated parsing name; the bitmap is deleted below.
    unsafe {
        let item: IShellItemImageFactory = SHCreateItemFromParsingName(&HSTRING::from(format!(r"shell:AppsFolder\{aumid}")), None).ok()?;
        let bitmap = item.GetImage(SIZE { cx: side, cy: side }, SIIGBF_ICONONLY).ok()?;
        let dc = CreateCompatibleDC(None);
        let result = (|| {
            let (width, height) = bitmap_size(bitmap)?;
            let mut pixels = dib(dc, bitmap, width, height)?;
            // BGRA → RGBA
            for pixel in pixels.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
            Some((width, height, pixels))
        })();
        let _ = DeleteDC(dc);
        let _ = DeleteObject(bitmap.into());
        result
    }
}

/// # Safety
/// `icon` must be a valid icon handle.
unsafe fn icon_pixels(icon: HICON) -> Option<(u32, u32, Vec<u8>)> {
    let mut info = ICONINFO::default();
    // SAFETY: guaranteed by the caller; both bitmaps are deleted below.
    unsafe {
        GetIconInfo(icon, &mut info).ok()?;
        let dc = CreateCompatibleDC(None);
        let result = (|| {
            let (width, height) = bitmap_size(info.hbmColor)?;
            let mut pixels = dib(dc, info.hbmColor, width, height)?;
            let (rgba, _) = pixels.as_chunks_mut::<4>();
            // Old icons have no alpha channel; their mask says what is transparent.
            if rgba.iter().all(|p| p[3] == 0) {
                let mask = dib(dc, info.hbmMask, width, height)?;
                for (pixel, m) in rgba.iter_mut().zip(mask.as_chunks::<4>().0) {
                    pixel[3] = if m[0] == 0 { 255 } else { 0 };
                }
            }
            // BGRA → RGBA
            for pixel in rgba.iter_mut() {
                pixel.swap(0, 2);
            }
            Some((width, height, pixels))
        })();
        let _ = DeleteDC(dc);
        if !info.hbmColor.is_invalid() {
            let _ = DeleteObject(info.hbmColor.into());
        }
        if !info.hbmMask.is_invalid() {
            let _ = DeleteObject(info.hbmMask.into());
        }
        result
    }
}

/// # Safety
/// `bitmap` must be null or a valid bitmap handle.
unsafe fn bitmap_size(bitmap: HBITMAP) -> Option<(u32, u32)> {
    if bitmap.is_invalid() {
        return None;
    }
    let mut info = BITMAP::default();
    // SAFETY: guaranteed by the caller; correctly sized out struct.
    let written = unsafe { GetObjectW(bitmap.into(), mem::size_of::<BITMAP>() as i32, Some((&mut info as *mut BITMAP).cast())) };
    (written != 0 && info.bmWidth > 0 && info.bmHeight > 0).then_some((info.bmWidth as u32, info.bmHeight as u32))
}

/// Top-down 32-bit BGRA pixels of a bitmap.
///
/// # Safety
/// `dc` and `bitmap` must be valid.
unsafe fn dib(dc: HDC, bitmap: HBITMAP, width: u32, height: u32) -> Option<Vec<u8>> {
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    // SAFETY: guaranteed by the caller; the buffer holds `height` rows of 32-bit pixels.
    let lines = unsafe { GetDIBits(dc, bitmap, 0, height, Some(pixels.as_mut_ptr().cast()), &mut info, DIB_RGB_COLORS) };
    (lines == height as i32).then_some(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_its_own_process() {
        let mut sampler = ProcessSampler::default();
        assert!(!sampler.has_baseline());
        sampler.sample();
        assert!(sampler.has_baseline());
        let own = std::process::id();
        let list = sampler.sample();
        let me = list.iter().find(|p| p.pid == own).expect("own process listed");
        assert!(me.memory > 0);
        assert!((0.0..=100.0).contains(&me.cpu));
        assert!(exe_path(own).is_some_and(|p| p.is_file()));
        // Process ids are multiples of 4; this one never exists.
        assert_eq!(kill(0xFFFF_FFF1), Err(KillError::NotFound));
    }

    #[test]
    fn extracts_an_exe_icon() {
        let exe = exe_path(std::process::id()).expect("own exe");
        // Test binaries have no icon resource; Explorer always has one.
        let explorer = PathBuf::from(std::env::var_os("WINDIR").expect("WINDIR")).join("explorer.exe");
        assert!(icon_rgba(&exe, 48).is_none_or(|(w, h, px)| px.len() == (w * h * 4) as usize));
        let (width, height, pixels) = icon_rgba(&explorer, 48).expect("explorer icon");
        assert_eq!(pixels.len(), (width * height * 4) as usize);
        assert!(pixels.as_chunks::<4>().0.iter().any(|p| p[3] > 0), "icon is not fully transparent");
    }

    #[test]
    fn finds_app_icons_by_aumid() {
        // File Explorer's AUMID exists on every Windows 11 installation.
        let (width, height, pixels) = app_icon_rgba("Microsoft.Windows.Explorer", 48).expect("explorer app icon");
        assert_eq!(pixels.len(), (width * height * 4) as usize);
        assert!(pixels.as_chunks::<4>().0.iter().any(|p| p[3] > 0), "icon is not fully transparent");
        assert!(app_icon_rgba("rice.test.NoSuchApp", 48).is_none());
        assert!(app_icon_rgba("", 48).is_none());
    }
}
