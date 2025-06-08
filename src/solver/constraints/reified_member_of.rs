use std::{collections::HashSet, marker::PhantomData};

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

/// Enforces `B <==> (V1, V2, ...) is in DataSet`
#[derive(Debug, Clone)]
pub struct ReifiedMemberOfConstraint<S: DomainSemantics + std::fmt::Debug>
where
    S::Value: From<StandardValue> + PartialEq,
{
    b: VariableId,
    vars: Vec<VariableId>,
    data_set: HashSet<Vec<S::Value>>,
    all_vars: Vec<VariableId>,
    _phantom: PhantomData<S>,
}

impl<S: DomainSemantics + std::fmt::Debug> ReifiedMemberOfConstraint<S>
where
    S::Value: From<StandardValue> + PartialEq,
{
    pub fn new(b: VariableId, vars: Vec<VariableId>, data_set: HashSet<Vec<S::Value>>) -> Self {
        let mut all_vars = vars.clone();
        all_vars.push(b);
        Self {
            b,
            vars,
            data_set,
            all_vars,
            _phantom: PhantomData,
        }
    }
}

impl<S: DomainSemantics + std::fmt::Debug> Constraint<S> for ReifiedMemberOfConstraint<S>
where
    S::Value: From<StandardValue> + PartialEq,
{
    fn variables(&self) -> &[VariableId] {
        &self.all_vars
    }

    fn descriptor(&self) -> ConstraintDescriptor {
        let terms_str = self
            .vars
            .iter()
            .map(|v| format!("?{}", v))
            .collect::<Vec<_>>()
            .join(", ");
        ConstraintDescriptor {
            name: "ReifiedMemberOfConstraint".to_string(),
            description: format!(
                "?{} <==> IsMember(({}), [{} items])",
                self.b,
                terms_str,
                self.data_set.len()
            ),
        }
    }

    fn revise(
        &self,
        _target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>> {
        let mut new_domains = solution.domains.clone();
        let mut changed = false;

        let b_domain = solution.domains.get(&self.b).unwrap();
        let true_val = S::Value::from(StandardValue::Bool(true));
        let false_val = S::Value::from(StandardValue::Bool(false));

        // B -> Vars propagation
        if b_domain.is_singleton() && b_domain.get_singleton_value().unwrap() == true_val {
            for (i, var_id) in self.vars.iter().enumerate() {
                let var_domain = new_domains.get(var_id).unwrap();
                let original_size = var_domain.len();
                let possible_values_for_var: HashSet<_> =
                    self.data_set.iter().map(|row| row[i].clone()).collect();
                let new_domain = var_domain.retain(&|val| possible_values_for_var.contains(val));
                if new_domain.len() < original_size {
                    new_domains.insert(*var_id, new_domain);
                    changed = true;
                }
            }
        }

        // Vars -> B propagation
        let domains: Vec<_> = self
            .vars
            .iter()
            .map(|v| solution.domains.get(v).unwrap())
            .collect();

        let possible = self.data_set.iter().any(|row| {
            row.iter()
                .zip(domains.iter())
                .all(|(val, domain)| domain.contains(val))
        });

        if !possible {
            let new_b_domain = b_domain.retain(&|v| *v == false_val);
            if new_b_domain.len() < b_domain.len() {
                new_domains.insert(self.b, new_b_domain);
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

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::Arc};

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

    fn int_val(i: i64) -> TestValue {
        TestValue(StandardValue::Int(i))
    }

    fn bool_val(b: bool) -> TestValue {
        TestValue(StandardValue::Bool(b))
    }

    fn bool_domain() -> Box<dyn DomainRepresentation<TestValue>> {
        Box::new(HashSetDomain::new(
            [bool_val(true), bool_val(false)].into_iter().collect(),
        ))
    }

    fn domain_from_bools(values: &[bool]) -> Box<dyn DomainRepresentation<TestValue>> {
        Box::new(HashSetDomain::new(
            values.iter().map(|&b| bool_val(b)).collect(),
        ))
    }

    fn domain_from_ints(values: &[i64]) -> Box<dyn DomainRepresentation<TestValue>> {
        Box::new(HashSetDomain::new(
            values.iter().map(|&i| int_val(i)).collect(),
        ))
    }

    // --- Tests ---

    #[test]
    fn revise_propagates_from_b_to_vars() {
        let b = 0;
        let v1 = 1;
        let v2 = 2;

        let data_set = HashSet::from([
            vec![int_val(1), int_val(10)],
            vec![int_val(2), int_val(20)],
            vec![int_val(3), int_val(10)],
        ]);

        let constraint = ReifiedMemberOfConstraint::<TestSemantics>::new(b, vec![v1, v2], data_set);

        let domains = im::hashmap! {
            b => domain_from_bools(&[true]),
            v1 => domain_from_ints(&[1, 2, 3, 4, 5]),
            v2 => domain_from_ints(&[10, 20, 30, 40]),
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));
        let new_solution = constraint.revise(&v1, &solution).unwrap().unwrap();

        let new_v1_domain = new_solution.domains.get(&v1).unwrap();
        let expected_v1: im::HashSet<TestValue> =
            [int_val(1), int_val(2), int_val(3)].into_iter().collect();
        assert_eq!(
            new_v1_domain.iter().cloned().collect::<im::HashSet<_>>(),
            expected_v1
        );

        let new_v2_domain = new_solution.domains.get(&v2).unwrap();
        let expected_v2: im::HashSet<TestValue> = [int_val(10), int_val(20)].into_iter().collect();
        assert_eq!(
            new_v2_domain.iter().cloned().collect::<im::HashSet<_>>(),
            expected_v2
        );
    }

    #[test]
    fn revise_propagates_from_vars_to_b() {
        let b = 0;
        let v1 = 1;
        let v2 = 2;

        let data_set =
            HashSet::from([vec![int_val(1), int_val(10)], vec![int_val(2), int_val(20)]]);

        let constraint = ReifiedMemberOfConstraint::<TestSemantics>::new(b, vec![v1, v2], data_set);

        let domains = im::hashmap! {
            b => bool_domain(),
            v1 => domain_from_ints(&[3, 4]), // No possible match
            v2 => domain_from_ints(&[10, 20]),
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));
        let new_solution = constraint.revise(&b, &solution).unwrap().unwrap();

        let new_b_domain = new_solution.domains.get(&b).unwrap();
        assert!(new_b_domain.is_singleton());
        assert_eq!(new_b_domain.get_singleton_value().unwrap(), bool_val(false));
    }

    #[test]
    fn revise_does_nothing_if_no_new_info() {
        let b = 0;
        let v1 = 1;
        let v2 = 2;

        let data_set =
            HashSet::from([vec![int_val(1), int_val(10)], vec![int_val(2), int_val(20)]]);

        let constraint = ReifiedMemberOfConstraint::<TestSemantics>::new(b, vec![v1, v2], data_set);

        // Case 1: Vars are consistent, B is not yet determined
        let domains1 = im::hashmap! {
            b => bool_domain(),
            v1 => domain_from_ints(&[1, 3]), // 1 is possible
            v2 => domain_from_ints(&[10, 30]), // 10 is possible
        };
        let solution1 = Solution::new(domains1, HashMap::new(), Arc::new(TestSemantics));
        assert!(constraint.revise(&b, &solution1).unwrap().is_none());

        // Case 2: B is true, but vars are already consistent
        let domains2 = im::hashmap! {
            b => domain_from_bools(&[true]),
            v1 => domain_from_ints(&[1, 2]),
            v2 => domain_from_ints(&[10, 20]),
        };
        let solution2 = Solution::new(domains2, HashMap::new(), Arc::new(TestSemantics));
        assert!(constraint.revise(&v1, &solution2).unwrap().is_none());
    }
}
