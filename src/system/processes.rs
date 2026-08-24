use serde::{Deserialize, Serialize};

/// A single process as displayed by the UI.
///
/// Plain data, deliberately decoupled from `sysinfo` so that sorting,
/// filtering and tree building are pure functions and unit-testable.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub parent: Option<u32>,
    pub name: String,
    /// Full command line (may be empty for kernel threads).
    pub command: String,
    pub cpu: f32,
    pub mem_bytes: u64,
    /// Percentage of total physical RAM.
    pub mem_percent: f32,
    pub virt_bytes: u64,
    /// Human-readable state, e.g. "Running", "Sleeping".
    pub state: String,
    pub user: String,
    /// Unix epoch seconds when the process started.
    pub start_epoch: u64,
    /// Seconds of wall-clock time since start.
    pub runtime_secs: u64,
}

/// Columns the process list can be sorted by.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    Pid,
    Name,
    #[default]
    Cpu,
    Memory,
    Runtime,
}

impl SortKey {
    /// All keys in cycle order used by the `s` shortcut.
    pub const ALL: [SortKey; 5] = [
        SortKey::Cpu,
        SortKey::Memory,
        SortKey::Pid,
        SortKey::Name,
        SortKey::Runtime,
    ];

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|k| *k == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn label(self) -> &'static str {
        match self {
            SortKey::Pid => "PID",
            SortKey::Name => "Name",
            SortKey::Cpu => "CPU",
            SortKey::Memory => "Memory",
            SortKey::Runtime => "Runtime",
        }
    }
}

/// Sort `rows` in place by `key`. Ordering direction follows `descending`.
///
/// Numeric keys compare numerically; ties fall back to PID so the order is
/// stable and predictable between frames.
pub fn sort_processes(rows: &mut [ProcessInfo], key: SortKey, descending: bool) {
    rows.sort_by(|a, b| {
        let ord = match key {
            SortKey::Pid => a.pid.cmp(&b.pid),
            SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortKey::Cpu => a
                .cpu
                .partial_cmp(&b.cpu)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortKey::Memory => a.mem_bytes.cmp(&b.mem_bytes),
            SortKey::Runtime => a.runtime_secs.cmp(&b.runtime_secs),
        };
        let ord = if descending {
            ord.reverse()
        } else {
            ord
        };
        // Tie-break by PID ascending regardless of direction.
        if ord == std::cmp::Ordering::Equal {
            a.pid.cmp(&b.pid)
        } else {
            ord
        }
    });
}

/// Case-insensitive substring match against the process name, user or PID.
pub fn matches_filter(p: &ProcessInfo, filter: &str) -> bool {
    let needle = filter.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    p.name.to_lowercase().contains(&needle)
        || p.user.to_lowercase().contains(&needle)
        || p.pid.to_string().contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pid: u32, name: &str, cpu: f32, mem: u64, runtime: u64) -> ProcessInfo {
        ProcessInfo {
            pid,
            parent: None,
            name: name.into(),
            command: name.into(),
            cpu,
            mem_bytes: mem,
            mem_percent: 0.0,
            virt_bytes: 0,
            state: "Running".into(),
            user: "u".into(),
            start_epoch: 0,
            runtime_secs: runtime,
        }
    }

    #[test]
    fn sort_by_cpu_descending() {
        let mut rows = vec![row(3, "c", 5.0, 10, 1), row(1, "a", 50.0, 20, 2)];
        sort_processes(&mut rows, SortKey::Cpu, true);
        assert_eq!(rows[0].pid, 1);
        assert_eq!(rows[1].pid, 3);
    }

    #[test]
    fn sort_by_memory_ascending() {
        let mut rows = vec![row(1, "a", 0.0, 100, 1), row(2, "b", 0.0, 50, 2)];
        sort_processes(&mut rows, SortKey::Memory, false);
        assert_eq!(rows[0].pid, 2);
    }

    #[test]
    fn sort_by_name_is_case_insensitive() {
        let mut rows = vec![
            row(1, "banana", 0.0, 0, 0),
            row(2, "Apple", 0.0, 0, 0),
            row(3, "cherry", 0.0, 0, 0),
        ];
        sort_processes(&mut rows, SortKey::Name, false);
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["Apple", "banana", "cherry"]);
    }

    #[test]
    fn sort_ties_break_by_pid_regardless_of_direction() {
        let mut rows = vec![row(9, "x", 1.0, 10, 5), row(2, "y", 1.0, 10, 5)];
        sort_processes(&mut rows, SortKey::Cpu, true);
        assert_eq!(rows[0].pid, 2);
    }

    #[test]
    fn sort_by_runtime() {
        let mut rows = vec![row(1, "a", 0.0, 0, 999), row(2, "b", 0.0, 0, 12)];
        sort_processes(&mut rows, SortKey::Runtime, true);
        assert_eq!(rows[0].pid, 1);
    }

    #[test]
    fn filter_matches_name_pid_and_user() {
        let p = row(1234, "Firefox", 0.0, 0, 0);
        let mut p = p;
        p.user = "alice".into();
        assert!(matches_filter(&p, "fire"));
        assert!(matches_filter(&p, "FOX"));
        assert!(matches_filter(&p, "123"));
        assert!(matches_filter(&p, "alic"));
        assert!(matches_filter(&p, ""));
        assert!(!matches_filter(&p, "chrome"));
    }

    #[test]
    fn sort_key_cycle() {
        assert_eq!(SortKey::Cpu.next(), SortKey::Memory);
        assert_eq!(SortKey::Runtime.next(), SortKey::Cpu);
    }
}
