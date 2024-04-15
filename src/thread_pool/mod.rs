pub mod naive_thread_pool;
pub mod shared_queue_thread_pool;

pub use naive_thread_pool::NaiveThreadPool;
pub use shared_queue_thread_pool::SharedQueueThreadPool;

use crate::Result;
pub trait ThreadPool {
    fn new(threads: u32) -> Result<Self>
    where
        Self: Sized;
    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static;
}

pub struct RayonThreadPool(rayon::ThreadPool);

impl ThreadPool for RayonThreadPool {
    fn new(threads: u32) -> Result<Self> {
        Ok(RayonThreadPool(
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads as usize)
                .build()
                .map_err(|err| err.to_string())?,
        ))
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.0.spawn(job);
    }
}
