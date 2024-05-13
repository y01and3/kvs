use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::Ordering;

use crossbeam::epoch::{self, Atomic, Owned};

use super::map::Map;

pub struct HashMapNode<K: Clone + PartialOrd + Hash, V: Clone> {
    map: Map<K, V>,
    hash: u64,
    next: Atomic<HashMapNode<K, V>>,
}

impl<K: Clone + PartialOrd + Hash, V: Clone> HashMapNode<K, V> {
    pub fn new(key: &K, value: &V, hash: u64) -> Owned<HashMapNode<K, V>> {
        let map = Map::new();
        map.add(key, value);
        Owned::new(HashMapNode {
            map,
            hash,
            next: Atomic::null(),
        })
    }

    pub fn new_head(
        old_head_ptr: Atomic<HashMapNode<K, V>>,
        key: &K,
        value: &V,
        hash: u64,
    ) -> Owned<HashMapNode<K, V>> {
        let map = Map::new();
        map.add(key, value);
        Owned::new(HashMapNode {
            map,
            hash: hash,
            next: old_head_ptr,
        })
    }

    pub fn new_insert(
        old: &HashMapNode<K, V>,
        key: &K,
        value: &V,
        hash: u64,
    ) -> Owned<HashMapNode<K, V>> {
        let map = Map::new();
        map.add(key, value);
        Owned::new(HashMapNode {
            map: old.map.copy(),
            hash: old.hash.clone(),
            next: Atomic::new(HashMapNode {
                map,
                hash,
                next: old.next.clone(),
            }),
        })
    }
}

fn default_hasher<K: Clone + PartialOrd + Hash>(key: &K) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

pub struct HashMap<K: Clone + PartialOrd + Hash, V: Clone> {
    head: Atomic<HashMapNode<K, V>>,
    hasher: fn(&K) -> u64,
}

impl<K: Clone + PartialOrd + Hash, V: Clone> HashMap<K, V> {
    pub fn new() -> Self {
        HashMap {
            head: Atomic::null(),
            hasher: default_hasher,
        }
    }

    pub fn new_with_hasher(hasher: fn(&K) -> u64) -> Self {
        HashMap {
            head: Atomic::null(),
            hasher,
        }
    }

    pub fn add(&self, key: &K, value: &V) -> Option<V> {
        let guard = &epoch::pin();
        let hash = (self.hasher)(key);
        loop {
            let mut prev_ptr = &self.head;
            let mut prev = prev_ptr.load(Ordering::Acquire, guard);
            loop {
                if prev.is_null() {
                    let new_prev = HashMapNode::new(key, value, hash);
                    match prev_ptr.compare_exchange_weak(
                        prev,
                        new_prev,
                        Ordering::Acquire,
                        Ordering::Relaxed,
                        guard,
                    ) {
                        Ok(_) => return None,
                        Err(_) => continue,
                    }
                }
                let prev_inner = unsafe { prev.deref() };
                if prev_inner.hash > hash {
                    let new_prev = HashMapNode::new_insert(prev_inner, key, value, hash);
                    match prev_ptr.compare_exchange_weak(
                        prev,
                        new_prev,
                        Ordering::Acquire,
                        Ordering::Relaxed,
                        guard,
                    ) {
                        Ok(_) => return None,
                        Err(_) => continue,
                    }
                } else if prev_inner.hash == hash {
                    return prev_inner.map.add(key, value);
                }
                prev_ptr = &prev_inner.next;
                prev = prev_ptr.load(Ordering::Acquire, guard);
            }
        }
    }

    pub fn get(&self, key: &K) -> Option<(K, V)> {
        let guard = &epoch::pin();
        let hash = (self.hasher)(key);
        let mut prev_ptr = &self.head;
        let mut prev = prev_ptr.load(Ordering::Acquire, guard);
        loop {
            if prev.is_null() {
                return None;
            }
            let prev_inner = unsafe { prev.deref() };
            if prev_inner.hash == hash {
                return prev_inner.map.get(key);
            } else if prev_inner.hash > hash {
                return None;
            }
            prev_ptr = &prev_inner.next;
            prev = prev_ptr.load(Ordering::Acquire, guard);
        }
    }

    pub fn remove(&self, key: &K) -> Option<V> {
        let guard = &epoch::pin();
        let hash = (self.hasher)(key);
        let mut prev_ptr = &self.head;
        let mut prev = prev_ptr.load(Ordering::Acquire, guard);
        loop {
            if prev.is_null() {
                return None;
            }
            let prev_inner = unsafe { prev.deref() };
            if prev_inner.hash == hash {
                let ret = prev_inner.map.remove(key);
                if prev_inner.map.is_null(){
                    match prev_ptr.compare_exchange_weak(
                        prev,
                        prev_inner.next.load(Ordering::Relaxed, guard),
                        Ordering::Acquire,
                        Ordering::Relaxed,
                        guard,
                    ) {
                        Ok(_) => return ret,
                        Err(_) => continue,
                    }
                } else {
                    return ret;
                }
            } else if prev_inner.hash > hash {
                return None;
            }
            prev_ptr = &prev_inner.next;
            prev = prev_ptr.load(Ordering::Acquire, guard);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn my_hasher(key: &i32) -> u64 {
        (key % 10) as u64
    }

    fn is_send<T: Send>() {}
    fn is_sync<T: Sync>() {}

    #[test]
    fn test_send_sync() {
        is_send::<Map<i32, i32>>();
        is_sync::<Map<i32, i32>>();
    }

    #[test]
    fn test_hashmap_add() {
        let map = HashMap::new_with_hasher(my_hasher);
        assert_eq!(map.add(&1, &2), None);
        assert_eq!(map.add(&11, &3), None);
        assert_eq!(map.add(&1, &4), Some(2));
        assert_eq!(map.add(&2, &5), None);
        assert_eq!(map.add(&2, &6), Some(5));
    }

    #[test]
    fn test_hashmap_get() {
        let map = HashMap::new_with_hasher(my_hasher);
        assert_eq!(map.add(&1, &2), None);
        assert_eq!(map.get(&1), Some((1, 2)));
        assert_eq!(map.add(&11, &3), None);
        assert_eq!(map.get(&11), Some((11, 3)));
        assert_eq!(map.add(&1, &4), Some(2));
        assert_eq!(map.get(&1), Some((1, 4)));
        assert_eq!(map.add(&2, &5), None);
        assert_eq!(map.get(&2), Some((2, 5)));
        assert_eq!(map.add(&2, &6), Some(5));
        assert_eq!(map.get(&2), Some((2, 6)));
    }

    #[test]
    fn test_hashmap_remove() {
        let map = HashMap::new_with_hasher(my_hasher);
        assert_eq!(map.add(&1, &2), None);
        assert_eq!(map.add(&11, &3), None);
        assert_eq!(map.add(&1, &4), Some(2));
        assert_eq!(map.add(&2, &5), None);
        assert_eq!(map.add(&2, &6), Some(5));
        assert_eq!(map.remove(&1), Some(4));
        assert_eq!(map.get(&1), None);
        assert_eq!(map.remove(&11), Some(3));
        assert_eq!(map.get(&11), None);
        assert_eq!(map.remove(&2), Some(6));
        assert_eq!(map.get(&2), None);
        assert_eq!(map.remove(&2), None);
    }
}
