use std::sync::Arc;

use im::{HashMap, HashSet, OrdSet};

use crate::solver::{
    engine::VariableId,
    semantics::DomainSemantics,
    value::{ValueEquality, ValueOrdering},
};

// V is a type that implements the ValueEquality trait, e.g., Pod2Value or SudokuValue
pub type Domain<V> = Box<dyn DomainRepresentation<V>>;
pub type Domains<V> = HashMap<VariableId, Domain<V>>;

/// Represents a single, immutable state in the solver's search space.
///
/// A `Solution` holds the current domain of possible values for every
/// variable in the problem. Because it uses persistent (immutable) data
/// structures, it can be cloned cheaply. When a constraint prunes a domain,
/// a new `Solution` is created rather than modifying the existing one.
#[derive(Clone, Debug)]
pub struct Solution<S: DomainSemantics> {
    /// A map from each variable's ID to its current domain of possible values.
    pub domains: Domains<S::Value>,
    /// Read-only access to the problem's semantics, shared across all solutions.
    pub semantics: Arc<S>,
}

impl<S: DomainSemantics> Solution<S> {
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

/// A trait for different ways to represent a variable's domain.
///
/// This allows the solver to be flexible about how domains are stored (e.g.,
/// as a hash set, a range, or a bitmask), while providing a consistent
/// interface for the solver's algorithms.
///
/// This trait allows for different underlying data structures to be used for
/// representing the set of possible values for a variable (e.g., hash sets,
/// ordered sets, bitsets, ranges).
pub trait DomainRepresentation<V: ValueEquality>: std::fmt::Debug {
    fn as_any(&self) -> &dyn std::any::Any;

    /// Returns the number of possible values in the domain.
    fn len(&self) -> usize;

    /// Returns `true` if the domain contains no values.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the domain contains exactly one value.
    fn is_singleton(&self) -> bool {
        self.len() == 1
    }

    /// If the domain is a singleton, returns the single value. Otherwise, `None`.
    fn get_singleton_value(&self) -> Option<V>;

    /// Returns an iterator over the values in the domain.
    fn iter(&self) -> Box<dyn Iterator<Item = &V> + '_>;

    fn debug_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn std::fmt::Debug> + 'a> {
        Box::new(self.iter().map(|item| item as &dyn std::fmt::Debug))
    }

    /// Creates a new domain containing only the values that satisfy the predicate.
    fn retain(&self, f: &dyn Fn(&V) -> bool) -> Box<dyn DomainRepresentation<V>>;

    /// Returns a boxed clone of the domain.
    fn clone_box(&self) -> Box<dyn DomainRepresentation<V>>;

    /// Creates a new domain representing the intersection of this domain and another.
    fn intersect(&self, other: &dyn DomainRepresentation<V>) -> Box<dyn DomainRepresentation<V>>;
}

/// A [`DomainRepresentation`] that uses an `im::HashSet` to store values.
///
/// This implementation is efficient for general-purpose use where value order
/// is not important.
#[derive(Clone, Debug)]
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
    fn iter(&self) -> Box<dyn Iterator<Item = &V> + '_> {
        Box::new(self.0.iter())
    }
    fn retain(&self, f: &dyn Fn(&V) -> bool) -> Box<dyn DomainRepresentation<V>> {
        let new_set = self.0.iter().filter(|v| f(v)).cloned().collect();
        Box::new(Self(new_set))
    }
    fn clone_box(&self) -> Box<dyn DomainRepresentation<V>> {
        Box::new(self.clone())
    }
    fn intersect(&self, other: &dyn DomainRepresentation<V>) -> Box<dyn DomainRepresentation<V>> {
        let other_values: im::HashSet<V> = other.iter().cloned().collect();
        let new_inner = self.0.clone().intersection(other_values);
        Box::new(Self(new_inner))
    }
}

/// A concrete domain implementation using an ordered set.
///
/// This is useful for domains where the order of values is meaningful and
/// can potentially be used for more efficient operations.
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
    fn iter(&self) -> Box<dyn Iterator<Item = &V> + '_> {
        Box::new(self.0.iter())
    }
    fn retain(&self, f: &dyn Fn(&V) -> bool) -> Box<dyn DomainRepresentation<V>> {
        let new_set = self.0.iter().filter(|v| f(v)).cloned().collect();
        Box::new(Self(new_set))
    }
    fn clone_box(&self) -> Box<dyn DomainRepresentation<V>> {
        Box::new(self.clone())
    }
    fn intersect(&self, other: &dyn DomainRepresentation<V>) -> Box<dyn DomainRepresentation<V>> {
        let other_values: im::HashSet<V> = other.iter().cloned().collect();
        let new_inner = self
            .0
            .iter()
            .filter(|v| other_values.contains(v))
            .cloned()
            .collect();
        Box::new(Self(new_inner))
    }
}

/// A [`DomainRepresentation`] that uses an `im::OrdSet` to store values.
///
/// This is useful for domains where the values have a natural order.
#[derive(Clone, Debug)]
pub struct OrdSetDomain<V: ValueOrdering>(pub OrdSet<V>);

impl<V: ValueOrdering> OrdSetDomain<V> {
    /// Creates a new `OrdSetDomain` from an ordered set.
    pub fn new(values: OrdSet<V>) -> Self {
        Self(values)
    }
}

impl<V: ValueOrdering> DomainRepresentation<V> for OrdSetDomain<V> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn get_singleton_value(&self) -> Option<V> {
        if self.len() == 1 {
            self.0.get_min().cloned()
        } else {
            None
        }
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &V> + '_> {
        Box::new(self.0.iter())
    }

    fn retain(&self, f: &dyn Fn(&V) -> bool) -> Box<dyn DomainRepresentation<V>> {
        let new_set = self.0.iter().filter(|v| f(v)).cloned().collect();
        Box::new(Self(new_set))
    }

    fn clone_box(&self) -> Box<dyn DomainRepresentation<V>> {
        Box::new(self.clone())
    }

    fn intersect(&self, other: &dyn DomainRepresentation<V>) -> Box<dyn DomainRepresentation<V>> {
        let other_values: im::HashSet<V> = other.iter().cloned().collect();
        let new_inner = self
            .0
            .iter()
            .filter(|v| other_values.contains(v))
            .cloned()
            .collect();
        Box::new(Self(new_inner))
    }
}

impl<V: ValueEquality> Clone for Box<dyn DomainRepresentation<V>> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
