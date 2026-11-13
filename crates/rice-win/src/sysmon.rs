//! System load: CPU, memory, GPU and network throughput.
//!
//! A [`Sampler`] keeps the previous counter readings and the PDH query; it is
//! not thread-safe and lives on the sampling thread.

use rice_core::sysmon::{CpuTimes, adapter_total, cpu_usage, current_ghz, luid_key, rate, usage_by_adapter};
use std::mem;
use std::time::Instant;
use windows::Win32::Foundation::{ERROR_SUCCESS, FILETIME};
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, IDXGIFactory1};
use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::System::Performance::{
    PDH_CSTATUS_VALID_DATA, PDH_FMT, PDH_FMT_COUNTERVALUE, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData,
    PdhGetFormattedCounterArrayW, PdhGetFormattedCounterValue, PdhOpenQueryW,
};
use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;
use windows::core::{HSTRING, PCWSTR, w};

/// `PDH_FMT_NOCAP100` from pdh.h (missing in the bindings): values above 100
/// are kept, e.g. `% Processor Performance` while boosting.
const PDH_FMT_NOCAP100: u32 = 0x0000_8000;
/// Software loopback interface type (`IF_TYPE_SOFTWARE_LOOPBACK`).
const LOOPBACK: u32 = 24;
/// Microsoft Basic Render Driver (software adapter that DXGI lists as hardware on some systems).
const BASIC_RENDER_VENDOR: u32 = 0x1414;
/// Below this much dedicated memory a GPU shares system RAM (integrated graphics).
const DEDICATED_THRESHOLD: u64 = 512 * 1024 * 1024;

/// One reading of everything the system section shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub cpu_usage: f64,
    pub cpu_name: String,
    pub ghz: Option<f64>,
    pub memory_used: u64,
    pub memory_total: u64,
    pub gpu: Option<Gpu>,
    pub down_per_sec: f64,
    pub up_per_sec: f64,
}

/// GPU part of a sample.
#[derive(Debug, Clone, PartialEq)]
pub struct Gpu {
    pub usage: f64,
    pub name: String,
    pub vram_used: u64,
    pub vram_total: u64,
}

struct Adapter {
    name: String,
    key: String,
    /// Integrated GPU: its memory is shared system RAM.
    shared: bool,
    total: u64,
}

/// Keeps the previous readings between samples.
pub struct Sampler {
    cpu_name: String,
    cpu_previous: Option<CpuTimes>,
    network_previous: Option<(Instant, u64, u64)>,
    adapter: Option<Adapter>,
    pdh: Option<Pdh>,
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

impl Sampler {
    /// Opens the counters. The first [`Sampler::sample`] has no deltas yet
    /// (CPU and network read as 0); take one, wait, then sample for real.
    pub fn new() -> Self {
        Self {
            cpu_name: cpu_name().unwrap_or_default(),
            cpu_previous: None,
            network_previous: None,
            adapter: main_adapter(),
            pdh: Pdh::open().inspect_err(|e| log::warn!("sysmon: performance counters unavailable ({e})")).ok(),
        }
    }

    /// Reads all values.
    pub fn sample(&mut self) -> Sample {
        if let Some(pdh) = &self.pdh {
            pdh.collect();
        }
        let times = cpu_times();
        let usage = match (self.cpu_previous, times) {
            (Some(previous), Some(next)) => cpu_usage(previous, next),
            _ => 0.0,
        };
        self.cpu_previous = times.or(self.cpu_previous);

        let (memory_used, memory_total) = memory();

        let now = Instant::now();
        let (down_per_sec, up_per_sec) = match (network_octets(), self.network_previous) {
            (Some((down, up)), Some((at, previous_down, previous_up))) => {
                self.network_previous = Some((now, down, up));
                let elapsed = now.duration_since(at).as_secs_f64();
                (rate(previous_down, down, elapsed), rate(previous_up, up, elapsed))
            }
            (Some((down, up)), None) => {
                self.network_previous = Some((now, down, up));
                (0.0, 0.0)
            }
            (None, _) => (0.0, 0.0),
        };

        Sample {
            cpu_usage: usage,
            cpu_name: self.cpu_name.clone(),
            ghz: self.pdh.as_ref().and_then(Pdh::ghz),
            memory_used,
            memory_total,
            gpu: self.gpu(),
            down_per_sec,
            up_per_sec,
        }
    }

