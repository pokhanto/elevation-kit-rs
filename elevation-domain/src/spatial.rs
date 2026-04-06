//! Spatial primitives and small geometry-related helpers.

use geo::{LineString, Polygon, Rect};
use serde::{Deserialize, Serialize};

/// Coordinate reference system identifier.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Crs(String);

impl Crs {
    // TODO: probalby need some validation
    /// Creates new CRS value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns placeholder CRS used when source CRS is unknown.
    pub fn unknown() -> Self {
        Self::new("Unknown")
    }
}

impl AsRef<str> for Crs {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Crs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Hint used to select an output raster resolution.
#[derive(Debug, Clone, PartialEq)]
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

/// Axis-aligned geographic bounds.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Bounds {
    /// Minimum longitude.
    pub min_lon: f64,
    /// Minimum latitude.
    pub min_lat: f64,
    /// Maximum longitude.
    pub max_lon: f64,
    /// Maximum latitude.
    pub max_lat: f64,
}

impl Bounds {
    /// Returns intersection of two bounding boxes, if any.
    pub fn intersection(&self, other: &Bounds) -> Option<Bounds> {
        let min_lon = self.min_lon.max(other.min_lon);
        let min_lat = self.min_lat.max(other.min_lat);
        let max_lon = self.max_lon.min(other.max_lon);
        let max_lat = self.max_lat.min(other.max_lat);

        if min_lon <= max_lon && min_lat <= max_lat {
            Some(Bounds {
                min_lon,
                min_lat,
                max_lon,
                max_lat,
            })
        } else {
            None
        }
    }

    /// Returns `true` if bounds contain provided point.
    pub fn contains_point(&self, lon: f64, lat: f64) -> bool {
        lon >= self.min_lon && lon <= self.max_lon && lat >= self.min_lat && lat <= self.max_lat
    }
}

impl From<Bounds> for Polygon<f64> {
    fn from(value: Bounds) -> Self {
        let exterior = LineString::from(vec![
            (value.min_lon, value.min_lat),
            (value.max_lon, value.min_lat),
            (value.max_lon, value.max_lat),
            (value.min_lon, value.max_lat),
            (value.min_lon, value.min_lat),
        ]);

        Polygon::new(exterior, vec![])
    }
}

impl From<Rect> for Bounds {
    fn from(value: Rect) -> Self {
        Self {
            min_lon: value.min().x,
            min_lat: value.min().y,
            max_lon: value.max().x,
            max_lat: value.max().y,
        }
    }
}

impl From<Bounds> for Rect<f64> {
    fn from(value: Bounds) -> Self {
        Rect::new(
            geo::Coord {
                x: value.min_lon,
                y: value.min_lat,
            },
            geo::Coord {
                x: value.max_lon,
                y: value.max_lat,
            },
        )
    }
}
