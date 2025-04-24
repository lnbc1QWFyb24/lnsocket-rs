//! Generally LDK uses `hashbrown`'s `HashMap`s with the `std` `SipHasher` and uses `getrandom` to
//! opportunistically randomize it, if randomization is available.
//!
//! This module simply re-exports the `HashMap` used in LDK for public consumption.

mod hashbrown_tables {
    mod hasher {
        pub use std::collections::hash_map::RandomState;
    }

    pub use hasher::*;

    /// The HashMap type used in LDK.
    pub type HashMap<K, V> = hashbrown::HashMap<K, V, RandomState>;
    /// The HashSet type used in LDK.
    pub type HashSet<K> = hashbrown::HashSet<K, RandomState>;

    pub(crate) type _OccupiedHashMapEntry<'a, K, V> =
        hashbrown::hash_map::OccupiedEntry<'a, K, V, RandomState>;
    pub(crate) type _VacantHashMapEntry<'a, K, V> =
        hashbrown::hash_map::VacantEntry<'a, K, V, RandomState>;

    /// Builds a new [`HashMap`].
    pub fn new_hash_map<K, V>() -> HashMap<K, V> {
        HashMap::with_hasher(RandomState::new())
    }
    /// Builds a new [`HashMap`] with the given capacity.
    pub fn hash_map_with_capacity<K, V>(cap: usize) -> HashMap<K, V> {
        HashMap::with_capacity_and_hasher(cap, RandomState::new())
    }
    pub(crate) fn _hash_map_from_iter<
        K: core::hash::Hash + Eq,
        V,
        I: IntoIterator<Item = (K, V)>,
    >(
        iter: I,
    ) -> HashMap<K, V> {
        let iter = iter.into_iter();
        let min_size = iter.size_hint().0;
        let mut res = HashMap::with_capacity_and_hasher(min_size, RandomState::new());
        res.extend(iter);
        res
    }

    /// Builds a new [`HashSet`].
    pub fn new_hash_set<K>() -> HashSet<K> {
        HashSet::with_hasher(RandomState::new())
    }
    /// Builds a new [`HashSet`] with the given capacity.
    pub(crate) fn hash_set_with_capacity<K>(cap: usize) -> HashSet<K> {
        HashSet::with_capacity_and_hasher(cap, RandomState::new())
    }
    pub(crate) fn _hash_set_from_iter<K: core::hash::Hash + Eq, I: IntoIterator<Item = K>>(
        iter: I,
    ) -> HashSet<K> {
        let iter = iter.into_iter();
        let min_size = iter.size_hint().0;
        let mut res = HashSet::with_capacity_and_hasher(min_size, RandomState::new());
        res.extend(iter);
        res
    }
}

pub use hashbrown_tables::*;
