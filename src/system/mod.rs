pub mod processes;
pub mod tree;

use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use sysinfo::{
    ComponentExt, CpuExt, DiskExt, NetworkExt, NetworksExt, PidExt, ProcessExt,
    ProcessRefreshKind, ProcessStatus, System, SystemExt, UserExt,
};

use processes::ProcessInfo;

/// Per-CPU-core utilisation sample.
#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub name: String,
    pub usage: f32,
    /// Frequency in MHz, when the kernel exposes it.
    pub freq_mhz: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct MemInfo {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub swap_total: u64,
    pub swap_used: u64,
}

impl MemInfo {
    pub fn used_percent(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.used as f32 / self.total as f32 * 100.0
        }
    }

    pub fn swap_percent(&self) -> f32 {
        if self.swap_total == 0 {
            0.0
        } else {
            self.swap_used as f32 / self.swap_total as f32 * 100.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub total: u64,
    pub available: u64,
    pub removable: bool,
}

impl DiskInfo {
    pub fn used(&self) -> u64 {
        self.total.saturating_sub(self.available)
    }

    pub fn used_percent(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.used() as f32 / self.total as f32 * 100.0
        }
    }
}

/// Aggregate disk I/O throughput across all physical devices.
#[derive(Debug, Clone, Copy)]
pub struct DiskIo {
    pub read_bps: f64,
    pub write_bps: f64,
}

#[derive(Debug, Clone)]
pub struct NetInfo {
    pub interface: String,
    /// Bytes per second since the previous snapshot.
    pub rx_bps: f64,
    pub tx_bps: f64,
    pub rx_total: u64,
    pub tx_total: u64,
}

/// Everything the UI renders for one refresh cycle.
///
/// Immutable value type: the app swaps in a fresh `Snapshot` each tick
/// (or keeps the old one while frozen), which cleanly separates data
/// collection from display state.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub hostname: String,
    pub os_name: String,
    pub kernel: String,
    pub uptime_secs: u64,
    pub cpu_total: f32,
    pub cpus: Vec<CpuInfo>,
    pub load: [f32; 3],
    /// Best-effort CPU package temperature in Celsius.
    pub temp_celsius: Option<f32>,
    pub mem: MemInfo,
    pub disks: Vec<DiskInfo>,
    pub disk_io: Option<DiskIo>,
    pub net: Vec<NetInfo>,
    pub processes: BTreeMap<u32, ProcessInfo>,
}

impl Snapshot {
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
}

/// Data layer wrapping `sysinfo` plus Linux-specific `/proc/diskstats`
/// sampling for block-device throughput (not exposed by sysinfo).
///
/// One instance is reused for the whole program lifetime; CPU percentages
/// are deltas between consecutive refreshes, so callers must keep a steady
/// tick cadence for meaningful values.
pub struct SystemMonitor {
    system: System,
    last_diskstats: Option<(Instant, u64, u64)>,
}

fn username_map(system: &System) -> HashMap<String, String> {
    system
        .users()
        .iter()
        .map(|u| ((**u.id()).to_string(), u.name().to_string()))
        .collect()
}

fn state_label(status: ProcessStatus) -> String {
    // Keep labels short and friendly; fall back to the raw letter code.
    match status {
        ProcessStatus::Run => "Running".to_string(),
        ProcessStatus::Sleep => "Sleeping".to_string(),
        ProcessStatus::Idle => "Idle".to_string(),
        ProcessStatus::Stop => "Stopped".to_string(),
        ProcessStatus::Zombie => "Zombie".to_string(),
        ProcessStatus::Dead => "Dead".to_string(),
        other => format!("{other}"),
    }
}

