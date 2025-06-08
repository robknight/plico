use std::{collections::HashSet, marker::PhantomData};

use crate::{
    error::Result,
    solver::{
        constraint::Constraint, engine::VariableId, semantics::DomainSemantics, solution::Solution,
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
            Ok(Some(Solution {
                domains: new_domains,
                semantics: solution.semantics.clone(),
            }))
        } else {
            Ok(None)
        }
    }
}
