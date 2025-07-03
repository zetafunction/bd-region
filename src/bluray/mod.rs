use clap::ValueEnum;
use std::fs::File;
use std::path::{Path, PathBuf};
use thiserror::Error;

const MOVIE_OBJECT_PATH: &str = "BDMV/MovieObject.bdmv";

/// Blu-Ray media region codes
#[derive(Clone, Copy, ValueEnum)]
pub enum Region {
    /// North America, South America, U.S. Territories, Japan, South Korea, Taiwan, and other areas of
    /// Southeast Asia.
    A,
    /// Europe, Africa, Middle East, Australia, and New Zealand.
    B,
    /// Asia (except for Japan, Korea, Taiwan, and other areas of Southeast Asia)
    C,
}

pub struct BluRay {
    path: PathBuf,
    movie_object: File,
}

#[derive(Debug, Error)]
pub enum OpenError {
    #[error("IO error for {0}")]
    IoError(&'static str, #[source] std::io::Error),
}

impl BluRay {
    pub fn open(path: &Path) -> Result<BluRay, OpenError> {
        let movie_object = File::open(path.join(MOVIE_OBJECT_PATH))
            .map_err(|e| OpenError::IoError(MOVIE_OBJECT_PATH, e))?;
        Ok(BluRay {
            path: path.to_path_buf(),
            movie_object,
        })
    }
}
