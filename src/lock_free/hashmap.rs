use std::{
    hash::{BuildHasher, BuildHasherDefault, DefaultHasher, Hash, Hasher},
    sync::atomic::{AtomicPtr, Ordering},
};

pub struct MapNode<K: Clone + PartialOrd, V: Clone> {
    key: K,
    value: V,
    next: Option<AtomicPtr<MapNode<K, V>>>,
}

impl<K: Clone + PartialOrd, V: Clone> MapNode<K, V> {
    pub fn new(key: &K, value: &V) -> AtomicPtr<MapNode<K, V>> {
        AtomicPtr::new(Box::into_raw(Box::new(MapNode {
            key: key.clone(),
            value: value.clone(),
            next: None,
        })))
    }

    pub fn new_head(
        old_head_ptr: AtomicPtr<MapNode<K, V>>,
        key: &K,
        value: &V,
    ) -> AtomicPtr<MapNode<K, V>> {
        AtomicPtr::new(Box::into_raw(Box::new(MapNode {
            key: key.clone(),
            value: value.clone(),
            next: Some(old_head_ptr),
        })))
    }

    pub fn new_insert(old: &MapNode<K, V>, key: &K, value: &V) -> AtomicPtr<MapNode<K, V>> {
        AtomicPtr::new(Box::into_raw(Box::new(MapNode {
            key: old.key.clone(),
            value: old.value.clone(),
            next: Some(AtomicPtr::new(Box::into_raw(Box::new(MapNode {
                key: key.clone(),
                value: value.clone(),
                next: old
                    .next
                    .as_ref()
                    .map(|ptr| AtomicPtr::new(ptr.load(Ordering::Acquire))),
            })))),
        })))
    }

    pub fn change_value(old: &MapNode<K, V>, value: &V) -> AtomicPtr<MapNode<K, V>> {
        AtomicPtr::new(Box::into_raw(Box::new(MapNode {
            key: old.key.clone(),
            value: value.clone(),
            next: old
                .next
                .as_ref()
                .map(|ptr| AtomicPtr::new(ptr.load(Ordering::Acquire))),
        })))
    }
}

impl<K: Clone + PartialOrd, V: Clone> Drop for MapNode<K, V> {
    fn drop(&mut self) {
        if let Some(next) = self.next.as_ref() {
            let _ = unsafe { Box::from_raw(next.load(Ordering::Acquire)) };
        }
    }
}

pub struct Map<K: Clone + PartialOrd, V: Clone> {
    head: AtomicPtr<MapNode<K, V>>,
}

