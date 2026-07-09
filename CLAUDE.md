# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Scraper service — a Rust/Axum backend for web scraping (anime/komik data extraction) and image proxy/CDN caching. Serves as the backend engine consumed by the `apps/solidjs` frontend.

## Commands

```bash
# Development
cargo run                          # Start server (binds 0.0.0.0:4091)
cargo test                         # Run all tests
cargo clippy -- -D warnings        # Lint (warnings are errors)
cargo fmt                          # Auto-format all source files

# Release build (full LTO, single CGU, stripped)
cargo build --release

# PM2 production
pm2 start ecosystem.config.cjs --env production    # Uses target/release/scraper
```

## Architecture

### Modular MVC + Service + Repository

Setiap module mengikuti arsitektur layered yang identik:

```
Request → Router (route.rs) → Controller → Service → Repository → Parser
                                                    │
                                                    ├── Redis (L1 cache)
                                                    ├── SeaORM/MySQL (L2, image_cache)
                                                    └── External HTTP (alqanime.si, picser CDN)
```

### Directory Layout

```
src/
├── main.rs                  # Entry point: builds Application, calls run()
├── lib.rs                   # Public module declarations
├── app.rs                   # Router assembly: modules + metrics + swagger + middleware layers
├── bootstrap/mod.rs         # Application::build(): tracing, Redis, browser pool, DB, AppState
├── modules/                 # Feature modules (vertical slices)
│   ├── anime/               # Otakudesu anime scraper
│   ├── anime2/              # Alqanime.si anime scraper
│   ├── komik/               # Komik scraper
│   └── proxy/               # Image proxy/cache/audit endpoints
└── shared/                  # Cross-cutting infrastructure
    ├── config/              # Lazy-static AppConfig from env vars (fail-fast at startup)
    ├── state/               # AppState (redis_pool, db, semaphore, event_bus)
    ├── database/
    │   ├── traits/          # ScrapingRepository, ImageCacheRepository (async_trait)
    │   ├── repositories/    # SeaOrmImageCacheRepository (impl ImageCacheRepository)
    │   └── persistence/     # SeaORM entities (image_cache)
    ├── services/images/     # ImageCache service + apply_cached_posters helper
    ├── errors/              # AppError enum → axum IntoResponse (500/404 by variant)
    ├── observability/       # Utoipa/Swagger OpenAPI doc
    ├── scheduler/           # Cron jobs (daily cache cleanup at 2 AM)
    ├── browser/             # Headless Chrome pool for JS-rendered scraping
    ├── scrapers/            # Site-specific scrapers (otakudesu)
    ├── utils/               # Cache helper, HTTP client, scraping helpers, retry, conversions
    ├── middlewares/         # Logging, rate limiting
    ├── events/              # EventBus for repair state updates
    └── types/               # ApiResponse<T>, shared entity types (HasPoster trait, Pagination)
```

### Module Structure (identik untuk setiap module)

Setiap `src/modules/<name>/`:

| File | Peran | Pola |
|---|---|---|
| `route.rs` | Daftar endpoint, mapping URL → controller | `Router<Arc<AppState>>`, tidak ada logic |
| `controller.rs` | Extract State/Path/Query/Body, panggil service | `Result<Json<T>, AppError>` |
| `service.rs` | Business logic, caching, delegasi ke repository + parser | Struct dengan repo di-inject via constructor `new(repo: XRepository)` |
| `repository.rs` | HTTP fetching, URL builders, DB queries | Struct + `impl ScrapingRepository` trait |
| `parser.rs` | HTML parsing dengan `scraper` crate | Free functions → `Result<T, AppError>`, via `spawn_blocking` |
| `schema.rs` | Validasi query/path/body params | Struct `Deserialize` + `ToSchema` |
| `types.rs` | Response structs | `Serialize` + `ToSchema`, `impl HasPoster` jika punya poster |

### Dependency Injection

Semua service menerima dependency via constructor:

```rust
// Controller creates and injects dependencies
let repo = AnimeRepository::new();
let service = AnimeService::new(repo);
service.get_anime_index(app_state).await.map(Json)

// Service stores injected repo
pub struct AnimeService {
    repository: AnimeRepository,
}
impl AnimeService {
    pub fn new(repository: AnimeRepository) -> Self { Self { repository } }
}
```

