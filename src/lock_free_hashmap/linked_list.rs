use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::{AtomicPtr, Ordering};

pub struct Node<T: Hash + Default + Clone> {
    data: T,
    hash: u64,
    next: Option<AtomicPtr<Node<T>>>,
}

impl<T: Hash + Default + Clone> Node<T> {
    pub fn new(data: &T) -> AtomicPtr<Node<T>> {
        AtomicPtr::new(Box::into_raw(Box::new(Node {
            data: data.clone(),
            hash: get_hash(&data),
            next: None,
        })))
    }
    pub fn new_with_next(data: &T, next: Option<AtomicPtr<Node<T>>>) -> AtomicPtr<Node<T>> {
        AtomicPtr::new(Box::into_raw(Box::new(Node {
            data: data.clone(),
            hash: get_hash(&data),
            next,
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

    // pub fn set_next(&mut self, next: Option<AtomicPtr<Node<T>>>) {
    //     self.next = next;
    // }

    // pub fn next(&self) -> Option<&AtomicPtr<Node<T>>> {
    //     self.next.as_ref()
    // }
    // pub fn data(&self) -> &T {
    //     &self.data
    // }
    pub fn hash(&self) -> u64 {
        self.hash
    }
}

pub struct LinkedList<T: Hash + Default + Clone> {
    head: AtomicPtr<Node<T>>,
}

impl<T: Hash + Default + Clone> LinkedList<T> {
    pub fn new() -> Self {
        let head = AtomicPtr::new(Box::into_raw(Box::new(Node {
            data: Default::default(),
            hash: u64::MIN,
            next: None,
        })));
        LinkedList { head }
    }
    pub fn add(&self, data: &T) -> Option<T> {
        let hash = get_hash(&data);
        let mut cur_ptr = &self.head;
        let mut current = self.head.load(Ordering::Acquire);
        loop {
            match unsafe { &(*current).next } {
                Some(next_ptr) => {
                    let next = next_ptr.load(Ordering::Acquire);
                    let next_hash = unsafe { (*next).hash() };
                    if next_hash == hash {
                        return None;
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
                    let cur_hash = unsafe { (*cur).hash() };
                    if cur_hash == hash {
                        let new_prev = AtomicPtr::new(Box::into_raw(Box::new(Node {
                            data: unsafe { (*prev).data.clone() },
                            hash: unsafe { (*prev).hash.clone() },
                            next: unsafe { (*cur).next.as_ref().map(|r| AtomicPtr::new(r.load(Ordering::Acquire))) },
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
                    let next_hash = unsafe { (*next_ptr.load(Ordering::Acquire)).hash };
                    if next_hash == hash {
                        return Some(index);
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
    fn test_linked_list_add() {
        let list = LinkedList::new();
        assert_eq!(list.add(&1), Some(1));
        assert_eq!(list.add(&1), None);
        assert_eq!(list.add(&2), Some(2));
    }
    #[test]
    fn test_linked_list_find() {
        let list = LinkedList::new();
        assert_eq!(list.add(&1), Some(1));
        assert_eq!(list.find(&1), Some(0));
        assert_eq!(list.find(&2), None);
        assert_eq!(list.add(&2), Some(2));
        assert_eq!(list.find(&1), Some(0));
        assert_eq!(list.find(&2), Some(1));
    }
    #[test]
    fn test_linked_list_remove() {
        let list = LinkedList::new();
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
        is_send::<LinkedList<i32>>();
        is_sync::<LinkedList<i32>>();
    }
}
