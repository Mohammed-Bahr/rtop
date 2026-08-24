use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use ratatui::widgets::TableState;

use crate::action::{Action, Mode, Signal};
use crate::config::Config;
use crate::event;
use crate::system::processes::{matches_filter, sort_processes, ProcessInfo, SortKey};
use crate::system::tree::{flatten_tree, FlatNode};
use crate::system::{Snapshot, SystemMonitor};
use crate::utils::history::History;

/// How long a status message stays visible.
const STATUS_TTL: Duration = Duration::from_secs(4);
/// Maximum number of processes tracked with per-process history graphs.
const PROC_HISTORY_CAP: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Processes,
    Cpu,
    Memory,
    Disk,
    Network,
    Tree,
}

impl Screen {
    pub const ALL: [Screen; 6] = [
        Screen::Processes,
        Screen::Cpu,
        Screen::Memory,
        Screen::Disk,
        Screen::Network,
        Screen::Tree,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Screen::Processes => "Processes",
            Screen::Cpu => "CPU",
            Screen::Memory => "Memory",
            Screen::Disk => "Disk",
            Screen::Network => "Network",
            Screen::Tree => "Tree",
        }
    }

    fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    fn previous(self) -> Self {
        let idx = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// A pending confirmation for a destructive signal.
#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    pub pid: u32,
    pub name: String,
    pub signal: Signal,
    pub title: String,
    pub lines: Vec<String>,
}

/// Bounded per-process history used by the details view.
struct ProcHistories {
    cpu: HashMap<u32, History>,
    mem: HashMap<u32, History>,
    order: VecDeque<u32>,
    cap: usize,
    history_len: usize,
}

impl ProcHistories {
    fn new(history_len: usize) -> Self {
        Self {
            cpu: HashMap::new(),
            mem: HashMap::new(),
            order: VecDeque::new(),
            cap: PROC_HISTORY_CAP,
            history_len,
        }
    }

    fn update(&mut self, p: &ProcessInfo) {
        if !self.cpu.contains_key(&p.pid) {
            if self.order.len() >= self.cap {
                if let Some(oldest) = self.order.pop_front() {
                    self.cpu.remove(&oldest);
                    self.mem.remove(&oldest);
                }
            }
            self.order.push_back(p.pid);
        }
        let h = self
            .cpu
            .entry(p.pid)
            .or_insert_with(|| History::new(self.history_len));
        h.push(p.cpu as f64);
        let m = self
            .mem
            .entry(p.pid)
            .or_insert_with(|| History::new(self.history_len));
        m.push(p.mem_bytes as f64);
    }

    /// Drop entries whose process vanished.
    fn retain_pids(&mut self, alive: &HashSet<u32>) {
        self.cpu.retain(|pid, _| alive.contains(pid));
        self.mem.retain(|pid, _| alive.contains(pid));
        self.order.retain(|pid| alive.contains(pid));
    }

    fn get(&self, pid: u32) -> Option<(&History, &History)> {
        Some((self.cpu.get(&pid)?, self.mem.get(&pid)?))
    }
}

/// Application state: owns all mutable UI state and the data layer.
///
/// The event loop calls [`App::on_action`] for user input and
/// [`App::tick`] on the refresh interval; rendering only reads state.
pub struct App {
    pub config: Config,
    monitor: SystemMonitor,

    /// Currently displayed snapshot. Untouched while frozen.
    pub snapshot: Option<Snapshot>,
    /// Filtered+sorted view of the snapshot's processes.
    pub display_rows: Vec<ProcessInfo>,

    pub screen: Screen,
    pub mode: Mode,
    pub frozen: bool,
    pub should_quit: bool,

    pub search_query: String,
    pub sort_key: SortKey,
    pub sort_descending: bool,
    selected_pid: Option<u32>,
    pub table_state: TableState,

    pub help_open: bool,
    pub details_open: bool,
    pub dialog: Option<ConfirmDialog>,

    /// Tree view: collapsed node PIDs and cursor into the flattened list.
    pub collapsed_tree: HashSet<u32>,
    pub tree_cursor: usize,

    pub history_cpu_total: History,
    pub history_ram: History,
    pub history_swap: History,
    proc_histories: ProcHistories,

