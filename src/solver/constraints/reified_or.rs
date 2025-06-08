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
    fn revise_propagates_from_input_to_output_true() {
        let b_out = 0;
        let b_in1 = 1;
        let b_in2 = 2;
        let constraint = ReifiedOrConstraint::<TestSemantics>::new(b_out, vec![b_in1, b_in2]);

        let domains = im::hashmap! {
            b_out => bool_domain(),
            b_in1 => domain_from_slice(&[true]),
            b_in2 => domain_from_slice(&[false]),
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));
        let new_solution = constraint.revise(&b_out, &solution).unwrap().unwrap();

        let new_b_out_domain = new_solution.domains.get(&b_out).unwrap();
        assert!(new_b_out_domain.is_singleton());
        assert_eq!(
            new_b_out_domain.get_singleton_value().unwrap(),
            bool_val(true)
        );
    }

    #[test]
    fn revise_propagates_from_input_to_output_false() {
        let b_out = 0;
        let b_in1 = 1;
        let b_in2 = 2;
        let constraint = ReifiedOrConstraint::<TestSemantics>::new(b_out, vec![b_in1, b_in2]);

        let domains = im::hashmap! {
            b_out => bool_domain(),
            b_in1 => domain_from_slice(&[false]),
            b_in2 => domain_from_slice(&[false]),
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));
        let new_solution = constraint.revise(&b_out, &solution).unwrap().unwrap();

        let new_b_out_domain = new_solution.domains.get(&b_out).unwrap();
        assert!(new_b_out_domain.is_singleton());
        assert_eq!(
            new_b_out_domain.get_singleton_value().unwrap(),
            bool_val(false)
        );
    }

    #[test]
    fn revise_propagates_from_output_to_inputs_false() {
        let b_out = 0;
        let b_in1 = 1;
        let b_in2 = 2;
        let constraint = ReifiedOrConstraint::<TestSemantics>::new(b_out, vec![b_in1, b_in2]);

        let domains = im::hashmap! {
            b_out => domain_from_slice(&[false]),
            b_in1 => bool_domain(),
            b_in2 => bool_domain(),
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));
        let new_solution = constraint.revise(&b_in1, &solution).unwrap().unwrap();

        let new_b_in1_domain = new_solution.domains.get(&b_in1).unwrap();
        assert!(new_b_in1_domain.is_singleton());
        assert_eq!(
            new_b_in1_domain.get_singleton_value().unwrap(),
            bool_val(false)
        );

        let new_b_in2_domain = new_solution.domains.get(&b_in2).unwrap();
        assert!(new_b_in2_domain.is_singleton());
        assert_eq!(
            new_b_in2_domain.get_singleton_value().unwrap(),
            bool_val(false)
        );
    }

    #[test]
    fn revise_does_nothing_if_insufficient_info() {
        let b_out = 0;
        let b_in1 = 1;
        let b_in2 = 2;
        let constraint = ReifiedOrConstraint::<TestSemantics>::new(b_out, vec![b_in1, b_in2]);

        // Case 1: Output is true, inputs are ambiguous
        let domains1 = im::hashmap! {
            b_out => domain_from_slice(&[true]),
            b_in1 => bool_domain(),
            b_in2 => bool_domain(),
        };
        let solution1 = Solution::new(domains1, HashMap::new(), Arc::new(TestSemantics));
        assert!(constraint.revise(&b_in1, &solution1).unwrap().is_none());

        // Case 2: Output is ambiguous, one input is false, one is ambiguous
        let domains2 = im::hashmap! {
            b_out => bool_domain(),
            b_in1 => domain_from_slice(&[false]),
            b_in2 => bool_domain(),
        };
        let solution2 = Solution::new(domains2, HashMap::new(), Arc::new(TestSemantics));
        assert!(constraint.revise(&b_out, &solution2).unwrap().is_none());
    }
}
