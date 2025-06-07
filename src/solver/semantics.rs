use crate::{solver::constraint::Constraint, solver::value::ValueEquality};

pub trait DomainSemantics: 'static {
    /// The concrete type that will be used for values in domains.
    /// This type MUST implement our ValueEquality trait.
    type Value: ValueEquality;

    /// An enum or struct that describes a constraint in this domain.
    /// For POD2, this might be an enum with variants like `SumOf {..}` or `Equal {..}`.
    type ConstraintDefinition: std::fmt::Debug;

    /// A factory method that turns a definition into a runnable constraint object.
    fn build_constraint(
        &self,
        definition: &Self::ConstraintDefinition,
    ) -> Box<dyn Constraint<Self>>;

    /// Defines the integer IDs for value "kinds".
    const NUMERIC_KIND_ID: u8;
    const POD_ID_KIND_ID: u8;
    const KEY_KIND_ID: u8;
}
