use geo_types::{Coord, Point};
use h3o::Resolution;

pub fn translate_zoom_to_h3_resolution(z: u32) -> u32 {
    if z >= 15 {
        return Resolution::Fifteen as u32;
    }

    let resolution = ((1.8 / 3.0) * z as f64 + 2.0).floor() as u32;
    resolution.min(Resolution::Fifteen as u32)
}

pub struct BBox {
    pub min: Point,
    pub max: Point,
}

pub fn get_max_tiles_from_zoom(zoom: u32) -> f64 {
    (1 << zoom) as f64
}

pub fn mercator_to_tile(longitude: f64, latitude: f64, zoom_level: u32) -> Point {
    let latitude_radians = latitude.to_radians();
    let n = u32::pow(2, zoom_level) as f64;
    let x = n * ((longitude + 180.0) / 360.0);
    let y = n * (1.0 - f64::log(f64::tan(latitude_radians) + 1.0 / f64::cos(latitude_radians), std::f64::consts::E) / std::f64::consts::PI) / 2.0;
    Point::new(x, y)
}

pub fn to_point(x: f64, y: f64, z: u32) -> (f64, f64) {
    let max_tiles = get_max_tiles_from_zoom(z);

    let pi = std::f64::consts::PI;
    let longitude = 360.0 * (x / max_tiles - 0.5);
    let latitude = 2.0 * f64::atan(
        f64::exp(
            pi - (2.0 * pi) * (y / max_tiles)
        )
    ) * (180.0 / pi) - 90.0;

    (longitude, latitude)
}

pub fn get_bounding_box_from_tile(x: u32, y: u32, z: u32) -> BBox {
    let x = x as f64;
    let y = y as f64;

    let min_x = x;
    let min_y = y.max(0.0);
    let max_x = x + 1.0;
    let max_y = (y + 1.0).min(get_max_tiles_from_zoom(z));

    let (min_longitude, max_latitude) = to_point(min_x, min_y, z);
    let (max_longitude, min_latitude) = to_point(max_x, max_y, z);

    BBox {
        min: Point(Coord {
            x: min_longitude,
            y: min_latitude,
        }),
        max: Point(Coord {
            x: max_longitude,
            y: max_latitude,
        }),
    }
}
