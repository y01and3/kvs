use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, RwLock};

pub struct Node<T: Hash + Default + Clone> {
    data: T,
    hash: u64,
    next: Option<Arc<RwLock<Node<T>>>>,
}

impl<T: Hash + Default + Clone> Node<T> {
    pub fn new(data: &T) -> Arc<RwLock<Node<T>>> {
        let hash = get_hash(&data);
        Arc::new(RwLock::new(Node {
            data: data.clone(),
            hash,
            next: None,
        }))
    }
    pub fn new_with_next(
        data: &T,
        hash: u64,
        next: Option<Arc<RwLock<Node<T>>>>,
    ) -> Arc<RwLock<Node<T>>> {
        Arc::new(RwLock::new(Node {
            data: data.clone(),
            hash,
            next,
        }))
    }

    pub fn set_next(&mut self, next: Option<Arc<RwLock<Node<T>>>>) {
        self.next = next;
    }

    pub fn next(&self) -> Option<Arc<RwLock<Node<T>>>> {
        self.next.clone()
    }
    pub fn data(&self) -> &T {
        &self.data
    }
    pub fn hash(&self) -> u64 {
        self.hash
    }
}

pub struct LinkedList<T: Hash + Default + Clone> {
    head: Arc<RwLock<Node<T>>>,
}

impl<T: Hash + Default + Clone> LinkedList<T> {
    pub fn new() -> Self {
        let head = Arc::new(RwLock::new(Node {
            data: Default::default(),
            hash: u64::MIN,
            next: None,
        }));
        LinkedList { head }
    }
    pub fn add(&self, data: &T) -> Option<T> {
        let hash = get_hash(&data);
        let mut current = Arc::clone(&self.head);
        loop {
            let next = current.read().unwrap().next();
            match next {
                Some(next) => {
                    let next_read = next.read().unwrap();
                    if next_read.hash() > hash {
                        current.write().unwrap().set_next(Some(Node::new_with_next(
                            data,
                            hash,
                            Some(Arc::clone(&next)),
                        )));
                        return Some(data.clone());
                    } else if next_read.hash() == hash {
                        return None;
                    }
                    current = Arc::clone(&next);
                }
                None => {
                    current.write().unwrap().set_next(Some(Node::new(data)));
                    return Some(data.clone());
                }
            }
        }
    }
    pub fn remove(&self, data: &T) -> Option<T> {
        let hash = get_hash(data);
        let mut prev = Arc::clone(&self.head);
        let mut current = Arc::clone(&self.head).read().unwrap().next();
        loop {
            match current {
                Some(cur) => {
                    let cur_read = cur.read().unwrap();
                    let next = cur.read().unwrap().next();
                    if cur_read.hash() == hash {
                        prev.write().unwrap().set_next(next);
                        return Some(data.clone());
                    } else if cur_read.hash() > hash {
                        return None;
                    }
                    prev = Arc::clone(&cur);
                    current = next;
                }
                None => return None,
            }
        }
    }
    pub fn find(&self, data: &T) -> Option<()> {
        let hash = get_hash(data);
        let mut current = Arc::clone(&self.head).read().unwrap().next();
        loop {
            match current {
                Some(cur) => {
                    let cur_read = cur.read().unwrap();
                    if cur_read.hash() == hash {
                        return Some(());
                    } else if cur_read.hash() > hash {
                        return None;
                    }
                    current = cur_read.next();
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
    }
    #[test]
    fn test_linked_list_find() {
        let list = LinkedList::new();
        assert_eq!(list.add(&1), Some(1));
        assert_eq!(list.find(&1), Some(()));
        assert_eq!(list.find(&2), None);
    }
    #[test]
    fn test_linked_list_remove() {
        let list = LinkedList::new();
        assert_eq!(list.add(&1), Some(1));
        assert_eq!(list.remove(&1), Some(1));
        assert_eq!(list.remove(&1), None);
    }

    fn is_send<T: Send>() {}
    fn is_sync<T: Sync>() {}

    #[test]
    fn test_send_sync() {
        is_send::<LinkedList<i32>>();
        is_sync::<LinkedList<i32>>();
    }
}
