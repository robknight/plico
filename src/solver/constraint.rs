use crate::{
    error::Result,
    solver::{engine::VariableId, semantics::DomainSemantics, solution::Solution},
};

/// Represents the priority of a constraint, used by the worklist to
/// determine the order of propagation. Higher numbers indicate higher priority.
pub type ConstraintPriority = u32;

/// A set of standard priority levels to guide implementation. Custom
/// numerical values can also be used for finer-grained control.
pub mod priority {
    use super::ConstraintPriority;

    /// The lowest priority, for constraints that are very cheap to run
    /// but have minimal pruning power.
    pub const LOW: ConstraintPriority = 10;
    /// The default priority for most simple constraints.
    pub const NORMAL: ConstraintPriority = 50;
    /// For constraints that are more effective at pruning than average,
    /// such as `AllDifferent`.
    pub const HIGH: ConstraintPriority = 100;
    /// The highest priority, for powerful global constraints that can
    /// drastically reduce the search space.
    pub const VERY_HIGH: ConstraintPriority = 200;
}

#[derive(Debug, Clone)]
pub struct ConstraintDescriptor {
    pub name: String,
    pub description: String,
}

pub trait Constraint<S: DomainSemantics>: std::fmt::Debug {
    fn variables(&self) -> &[VariableId];

    fn descriptor(&self) -> ConstraintDescriptor;

    /// Returns the priority of the constraint.
    ///
    /// This method allows constraints to specify their propagation order.
    /// Higher priority constraints will be processed first.
    ///
    /// The default implementation returns `priority::NORMAL`.
    ///
    /// This will typically be a static value, based on the constraint's
    /// complexity and pruning power. However, it could also be instance-
    /// specific for greater flexibility. Dynamic priorities are not fully-
    /// supported, in that a change in priority will only take effect at the
    /// next time the constraint is added to the worklist.
    fn priority(&self) -> ConstraintPriority {
        priority::NORMAL
    }

    fn revise(
        &self,
        target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>>;
    // ...
}