### Image Caching Architecture

Single unified image cache system:

1. **Trait**: `ImageCacheRepository` (`shared/database/traits/image_cache.rs`) — Redis ops, DB ops, locks, cache invalidation
2. **Impl**: `SeaOrmImageCacheRepository` (`shared/database/repositories/image_cache.rs`)
3. **Service**: `ImageCache` struct (`shared/services/images/cache.rs`) — download, MIME-verify dengan `infer`, upload ke Picser CDN, verifikasi CDN URL (10 retry dengan backoff)
4. **Concurrency**: `Semaphore` (default 5 concurrent uploads) + request coalescing via `DashMap<broadcast::Sender>`
5. **Lazy batch helper**: `cache_image_urls_batch_lazy()` — Redis batch check → DB batch check → background spawn untuk misses

### HasPoster Trait & apply_cached_posters

`HasPoster` trait di `shared/types/entities/anime.rs` memungkinkan generic poster caching:

```rust
pub trait HasPoster {
    fn poster(&self) -> &str;
    fn set_poster(&mut self, url: String);
}
```

Semua item type dengan field `poster` mengimplementasikan trait ini (`OngoingAnimeItem`, `KomikItem`, `FilterAnimeItem`, `Recommendation`, dll).

`apply_cached_posters()` di `shared/services/images/cache.rs` menerima `&mut [T]` where `T: HasPoster`, menggantikan pola manual ~15 baris yang sebelumnya berulang di setiap service method.

### ScrapingRepository Trait

```rust
#[async_trait]
pub trait ScrapingRepository: Send + Sync {
    async fn fetch_html(&self, url: &str) -> Result<String, AppError>;
}
```

Semua module repository (`AnimeRepository`, `Anime2Repository`, `KomikRepository`, `ProxyRepository`) mengimplementasikan trait ini.

### Error Handling

`AppError` enum di `src/shared/errors/app_error.rs` — derives `thiserror::Error` dan implements `IntoResponse` (404 untuk `NotFound`, 500 untuk lainnya).

**Kontrak error per layer:**
- **Parser** → `Result<T, AppError>`
- **Repository** → `Result<T, AppError>` (via `ScrapingRepository` trait)
- **Service** → `Result<T, AppError>` (tidak ada `Result<T, String>` atau `Box<dyn Error>`)
- **Controller** → `Result<Json<T>, AppError>` (kecuali proxy yang return raw `Response`)

### Configuration

`src/shared/config/mod.rs` — global `CONFIG` lazy-static loaded from:
1. `.env` file (dotenvy)
2. `config/default.toml` / `config/{RUN_MODE}.toml`
3. Environment variables (`APP__` prefix or legacy `DATABASE_URL`/`JWT_SECRET`/`REDIS_URL`)

Panics at startup if required config is missing — intentional fail-fast design.

## Constraints

- **No suppression flags**: `#[allow(...)]`, `#[ignore]`, `@ts-ignore` are prohibited. Fix the underlying issue.
- **Lint strictness**: `unsafe_code = "forbid"`, `panic = "deny"`, `todo = "deny"`, `unimplemented = "deny"`, `unwrap_used = "warn"`, `expect_used = "warn"`
- **Minimal dependencies**: Before adding a crate, evaluate if existing deps or std can handle it.
- **Dead code**: Remove unused functions, types, modules rather than leaving them.
- **Performance**: Use `spawn_blocking` for CPU-heavy work (HTML parsing).
- **No duplicate infrastructure**: Satu trait, satu impl. Jangan membuat trait/repository duplikat seperti `ImageRepository` dan `ImageCacheRepository` yang berbeda.
- **No thin wrappers**: Hindari wrapper tipis seperti `CacheImageUseCase` yang hanya meneruskan panggilan ke service lain.

## Useful Endpoints

- `GET /docs` — Swagger UI
- `GET /api-docs/openapi.json` — OpenAPI spec
- `POST /api/proxy/image-cache` — Cache an image URL
- `POST /api/proxy/image-cache/audit` — Audit/repair cached images
- `GET /api/anime/*` — Otakudesu scraping endpoints
- `GET /api/anime2/*` — Alqanime scraping endpoints
- `GET /api/komik/*` — Komik scraping endpoints
