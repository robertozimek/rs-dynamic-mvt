use crate::mvt::constants::DEFAULT_EXTENT;
use crate::mvt::geometry_command_encoder::{FromGeometry, GeometryCommandEncoder, TileProjection};
use crate::mvt::mvt_error::BinaryTileError;
use crate::protos::vector_tile::tile::{Feature as ProtoFeature, Layer as ProtoLayer, Value};
use crate::protos::vector_tile::Tile;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use geo_types::Geometry;
use protobuf::{CodedOutputStream, Message};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Feature {
    pub geometry: Geometry,
    pub properties: serde_json::Value,
}

pub struct Coordinates {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

pub struct MapboxLayer {
    keys: Arc<Mutex<Vec<String>>>,
    values: Arc<Mutex<Vec<Value>>>,
    layer: ProtoLayer,
    tile_projector: Arc<TileProjection>,
}

pub fn get_key_index(keys: Arc<Mutex<Vec<String>>>, key: &str) -> usize {
    let mut keys = keys.lock().unwrap();
    let position = keys.iter().position(|x| *x == key);

    if let Some(index) = position {
        index
    } else {
        keys.push(key.to_string());
        keys.len() - 1
    }
}

pub fn get_value_index(values: Arc<Mutex<Vec<Value>>>, value: Value) -> usize {
    let mut values = values.lock().unwrap();
    let position = values.iter().position(|v| *v == value);

    if let Some(index) = position {
        index
    } else {
        values.push(value);
        values.len() - 1
    }
}

fn add_properties(keys: Arc<Mutex<Vec<String>>>, values: Arc<Mutex<Vec<Value>>>, feature: &mut ProtoFeature, properties: &serde_json::Value) {
    if let serde_json::Value::Object(properties) = properties {
        for (key, property) in properties {
            add_property(keys.clone(), values.clone(), feature, key.as_str(), property);
        }
    }
}

fn add_property(keys: Arc<Mutex<Vec<String>>>, values: Arc<Mutex<Vec<Value>>>, feature: &mut ProtoFeature, key: &str, property: &serde_json::Value) {
    let mut value = Value::new();
    match property {
        serde_json::Value::Null => {}
        serde_json::Value::Bool(val) => {
            value.set_bool_value(*val);
        }
        serde_json::Value::Number(val) => {
            if val.is_i64() {
                value.set_int_value(val.as_i64().unwrap());
            } else if val.is_f64() {
                value.set_double_value(val.as_f64().unwrap());
            } else if val.is_u64() {
                value.set_uint_value(val.as_u64().unwrap());
            }
        }
        serde_json::Value::String(val) => {
            value.set_string_value(val.to_string());
        }
        _ => {}
    }

    let key_index = get_key_index(keys, key);
    feature.tags.push(key_index as u32);
    let value_index = get_value_index(values, value);
    feature.tags.push(value_index as u32);
}

pub async fn add_feature(tile_projector: Arc<TileProjection>, keys: Arc<Mutex<Vec<String>>>, values: Arc<Mutex<Vec<Value>>>, feature: &Feature) -> Option<ProtoFeature> {
    let mut proto_feature = ProtoFeature::new();

    if let Geometry::GeometryCollection(_) = &feature.geometry {
        return None;
    }

    let result = GeometryCommandEncoder::from_geometry_with_projection(
        &feature.geometry,
        &tile_projector,
    );

    match result {
        Ok(geometry_data) => {
            proto_feature.set_type(geometry_data.geometry_type);
            proto_feature.geometry = geometry_data.geometry;
            add_properties(keys.clone(), values.clone(), &mut proto_feature, &feature.properties);
            Some(proto_feature)
        }
        Err(err) => {
            println!("{:?} {:?}", feature, err);
            None
        }
    }
}

impl MapboxLayer {
    pub fn new(name: String, tile_projector: TileProjection) -> MapboxLayer {
        let mut layer = ProtoLayer::new();
        layer.name = Some(name);
        layer.version = Some(1);
        layer.extent = Some(DEFAULT_EXTENT);
        Self {
            layer,
            tile_projector: Arc::new(tile_projector),
            keys: Arc::new(Mutex::new(vec![])),
            values: Arc::new(Mutex::new(vec![])),
        }
    }

    pub async fn add_features(&mut self, features: &[Feature]) {
        let mut proto_features: Vec<ProtoFeature> = Vec::new();

        let mut tasks = FuturesUnordered::new();

        for feature in features.iter() {
            let tile_projector = self.tile_projector.clone();
            let keys = self.keys.clone();
            let values = self.values.clone();

            tasks.push(add_feature(tile_projector, keys, values, feature));
        }

        while let Some(Some(proto_feature)) = tasks.next().await {
            proto_features.push(proto_feature);
        }

        self.layer.features = proto_features;

        let keys = self.keys.lock().unwrap();
        let values = self.values.lock().unwrap();
        self.layer.keys = keys.clone();
        self.layer.values = values.clone();
    }

    pub fn get_layer(&self) -> ProtoLayer {
        self.layer.clone()
    }
}

pub struct MapboxVectorTile {
    tile: Tile,
}

impl MapboxVectorTile {
    pub async fn new(coordinates: &Coordinates, layers: &HashMap<String, Vec<Feature>>) -> MapboxVectorTile {
        let mut tile = Tile::new();

        for (name, features) in layers.iter() {
            let mut mapbox_layer = MapboxLayer::new(
                name.to_string(),
                TileProjection::new(coordinates, DEFAULT_EXTENT),
            );
            mapbox_layer.add_features(features).await;
            tile.layers.push(mapbox_layer.get_layer());
        }

        MapboxVectorTile {
            tile,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, BinaryTileError> {
        let mut v: Vec<u8> = Vec::with_capacity(self.tile.compute_size() as usize);
        if self.write_to(&mut v).is_err() {
            return Err(BinaryTileError);
        }
        Ok(v)
    }

    fn write_to(&self, mut out: &mut Vec<u8>) -> Result<(), BinaryTileError> {
        let mut coded_output_stream = CodedOutputStream::new(&mut out);
        if self.tile.write_to(&mut coded_output_stream).is_err() {
            return Err(BinaryTileError);
        }

        if coded_output_stream.flush().is_err() {
            return Err(BinaryTileError);
        }
        Ok(())
    }
}