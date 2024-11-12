use geo_types::Geometry;
use sqlx::error::BoxDynError;
use sqlx::postgres::PgTypeInfo;
use sqlx::{Database, Decode, FromRow, Postgres};
use std::io::Cursor;
use wkb::WKBReadExt;

#[derive(Debug)]
pub struct GeometryWkb(pub Geometry);

impl sqlx::Type<Postgres> for GeometryWkb {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("bytea")
    }
}

impl<'r> Decode<'r, Postgres> for GeometryWkb
{
    fn decode(value: <Postgres as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let data = value.as_bytes().expect("Wkb data could not be decoded");
        let mut bytes_cursor = Cursor::new(data);
        let geometry = bytes_cursor.read_wkb().expect("Failed to decode WKB into geometry");
        Ok(GeometryWkb(geometry))
    }
}

#[derive(Debug, FromRow)]
pub struct TileRow {
    #[sqlx(rename = "__internal_geometry_bin__")]
    pub geometry_bin: GeometryWkb,
    #[sqlx(rename = "h3clustercount")]
    pub h3_cluster_count: i64,
    pub properties: Option<serde_json::Value>,
}
