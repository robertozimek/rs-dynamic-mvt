use crate::geo::geo_utils::mercator_to_tile;
use crate::mvt::mapbox_vector_tile::Coordinates;
use crate::protos::vector_tile::tile::GeomType;
use geo_types::{Coord, Geometry, LineString, Point, Polygon};

const MOVE_TO: usize = 1;
const LINE_TO: usize = 2;
const CLOSE_PATH: usize = 7;

pub struct EncoderPoint {
    pub x: i32,
    pub y: i32,
}

pub struct TileProjection {
    zoom_level: u32,
    min_point: Point,
}

impl TileProjection {
    pub fn new(coordinates: &Coordinates, extent: u32) -> TileProjection {
        let n = extent.trailing_zeros();
        let z = coordinates.z + n;
        let min_x = ((coordinates.x as u64) << n) as f64;
        let min_y = ((coordinates.y as u64) << n) as f64;

        TileProjection {
            min_point: Point(Coord { x: min_x, y: min_y }),
            zoom_level: z,
        }
    }

    pub fn project_point(&self, point: &Point) -> Point {
        let point = mercator_to_tile(point.0.x, point.0.y, self.zoom_level);
        Point(Coord {
            x: f64::floor(point.0.x - self.min_point.0.x),
            y: f64::floor(point.0.y - self.min_point.0.y),
        })
    }
}

pub struct GeometryCommandEncoder<'a> {
    point_projection: &'a TileProjection,
    prev_point: EncoderPoint,
    pub data: Vec<u32>,
}

pub trait FromGeometry {
    fn from_geometry_with_projection(
        geom: &Geometry,
        point_projection: &TileProjection,
    ) -> Result<GeometryData, String>;
}

impl<'a> GeometryCommandEncoder<'a> {
    fn new(point_projection: &'a TileProjection) -> GeometryCommandEncoder<'a> {
        GeometryCommandEncoder {
            point_projection,
            prev_point: EncoderPoint { x: 0, y: 0 },
            data: vec![],
        }
    }

    fn move_to(&mut self, points: &[Point]) {
        let size = points.len();
        self.data.push(((MOVE_TO & 0x7) | (size << 3)) as u32);
        self.push_points(points);
    }

    fn line_to(&mut self, points: &[Point]) {
        let size = points.len();
        self.data.push(((LINE_TO & 0x7) | (size << 3)) as u32);
        self.push_points(points);
    }

    fn close_path(&mut self) {
        self.data.push(((CLOSE_PATH & 0x7) | (1 << 3)) as u32)
    }

    fn push_points(&mut self, points: &[Point]) {
        for point in points.iter() {
            let point = self.point_projection.project_point(point);

            let x = point.0.x as i32 - self.prev_point.x;
            let y = point.0.y as i32 - self.prev_point.y;

            self.prev_point.x = point.0.x as i32;
            self.prev_point.y = point.0.y as i32;

            self.data.push(((x << 1) ^ (x >> 31)) as u32);
            self.data.push(((y << 1) ^ (y >> 31)) as u32);
        }
    }
}

pub struct GeometryData {
    pub geometry_type: GeomType,
    pub geometry: Vec<u32>,
}

impl<'a> FromGeometry for GeometryCommandEncoder<'a> {
    fn from_geometry_with_projection(
        geom: &Geometry,
        point_projection: &TileProjection,
    ) -> Result<GeometryData, String> {
        let add_line = |encoder: &mut GeometryCommandEncoder, line_string: &LineString| {
            let points = line_string.points().collect::<Vec<Point>>();
            let point = points.first().unwrap();
            encoder.move_to(&[*point]);

            if line_string.is_closed() {
                let size = points.len() - 1;
                encoder.line_to(&points[1..size]);
            } else {
                encoder.line_to(&points[1..]);
            }
            encoder.close_path();
        };

        let add_polygon = |encoder: &mut GeometryCommandEncoder, polygon: &Polygon| {
            let lines: Vec<LineString> = [
                vec![polygon.exterior().clone()],
                polygon.interiors().to_vec(),
            ]
            .concat();

            for line_string in lines.iter() {
                add_line(encoder, line_string);
            }
        };

        match geom {
            Geometry::Point(point) => {
                let mut encoder = GeometryCommandEncoder::new(point_projection);
                let points = [*point];
                encoder.move_to(&points);

                Ok(GeometryData {
                    geometry_type: GeomType::POINT,
                    geometry: encoder.data,
                })
            }
            Geometry::LineString(line_string) => {
                let mut encoder = GeometryCommandEncoder::new(point_projection);
                let points = line_string.points().collect::<Vec<Point>>();
                encoder.move_to(&[points[0]]);
                encoder.line_to(&points[1..]);

                Ok(GeometryData {
                    geometry_type: GeomType::LINESTRING,
                    geometry: encoder.data,
                })
            }
            Geometry::MultiLineString(multi_line_string) => {
                let mut encoder = GeometryCommandEncoder::new(point_projection);
                for line_string in multi_line_string.iter() {
                    let points = line_string.points().collect::<Vec<Point>>();
                    encoder.move_to(&[points[0]]);
                    encoder.line_to(&points[1..]);
                }
                Ok(GeometryData {
                    geometry_type: GeomType::LINESTRING,
                    geometry: encoder.data,
                })
            }
            Geometry::Polygon(polygon) => {
                let mut encoder = GeometryCommandEncoder::new(point_projection);
                add_polygon(&mut encoder, polygon);

                Ok(GeometryData {
                    geometry_type: GeomType::POLYGON,
                    geometry: encoder.data,
                })
            }
            Geometry::MultiPolygon(multi_polygon) => {
                let mut encoder = GeometryCommandEncoder::new(point_projection);

                for polygon in multi_polygon.iter() {
                    add_polygon(&mut encoder, polygon);
                }

                Ok(GeometryData {
                    geometry_type: GeomType::POLYGON,
                    geometry: encoder.data,
                })
            }
            Geometry::MultiPoint(multi_point) => {
                let mut encoder = GeometryCommandEncoder::new(point_projection);
                encoder.move_to(&multi_point.0);
                Ok(GeometryData {
                    geometry_type: GeomType::POINT,
                    geometry: encoder.data,
                })
            }
            _ => Err("Unsupported geometry type".to_string()),
        }
    }
}
