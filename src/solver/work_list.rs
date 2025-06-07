use std::collections::{HashSet, VecDeque};

use crate::solver::engine::{ConstraintId, VariableId};

pub struct WorkList {
    queue: VecDeque<(VariableId, ConstraintId)>,
    queue_members: HashSet<(VariableId, ConstraintId)>,
}

impl WorkList {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            queue_members: HashSet::new(),
        }
    }

    pub fn push_back(&mut self, variable_id: VariableId, constraint_id: ConstraintId) {
        if !self.queue_members.contains(&(variable_id, constraint_id)) {
            self.queue.push_back((variable_id, constraint_id));
            self.queue_members.insert((variable_id, constraint_id));
        }
    }

    pub fn pop_front(&mut self) -> Option<(VariableId, ConstraintId)> {
        let (variable_id, constraint_id) = self.queue.pop_front()?;
        self.queue_members.remove(&(variable_id, constraint_id));
        Some((variable_id, constraint_id))
    }
}

impl Default for WorkList {
    fn default() -> Self {
        Self::new()
    }
}
