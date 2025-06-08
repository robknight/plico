//! A constraint that reifies an equality relationship.
//!
//! This constraint links a boolean variable `B` to the outcome of an equality
//! check between two other variables, `X` and `Y`. The core relationship is
//! `B <==> (X == Y)`.

use std::marker::PhantomData;

use crate::{
    error::Result,
    solver::{
        constraint::{Constraint, ConstraintDescriptor},
        engine::VariableId,
        semantics::DomainSemantics,
        solution::Solution,
        value::StandardValue,
    },
};

/// Enforces `B <==> (X == Y)`.
#[derive(Debug, Clone)]
pub struct ReifiedEqualConstraint<S: DomainSemantics + std::fmt::Debug>
where
    S::Value: From<StandardValue>,
{
    vars: [VariableId; 3],
    _phantom: PhantomData<S>,
}

impl<S: DomainSemantics + std::fmt::Debug> ReifiedEqualConstraint<S>
where
    S::Value: From<StandardValue>,
{
    pub fn new(b: VariableId, x: VariableId, y: VariableId) -> Self {
        Self {
            vars: [b, x, y],
            _phantom: PhantomData,
        }
    }
}

impl<S: DomainSemantics + std::fmt::Debug> Constraint<S> for ReifiedEqualConstraint<S>
where
    S::Value: From<StandardValue>,
{
    fn variables(&self) -> &[VariableId] {
        &self.vars
    }

    fn descriptor(&self) -> ConstraintDescriptor {
        ConstraintDescriptor {
            name: "ReifiedEqualConstraint".to_string(),
            description: format!(
                "?{} <==> (?{} == ?{})",
                self.vars[0], self.vars[1], self.vars[2]
            ),
        }
    }

    fn revise(
        &self,
        _target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>> {
        let b_var = self.vars[0];
        let x_var = self.vars[1];
        let y_var = self.vars[2];

        let b_domain = solution.domains.get(&b_var).unwrap();
        let x_domain = solution.domains.get(&x_var).unwrap();
        let y_domain = solution.domains.get(&y_var).unwrap();

        let mut new_domains = solution.domains.clone();
        let mut changed = false;

        // B -> (X, Y) propagation
        if b_domain.is_singleton() {
            let b_val = b_domain.get_singleton_value().unwrap();
            if b_val == S::Value::from(StandardValue::Bool(true)) {
                // Enforce X == Y
                let intersection = x_domain.intersect(y_domain.as_ref());
                if intersection.len() < x_domain.len() {
                    new_domains.insert(x_var, intersection.clone_box());
                    changed = true;
                }
                if intersection.len() < y_domain.len() {
                    new_domains.insert(y_var, intersection.clone_box());
                    changed = true;
                }
            } else {
                // Enforce X != Y
                if x_domain.is_singleton() && y_domain.len() > 1 {
                    let x_val = x_domain.get_singleton_value().unwrap();
                    let new_y_domain = y_domain.retain(&|v| v != &x_val);
                    if new_y_domain.len() < y_domain.len() {
                        new_domains.insert(y_var, new_y_domain);
                        changed = true;
                    }
                }
                if y_domain.is_singleton() && x_domain.len() > 1 {
                    let y_val = y_domain.get_singleton_value().unwrap();
                    let new_x_domain = x_domain.retain(&|v| v != &y_val);
                    if new_x_domain.len() < x_domain.len() {
                        new_domains.insert(x_var, new_x_domain);
                        changed = true;
                    }
                }
            }
        }

        // (X, Y) -> B propagation
        let x_is_singleton = x_domain.is_singleton();
        let y_is_singleton = y_domain.is_singleton();

        // If domains are disjoint, B must be false
        if x_domain.intersect(y_domain.as_ref()).is_empty() {
            let b_must_be_false =
                b_domain.retain(&|v| v == &S::Value::from(StandardValue::Bool(false)));
            if b_must_be_false.len() < b_domain.len() {
                new_domains.insert(b_var, b_must_be_false);
                changed = true;
            }
        }

        // If X and Y are both singletons and equal, B must be true
        if x_is_singleton
            && y_is_singleton
            && x_domain.get_singleton_value() == y_domain.get_singleton_value()
        {
            let b_must_be_true =
                b_domain.retain(&|v| v == &S::Value::from(StandardValue::Bool(true)));
            if b_must_be_true.len() < b_domain.len() {
                new_domains.insert(b_var, b_must_be_true);
                changed = true;
            }
        }

        if changed {
            Ok(Some(solution.clone_with_domains(new_domains)))
        } else {
            Ok(None)
        }
    }
}
