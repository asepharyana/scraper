//! Application job queue — backed by `mytheclipse-queue`.
//!
//! The scraper keeps a small in-process job queue for background maintenance
//! work (currently: image-cache repair / re-sync jobs). The queue is an
//! [`InMemoryQueue`] consumed by a [`WorkerPool`] with bounded concurrency
//! and retry/backoff, all from the mytheclipse queue crate.

use std::sync::Arc;
use std::time::Duration;

use mytheclipse_queue::in_memory::InMemoryQueue;
use mytheclipse_queue::traits::Queue;
use mytheclipse_queue::worker::{JobHandler, WorkerConfig, WorkerPool};

/// Topic for background image-cache repair jobs.
pub const TOPIC_CACHE_REPAIR: &str = "scrape:repair";

/// A handle to the application job queue + its worker pool.
#[derive(Clone)]
pub struct JobQueue {
    queue: Arc<InMemoryQueue>,
    pool: Arc<WorkerPool<InMemoryQueue>>,
    backpressure: Arc<mytheclipse_queue::backpressure_enqueue::BackpressureEnforcer>,
}

impl JobQueue {
    /// Builds a queue with `concurrency` workers consuming the repair topic.
    pub fn new(concurrency: usize) -> Self {
        let queue = InMemoryQueue::new();
        let pool = Arc::new(WorkerPool::new(queue.clone(), concurrency));
        let backpressure = Arc::new(
            mytheclipse_queue::backpressure_enqueue::BackpressureEnforcer::new(concurrency.max(4)),
        );
        Self {
            queue: Arc::new(queue),
            pool,
            backpressure,
        }
    }

    /// Starts `concurrency` workers for the given topic and handler.
    pub fn start_workers<H>(&self, topic: &str, handler: H)
    where
        H: JobHandler + 'static,
    {
        self.pool.start(topic, handler);
    }

    /// Enqueues a raw payload onto `topic`, rejecting under backpressure.
    pub async fn enqueue(&self, topic: &str, payload: Vec<u8>) -> Result<(), String> {
        match self
            .backpressure
            .try_enqueue(&*self.queue, topic, payload)
            .await
        {
            Ok(()) => Ok(()),
            Err(e) => Err(format!("queue backpressure: {e}")),
        }
    }

    /// Number of jobs waiting in a topic.
    pub async fn len(&self, topic: &str) -> u64 {
        self.queue.len(topic).await.unwrap_or(0)
    }

    /// Underlying queue reference (for direct Queue trait calls).
    pub fn queue(&self) -> &Arc<InMemoryQueue> {
        &self.queue
    }
}

/// Default worker configuration for repair jobs: 4 concurrent, 3 retries,
/// exponential backoff 500ms→10s.
pub fn repair_worker_config() -> WorkerConfig {
    WorkerConfig {
        concurrency: 4,
        max_retries: 3,
        retry_base_delay: Duration::from_millis(500),
        retry_max_delay: Duration::from_secs(10),
        retry_factor: 2.0,
        visibility_timeout: Duration::from_secs(30),
        poll_interval: Duration::from_millis(100),
    }
}

/// Convenience: enqueue a cache-repair job for a poster URL.
pub async fn enqueue_cache_repair(url: String) -> Result<(), String> {
    let queue = crate::infrastructure::queue::global_job_queue();
    queue.enqueue(TOPIC_CACHE_REPAIR, url.into_bytes()).await
}

use std::sync::OnceLock;

static JOB_QUEUE: OnceLock<JobQueue> = OnceLock::new();

/// Global job queue handle.
pub fn global_job_queue() -> &'static JobQueue {
    JOB_QUEUE.get_or_init(|| JobQueue::new(4))
}

/// Initialize the global queue with its repair-topic workers.
pub fn init_global_job_queue() {
    let queue = global_job_queue();
    queue.start_workers(
        TOPIC_CACHE_REPAIR,
        |job: mytheclipse_queue::job::Job| async move {
            let url = String::from_utf8_lossy(&job.payload).to_string();
            tracing::info!("[repair] processing cache job for {url}");
            // Current repair action: log + no-op cache touch. Real repair logic
            // (re-fetch + re-upload poster) is wired in a follow-up round.
            Ok(())
        },
    );
}
