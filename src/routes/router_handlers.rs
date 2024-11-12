use crate::cache::cache_provider::CacheProvider;
use crate::routes::dep::AppStateGeneric;
use crate::tiling::tile_service::TileService;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn default_srid() -> String {
    "4326".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MVTCoordinates {
    x: u32,
    y: u32,
    z: u32,
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct MVTQuery {
    query: String,
    #[serde(alias = "geoCol")]
    geo_col: String,
    #[serde(default = "default_srid")]
    srid: String,
}

struct MVTBody(Bytes);

impl IntoResponse for MVTBody {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/protobuf")
            .body(Body::from(self.0))
            .unwrap()
    }
}

fn calculate_hash(t: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(t);
    let digest = hasher.finalize();
    format!("{:x}", digest)
}

fn get_cache_key(coordinates: MVTCoordinates, query: MVTQuery) -> String {
    let as_string = format!("{:?}{:?}", coordinates, query);
    calculate_hash(&as_string)
}

pub async fn get_tile(State(mut state): State<AppStateGeneric<PgPool, CacheProvider>>, Path(params): Path<MVTCoordinates>, Query(query): Query<MVTQuery>) -> impl IntoResponse {
    let cache_key = get_cache_key(params.clone(), query.clone());

    if let Some(value) = state.cache.get_bytes(&cache_key) {
        let bytes = Bytes::from(value);
        return MVTBody(Bytes::from(bytes)).into_response();
    }

    let tile_service = TileService::new(&state.pool);
    let result = tile_service.get_tile(params.x, params.y, params.z, &query.query, &query.geo_col, &query.srid).await;

    if let Ok(bytes) = result {
        state.cache.set(&cache_key, &bytes);
        MVTBody(Bytes::from(bytes)).into_response()
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()
    }
}