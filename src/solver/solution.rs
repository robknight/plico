use std::sync::Arc;

use im::{HashMap, HashSet, OrdSet};

use crate::solver::{
    engine::VariableId,
    semantics::DomainSemantics,
    value::{ValueEquality, ValueOrdering, ValueRange},
};

/// A type alias for a boxed, dynamically-dispatchable [`DomainRepresentation`].
///
/// This allows the solver to work with different domain implementations
/// (e.g., `HashSetDomain`, `RangeDomain`) through a single, consistent interface.
pub type Domain<V> = Box<dyn DomainRepresentation<V>>;

/// A type alias for the persistent map holding the domains for all variables.
pub type Domains<V> = HashMap<VariableId, Domain<V>>;

/// Represents a single, immutable state in the solver's search space.
///
/// A `Solution` holds the current domain of possible values for every
/// variable in the problem. Because it uses persistent (immutable) data
/// structures from the `im` crate, it can be cloned cheaply. When a constraint
/// prunes a domain, a new `Solution` is created with the updated domain, rather
/// than modifying the existing one in place. This is crucial for backtracking
/// search and simplifies state management.
#[derive(Clone, Debug)]
pub struct Solution<S: DomainSemantics> {
    /// A map from each variable's ID to its current domain of possible values.
    pub domains: Domains<S::Value>,
    /// A map from each variable's ID to its semantic metadata tag.
    pub variable_metadata: HashMap<VariableId, S::VariableMetadata>,
    /// Read-only access to the problem's semantics, shared across all solutions.
    pub semantics: Arc<S>,
}

impl<S: DomainSemantics> Solution<S> {
    /// Checks if the solution is complete, meaning every variable's domain
    /// has been pruned to a single value.
    pub fn is_complete(&self) -> bool {
        self.domains.values().all(|domain| domain.is_singleton())
    }

    /// Creates a new solution from a set of domains, variable metadata, and semantics.
    pub fn new(
        domains: Domains<S::Value>,
        variable_metadata: HashMap<VariableId, S::VariableMetadata>,
        semantics: Arc<S>,
    ) -> Self {
        Self {
            domains,
            variable_metadata,
            semantics,
        }
    }

    /// Creates a solution with the same variable metadata and semantics, but with a new set of domains.
    ///
    /// This is a cheap clone operation due to the use of persistent data structures.
    pub fn clone_with_domains(&self, domains: Domains<S::Value>) -> Self {
        Self::new(
            domains,
            self.variable_metadata.clone(),
            self.semantics.clone(),
        )
    }
}

/// A trait for different ways to represent a variable's domain.
///
/// This allows the solver to be flexible about how domains are stored (e.g.,
/// as a hash set, a range, or a bitmask), while providing a consistent
/// interface for the solver's algorithms. Different representations have
/// different performance and memory characteristics, and choosing the right
/// one can significantly impact solver performance.
pub trait DomainRepresentation<V: ValueEquality>: std::fmt::Debug {
    /// Returns the domain as a `&dyn Any` to allow for downcasting to a concrete type.
    ///
    /// This is used, for example, to implement specialized, efficient intersections
    /// between two domains of the same concrete type (e.g., two `RangeDomain`s).
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

    /// If the domain is a singleton, returns the single value. Otherwise, returns `None`.
    fn get_singleton_value(&self) -> Option<V>;

    /// Returns `true` if the domain contains the specified value.
    fn contains(&self, value: &V) -> bool;

    /// Returns an iterator over the values in the domain.
    ///
    /// Note: For some domain types like `RangeDomain`, this may have special
    /// performance considerations. See the documentation on the specific
    /// implementation.
    fn iter(&self) -> Box<dyn Iterator<Item = &V> + '_>;

    /// Creates a new domain containing only the values that satisfy the predicate.
    fn retain(&self, f: &dyn Fn(&V) -> bool) -> Box<dyn DomainRepresentation<V>>;

    /// Returns a boxed clone of the domain.
    fn clone_box(&self) -> Box<dyn DomainRepresentation<V>>;

    /// Creates a new domain representing the intersection of this domain and another.
    ///
    /// The default implementation should be correct for any two domains, but
    /// concrete types can provide more efficient, specialized implementations.
    fn intersect(&self, other: &dyn DomainRepresentation<V>) -> Box<dyn DomainRepresentation<V>>;

    /// If the domain supports ordering, returns the minimum value.
    fn get_min_value(&self) -> Option<V>
    where
        V: ValueOrdering;

