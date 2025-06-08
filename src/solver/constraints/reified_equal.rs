//! A constraint that reifies an equality relationship.
//!
//! This constraint links a boolean variable `B` to the outcome of an equality
//! check between two other variables, `X` and `Y`. The core relationship is
//! `B <==> (X == Y)`.

use std::marker::PhantomData;

use crate::{
    error::{Result, SolverError},
    solver::{
        constraint::{Constraint, ConstraintDescriptor},
        engine::VariableId,
        semantics::DomainSemantics,
        solution::Solution,
        value::StandardValue,
    },
};

/// A constraint that reifies an equality relationship, enforcing `B <==> (X == Y)`.
///
/// Reification turns a constraint's satisfaction status into a boolean variable.
/// This constraint links a boolean variable `B` to the outcome of `X == Y`.
/// Propagation works in all directions:
/// - If `B` is true, it enforces `X == Y`.
/// - If `B` is false, it enforces `X != Y`.
/// - If the domains of `X` and `Y` are proven to be equal (i.e., both are
///   singletons with the same value), `B` is forced to be true.
/// - If the domains of `X` and `Y` are proven to be disjoint, `B` is forced to be
///   false.
#[derive(Debug, Clone)]
pub struct ReifiedEqualConstraint<S: DomainSemantics + std::fmt::Debug>
where
    S::Value: From<StandardValue>,
{
    b: VariableId,
    x: VariableId,
    y: VariableId,
    vars: [VariableId; 3],
    _phantom: PhantomData<S>,
}

