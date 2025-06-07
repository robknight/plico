use crate::{
    error::Result,
    solver::{engine::VariableId, semantics::DomainSemantics, solution::CandidateSolution},
};

pub trait Constraint<S: DomainSemantics>: std::fmt::Debug {
    fn variables(&self) -> &[VariableId];

    fn revise(
        &self,
        target_var: &VariableId,
        solution: &CandidateSolution<S>,
    ) -> Result<Option<CandidateSolution<S>>>;
    // ...
}