    fn gpu(&self) -> Option<Gpu> {
        let adapter = self.adapter.as_ref()?;
        let pdh = self.pdh.as_ref();
        let usage = pdh
            .and_then(|p| p.engine)
            .map(|counter| usage_by_adapter(&array(counter, PDH_FMT_DOUBLE)).get(&adapter.key).copied().unwrap_or(0.0))
            .unwrap_or(0.0);
        let memory_counter = pdh.and_then(|p| if adapter.shared { p.shared } else { p.dedicated });
        let vram_used = memory_counter.map_or(0.0, |counter| adapter_total(&array(counter, PDH_FMT_DOUBLE), &adapter.key));
        Some(Gpu { usage, name: adapter.name.clone(), vram_used: vram_used as u64, vram_total: adapter.total })
    }
}

fn filetime(time: FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}

fn cpu_times() -> Option<CpuTimes> {
    let (mut idle, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
    // SAFETY: out-pointers to locals.
    unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.ok()?;
    Some(CpuTimes { idle: filetime(idle), kernel: filetime(kernel), user: filetime(user) })
}

fn cpu_name() -> Option<String> {
    let mut buffer = [0u16; 256];
    let mut size = mem::size_of_val(&buffer) as u32;
    // SAFETY: out-buffer of `size` bytes; constant key and value names.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0"),
            w!("ProcessorNameString"),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    (status == ERROR_SUCCESS).then(|| crate::from_wide(&buffer).trim().to_owned())
}

/// Used and total physical memory in bytes.
fn memory() -> (u64, u64) {
    let mut status = MEMORYSTATUSEX { dwLength: mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    // SAFETY: correctly sized out struct.
    if unsafe { GlobalMemoryStatusEx(&mut status) }.is_err() {
        return (0, 0);
    }
    (status.ullTotalPhys.saturating_sub(status.ullAvailPhys), status.ullTotalPhys)
}

/// Received and sent bytes of all physical interfaces that are up.
fn network_octets() -> Option<(u64, u64)> {
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    // SAFETY: GetIfTable2 allocates the table; it is freed with FreeMibTable below.
    unsafe {
        if GetIfTable2(&mut table) != ERROR_SUCCESS || table.is_null() {
            return None;
        }
        let count = (*table).NumEntries as usize;
        let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), count);
        let (mut down, mut up) = (0u64, 0u64);
        for row in rows {
            let flags = row.InterfaceAndOperStatusFlags._bitfield;
            let hardware = flags & 0b01 != 0;
            // Filter drivers (QoS, WFP …) repeat the adapter's traffic.
            let filter = flags & 0b10 != 0;
            if hardware && !filter && row.Type != LOOPBACK && row.OperStatus == IfOperStatusUp {
                down = down.saturating_add(row.InOctets);
                up = up.saturating_add(row.OutOctets);
            }
        }
        FreeMibTable(table as *const _);
        Some((down, up))
    }
}

/// The GPU worth showing: the hardware adapter with the most dedicated memory.
fn main_adapter() -> Option<Adapter> {
    // SAFETY: DXGI factory and adapter descriptions are read-only queries.
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1().ok()?;
        let mut best: Option<(u64, Adapter)> = None;
        let mut index = 0;
        while let Ok(adapter) = factory.EnumAdapters1(index) {
            index += 1;
            let Ok(desc) = adapter.GetDesc1() else { continue };
            if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 || desc.VendorId == BASIC_RENDER_VENDOR {
                continue;
            }
            let dedicated = desc.DedicatedVideoMemory as u64;
            let shared = dedicated < DEDICATED_THRESHOLD;
            let candidate = Adapter {
                name: crate::from_wide(&desc.Description).trim().to_owned(),
                key: luid_key(desc.AdapterLuid.HighPart, desc.AdapterLuid.LowPart),
                shared,
                total: if shared { desc.SharedSystemMemory as u64 } else { dedicated },
            };
            if best.as_ref().is_none_or(|(size, _)| dedicated > *size) {
                best = Some((dedicated, candidate));
            }
        }
        best.map(|(_, adapter)| adapter)
    }
}

/// PDH query with the counters the sampler reads.
struct Pdh {
    query: PDH_HQUERY,
    engine: Option<PDH_HCOUNTER>,
    dedicated: Option<PDH_HCOUNTER>,
    shared: Option<PDH_HCOUNTER>,
    frequency: Option<PDH_HCOUNTER>,
    performance: Option<PDH_HCOUNTER>,
}

