//! Which tasks the lead agent may start now, for `smartpowers-board next`.
//!
//! Tasks are considered in `tasks/PLAN.md` order. A task is ready when it is
//! `Planned` and every task it depends on is `Done`. Tasks run concurrently
//! only when they share a parallel group: the planning guide assigns a group
//! only to tasks with no dependency edge and no overlapping file ownership.

use super::{Project, StatusKey, Task};

pub struct Schedule<'a> {
    /// Tasks to hand to subagents now, in plan order.
    pub start: Vec<&'a Task>,
    pub running: Vec<&'a Task>,
    /// Why nothing can start, when `start` is empty.
    pub reason: Option<String>,
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

pub fn next(project: &Project, limit: Option<usize>) -> Schedule<'_> {
    let rank = |task: &Task| {
        project
            .plan_order
            .iter()
            .position(|number| *number == task.number)
            .unwrap_or(usize::MAX)
    };
    let mut tasks: Vec<&Task> = project.tasks.iter().collect();
    tasks.sort_by_key(|task| (rank(task), task.number.clone()));

    let is_done = |number: &String| {
        project
            .tasks
            .iter()
            .any(|task| task.number == *number && task.status_key == StatusKey::Done)
    };
    let running: Vec<&Task> = tasks
        .iter()
        .copied()
        .filter(|task| task.status_key == StatusKey::InProgress)
        .collect();
    let ready: Vec<&Task> = tasks
        .iter()
        .copied()
        .filter(|task| task.status_key == StatusKey::Planned && task.depends_on.iter().all(is_done))
        .collect();

    let (group, reason) = match running.as_slice() {
        [] => match ready.first() {
            None => (None, Some(idle_reason(&tasks))),
            Some(first) => (Some(first.parallel_group.as_str()), None),
        },
        [first, rest @ ..] => {
            let group = first.parallel_group.as_str();
            if group.is_empty() || rest.iter().any(|task| task.parallel_group != group) {
                let numbers: Vec<&str> = running.iter().map(|t| t.number.as_str()).collect();
                (None, Some(format!("Task {} is running and shares no parallel group with other tasks; wait for it to finish.", numbers.join(", "))))
            } else {
                (Some(group), None)
            }
        }
    };

    let mut start: Vec<&Task> = match group {
        // A task without a parallel group runs alone, and only when nothing else is running.
        Some("") => ready.first().copied().into_iter().collect(),
        Some(group) => ready
            .iter()
            .copied()
            .filter(|task| task.parallel_group == group)
            .collect(),
        None => Vec::new(),
    };
    if let Some(limit) = limit {
        start.truncate(limit);
    }
    let reason = match (&reason, start.is_empty()) {
        (None, true) => Some(format!(
            "No ready task is in parallel group {} with the running tasks; wait for them to finish.",
            group.unwrap_or_default()
        )),
        _ => reason,
    };
    Schedule {
        start,
        running,
        reason,
    }
}

fn idle_reason(tasks: &[&Task]) -> String {
    if tasks.iter().all(|task| task.status_key == StatusKey::Done) {
        return "Every task is Done.".into();
    }
    let blocked: Vec<&str> = tasks
        .iter()
        .filter(|task| task.status_key == StatusKey::Blocked)
        .map(|task| task.number.as_str())
        .collect();
    if blocked.is_empty() {
        "No Planned task has all of its dependencies Done.".into()
    } else {
        format!(
            "No Planned task has all of its dependencies Done; blocked: {}.",
            blocked.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{ProjectSummary, Remote};
    use std::collections::BTreeMap;
    use std::time::UNIX_EPOCH;

    fn task(number: &str, status: StatusKey, group: &str, depends_on: &[&str]) -> Task {
        Task {
            number: number.into(),
            file: format!("{number}_t.md"),
            title: format!("Task {number}"),
            status: status.as_str().into(),
            status_key: status,
            requirements: Vec::new(),
            depends_on: depends_on.iter().map(|d| d.to_string()).collect(),
            parallel_group: group.into(),
            remote: None::<Remote>,
            updated: UNIX_EPOCH,
            body: String::new(),
            comments: Vec::new(),
            log: Vec::new(),
        }
    }

    fn project(tasks: Vec<Task>, plan_order: &[&str]) -> Project {
        Project {
            summary: ProjectSummary {
                id: "p".into(),
                title: "p".into(),
                status: "Active".into(),
                updated: UNIX_EPOCH,
                task_count: tasks.len(),
                counts: BTreeMap::new(),
            },
            phases: Vec::new(),
            path: String::new(),
            plan_order: plan_order.iter().map(|n| n.to_string()).collect(),
            tasks,
        }
    }

    fn numbers(tasks: &[&Task]) -> Vec<String> {
        tasks.iter().map(|t| t.number.clone()).collect()
    }

    use StatusKey::{Blocked, Done, InProgress, Planned};

    #[test]
    fn reads_execution_order_from_the_plan() {
        let plan = "| 02 | [B](02_domain.md) | 01 |\n| 01 | [A](01_infra.md) | None |\nsee 02_domain.md\nx01_no.md";
        assert_eq!(plan_order(plan), ["02", "01"]);
    }

    #[test]
    fn starts_the_first_ready_task_with_its_parallel_group() {
        let p = project(
            vec![
                task("01", Done, "", &[]),
                task("02", Planned, "B", &["01"]),
                task("03", Planned, "B", &["01"]),
                task("04", Planned, "C", &["01"]),
                task("05", Planned, "B", &["04"]),
            ],
            &["01", "02", "03", "04", "05"],
        );
        let schedule = next(&p, None);
        assert_eq!(numbers(&schedule.start), ["02", "03"]);
        assert_eq!(numbers(&next(&p, Some(1)).start), ["02"]);
    }

    #[test]
    fn follows_plan_order_rather_than_numbers() {
        let p = project(
            vec![task("01", Planned, "A", &[]), task("02", Planned, "B", &[])],
            &["02", "01"],
        );
        assert_eq!(numbers(&next(&p, None).start), ["02"]);
    }

    #[test]
    fn only_joins_the_running_group() {
        let p = project(
            vec![
                task("01", InProgress, "A", &[]),
                task("02", Planned, "A", &[]),
                task("03", Planned, "B", &[]),
            ],
            &[],
        );
        let schedule = next(&p, None);
        assert_eq!(numbers(&schedule.start), ["02"]);
        assert_eq!(numbers(&schedule.running), ["01"]);

        let solo = project(
            vec![
                task("01", InProgress, "", &[]),
                task("02", Planned, "", &[]),
            ],
            &[],
        );
        let schedule = next(&solo, None);
        assert!(schedule.start.is_empty());
        assert!(schedule.reason.unwrap().contains("Task 01 is running"));
    }

    #[test]
    fn ungrouped_tasks_run_alone_and_reasons_explain_idleness() {
        let p = project(
            vec![task("01", Planned, "", &[]), task("02", Planned, "", &[])],
            &[],
        );
        assert_eq!(numbers(&next(&p, None).start), ["01"]);

        let blocked = project(
            vec![
                task("01", Blocked, "", &[]),
                task("02", Planned, "", &["01"]),
            ],
            &[],
        );
        assert!(next(&blocked, None).reason.unwrap().contains("blocked: 01"));
        let done = project(vec![task("01", Done, "", &[])], &[]);
        assert_eq!(
            next(&done, None).reason.as_deref(),
            Some("Every task is Done.")
        );
    }
}
