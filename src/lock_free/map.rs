use std::sync::atomic::Ordering;

use crossbeam::epoch::{self, Atomic, Owned};

pub struct MapNode<K: Clone + PartialOrd, V: Clone> {
    key: K,
    value: V,
    next: Atomic<MapNode<K, V>>,
}

impl<K: Clone + PartialOrd, V: Clone> MapNode<K, V> {
    pub fn new(key: &K, value: &V) -> Owned<MapNode<K, V>> {
        Owned::new(MapNode {
            key: key.clone(),
            value: value.clone(),
            next: Atomic::null(),
        })
    }

    pub fn new_head(
        old_head_ptr: Atomic<MapNode<K, V>>,
        key: &K,
        value: &V,
    ) -> Owned<MapNode<K, V>> {
        Owned::new(MapNode {
            key: key.clone(),
            value: value.clone(),
            next: old_head_ptr,
        })
    }

    pub fn new_insert(old: &MapNode<K, V>, key: &K, value: &V) -> Owned<MapNode<K, V>> {
        Owned::new(MapNode {
            key: old.key.clone(),
            value: old.value.clone(),
            next: Atomic::new(MapNode {
                key: key.clone(),
                value: value.clone(),
                next: old.next.clone(),
            }),
        })
    }

    pub fn change_value(old: &MapNode<K, V>, value: &V) -> Owned<MapNode<K, V>> {
        Owned::new(MapNode {
            key: old.key.clone(),
            value: value.clone(),
            next: old.next.clone(),
        })
    }
}

pub struct Map<K: Clone + PartialOrd, V: Clone> {
    head: Atomic<MapNode<K, V>>,
}

impl<K: Clone + PartialOrd, V: Clone> Map<K, V> {
    pub fn new() -> Self {
        Map {
            head: Atomic::null(),
        }
    }