    /// If the domain supports ordering, returns the maximum value.
    fn get_max_value(&self) -> Option<V>
    where
        V: ValueOrdering;
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
    fn contains(&self, value: &V) -> bool {
        self.0.contains(value)
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
    fn get_min_value(&self) -> Option<V>
    where
        V: ValueOrdering,
    {
        self.0.iter().min().cloned()
    }
    fn get_max_value(&self) -> Option<V>
    where
        V: ValueOrdering,
    {
        self.0.iter().max().cloned()
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
    fn contains(&self, value: &V) -> bool {
        self.0.contains(value)
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
    fn get_min_value(&self) -> Option<V>
    where
        V: ValueOrdering,
    {
        self.0.get_min().cloned()
    }
    fn get_max_value(&self) -> Option<V>
    where
        V: ValueOrdering,
    {
        self.0.get_max().cloned()
    }
}

/// A [`DomainRepresentation`] that uses a simple `min` and `max` bound.
///
/// This domain is highly efficient for problems with large, continuous ranges
/// of values where intermediate "holes" are not needed. It uses less memory
/// and allows for faster bounds propagation than discrete domains.
///
/// # Warning: Iterator Memory Leak
///
/// The `iter()` method for this domain has a significant side effect: it leaks
/// memory for every value it yields. This is a deliberate trade-off to satisfy
/// the `DomainRepresentation` trait's lifetime requirements while avoiding
/// performance-killing allocations on every iteration for other domain types.
/// This domain is optimized for bounds propagation (`get_min_value`, `get_max_value`,
/// and `intersect` with other `RangeDomain`s), not for iteration.
///
/// **Use this domain with caution if you need to frequently iterate over the
/// values of a large range.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeDomain<V: ValueRange> {
    min: V,
    max: V,
}

impl<V: ValueRange> RangeDomain<V> {
    /// Creates a new `RangeDomain`. Returns `None` if `min > max`.
    pub fn new(min: V, max: V) -> Option<Self> {
        if min > max {
            None
        } else {
            Some(Self { min, max })
        }
    }
}

/// An iterator that generates values for a `RangeDomain`.
///
/// To satisfy the `DomainRepresentation::iter` trait which must return an
/// iterator of references (`&V`), this iterator uses `Box::leak`. This safely
/// leaks memory for each value it creates, producing a `&'static V`. This is a
/// known trade-off for using `RangeDomain` with algorithms that require value
/// iteration, and it should be used with caution on very large ranges.
struct RangeDomainIterator<V: ValueRange> {
    current: V,
    max: V,
}

impl<V: ValueRange> Iterator for RangeDomainIterator<V> {
    type Item = &'static V;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current > self.max {
            None
        } else {
            let val = Box::new(self.current.clone());
            self.current = self.current.successor();
            Some(Box::leak(val))
        }
    }
}

impl<V: ValueRange> DomainRepresentation<V> for RangeDomain<V> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn len(&self) -> usize {
        (self.min.distance(&self.max) + 1) as usize
    }
    fn get_singleton_value(&self) -> Option<V> {
        if self.min == self.max {
            Some(self.min.clone())
        } else {
            None
        }
    }
    fn contains(&self, value: &V) -> bool {
        &self.min <= value && value <= &self.max
    }
    fn iter(&self) -> Box<dyn Iterator<Item = &V> + '_> {
        // WARNING: This iterator leaks memory for each value yielded. See the
        // documentation for `RangeDomainIterator` for more details.
        let static_iterator: Box<dyn Iterator<Item = &'static V>> = Box::new(RangeDomainIterator {
            current: self.min.clone(),
            max: self.max.clone(),
        });
        // SAFETY: This is a safe transmutation because the iterator's items have a 'static
        // lifetime, which is strictly longer than the `'_` lifetime required by the trait.
        // The iterator itself does not borrow from `self`.
        unsafe { std::mem::transmute(static_iterator) }
    }
    fn retain(&self, f: &dyn Fn(&V) -> bool) -> Box<dyn DomainRepresentation<V>> {
        // We can't use the default self.iter() here as it leaks.
        // Instead, we implement the loop manually to avoid the leak.
        let mut current = self.min.clone();
        let mut new_values = im::HashSet::new();
        while current <= self.max {
            if f(&current) {
                new_values.insert(current.clone());
            }
            if current == self.max {
                break;
            }
            current = current.successor();
        }

        // This is not a true range domain anymore. A better implementation might
        // try to find the new min/max, but that's complex with a generic predicate.
        // For now, we degrade to a HashSetDomain if retain is used on a RangeDomain.
        Box::new(HashSetDomain::new(new_values))
    }
    fn clone_box(&self) -> Box<dyn DomainRepresentation<V>> {
        Box::new(self.clone())
    }
    fn intersect(&self, other: &dyn DomainRepresentation<V>) -> Box<dyn DomainRepresentation<V>> {
        if let Some(other_range) = other.as_any().downcast_ref::<Self>() {
            // Efficient O(1) intersection for two RangeDomains
            let new_min = std::cmp::max(self.min.clone(), other_range.min.clone());
            let new_max = std::cmp::min(self.max.clone(), other_range.max.clone());
            if let Some(domain) = Self::new(new_min, new_max) {
                Box::new(domain)
            } else {
                // If min > max, the intersection is empty. We'll represent this
                // with an empty discrete domain.
                Box::new(HashSetDomain::new(im::HashSet::new()))
            }
        } else {
            // Fallback for intersecting with a non-range domain.
            // This is inefficient because it relies on the leaking iterator.
            let other_values: im::HashSet<V> = other.iter().cloned().collect();
            let mut new_set = im::HashSet::new();
            let mut current = self.min.clone();
            while current <= self.max {
                if other_values.contains(&current) {
                    new_set.insert(current.clone());
                }
                if current == self.max {
                    break;
                }
                current = current.successor();
            }
            Box::new(HashSetDomain::new(new_set))
        }
    }
    fn get_min_value(&self) -> Option<V>
    where
        V: ValueOrdering,
    {
        Some(self.min.clone())
    }
    fn get_max_value(&self) -> Option<V>
    where
        V: ValueOrdering,
    {
        Some(self.max.clone())
    }
}

impl<V: ValueEquality> Clone for Box<dyn DomainRepresentation<V>> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
