use elevation_types::{DatasetMetadata, Elevation, MetadataStorage, RasterReader};

#[derive(Debug, thiserror::Error)]
pub enum ElevationServiceError {
    #[error("Can't read metadata")]
    Metadata,

    #[error("Can't read raster")]
    Raster,
}

struct PixelCoordinate {
    pub col: usize,
    pub row: usize,
}

pub struct ElevationService<M, R> {
    metadata: M,
    raster: R,
}

impl<M, R> ElevationService<M, R> {
    pub fn new(metadata: M, raster: R) -> Self {
        Self { metadata, raster }
    }
}

impl<M, R> ElevationService<M, R>
where
    M: MetadataStorage,
    R: RasterReader,
{
    #[tracing::instrument(skip(self), fields(lon, lat))]
    pub fn elevation_at(
        &self,
        lon: f64,
        lat: f64,
    ) -> Result<Option<Elevation>, ElevationServiceError> {
        tracing::info!(lon, lat, "starting getting elevation at point");
        let datasets = self.metadata.load_metadata().map_err(|err| {
            tracing::error!(
                error = %err,
                lon = lon,
                lat = lat,
                "failed to load dataset metadata"
            );

            ElevationServiceError::Metadata
        })?;
        let dataset_metadata = match resolve_dataset_metadata(datasets, lon, lat) {
            Some(dataset) => dataset,
            None => return Ok(None),
        };
        tracing::info!(dataset_id = %dataset_metadata.dataset_id, "dataset selected");

        let pixel_coordinate = match latlon_to_pixel(&dataset_metadata, lon, lat) {
            Some(pixel) => pixel,
            None => return Ok(None),
        };

        tracing::info!(
            col = pixel_coordinate.col,
            row = pixel_coordinate.row,
            "pixel resolved"
        );

        let elevation = self
            .raster
            .read_pixel(
                &dataset_metadata.artifact_path,
                pixel_coordinate.col,
                pixel_coordinate.row,
            )
            .map_err(|err| {
                tracing::error!(
                    error = %err,
                    dataset_id = %dataset_metadata.dataset_id,
                    artifact = %dataset_metadata.artifact_path,
                    lon = lon,
                    lat = lat,
                    col = pixel_coordinate.col,
                    row = pixel_coordinate.row,
                    "failed to read raster pixel"
                );

                ElevationServiceError::Raster
            })?;

        if dataset_metadata.raster.nodata == Some(elevation.0) {
            tracing::info!("elevation at point resolved to be nodata");
            return Ok(None);
        }

        tracing::info!(elevation = elevation.0, "elevation at point resolved");
        Ok(Some(elevation))
    }
}

fn resolve_dataset_metadata(
    datasets: Vec<DatasetMetadata>,
    lon: f64,
    lat: f64,
) -> Option<DatasetMetadata> {
    let mut filtered: Vec<DatasetMetadata> = datasets
        .into_iter()
        .filter(|ds| {
            lon >= ds.raster.bounds.min_lon
                && lon <= ds.raster.bounds.max_lon
                && lat >= ds.raster.bounds.min_lat
                && lat <= ds.raster.bounds.max_lat
        })
        .collect();
    // TODO: filter by quality too

    filtered.pop()
}

// TODO: consider lat lon order, not lon lat
fn latlon_to_pixel(metadata: &DatasetMetadata, lon: f64, lat: f64) -> Option<PixelCoordinate> {
    let gt = &metadata.raster.geo_transform;

    let col = ((lon - gt.origin_lon) / gt.pixel_width).floor();
    let row = ((lat - gt.origin_lat) / gt.pixel_height).floor();

    if !col.is_finite() || !row.is_finite() {
        tracing::debug!(
            lon,
            lat,
            origin_lon = gt.origin_lon,
            origin_lat = gt.origin_lat,
            pixel_width = gt.pixel_width,
            pixel_height = gt.pixel_height,
            col,
            row,
            "pixel coordinate produced non finite values"
        );
        return None;
    }

    let col = col as i64;
    let row = row as i64;

    if col < 0 || row < 0 {
        tracing::debug!(col, row, "requested coordinates are less than 0");
        return None;
    }

    let col = col as usize;
    let row = row as usize;

    if col >= metadata.raster.width || row >= metadata.raster.height {
        tracing::debug!(
            col,
            row,
            width = metadata.raster.width,
            height = metadata.raster.height,
            "requested coordinates are out of bounds"
        );
        return None;
    }

    Some(PixelCoordinate { col, row })
}