impl<S: DomainSemantics + std::fmt::Debug> ReifiedEqualConstraint<S>
where
    S::Value: From<StandardValue>,
{
    /// Creates a new `ReifiedEqualConstraint` enforcing `b <==> (x == y)`.
    pub fn new(b: VariableId, x: VariableId, y: VariableId) -> Self {
        Self {
            b,
            x,
            y,
            vars: [b, x, y],
            _phantom: PhantomData,
        }
    }

    /// Revision logic when the target is one of the data variables (X or Y).
    fn revise_xy(
        &self,
        target_var: VariableId,
        other_var: VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>> {
        let b_domain = solution.domains.get(&self.b).unwrap();
        let target_domain = solution.domains.get(&target_var).unwrap();
        let other_domain = solution.domains.get(&other_var).unwrap();

        let b_is_true = b_domain.is_singleton()
            && b_domain.get_singleton_value().unwrap() == S::Value::from(StandardValue::Bool(true));
        let b_is_false = b_domain.is_singleton()
            && b_domain.get_singleton_value().unwrap()
                == S::Value::from(StandardValue::Bool(false));

        let mut new_target_domain = target_domain.clone_box();
        let original_size = new_target_domain.len();

        if b_is_true {
            // Enforce target == other
            new_target_domain = new_target_domain.intersect(other_domain.as_ref());
        } else if b_is_false && other_domain.is_singleton() {
            // Enforce target != other
            let other_val = other_domain.get_singleton_value().unwrap();
            new_target_domain = new_target_domain.retain(&|v| v != &other_val);
        }

        if new_target_domain.len() < original_size {
            let new_domains = solution.domains.update(target_var, new_target_domain);
            Ok(Some(solution.clone_with_domains(new_domains)))
        } else {
            Ok(None)
        }
    }

    /// Revision logic when the target is the boolean variable (B).
    fn revise_b(&self, solution: &Solution<S>) -> Result<Option<Solution<S>>> {
        let b_domain = solution.domains.get(&self.b).unwrap();
        let x_domain = solution.domains.get(&self.x).unwrap();
        let y_domain = solution.domains.get(&self.y).unwrap();

        // If domains are disjoint, B must be false
        if x_domain.intersect(y_domain.as_ref()).is_empty() {
            let new_b_domain =
                b_domain.retain(&|v| v == &S::Value::from(StandardValue::Bool(false)));
            if new_b_domain.len() < b_domain.len() {
                let new_domains = solution.domains.update(self.b, new_b_domain);
                return Ok(Some(solution.clone_with_domains(new_domains)));
            }
        }

        // If X and Y must be equal, B must be true
        if x_domain.is_singleton()
            && x_domain.get_singleton_value() == y_domain.get_singleton_value()
        {
            let new_b_domain =
                b_domain.retain(&|v| v == &S::Value::from(StandardValue::Bool(true)));
            if new_b_domain.len() < b_domain.len() {
                let new_domains = solution.domains.update(self.b, new_b_domain);
                return Ok(Some(solution.clone_with_domains(new_domains)));
            }
        }

        Ok(None)
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
            description: format!("?{} <==> (?{} == ?{})", self.b, self.x, self.y),
        }
    }

    fn revise(
        &self,
        target_var: &VariableId,
        solution: &Solution<S>,
    ) -> Result<Option<Solution<S>>> {
        if *target_var == self.b {
            self.revise_b(solution)
        } else if *target_var == self.x {
            self.revise_xy(self.x, self.y, solution)
        } else if *target_var == self.y {
            self.revise_xy(self.y, self.x, solution)
        } else {
            Err(SolverError::Custom(format!(
                "ReifiedEqualConstraint revised with a variable not in the constraint: {}",
                target_var
            ))
            .into())
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

    // --- Tests ---

    #[test]
    fn revise_b_when_xy_are_equal_singletons() {
        let b: VariableId = 0;
        let x: VariableId = 1;
        let y: VariableId = 2;
        let constraint = ReifiedEqualConstraint::<TestSemantics>::new(b, x, y);

        let domains = im::hashmap! {
            b => bool_domain(),
            x => Box::new(HashSetDomain::new([int_val(5)].into_iter().collect())) as Box<dyn DomainRepresentation<TestValue>>,
            y => Box::new(HashSetDomain::new([int_val(5)].into_iter().collect())) as Box<dyn DomainRepresentation<TestValue>>,
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let new_solution = constraint.revise(&b, &solution).unwrap().unwrap();
        let new_b_domain = new_solution.domains.get(&b).unwrap();

        assert!(new_b_domain.is_singleton());
        assert_eq!(new_b_domain.get_singleton_value().unwrap(), bool_val(true));
    }

    #[test]
    fn revise_b_when_xy_are_disjoint() {
        let b: VariableId = 0;
        let x: VariableId = 1;
        let y: VariableId = 2;
        let constraint = ReifiedEqualConstraint::<TestSemantics>::new(b, x, y);

        let domains = im::hashmap! {
            b => bool_domain(),
            x => Box::new(HashSetDomain::new(
                    [int_val(1), int_val(2)].into_iter().collect(),
                )) as Box<dyn DomainRepresentation<TestValue>>,
            y => Box::new(HashSetDomain::new(
                    [int_val(3), int_val(4)].into_iter().collect(),
                )) as Box<dyn DomainRepresentation<TestValue>>,
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let new_solution = constraint.revise(&b, &solution).unwrap().unwrap();
        let new_b_domain = new_solution.domains.get(&b).unwrap();

        assert!(new_b_domain.is_singleton());
        assert_eq!(new_b_domain.get_singleton_value().unwrap(), bool_val(false));
    }

    #[test]
    fn revise_x_when_b_is_true() {
        let b: VariableId = 0;
        let x: VariableId = 1;
        let y: VariableId = 2;
        let constraint = ReifiedEqualConstraint::<TestSemantics>::new(b, x, y);

        let domains = im::hashmap! {
            b => Box::new(HashSetDomain::new([bool_val(true)].into_iter().collect())) as Box<dyn DomainRepresentation<TestValue>>,
            x => Box::new(HashSetDomain::new(
                    [int_val(1), int_val(2), int_val(3)]
                        .into_iter()
                        .collect(),
                )) as Box<dyn DomainRepresentation<TestValue>>,
            y => Box::new(HashSetDomain::new(
                    [int_val(2), int_val(3), int_val(4)]
                        .into_iter()
                        .collect(),
                )) as Box<dyn DomainRepresentation<TestValue>>,
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let new_solution = constraint.revise(&x, &solution).unwrap().unwrap();
        let new_x_domain = new_solution.domains.get(&x).unwrap();
        let expected_domain: im::HashSet<TestValue> =
            [int_val(2), int_val(3)].into_iter().collect();

        assert_eq!(new_x_domain.len(), 2);
        assert_eq!(
            new_x_domain.iter().cloned().collect::<im::HashSet<_>>(),
            expected_domain
        );
    }

    #[test]
    fn revise_x_when_b_is_false_and_y_is_singleton() {
        let b: VariableId = 0;
        let x: VariableId = 1;
        let y: VariableId = 2;
        let constraint = ReifiedEqualConstraint::<TestSemantics>::new(b, x, y);

        let domains = im::hashmap! {
            b => Box::new(HashSetDomain::new(
                    [bool_val(false)].into_iter().collect(),
                )) as Box<dyn DomainRepresentation<TestValue>>,
            x => Box::new(HashSetDomain::new(
                    [int_val(1), int_val(2), int_val(3)]
                        .into_iter()
                        .collect(),
                )) as Box<dyn DomainRepresentation<TestValue>>,
            y => Box::new(HashSetDomain::new([int_val(2)].into_iter().collect())) as Box<dyn DomainRepresentation<TestValue>>,
        };
        let solution = Solution::new(domains, HashMap::new(), Arc::new(TestSemantics));

        let new_solution = constraint.revise(&x, &solution).unwrap().unwrap();
        let new_x_domain = new_solution.domains.get(&x).unwrap();
        let expected_domain: im::HashSet<TestValue> =
            [int_val(1), int_val(3)].into_iter().collect();

        assert_eq!(new_x_domain.len(), 2);
        assert_eq!(
            new_x_domain.iter().cloned().collect::<im::HashSet<_>>(),
            expected_domain
        );
    }
}
