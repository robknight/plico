use crate::{
    error::Result,
    solver::{engine::VariableId, semantics::DomainSemantics, solution::Solution},
};

pub trait Constraint<S: DomainSemantics>: std::fmt::Debug {
    fn variables(&self) -> &[VariableId];

    fn revise(
        &self,
        target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>>;
    // ...
}
