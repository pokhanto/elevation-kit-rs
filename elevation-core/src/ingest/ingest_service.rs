//! Raster ingest pipeline.
//!
//! This module prepares source rasters for use by application by optionally
//! reprojecting them, converting them to COG, extracting metadata,
//! and storing artifacts and metadata.

use elevation_domain::{
    ArtifactStorage, BlockSize, Bounds, Crs, DatasetMetadata, GeoTransform, MetadataStorage,
    RasterMetadata,
};
use gdal::{Dataset, Metadata};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use super::gdal_processor::{GdalProcessSettings, GdalProcessor};

/// Errors returned during dataset ingest.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum IngestServiceError {
    #[error("Failed to reproject source raster.")]
    Reprojection,

    #[error("Failed to convert raster to cloud-optimized geotiff.")]
    CogConversion,

    #[error("Failed to extract raster metadata.")]
    MetadataExtraction,

    #[error("Failed to save artifact.")]
    ArtifactStorage,

    #[error("Failed to save metadata.")]
    MetadataStorage,

    #[error("Failed to create temporary workspace.")]
    TempWorkspace,
}

pub struct IngestService<A, M> {
    crs: Crs,
    artifact_storage: A,
    metadata_storage: M,
}

impl<A, M> IngestService<A, M> {
    pub fn new(crs: Crs, artifact_storage: A, metadata_storage: M) -> Self {
        Self {
            crs,
            artifact_storage,
            metadata_storage,
        }
    }
}

impl<A, M> IngestService<A, M>
where
    A: ArtifactStorage,
    M: MetadataStorage,
{
    #[tracing::instrument(
    skip(self),
    fields(
        crs = %self.crs,
        dataset_id = %dataset_id,
        source_path = %source_path.display(),
    )
)]
    pub async fn run(
        self,
        dataset_id: String,
        source_path: PathBuf,
    ) -> Result<(), IngestServiceError> {
        tracing::info!("starting ingest");

        let temp_dir = TempDir::new().map_err(|err| {
            tracing::error!(error = %err, "failed to create temp workspace");
            IngestServiceError::TempWorkspace
        })?;

        let mut current_path = source_path;
        let gdal_processor = GdalProcessor::new(GdalProcessSettings::default());

        let dataset = open_dataset(&current_path)?;

        let source_crs = get_crs(&dataset).unwrap_or_else(|err| {
            tracing::warn!(error = %err, "failed to determine CRS, falling back to unknown CRS");
            Crs::unknown()
        });

        if source_crs != self.crs {
            tracing::info!(from = %source_crs, to = %self.crs, "reprojection required");

            let reprojected_path = temp_dir.path().join("reprojected.tif");

            gdal_processor
                .reproject_to_path(&current_path, self.crs.as_ref(), &reprojected_path)
                .map_err(|err| {
                    tracing::error!(
                        error = %err,
                        path = %current_path.display(),
                        crs_from = %source_crs,
                        crs_to = %self.crs.as_ref(),
                        "failed to reproject raster"
                    );
                    IngestServiceError::Reprojection
                })?;

            current_path = reprojected_path;
        }

        let dataset = open_dataset(&current_path)?;

        if !is_cog(&dataset) {
            tracing::info!("cog conversion required");

            let translated_path = temp_dir.path().join("translated.cog.tif");

            gdal_processor
                .translate_to_cog_path(&current_path, &translated_path)
                .map_err(|err| {
                    tracing::error!(
                        error = %err,
                        path = %current_path.display(),
                        "failed to translate raster to COG"
                    );
                    IngestServiceError::CogConversion
                })?;

            current_path = translated_path;
        }

        let artifact_path = self
            .artifact_storage
            .save_artifact(&dataset_id, current_path.as_path())
            .await
            .map_err(|err| {
                tracing::error!(
                    error = %err,
                    path = %current_path.display(),
                    "failed to save artifact"
                );
                IngestServiceError::ArtifactStorage
            })?;
        tracing::info!(artifact_path = %artifact_path, "artifact stored");

        let raster_metadata = read_raster_metadata(&current_path).map_err(|err| {
            tracing::error!(
                error = %err,
                path = %current_path.display(),
                "failed to extract raster metadata"
            );
            IngestServiceError::MetadataExtraction
        })?;

        let metadata = DatasetMetadata {
            dataset_id,
            artifact_path,
            raster: raster_metadata,
        };

        self.metadata_storage
            .save_metadata(metadata)
            .await
            .map_err(|err| {
                tracing::error!(error = %err, "failed to save metadata");
                IngestServiceError::MetadataStorage
            })?;
        tracing::info!("metadata stored");

        tracing::info!("ingest completed");
        Ok(())
    }
}

