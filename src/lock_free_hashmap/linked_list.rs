use std::cell::RefCell;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

pub struct Node<T: Hash + Default + Clone> {
    data: T,
    hash: u64,
    next: Option<Rc<RefCell<Node<T>>>>,
}

impl<T: Hash + Default + Clone> Node<T> {
    pub fn new(data: &T) -> Rc<RefCell<Node<T>>> {
        let hash = get_hash(&data);
        Rc::new(RefCell::new(Node {
            data: data.clone(),
            hash,
            next: None,
        }))
    }
    pub fn only_hash(hash: u64) -> Rc<RefCell<Node<T>>> {
        Rc::new(RefCell::new(Node {
            data: Default::default(),
            hash,
            next: None,
        }))
    }

    pub fn set_next(&mut self, next: Option<Rc<RefCell<Node<T>>>>) {
        self.next = next;
    }

    pub fn next(&self) -> Option<Rc<RefCell<Node<T>>>> {
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
    head: Rc<RefCell<Node<T>>>,
}

impl<T: Hash + Default + Clone> LinkedList<T> {
    pub fn new() -> Self {
        let head = Rc::new(RefCell::new(Node {
            data: Default::default(),
            hash: u64::MIN,
            next: None,
        }));
        LinkedList { head }
    }
    pub fn add(&self, data: &T) -> Option<T> {
        let new_node = Node::new(data);
        let mut current = Rc::clone(&self.head);
        loop {
            let next = current.borrow().next();
            match next {
                Some(next) => {
                    if next.borrow().hash() > new_node.borrow().hash() {
                        new_node.borrow_mut().set_next(Some(Rc::clone(&next)));
                        current.borrow_mut().set_next(Some(new_node));
                        return Some(data.clone());
                    } else if next.borrow().hash() == new_node.borrow().hash() {
                        return None;
                    }
                    current = Rc::clone(&next);
                }
                None => {
                    current.borrow_mut().set_next(Some(new_node));
                    return Some(data.clone());
                }
            }
        }
    }
    pub fn remove(&self, data: &T) -> Option<T> {
        let hash = get_hash(data);
        let mut prev = Rc::clone(&self.head);
        let mut current = Rc::clone(&self.head).borrow().next();
        loop {
            match current {
                Some(cur) => {
                    if cur.borrow().hash() == hash {
                        let next = cur.borrow().next();
                        prev.borrow_mut().set_next(next);
                        drop(cur);
                        return Some(data.clone());
                    } else if cur.borrow().hash() > hash {
                        return None;
                    }
                    current = cur.borrow().next();
                    prev = Rc::clone(&cur);
                }
                None => return None,
            }
        }
    }
    pub fn find(&self, data: &T) -> Option<()> {
        let hash = get_hash(data);
        let mut current = Rc::clone(&self.head).borrow().next();
        loop {
            match current {
                Some(cur) => {
                    if cur.borrow().hash() == hash {
                        return Some(());
                    } else if cur.borrow().hash() > hash {
                        return None;
                    }
                    current = cur.borrow().next();
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
}
