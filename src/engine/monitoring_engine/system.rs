use std::collections::HashMap;
use std::fs;
use super::types::*;

fn parse_proc_stat_line(line: &str) -> PrevCpuCounters {
    let parts: Vec<&str> = line.split_whitespace().collect();
    PrevCpuCounters {
        user: parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0),
        nice: parts.get(2).and_then(|v| v.parse().ok()).unwrap_or(0),
        system: parts.get(3).and_then(|v| v.parse().ok()).unwrap_or(0),
        idle: parts.get(4).and_then(|v| v.parse().ok()).unwrap_or(0),
        iowait: parts.get(5).and_then(|v| v.parse().ok()).unwrap_or(0),
        irq: parts.get(6).and_then(|v| v.parse().ok()).unwrap_or(0),
        softirq: parts.get(7).and_then(|v| v.parse().ok()).unwrap_or(0),
        steal: parts.get(8).and_then(|v| v.parse().ok()).unwrap_or(0),
    }
}

pub fn compute_cpu_deltas(
    current: PrevCpuCounters,
    prev: Option<PrevCpuCounters>,
    load_avg: [f64; 3],
) -> CpuStats {
    let (user_pct, system_pct, idle_pct, iowait_pct, steal_pct) = if let Some(p) = prev {
        let total_delta = current.total().saturating_sub(p.total());
        if total_delta == 0 {
            (0.0, 0.0, 100.0, 0.0, 0.0)
        } else {
            let td = total_delta as f64;
            let user_delta = current
                .user
                .saturating_add(current.nice)
                .saturating_sub(p.user.saturating_add(p.nice));
            let sys_delta = current
                .system
                .saturating_add(current.irq)
                .saturating_add(current.softirq)
                .saturating_sub(p.system.saturating_add(p.irq).saturating_add(p.softirq));
            let idle_delta = current.idle.saturating_sub(p.idle);
            let iowait_delta = current.iowait.saturating_sub(p.iowait);
            let steal_delta = current.steal.saturating_sub(p.steal);

            (
                (user_delta as f64 / td) * 100.0,
                (sys_delta as f64 / td) * 100.0,
                (idle_delta as f64 / td) * 100.0,
                (iowait_delta as f64 / td) * 100.0,
                (steal_delta as f64 / td) * 100.0,
            )
        }
    } else {
        (0.0, 0.0, 100.0, 0.0, 0.0)
    };

    let round2 = |v: f64| (v * 100.0).round() / 100.0;

    CpuStats {
        user_pct: round2(user_pct),
        system_pct: round2(system_pct),
        idle_pct: round2(idle_pct),
        iowait_pct: round2(iowait_pct),
        steal_pct: round2(steal_pct),
        load_avg_1m: load_avg[0],
        load_avg_5m: load_avg[1],
        load_avg_15m: load_avg[2],
    }
}

/// Reads real per-field CPU counters from `/proc/stat` and computes exact deltas against `prev`.
pub fn read_cpu_stats(prev: Option<PrevCpuCounters>) -> (CpuStats, PrevCpuCounters) {
    let content = fs::read_to_string("/proc/stat").unwrap_or_default();
    let line = content
        .lines()
        .find(|l| l.starts_with("cpu "))
        .unwrap_or("");
    let current = parse_proc_stat_line(line);
    let load_avg = read_load_avg();
    let stats = compute_cpu_deltas(current, prev, load_avg);
    (stats, current)
}

/// Read load average from `/proc/loadavg`.
pub fn read_load_avg() -> [f64; 3] {
    let content = fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let parts: Vec<&str> = content.split_whitespace().collect();
    [
        parts.first().and_then(|v| v.parse().ok()).unwrap_or(0.0),
        parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.0),
        parts.get(2).and_then(|v| v.parse().ok()).unwrap_or(0.0),
    ]
}

/// Read memory stats from `/proc/meminfo`.
pub fn read_memory_stats() -> MemoryStats {
    let content = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut map: HashMap<&str, u64> = HashMap::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(val) = parts[1].parse::<u64>() {
                map.insert(parts[0].trim_end_matches(':'), val);
            }
        }
    }

    let total = map.get("MemTotal").copied().unwrap_or(0);
    let available = map.get("MemAvailable").copied().unwrap_or(0);
    let free = map.get("MemFree").copied().unwrap_or(0);
    let buffers = map.get("Buffers").copied().unwrap_or(0);
    let cached = map.get("Cached").copied().unwrap_or(0);
    let swap_total = map.get("SwapTotal").copied().unwrap_or(0);
    let swap_free = map.get("SwapFree").copied().unwrap_or(0);

    let used = total.saturating_sub(available);
    let used_pct = if total > 0 {
        (used as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let swap_used_pct = if swap_total > 0 {
        (swap_total.saturating_sub(swap_free) as f64 / swap_total as f64) * 100.0
    } else {
        0.0
    };

    MemoryStats {
        total_kb: total,
        available_kb: available,
        free_kb: free,
        buffers_kb: buffers,
        cached_kb: cached,
        swap_total_kb: swap_total,
        swap_free_kb: swap_free,
        used_pct: (used_pct * 100.0).round() / 100.0,
        swap_used_pct: (swap_used_pct * 100.0).round() / 100.0,
    }
}

/// Read disk stats using `libc::statvfs` for the root filesystem.
pub fn read_disk_stats() -> DiskStats {
    #[cfg(unix)]
    {
        use std::mem::MaybeUninit;
        let mut buf = MaybeUninit::<libc::statvfs>::uninit();
        let path = b"/\0";
        let result =
            unsafe { libc::statvfs(path.as_ptr() as *const libc::c_char, buf.as_mut_ptr()) };
        if result == 0 {
            let s = unsafe { buf.assume_init() };
            let block_size = s.f_frsize as u64;
            let total = s.f_blocks as u64 * block_size;
            let free = s.f_bavail as u64 * block_size;
            let used = total.saturating_sub(free);
            let used_pct = if total > 0 {
                (used as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            return DiskStats {
                total_bytes: total,
                used_bytes: used,
                free_bytes: free,
                used_pct: (used_pct * 100.0).round() / 100.0,
            };
        }
    }
    DiskStats {
        total_bytes: 0,
        used_bytes: 0,
        free_bytes: 0,
        used_pct: 0.0,
    }
}

/// Read network interface stats from `/proc/net/dev`.
pub fn read_network_stats() -> Vec<NetworkInterfaceStats> {
    let content = fs::read_to_string("/proc/net/dev").unwrap_or_default();
    let mut result = Vec::new();

    for line in content.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 12 {
            continue;
        }
        let name = parts[0].trim_end_matches(':').to_string();
        if name == "lo" {
            continue;
        }
        result.push(NetworkInterfaceStats {
            name,
            rx_bytes: parts[1].parse().unwrap_or(0),
            rx_packets: parts[2].parse().unwrap_or(0),
            rx_errors: parts[3].parse().unwrap_or(0),
            tx_bytes: parts[9].parse().unwrap_or(0),
            tx_packets: parts[10].parse().unwrap_or(0),
            tx_errors: parts[11].parse().unwrap_or(0),
        });
    }
    result
}
