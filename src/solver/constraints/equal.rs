use crate::{
    error::Result,
    solver::{
        constraint::Constraint, engine::VariableId, semantics::DomainSemantics,
        solution::CandidateSolution,
    },
};

#[derive(Debug, Clone)]
pub struct EqualConstraint<S: DomainSemantics + std::fmt::Debug> {
    vars: [VariableId; 2],
    _phantom: std::marker::PhantomData<S>,
}

impl<S: DomainSemantics + std::fmt::Debug> EqualConstraint<S> {
    pub fn new(a: VariableId, b: VariableId) -> Self {
        Self {
            vars: [a, b],
            _phantom: std::marker::PhantomData,
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
        solution: &CandidateSolution<S>,
    ) -> Result<Option<CandidateSolution<S>>> {
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
