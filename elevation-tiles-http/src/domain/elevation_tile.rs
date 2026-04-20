//! Tile domain model.

use georaster_domain::RasterValue;

/// Tile with aggregated elevation data.
#[derive(Debug, Clone)]
pub struct ElevationTile {
    id: String,
    elevation: Option<RasterValue>,
}

impl ElevationTile {
    /// Creates a new tile.
    pub fn new(id: String, elevation: Option<RasterValue>) -> Self {
        Self { id, elevation }
    }

    /// Returns tile id.
    pub fn id(&self) -> &str {
        self.id.as_ref()
    }

    /// Returns tile elevation.
    pub fn elevation(&self) -> Option<RasterValue> {
        self.elevation
    }
}