impl<K: Clone + PartialOrd, V: Clone> Map<K, V> {
    pub fn new() -> Self {
        Map {
            head: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    pub fn add(&self, key: &K, value: &V) -> Option<&V> {
        loop {
            let prev_ptr = self.get_ptr(key);
            let prev = prev_ptr.load(Ordering::Acquire);
            if prev.is_null() {
                let new_head = MapNode::new(key, value);
                match self.head.compare_exchange(
                    self.head.load(Ordering::Acquire),
                    new_head.load(Ordering::Relaxed),
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return None,
                    Err(_) => {
                        let _ = unsafe { Box::from_raw(new_head.load(Ordering::Acquire)) };
                        continue;
                    },
                }
            } else {
                let mut ret = None;
                let new_prev = if unsafe { &(*prev).key } == key {
                    ret = Some(unsafe { &(*prev).value });
                    MapNode::change_value(unsafe { &(*prev) }, value)
                } else {
                    MapNode::new_insert(unsafe { &(*prev) }, key, value)
                };
                match prev_ptr.compare_exchange(
                    prev,
                    new_prev.load(Ordering::Relaxed),
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return ret,
                    Err(_) => {
                        let _ = unsafe { Box::from_raw(new_prev.load(Ordering::Acquire)) };
                        continue;
                    },
                }
            }
        }
    }
    pub fn remove(&self, key: &K) -> Option<V> {
        loop {
            let mut prev = self.head.load(Ordering::Acquire);
            let mut prev_ptr = &self.head;
            if prev.is_null() {
                return None;
            }
            if unsafe { &(*prev).key } == key {
                let new_head = match unsafe { (*prev).next.as_ref() } {
                    Some(next) => next.load(Ordering::Acquire),
                    None => std::ptr::null_mut(),
                };
                match prev_ptr.compare_exchange(
                    prev,
                    new_head,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        let value = unsafe { (*prev).value.clone() };
                        let _ = unsafe { Box::from_raw(prev) };
                        return Some(value);
                    }
                    Err(_) => {
                        let _ = unsafe { Box::from_raw(new_head) };
                        continue;
                    }
                }
            } else if unsafe { (*prev).next.is_some() } {
                let mut cur_ptr = unsafe { (*prev).next.as_ref().unwrap() };
                let mut cur = cur_ptr.load(Ordering::Acquire);
                loop {
                    if unsafe { &(*cur).key } == key {
                        let new_prev = AtomicPtr::new(Box::into_raw(Box::new(MapNode {
                            key: unsafe { (*prev).key.clone() },
                            value: unsafe { (*prev).value.clone() },
                            next: unsafe {
                                (*cur)
                                    .next
                                    .as_ref()
                                    .map(|r| AtomicPtr::new(r.load(Ordering::Acquire)))
                            },
                        })));
                        match prev_ptr.compare_exchange(
                            prev,
                            new_prev.load(Ordering::Relaxed),
                            Ordering::Acquire,
                            Ordering::Relaxed,
                        ) {
                            Ok(_) => {
                                let value = unsafe { (*cur).value.clone() };
                                let _ = unsafe { Box::from_raw(cur) };
                                return Some(value);
                            }
                            Err(_) => {
                                let _ = unsafe { Box::from_raw(new_prev.load(Ordering::Acquire)) };
                                continue;
                            }
                        }
                    }
                    prev = cur;
                    prev_ptr = cur_ptr;
                    match unsafe { (*cur).next.as_ref() } {
                        Some(next) => {
                            cur_ptr = next;
                            cur = cur_ptr.load(Ordering::Acquire)
                        }
                        None => return None,
                    }
                }
            } else {
                return None;
            }
        }
    }
    pub fn get(&self, key: &K) -> Option<(&K, &V)> {
        let prev_ptr = self.get_ptr(key);
        let prev = prev_ptr.load(Ordering::Acquire);
        if prev.is_null() {
            return None;
        }
        let k = unsafe { &(*prev).key };
        let v = unsafe { &(*prev).value };
        if k == key {
            Some((k, v))
        } else {
            None
        }
    }

    pub fn is_null(&self) -> bool {
        self.head.load(Ordering::Acquire).is_null()
    }

    pub fn copy(&self) -> Self {
        Map {
            head: AtomicPtr::new(self.head.load(Ordering::Acquire)),
        }
    }

    fn get_ptr(&self, key: &K) -> &AtomicPtr<MapNode<K, V>> {
        let mut cur_ptr = &self.head;
        let mut cur = cur_ptr.load(Ordering::Acquire);
        if cur.is_null() {
            return cur_ptr;
        }
        loop {
            if unsafe { &(*cur).key } >= key {
                return cur_ptr;
            }
            match unsafe { (*cur).next.as_ref() } {
                Some(next) => {
                    cur_ptr = next;
                    cur = cur_ptr.load(Ordering::Acquire);
                }
                None => return cur_ptr,
            }
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
        assert_eq!(list.add(&1, &2), Some(&1));
        assert_eq!(list.add(&2, &1), None);
    }
    #[test]
    fn test_map_get() {
        let list = Map::new();
        assert_eq!(list.add(&1, &1), None);
        assert_eq!(list.get(&1), Some((&1, &1)));
        assert_eq!(list.add(&1, &2), Some(&1));
        assert_eq!(list.get(&1), Some((&1, &2)));
        assert_eq!(list.get(&2), None);
        assert_eq!(list.add(&2, &1), None);
        assert_eq!(list.get(&1), Some((&1, &2)));
        assert_eq!(list.get(&2), Some((&2, &1)));
    }
    #[test]
    fn test_map_null_add_get_remove() {
        let list = Map::new();
        assert_eq!(list.is_null(), true);
        assert_eq!(list.get(&1), None);
        assert_eq!(list.remove(&1), None);
        assert_eq!(list.add(&1, &1), None);
        assert_eq!(list.is_null(), false);
        assert_eq!(list.get(&1), Some((&1, &1)));
        assert_eq!(list.remove(&1), Some(1));
        assert_eq!(list.is_null(), true);
    }
    #[test]
    fn test_map_remove() {
        let list = Map::new();
        assert_eq!(list.remove(&1), None);
        assert_eq!(list.add(&1, &1), None);
        assert_eq!(list.get(&1), Some((&1, &1)));
        assert_eq!(list.add(&2, &1), None);
        assert_eq!(list.get(&2), Some((&2, &1)));
        assert_eq!(list.remove(&2), Some(1));
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