    pub fn add(&self, key: &K, value: &V) -> Option<V> {
        let guard = &epoch::pin();
        loop {
            let mut prev_ptr = &self.head;
            let mut prev = prev_ptr.load(Ordering::Acquire, guard);
            loop {
                if prev.is_null() {
                    let new_prev = MapNode::new(key, value);
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
                if &prev_inner.key >= key {
                    let prev_inner = unsafe { prev.deref() };
                    let mut ret = None;
                    let new_prev = if &prev_inner.key == key {
                        ret = Some(prev_inner.value.clone());
                        MapNode::change_value(prev_inner, value)
                    } else {
                        MapNode::new_insert(prev_inner, key, value)
                    };
                    match prev_ptr.compare_exchange_weak(
                        prev,
                        new_prev,
                        Ordering::Acquire,
                        Ordering::Relaxed,
                        guard,
                    ) {
                        Ok(_) => return ret,
                        Err(_) => continue,
                    }
                }
                prev_ptr = &prev_inner.next;
                prev = prev_ptr.load(Ordering::Acquire, guard);
            }
        }
    }
    pub fn remove(&self, key: &K) -> Option<V> {
        let guard = &epoch::pin();
        loop {
            let mut prev_ptr = &self.head;
            let mut prev = prev_ptr.load(Ordering::Acquire, guard);
            if prev.is_null() {
                return None;
            }
            let mut prev_inner = unsafe { prev.deref() };
            if &prev_inner.key == key {
                match prev_ptr.compare_exchange_weak(
                    prev,
                    prev_inner.next.load(Ordering::Acquire, guard),
                    Ordering::Acquire,
                    Ordering::Relaxed,
                    guard,
                ) {
                    Ok(_) => {
                        let value = prev_inner.value.clone();
                        let _ = unsafe { prev.into_owned() };
                        return Some(value);
                    }
                    Err(_) => continue,
                }
            }
            let mut cur_ptr = &prev_inner.next;
            let mut cur = cur_ptr.load(Ordering::Acquire, guard);
            loop {
                if cur.is_null() {
                    return None;
                }
                let cur_inner = unsafe { cur.deref() };
                if &cur_inner.key == key {
                    let new_prev = Owned::new(MapNode {
                        key: prev_inner.key.clone(),
                        value: prev_inner.value.clone(),
                        next: cur_inner.next.clone(),
                    });
                    match prev_ptr.compare_exchange_weak(
                        prev,
                        new_prev,
                        Ordering::Acquire,
                        Ordering::Relaxed,
                        guard,
                    ) {
                        Ok(_) => {
                            let value = cur_inner.value.clone();
                            let _ = unsafe { cur.into_owned() };
                            return Some(value);
                        }
                        Err(_) => continue,
                    }
                }
                prev_inner = cur_inner;
                prev = cur;
                prev_ptr = cur_ptr;
                cur_ptr = &prev_inner.next;
                cur = cur_ptr.load(Ordering::Acquire, guard);
            }
        }
    }
    pub fn get(&self, key: &K) -> Option<(K, V)> {
        let guard = &epoch::pin();
        let mut prev_ptr = &self.head;
        let mut prev = prev_ptr.load(Ordering::Acquire, guard);
        loop {
            if prev.is_null() {
                return None;
            }
            let prev_inner = unsafe { prev.deref() };
            if &prev_inner.key == key {
                return Some((prev_inner.key.clone(), prev_inner.value.clone()));
            } else if &prev_inner.key > key {
                return None;
            }
            prev_ptr = &prev_inner.next;
            prev = prev_ptr.load(Ordering::Acquire, guard);
        }
    }

    pub fn is_null(&self) -> bool {
        let guard = &epoch::pin();
        self.head.load(Ordering::Acquire, guard).is_null()
    }

    pub fn copy(&self) -> Self {
        Map {
            head: self.head.clone(),
        }
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;

    #[test]
    fn test_map_add() {
        let list = Map::new();
        assert_eq!(list.add(&1, &1), None);
        assert_eq!(list.add(&1, &2), Some(1));
        assert_eq!(list.add(&2, &1), None);
    }
    #[test]
    fn test_map_get() {
        let list = Map::new();
        assert_eq!(list.add(&1, &1), None);
        assert_eq!(list.get(&1), Some((1, 1)));
        assert_eq!(list.add(&1, &2), Some(1));
        assert_eq!(list.get(&1), Some((1, 2)));
        assert_eq!(list.get(&2), None);
        assert_eq!(list.add(&2, &1), None);
        assert_eq!(list.get(&1), Some((1, 2)));
        assert_eq!(list.get(&2), Some((2, 1)));
    }
    #[test]
    fn test_map_null() {
        let list = Map::new();
        assert_eq!(list.is_null(), true);
        assert_eq!(list.get(&0), None);
        assert_eq!(list.remove(&0), None);
        assert_eq!(list.add(&1, &1), None);
        assert_eq!(list.is_null(), false);
        assert_eq!(list.get(&1), Some((1, 1)));
        assert_eq!(list.remove(&1), Some(1));
        assert_eq!(list.is_null(), true);
    }
    #[test]
    fn test_map_remove() {
        let list = Map::new();
        assert_eq!(list.remove(&1), None);
        assert_eq!(list.add(&1, &1), None);
        assert_eq!(list.get(&1), Some((1, 1)));
        assert_eq!(list.add(&2, &1), None);
        assert_eq!(list.get(&1), Some((1, 1)));
        assert_eq!(list.get(&2), Some((2, 1)));
        assert_eq!(list.remove(&2), Some(1));
        assert_eq!(list.get(&1), Some((1, 1)));
        assert_eq!(list.get(&2), None);
        assert_eq!(list.remove(&1), Some(1));
        assert_eq!(list.get(&1), None);
        assert_eq!(list.get(&2), None);
    }

    fn is_send<T: Send>() {}
    fn is_sync<T: Sync>() {}

    #[test]
    fn test_send_sync() {
        is_send::<Map<i32, i32>>();
        is_sync::<Map<i32, i32>>();
    }
}
