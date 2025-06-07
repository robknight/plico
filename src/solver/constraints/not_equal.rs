use crate::{
    error::Result,
    solver::{
        constraint::Constraint, engine::VariableId, semantics::DomainSemantics,
        solution::CandidateSolution,
    },
};

#[derive(Debug, Clone)]
pub struct NotEqualConstraint<S: DomainSemantics + std::fmt::Debug> {
    pub vars: [VariableId; 2],
    _phantom: std::marker::PhantomData<S>,
}

impl<S: DomainSemantics + std::fmt::Debug> NotEqualConstraint<S> {
    pub fn new(a: VariableId, b: VariableId) -> Self {
        Self {
            vars: [a, b],
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: DomainSemantics + std::fmt::Debug> Constraint<S> for NotEqualConstraint<S> {
    fn variables(&self) -> &[VariableId] {
        &self.vars
    }

    fn revise(
        &self,
        target_var: &VariableId,
        solution: &CandidateSolution<S>,
    ) -> Result<Option<CandidateSolution<S>>> {
        let other_var = if *target_var == self.vars[0] {
            self.vars[1]
        } else {
            self.vars[0]
        };

        let other_domain = solution.domains.get(&other_var).unwrap();

        // If the other variable's domain is not a singleton, we can't prune.
        if !other_domain.is_singleton() {
            return Ok(None);
        }

        // The value to remove is the single value in the other domain.
        let value_to_remove = other_domain.get_singleton_value().unwrap();

        let target_domain = solution.domains.get(target_var).unwrap();
        let original_size = target_domain.len();

        let new_domain = target_domain.retain(&|val| *val != value_to_remove);

        if new_domain.len() < original_size {
            let new_domains = solution.domains.update(*target_var, new_domain);
            let new_solution = CandidateSolution {
                domains: new_domains,
                semantics: solution.semantics.clone(),
            };
            Ok(Some(new_solution))
        } else {
            Ok(None)
        }
    }
}
