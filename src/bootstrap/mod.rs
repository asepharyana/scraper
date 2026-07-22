pub mod setup;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

use axum::Router;
use sea_orm::Database;
use tracing_subscriber::EnvFilter;

use crate::config::CONFIG;
use crate::infrastructure::cache::redis_pool::get_redis_conn;
use crate::presentation::state::AppState;

pub struct Application {
    pub port: u16,
    router: Router,
    listener: TcpListener,
}

impl Application {
    pub async fn build() -> anyhow::Result<Self> {
        // Initialize tracing. Default to warn/error globally unless RUST_LOG is explicitly set.
        let env_filter = match std::env::var("RUST_LOG") {
            Ok(filter) => EnvFilter::new(filter).add_directive("html5ever=error".parse()?),
            Err(_) => EnvFilter::new("warn,html5ever=error"),
        };

        tracing_subscriber::fmt().with_env_filter(env_filter).init();

        // Initialize OpenTelemetry metrics
        crate::observability::metrics::init_otel_metrics();

        tracing::info!("🚀 Scraper starting up...");
        tracing::info!("   Environment: {}", CONFIG.environment);

        // Log thread configuration
        let worker_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        tracing::info!(
            "   Tokio Worker Threads: (Defaulting to CPU cores: {})",
            worker_threads
        );

        // Redis
        let _ = get_redis_conn().await;

        // Database
        let mut opt = sea_orm::ConnectOptions::new(CONFIG.database_url.clone());
        opt.max_connections(20)
            .min_connections(1)
            .connect_timeout(std::time::Duration::from_secs(
                CONFIG.db.connect_timeout_seconds,
            ))
            .idle_timeout(std::time::Duration::from_secs(
                CONFIG.db.idle_timeout_seconds,
            ))
            .acquire_timeout(std::time::Duration::from_secs(
                CONFIG.db.acquire_timeout_seconds,
            ))
            .max_lifetime(std::time::Duration::from_secs(
                CONFIG.db.max_lifetime_seconds,
            ))
            .sqlx_logging(CONFIG.log_level == "debug")
            .map_sqlx_postgres_opts(|opts| opts.extra_float_digits(None));

        let db = Database::connect(opt)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;
        tracing::info!("✓ SeaORM database connection established");

        // Schema & Seeding
        if let Err(e) = crate::bootstrap::setup::init(&db).await {
            tracing::error!("Failed to init DB schema: {}", e);
        }

        // App State components
        let db_arc = Arc::new(db);
        let event_bus = Arc::new(crate::events::bus::EventBus::new());

        let redis_pool = crate::infrastructure::cache::redis_pool::redis_pool()
            .map_err(|e| anyhow::anyhow!("Failed to init Redis pool: {}", e))?;

        let app_state = Arc::new(AppState {
            redis_pool,
            db: db_arc.clone(),
            event_bus: event_bus.clone(),
        });

        let app = crate::presentation::router::build_router(app_state.clone())?;

        // Listener
        let port = CONFIG.server_port;
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = TcpListener::bind(&addr).await?;
        tracing::info!("Server listening on {}", listener.local_addr()?);

        Ok(Self {
            port,
            router: app,
            listener,
        })
    }

    pub async fn run(self) -> std::io::Result<()> {
        axum::serve(self.listener, self.router.into_make_service()).await
    }
}
