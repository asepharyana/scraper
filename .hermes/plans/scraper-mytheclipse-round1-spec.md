# Spec: Migrate scraper infra to mytheclipse crates (Round 1)

Date: 2026-08-30
Repo: /home/code/scraper (github.com/asepharyana/scraper)
Goal: Replace hand-rolled infrastructure in the scraper with the user's
custom `mytheclipse` library crates where there is a clear 1:1 mapping.

## Library crates (from /home/code/mytheclipse, all v1.20.0)

- `mytheclipse`      — core: retry (retry/RetryConfig/JitterKind/RetryError),
                        ratelimit (RateLimiter/RateLimitError), timeouts
                        (timeout), concurrency primitives, ServiceBuilder,
                        runtime_auto (available_parallelism), spawn_io/spawn_bg.
- `mytheclipse-cache` — Cache trait, MemoryCache, RedisCache (l2-redis),
                        CacheAside (cache-aside), CacheError.
- `mytheclipse-config`— ConfigLoader<T> (file + env merge, hot-reload, validation).
- `mytheclipse-event` — EventBus trait, InMemoryEventBus, TypedEventBus (byte/JSON pub/sub).
- `mytheclipse-crypto`— (NOT used in this round — scraper has no crypto/JWT usage.)

## Current state (baseline)

- Cargo.toml has NO mytheclipse crates. Uses directly:
  - `backoff` — retry with exponential backoff (hand-rolled wrapper in
    src/infrastructure/scraping/retry.rs)
  - `deadpool-redis` — Redis pool + raw `redis::AsyncCommands` in
    src/infrastructure/cache/{redis_pool.rs,redis.rs}
  - `dashmap` — request coalescing in proxy_fetch.rs (IN_FLIGHT map)
  - `rayon` — komik_parser parallel map
  - `config` crate — config loading in src/config/mod.rs
  - `opentelemetry*` — metrics in src/observability/metrics.rs
  - custom EventBus in src/events/bus.rs (unused by any handler)
  - custom RateLimiter in src/presentation/middleware/ratelimit.rs (unused by router)
- Baseline: `cargo check` currently clean (verified in background).

## Replacement mapping (behavior-preserving)

