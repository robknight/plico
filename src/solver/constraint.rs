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
    /// The general name of the constraint type (e.g., "AllDifferent").
    pub name: String,
    /// A specific description of this constraint instance (e.g., "?A != ?B").
    pub description: String,
}

/// The fundamental trait for all constraints in the solver.
///
/// A `Constraint` implements the logic for propagating information and pruning
/// variable domains. It is the core building block for defining the rules of a
/// constraint satisfaction problem.
pub trait Constraint<S: DomainSemantics>: std::fmt::Debug {
    /// Returns a slice of the [`VariableId`]s that this constraint operates on.
    ///
    /// This information is used by the solver engine to build a dependency graph,
    /// so it knows which constraints to re-evaluate when a variable's domain changes.
    fn variables(&self) -> &[VariableId];

    /// Returns a descriptor containing metadata about the constraint instance.
    ///
    /// This is used for logging, debugging, and rendering statistics. The
    /// descriptor should provide a clear, human-readable summary of what the
    /// constraint enforces.
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

    /// The core logic of the constraint.
    ///
    /// This method is called by the solver's propagation engine (e.g., AC-3).
    /// Its goal is to prune the domain of the `target_var` by removing values
    /// that are inconsistent with the constraint, given the current domains of
    /// the other variables involved.
    ///
    /// # Arguments
    ///
    /// * `target_var`: The ID of the variable whose domain is to be revised.
    /// * `solution`: The current state of the solver, containing the domains of all variables.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(new_solution))` if the domain of `target_var` was pruned. The
    ///   `new_solution` reflects this change.
    /// * `Ok(None)` if no changes were made to the domain of `target_var`.
    /// * `Err(error)` if an unrecoverable error occurs.
    fn revise(
        &self,
        target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>>;
    // ...
}
