use std::sync::Arc;

use im::{HashMap, HashSet, OrdSet};

use crate::{
    solver::engine::VariableId,
    solver::{
        semantics::DomainSemantics,
        value::{ValueEquality, ValueOrdering},
    },
};

// V is a type that implements the ValueEquality trait, e.g., Pod2Value or SudokuValue
pub type Domain<V> = Box<dyn DomainRepresentation<V>>;
pub type WildcardDomains<V> = HashMap<VariableId, Domain<V>>;

// The primary state passed to `revise`
#[derive(Clone)]
pub struct CandidateSolution<S: DomainSemantics> {
    pub domains: WildcardDomains<S::Value>,
    pub semantics: Arc<S>, // Using Arc here as semantics are read-only and shared widely
}

impl<S: DomainSemantics> CandidateSolution<S> {
    /// Checks if every variable's domain is a singleton.
    pub fn is_complete(&self) -> bool {
        self.domains.values().all(|domain| domain.is_singleton())
    }

    /// Selects the first variable with more than one value in its domain.
    /// A more sophisticated heuristic (e.g., minimum remaining values) could be used here.
    pub fn select_unassigned_variable(&self) -> Option<VariableId> {
        self.domains
            .iter()
            .find(|(_, domain)| domain.len() > 1)
            .map(|(var_id, _)| *var_id)
    }
}

pub trait DomainRepresentation<V: ValueEquality>: 'static {
    fn as_any(&self) -> &dyn std::any::Any;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn is_singleton(&self) -> bool {
        self.len() == 1
    }

    fn get_singleton_value(&self) -> Option<V>;

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a V> + 'a>;

    fn debug_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn std::fmt::Debug> + 'a> {
        Box::new(self.iter().map(|item| item as &dyn std::fmt::Debug))
    }

    fn retain(&self, f: &dyn Fn(&V) -> bool) -> Box<dyn DomainRepresentation<V>>;

    fn clone_box(&self) -> Box<dyn DomainRepresentation<V>>;
}

// A concrete implementation using a simple hash set
#[derive(Debug, Clone)]
pub struct HashSetDomain<V: ValueEquality>(pub HashSet<V>);

impl<V: ValueEquality> HashSetDomain<V> {
    pub fn new(values: HashSet<V>) -> Self {
        Self(values)
    }
}

impl<V: ValueEquality> DomainRepresentation<V> for HashSetDomain<V> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get_singleton_value(&self) -> Option<V> {
        if self.len() == 1 {
            self.0.iter().next().cloned()
        } else {
            None
        }
    }
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a V> + 'a> {
        Box::new(self.0.iter())
    }
    fn retain(&self, f: &dyn Fn(&V) -> bool) -> Box<dyn DomainRepresentation<V>> {
        let new_set = self.0.iter().filter(|v| f(v)).cloned().collect();
        Box::new(Self(new_set))
    }
    fn clone_box(&self) -> Box<dyn DomainRepresentation<V>> {
        Box::new(self.clone())
    }
}

// A concrete implementation for ordered domains
#[derive(Debug, Clone)]
pub struct OrderedDomain<V: ValueOrdering>(pub OrdSet<V>);

impl<V: ValueOrdering> OrderedDomain<V> {
    pub fn new(values: OrdSet<V>) -> Self {
        Self(values)
    }
}

impl<V: ValueOrdering> DomainRepresentation<V> for OrderedDomain<V> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get_singleton_value(&self) -> Option<V> {
        if self.len() == 1 {
            self.0.iter().next().cloned()
        } else {
            None
        }
    }
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a V> + 'a> {
        Box::new(self.0.iter())
    }
    fn retain(&self, f: &dyn Fn(&V) -> bool) -> Box<dyn DomainRepresentation<V>> {
        let new_set = self.0.iter().filter(|v| f(v)).cloned().collect();
        Box::new(Self(new_set))
    }
    fn clone_box(&self) -> Box<dyn DomainRepresentation<V>> {
        Box::new(self.clone())
    }
}

impl<V: ValueEquality> Clone for Box<dyn DomainRepresentation<V>> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
