use crate::geo::geo::{get_bounding_box_from_tile, translate_zoom_to_h3_resolution};
use h3o::Resolution;
use indoc::indoc;

pub fn get_tile_query(
    x: u32,
    y: u32,
    z: u32,
    query: &str,
    geo_col: &str,
    srid: &str,
) -> String {
    let bbox = get_bounding_box_from_tile(x, y, z);

    let envelope = format!(
        "ST_MakeEnvelope({min_x:.8}, {min_y:.8}, {max_x:.8}, {max_y:.8}, {srid})",
        min_x = bbox.min.x(),
        min_y = bbox.min.y(),
        max_x = bbox.max.x(),
        max_y = bbox.max.y(),
        srid = srid
    );

    let h3_resolution = translate_zoom_to_h3_resolution(z);

    let mut raw_query = format!(indoc! {r#"
        WITH geometry_type AS (
            SELECT
                row_to_json(t) as properties,
                t.{geo_col},
                ST_GeometryType({geo_col}) as __internal_geometry_type__,
                ROUND(0.7 / (2 ^ {zoom})::numeric, 3) as __internal_geometry_simplify__
            FROM ({query}) t
            WHERE
                ST_INTERSECTS({bbox}, {geo_col})
        ),
        setup AS (
            SELECT
                t.*,
                CASE
                    WHEN
                        __internal_geometry_type__ = 'ST_GeometryCollection'
                    THEN
                        ST_CollectionExtract(ST_Simplify({geo_col}, 0.7 / (2 ^ {zoom}), true))
                    WHEN
                        __internal_geometry_type__ = 'ST_Point'
                    THEN
                        {geo_col}
                    ELSE
                        ST_Simplify({geo_col}, t.__internal_geometry_simplify__, true)
                END as __internal_geometry_mapped__,
                CAST(1 as int8) as h3ClusterCount
            FROM geometry_type t
		) SELECT *, ST_AsBinary(__internal_geometry_mapped__) as __internal_geometry_bin__ FROM setup
    "#}, query = query, bbox = envelope, geo_col = geo_col, zoom = z);

    if h3_resolution < Resolution::Fifteen as u32 {
        raw_query = format!(indoc! {r#"
        WITH geometry_type AS (
				SELECT
					row_to_json(t) as properties,
					t.{geo_col},
					ST_GeometryType({geo_col}) as __internal_geometry_type__,
					ROUND(0.7 / (2 ^ {zoom})::numeric, 3) as __internal_geometry_simplify__
				FROM ({query}) t
				WHERE
					ST_INTERSECTS({bbox}, {geo_col})
			), setup AS (
				SELECT
					t.*,
					CASE
						WHEN
							__internal_geometry_type__ = 'ST_GeometryCollection'
						THEN
							ST_CollectionExtract(ST_Simplify({geo_col}, 0.7 / (2 ^ {zoom}), true))
						WHEN
							__internal_geometry_type__ = 'ST_Point'
						THEN
							{geo_col}
						ELSE
							ST_Simplify({geo_col}, t.__internal_geometry_simplify__, true)
					END as __internal_geometry_mapped__
				FROM geometry_type t
			), shapes AS (
				SELECT
					CAST('1' as h3index) as __internal_h3_index__,
					*,
					CAST(1 as int8) as h3ClusterCount
				FROM setup
				WHERE __internal_geometry_type__ <> 'ST_Point' AND __internal_geometry_mapped__ IS NOT NULL
			), points AS (
				(WITH data AS (
					SELECT  * FROM setup WHERE __internal_geometry_type__ = 'ST_Point'
				), indexed AS (
					SELECT h3_lat_lng_to_cell(CAST({geo_col} as point), {h3_resolution}) as __internal_h3_index__, * FROM data
				), counted_index AS (
					SELECT *, row_number() over (partition by __internal_h3_index__ ORDER BY __internal_h3_index__ DESC) as h3ClusterCount FROM indexed
				)
				SELECT
					distinct on(ci.__internal_h3_index__) ci.*
				FROM counted_index ci
				ORDER BY ci.__internal_h3_index__, ci.h3ClusterCount DESC)
			)
			SELECT
			    *,
			    ST_AsBinary(__internal_geometry_mapped__) as __internal_geometry_bin__
            FROM shapes
            UNION ALL
            SELECT
                *,
                ST_AsBinary(__internal_geometry_mapped__) as __internal_geometry_bin__
            FROM points
        "#}, query = query, bbox = envelope, geo_col = geo_col, zoom = z, h3_resolution = h3_resolution);
    }

    raw_query
}