/// Opens GDAL dataset from disk.
fn open_dataset(path: &Path) -> Result<Dataset, IngestServiceError> {
    Dataset::open(path).map_err(|err| {
        tracing::error!(error = %err, path = %path.display(), "failed to open raster dataset");
        IngestServiceError::MetadataExtraction
    })
}

/// Extracts raster metadata from dataset at given path.
#[tracing::instrument(skip_all, fields(path = %path.display()))]
pub fn read_raster_metadata(path: &Path) -> Result<RasterMetadata, IngestServiceError> {
    tracing::info!("starting metadata extraction");

    let dataset = open_dataset(path)?;

    let (width, height) = dataset.raster_size();
    tracing::debug!(width, height, "raster size extracted");

    let geo_transform = dataset.geo_transform().map_err(|err| {
        tracing::error!(error = %err, "failed to get geotransform");
        IngestServiceError::MetadataExtraction
    })?;
    tracing::debug!(?geo_transform, "geotransform extracted");

    // TODO: not north up rasters should be reprojected
    // NOTE: this assume that raster in north up -
    // pixel_width is positive, and pixel_height is negative
    let [min_lon, pixel_width, _, max_lat, _, pixel_height] = geo_transform;
    let max_lon = min_lon + width as f64 * pixel_width;
    let min_lat = max_lat + height as f64 * pixel_height;

    let crs = get_crs(&dataset)?;
    tracing::debug!(crs = ?crs, "crs extracted");

    let band = dataset.rasterband(1).map_err(|err| {
        tracing::error!(error = %err, "failed to get first raster band");
        IngestServiceError::MetadataExtraction
    })?;

    let (block_width, block_height) = band.block_size();
    tracing::debug!(block_width, block_height, "block size extracted");

    let nodata = band.no_data_value();
    tracing::debug!(?nodata, "nodata value extracted");

    tracing::info!("metadata extraction completed");

    let bounds = Bounds::try_new(min_lon, min_lat, max_lon, max_lat).map_err(|err| {
        dbg!(&err);
        tracing::error!(error = %err, min_lon, min_lat, max_lon, max_lat, "provided bounds are not valid");
        IngestServiceError::MetadataExtraction
    })?;

    Ok(RasterMetadata {
        crs,
        width,
        height,
        geo_transform: GeoTransform {
            origin_lon: geo_transform[0],
            origin_lat: geo_transform[3],
            pixel_width: geo_transform[1],
            pixel_height: geo_transform[5],
        },
        bounds,
        nodata,
        block_size: BlockSize {
            width: block_width,
            height: block_height,
        },
        overview_count: 0,
    })
}

/// Returns `true` if dataset is marked as Cloud Optimized GeoTIFF.
fn is_cog(dataset: &Dataset) -> bool {
    dataset
        .metadata_domain("IMAGE_STRUCTURE")
        .unwrap_or_default()
        .iter()
        .any(|item| item == "LAYOUT=COG")
}

/// Extracts dataset's CRS.
fn get_crs(dataset: &Dataset) -> Result<Crs, IngestServiceError> {
    let crs_string = dataset
        .spatial_ref()
        .and_then(|spatial_ref| spatial_ref.authority())
        .map_err(|err| {
            tracing::error!(error = %err, "failed to get spatial authority");
            IngestServiceError::MetadataExtraction
        })?;

    Ok(Crs::new(crs_string))
}
