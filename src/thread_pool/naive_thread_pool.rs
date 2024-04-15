use std::{sync::Mutex, thread};

use crate::{thread_pool::ThreadPool, Result};
pub struct NaiveThreadPool {
    threads: u32,
    children: Mutex<Vec<thread::JoinHandle<()>>>,
}

impl ThreadPool for NaiveThreadPool {
    fn new(threads: u32) -> Result<Self> {
        Ok(NaiveThreadPool {
            threads,
            children: Mutex::new(vec![]),
        })
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        loop {
            match self.children.try_lock() {
                Ok(mut children) => {
                    if children.len() < self.threads as usize {
                        children.push(thread::spawn(job));
                        break;
                    } else {
                        let mut i = 0;
                        while i < children.len() {
                            match children[i].is_finished() {
                                true => {
                                    children.remove(i);
                                }
                                false => {
                                    i += 1;
                                }
                            }
                        }
                    }
                }
                Err(std::sync::TryLockError::WouldBlock) => continue,
                Err(_) => panic!("Poisoned lock"),
            }
        }
    }
}
