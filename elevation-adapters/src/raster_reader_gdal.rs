use elevation_types::RasterReader;
use gdal::Dataset;

pub struct GdalRasterReader;

impl RasterReader for GdalRasterReader {
    fn read_pixel(&self, path: &str, col: usize, row: usize) -> f64 {
        // TODO: preload/cache dataset
        let dataset = Dataset::open(path).unwrap();
        let band = dataset.rasterband(1).unwrap();

        // TODO: reowrk casting
        let buf = band
            .read_as::<f64>((col as isize, row as isize), (1, 1), (1, 1), None)
            .unwrap();

        buf.data()[0]
    }
}
