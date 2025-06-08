use crate::{
    error::Result,
    solver::{engine::VariableId, semantics::DomainSemantics, solution::Solution},
};

#[derive(Debug, Clone)]
pub struct ConstraintDescriptor {
    pub name: String,
    pub description: String,
}

pub trait Constraint<S: DomainSemantics>: std::fmt::Debug {
    fn variables(&self) -> &[VariableId];

    fn descriptor(&self) -> ConstraintDescriptor;

    fn revise(
        &self,
        target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>>;
    // ...
}
