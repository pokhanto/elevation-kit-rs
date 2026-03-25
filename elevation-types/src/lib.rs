use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

//TODO: reorganize types to separate modules
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Elevation(pub f64);

#[derive(Debug, thiserror::Error)]
pub enum MetadataStorageError {
    #[error("Failed to prepare metadata storage")]
    PrepareStorage,

    #[error("Failed to save metadata")]
    Save,

    #[error("Failed to load metadata")]
    Load,

    #[error("Metadata with Id already exists")]
    DuplicateId,
}

pub trait MetadataStorage {
    fn save_metadata(&self, metadata: DatasetMetadata) -> Result<(), MetadataStorageError>;
    fn load_metadata(&self) -> Result<Vec<DatasetMetadata>, MetadataStorageError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ArtifactStorageError {
    #[error("Failed to prepare artifact storage location")]
    PrepareStorage,

    #[error("Failed to save artifact")]
    Save,
}

pub trait ArtifactStorage {
    fn save_artifact(
        &self,
        dataset_id: &str,
        source_path: &Path,
    ) -> Result<ArtifactLocator, ArtifactStorageError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactLocator(String);

// TODO: add validation
impl ArtifactLocator {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl From<ArtifactLocator> for String {
    fn from(value: ArtifactLocator) -> Self {
        value.0
    }
}

impl From<String> for ArtifactLocator {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ArtifactLocator {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<PathBuf> for ArtifactLocator {
    fn from(value: PathBuf) -> Self {
        Self::new(value.to_string_lossy())
    }
}

impl AsRef<str> for ArtifactLocator {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ArtifactLocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RasterReaderError {
    #[error("Failed to open raster")]
    Open,

    #[error("Failed to read raster pixel")]
    Read,
}

pub trait RasterReader {
    fn read_pixel(
        &self,
        locator: &ArtifactLocator,
        col: usize,
        row: usize,
    ) -> Result<Elevation, RasterReaderError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Crs(String);

impl std::fmt::Display for Crs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// TODO: validation on creation/conversion
impl Crs {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn unknown() -> Self {
        Self::new("Unknown")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub dataset_id: String,
    pub artifact_path: ArtifactLocator,
    pub raster: RasterMetadata,
    // pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RasterMetadata {
    pub crs: Crs,
    pub width: usize,
    pub height: usize,
    pub geo_transform: GeoTransform,
    pub bounds: Bounds,
    pub nodata: Option<f64>,
    pub block_size: BlockSize,
    pub overview_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoTransform {
    pub origin_lon: f64,
    pub origin_lat: f64,
    pub pixel_width: f64,
    pub pixel_height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSize {
    pub width: usize,
    pub height: usize,
}
