use std::collections::{BTreeMap, HashSet};

use crate::system::processes::ProcessInfo;

/// A node in the flattened process tree ready for rendering.
#[derive(Debug, Clone)]
pub struct FlatNode {
    pub pid: u32,
    pub depth: usize,
    /// True when this node has children and they are currently hidden.
    pub collapsed: bool,
    /// True when this node has children at all.
    pub has_children: bool,
}

/// Flatten a process map into an ordered list of visible tree nodes.
///
/// * Roots are processes whose parent is absent from the map (or that are
///   their own parent, as some kernel threads report).
/// * Children are ordered by name.
/// * PIDs listed in `collapsed` are rendered but their descendants are hidden.
///
/// Guards against parent/child cycles so malformed data cannot cause
/// infinite recursion or duplicate output.
pub fn flatten_tree(
    processes: &BTreeMap<u32, ProcessInfo>,
    collapsed: &HashSet<u32>,
) -> Vec<FlatNode> {
    // child_map: parent pid -> sorted child pids
    let mut child_map: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut roots: Vec<u32> = Vec::new();

    for (pid, info) in processes {
        let parent = match info.parent {
            Some(p) if p != *pid && processes.contains_key(&p) => p,
            _ => {
                roots.push(*pid);
                continue;
            }
        };
        child_map.entry(parent).or_default().push(*pid);
    }

    let order_key = |pid: &u32| {
        processes
            .get(pid)
            .map(|p| (p.name.to_lowercase(), p.pid))
            .unwrap_or_default()
    };
    for children in child_map.values_mut() {
        children.sort_by_key(order_key);
    }
    roots.sort_by_key(order_key);

    let mut out = Vec::with_capacity(processes.len());
    let mut visited = HashSet::new();

    fn walk(
        pid: u32,
        depth: usize,
        processes: &BTreeMap<u32, ProcessInfo>,
        child_map: &BTreeMap<u32, Vec<u32>>,
        collapsed: &HashSet<u32>,
        visited: &mut HashSet<u32>,
        out: &mut Vec<FlatNode>,
    ) {
        if !visited.insert(pid) {
            return; // cycle guard
        }
        let children = child_map.get(&pid).map(Vec::as_slice).unwrap_or(&[]);
        out.push(FlatNode {
            pid,
            depth,
            collapsed: !children.is_empty() && collapsed.contains(&pid),
            has_children: !children.is_empty(),
        });
        if collapsed.contains(&pid) {
            return;
        }
        for child in children {
            walk(*child, depth + 1, processes, child_map, collapsed, visited, out);
        }
    }

    let mut ordered_roots = roots;
    ordered_roots.sort_by_key(order_key);

    for root in &ordered_roots {
        walk(
            *root,
            0,
            processes,
            &child_map,
            collapsed,
            &mut visited,
            &mut out,
        );
    }

    // Processes caught in parent/child cycles are never reached from a real
    // root; surface them as additional roots so nothing silently disappears.
    // (Descendants merely hidden behind a collapsed node are NOT stranded.)
    let mut stranded: Vec<u32> = processes
        .keys()
        .copied()
        .filter(|pid| !visited.contains(pid) && leads_to_cycle(*pid, processes, &visited))
        .collect();
    stranded.sort_by_key(order_key);
    for pid in &stranded {
        walk(
            *pid,
            0,
            processes,
            &child_map,
            collapsed,
            &mut visited,
            &mut out,
        );
    }

    out
}

