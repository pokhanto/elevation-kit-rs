//! Elevation types.

use crate::Bounds;

/// Elevation value in the dataset's units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Elevation(pub f64);

/// Elevations returned for a bounding box request.
#[derive(Debug, Clone, PartialEq)]
pub struct BboxElevations {
    /// Requested bounding box.
    pub bbox: Bounds,
    /// Raster width in samples.
    pub width: usize,
    /// Raster height in samples.
    pub height: usize,
    /// Raster values in row-major order.
    pub values: Vec<Option<Elevation>>,
}
