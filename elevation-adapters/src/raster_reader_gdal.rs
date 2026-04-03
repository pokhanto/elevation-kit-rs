use elevation_types::{
    ArtifactLocator, RasterReadWindow, RasterReader, RasterReaderError, RasterWindowData,
};
use gdal::Dataset;

pub struct GdalRasterReader;

impl RasterReader<f64> for GdalRasterReader {
    #[tracing::instrument(
        skip(self),
        fields(
            artifact_path = %path,
        ),
        err
    )]
    fn read_window(
        &self,
        path: &ArtifactLocator,
        raster_window: RasterReadWindow,
    ) -> Result<RasterWindowData<f64>, RasterReaderError> {
        // TODO: preload/cache dataset
        let dataset = Dataset::open(path.as_ref()).map_err(|err| {
            tracing::debug!(
                error = %err,
                "failed to open raster dataset"
            );
            RasterReaderError::Open
        })?;
        let band_index = 1;
        let band = dataset.rasterband(band_index).map_err(|err| {
            tracing::debug!(
                error = %err,
                band_index,
                "failed to read band at requested index"
            );
            RasterReaderError::Read
        })?;

        let RasterReadWindow {
            placement,
            source_size,
            target_size,
        } = raster_window;

        let buffer = band
            .read_as::<f64>(
                (placement.column() as isize, placement.row() as isize),
                (source_size.width(), source_size.height()),
                (target_size.width(), target_size.height()),
                None,
            )
            .map_err(|err| {
                tracing::debug!(
                    error = %err,
                    placement = ?placement,
                    source_size = ?source_size,
                    target_size = ?target_size,
                    band_index,
                    "failed to read window in band"
                );
                RasterReaderError::Read
            })?;

        RasterWindowData::new(raster_window, buffer.data()).map_err(|err| {
            tracing::debug!(
                error = %err,
                placement = ?placement,
                source_size = ?source_size,
                target_size = ?target_size,
                "failed to construct resulting data for requested window"
            );
            RasterReaderError::Read
        })
    }
}