| # | Hand-rolled | mytheclipse replacement | Files touched |
|---|---|---|---|
| 1 | backoff + retry.rs wrapper | `mytheclipse::retry` + `RetryConfig` | src/infrastructure/scraping/retry.rs, parsing_utils.rs, otakudesu.rs |
| 2 | deadpool-redis pool in redis_pool.rs | Keep pool, wrap conn with `RedisCache` (mytheclipse-cache) | redis_pool.rs + new bridge, redis.rs, proxy_fetch.rs |
| 3 | Cache helper (redis.rs) | `CacheAside` + typed JSON serde over `RedisCache` | redis.rs, application/*/use_cases.rs |
| 4 | custom EventBus (events/bus.rs) | `InMemoryEventBus`/`TypedEventBus` (mytheclipse-event) | events/bus.rs → re-export, state.rs, bootstrap |
| 5 | custom RateLimiter middleware | `mytheclipse::RateLimiter` (token bucket) | presentation/middleware/ratelimit.rs |
| 6 | `config` crate loader in config/mod.rs | `mytheclipse-config` ConfigLoader<T> | src/config/mod.rs |
| 7 | HTTP client wrapper (http_client.rs) | `mytheclipse-http` HttpClient (optional) | http_client.rs (SKIP this round — retry semantics differ; reqwest needs headers/UA control) |
| 8 | OTel metrics (metrics.rs) | Keep opentelemetry (mytheclipse-tracing has no metrics exporter; avoid behavior change) | SKIP this round |
| 9 | rayon in komik_parser | `mytheclipse::compute::compute_par_for_each` (feature compute) | komik_parser.rs (SKIP this round — parser correctness risk; rayon works) |

## Scope decision (this round)

Implement items #1–#6. Skip #7–#9 with rationale:
- #7 mytheclipse-http client is a thin reqwest wrapper without header/UA control
  needed by common_headers(); converting scrapers through it changes fetch
  semantics (returns bytes, loses status/content-type) — not behavior-preserving.
- #8 mytheclipse-tracing has no metrics exporter; opentelemetry stays.
- #9 parser logic (2k+ LOC) is out of scope for infra migration; rayon stays.

## Detail per item

### 1. retry.rs → mytheclipse::retry
- Replace `backoff::ExponentialBackoff` with `mytheclipse::{retry, RetryConfig, JitterKind}`.
- Keep signature-compatible helpers so call sites barely change:
  `default_backoff() -> RetryConfig`, `quick_backoff()`, `slow_backoff()`,
  `custom_backoff(...)`.
- `transient/permanent` helpers: mytheclipse retry uses a `predicate` closure
  `Fn(&E) -> bool`. Replace transient/permanent with a retryable predicate
  (retry on any error except a marker). To preserve "transient = retry,
  permanent = stop", use a wrapper type or predicate returning true for all
  errors, and treat 4xx-style permanent errors by converting callers to return
  a `Permanent` variant.
- Keep the `retry` fn name re-exported for minimal call-site churn.

### 2. Redis: mytheclipse-cache RedisCache + bridge
- Add `mytheclipse-cache = { version = "1.20", features = ["l2-redis", "cache-aside"] }`.
- Keep deadpool pool (mytheclipse has no pool); obtain
  `redis::aio::MultiplexedConnection` from deadpool conn (deadpool_redis::Connection
  derefs to `&mut redis::aio::ConnectionLike` — need to convert via `into_multiplexed()`).
- New bridge file `src/infrastructure/cache/mytheclipse.rs`:
  `pub fn redis_cache() -> &'static mytheclipse_cache::RedisCache` building
  from the pool lazily (LazyLock).

### 3. Cache helper → CacheAside
- Rewrite `src/infrastructure/cache/redis.rs` as a thin typed wrapper over
  `RedisCache` implementing `get/get_or_set/set/set_with_ttl/delete/exists`
  with serde_json, so use_cases keep the same ergonomic API (minimal churn)
  but delegate to mytheclipse `Cache` trait underneath.
- `get_or_set` becomes CacheAside-equivalent (read-through).

### 4. events/bus.rs → mytheclipse-event
- Replace custom Event/EventHandler/pub-sub with
  `TypedEventBus<InMemoryEventBus>`.
- Define domain events as serde structs implementing `mytheclipse_event::Event`
  (which is a blanket trait on Serialize+DeserializeOwned types).
- `EventBus` type alias: `pub type EventBus = TypedEventBus<InMemoryEventBus>`.
- state.rs + bootstrap keep `Arc<EventBus>`; `new()` -> `EventBus::new(InMemoryEventBus::default())`.
- Publish/subscribe usage sites (none currently) adapt if any.

### 5. Ratelimit middleware → mytheclipse::RateLimiter
- Keep middleware shape (axum State) but inner limiter becomes
  `mytheclipse::RateLimiter` token bucket: `RateLimiter::new(rate_per_sec, burst)`.
- `check()` -> `try_acquire()` returns Result<_, RateLimitError>.

### 6. config/mod.rs → mytheclipse-config
- Replace `config::Config::builder()` with `mytheclipse_config::ConfigLoader<AppConfig>`.
- Sources: load_dotenv (env feature) → merge_file(config/default.toml) →
  merge_file(config/{RUN_MODE}.toml) → merge_env (APP__ prefix + legacy env
  var mapping preserved).
- Keep `CONFIG` LazyLock and the same `AppConfig` struct (`Deserialize` +
  `mytheclipse_config::Config` blanket trait).

## Schema/type changes
- `deadpool_redis::Pool` still in AppState (no change) — RedisCache wraps conns.
- `AppState.event_bus` becomes `Arc<TypedEventBus<InMemoryEventBus>>`.
- Error mapping: add `From<mytheclipse_cache::CacheError>` for AppError +
  DomainError? Keep the existing string-based paths; add explicit From impls
  where needed.

## Backend surface
- No HTTP route changes. No API contract changes.
- Redis cache keys/TTLs unchanged. Event topics unchanged (none active).

## Frontend surface
- None (backend-only service).

## Verification steps
1. `cargo check` — exit 0
2. `cargo test` — all pass
3. `cargo clippy -- -D warnings` — 0 warnings (project lint gate)
4. `cargo fmt --check` — clean
5. `cargo build --release` — succeeds
6. Smoke: boot server briefly if feasible (needs Redis/DB; else compile-only + unit tests)

## Risk register
- deadpool Connection → MultiplexedConnection conversion API differences
  (0.22 vs 0.27 redis). Verify at compile time; fallback = keep raw conn in
  Cache impl.
- `mytheclipse::retry` predicate-based vs backoff transient/permanent — call
  sites that relied on permanent-stop need review (repository fetch_html uses
  `transient()` on everything — safe to retry-all).
- ConfigLoader merge_env nesting: existing env var mapping uses `APP__` prefix
  + legacy names. Must keep exact names (APP_DATABASE_URL etc.) — verify with
  `peek()` / unit test on the real struct.
- EventBus trait object: `Arc<TypedEventBus<InMemoryEventBus>>` is concrete —
  no trait object; AppState carries the concrete type to avoid dyn issues.

## Migration order (batches, each verified)
Batch A: Cargo.toml deps + retry.rs rewrite + parsing_utils/otakudesu call sites
Batch B: redis_pool bridge + Cache rewrite (redis.rs) + proxy_fetch cache calls
Batch C: events/bus.rs + state.rs + bootstrap
Batch D: ratelimit middleware + router wiring
Batch E: config/mod.rs ConfigLoader
Batch F: full verification + commit

Each batch: cargo check + cargo test must stay green. Commit once at end with
message: `refactor: migrate infra to mytheclipse crates (retry, cache, event, ratelimit, config)`