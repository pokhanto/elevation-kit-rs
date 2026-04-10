use std::str::FromStr;

use geo::{BoundingRect, LineString, Polygon, Rect};
use h3o::CellIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tile(CellIndex);

impl Tile {
    pub fn new(cell: CellIndex) -> Self {
        Self(cell)
    }

    pub fn cell(&self) -> CellIndex {
        self.0
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }

    pub fn polygon(&self) -> Polygon<f64> {
        let mut coords = self
            .0
            .boundary()
            .iter()
            .map(|coord| (coord.lng(), coord.lat()))
            .collect::<Vec<_>>();

        coords.push(coords[0]);

        Polygon::new(LineString::from(coords), vec![])
    }

    pub fn bounding_rect(&self) -> Option<Rect<f64>> {
        self.polygon().bounding_rect()
    }
}

impl std::fmt::Display for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<CellIndex> for Tile {
    fn from(value: CellIndex) -> Self {
        Self(value)
    }
}

impl From<Tile> for CellIndex {
    fn from(value: Tile) -> Self {
        value.0
    }
}

impl FromStr for Tile {
    type Err = h3o::error::InvalidCellIndex;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(CellIndex::from_str(s)?))
    }
}
