use std::marker::PhantomData;

use crate::{
    error::Result,
    solver::{
        constraint::{Constraint, ConstraintDescriptor},
        engine::VariableId,
        semantics::DomainSemantics,
        solution::Solution,
        value::ValueArithmetic,
    },
};

/// A constraint that enforces `abs(X - Y) != C`.
///
/// This constraint is specialized for values that support arithmetic. When one
/// variable becomes a singleton, it prunes the values `other ± C` from the
/// domain of the other variable.
#[derive(Debug, Clone)]
pub struct AbsoluteDifferenceNotEqualConstraint<S: DomainSemantics>
where
    S::Value: ValueArithmetic,
{
    vars: [VariableId; 2],
    c: S::Value,
    _phantom: PhantomData<S>,
}

impl<S: DomainSemantics> AbsoluteDifferenceNotEqualConstraint<S>
where
    S::Value: ValueArithmetic,
{
    pub fn new(x: VariableId, y: VariableId, c: S::Value) -> Self {
        Self {
            vars: [x, y],
            c,
            _phantom: PhantomData,
        }
    }
}

impl<S: DomainSemantics + std::fmt::Debug> Constraint<S> for AbsoluteDifferenceNotEqualConstraint<S>
where
    S::Value: ValueArithmetic,
{
    fn variables(&self) -> &[VariableId] {
        &self.vars
    }

    fn descriptor(&self) -> ConstraintDescriptor {
        ConstraintDescriptor {
            name: "AbsDiffNotEqualConstraint".to_string(),
            description: format!("abs(?{} - ?{}) != 1", self.vars[0], self.vars[1]),
        }
    }

    fn revise(
        &self,
        target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>> {
        let other_var = if *target_var == self.vars[0] {
            self.vars[1]
        } else {
            self.vars[0]
        };

        let target_domain = solution.domains.get(target_var).unwrap();
        let other_domain = solution.domains.get(&other_var).unwrap();

        if !other_domain.is_singleton() {
            return Ok(None);
        }

        let other_value = other_domain.get_singleton_value().unwrap();

        let value_to_remove1 = other_value.add(&self.c);
        let value_to_remove2 = other_value.sub(&self.c);

        let original_size = target_domain.len();
        let new_domain =
            target_domain.retain(&|val| *val != value_to_remove1 && *val != value_to_remove2);

        if new_domain.len() < original_size {
            let new_domains = solution.domains.update(*target_var, new_domain);
            let new_solution = solution.clone_with_domains(new_domains);
            Ok(Some(new_solution))
        } else {
            Ok(None)
        }
    }
}