/// Walk up the parent chain from `pid`; `true` if the chain loops without
/// reaching an already-visited node (i.e. the process sits on a cycle).
fn leads_to_cycle(
    start: u32,
    processes: &BTreeMap<u32, ProcessInfo>,
    visited: &HashSet<u32>,
) -> bool {
    let mut seen: HashSet<u32> = HashSet::new();
    let mut current = Some(start);
    while let Some(pid) = current {
        if !seen.insert(pid) {
            return true;
        }
        if visited.contains(&pid) {
            return false;
        }
        current = processes.get(&pid).and_then(|info| info.parent).and_then(|p| {
            (p != pid && processes.contains_key(&p)).then_some(p)
        });
    }
    // Chain left the map entirely; such orphans are always roots, so they
    // cannot end up here unvisited in practice.
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc(pid: u32, parent: Option<u32>, name: &str) -> ProcessInfo {
        ProcessInfo {
            pid,
            parent,
            name: name.into(),
            command: name.into(),
            cpu: 0.0,
            mem_bytes: 0,
            mem_percent: 0.0,
            virt_bytes: 0,
            state: "S".into(),
            user: "u".into(),
            start_epoch: 0,
            runtime_secs: 0,
        }
    }

    fn map(entries: Vec<(u32, Option<u32>, &str)>) -> BTreeMap<u32, ProcessInfo> {
        entries
            .into_iter()
            .map(|(pid, parent, name)| (pid, proc(pid, parent, name)))
            .collect()
    }

    #[test]
    fn basic_hierarchy() {
        // systemd -> {firefox, docker -> containerd}
        let procs = map(vec![
            (1, None, "systemd"),
            (10, Some(1), "firefox"),
            (20, Some(1), "docker"),
            (21, Some(20), "containerd"),
        ]);
        let flat = flatten_tree(&procs, &HashSet::new());
        let ids: Vec<u32> = flat.iter().map(|n| n.pid).collect();
        assert_eq!(ids, vec![1, 20, 21, 10]);
        // containerd is two levels deep under systemd.
        assert_eq!(flat[2].depth, 2);
        // docker has children, firefox does not.
        assert!(flat[1].has_children);
        assert!(!flat[3].has_children);
    }

    #[test]
    fn collapsing_hides_descendants_only() {
        let procs = map(vec![
            (1, None, "init"),
            (10, Some(1), "shell"),
            (11, Some(10), "child"),
        ]);
        let collapsed: HashSet<u32> = [10u32].into();
        let flat = flatten_tree(&procs, &collapsed);
        eprintln!("flat = {flat:?}");
        assert_eq!(flat.len(), 2);
        assert_eq!(flat[1].pid, 10);
        assert!(flat[1].collapsed);
    }

    #[test]
    fn missing_parent_becomes_root() {
        let procs = map(vec![(7, Some(999), "orphan"), (1, None, "init")]);
        let flat = flatten_tree(&procs, &HashSet::new());
        let ids: Vec<u32> = flat.iter().map(|n| n.pid).collect();
        assert_eq!(ids, vec![1, 7]);
    }

    #[test]
    fn self_parent_does_not_recurse() {
        let procs = map(vec![(5, Some(5), "weird")]);
        let flat = flatten_tree(&procs, &HashSet::new());
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].depth, 0);
    }

    #[test]
    fn cycles_are_broken() {
        // A -> B -> A cycle plus normal root.
        let procs = map(vec![
            (1, None, "init"),
            (2, Some(3), "a"),
            (3, Some(2), "b"),
        ]);
        let flat = flatten_tree(&procs, &HashSet::new());
        // Every process appears exactly once.
        let ids: Vec<u32> = flat.iter().map(|n| n.pid).collect();
        let mut sorted_ids = ids.clone();
        sorted_ids.sort_unstable();
        assert_eq!(sorted_ids, vec![1, 2, 3]);
    }

    #[test]
    fn siblings_sorted_by_name() {
        let procs = map(vec![
            (1, None, "z"),
            (2, Some(1), "alpha"),
            (3, Some(1), "Beta"),
        ]);
        let flat = flatten_tree(&procs, &HashSet::new());
        let names: Vec<&str> = flat.iter().map(|n| procs[&n.pid].name.as_str()).collect();
        assert_eq!(names, vec!["z", "alpha", "Beta"]);
    }
}
