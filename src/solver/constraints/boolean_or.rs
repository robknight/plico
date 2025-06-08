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
                return Ok(Some(solution.clone_with_domains(new_domains)));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use im::HashMap;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::solver::{
        solution::{DomainRepresentation, HashSetDomain},
        value::StandardValue,
    };

    // --- Test Setup ---

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct TestValue(StandardValue);

    impl From<StandardValue> for TestValue {
        fn from(v: StandardValue) -> Self {
            Self(v)
        }
    }

    #[derive(Debug, Clone)]
    struct TestSemantics;

    impl DomainSemantics for TestSemantics {
        type Value = TestValue;
        type ConstraintDefinition = ();
        type VariableMetadata = ();

        fn build_constraint(
            &self,
            _definition: &Self::ConstraintDefinition,
        ) -> Box<dyn Constraint<Self>> {
            unimplemented!("Not needed for constraint unit tests")
        }
    }

    fn bool_val(b: bool) -> TestValue {
        TestValue(StandardValue::Bool(b))
    }

    fn bool_domain() -> Box<dyn DomainRepresentation<TestValue>> {
        Box::new(HashSetDomain::new(
            [bool_val(true), bool_val(false)].into_iter().collect(),
        ))
    }

    fn domain_from_slice(values: &[bool]) -> Box<dyn DomainRepresentation<TestValue>> {
        Box::new(HashSetDomain::new(
            values.iter().map(|&b| bool_val(b)).collect(),
        ))
    }

    // --- Tests ---

    #[test]
    fn revise_forces_last_variable_to_be_true() {
        let b1: VariableId = 0;
        let b2: VariableId = 1;
        let b3: VariableId = 2;
        let constraint = BooleanOrConstraint::<TestSemantics>::new(vec![b1, b2, b3]);

        let domains = im::hashmap! {
            b1 => domain_from_slice(&[false]),
            b2 => domain_from_slice(&[false]),
            b3 => bool_domain(), // This one can be true or false
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        // The constraint is not revised for a specific target, so we pass any var.
        let new_solution = constraint.revise(&b3, &solution).unwrap().unwrap();
        let new_b3_domain = new_solution.domains.get(&b3).unwrap();

        assert!(new_b3_domain.is_singleton());
        assert_eq!(new_b3_domain.get_singleton_value().unwrap(), bool_val(true));
    }

    #[test]
    fn revise_does_nothing_if_one_var_is_already_true() {
        let b1: VariableId = 0;
        let b2: VariableId = 1;
        let b3: VariableId = 2;
        let constraint = BooleanOrConstraint::<TestSemantics>::new(vec![b1, b2, b3]);

        let domains = im::hashmap! {
            b1 => domain_from_slice(&[false]),
            b2 => domain_from_slice(&[true]), // This one is true
            b3 => bool_domain(),
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let result = constraint.revise(&b3, &solution).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn revise_does_nothing_if_multiple_vars_could_be_true() {
        let b1: VariableId = 0;
        let b2: VariableId = 1;
        let b3: VariableId = 2;
        let constraint = BooleanOrConstraint::<TestSemantics>::new(vec![b1, b2, b3]);

        let domains = im::hashmap! {
            b1 => domain_from_slice(&[false]),
            b2 => bool_domain(), // Could be true
            b3 => bool_domain(), // Could also be true
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let result = constraint.revise(&b1, &solution).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn revise_does_nothing_if_already_consistent() {
        let b1: VariableId = 0;
        let b2: VariableId = 1;
        let b3: VariableId = 2;
        let constraint = BooleanOrConstraint::<TestSemantics>::new(vec![b1, b2, b3]);

        let domains = im::hashmap! {
            b1 => domain_from_slice(&[false]),
            b2 => domain_from_slice(&[false]),
            b3 => domain_from_slice(&[true]), // Already forced to true
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let result = constraint.revise(&b3, &solution).unwrap();
        assert!(result.is_none());
    }
}
