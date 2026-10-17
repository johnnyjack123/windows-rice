//! Arithmetic of the system monitor: usage from counter deltas, rates and
//! GPU counter instances.

use std::collections::HashMap;

/// System-wide CPU times in 100-ns ticks (`GetSystemTimes`; kernel includes idle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CpuTimes {
    pub idle: u64,
    pub kernel: u64,
    pub user: u64,
}

/// CPU usage in percent between two samples.
pub fn cpu_usage(previous: CpuTimes, next: CpuTimes) -> f64 {
    let total = next.kernel.saturating_sub(previous.kernel) + next.user.saturating_sub(previous.user);
    let idle = next.idle.saturating_sub(previous.idle);
    if total == 0 {
        return 0.0;
    }
    percent(total.saturating_sub(idle) as f64 / total as f64)
}

/// A process' share of the whole machine in percent: CPU time used in the
/// interval divided by the interval times the number of logical processors.
pub fn process_cpu(previous_ticks: u64, next_ticks: u64, elapsed_ticks: u64, processors: u32) -> f64 {
    let capacity = elapsed_ticks as f64 * f64::from(processors.max(1));
    if capacity <= 0.0 {
        return 0.0;
    }
    percent(next_ticks.saturating_sub(previous_ticks) as f64 / capacity)
}

fn percent(fraction: f64) -> f64 {
    (fraction * 100.0).clamp(0.0, 100.0)
}

/// Bytes per second between two counter readings; a reset counter gives 0.
pub fn rate(previous: u64, next: u64, elapsed_secs: f64) -> f64 {
    if elapsed_secs <= 0.0 || next < previous {
        return 0.0;
    }
    (next - previous) as f64 / elapsed_secs
}

/// The PDH instance key of a GPU adapter, e.g. `luid_0x00000000_0x0000d1b2`.
pub fn luid_key(high: i32, low: u32) -> String {
    format!("luid_0x{:08x}_0x{low:08x}", high as u32)
}

/// Extracts the adapter key from a PDH instance name such as
/// `pid_1234_luid_0x00000000_0x0000D1B2_phys_0_eng_3_engtype_3D`.
pub fn instance_luid(instance: &str) -> Option<String> {
    let lower = instance.to_ascii_lowercase();
    let start = lower.find("luid_0x")?;
    let key = lower.get(start..start + "luid_0x00000000_0x00000000".len())?;
    key.chars().skip(7).all(|c| c.is_ascii_hexdigit() || c == '_' || c == 'x').then(|| key.to_owned())
}

/// Sums engine utilisation per adapter (several processes use one engine),
/// capped at 100 % like the Task Manager.
pub fn usage_by_adapter(instances: &[(String, f64)]) -> HashMap<String, f64> {
    let mut sums: HashMap<String, f64> = HashMap::new();
    for (name, value) in instances {
        if let Some(key) = instance_luid(name)
            && value.is_finite()
        {
            *sums.entry(key).or_default() += value.max(0.0);
        }
    }
    for value in sums.values_mut() {
        *value = value.min(100.0);
    }
    sums
}

/// Sum of all counter instances that belong to one adapter.
pub fn adapter_total(instances: &[(String, f64)], key: &str) -> f64 {
    instances
        .iter()
        .filter(|(name, value)| value.is_finite() && instance_luid(name).as_deref() == Some(key))
        .map(|(_, value)| value.max(0.0))
        .sum()
}

/// Current clock in GHz from the nominal frequency (MHz) and `% Processor
/// Performance` (above 100 while boosting).
pub fn current_ghz(frequency_mhz: f64, performance: f64) -> Option<f64> {
    (frequency_mhz > 0.0 && performance > 0.0 && frequency_mhz.is_finite() && performance.is_finite())
        .then(|| (frequency_mhz * performance / 100.0 / 1000.0 * 100.0).round() / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_cpu_usage() {
        let a = CpuTimes { idle: 1_000, kernel: 2_000, user: 1_000 };
        let b = CpuTimes { idle: 1_750, kernel: 3_000, user: 1_000 };
        // 1000 ticks elapsed, 750 idle → 25 %.
        assert_eq!(cpu_usage(a, b), 25.0);
        assert_eq!(cpu_usage(a, a), 0.0);
    }

    #[test]
    fn computes_process_share() {
        // 2 s on 8 logical CPUs, the process used 4 CPU-seconds → 25 %.
        assert_eq!(process_cpu(0, 40_000_000, 20_000_000, 8), 25.0);
        assert_eq!(process_cpu(10, 5, 20_000_000, 8), 0.0);
        assert_eq!(process_cpu(0, 10, 0, 8), 0.0);
        assert_eq!(process_cpu(0, 1_000_000_000, 10, 1), 100.0);
    }

    #[test]
    fn computes_rates() {
        assert_eq!(rate(1_000, 3_000, 2.0), 1_000.0);
        assert_eq!(rate(3_000, 1_000, 2.0), 0.0);
        assert_eq!(rate(0, 10, 0.0), 0.0);
    }

    #[test]
    fn groups_gpu_engines_by_adapter() {
        let key = luid_key(0, 0xd1b2);
        assert_eq!(key, "luid_0x00000000_0x0000d1b2");
        assert_eq!(instance_luid("pid_1234_luid_0x00000000_0x0000D1B2_phys_0_eng_3_engtype_3D").as_deref(), Some(key.as_str()));
        assert_eq!(instance_luid("_Total"), None);
        let items = vec![
            ("pid_1_luid_0x00000000_0x0000D1B2_phys_0_eng_0_engtype_3D".to_owned(), 30.0),
            ("pid_2_luid_0x00000000_0x0000D1B2_phys_0_eng_0_engtype_3D".to_owned(), 25.5),
            ("pid_2_luid_0x00000000_0x0000AAAA_phys_0_eng_0_engtype_3D".to_owned(), 90.0),
            ("pid_3_luid_0x00000000_0x0000AAAA_phys_0_eng_0_engtype_3D".to_owned(), 40.0),
        ];
        let usage = usage_by_adapter(&items);
        assert_eq!(usage[&key], 55.5);
        assert_eq!(usage["luid_0x00000000_0x0000aaaa"], 100.0);
        let memory = vec![("luid_0x00000000_0x0000D1B2_phys_0".to_owned(), 512.0)];
        assert_eq!(adapter_total(&memory, &key), 512.0);
        assert_eq!(luid_key(-1, 1), "luid_0xffffffff_0x00000001");
    }

    #[test]
    fn computes_the_current_clock() {
        assert_eq!(current_ghz(3000.0, 120.0), Some(3.6));
        assert_eq!(current_ghz(0.0, 100.0), None);
        assert_eq!(current_ghz(2995.0, f64::NAN), None);
    }
}
