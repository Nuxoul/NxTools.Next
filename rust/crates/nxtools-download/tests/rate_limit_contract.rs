use std::time::{Duration, Instant};

use nxtools_download::DownloadRateLimiter;

#[tokio::test]
async fn one_shared_budget_limits_the_aggregate_bytes_of_concurrent_workers() {
    let limiter =
        DownloadRateLimiter::new(Some(64 * 1024)).expect("create a 64 KiB/s shared limiter");
    let started = Instant::now();

    let first = limiter.acquire(32 * 1024);
    let second = limiter.acquire(32 * 1024);
    tokio::join!(first, second);

    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_millis(800),
        "two workers must share one 64 KiB/s budget; elapsed={elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "the shared limiter should not serialize independent per-worker limits; elapsed={elapsed:?}"
    );
}

#[tokio::test]
async fn changing_a_limit_wakes_current_waiters_and_takes_effect_immediately() {
    let limiter = DownloadRateLimiter::new(Some(8 * 1024)).expect("create slow limiter");
    let worker_limiter = limiter.clone();
    let worker = tokio::spawn(async move {
        worker_limiter.acquire(64 * 1024).await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    limiter
        .set_bytes_per_second(None)
        .expect("remove speed limit");
    tokio::time::timeout(Duration::from_millis(300), worker)
        .await
        .expect("live limit update must wake the transfer")
        .expect("limiter worker completes");
}
