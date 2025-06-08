//! A constraint that enforces a sum relationship: `Σ(terms) = sum`.

use std::marker::PhantomData;

use crate::{
    error::Result,
    solver::{
        constraint::{Constraint, ConstraintDescriptor},
        engine::VariableId,
        semantics::DomainSemantics,
        solution::Solution,
        value::{ValueArithmetic, ValueOrdering},
    },
};

/// Enforces `terms[0] + terms[1] + ... == sum`.
#[derive(Debug, Clone)]
pub struct SumOfConstraint<S: DomainSemantics>
where
    S::Value: ValueArithmetic + ValueOrdering,
{
    terms: Vec<VariableId>,
    sum: VariableId,
    all_vars: Vec<VariableId>,
    _phantom: PhantomData<S>,
}

impl<S: DomainSemantics> SumOfConstraint<S>
where
    S::Value: ValueArithmetic + ValueOrdering,
{
    pub fn new(terms: Vec<VariableId>, sum: VariableId) -> Self {
        let mut all_vars = terms.clone();
        all_vars.push(sum);
        Self {
            terms,
            sum,
            all_vars,
            _phantom: PhantomData,
        }
    }
}

impl<S: DomainSemantics + std::fmt::Debug> Constraint<S> for SumOfConstraint<S>
where
    S::Value: ValueArithmetic + ValueOrdering,
{
    fn variables(&self) -> &[VariableId] {
        &self.all_vars
    }

    fn descriptor(&self) -> ConstraintDescriptor {
        let terms_str = self
            .terms
            .iter()
            .map(|v| format!("?{}", v))
            .collect::<Vec<_>>()
            .join(" + ");
        ConstraintDescriptor {
            name: "SumOfConstraint".to_string(),
            description: format!("{} == ?{}", terms_str, self.sum),
        }
    }

    fn revise(
        &self,
        target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>> {
        let is_target_in_terms = self.terms.contains(target_var);
        let is_target_the_sum = *target_var == self.sum;

        if !is_target_in_terms && !is_target_the_sum {
            return Ok(None);
        }

        let target_domain = solution.domains.get(target_var).unwrap();
        let original_size = target_domain.len();

        let new_domain = if is_target_the_sum {
            // Case 1: Revise the SUM variable.
            // min(S) must be >= sum(min(terms))
            // max(S) must be <= sum(max(terms))
            let mut sum_of_mins: Option<S::Value> = None;
            let mut sum_of_maxs: Option<S::Value> = None;

            for term_id in &self.terms {
                let term_domain = solution.domains.get(term_id).unwrap();
                let (min_val, max_val) = (term_domain.get_min_value(), term_domain.get_max_value());

                if min_val.is_none() || max_val.is_none() {
                    return Ok(None); // Cannot propagate if any term domain is unbounded or empty.
                }

                let min_v = min_val.unwrap();
                sum_of_mins = Some(
                    sum_of_mins
                        .take()
                        .map_or(min_v.clone(), |acc| acc.add(&min_v)),
                );

                let max_v = max_val.unwrap();
                sum_of_maxs = Some(
                    sum_of_maxs
                        .take()
                        .map_or(max_v.clone(), |acc| acc.add(&max_v)),
                );
            }

            if self.terms.is_empty() {
                return Ok(None); // Or handle sum of zero if we have an identity
            }

            let new_min_s = sum_of_mins.unwrap();
            let new_max_s = sum_of_maxs.unwrap();

            target_domain.retain(&|v| v >= &new_min_s && v <= &new_max_s)
        } else {
            // Case 2: Revise a TERM variable.
            // max(T_i) <= max(S) - sum(min(T_j)) for j!=i
            // min(T_i) >= min(S) - sum(max(T_j)) for j!=i
            let sum_domain = solution.domains.get(&self.sum).unwrap();
            let Some(min_s) = sum_domain.get_min_value() else {
                return Ok(None);
            };
            let Some(max_s) = sum_domain.get_max_value() else {
                return Ok(None);
            };

            let mut sum_of_mins_of_others: Option<S::Value> = None;
            let mut sum_of_maxs_of_others: Option<S::Value> = None;

            for other_term_id in &self.terms {
                if other_term_id == target_var {
                    continue;
                }
                let term_domain = solution.domains.get(other_term_id).unwrap();
                let (min_val, max_val) = (term_domain.get_min_value(), term_domain.get_max_value());

                if min_val.is_none() || max_val.is_none() {
                    return Ok(None);
                }

                let min_v = min_val.unwrap();
                sum_of_mins_of_others = Some(
                    sum_of_mins_of_others
                        .take()
                        .map_or(min_v.clone(), |acc| acc.add(&min_v)),
                );

                let max_v = max_val.unwrap();
                sum_of_maxs_of_others = Some(
                    sum_of_maxs_of_others
                        .take()
                        .map_or(max_v.clone(), |acc| acc.add(&max_v)),
                );
            }

            let new_max_t = sum_of_mins_of_others
                .as_ref()
                .map_or(max_s.clone(), |s| max_s.sub(s));
            let new_min_t = sum_of_maxs_of_others
                .as_ref()
                .map_or(min_s.clone(), |s| min_s.sub(s));

            target_domain.retain(&|v| v >= &new_min_t && v <= &new_max_t)
        };

        if new_domain.len() < original_size {
            let new_domains = solution.domains.update(*target_var, new_domain);
            let new_solution = solution.clone_with_domains(new_domains);
            Ok(Some(new_solution))
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
        value::{StandardValue, ValueArithmetic},
    };

    // --- Test Setup ---

    #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
    struct TestValue(StandardValue);

    impl From<StandardValue> for TestValue {
        fn from(v: StandardValue) -> Self {
            Self(v)
        }
    }

    impl ValueArithmetic for TestValue {
        fn add(&self, other: &Self) -> Self {
            match (&self.0, &other.0) {
                (StandardValue::Int(a), StandardValue::Int(b)) => Self(StandardValue::Int(a + b)),
                _ => panic!("Unsupported types for addition in test"),
            }
        }
        fn sub(&self, other: &Self) -> Self {
            match (&self.0, &other.0) {
                (StandardValue::Int(a), StandardValue::Int(b)) => Self(StandardValue::Int(a - b)),
                _ => panic!("Unsupported types for subtraction in test"),
            }
        }
        fn abs(&self) -> Self {
            match &self.0 {
                StandardValue::Int(a) => Self(StandardValue::Int(a.abs())),
                _ => panic!("Unsupported types for abs in test"),
            }
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

    fn domain_from_slice(values: &[i64]) -> Box<dyn DomainRepresentation<TestValue>> {
        Box::new(HashSetDomain::new(
            values.iter().map(|&i| int_val(i)).collect(),
        ))
    }

    // --- Tests ---

    #[test]
    fn revise_sum_variable_prunes_domain() {
        let t1: VariableId = 0;
        let t2: VariableId = 1;
        let s: VariableId = 2;
        let constraint = SumOfConstraint::<TestSemantics>::new(vec![t1, t2], s);

        let domains = im::hashmap! {
            t1 => domain_from_slice(&[1, 2]),
            t2 => domain_from_slice(&[3, 4]),
            s => domain_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let new_solution = constraint.revise(&s, &solution).unwrap().unwrap();
        let new_s_domain = new_solution.domains.get(&s).unwrap();
        let expected_domain: im::HashSet<TestValue> =
            [int_val(4), int_val(5), int_val(6)].into_iter().collect();

        assert_eq!(
            new_s_domain.iter().cloned().collect::<im::HashSet<_>>(),
            expected_domain
        );
    }

    #[test]
    fn revise_term_variable_prunes_domain() {
        let t1: VariableId = 0;
        let t2: VariableId = 1;
        let s: VariableId = 2;
        let constraint = SumOfConstraint::<TestSemantics>::new(vec![t1, t2], s);

        let domains = im::hashmap! {
            t1 => domain_from_slice(&[1, 2]),
            t2 => domain_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
            s => domain_from_slice(&[5, 6]),
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let new_solution = constraint.revise(&t2, &solution).unwrap().unwrap();
        let new_t2_domain = new_solution.domains.get(&t2).unwrap();
        let expected_domain: im::HashSet<TestValue> =
            [int_val(3), int_val(4), int_val(5)].into_iter().collect();

        assert_eq!(
            new_t2_domain.iter().cloned().collect::<im::HashSet<_>>(),
            expected_domain
        );
    }

    #[test]
    fn revise_does_nothing_if_already_consistent() {
        let t1: VariableId = 0;
        let t2: VariableId = 1;
        let s: VariableId = 2;
        let constraint = SumOfConstraint::<TestSemantics>::new(vec![t1, t2], s);

        let domains = im::hashmap! {
            t1 => domain_from_slice(&[1, 2]),
            t2 => domain_from_slice(&[3, 4]),
            s => domain_from_slice(&[4, 5, 6]),
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let result = constraint.revise(&s, &solution).unwrap();
        assert!(result.is_none());

        let result2 = constraint.revise(&t1, &solution).unwrap();
        assert!(result2.is_none());
    }

    #[test]
    fn revise_does_nothing_if_any_term_domain_is_empty() {
        let t1: VariableId = 0;
        let t2: VariableId = 1;
        let s: VariableId = 2;
        let constraint = SumOfConstraint::<TestSemantics>::new(vec![t1, t2], s);

        let domains = im::hashmap! {
            t1 => domain_from_slice(&[]),
            t2 => domain_from_slice(&[3, 4]),
            s => domain_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let result = constraint.revise(&s, &solution).unwrap();
        assert!(result.is_none());
    }
}
