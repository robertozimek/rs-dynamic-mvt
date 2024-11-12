use crate::db::db_types::TileRow;
use crate::mvt::mapbox_vector_tile::{Coordinates, Feature, MapboxVectorTile};
use crate::tiling::tile_error::TileError;
use crate::tiling::tile_query_constructor::get_tile_query;
use serde_json::Value;
use sqlx::PgPool;
use std::collections::HashMap;

pub struct TileService<'a> {
    pool: &'a PgPool,
}

impl<'a> TileService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_tile(
        &self,
        x: u32,
        y: u32,
        z: u32,
        query: &str,
        geo_col: &str,
        srid: &str,
    ) -> Result<Vec<u8>, TileError> {
        let raw_query = get_tile_query(x, y, z, query, geo_col, srid);

        let query_results = sqlx::query_as::<_, TileRow>(&raw_query).fetch_all(self.pool).await;
        if let Err(error) = query_results {
            return Err(TileError::DatabaseError(error.to_string()));
        }

        let rows = query_results.unwrap();
        let mut features: Vec<Feature> = vec![];
        for tile_row in rows {
            if let Some(Value::Object(mut properties)) = tile_row.properties {
                if properties.contains_key(geo_col) {
                    properties.remove(geo_col);
                }

                let h3_cluster_count = serde_json::Number::from(tile_row.h3_cluster_count);
                properties.insert("h3ClusterCount".to_string(), Value::Number(h3_cluster_count));

                let feature = Feature {
                    geometry: tile_row.geometry_bin.0,
                    properties: Value::Object(properties),
                };

                features.push(feature);
            }
        }

        let mut layer_map: HashMap<String, Vec<Feature>> = HashMap::new();
        layer_map.insert("default".to_string(), features);

        let tile = MapboxVectorTile::new(&Coordinates {
            x,
            y,
            z,
        }, &layer_map).await;

        match tile.to_bytes() {
            Ok(bytes) => {
                Ok(bytes)
            }
            Err(error) => {
                Err(TileError::EncodingError(error.to_string()))
            }
        }
    }
}