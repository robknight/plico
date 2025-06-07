use std::marker::PhantomData;

use crate::{
    error::Result,
    solver::{
        constraint::Constraint, engine::VariableId, semantics::DomainSemantics,
        solution::Solution,
    },
};

/// A constraint that enforces equality between two variables (`A == B`).
///
/// When this constraint is revised, it ensures that the domain of the target
/// variable is pruned to the intersection of its own domain and the other
/// variable's domain.
#[derive(Debug, Clone)]
pub struct EqualConstraint<S: DomainSemantics + std::fmt::Debug> {
    vars: [VariableId; 2],
    _phantom: std::marker::PhantomData<S>,
}

impl<S: DomainSemantics + std::fmt::Debug> EqualConstraint<S> {
    pub fn new(a: VariableId, b: VariableId) -> Self {
        Self {
            vars: [a, b],
            _phantom: PhantomData,
        }
    }
}

impl<S: DomainSemantics + std::fmt::Debug> Constraint<S> for EqualConstraint<S> {
    fn variables(&self) -> &[VariableId] {
        &self.vars
    }

    fn revise(
        &self,
        target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>> {
        let other_var = if *target_var == self.vars[0] {
            self.vars[1]
        } else {
            self.vars[0]
        };

        let target_domain = solution.domains.get(target_var).unwrap();
        let other_domain = solution.domains.get(&other_var).unwrap();

        let original_size = target_domain.len();
        let new_domain = target_domain.intersect(other_domain.as_ref());
        let changed = new_domain.len() < original_size;

        if changed {
            let new_domains = solution.domains.update(*target_var, new_domain);
            let new_solution = Solution {
                domains: new_domains,
                semantics: solution.semantics.clone(),
            };
            Ok(Some(new_solution))
        } else {
            Ok(None)
        }
    }
}
