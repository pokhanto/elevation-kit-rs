use elevation_types::{DatasetMetadata, MetadataStorage, RasterReader};
use tracing::{info, instrument};

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
    #[instrument(skip(self), fields(lon, lat))]
    pub fn elevation_at(&self, lon: f64, lat: f64) -> Option<f64> {
        let datasets = self.metadata.load_metadata().unwrap();
        let dataset = get_metadata(datasets, lon, lat);

        if let Some(dataset) = dataset {
            info!(dataset_id = %dataset.dataset_id, "dataset selected");

            let pixel = world_to_pixel(&dataset, lon, lat).unwrap();

            info!(col = pixel.col, row = pixel.row, "pixel resolved");

            let value = self
                .raster
                .read_pixel(&dataset.artifact_path, pixel.col, pixel.row);

            // TODO: check nodata

            return Some(value);
        }

        None
    }
}

// TODO: rename to more meaningful name
fn get_metadata(datasets: Vec<DatasetMetadata>, lon: f64, lat: f64) -> Option<DatasetMetadata> {
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

// TODO: support more crs, add conversion
fn world_to_pixel(metadata: &DatasetMetadata, lon: f64, lat: f64) -> Option<PixelCoordinate> {
    let gt = &metadata.raster.geo_transform;

    let col = ((lon - gt.origin_lon) / gt.pixel_width).floor();
    let row = ((lat - gt.origin_lat) / gt.pixel_height).floor();

    if !col.is_finite() || !row.is_finite() {
        return None;
    }

    let col = col as i64;
    let row = row as i64;

    if col < 0 || row < 0 {
        return None;
    }

    let col = col as usize;
    let row = row as usize;

    if col >= metadata.raster.width || row >= metadata.raster.height {
        return None;
    }

    Some(PixelCoordinate { col, row })
}
