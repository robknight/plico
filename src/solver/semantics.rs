use crate::solver::{constraint::Constraint, value::ValueEquality};

/// A trait that defines the "frontend" for a specific problem domain.
///
/// This is the primary interface for connecting a concrete problem (like Sudoku or
/// map colouring) to the generic solver engine. By implementing this trait, you
/// provide the solver with all the necessary information about your problem's
/// specific types and rules.
pub trait DomainSemantics: 'static + Clone {
    /// The concrete type for a value in a variable's domain.
    ///
    /// For Sudoku, this might be a number from 1 to 9. For map colouring, this
    /// could be an enum of colours like `Red`, `Green`, `Blue`.
    type Value: ValueEquality;

    /// An enum or struct used to tag variables with semantic information.
    /// This allows heuristics to apply different strategies to different types of variables.
    type VariableMetadata: Clone + std::fmt::Debug + Eq + std::hash::Hash + 'static;

    /// A structure that defines a single constraint in the problem domain.
    ///
    /// This is typically an enum where each variant represents a different kind
    /// of constraint (e.g., `AllDifferent`, `NotEqual`).
    type ConstraintDefinition: std::fmt::Debug;

    /// A factory method that constructs a runnable [`Constraint`] object from its
    /// definition.
    ///
    /// The solver will call this method to turn the declarative constraint
    /// definitions into executable logic.
    fn build_constraint(
        &self,
        definition: &Self::ConstraintDefinition,
    ) -> Box<dyn Constraint<Self>>;
}