fn parse_proc_diskstats(line: &str) -> Option<(u64, u64)> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 10 {
        return None;
    }
    let dev = fields[2];
    if dev.starts_with("loop") || dev.starts_with("ram") || dev.starts_with("zram") {
        return None;
    }
    let read_sectors: u64 = fields[5].parse().ok()?;
    let write_sectors: u64 = fields[9].parse().ok()?;
    // Sectors are always 512 bytes at this interface.
    Some((read_sectors * 512, write_sectors * 512))
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut monitor = Self {
            system: System::new_all(),
            last_diskstats: None,
        };
        // sysinfo needs two samples to compute CPU deltas; take them back to
        // back at startup so the first rendered frame is meaningful.
        monitor.system.refresh_all();
        std::thread::sleep(std::time::Duration::from_millis(200));
        monitor.system.refresh_all();
        monitor
    }

    /// Collect a full snapshot. Cost scales with process count; call this
    /// once per tick, never per frame.
    pub fn snapshot(&mut self) -> Snapshot {
        self.system.refresh_memory();
        let sys = &mut self.system;
        sys.refresh_cpu();
        sys.refresh_cpu();
        sys.refresh_processes_specifics(ProcessRefreshKind::everything());
        sys.refresh_networks();
        sys.refresh_disks_list();
        sys.refresh_components_list();

        let users = username_map(sys);

        // --- CPUs ---
        let cpus: Vec<CpuInfo> = sys
            .cpus()
            .iter()
            .map(|c| CpuInfo {
                name: c.name().to_string(),
                usage: c.cpu_usage(),
                freq_mhz: {
                    let f = c.frequency();
                    (f > 0).then_some(f)
                },
            })
            .collect();
        let cpu_total = cpus.first().map(|c| c.usage).unwrap_or(0.0);

        // --- Temperature (best effort; label varies by driver) ---
        let temp_celsius = sys
            .components()
            .iter()
            .find(|c| {
                let l = c.label().to_lowercase();
                l.contains("package") || l.contains("cpu")
            })
            .map(|c| c.temperature());

        // --- Memory ---
        let mem = MemInfo {
            total: sys.total_memory(),
            used: sys.used_memory(),
            available: sys.available_memory(),
            swap_total: sys.total_swap(),
            swap_used: sys.used_swap(),
        };

        // --- Disks ---
        let disks: Vec<DiskInfo> = sys
            .disks()
            .iter()
            .map(|d| DiskInfo {
                name: d.name().to_string_lossy().into_owned(),
                mount_point: d.mount_point().display().to_string(),
                file_system: String::from_utf8_lossy(d.file_system()).into_owned(),
                total: d.total_space(),
                available: d.available_space(),
                removable: d.is_removable(),
            })
            .collect();

        let disk_io = self.sample_disk_io();
        let sys = &self.system;

        // --- Network ---
        let net: Vec<NetInfo> = sys
            .networks()
            .iter()
            .map(|(name, data)| NetInfo {
                interface: name.clone(),
                rx_bps: data.received() as f64,
                tx_bps: data.transmitted() as f64,
                rx_total: data.total_received(),
                tx_total: data.total_transmitted(),
            })
            .collect();

        // --- Processes ---
        let mut procs = BTreeMap::new();
        for p in sys.processes().values() {
            let uid_key = p.user_id().map(|u| (**u).to_string());
            procs.insert(
                p.pid().as_u32(),
                ProcessInfo {
                    pid: p.pid().as_u32(),
                    parent: p.parent().map(|pid| pid.as_u32()),
                    name: p.name().to_string(),
                    command: p.cmd().join(" "),
                    cpu: p.cpu_usage(),
                    mem_bytes: p.memory(),
                    mem_percent: if mem.total > 0 {
                        p.memory() as f32 / mem.total as f32 * 100.0
                    } else {
                        0.0
                    },
                    virt_bytes: p.virtual_memory(),
                    state: state_label(p.status()),
                    user: uid_key.and_then(|k| users.get(&k).cloned()).unwrap_or_default(),
                    start_epoch: p.start_time(),
                    runtime_secs: p.run_time(),
                },
            );
        }

        let la = sys.load_average();
        Snapshot {
            hostname: sys.host_name().unwrap_or_default(),
            os_name: sys
                .long_os_version()
                .or_else(|| sys.name().map(|n| n.to_string()))
                .unwrap_or_else(|| "unknown".into()),
            kernel: sys.kernel_version().unwrap_or_default(),
            uptime_secs: sys.uptime(),
            cpu_total,
            cpus,
            load: [la.one as f32, la.five as f32, la.fifteen as f32],
            temp_celsius,
            mem,
            disks,
            disk_io,
            net,
            processes: procs,
        }
    }

    /// Read `/proc/diskstats`, diff against the previous sample and derive
    /// aggregate read/write throughput. Returns `None` on the very first
    /// call or when `/proc/diskstats` is unavailable.
    #[cfg(target_os = "linux")]
    fn sample_disk_io(&mut self) -> Option<DiskIo> {
        let content = std::fs::read_to_string("/proc/diskstats").ok()?;
        let now = Instant::now();

        let mut total_read: u64 = 0;
        let mut total_write: u64 = 0;
        for line in content.lines() {
            if let Some((r, w)) = parse_proc_diskstats(line) {
                total_read += r;
                total_write += w;
            }
        }

        let prev = self.last_diskstats.replace((now, total_read, total_write))?;
        let dt = now.duration_since(prev.0).as_secs_f64();
        if dt <= 0.0 {
            return None;
        }
        Some(DiskIo {
            read_bps: total_read.saturating_sub(prev.1) as f64 / dt,
            write_bps: total_write.saturating_sub(prev.2) as f64 / dt,
        })
    }

    #[cfg(not(target_os = "linux"))]
    fn sample_disk_io(&mut self) -> Option<DiskIo> {
        None
    }

    /// Send a POSIX signal to a PID. Isolated here so the rest of the app
    /// stays platform-neutral; returns a human-readable error on failure.
    #[cfg(target_os = "linux")]
    pub fn send_signal(pid: u32, sig: i32) -> Result<(), String> {
        // SAFETY: kill is a plain syscall wrapper; we only pass integers and
        // inspect errno afterwards.
        let rc = unsafe { libc::kill(pid as libc::pid_t, sig) };
        if rc == 0 {
            Ok(())
        } else {
            let msg = match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::EPERM) => "permission denied",
                Some(libc::ESRCH) => "process no longer exists",
                _ => "signal failed",
            };
            Err(msg.to_string())
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn send_signal(_pid: u32, _sig: i32) -> Result<(), String> {
        Err("process signals are only supported on Linux".to_string())
    }

}
