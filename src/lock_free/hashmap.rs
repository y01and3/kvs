use std::sync::atomic::{AtomicPtr, Ordering};

pub struct Node<K: Clone + PartialOrd, V: Clone> {
    key: K,
    value: V,
    next: Option<AtomicPtr<Node<K, V>>>,
}

impl<K: Clone + PartialOrd, V: Clone> Node<K, V> {
    pub fn new(key: &K, value: &V) -> AtomicPtr<Node<K, V>> {
        AtomicPtr::new(Box::into_raw(Box::new(Node {
            key: key.clone(),
            value: value.clone(),
            next: None,
        })))
    }

    pub fn new_head(
        old_head_ptr: AtomicPtr<Node<K, V>>,
        key: &K,
        value: &V,
    ) -> AtomicPtr<Node<K, V>> {
        AtomicPtr::new(Box::into_raw(Box::new(Node {
            key: key.clone(),
            value: value.clone(),
            next: Some(old_head_ptr),
        })))
    }

    pub fn new_insert(old: &Node<K, V>, key: &K, value: &V) -> AtomicPtr<Node<K, V>> {
        AtomicPtr::new(Box::into_raw(Box::new(Node {
            key: old.key.clone(),
            value: old.value.clone(),
            next: Some(AtomicPtr::new(Box::into_raw(Box::new(Node {
                key: key.clone(),
                value: value.clone(),
                next: old
                    .next
                    .as_ref()
                    .map(|ptr| AtomicPtr::new(ptr.load(Ordering::Acquire))),
            })))),
        })))
    }

    pub fn change_value(old: &Node<K, V>, value: &V) -> AtomicPtr<Node<K, V>> {
        AtomicPtr::new(Box::into_raw(Box::new(Node {
            key: old.key.clone(),
            value: value.clone(),
            next: old
                .next
                .as_ref()
                .map(|ptr| AtomicPtr::new(ptr.load(Ordering::Acquire))),
        })))
    }
}

pub struct Map<K: Clone + PartialOrd, V: Clone> {
    head: AtomicPtr<Node<K, V>>,
}

impl<K: Clone + PartialOrd, V: Clone> Map<K, V> {
    pub fn new(key: &K, value: &V) -> Self {
        Map {
            head: Node::new(key, value),
        }
    }
    pub fn add(&self, key: &K, value: &V) -> Option<V> {
        if self.head.load(Ordering::Acquire).is_null() {
            let new_head = Node::new(key, value);
            match self.head.compare_exchange(
                self.head.load(Ordering::Acquire),
                new_head.load(Ordering::Relaxed),
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => None,
                Err(_) => self.add(key, value),
            }
        } else {
            let prev_ptr = self.get_prev_ptr(key);
            let prev = prev_ptr.load(Ordering::Acquire);
            let mut ret = None;
            let new_node = if unsafe { (*prev).key.clone() } == *key {
                ret = Some(unsafe { (*prev).value.clone() });
                Node::change_value(unsafe { &(*prev) }, value)
            } else {
                Node::new_insert(unsafe { &(*prev) }, key, value)
            };
            match prev_ptr.compare_exchange(
                prev,
                new_node.load(Ordering::Relaxed),
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => ret,
                Err(_) => self.add(key, value),
            }
        }
    }
    pub fn remove(&self, key: &K) -> Option<V> {
        let key = key.clone();
        let mut prev = self.head.load(Ordering::Acquire);
        let mut prev_ptr = &self.head;
        if prev.is_null() {
            return None;
        }
        if unsafe { (*prev).key.clone() } == key {
            let new_head = unsafe { (*prev).next.as_ref().unwrap().load(Ordering::Acquire) };
            match prev_ptr.compare_exchange(prev, new_head, Ordering::Acquire, Ordering::Relaxed) {
                Ok(_) => return Some(unsafe { (*prev).value.clone() }),
                Err(_) => return self.remove(&key),
            }
        } else if unsafe { (*prev).next.is_some() } {
            let mut cur_ptr = unsafe { (*prev).next.as_ref().unwrap() };
            let mut cur = cur_ptr.load(Ordering::Acquire);
            loop {
                if unsafe { (*cur).key.clone() } == key {
                    let new_prev = AtomicPtr::new(Box::into_raw(Box::new(Node {
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
                        Ok(_) => return Some(unsafe { (*cur).value.clone() }),
                        Err(_) => return self.remove(&key),
                    }
                }
                prev = cur;
                prev_ptr = cur_ptr;
                match unsafe { (*cur).next.as_ref() } {
                    Some(next) => {
                        cur_ptr = next;
                        cur = cur_ptr.load(Ordering::Acquire)},
                    None => return None,
                }
            }
        } else {
            None
        }
    }
    pub fn get(&self, key: &K) -> Option<(&K, &V)> {
        if self.head.load(Ordering::Acquire).is_null() {
            return None;
        }
        let prev_ptr = self.get_prev_ptr(key);
        let prev = prev_ptr.load(Ordering::Acquire);
        if unsafe { (*prev).key.clone() } == *key {
            Some((unsafe { &(*prev).key }, unsafe { &(*prev).value }))
        } else {
            None
        }
    }
    pub fn is_null(&self) -> bool {
        self.head.load(Ordering::Acquire).is_null()
    }
    fn get_prev_ptr(&self, key: &K) -> &AtomicPtr<Node<K, V>> {
        let key = key.clone();
        let mut current = &self.head;
        if current.load(Ordering::Relaxed).is_null() {
            return current;
        }
        loop {
            if unsafe { (*current.load(Ordering::Acquire)).key.clone() } >= key {
                return current;
            }
            match unsafe { (*current.load(Ordering::Acquire)).next.as_ref() } {
                Some(next) => current = next,
                None => return current,
            }
        }
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;

    #[test]
    fn test_map_add() {
        let list = Map::new(&1, &1);
        assert_eq!(list.add(&1, &2), Some(1));
        assert_eq!(list.add(&2, &1), None);
    }
    #[test]
    fn test_map_get() {
        let list = Map::new(&1, &1);
        assert_eq!(list.get(&1), Some((&1, &1)));
        assert_eq!(list.add(&1, &2), Some(1));
        assert_eq!(list.get(&1), Some((&1, &2)));
        assert_eq!(list.get(&2), None);
        assert_eq!(list.add(&2, &1), None);
        assert_eq!(list.get(&1), Some((&1, &2)));
        assert_eq!(list.get(&2), Some((&2, &1)));
    }
    #[test]
    fn test_map_remove() {
        let list = Map::new(&1, &1);
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
