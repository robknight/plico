//! A constraint that enforces a logical OR relationship between a set of boolean variables.
//!
//! This constraint takes a list of boolean variables `[B1, B2, ..., Bn]` and
//! enforces that `B1 OR B2 OR ... OR Bn` is true.

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

/// Enforces `B1 OR B2 OR ... OR Bn`.
#[derive(Debug, Clone)]
pub struct BooleanOrConstraint<S: DomainSemantics + std::fmt::Debug>
where
    S::Value: From<StandardValue>,
{
    vars: Vec<VariableId>,
    _phantom: PhantomData<S>,
}

impl<S: DomainSemantics + std::fmt::Debug> BooleanOrConstraint<S>
where
    S::Value: From<StandardValue>,
{
    pub fn new(vars: Vec<VariableId>) -> Self {
        Self {
            vars,
            _phantom: PhantomData,
        }
    }
}

impl<S: DomainSemantics + std::fmt::Debug> Constraint<S> for BooleanOrConstraint<S>
where
    S::Value: From<StandardValue>,
{
    fn variables(&self) -> &[VariableId] {
        &self.vars
    }

    fn descriptor(&self) -> ConstraintDescriptor {
        let vars_str = self
            .vars
            .iter()
            .map(|v| format!("?{}", v))
            .collect::<Vec<_>>()
            .join(" OR ");
        ConstraintDescriptor {
            name: "BooleanOrConstraint".to_string(),
            description: vars_str,
        }
    }

    fn revise(
        &self,
        _target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>> {
        let _false_val = S::Value::from(StandardValue::Bool(false));
        let true_val = S::Value::from(StandardValue::Bool(true));

        let mut possibly_true_vars = vec![];
        let mut known_false_count = 0;

        for var_id in &self.vars {
            let domain = solution.domains.get(var_id).unwrap();
            if domain.is_singleton() {
                if domain.get_singleton_value().unwrap() == true_val {
                    // One of the variables is already true, so the OR is satisfied.
                    return Ok(None);
                } else {
                    // This variable is definitely false.
                    known_false_count += 1;
                }
            } else {
                // This variable could still be true.
                possibly_true_vars.push(*var_id);
            }
        }

        if possibly_true_vars.len() == 1 && known_false_count == self.vars.len() - 1 {
            // All other variables are false, so this one must be true.
            let last_hope_var = possibly_true_vars[0];
            let domain = solution.domains.get(&last_hope_var).unwrap();
            let new_domain = domain.retain(&|v| v == &true_val);

            if new_domain.len() < domain.len() {
                let mut new_domains = solution.domains.clone();
                new_domains.insert(last_hope_var, new_domain);
                return Ok(Some(Solution {
                    domains: new_domains,
                    semantics: solution.semantics.clone(),
                }));
            }
        }

        Ok(None)
    }
}