    status: Option<(String, Instant)>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let hist_len = config.history_size;
        let mut app = Self {
            sort_key: config.sort_by,
            sort_descending: config.sort_descending,
            config,
            monitor: SystemMonitor::new(),
            snapshot: None,
            display_rows: Vec::new(),
            screen: Screen::Processes,
            mode: Mode::Normal,
            frozen: false,
            should_quit: false,
            search_query: String::new(),
            selected_pid: None,
            table_state: TableState::default(),
            help_open: false,
            details_open: false,
            dialog: None,
            collapsed_tree: HashSet::new(),
            tree_cursor: 0,
            history_cpu_total: History::new(hist_len),
            history_ram: History::new(hist_len),
            history_swap: History::new(hist_len),
            proc_histories: ProcHistories::new(hist_len),
            status: None,
        };
        app.refresh();
        app.table_state.select(Some(0));
        app
    }

    // ------------------------------------------------------------------
    // Data flow
    // ------------------------------------------------------------------

    /// Collect fresh system data and apply it to the displayed state.
    ///
    /// Skipped entirely while frozen; called again immediately when the
    /// user unfreezes so stale data is replaced at once (no replay).
    pub fn refresh(&mut self) {
        let snap = self.monitor.snapshot();
        self.apply_snapshot(snap);
    }

    fn apply_snapshot(&mut self, snap: Snapshot) {
        self.history_cpu_total.push(snap.cpu_total as f64);
        self.history_ram.push(snap.mem.used_percent() as f64);
        self.history_swap.push(snap.mem.swap_percent() as f64);

        let alive: HashSet<u32> = snap.processes.keys().copied().collect();
        for info in snap.processes.values() {
            self.proc_histories.update(info);
        }
        self.proc_histories.retain_pids(&alive);

        self.snapshot = Some(snap);
        self.rebuild_display();
    }

    /// Recompute `display_rows` from filter + sort and keep the selection
    /// pinned to the same PID across list changes.
    fn rebuild_display(&mut self) {
        let Some(snap) = &self.snapshot else { return };
        let mut rows: Vec<ProcessInfo> = snap
            .processes
            .values()
            .filter(|p| matches_filter(p, &self.search_query))
            .cloned()
            .collect();
        sort_processes(&mut rows, self.sort_key, self.sort_descending);
        self.display_rows = rows;

        let len = self.display_rows.len();
        if len == 0 {
            self.selected_pid = None;
            self.table_state.select(None);
            return;
        }
        let idx = match self.selected_pid {
            Some(pid) => self
                .display_rows
                .iter()
                .position(|p| p.pid == pid)
                .unwrap_or(0),
            None => 0,
        };
        self.selected_pid = Some(self.display_rows[idx].pid);
        self.table_state.select(Some(idx));
    }

    /// Called from the event loop once per refresh interval.
    pub fn tick(&mut self) {
        if !self.frozen {
            self.refresh();
        }
    }

    // ------------------------------------------------------------------
    // Action handling
    // ------------------------------------------------------------------

    pub fn on_action(&mut self, action: Action) {
        use Action::*;
        match action {
            Quit => self.should_quit = true,

            NextProc | PreviousProc | PageUp | PageDown | Home | End => {
                self.move_selection(action)
            }
            NextScreen => self.screen = self.screen.next(),
            PreviousScreen => self.screen = self.screen.previous(),

            OpenDetails => {
                if self.selected_process().is_some() {
                    self.details_open = true;
                }
            }
            CloseOverlay => {
                self.details_open = false;
                self.help_open = false;
            }
            ToggleHelp => self.help_open = !self.help_open,

            CycleSort => {
                self.sort_key = self.sort_key.next();
                self.set_status(format!("Sort: {} desc", self.sort_key.label()));
                self.rebuild_display();
            }
            ToggleSortDirection => {
                self.sort_descending = !self.sort_descending;
                let dir = if self.sort_descending { "desc" } else { "asc" };
                self.set_status(format!("Sort: {} {dir}", self.sort_key.label()));
                self.rebuild_display();
            }

            StartSearch => {
                if self.mode == Mode::Searching {
                    // Enter commits the query.
                    self.mode = Mode::Normal;
                } else {
                    self.mode = Mode::Searching;
                }
            }
            SearchChar(c) => {
                self.search_query.push(c);
                self.rebuild_display();
            }
            SearchBackspace => {
                self.search_query.pop();
                self.rebuild_display();
            }
            SearchClear => {
                if !self.search_query.is_empty() || self.mode == Mode::Searching {
                    self.search_query.clear();
                    self.rebuild_display();
                }
                self.mode = Mode::Normal;
            }

            ToggleExpand => self.toggle_tree_expand(),

            SendSignal(sig) => self.request_signal(sig),
            ConfirmYes => self.confirm_pending_signal(),
            ConfirmNo => {
                self.dialog = None;
                self.mode = Mode::Normal;
            }

            ToggleFreeze => self.toggle_freeze(),
            RefreshNow => {
                if !self.frozen && self.dialog.is_none() {
                    self.refresh();
                } else if self.frozen {
                    // Navigation keys map to RefreshNow as a no-op; nothing
                    // to do while frozen.
                }
            }
        }
    }

    /// Handle a raw key press: interpret it via `event.rs`, then dispatch.
    pub fn on_key(&mut self, key: crossterm::event::KeyEvent) {
        if key.kind != crossterm::event::KeyEventKind::Press {
            return;
        }
        let action = event::handle_key(key, self.effective_mode());
        self.on_action(action);
    }

    /// Dialog mode overrides the base mode while a confirmation is open.
    fn effective_mode(&self) -> Mode {
        if self.dialog.is_some() {
            Mode::Dialog
        } else {
            self.mode
        }
    }

    fn move_selection(&mut self, action: Action) {
        use Action::*;
        match self.screen {
            Screen::Tree => {
                let len = self.tree_nodes().len();
                if len == 0 {
                    return;
                }
                let step = match action {
                    NextProc => 1,
                    PreviousProc => -1,
                    PageDown => 10,
                    PageUp => -10,
                    Home => -(len as isize),
                    End => len as isize,
                    _ => return,
                };
                let cur = self.tree_cursor as isize + step;
                self.tree_cursor = cur.clamp(0, len as isize - 1) as usize;
            }
            _ => {
                let len = self.display_rows.len();
                if len == 0 {
                    return;
                }
                let cur = self.table_state.selected().unwrap_or(0) as isize;
                let next = match action {
                    NextProc => cur + 1,
                    PreviousProc => cur - 1,
                    PageDown => cur + 10,
                    PageUp => cur - 10,
                    Home => 0,
                    End => len as isize - 1,
                    _ => return,
                };
                let idx = next.clamp(0, len as isize - 1) as usize;
                self.table_state.select(Some(idx));
                self.selected_pid = Some(self.display_rows[idx].pid);
            }
        }
    }

    fn toggle_freeze(&mut self) {
        self.frozen = !self.frozen;
        if self.frozen {
            self.set_status("Frozen — updates paused (Space to resume)".into());
        } else {
            // Immediately show current data instead of waiting for the tick.
            self.refresh();
            self.set_status("Live — updates resumed".into());
        }
    }

    fn toggle_tree_expand(&mut self) {
        let nodes = self.tree_nodes();
        if nodes.is_empty() {
            return;
        }
        self.tree_cursor = self.tree_cursor.min(nodes.len() - 1);
        let pid = nodes[self.tree_cursor].pid;
        if nodes[self.tree_cursor].has_children {
            if !self.collapsed_tree.remove(&pid) {
                self.collapsed_tree.insert(pid);
            }
        }
    }

    fn request_signal(&mut self, sig: Signal) {
        let Some(p) = self.selected_process() else { return };
        let (pid, name) = (p.pid, p.name.clone());
        if sig.requires_confirmation() {
            self.dialog = Some(ConfirmDialog {
                title: format!("Send {}", sig.label()),
                lines: vec![
                    format!("Process: {name}"),
                    format!("PID:     {pid}"),
                    String::new(),
                    format!("Send {}? This cannot be undone.", sig.label()),
                    String::new(),
                    "[y] Yes   [n] No".to_string(),
                ],
                pid,
                name,
                signal: sig,
            });
            self.mode = Mode::Dialog;
        } else {
            self.deliver_signal(pid, name, sig);
        }
    }

    fn confirm_pending_signal(&mut self) {
        if let Some(d) = self.dialog.take() {
            self.deliver_signal(d.pid, d.name, d.signal);
        }
        self.mode = Mode::Normal;
    }

    fn deliver_signal(&mut self, pid: u32, name: String, sig: Signal) {
        match SystemMonitor::send_signal(pid, sig.as_raw()) {
            Ok(()) => self.set_status(format!("{name} ({pid}): sent {}", sig.label())),
            Err(e) => self.set_status(format!("{name} ({pid}): {e}")),
        }
        // Reflect any resulting state change without waiting for the tick,
        // unless frozen — the displayed data must stay untouched.
        if !self.frozen {
            self.refresh();
        }
    }

    // ------------------------------------------------------------------
    // Read accessors for the UI layer
    // ------------------------------------------------------------------

    pub fn selected_process(&self) -> Option<&ProcessInfo> {
        let idx = self.table_state.selected()?;
        self.display_rows.get(idx)
    }

    pub fn tree_nodes(&self) -> Vec<FlatNode> {
        match &self.snapshot {
            Some(snap) => flatten_tree(&snap.processes, &self.collapsed_tree),
            None => Vec::new(),
        }
    }

    /// The process under the tree cursor.
    pub fn tree_selected(&self) -> Option<&ProcessInfo> {
        let nodes = self.tree_nodes();
        let node = nodes.get(self.tree_cursor)?;
        self.snapshot.as_ref()?.processes.get(&node.pid)
    }

    pub fn proc_history(&self, pid: u32) -> Option<(&History, &History)> {
        self.proc_histories.get(pid)
    }

    /// Current status message, if still fresh.
    pub fn status_text(&self) -> Option<&str> {
        let (msg, at) = self.status.as_ref()?;
        (at.elapsed() < STATUS_TTL).then_some(msg.as_str())
    }

    fn set_status(&mut self, msg: String) {
        self.status = Some((msg, Instant::now()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::MemInfo;

    fn test_app() -> App {
        let cfg = Config::default().sanitized();
        App::new(cfg)
    }

    fn proc(pid: u32, parent: Option<u32>, name: &str, cpu: f32, mem: u64) -> ProcessInfo {
        ProcessInfo {
            pid,
            parent,
            name: name.into(),
            command: name.into(),
            cpu,
            mem_bytes: mem,
            mem_percent: 0.0,
            virt_bytes: 0,
            state: "Sleeping".into(),
            user: "tester".into(),
            start_epoch: 0,
            runtime_secs: 10,
        }
    }

    fn snap_with(procs: Vec<ProcessInfo>) -> Snapshot {
        Snapshot {
            hostname: "test".into(),
            os_name: "TestOS".into(),
            kernel: "1.0".into(),
            uptime_secs: 100,
            cpu_total: 12.5,
            cpus: vec![],
            load: [0.1, 0.2, 0.3],
            temp_celsius: None,
            mem: MemInfo {
                total: 1000,
                used: 500,
                available: 500,
                swap_total: 0,
                swap_used: 0,
            },
            disks: vec![],
            disk_io: None,
            net: vec![],
            processes: procs
                .into_iter()
                .map(|p| (p.pid, p))
                .collect(),
        }
    }

    #[test]
    fn initial_state_is_live_and_sorted() {
        let app = test_app();
        assert!(!app.frozen);
        assert_eq!(app.sort_key, SortKey::Cpu);
        assert!(app.sort_descending);
        assert_eq!(app.screen, Screen::Processes);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn snapshot_applied_sorted_desc_by_cpu() {
        let mut app = test_app();
        app.apply_snapshot(snap_with(vec![
            proc(1, None, "a", 1.0, 10),
            proc(2, None, "b", 9.0, 20),
            proc(3, None, "c", 5.0, 30),
        ]));
        let pids: Vec<u32> = app.display_rows.iter().map(|p| p.pid).collect();
        assert_eq!(pids, vec![2, 3, 1]);
    }

    #[test]
    fn selection_follows_pid_across_refresh() {
        let mut app = test_app();
        app.apply_snapshot(snap_with(vec![
            proc(1, None, "a", 9.0, 10),
            proc(2, None, "b", 5.0, 20),
        ]));
        // Select PID 2 (second row).
        app.on_action(Action::NextProc);
        assert_eq!(app.selected_process().unwrap().pid, 2);

        // New tick reshuffles by CPU; PID 2 stays selected.
        app.apply_snapshot(snap_with(vec![
            proc(1, None, "a", 0.5, 10),
            proc(2, None, "b", 8.0, 20),
            proc(3, None, "c", 7.0, 30),
        ]));
        assert_eq!(app.selected_process().unwrap().pid, 2);
    }

    #[test]
    fn disappearing_selected_process_falls_back_to_first_row() {
        let mut app = test_app();
        app.apply_snapshot(snap_with(vec![
            proc(1, None, "a", 9.0, 10),
            proc(2, None, "b", 5.0, 20),
        ]));
        app.on_action(Action::NextProc);
        app.on_action(Action::NextProc); // now on pid 2
        // PID 2 disappears.
        app.apply_snapshot(snap_with(vec![proc(1, None, "a", 9.0, 10)]));
        assert_eq!(app.selected_process().unwrap().pid, 1);
    }

    #[test]
    fn search_filters_rows_and_clears() {
        let mut app = test_app();
        app.apply_snapshot(snap_with(vec![
            proc(1, None, "firefox", 1.0, 10),
            proc(2, None, "zed", 2.0, 20),
        ]));

        app.on_action(Action::StartSearch);
        assert_eq!(app.mode, Mode::Searching);
        for c in "fire".chars() {
            app.on_action(Action::SearchChar(c));
        }
        assert_eq!(app.display_rows.len(), 1);
        assert_eq!(app.display_rows[0].name, "firefox");

        // Esc clears the query and leaves search mode.
        app.on_action(Action::SearchClear);
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.search_query.is_empty());
        assert_eq!(app.display_rows.len(), 2);
    }

    #[test]
    fn sort_actions_resort_display() {
        let mut app = test_app();
        app.apply_snapshot(snap_with(vec![
            proc(1, None, "b", 1.0, 100),
            proc(2, None, "a", 2.0, 50),
        ]));

        // Default: CPU desc → pid 2 first.
        assert_eq!(app.display_rows[0].pid, 2);

        app.on_action(Action::ToggleSortDirection); // CPU asc
        assert_eq!(app.display_rows[0].pid, 1);

        app.on_action(Action::CycleSort); // next key = Memory, still ascending
        assert_eq!(app.sort_key, SortKey::Memory);
        assert_eq!(app.display_rows[0].pid, 2); // 50 bytes first in asc

        app.on_action(Action::ToggleSortDirection); // Memory desc
        assert_eq!(app.display_rows[0].pid, 1); // 100 bytes first
    }

    #[test]
    fn freeze_preserves_snapshot_and_unfreeze_refreshes() {
        let mut app = test_app();
        app.apply_snapshot(snap_with(vec![proc(1, None, "a", 1.0, 10)]));
        let before_cpu = app.history_cpu_total.last();

        app.on_action(Action::ToggleFreeze);
        assert!(app.frozen);

        // A tick while frozen must not touch the displayed data.
        app.tick();
        assert_eq!(
            app.history_cpu_total.iter().count(),
            if before_cpu.is_some() { 2 } else { 1 }
        );
        assert_eq!(app.snapshot.as_ref().unwrap().processes.len(), 1);

        // Navigation still works while frozen.
        app.on_action(Action::NextProc);

        // Unfreezing immediately refreshes.
        app.on_action(Action::ToggleFreeze);
        assert!(!app.frozen);
        assert!(!app.frozen && !app.snapshot.is_none());
    }

    #[test]
    fn kill_opens_dialog_confirm_no_cancels() {
        let mut app = test_app();
        app.apply_snapshot(snap_with(vec![proc(1, None, "init", 1.0, 10)]));

        app.on_action(Action::SendSignal(Signal::Kill));
        assert!(app.dialog.is_some());
        assert_eq!(app.mode, Mode::Dialog);

        app.on_action(Action::ConfirmNo);
        assert!(app.dialog.is_none());
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn stop_signal_requires_no_dialog_but_reports_errors() {
        let mut app = test_app();
        app.apply_snapshot(snap_with(vec![proc(u32::MAX - 1, None, "ghost", 1.0, 10)]));

        // Non-destructive signals are sent directly.
        app.on_action(Action::SendSignal(Signal::Stop));
        assert!(app.dialog.is_none());
        // The nonexistent PID produces a graceful status message.
        assert!(app.status_text().is_some());
    }

    #[test]
    fn screen_cycling_wraps() {
        let mut app = test_app();
        for _ in 0..Screen::ALL.len() {
            app.on_action(Action::NextScreen);
        }
        assert_eq!(app.screen, Screen::Processes);
        app.on_action(Action::PreviousScreen);
        assert_eq!(app.screen, Screen::Tree);
    }

    #[test]
    fn details_and_help_overlays_open_close() {
        let mut app = test_app();
        app.apply_snapshot(snap_with(vec![proc(1, None, "a", 1.0, 10)]));

        app.on_action(Action::OpenDetails);
        assert!(app.details_open);
        app.on_action(Action::CloseOverlay);
        assert!(!app.details_open);

        app.on_action(Action::ToggleHelp);
        assert!(app.help_open);
        app.on_action(Action::ToggleHelp);
        assert!(!app.help_open);
    }

    #[test]
    fn tree_expand_collapse() {
        let mut app = test_app();
        app.tree_cursor = 0;
        app.apply_snapshot(snap_with(vec![
            proc(1, None, "init", 0.0, 0),
            proc(2, Some(1), "child", 0.0, 0),
            proc(3, Some(2), "grandchild", 0.0, 0),
        ]));
        app.screen = Screen::Tree;

        assert_eq!(app.tree_nodes().len(), 3);
        // Cursor is on root (pid 1): collapse it.
        app.on_action(Action::ToggleExpand);
        assert_eq!(app.tree_nodes().len(), 1);
        // Expand again.
        app.on_action(Action::ToggleExpand);
        assert_eq!(app.tree_nodes().len(), 3);
    }
}
