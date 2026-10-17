//! Live comment delivery. Agents subscribe to a project, or one task, over
//! SSE and receive each new comment as soon as the board saves it. The board
//! also shows which tasks have an agent listening.

use std::collections::BTreeSet;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::project::comments::Comment;
use crate::watcher::Watcher;

struct Subscriber {
    id: u64,
    project: String,
    /// `None` listens to every task in the project.
    task: Option<String>,
    sender: Sender<Comment>,
}

#[derive(Default)]
struct State {
    next_id: u64,
    subscribers: Vec<Subscriber>,
}

pub struct Hub {
    state: Mutex<State>,
    /// Bumped when listeners change, so open boards refresh their badges.
    watcher: Arc<Watcher>,
}

/// Who is listening in a project.
#[derive(Default)]
pub struct Listeners {
    /// Some agent, usually the lead, listens to every task.
    pub all_tasks: bool,
    /// Tasks with a dedicated listener, usually a subagent.
    pub tasks: BTreeSet<String>,
}

/// Unsubscribes when dropped, so a closed SSE connection stops counting.
pub struct Subscription<'a> {
    hub: &'a Hub,
    id: u64,
    pub receiver: Receiver<Comment>,
}

impl Hub {
    pub fn new(watcher: Arc<Watcher>) -> Self {
        Hub {
            state: Mutex::new(State::default()),
            watcher,
        }
    }

    pub fn subscribe(&self, project: &str, task: Option<&str>) -> Subscription<'_> {
        let (sender, receiver) = mpsc::channel();
        let id = {
            let mut state = self.state.lock().unwrap();
            state.next_id += 1;
            let id = state.next_id;
            state.subscribers.push(Subscriber {
                id,
                project: project.to_string(),
                task: task.map(str::to_string),
                sender,
            });
            id
        };
        self.watcher.bump();
        Subscription {
            hub: self,
            id,
            receiver,
        }
    }

    pub fn publish(&self, project: &str, comment: &Comment) {
        let state = self.state.lock().unwrap();
        for subscriber in &state.subscribers {
            let wants = subscriber.project == project
                && subscriber
                    .task
                    .as_ref()
                    .is_none_or(|task| *task == comment.task);
            if wants {
                let _ = subscriber.sender.send(comment.clone());
            }
        }
    }

    pub fn listeners(&self, project: &str) -> Listeners {
        let state = self.state.lock().unwrap();
        let mut listeners = Listeners::default();
        for subscriber in state.subscribers.iter().filter(|s| s.project == project) {
            match &subscriber.task {
                Some(task) => {
                    listeners.tasks.insert(task.clone());
                }
                None => listeners.all_tasks = true,
            }
        }
        listeners
    }
}

impl Drop for Subscription<'_> {
    fn drop(&mut self) {
        self.hub
            .state
            .lock()
            .unwrap()
            .subscribers
            .retain(|subscriber| subscriber.id != self.id);
        self.hub.watcher.bump();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::comments::{Author, CommentStatus};

    fn comment(task: &str) -> Comment {
        Comment {
            id: "20261017-101530-0001".into(),
            task: task.into(),
            author: Author::User,
            created: "2026-10-17T10:15:30.000Z".into(),
            status: CommentStatus::Open,
            delivered: false,
            reply_to: None,
            body: "hi".into(),
        }
    }

    #[test]
    fn routes_comments_to_matching_subscribers() {
        let watcher = Watcher::new();
        let hub = Hub::new(Arc::clone(&watcher));
        let lead = hub.subscribe("p", None);
        let task_02 = hub.subscribe("p", Some("02"));
        let other = hub.subscribe("q", None);
        assert!(watcher.current() >= 3);

        hub.publish("p", &comment("01"));
        hub.publish("p", &comment("02"));
        assert_eq!(lead.receiver.try_iter().count(), 2);
        assert_eq!(
            task_02
                .receiver
                .try_iter()
                .map(|c| c.task)
                .collect::<Vec<_>>(),
            ["02"]
        );
        assert_eq!(other.receiver.try_iter().count(), 0);

        let listeners = hub.listeners("p");
        assert!(listeners.all_tasks);
        assert_eq!(listeners.tasks.iter().collect::<Vec<_>>(), ["02"]);
        drop(task_02);
        assert!(hub.listeners("p").tasks.is_empty());
    }
}
