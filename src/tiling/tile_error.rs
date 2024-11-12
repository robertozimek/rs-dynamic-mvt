use std::error::Error;
use std::fmt::Formatter;
use std::{fmt, io};

#[derive(PartialEq, Debug)]
pub enum TileError {
    EncodingError(String),
    DatabaseError(String),
    NotFound,
}

impl Error for TileError {}

impl fmt::Display for TileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "failed to generate tile")
    }
}

impl From<TileError> for io::Error {
    fn from(err: TileError) -> io::Error {
        match err {
            TileError::EncodingError(message) => {
                io::Error::new(io::ErrorKind::InvalidInput, message)
            }
            TileError::DatabaseError(message) => {
                io::Error::new(io::ErrorKind::InvalidData, message)
            }
            TileError::NotFound => io::Error::new(io::ErrorKind::NotFound, "No results"),
        }
    }
}
