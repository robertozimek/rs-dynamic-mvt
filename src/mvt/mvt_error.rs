use std::fmt;

#[derive(Debug, Clone)]
pub struct BinaryTileError;

impl fmt::Display for BinaryTileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to encode tile binary")
    }
}