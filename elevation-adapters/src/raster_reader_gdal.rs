use elevation_types::{ArtifactLocator, Elevation, RasterReader, RasterReaderError};
use gdal::Dataset;

pub struct GdalRasterReader;

impl RasterReader for GdalRasterReader {
    #[tracing::instrument(
        skip(self),
        fields(
            artifact_path = %path,
            col = col,
            row = row,
        ),
        err
    )]
    fn read_pixel(
        &self,
        path: &ArtifactLocator,
        col: usize,
        row: usize,
    ) -> Result<Elevation, RasterReaderError> {
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

        let buffer = band
            .read_as::<f64>((col as isize, row as isize), (1, 1), (1, 1), None)
            .map_err(|err| {
                tracing::debug!(
                    error = %err,
                    col,
                    row,
                    band_index,
                    "failed to read window in band"
                );
                RasterReaderError::Read
            })?;

        Ok(Elevation(buffer.data()[0]))
    }
}
