use std::{cell::UnsafeCell, hash::Hash};

use ahash::AHashMap;

/// a util map type used for across Future or Task but same thread!
/// user should ensure that the map must not reach data race access!
pub struct LocalMap<K: Hash + Eq + PartialEq, V>(UnsafeCell<AHashMap<K, V>>);

unsafe impl<K: Hash + Eq + PartialEq, V> Send for LocalMap<K, V> {}
unsafe impl<K: Hash + Eq + PartialEq, V> Sync for LocalMap<K, V> {}

impl<K: Hash + Eq + PartialEq, V> LocalMap<K, V> {
    pub fn new() -> Self {
        Self(UnsafeCell::new(AHashMap::new()))
    }

    pub fn insert(&self, key: K, value: V) {
        let map = unsafe {&mut *self.0.get()};
        map.insert(key, value);
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        unsafe { (*self.0.get()).get(key) }
    }

    pub fn get_mut(&self, key: &K) -> Option<&mut V> {
        unsafe { (*self.0.get()).get_mut(key) }
    }

    pub fn remove(&self, key: &K) -> Option<V> {
        unsafe { (*self.0.get()).remove(key) }
    }
}