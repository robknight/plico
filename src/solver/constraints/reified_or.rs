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

/// Enforces `B_out <==> (B_in_1 OR B_in_2 OR ...)`
#[derive(Debug, Clone)]
pub struct ReifiedOrConstraint<S: DomainSemantics + std::fmt::Debug>
where
    S::Value: From<StandardValue> + PartialEq,
{
    b_out: VariableId,
    b_in: Vec<VariableId>,
    all_vars: Vec<VariableId>,
    _phantom: PhantomData<S>,
}

impl<S: DomainSemantics + std::fmt::Debug> ReifiedOrConstraint<S>
where
    S::Value: From<StandardValue> + PartialEq,
{
    pub fn new(b_out: VariableId, b_in: Vec<VariableId>) -> Self {
        let mut all_vars = b_in.clone();
        all_vars.push(b_out);
        Self {
            b_out,
            b_in,
            all_vars,
            _phantom: PhantomData,
        }
    }
}

impl<S: DomainSemantics + std::fmt::Debug> Constraint<S> for ReifiedOrConstraint<S>
where
    S::Value: From<StandardValue> + PartialEq,
{
    fn variables(&self) -> &[VariableId] {
        &self.all_vars
    }

    fn descriptor(&self) -> ConstraintDescriptor {
        let terms_str = self
            .b_in
            .iter()
            .map(|v| format!("?{}", v))
            .collect::<Vec<_>>()
            .join(" OR ");
        ConstraintDescriptor {
            name: "ReifiedOrConstraint".to_string(),
            description: format!("?{} <==> ({})", self.b_out, terms_str),
        }
    }

    fn revise(
        &self,
        _target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>> {
        let mut new_domains = solution.domains.clone();
        let mut changed = false;

        let true_val = S::Value::from(StandardValue::Bool(true));
        let false_val = S::Value::from(StandardValue::Bool(false));

        let b_out_domain = solution.domains.get(&self.b_out).unwrap();

        // Propagation from inputs to output
        let mut any_input_is_true = false;
        let mut all_inputs_are_false = true;
        for b_in_var in &self.b_in {
            let b_in_domain = solution.domains.get(b_in_var).unwrap();
            if b_in_domain.is_singleton() {
                if b_in_domain.get_singleton_value().unwrap() == true_val {
                    any_input_is_true = true;
                    all_inputs_are_false = false;
                    break;
                }
            } else {
                all_inputs_are_false = false;
            }
        }

        if any_input_is_true {
            // If any input is true, output must be true
            let new_b_out_domain = b_out_domain.retain(&|v| *v == true_val);
            if new_b_out_domain.len() < b_out_domain.len() {
                new_domains.insert(self.b_out, new_b_out_domain);
                changed = true;
            }
        } else if all_inputs_are_false {
            // If all inputs are false, output must be false
            let new_b_out_domain = b_out_domain.retain(&|v| *v == false_val);
            if new_b_out_domain.len() < b_out_domain.len() {
                new_domains.insert(self.b_out, new_b_out_domain);
                changed = true;
            }
        }

        // Propagation from output to inputs
        if b_out_domain.is_singleton() {
            let b_out_val = b_out_domain.get_singleton_value().unwrap();
            if b_out_val == false_val {
                // If output is false, all inputs must be false
                for b_in_var in &self.b_in {
                    let b_in_domain = new_domains.get(b_in_var).unwrap();
                    let new_b_in_domain = b_in_domain.retain(&|v| *v == false_val);
                    if new_b_in_domain.len() < b_in_domain.len() {
                        new_domains.insert(*b_in_var, new_b_in_domain);
                        changed = true;
                    }
                }
            }
        }

        if changed {
            Ok(Some(solution.clone_with_domains(new_domains)))
        } else {
            Ok(None)
        }
    }
}
