use std::{fmt, io};


#[derive(PartialEq, Debug)]
pub enum TileError {
    EncodingError(String),
    DatabaseError(String),
    NotFound,
}

impl fmt::Display for TileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
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
            TileError::NotFound => {
                io::Error::new(io::ErrorKind::NotFound, "No results")
            }
        }
    }
}