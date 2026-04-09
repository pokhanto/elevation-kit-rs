//! Raster window, raster reading, and raster payload types.

use crate::storage::ArtifactLocator;

/// Position of window inside raster.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowPlacement {
    column: usize,
    row: usize,
}

impl WindowPlacement {
    /// Creates new placement.
    pub fn new(column: usize, row: usize) -> Self {
        Self { column, row }
    }

    /// Returns starting column.
    pub fn column(&self) -> usize {
        self.column
    }

    /// Returns starting row.
    pub fn row(&self) -> usize {
        self.row
    }
}

/// Raster size in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RasterSize {
    width: usize,
    height: usize,
}

impl RasterSize {
    /// Creates new size.
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    /// Returns size of single pixel.
    pub fn point() -> Self {
        Self {
            width: 1,
            height: 1,
        }
    }

    /// Returns width.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns height.
    pub fn height(&self) -> usize {
        self.height
    }
}

/// Window describing raster read operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RasterReadWindow {
    /// Placement of window inside source raster.
    placement: WindowPlacement,
    /// Size of source window.
    source_size: RasterSize,
    /// Size of returned target data.
    target_size: RasterSize,
}

impl RasterReadWindow {
    /// Creates new raster read window.
    pub fn new(
        placement: WindowPlacement,
        source_size: RasterSize,
        target_size: RasterSize,
    ) -> Self {
        Self {
            placement,
            source_size,
            target_size,
        }
    }

    /// Creates point read window.
    pub fn new_point(placement: WindowPlacement) -> Self {
        Self {
            placement,
            source_size: RasterSize::point(),
            target_size: RasterSize::point(),
        }
    }

    /// Returns placement of window.
    pub fn placement(&self) -> WindowPlacement {
        self.placement
    }

    /// Returns source size of window.
    pub fn source_size(&self) -> RasterSize {
        self.source_size
    }

    /// Returns target size of window.
    pub fn target_size(&self) -> RasterSize {
        self.target_size
    }
}

/// Errors returned when building raster window data.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RasterWindowDataError {
    #[error("values length does not match window dimensions")]
    InvalidValuesLength,
}

/// Raster values returned for window.
#[derive(Debug, Clone, PartialEq)]
pub struct RasterWindowData<T> {
    window: RasterReadWindow,
    values: Vec<T>,
}

impl<T> RasterWindowData<T> {
    /// Creates new raster window payload.
    pub fn try_new(
        window: RasterReadWindow,
        values: impl Into<Vec<T>>,
    ) -> Result<Self, RasterWindowDataError> {
        let values = values.into();
        let target_size = window.target_size.width * window.target_size.height;

        if values.len() != target_size {
            return Err(RasterWindowDataError::InvalidValuesLength);
        }

        Ok(Self { window, values })
    }

    /// Returns all values as slice.
    pub fn values(&self) -> &[T] {
        &self.values
    }

    /// Consumes payload and returns inner values.
    pub fn into_values(self) -> Vec<T> {
        self.values
    }

    /// Returns value by target column and row.
    pub fn get(&self, col: usize, row: usize) -> Option<&T> {
        if col >= self.window.target_size.width || row >= self.window.target_size.height {
            return None;
        }

        self.values.get(row * self.window.target_size.width + col)
    }

    /// Returns target height.
    pub fn target_height(&self) -> usize {
        self.window.target_size.height
    }

    /// Returns target width.
    pub fn target_width(&self) -> usize {
        self.window.target_size.width
    }
}

/// Hint used to select an output raster resolution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResolutionHint {
    /// Use highest available resolution.
    Highest,
    /// Use lowest available resolution.
    Lowest,
    /// Use explicit target resolution in degrees.
    Degrees {
        /// Target longitudinal resolution.
        lon_resolution: f64,
        /// Target latitudinal resolution.
        lat_resolution: f64,
    },
}

/// Errors returned by raster readers.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RasterReaderError {
    #[error("Failed to resolve path")]
    Path,
    #[error("Failed to open raster")]
    Open,
    #[error("Failed to read raster pixel")]
    Read,
}

