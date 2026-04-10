//! Helpers for building tiles from elevation samples.

use elevation_domain::Elevation;

#[derive(Debug, Clone, Default)]
pub struct MeanElevationAccumulator {
    sum: f64,
    count: usize,
}

impl MeanElevationAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, value: Elevation) {
        self.sum += value.0;
        self.count += 1;
    }

    pub fn add_optional(&mut self, value: Option<Elevation>) {
        if let Some(value) = value {
            self.add(value);
        }
    }

    pub fn extend<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Option<Elevation>>,
    {
        for value in values {
            self.add_optional(value);
        }
    }

    pub fn mean(&self) -> Option<Elevation> {
        (self.count > 0).then(|| Elevation(self.sum / self.count as f64))
    }
}
