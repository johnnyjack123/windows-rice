//! System monitor (docs/BACKEND.md §4 "Systemmonitor"). Samples are only
//! taken while a window is subscribed; subscriptions are counted per window
//! and dropped when that window reloads or closes, so a reloaded page never
//! leaves the sampler running.

use crate::blocking::blocking;
use crate::state::{AppState, Locked};
use rice_core::contract::{CpuSample, GpuSample, MemorySample, NetworkSample, ProcessInfo, SystemSample, events};
use rice_core::time::now_iso;
use rice_core::{BackendError, BackendResult};
use rice_services::imaging::rgba_to_png_data_url;
use rice_win::processes::{KillError, ProcessSampler};
use rice_win::sysmon::{Sample, Sampler};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, Thread};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Time between the baseline and the first real sample.
const WARM_UP: Duration = Duration::from_millis(300);
const ICON_SIZE: u32 = 48;

/// System monitor state.
#[derive(Default)]
pub struct Monitor {
    /// Subscriptions per window label.
    subscribers: Mutex<HashMap<String, u32>>,
    runner: Mutex<Option<Runner>>,
    processes: Mutex<ProcessSampler>,
    /// Exe path → icon data URL.
    icons: Mutex<HashMap<PathBuf, Option<String>>>,
}

struct Runner {
    stop: Arc<AtomicBool>,
    thread: Thread,
}

/// `system_subscribe`: the first subscriber starts the sampler.
pub fn subscribe(app: &AppHandle, window: &str, interval_ms: f64) {
    let interval = if interval_ms.is_finite() { interval_ms.clamp(250.0, 10_000.0) } else { 1000.0 };
    let monitor = &app.state::<AppState>().sysmon;
    let mut subscribers = monitor.subscribers.locked();
    *subscribers.entry(window.to_owned()).or_default() += 1;
    let mut runner = monitor.runner.locked();
    if runner.is_none() {
        match start(app.clone(), Duration::from_millis(interval as u64)) {
            Ok(started) => *runner = Some(started),
            Err(error) => log::warn!("sysmon: sampler not started ({error})"),
        }
    }
}

/// `system_unsubscribe`: the last one stops it.
pub fn unsubscribe(app: &AppHandle, window: &str) {
    let monitor = &app.state::<AppState>().sysmon;
    let mut subscribers = monitor.subscribers.locked();
    if let Some(count) = subscribers.get_mut(window) {
        *count = count.saturating_sub(1);
        if *count == 0 {
            subscribers.remove(window);
        }
    }
    stop_if_unused(monitor, &subscribers);
}

/// A window reloaded or closed: its subscriptions are gone.
pub fn forget(app: &AppHandle, window: &str) {
    let monitor = &app.state::<AppState>().sysmon;
    let mut subscribers = monitor.subscribers.locked();
    if subscribers.remove(window).is_some() {
        stop_if_unused(monitor, &subscribers);
    }
}

fn stop_if_unused(monitor: &Monitor, subscribers: &HashMap<String, u32>) {
    if subscribers.is_empty()
        && let Some(runner) = monitor.runner.locked().take()
    {
        runner.stop.store(true, Ordering::SeqCst);
        runner.thread.unpark();
    }
}

fn start(app: AppHandle, interval: Duration) -> std::io::Result<Runner> {
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = Arc::clone(&stop);
    let handle = thread::Builder::new().name("rice-sysmon".into()).spawn(move || {
        let mut sampler = Sampler::new();
        sampler.sample();
        thread::park_timeout(WARM_UP);
        while !stopped.load(Ordering::SeqCst) {
            let _ = app.emit(events::SYSTEM_SAMPLE, to_contract(sampler.sample()));
            thread::park_timeout(interval);
        }
    })?;
    Ok(Runner { stop, thread: handle.thread().clone() })
}

fn to_contract(sample: Sample) -> SystemSample {
    SystemSample {
        time: now_iso(),
        cpu: CpuSample { usage: sample.cpu_usage, name: sample.cpu_name, ghz: sample.ghz },
        memory: MemorySample { used_bytes: sample.memory_used, total_bytes: sample.memory_total },
        gpu: sample.gpu.map(|gpu| GpuSample {
            usage: gpu.usage,
            name: gpu.name,
            vram_used_bytes: gpu.vram_used,
            vram_total_bytes: gpu.vram_total,
        }),
        network: NetworkSample { down_bytes_per_sec: sample.down_per_sec, up_bytes_per_sec: sample.up_per_sec },
    }
}

/// `system_processes`: the top processes by CPU.
pub async fn processes(app: &AppHandle, limit: f64) -> BackendResult<Vec<ProcessInfo>> {
    let limit = if limit.is_finite() { limit.clamp(0.0, 200.0) as usize } else { 12 };
    let handle = app.clone();
    blocking(move || {
        let monitor = &handle.state::<AppState>().sysmon;
        let mut list = {
            let mut sampler = monitor.processes.locked();
            if !sampler.has_baseline() {
                sampler.sample();
                thread::sleep(WARM_UP);
            }
            sampler.sample()
        };
        list.sort_by(|a, b| b.cpu.total_cmp(&a.cpu).then(b.memory.cmp(&a.memory)));
        list.truncate(limit);
        Ok(list
            .into_iter()
            .map(|p| ProcessInfo { pid: p.pid, name: p.name, cpu: p.cpu, memory_bytes: p.memory, icon_url: icon(monitor, p.pid) })
            .collect())
    })
    .await
}

/// Icon of a process' executable, cached by path.
fn icon(monitor: &Monitor, pid: u32) -> Option<String> {
    let path = rice_win::processes::exe_path(pid)?;
    if let Some(cached) = monitor.icons.locked().get(&path) {
        return cached.clone();
    }
    let url = rice_win::processes::icon_rgba(&path, ICON_SIZE)
        .and_then(|(width, height, rgba)| rgba_to_png_data_url(width, height, rgba).ok());
    monitor.icons.locked().insert(path, url.clone());
    url
}

/// `system_kill`. The idle and system processes and rice itself are off limits.
pub async fn kill(pid: u32) -> BackendResult<()> {
    if pid == 0 || pid == 4 || pid == std::process::id() {
        return Err(BackendError::access_denied("this process cannot be ended"));
    }
    blocking(move || {
        rice_win::processes::kill(pid).map_err(|error| match error {
            KillError::AccessDenied => BackendError::access_denied("the process is elevated or protected"),
            KillError::NotFound => BackendError::not_found(format!("no process {pid}")),
            KillError::Failed(message) => BackendError::internal(message),
        })
    })
    .await
}
