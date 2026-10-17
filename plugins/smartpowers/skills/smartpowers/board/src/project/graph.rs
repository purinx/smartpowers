//! The dependency graph of a project's tasks: the execution order read
//! from `tasks/PLAN.md`, each task's depth, which tasks are ready to start,
//! and what is wrong with the graph.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::task::{StatusKey, Task};

/// Computed once per `load_project`. A broken graph never fails the load:
/// every task gets a depth and the problems are listed in `issues`.
#[derive(Debug, Default)]
pub struct Graph {
    /// Length of the longest dependency path from a task with no
    /// dependencies (depth 0) to each task.
    pub depth: BTreeMap<String, u32>,
    /// Planned tasks whose dependencies all exist and are Done, outside
    /// any cycle.
    pub ready: BTreeSet<String>,
    pub issues: Vec<GraphIssue>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GraphIssue {
    /// `task` depends on a task number with no task file.
    MissingDependency { task: String, dependency: String },
    /// Tasks in or downstream of a dependency cycle, in plan order.
    Cycle { tasks: Vec<String> },
}

/// Task numbers in the order `PLAN.md` first mentions their files
/// (`NN_name.md`), which is the plan's execution order.
pub fn plan_order(plan: &str) -> Vec<String> {
    let bytes = plan.as_bytes();
    let mut order: Vec<String> = Vec::new();
    for start in 0..bytes.len() {
        let rest = &bytes[start..];
        let boundary =
            start == 0 || !(bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
        if !boundary
            || rest.len() < 7
            || !rest[..2].iter().all(u8::is_ascii_digit)
            || rest[2] != b'_'
        {
            continue;
        }
        let name_len = rest[3..]
            .iter()
            .take_while(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || **b == b'_')
            .count();
        if name_len > 0 && rest[3 + name_len..].starts_with(b".md") {
            let number = String::from_utf8_lossy(&rest[..2]).into_owned();
            if !order.contains(&number) {
                order.push(number);
            }
        }
    }
    order
}

/// Depths by Kahn's algorithm over the edges between existing tasks,
/// readiness, and issues for missing dependencies and cycles.
pub fn analyze(tasks: &[Task], plan_order: &[String]) -> Graph {
    let known: BTreeSet<&str> = tasks.iter().map(|t| t.number.as_str()).collect();
    let mut issues = Vec::new();
    // Resolved dependencies per task number, and the reverse edges.
    let mut deps: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for task in tasks {
        let entry = deps.entry(task.number.as_str()).or_default();
        for dep in &task.depends_on {
            if known.contains(dep.as_str()) {
                entry.insert(dep.as_str());
            } else {
                issues.push(GraphIssue::MissingDependency {
                    task: task.number.clone(),
                    dependency: dep.clone(),
                });
            }
        }
    }
    for (&task, task_deps) in &deps {
        for &dep in task_deps {
            dependents.entry(dep).or_default().push(task);
        }
    }

    let mut remaining: BTreeMap<&str, usize> =
        deps.iter().map(|(&task, d)| (task, d.len())).collect();
    let mut depth: BTreeMap<String, u32> = BTreeMap::new();
    let mut queue: VecDeque<&str> = remaining
        .iter()
        .filter(|(_, &count)| count == 0)
        .map(|(&task, _)| task)
        .collect();
    for &task in &queue {
        depth.insert(task.to_string(), 0);
    }
    while let Some(task) = queue.pop_front() {
        remaining.remove(task);
        let next = depth[task] + 1;
        for &dependent in dependents.get(task).into_iter().flatten() {
            let slot = depth.entry(dependent.to_string()).or_insert(0);
            *slot = (*slot).max(next);
            let count = remaining.get_mut(dependent).expect("unprocessed dependent");
            *count -= 1;
            if *count == 0 {
                queue.push_back(dependent);
            }
        }
    }

    // Whatever Kahn's algorithm left is in or downstream of a cycle. Place
    // each one after its dependencies that did get a depth.
    let cyclic: BTreeSet<&str> = remaining.keys().copied().collect();
    for &task in &cyclic {
        let placed = deps[task]
            .iter()
            .filter(|dep| !cyclic.contains(*dep))
            .map(|dep| depth[*dep] + 1)
            .max()
            .unwrap_or(0);
        depth.insert(task.to_string(), placed);
    }
    if !cyclic.is_empty() {
        let position = |task: &str| {
            plan_order
                .iter()
                .position(|n| n == task)
                .unwrap_or(usize::MAX)
        };
        let mut tasks: Vec<&str> = cyclic.iter().copied().collect();
        tasks.sort_by_key(|task| (position(task), *task));
        issues.push(GraphIssue::Cycle {
            tasks: tasks.into_iter().map(str::to_string).collect(),
        });
    }

    let status: BTreeMap<&str, StatusKey> = tasks
        .iter()
        .map(|t| (t.number.as_str(), t.status_key))
        .collect();
    let ready = tasks
        .iter()
        .filter(|task| {
            task.status_key == StatusKey::Planned
                && !cyclic.contains(task.number.as_str())
                && task
                    .depends_on
                    .iter()
                    .all(|dep| status.get(dep.as_str()) == Some(&StatusKey::Done))
        })
        .map(|task| task.number.clone())
        .collect();

    Graph {
        depth,
        ready,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn task(number: &str, status: StatusKey, depends_on: &[&str]) -> Task {
        Task {
            number: number.to_string(),
            file: format!("{number}_task.md"),
            title: String::new(),
            status: String::new(),
            status_key: status,
            requirements: Vec::new(),
            depends_on: depends_on.iter().map(|d| d.to_string()).collect(),
            remote: None,
            updated: SystemTime::UNIX_EPOCH,
            body: String::new(),
            comments: Vec::new(),
            log: Vec::new(),
        }
    }

    fn depths(graph: &Graph) -> Vec<(&str, u32)> {
        graph.depth.iter().map(|(n, d)| (n.as_str(), *d)).collect()
    }

    fn ready(graph: &Graph) -> Vec<&str> {
        graph.ready.iter().map(String::as_str).collect()
    }

    use StatusKey::{Blocked, Done, InProgress, Planned};

    #[test]
    fn reads_execution_order_from_the_plan() {
        let plan = "| 02 | [B](02_domain.md) | 01 |\n| 01 | [A](01_infra.md) | None |\nsee 02_domain.md\nx01_no.md";
        assert_eq!(plan_order(plan), ["02", "01"]);
    }

    #[test]
    fn linear_chain_depths() {
        let tasks = [
            task("01", Done, &[]),
            task("02", Planned, &["01"]),
            task("03", Planned, &["02"]),
        ];
        let graph = analyze(&tasks, &[]);
        assert_eq!(depths(&graph), [("01", 0), ("02", 1), ("03", 2)]);
        assert_eq!(ready(&graph), ["02"]);
        assert!(graph.issues.is_empty());
    }

    #[test]
    fn diamond_uses_the_longest_path() {
        // 01 -> 02 -> 03 -> 05 and 01 -> 04 -> 05.
        let tasks = [
            task("01", Planned, &[]),
            task("02", Planned, &["01"]),
            task("03", Planned, &["02"]),
            task("04", Planned, &["01"]),
            task("05", Planned, &["03", "04"]),
        ];
        let graph = analyze(&tasks, &[]);
        assert_eq!(
            depths(&graph),
            [("01", 0), ("02", 1), ("03", 2), ("04", 1), ("05", 3)]
        );
        assert_eq!(ready(&graph), ["01"]);
    }

    #[test]
    fn ready_only_when_planned_and_all_dependencies_done() {
        let tasks = [
            task("01", Done, &[]),
            task("02", InProgress, &[]),
            task("03", Planned, &["01", "02"]),
            task("04", Planned, &["01"]),
            task("05", Blocked, &["01"]),
            task("06", InProgress, &["01"]),
            task("07", Planned, &[]),
        ];
        let graph = analyze(&tasks, &[]);
        assert_eq!(ready(&graph), ["04", "07"]);
    }

    #[test]
    fn missing_dependency_is_reported_and_not_ready() {
        let tasks = [task("01", Done, &[]), task("03", Planned, &["01", "09"])];
        let graph = analyze(&tasks, &[]);
        assert_eq!(
            graph.issues,
            [GraphIssue::MissingDependency {
                task: "03".into(),
                dependency: "09".into()
            }]
        );
        assert_eq!(depths(&graph), [("01", 0), ("03", 1)]);
        assert!(graph.ready.is_empty());
    }

    #[test]
    fn cycle_is_one_issue_with_depths_and_nothing_ready() {
        let tasks = [
            task("01", Done, &[]),
            task("04", Planned, &["01", "05"]),
            task("05", Planned, &["04"]),
            task("06", Planned, &["05"]),
        ];
        let order = ["01", "05", "04"].map(String::from);
        let graph = analyze(&tasks, &order);
        assert_eq!(
            graph.issues,
            [GraphIssue::Cycle {
                tasks: vec!["05".into(), "04".into(), "06".into()]
            }]
        );
        assert_eq!(depths(&graph), [("01", 0), ("04", 1), ("05", 0), ("06", 0)]);
        assert!(graph.ready.is_empty());
    }
}