impl Pdh {
    fn open() -> Result<Self, String> {
        let mut query = PDH_HQUERY::default();
        // SAFETY: out-pointer to a local; closed in Drop.
        let status = unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) };
        if status != ERROR_SUCCESS.0 {
            return Err(format!("PdhOpenQuery {status:#x}"));
        }
        let add = |path: &str| {
            let mut counter = PDH_HCOUNTER::default();
            // SAFETY: valid query, NUL-terminated English counter path.
            let status = unsafe { PdhAddEnglishCounterW(query, &HSTRING::from(path), 0, &mut counter) };
            if status == ERROR_SUCCESS.0 {
                Some(counter)
            } else {
                log::debug!("sysmon: counter {path} unavailable ({status:#x})");
                None
            }
        };
        let pdh = Self {
            engine: add("\\GPU Engine(*engtype_3D)\\Utilization Percentage"),
            dedicated: add("\\GPU Adapter Memory(*)\\Dedicated Usage"),
            shared: add("\\GPU Adapter Memory(*)\\Shared Usage"),
            frequency: add("\\Processor Information(_Total)\\Processor Frequency"),
            performance: add("\\Processor Information(_Total)\\% Processor Performance"),
            query,
        };
        // Rate counters need a first collection as their baseline.
        pdh.collect();
        Ok(pdh)
    }

    fn collect(&self) {
        // SAFETY: valid query owned by self.
        unsafe { PdhCollectQueryData(self.query) };
    }

    fn ghz(&self) -> Option<f64> {
        let frequency = value(self.frequency?, PDH_FMT_DOUBLE)?;
        let performance = value(self.performance?, PDH_FMT(PDH_FMT_DOUBLE.0 | PDH_FMT_NOCAP100))?;
        current_ghz(frequency, performance)
    }
}

impl Drop for Pdh {
    fn drop(&mut self) {
        // SAFETY: closes the query opened in `open`, with all its counters.
        unsafe { PdhCloseQuery(self.query) };
    }
}

fn value(counter: PDH_HCOUNTER, format: PDH_FMT) -> Option<f64> {
    let mut result = PDH_FMT_COUNTERVALUE::default();
    // SAFETY: valid counter and out struct.
    let status = unsafe { PdhGetFormattedCounterValue(counter, format, None, &mut result) };
    // SAFETY: PDH_FMT_DOUBLE fills the double member; any bit pattern is a valid f64.
    let number = unsafe { result.Anonymous.doubleValue };
    (status == ERROR_SUCCESS.0 && result.CStatus == PDH_CSTATUS_VALID_DATA).then_some(number)
}

/// All instances of a wildcard counter as (instance name, value).
fn array(counter: PDH_HCOUNTER, format: PDH_FMT) -> Vec<(String, f64)> {
    let (mut size, mut count) = (0u32, 0u32);
    // SAFETY: the first call reports the buffer size; the second fills a buffer
    // of that size (items followed by their name strings).
    unsafe {
        if PdhGetFormattedCounterArrayW(counter, format, &mut size, &mut count, None) != PDH_MORE_DATA || size == 0 {
            return Vec::new();
        }
        let item_size = mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        let mut buffer = vec![PDH_FMT_COUNTERVALUE_ITEM_W::default(); (size as usize).div_ceil(item_size)];
        if PdhGetFormattedCounterArrayW(counter, format, &mut size, &mut count, Some(buffer.as_mut_ptr())) != ERROR_SUCCESS.0 {
            return Vec::new();
        }
        buffer
            .iter()
            .take(count as usize)
            .filter(|item| item.FmtValue.CStatus == PDH_CSTATUS_VALID_DATA)
            .map(|item| (item.szName.to_string().unwrap_or_default(), item.FmtValue.Anonymous.doubleValue))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn samples_plausible_values() {
        let mut sampler = Sampler::new();
        sampler.sample();
        thread::sleep(Duration::from_millis(300));
        let sample = sampler.sample();
        assert!((0.0..=100.0).contains(&sample.cpu_usage));
        assert!(sample.memory_total > 0 && sample.memory_used <= sample.memory_total);
        assert!(sample.down_per_sec >= 0.0 && sample.up_per_sec >= 0.0);
        if let Some(gpu) = &sample.gpu {
            assert!((0.0..=100.0).contains(&gpu.usage));
        }
    }

    /// `cargo test -p rice-win prints_a_sample -- --ignored --nocapture`
    #[test]
    #[ignore = "diagnostic output for comparing with the Task Manager"]
    fn prints_a_sample() {
        let mut sampler = Sampler::new();
        sampler.sample();
        thread::sleep(Duration::from_secs(1));
        println!("{:#?}", sampler.sample());
    }
}
