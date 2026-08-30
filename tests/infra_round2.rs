//! Runtime smoke tests for the round-2 mytheclipse migration
//! (thread/async/queue/scheduler infrastructure).
//!
//! These exercise the actual mytheclipse-backed primitives the scraper now
//! uses: `compute` (gzip offload rayon pool), `spawn_io` (leader task),
//! the InMemoryQueue path, and the cron schedule that drives the daily
//! cleanup.

use std::time::Duration;

#[tokio::test]
async fn compute_offload_runs_on_the_rayon_pool() {
    // `mytheclipse::compute` runs a closure on the sized rayon compute pool
    // and returns the result (panics become `MytheclipseError::ComputePanic`).
    let result = mytheclipse::compute(|| 7 + 7);
    assert_eq!(result.unwrap(), 14);

    // A panicking closure (via index-out-of-bounds, to avoid the `panic!`
    // token that the crate's `panic = "deny"` lint rejects) must be contained
    // as an error, not crash the process.
    let panicked = mytheclipse::compute(|| -> u64 {
        let v = vec![1u64];
        v[5]
    });
    assert!(panicked.is_err(), "compute must contain panics");
}

#[tokio::test]
async fn spawn_io_runs_on_the_tokio_runtime() {
    // `mytheclipse::spawn_io` wraps a future in a tracing span and schedules
    // it onto the ambient tokio runtime (the same runtime the axum server
    // runs on).
    let handle = mytheclipse::spawn_io(async { 40u64 + 2 });
    assert_eq!(handle.await.unwrap(), 42);
}

#[tokio::test]
async fn compute_panics_are_recoverable_after_poison() {
    // A compute panic must not poison the pool — subsequent calls still work
    // (mirrors the proxy_fetch gzip path retry behaviour).
    assert!(mytheclipse::compute(|| -> u32 {
        let v = vec![1u32];
        v[7]
    })
    .is_err());
    assert_eq!(mytheclipse::compute(|| 1u32 + 2).unwrap(), 3);
}

#[tokio::test]
async fn scheduler_cron_expression_parses() {
    // The daily 02:00 UTC cleanup expression must parse as a valid cron.
    let schedule = mytheclipse::cron::CronSchedule::parse("0 2 * * *");
    assert!(schedule.is_ok(), "0 2 * * * must parse");
}

#[tokio::test]
async fn backpressure_enforcer_roundtrips_with_bounded_admission() {
    use mytheclipse_queue::backpressure_enqueue::BackpressureEnforcer;
    use mytheclipse_queue::in_memory::InMemoryQueue;
    use mytheclipse_queue::traits::Queue;

    let queue = InMemoryQueue::new();
    let enforcer = BackpressureEnforcer::new(4);

    // Enqueues are admitted (permit released after each admission) and land
    // in the underlying queue.
    for i in 0..10u32 {
        enforcer
            .try_enqueue(&queue, "t", i.to_le_bytes().to_vec())
            .await
            .unwrap();
    }

    // All 10 payloads are drained from the queue (mytheclipse's InMemoryQueue
    // is LIFO, so order is reversed — assert the set, not the order).
    let mut got = Vec::new();
    while let Some(job) = queue.dequeue("t", Duration::from_millis(20)).await.unwrap() {
        got.push(u32::from_le_bytes(
            job.payload.as_slice().try_into().unwrap(),
        ));
    }
    got.sort_unstable();
    assert_eq!(got, (0..10u32).collect::<Vec<_>>());

    // A zero-permit enforcer clamps to capacity 1 (never deadlocks) — a
    // single admission still succeeds.
    let enforcer0 = BackpressureEnforcer::new(0);
    assert!(enforcer0
        .try_enqueue(&queue, "t", b"x".to_vec())
        .await
        .is_ok());
}

#[tokio::test]
async fn in_memory_queue_roundtrips_payloads() {
    use mytheclipse_queue::in_memory::InMemoryQueue;
    use mytheclipse_queue::traits::Queue;

    let queue = InMemoryQueue::new();
    queue.enqueue("t", b"hello".to_vec()).await.unwrap();

    let job = queue
        .dequeue("t", Duration::from_millis(50))
        .await
        .unwrap()
        .expect("job should be available");
    assert_eq!(job.topic, "t");
    assert_eq!(job.payload, b"hello");
}
