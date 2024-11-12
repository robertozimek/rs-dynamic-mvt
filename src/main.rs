use axum::{
    routing::get, Router,
};
use dotenv::dotenv;
use rs_dynamic_mvt::cache::cache_provider::CacheProvider;
use rs_dynamic_mvt::routes::dep::AppStateGeneric;
use rs_dynamic_mvt::routes::router_handlers::get_tile;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any)
        .max_age(Duration::from_secs(3600));

    let cache_url = if let Ok(cache_url) = env::var("CACHE_URL") {
        Some(cache_url)
    } else {
        None
    };

    let cache_provider = CacheProvider::new(cache_url);

    let db_url = env::var("DATABASE_URL").unwrap();
    // set up connection pool
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_url)
        .await
        .expect("can't connect to database");

    let mvt_route = Router::new()
        .route("/:x/:y/:z", get(get_tile));

    let app = Router::new()
        .nest("/mvt", mvt_route)
        .layer(cors)
        .with_state(AppStateGeneric {
            pool: pool,
            cache: cache_provider,
        });

    let listener = TcpListener::bind("127.0.0.1:8095").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

