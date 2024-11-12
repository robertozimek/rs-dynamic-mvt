use crate::cache::cache_provider::CacheProvider;
use crate::config::Config;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub cache: CacheProvider,
    pub config: Config,
}