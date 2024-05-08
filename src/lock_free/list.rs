use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::{AtomicPtr, Ordering};

pub struct Node<T: Hash + Default + Clone + Eq> {
    data: T,
    hash: u64,
    next: Option<AtomicPtr<Node<T>>>,
}

impl<T: Hash + Default + Clone + Eq> Node<T> {
    pub fn new(data: &T) -> AtomicPtr<Node<T>> {
        AtomicPtr::new(Box::into_raw(Box::new(Node {
            data: data.clone(),
            hash: get_hash(&data),
            next: None,
        })))
    }
    
    pub fn new_with_old(
        old: &Node<T>,
        data: &T,
        next: Option<AtomicPtr<Node<T>>>,
    ) -> AtomicPtr<Node<T>> {
        AtomicPtr::new(Box::into_raw(Box::new(Node {
            data: old.data.clone(),
            hash: old.hash.clone(),
            next: Some(AtomicPtr::new(Box::into_raw(Box::new(Node {
                data: data.clone(),
                hash: get_hash(data),
                next,
            })))),
        })))
    }
}

pub struct List<T: Hash + Default + Clone + Eq> {
    head: AtomicPtr<Node<T>>,
}

impl<T: Hash + Default + Clone + Eq> List<T> {
    pub fn new() -> Self {
        let head = AtomicPtr::new(Box::into_raw(Box::new(Node {
            data: Default::default(),
            hash: u64::MIN,
            next: None,
        })));
        List { head }
    }
    pub fn add(&self, data: &T) -> Option<T> {
        let hash = get_hash(&data);
        let mut cur_ptr = &self.head;
        let mut current = self.head.load(Ordering::Acquire);
        loop {
            match unsafe { &(*current).next } {
                Some(next_ptr) => {
                    let next = next_ptr.load(Ordering::Acquire);
                    let next_hash = unsafe { (*next).hash.clone() };
                    if next_hash == hash {
                        if unsafe { &(*next).data } == data {
                            return None;
                        }
                    } else if next_hash > hash {
                        let new_current = Node::new_with_old(
                            unsafe { &(*current) },
                            data,
                            Some(AtomicPtr::new(next)),
                        );
                        match cur_ptr.compare_exchange(
                            current,
                            new_current.load(Ordering::Relaxed),
                            Ordering::SeqCst,
                            Ordering::Relaxed,
                        ) {
                            Ok(_) => {
                                return Some(data.clone());
                            }
                            Err(_) => {
                                return self.add(data);
                            }
                        }
                    }
                    cur_ptr = next_ptr;
                    current = next;
                }
                None => {
                    let new_current = Node::new_with_old(unsafe { &(*current) }, data, None);
                    match cur_ptr.compare_exchange(
                        current,
                        new_current.load(Ordering::Relaxed),
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => {
                            return Some(data.clone());
                        }
                        Err(_) => {
                            return self.add(data);
                        }
                    }
                }
            }
        }
    }
    pub fn remove(&self, data: &T) -> Option<T> {
        let hash = get_hash(data);
        let mut prev_ptr = &self.head;
        let mut prev = self.head.load(Ordering::Acquire);
        let mut current = unsafe { &(*self.head.load(Ordering::Acquire)).next };
        loop {
            match current {
                Some(cur_ptr) => {
                    let cur = cur_ptr.load(Ordering::Acquire);
                    let cur_hash = unsafe { (*cur).hash.clone() };
                    if cur_hash == hash {
                        if unsafe { &(*cur).data } == data {
                            let new_prev = AtomicPtr::new(Box::into_raw(Box::new(Node {
                                data: unsafe { (*prev).data.clone() },
                                hash: unsafe { (*prev).hash.clone() },
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
                                Ordering::SeqCst,
                                Ordering::Relaxed,
                            ) {
                                Ok(_) => {
                                    return Some(data.clone());
                                }
                                Err(_) => {
                                    return self.remove(data);
                                }
                            }
                        } else {
                            return None;
                        }
                    } else if cur_hash > hash {
                        return None;
                    }
                    prev_ptr = cur_ptr;
                    prev = cur;
                    current = unsafe { &((*cur).next) };
                }
                None => return None,
            }
        }
    }
    pub fn find(&self, data: &T) -> Option<u32> {
        let hash = get_hash(data);
        let mut cur_ptr = &self.head;
        let mut index = 0;
        loop {
            match unsafe { &(*cur_ptr.load(Ordering::Acquire)).next } {
                Some(next_ptr) => {
                    let next_hash = unsafe { (*next_ptr.load(Ordering::Acquire)).hash.clone() };
                    if next_hash == hash {
                        if unsafe { &(*next_ptr.load(Ordering::Acquire)).data } == data {
                            return Some(index);
                        }
                    } else if next_hash > hash {
                        return None;
                    }
                    cur_ptr = next_ptr;
                    index += 1;
                }
                None => return None,
            }
        }
    }
}

fn get_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

#[cfg(test)]
pub mod tests {

    use super::*;

    #[test]
    fn test_list_add() {
        let list = List::new();
        assert_eq!(list.add(&1), Some(1));
        assert_eq!(list.add(&1), None);
        assert_eq!(list.add(&2), Some(2));
    }
    #[test]
    fn test_list_find() {
        let list = List::new();
        assert_eq!(list.add(&1), Some(1));
        assert_eq!(list.find(&1), Some(0));
        assert_eq!(list.find(&2), None);
        assert_eq!(list.add(&2), Some(2));
        assert_eq!(list.find(&1), Some(0));
        assert_eq!(list.find(&2), Some(1));
    }
    #[test]
    fn test_list_remove() {
        let list = List::new();
        assert_eq!(list.add(&1), Some(1));
        assert_eq!(list.add(&2), Some(2));
        assert_eq!(list.remove(&1), Some(1));
        assert_eq!(list.find(&1), None);
        assert_eq!(list.find(&2), Some(0));
    }

    fn is_send<T: Send>() {}
    fn is_sync<T: Sync>() {}

    #[test]
    fn test_send_sync() {
        is_send::<List<i32>>();
        is_sync::<List<i32>>();
    }
}
