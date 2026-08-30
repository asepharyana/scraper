# Spec Round 2 — Scraper → mytheclipse Deep Infra Migration (thread/async/queue)

Date: 2026-08-30
Repo: /home/code/scraper (scraper-service)
Library: /home/code/mytheclipse (mytheclipse crates v1.20+ local path)

## Scope

Round 1 (committed c7ffd29) migrated retry, redis cache, event bus, ratelimit,
config. Round 2 targets the remaining **runtime/concurrency/queue/observability**
infrastructure — the parts the user explicitly asked to push "sampai tingkat
thread, async, queue dan lainnya".

## 1. Runtime thread sizing — `mytheclipse::runtime_auto::RuntimeConfig`

**Current**: bootstrap logs `std::thread::available_parallelism()` manually;
tokio runtime is `#[tokio::main(flavor = "multi_thread")]` with default
worker counts.

**Change**:
- Add `mytheclipse = { features = ["lifecycle", ...] }` (add `lifecycle` feature).
- In `bootstrap/mod.rs`, compute `RuntimeConfig::auto()` once at build, log
  worker/blocking/compute/io counts, and keep the tokio runtime default
  (already multi_thread). Optionally pass `worker_threads` via a `#[tokio::main]`
  alternative — but since `main` uses the macro, we keep the macro form and
  simply surface the auto-derived counts in logs + store them in `AppState`
  for future pool sizing.
- Replace the manual `available_parallelism()` log with `RuntimeConfig::auto()`.

## 2. Async I/O + background task spawns — `mytheclipse::spawn_io` / `spawn_bg`

**Current**:
- `proxy_fetch.rs:114` uses `tokio::spawn(...)` for the request-coalescing leader
- 30+ `tokio::task::spawn_blocking(...)` sites across otakudesu.rs, anime2
  use_cases, komik use_cases, proxy_fetch (gzip decompress)
- No bounded background task pool

**Change**:
- `proxy_fetch.rs` leader task → `mytheclipse::spawn_io(...)` (same semantics,
  adds tracing span instrumentation). No behavior change.
- Add `mytheclipse = { features = ["io", "bg"] }`.
- The `spawn_blocking` sites stay as-is **this round** (they're CPU-bound
  parser calls and `spawn_blocking` is the correct tokio primitive; mytheclipse
  `compute()` would replace them but that's a 2k+ LOC parser sweep — deferred
  as before). Document this in the spec's residual section.
- Add `bg` feature and expose a bounded background executor via
  `mytheclipse::spawn_bg` for the **cache-write / relay-fallback** background
  tasks that currently fire-and-forget (if any are found in use_cases).

## 3. Worker queue — `mytheclipse-queue` `InMemoryQueue` + `WorkerPool`

**Current**: no queue abstraction; the only "background" work is the cache
batch writes (`cache_image_urls_batch_lazy` in the proxy image-cache path —
but proxy routes were removed, so that path may be dead). Event bus has no
worker pool.

**Change**:
- Add `mytheclipse-queue = { path = ..., features = ["in-memory"] }`.
- Wire a small application-level `JobQueue` in `src/infrastructure/queue/`:
  - `InMemoryQueue` + `WorkerPool` consuming a `scrape:repair` topic for
    background image-cache repair/re-sync jobs.
  - Graceful shutdown: on app shutdown, stop workers.
  - This is additive infra (no existing behavior replaced), so it's low-risk
    and demonstrates the queue crate in the scraper.
- Actually **use** it: the `image_cache` repair path (cache cleanup at 2 AM)
  is currently absent (scheduler dir missing). Introduce a small
  `src/infrastructure/scheduler.rs` using `mytheclipse::cron` (lifecycle
  feature) OR `tokio::time::interval` for the daily 02:00 cleanup that
  enqueues repair jobs onto the queue. Given the original CLAUDE.md mentions
  "scheduler — Cron jobs (daily cache cleanup at 2 AM)", recreating it on
  mytheclipse-queue is a faithful, additive migration.

## 4. Backpressure / concurrency — `mytheclipse::ConcurrencyLimiter` / `BackpressureQueue`

**Current**: proxy_fetch has request coalescing via DashMap + broadcast (kept —
it's application logic, not infra). No app-level backpressure.

**Change**:
- Add `mytheclipse = { features = ["traffic"] }` (already enabled for
  RateLimiter in round 1).
- Wrap the **fetch-with-proxy** path with a `ConcurrencyLimiter` (bound
  concurrent outbound scrapes, default e.g. 16) so burst traffic can't
  saturate the upstream. This is a real, central hot path (every scrape goes
  through it).
- Keep the RateLimiter middleware (round 1) for inbound.

## 5. Observability — `mytheclipse-tracing` `TracingLayer` (+ keep OTel metrics)

**Current**: `tracing_subscriber::fmt().with_env_filter(...)` in bootstrap;
opentelemetry metrics in `observability/metrics.rs` (kept — `mytheclipse-tracing`
has no metrics exporter, round-1 decision).

**Change**:
- Add `mytheclipse-tracing = { path = ..., features = ["env"] }`.
- Replace `tracing_subscriber::fmt()` in bootstrap with
  `mytheclipse_tracing::TracingLayer::install()` (same RUST_LOG env-filter
  semantics, adds tracing span layer compatibility).
- Keep `init_otel_metrics()` untouched (OTel exporter stays).

## 6. HTTP client — `mytheclipse-http` `HttpClient` (optional, re-evaluate)

Round 1 deferred this because mytheclipse-http's client "loses header/UA/status
control". Re-check: `mytheclipse-http::client::HttpClient` — if it exposes
`with_timeout`, `get_text`, `get_json`, `post_json`, and header injection,
use it for the non-critical relay fetches (`fetch_via_relays`). If not
header-capable, skip (kept as residual).

## Files touched

- Cargo.toml (features: add lifecycle, io, bg; add mytheclipse-queue, mytheclipse-tracing)
- src/bootstrap/mod.rs (RuntimeConfig log + TracingLayer + scheduler start)
- src/infrastructure/scraping/proxy_fetch.rs (spawn_io + ConcurrencyLimiter)
- src/infrastructure/queue/mod.rs (NEW — InMemoryQueue + WorkerPool + repair topic)
- src/infrastructure/scheduler.rs (NEW — daily 02:00 cleanup → queue enqueue)
- src/infrastructure/cache/mytheclipse.rs (maybe wire limiter through cache path)
- src/presentation/state.rs (add queue handle / limiter if needed)

## Verification

1. `cargo check` — green after each batch
2. `cargo test` — all pass
3. `cargo build --release` — green
4. `cargo fmt --check` on touched files
5. Runtime smoke: boot server, hit `/health`, confirm logs show RuntimeConfig
   and scheduler started; queue jobs process (test harness)

## Residual (deliberately deferred)

- `spawn_blocking` → `mytheclipse::compute()` sweep (2k+ LOC parser files)
- OTel metrics → mytheclipse-tracing (no metrics exporter in library yet)
- rayon in komik_parser
- mytheclipse-http client if header control is insufficient

## Commit

Single commit at end: `refactor: migrate runtime/queue/concurrency infra to mytheclipse (round 2)`