/// Reads raster data from stored artifact.
///
/// Implementations are responsible for opening raster artifact identified by
/// an [`ArtifactLocator`] and returning data for requested read window.
/// This trait is used by higher-level services to fetch raster samples without
/// depending on specific raster library or file format implementation.
///
/// Generic parameter `T` represents value type returned from raster.
pub trait RasterReader<T> {
    /// Reads raster window from artifact.
    ///
    /// Returned [`RasterWindowData`] must match requested target window
    /// dimensions and contain values in row-major order.
    ///
    /// Returns error if raster cannot be opened or window cannot be read.
    fn read_window(
        &self,
        locator: &ArtifactLocator,
        raster_window: RasterReadWindow,
    ) -> impl Future<Output = Result<RasterWindowData<T>, RasterReaderError>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_placement_returns_column_and_row() {
        let placement = WindowPlacement::new(3, 7);

        assert_eq!(placement.column(), 3);
        assert_eq!(placement.row(), 7);
    }

    #[test]
    fn raster_size_returns_width_and_height() {
        let size = RasterSize::new(10, 20);

        assert_eq!(size.width(), 10);
        assert_eq!(size.height(), 20);
    }

    #[test]
    fn raster_size_point_is_one_by_one() {
        let size = RasterSize::point();

        assert_eq!(size.width(), 1);
        assert_eq!(size.height(), 1);
    }

    #[test]
    fn raster_read_window_returns_parts() {
        let placement = WindowPlacement::new(2, 4);
        let source_size = RasterSize::new(5, 6);
        let target_size = RasterSize::new(7, 8);

        let window = RasterReadWindow::new(placement, source_size, target_size);

        assert_eq!(window.placement(), placement);
        assert_eq!(window.source_size(), source_size);
        assert_eq!(window.target_size(), target_size);
    }

    #[test]
    fn raster_read_window_new_point_creates_one_by_one_window() {
        let placement = WindowPlacement::new(9, 11);

        let window = RasterReadWindow::new_point(placement);

        assert_eq!(window.placement(), placement);
        assert_eq!(window.source_size(), RasterSize::point());
        assert_eq!(window.target_size(), RasterSize::point());
    }

    #[test]
    fn raster_window_data_try_new_accepts_matching_values() {
        let window = RasterReadWindow::new(
            WindowPlacement::new(0, 0),
            RasterSize::new(2, 2),
            RasterSize::new(2, 3),
        );

        let data = RasterWindowData::try_new(window, vec![1, 2, 3, 4, 5, 6]).unwrap();

        assert_eq!(data.target_width(), 2);
        assert_eq!(data.target_height(), 3);
        assert_eq!(data.values(), &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn raster_window_data_try_new_rejects_invalid_values() {
        let window = RasterReadWindow::new(
            WindowPlacement::new(0, 0),
            RasterSize::new(2, 2),
            RasterSize::new(2, 3),
        );

        let err = RasterWindowData::try_new(window, vec![1, 2, 3]).unwrap_err();

        assert_eq!(err, RasterWindowDataError::InvalidValuesLength);
    }

    #[test]
    fn raster_window_data_get_returns_value_by_row_major_index() {
        let window = RasterReadWindow::new(
            WindowPlacement::new(0, 0),
            RasterSize::new(2, 2),
            RasterSize::new(3, 2),
        );

        let data = RasterWindowData::try_new(window, vec![10, 11, 12, 20, 21, 22]).unwrap();

        assert_eq!(data.get(0, 0), Some(&10));
        assert_eq!(data.get(1, 0), Some(&11));
        assert_eq!(data.get(2, 0), Some(&12));
        assert_eq!(data.get(0, 1), Some(&20));
        assert_eq!(data.get(1, 1), Some(&21));
        assert_eq!(data.get(2, 1), Some(&22));
    }

    #[test]
    fn raster_window_data_get_returns_none_when_out_of_bounds() {
        let window = RasterReadWindow::new(
            WindowPlacement::new(0, 0),
            RasterSize::new(1, 1),
            RasterSize::new(2, 2),
        );

        let data = RasterWindowData::try_new(window, vec![1, 2, 3, 4]).unwrap();

        assert_eq!(data.get(2, 0), None);
        assert_eq!(data.get(0, 2), None);
        assert_eq!(data.get(2, 2), None);
    }

    #[test]
    fn raster_window_data_into_values_returns_inner_vector() {
        let window = RasterReadWindow::new_point(WindowPlacement::new(0, 0));
        let data = RasterWindowData::try_new(window, vec![42]).unwrap();

        assert_eq!(data.into_values(), vec![42]);
    }
}
