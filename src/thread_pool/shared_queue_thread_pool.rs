#![allow(unused)]
use log::error;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    thread,
};

use crossbeam::channel::Sender;

use crate::{thread_pool::ThreadPool, Result};

enum ThreadPoolMessage {
    RunJob(Box<dyn FnOnce() + Send + 'static>),
    Shutdown,
}

pub struct SharedQueueThreadPool {
    sender: Sender<ThreadPoolMessage>,
}

impl ThreadPool for SharedQueueThreadPool {
    fn new(threads: u32) -> Result<Self> {
        let (sender, receiver) = crossbeam::channel::unbounded();
        for _ in 0..threads {
            let receiver = receiver.clone();
            thread::spawn(move || loop {
                match receiver.recv() {
                    Ok(ThreadPoolMessage::RunJob(job)) => {
                        match catch_unwind(AssertUnwindSafe(|| job())) {
                            Ok(_) => (),
                            Err(err) => error!("Thread panicked: {:?}", err),
                        }
                    }
                    Ok(ThreadPoolMessage::Shutdown) => break,
                    Err(_) => break,
                }
            });
        }
        Ok(SharedQueueThreadPool { sender })
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender
            .send(ThreadPoolMessage::RunJob(Box::new(job)))
            .unwrap();
    }
}

impl SharedQueueThreadPool {
    fn shutdown(&self) {
        while self.sender.send(ThreadPoolMessage::Shutdown).is_ok() {}
    }
}
