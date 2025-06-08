use std::collections::{BinaryHeap, HashSet};

use crate::solver::{
    constraint::ConstraintPriority,
    engine::{ConstraintId, VariableId},
};

#[derive(Debug, Clone, Eq, PartialEq)]
struct WorkItem {
    priority: ConstraintPriority,
    variable_id: VariableId,
    constraint_id: ConstraintId,
}

impl Ord for WorkItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl PartialOrd for WorkItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct WorkList {
    queue: BinaryHeap<WorkItem>,
    queue_members: HashSet<(VariableId, ConstraintId)>,
}

impl WorkList {
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
            queue_members: HashSet::new(),
        }
    }

    pub fn push_back(
        &mut self,
        priority: ConstraintPriority,
        variable_id: VariableId,
        constraint_id: ConstraintId,
    ) {
        if !self.queue_members.contains(&(variable_id, constraint_id)) {
            self.queue.push(WorkItem {
                priority,
                variable_id,
                constraint_id,
            });
            self.queue_members.insert((variable_id, constraint_id));
        }
    }

    pub fn pop_front(&mut self) -> Option<(VariableId, ConstraintId)> {
        if let Some(item) = self.queue.pop() {
            self.queue_members
                .remove(&(item.variable_id, item.constraint_id));
            Some((item.variable_id, item.constraint_id))
        } else {
            None
        }
    }
}

impl Default for WorkList {
    fn default() -> Self {
        Self::new()
    }
}
