use im::HashSet;

use crate::{
    error::Result,
    solver::{
        constraint::{Constraint, ConstraintDescriptor},
        engine::VariableId,
        semantics::DomainSemantics,
        solution::Solution,
    },
};

/// A constraint that ensures all variables in a given set have unique values.
///
/// This is a common constraint in problems like Sudoku, where every cell in a
/// row, column, or box must contain a different number.
#[derive(Debug, Clone)]
pub struct AllDifferentConstraint<S: DomainSemantics + std::fmt::Debug> {
    pub vars: Vec<VariableId>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: DomainSemantics + std::fmt::Debug> AllDifferentConstraint<S> {
    pub fn new(vars: Vec<VariableId>) -> Self {
        Self {
            vars,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: DomainSemantics + std::fmt::Debug> Constraint<S> for AllDifferentConstraint<S> {
    fn variables(&self) -> &[VariableId] {
        &self.vars
    }

    fn descriptor(&self) -> ConstraintDescriptor {
        let vars_str = self
            .vars
            .iter()
            .map(|v| format!("?{}", v))
            .collect::<Vec<_>>()
            .join(", ");
        ConstraintDescriptor {
            name: "AllDifferentConstraint".to_string(),
            description: format!("AllDifferent({})", vars_str),
        }
    }

    fn revise(
        &self,
        target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>> {
        // Find all values that are already fixed in other variables in this group.
        let mut fixed_values_to_remove = HashSet::new();
        for var in &self.vars {
            if *var != *target_var {
                if let Some(domain) = solution.domains.get(var) {
                    if domain.is_singleton() {
                        if let Some(fixed_value) = domain.get_singleton_value() {
                            fixed_values_to_remove.insert(fixed_value.clone());
                        }
                    }
                }
            }
        }

        if fixed_values_to_remove.is_empty() {
            return Ok(None);
        }

        // Now, remove those fixed values from the target's domain.
        if let Some(target_domain) = solution.domains.get(target_var) {
            let original_size = target_domain.len();
            let new_domain = target_domain.retain(&|val| !fixed_values_to_remove.contains(val));
            let changed = new_domain.len() < original_size;
            if changed {
                let new_domains = solution.domains.update(*target_var, new_domain);
                let new_solution = Solution {
                    domains: new_domains,
                    semantics: solution.semantics.clone(),
                };
                return Ok(Some(new_solution));
            }
        }

        Ok(None)
    }
}
