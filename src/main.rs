use axum::http::header::CONTENT_ENCODING;
use axum::http::{HeaderMap, HeaderValue, Method};
use axum::{routing::get, Router};
use rs_dynamic_mvt::cache::cache_provider::CacheProvider;
use rs_dynamic_mvt::config::Config;
use rs_dynamic_mvt::default_header_layer::DefaultHeaderLayer;
use rs_dynamic_mvt::dep::AppState;
use rs_dynamic_mvt::routes::mvt_handler::get_tile;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let config = Config::from_env().unwrap();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut cors = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_headers(Any)
        .max_age(Duration::from_secs(3600));

    if let Some(allowed_origins) = config.allowed_origins.clone() {
        let allowed_origins = allowed_origins
            .split(' ')
            .map(|s| HeaderValue::from_str(s).unwrap());
        cors = cors.allow_origin(AllowOrigin::list(allowed_origins));
    } else {
        cors = cors.allow_origin(Any);
    };

    let cache_provider = CacheProvider::new(config.cache_url.clone());
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&config.database_url)
        .await
        .expect("can't connect to database");

    let mvt_route = Router::new().route("/:x/:y/:z", get(get_tile));

    let mut app = Router::new()
        .nest("/mvt", mvt_route)
        .layer(cors)
        .with_state(AppState {
            pool,
            cache: cache_provider,
            config: config.clone(),
        });

    let disabled_gzip = config.disable_gzip.unwrap_or(false);
    if !disabled_gzip {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(CONTENT_ENCODING, HeaderValue::from_static("gzip"));

        let compression_layer = CompressionLayer::new().gzip(true);
        app = app
            .layer(compression_layer)
            .layer(DefaultHeaderLayer::new(default_headers));
    }

    let listener = TcpListener::bind("127.0.0.1:8095").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
