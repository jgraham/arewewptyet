use csv;
use serde_json;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Csv(#[from] csv::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
